// Native IPC regression with synthetic records and a loopback-only mock provider.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import {createRequire} from 'node:module';

assert.equal(process.env.MSL_ISOLATED_TEST, '1');
assert.ok(process.env.MSL_TEST_PROFILE, 'An explicit isolated profile is required');
const profile = path.resolve(process.env.MSL_TEST_PROFILE);
assert.ok(profile.split(path.sep).includes('.test-runtime'));
for (const [key, part] of Object.entries({APPDATA:'appdata', LOCALAPPDATA:'localappdata', TEMP:'temp', TMP:'temp'})) {
  assert.equal(path.resolve(process.env[key] ?? ''), path.join(profile, part), `Unsafe ${key}`);
}
assert.ok(process.env.MSL_NODE_MODULES && process.env.MSL_CDP_PORT);
const {chromium} = createRequire(path.join(process.env.MSL_NODE_MODULES, 'runtime.cjs'))('playwright');
let browser;
for (const host of ['127.0.0.1', '[::1]']) {
  try { browser = await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`); break; } catch {}
}
assert.ok(browser, 'An isolated native WebView must be running');
const page = browser.contexts()[0].pages()[0];
const call = (command, args = {}) => page.evaluate(
  ({command, args}) => window.__TAURI_INTERNALS__.invoke(command, args), {command, args},
);
const artifacts = path.join(profile, 'artifacts');
fs.mkdirSync(artifacts, {recursive:true});
const receiptFile = path.join(artifacts, 'workflow-continuity.json');
const taskList = () => call('list_tasks', {status:null, workId:null});
const waitingList = () => call('list_waiting', {status:null, workId:null});
const taskById = async id => (await taskList()).find(row => row.id === id);
const scope = row => JSON.parse(row.scope_json).sort((a, b) => a - b);
const reject = async (command, args) => {
  let rejected = false;
  try { await call(command, args); } catch { rejected = true; }
  assert.ok(rejected, `${command} unexpectedly accepted an unsafe operation`);
};
const runJob = async (command, args) => {
  const job = await call('start_ai_job', {request:{command, args}});
  const deadline = Date.now() + 90000;
  while (Date.now() < deadline) {
    const current = await call('get_ai_job', {id:job.id});
    if (current.status !== 'running') {
      assert.equal(current.status, 'completed', current.error ?? 'Background job failed');
      return current.result;
    }
    await new Promise(resolve => setTimeout(resolve, 150));
  }
  throw new Error('Synthetic background job did not complete');
};
let mockServer;

try {
  const status = await call('backup_status');
  assert.equal(path.resolve(status.data_directory), path.join(profile, 'appdata', 'MSLDesktop'));
  const schedule = await call('get_analysis_schedule');
  await call('save_analysis_schedule', {schedule:{...schedule, enabled:false, daily_enabled:false}});

  if (process.argv.includes('--reopened')) {
    const saved = JSON.parse(fs.readFileSync(receiptFile, 'utf8'));
    const receipts = await call('list_manual_completions');
    assert.ok(receipts.some(row => row.id === saved.waitingReceipt && !row.undone_at));
    const session = (await call('list_qa_sessions')).find(row => row.id === saved.expertSession);
    assert.ok(session.expert_scoped, 'Expert-only scope was lost on restart');
    assert.equal(session.expert_id, saved.expertId);
    assert.deepEqual(scope(session), []);
    const turns = await call('list_qa_turns', {sessionId:session.id});
    assert.ok(turns.some(row => row.id === saved.expertTurn && row.expert_scoped));
    assert.ok(turns.some(row => row.id === saved.expertTurn && row.status === 'completed'));
    const report = (await call('list_reports', {kind:null, limit:100})).find(row => row.id === saved.reportId);
    assert.ok(report.structured_json && report.evidence_json, 'Structured report was lost on restart');
    await call('undo_manual_completion', {receiptId:saved.waitingReceipt});
    assert.equal((await waitingList()).find(row => row.id === saved.waitingId).status, saved.waitingStatus);
    await reject('undo_manual_completion', {receiptId:saved.changedReceipt});
    assert.equal((await taskById(saved.changedTask)).notes, 'Synthetic later edit must survive');
    fs.writeFileSync(path.join(artifacts, 'workflow-continuity-reopened.json'), JSON.stringify({pass:true}));
    console.log('PASS restart: durable undo, preserved expert-only QA scope, concurrent edit protection');
  } else {
    assert.ok(!fs.existsSync(receiptFile), 'Use a fresh profile for the initial run');
    const projectA = await call('create_work', {title:'Synthetic continuity project A', status:'active'});
    const projectB = await call('create_work', {title:'Synthetic continuity project B', status:'active'});
    const createTask = (title, workId = projectA.id) => call('create_task', {
      workId, title, priority:'normal', dueAt:null, notes:'Synthetic test record',
    });
    const task = await createTask('Synthetic reversible completion');
    const receipt = await call('complete_task', {id:task.id});
    assert.equal(receipt.entity_kind, 'task');
    assert.equal(receipt.entity_id, task.id);
    assert.equal(typeof receipt.id, 'string');
    assert.equal((await taskById(task.id)).status, 'done');
    assert.ok((await call('list_manual_completions')).some(row => row.id === receipt.id));
    await call('undo_manual_completion', {receiptId:receipt.id});
    assert.equal((await taskById(task.id)).status, task.status);
    await reject('undo_manual_completion', {receiptId:receipt.id});
    console.log('PASS native completion/undo restores original status and rejects duplicate undo');

    const edited = await createTask('Synthetic concurrent completion');
    const editedReceipt = await call('complete_task', {id:edited.id});
    await call('update_task', {
      id:edited.id, workId:edited.work_id, title:edited.title, priority:edited.priority,
      dueAt:edited.due_at, notes:'Synthetic later edit must survive',
    });
    await reject('undo_manual_completion', {receiptId:editedReceipt.id});
    assert.equal((await taskById(edited.id)).notes, 'Synthetic later edit must survive');

    const removal = await createTask('Synthetic guarded deletion', null);
    await reject('delete_task', {id:removal.id});
    await reject('delete_task', {id:removal.id, confirmed:false, expectedUpdatedAt:removal.updated_at, expectedRecord:removal});
    await reject('delete_task', {id:removal.id, confirmed:true, expectedUpdatedAt:removal.updated_at - 1, expectedRecord:removal});
    await reject('delete_task', {id:removal.id, confirmed:true, expectedUpdatedAt:removal.updated_at, expectedRecord:{...removal, notes:'Synthetic stale content at the same timestamp'}});
    assert.ok(await taskById(removal.id));
    await call('delete_task', {id:removal.id, confirmed:true, expectedUpdatedAt:removal.updated_at, expectedRecord:removal});
    assert.equal(await taskById(removal.id), undefined);
    console.log('PASS native delete requires explicit current confirmation; concurrent edit survives undo');

    const waiting = await call('create_waiting', {
      workId:projectB.id, title:'Synthetic persistent waiting receipt', waitingFor:'Synthetic collaborator',
      followUpAt:null, notes:'Synthetic restart test',
    });
    const waitingReceipt = await call('resolve_waiting', {id:waiting.id});
    assert.equal(waitingReceipt.entity_kind, 'waiting');
    const createExpert = (name, projects) => call('save_kol_expert', {
      id:null, revision:null, name, institution:'Synthetic institution', department:'Synthetic department',
      specialty:'Synthetic evidence review', projects, archived:false,
    });
    const independent = await createExpert('Synthetic independent expert', []);
    const linked = await createExpert('Synthetic linked expert', [projectA.id, projectB.id]);
    const independentNote = await call('capture_kol_note', {
      expertId:independent.id, workId:null, inboxId:null,
      content:'SYNTHETIC_ONLY_EXPERT_NOTE: clarify the next evidence question.', occurredAt:Math.floor(Date.now()/1000),
    });
    const linkedNote = await call('capture_kol_note', {
      expertId:linked.id, workId:projectA.id, inboxId:null,
      content:'SYNTHETIC_LINKED_EXPERT_NOTE: review the linked project evidence.', occurredAt:Math.floor(Date.now()/1000),
    });
    const expertSession = await call('create_qa_session', {title:'Synthetic expert-only conversation', scope:[], expertId:independent.id});
    assert.ok(expertSession.expert_scoped);
    assert.equal(expertSession.expert_id, independent.id);
    assert.deepEqual(scope(expertSession), []);
    const expertTurn = await call('queue_qa_question', {sessionId:expertSession.id, question:'Synthetic question without a provider call', scope:[], expertId:independent.id});
    assert.equal(expertTurn.expert_id, independent.id);
    assert.ok(expertTurn.expert_scoped);
    const linkedSession = await call('create_qa_session', {title:'Synthetic linked expert conversation', scope:[], expertId:linked.id});
    assert.deepEqual(scope(linkedSession), [projectA.id, projectB.id].sort((a, b) => a - b));
    const general = await call('create_qa_session', {title:'Synthetic general conversation', scope:[], expertId:null});
    assert.ok(!general.expert_scoped);
    console.log('PASS expert-only scope differs from global; linked projects are resolved server-side');

    const removedExpert = await createExpert('Synthetic removable expert', []);
    const orphanSession = await call('create_qa_session', {title:'Synthetic unavailable expert', scope:[], expertId:removedExpert.id});
    await call('delete_kol_expert', {id:removedExpert.id, confirmationName:removedExpert.name, expectedRevision:removedExpert.revision});
    await reject('queue_qa_question', {sessionId:orphanSession.id, question:'Must not become global', scope:[], expertId:null});
    assert.ok((await call('list_qa_sessions')).find(row => row.id === orphanSession.id).expert_scoped);
    console.log('PASS deleted expert never expands a conversation into global scope');

    let expectedNote = independentNote.id;
    let expectedProjects = [];
    let expectHistory = false;
    let qaRequests = 0;
    let reportRequests = 0;
    const mockErrors = [];
    mockServer = http.createServer(async (req, res) => {
      try {
        const chunks = [];
        for await (const chunk of req) chunks.push(chunk);
        const body = JSON.parse(Buffer.concat(chunks).toString());
        const input = JSON.parse(body.messages.find(message => message.role === 'user').content);
        let output;
        if (input.evidence) {
          qaRequests++;
          assert.deepEqual([...input.evidence.scope_ids].sort((a,b) => a-b), expectedProjects);
          const sources = input.evidence.sources;
          const note = sources.find(source => source.kind === 'kol_note' && source.entity_id === expectedNote);
          assert.ok(note, 'Selected expert note did not reach the model');
          assert.ok(!sources.some(source => source.kind === 'kol_note' && source.entity_id === (expectedNote === independentNote.id ? linkedNote.id : independentNote.id)));
          if (!expectedProjects.length) assert.ok(!sources.some(source => ['work','task','waiting','calendar'].includes(source.kind)), 'Expert-only evidence leaked global work');
          assert.equal(Boolean(input.history_context_only?.length), expectHistory);
          assert.ok(note.location?.available !== false && note.location?.entity_id === expectedNote, 'Trusted note navigation is absent');
          const quote = expectedNote === independentNote.id ? 'SYNTHETIC_ONLY_EXPERT_NOTE' : 'SYNTHETIC_LINKED_EXPERT_NOTE';
          output = {claims:[{text:'Synthetic answer based on the selected expert note.', basis:'fact', citations:[{source_id:note.id, quote}]}], gaps:['Synthetic materials do not establish any medical conclusion.']};
        } else {
          reportRequests++;
          assert.ok(!input.period_changes.some(change => change.event_type === 'task.completed' && change.target_id === task.id), 'Reversed completion leaked into report progress');
          assert.ok(input.period_changes.some(change => change.event_type === 'task.completion_undone' && JSON.parse(change.change_details ?? '{}').manual_completion_receipt_id === receipt.id), 'Report input lost the explicit completion reversal');
          const source = input.analysis.source_refs.find(ref => ref.source_type === 'task_open' && ref.entity_id === task.id);
          assert.ok(source, 'Synthetic current task was absent from report evidence');
          output = {items:[{category:'progress', project_id:projectA.id, headline:'Synthetic project follow-up', change:'The synthetic task remains available for follow-up.', impact:'Its current state can be inspected from the source.', next_action:'Review the synthetic question before the next discussion.', certainty:'observed', horizon:'current', evidence_refs:[source]}]};
        }
        res.writeHead(200, {'content-type':'application/json'});
        res.end(JSON.stringify({choices:[{message:{role:'assistant', content:JSON.stringify(output)}, finish_reason:'stop'}]}));
      } catch (error) {
        mockErrors.push(error.message);
        res.writeHead(500, {'content-type':'application/json'});
        res.end('{"error":"Synthetic contract assertion failed"}');
      }
    });
    await new Promise(resolve => mockServer.listen(0, '127.0.0.1', resolve));
    const provider = await call('save_provider_connection', {
      id:null, displayName:'Synthetic continuity provider', providerType:'custom',
      baseUrl:`http://127.0.0.1:${mockServer.address().port}/v1`, legacyModel:'', templateKind:'custom',
      authMode:'none', modelsEndpoint:null, enabled:true, apiKey:null,
    });
    const model = await call('save_provider_model', {
      providerId:provider.id, modelId:'synthetic-continuity', displayName:'Synthetic continuity model',
      protocol:'chat_completions', endpointPath:'/chat/completions', capabilitiesJson:'{}', source:'manual', enabled:true, available:true,
    });
    for (const taskKind of ['workbench_qa','weekly_report']) await call('save_ai_task_route', {taskKind, providerModelId:model.id});
    await runJob('ask_workbench', {turnId:expertTurn.id, locale:'zh-CN'});
    expectHistory = true;
    const followup = await call('queue_qa_question', {sessionId:expertSession.id, question:'Synthetic follow-up within the same expert', scope:[], expertId:independent.id});
    await runJob('ask_workbench', {turnId:followup.id, locale:'zh-CN'});
    expectedNote = linkedNote.id;
    expectedProjects = [projectA.id, projectB.id].sort((a,b) => a-b);
    expectHistory = false;
    const linkedTurn = await call('queue_qa_question', {sessionId:linkedSession.id, question:'Synthetic question for linked projects', scope:[], expertId:linked.id});
    await runJob('ask_workbench', {turnId:linkedTurn.id, locale:'zh-CN'});
    const now = Math.floor(Date.now()/1000);
    const reportId = await runJob('generate_report', {kind:'weekly', periodStart:now-86400, periodEnd:now+60});
    const report = (await call('list_reports', {kind:'weekly', limit:100})).find(row => row.id === reportId);
    assert.ok(report.content.includes('Synthetic project follow-up'));
    assert.ok(report.structured_json && report.evidence_json);
    assert.ok(JSON.parse(report.structured_json).items.length);
    assert.deepEqual(mockErrors, []);
    assert.equal(qaRequests, 3);
    assert.equal(reportRequests, 1);
    console.log('PASS loopback AI: isolated expert evidence, continuous history, trusted sources and durable structured report');
    fs.writeFileSync(receiptFile, JSON.stringify({
      waitingReceipt:waitingReceipt.id, waitingId:waiting.id, waitingStatus:waiting.status,
      changedReceipt:editedReceipt.id, changedTask:edited.id,
      expertSession:expertSession.id, expertId:independent.id, expertTurn:expertTurn.id, reportId,
    }, null, 2));
  }
} finally {
  await browser.close();
  if (mockServer) await new Promise(resolve => mockServer.close(resolve));
}
