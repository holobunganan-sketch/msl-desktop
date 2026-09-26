<script lang="ts">
 import AiDocument from './AiDocument.svelte';
 import {aiDocumentText} from '$lib/types/aiDocument';
 import '$lib/styles/knowledge.css';
 import '$lib/styles/qa-chat.css';
 import {onMount, tick} from 'svelte';
 import {locale} from '$lib/i18n';
 import {command,normalizeError,listWorks} from '$lib/services/api';
 import {navigateTo} from '$lib/services/navigation';
 import {aiJobs} from '$lib/stores/aiJobs';
 import {json,mentionAt,addScope,type Project,type Session,type Turn,type Answer,type Pack} from '$lib/services/knowledge';
 import {chatModels,chatModelId,chatEnter,chatDrafts,chatSelection,chatScrollPositions} from '$lib/services/qaChat';
 import type {AiTaskRoute,ProviderConnection,ProviderModel} from '$lib/types/domain';
 import Modal from './ui/Modal.svelte';
 import KnowledgeSources from './KnowledgeSources.svelte';
 import {expertSession} from '$lib/services/workflowContinuity';
 import type {Expert} from '$lib/services/knowledge';
 let {focusId=null,workId=null,expertId=null}:{focusId?:number|null;workId?:number|null;expertId?:number|null}=$props();
 let experts=$state<Expert[]>([]),selectedExpert=$state<number|null>(null),expertScoped=$state(false),expertLabel=$state('');
 const linkedProjects=$derived(json<number[]>(experts.find(e=>e.id===selectedExpert)?.project_ids_json,[]));
 const draftKey=(id:number|null)=>String(id??(expertScoped?`expert:${selectedExpert??'deleted'}`:'new'));
 let en=$derived($locale==='en-US');
 let projects=$state<Project[]>([]),sessions=$state<Session[]>([]),turns=$state<Turn[]>([]);
 let providers=$state<ProviderConnection[]>([]),models=$state<ProviderModel[]>([]),routes=$state<AiTaskRoute[]>([]);
 let sessionId=$state<number|null>(null),scope=$state<number[]>([]),question=$state(''),error=$state('');
 let sessionLoading=$state(false),errorOpen=$state(false);
 let sessionEpoch=0,refreshSequence=0,disposed=false;
 const scrollPositions=chatScrollPositions;
 let scrollRestoring=false;
 let busy=$state(false),savingModel=$state(false),ready=$state(false),removeOpen=$state(false),historyOpen=$state(false);
 let textarea:HTMLTextAreaElement,transcript:HTMLDivElement;
 let caret=$state(0),picker=$state(false),suppressMention=$state(false),highlight=$state(0),followBottom=$state(true),copied=$state<number|null>(null);
 let mention=$derived(suppressMention?null:mentionAt(question,caret));
 let options=$derived(projects.filter(p=>!scope.includes(p.id)&&(!mention?.query||p.title.toLowerCase().includes(mention.query.toLowerCase()))));
 let modelOptions=$derived(chatModels(providers,models)),modelId=$derived(chatModelId(routes));
 let modelReady=$derived(modelOptions.some(m=>m.id===modelId));
 let processing=$derived(turns.some(t=>['running','pending'].includes(t.status)));
 let modelLocked=$derived(busy||savingModel||processing||$aiJobs.some(j=>j.command==='ask_workbench'&&j.status==='running'));
 let canSend=$derived(ready&&!sessionLoading&&modelReady&&!busy&&!processing&&!savingModel&&!!question.trim()&&(!expertScoped||selectedExpert!==null));
 let conversationTitle=$derived(sessions.find(s=>s.id===sessionId)?.title??(en?'New conversation':'新对话'));
 const scopeName=(ids:number[])=>ids.length?ids.map(id=>projects.find(p=>p.id===id)?.title??(en?'Unavailable project':'不可用项目')+' #'+id).join(' · '):expertScoped?(en?'Expert records':'该专家的记录'):en?'All workbench information':'全部工作台信息';
 const modelLabel=(id:number)=>{const m=models.find(m=>m.id===id);return m?(providers.find(p=>p.id===m.provider_id)?.display_name??'')+' · '+m.display_name:(en?'Unavailable model':'不可用模型');};
 function remember(){if(!ready)return;if(sessionId&&transcript&&!sessionLoading&&!scrollRestoring)scrollPositions.set(sessionId,{top:transcript.scrollTop,follow:followBottom});chatDrafts.set(draftKey(sessionId),{question,scope:[...scope],expertId:selectedExpert});chatSelection.sessionId=sessionId;}
 function restore(id:number|null,defaultScope:number[]){const draft=chatDrafts.get(draftKey(id));question=draft?.question??'';scope=[...new Set([...(draft?.scope??defaultScope),...linkedProjects])];caret=question.length;picker=false;suppressMention=true;highlight=0;followBottom=id===null?true:(scrollPositions.get(id)?.follow??true);}
 async function refresh(){if(sessionLoading)return;const id=sessionId,epoch=sessionEpoch,sequence=++refreshSequence;const nextSessions=await command<Session[]>('list_qa_sessions');if(disposed||epoch!==sessionEpoch)return;sessions=nextSessions;if(id){const next=await command<Turn[]>('list_qa_turns',{sessionId:id});if(!disposed&&epoch===sessionEpoch&&id===sessionId&&sequence===refreshSequence)turns=next;}}
 async function select(s:Session){
  remember();const position=scrollPositions.get(s.id);const saved=position?{...position}:undefined;
  const epoch=++sessionEpoch;scrollRestoring=true;sessionId=s.id;sessionLoading=true;selectedExpert=s.expert_id??null;expertScoped=s.expert_scoped===1;expertLabel=s.expert_label??'';restore(s.id,json(s.scope_json,[]));turns=[];error=expertScoped&&selectedExpert===null?(en?'This expert was deleted; previous answers remain readable.':'该专家已删除，历史回答仍可查看。'):'';removeOpen=false;historyOpen=false;
  try{const next=await command<Turn[]>('list_qa_turns',{sessionId:s.id});if(epoch===sessionEpoch&&!disposed)turns=next;}catch(e){if(epoch===sessionEpoch)error=normalizeError(e);}
  finally{if(epoch===sessionEpoch&&!disposed){
   followBottom=saved?.follow??true;sessionLoading=false;await tick();
   const restorePosition=()=>{if(epoch===sessionEpoch&&!disposed&&transcript)transcript.scrollTop=followBottom?transcript.scrollHeight:(saved?.top??0);};
   restorePosition();
   requestAnimationFrame(()=>{if(epoch===sessionEpoch&&!disposed){restorePosition();scrollRestoring=false;remember();}});
  }}
 }
 function fresh(){remember();sessionEpoch++;scrollRestoring=false;sessionLoading=false;sessionId=null;selectedExpert=null;expertScoped=false;expertLabel='';restore(null,[]);turns=[];error='';removeOpen=false;historyOpen=false;void tick().then(()=>textarea?.focus());}
 function pick(p:Project){const selectedMention=mention;scope=addScope(scope,p.id);if(selectedMention){question=question.slice(0,selectedMention.start)+question.slice(selectedMention.end);caret=selectedMention.start;}picker=false;suppressMention=true;highlight=0;void tick().then(()=>{textarea?.focus();textarea?.setSelectionRange(caret,caret);});}
 async function loadModels(){[providers,models,routes]=await Promise.all([command<ProviderConnection[]>('list_provider_connections'),command<ProviderModel[]>('list_provider_models'),command<AiTaskRoute[]>('list_ai_task_routes')]);}
 async function chooseModel(value:string){if(!value||modelLocked)return;savingModel=true;error='';try{await command('save_ai_task_route',{taskKind:'workbench_qa',providerModelId:Number(value)});await loadModels();}catch(e){error=normalizeError(e);}finally{savingModel=false;}}
 async function ask(turn?:Turn){if(!turn&&!canSend)return;error='';busy=true;followBottom=true;const epoch=sessionEpoch;try{let id=turn?.id;if(!id){const sentQuestion=question,sentScope=[...scope],sentExpert=selectedExpert,key=draftKey(sessionId);let target=sessionId;if(!target){const s=await command<Session>('create_qa_session',{title:'',scope:sentScope,expertId:sentExpert});target=s.id;if(epoch===sessionEpoch&&!disposed){sessionId=s.id;scope=json(s.scope_json,sentScope);}}const queued=await command<Turn>('queue_qa_question',{sessionId:target,question:sentQuestion,scope:sentScope,expertId:sentExpert});id=queued.id;if(chatDrafts.get(key)?.question===sentQuestion)chatDrafts.delete(key);if(epoch===sessionEpoch&&!disposed){scope=json(queued.scope_json,sentScope);if(question===sentQuestion){question='';caret=0;}remember();}}await refresh();await command('ask_workbench',{turnId:id,locale:$locale});await refresh();}catch(e){if(epoch===sessionEpoch)error=normalizeError(e);await refresh().catch(()=>{});}finally{busy=false;}}
 async function remove(){try{const id=sessionId;await command('delete_qa_session',{id});chatDrafts.delete(String(id));ready=false;sessionEpoch++;sessionLoading=false;sessionId=null;restore(null,[]);turns=[];removeOpen=false;ready=true;await refresh();}catch(e){error=normalizeError(e);}}
 function onKey(event:KeyboardEvent){if(event.isComposing||event.keyCode===229)return;if(event.key==='Escape'){picker=false;suppressMention=true;return;}if((picker||mention)&&options.length){if(['ArrowDown','ArrowUp'].includes(event.key)){event.preventDefault();highlight=(highlight+(event.key==='ArrowDown'?1:-1)+options.length)%options.length;return;}if(chatEnter(event)){event.preventDefault();pick(options[Math.min(highlight,options.length-1)]);return;}}if(chatEnter(event)){event.preventDefault();if(canSend)void ask();}}
 async function copyAnswer(turn:Turn,answer:Answer){try{const text=turn.document||answer.document ? aiDocumentText((turn.document??answer.document)!) : [...answer.claims.map(c=>(c.basis==='inference'?(en?'Inference: ':'推断：'):'')+c.text),...(answer.gaps.length?[(en?'Uncertain / missing:':'尚不确定 / 资料缺口：'),...answer.gaps]:[])].join('\n\n');await navigator.clipboard.writeText(text);copied=turn.id;}catch{error=en?'Copy failed. You can select and copy the answer text.':'复制未成功，您可以选中回答文字进行复制。';}}
 $effect(()=>{sessionId;question;scope;remember();});
 $effect(()=>{const signature=turns.map(t=>t.id+':'+t.status).join('|');if(sessionLoading||scrollRestoring)return;const epoch=sessionEpoch;if(signature)void tick().then(()=>{if(epoch===sessionEpoch&&!sessionLoading&&!scrollRestoring&&followBottom&&transcript)transcript.scrollTop=transcript.scrollHeight;});});
 onMount(()=>{let active=true;void (async()=>{try{
  await Promise.all([listWorks().then(p=>projects=p),command<Expert[]>('list_kol_experts').then(value=>experts=value),loadModels(),refresh()]);
  if(!active)return;
  if(expertId){
   selectedExpert=experts.some(e=>e.id===expertId)?expertId:null;expertScoped=true;expertLabel=experts.find(e=>e.id===expertId)?.name??(en?'Unavailable expert':'不可用专家');
   const matching=expertSession(sessions,expertId,linkedProjects);
   if(matching)await select(matching);else restore(null,linkedProjects);
   if(!selectedExpert)error=en?'This expert is no longer available.':'该专家已不可用，请返回专家列表核对。';
  }else if(workId){scope=[workId];}else{
   const chosen=focusId??chatSelection.sessionId;
   const s=chosen===null?undefined:sessions.find(s=>s.id===chosen)??(focusId?undefined:sessions[0]);
   if(s)await select(s);else restore(null,[]);
  }
  ready=true;
 }catch(e){error=normalizeError(e);}})();
 const timer=setInterval(()=>{void refresh().catch(()=>{});},2200);
 return()=>{remember();active=false;disposed=true;sessionEpoch++;clearInterval(timer);};});
</script>

<section class="knowledge-page qa-chat" data-testid="qa-page">
 <header class="qa-heading"><div><h1>{en?'Ask your workbench':'工作台问答'}</h1><p class="muted">{en?'A conversation with your work, grounded in your records.':'和您的工作聊一聊，让已有记录带来清晰答案。'}</p></div><span class="qa-readonly">{en?'Read-only · Evidence-backed':'只读问答 · 有据可查'}</span></header>
 <div class="qa-layout">
  <aside class="qa-history" class:open={historyOpen} aria-label={en?'Conversation history':'对话记录'}>
   <button class="qa-new" data-testid="qa-new" onclick={fresh}>＋ {en?'New conversation':'新对话'}</button>
   <div class="qa-history-label">{en?'Recent conversations':'最近对话'}<button class="qa-history-close" onclick={()=>historyOpen=false} aria-label={en?'Close history':'关闭对话记录'}>×</button></div>
   <div class="qa-sessions">{#each sessions as s(s.id)}<button class:active={s.id===sessionId} data-testid={'qa-session-'+s.id} onclick={()=>select(s)} aria-current={s.id===sessionId?'true':undefined}><strong>{s.title}</strong><small>{new Date(s.updated_at*1000).toLocaleDateString($locale)}</small></button>{:else}<p class="muted">{en?'Your conversations will appear here.':'聊过的内容会保存在这里。'}</p>{/each}</div>
   <p class="qa-history-note">{en?'Keep asking. Each conversation keeps its own context.':'随时回来接着问，每段对话保留自己的上下文。'}</p>
  </aside>
  <main class="qa-main">
   <div class="qa-toolbar">
    <button class="qa-history-toggle" onclick={()=>historyOpen=!historyOpen} aria-expanded={historyOpen}>{historyOpen?(en?'Close history':'收起记录'):(en?'History':'对话记录')}</button>
    <div class="qa-conversation-title" title={conversationTitle}>{conversationTitle}</div><div class="qa-model-field"><label for="qa-model">{en?'Chat model':'问答模型'}</label><select id="qa-model" data-testid="qa-model" value={modelId??''} disabled={modelLocked} onchange={e=>chooseModel(e.currentTarget.value)} aria-describedby="qa-model-hint">{#if !modelReady}<option value={modelId??''}>{modelId?modelLabel(modelId)+' · '+(en?'Unavailable':'不可用'):(en?'Select a model':'请选择模型')}</option>{/if}{#each modelOptions as model(model.id)}<option value={model.id}>{modelLabel(model.id)}</option>{/each}</select><small id="qa-model-hint">{savingModel?(en?'Saving…':'正在保存…'):modelLocked?(en?'Switch models after this answer finishes.':'本轮完成后可切换模型。'):(en?'Saved for Q&A only':'仅用于问答，自动保存')}</small></div>
    <div class="qa-tools"><button class="qa-mobile-new" onclick={fresh} aria-label={en?'New conversation':'新对话'}>＋</button>{#if sessionId}<button onclick={()=>removeOpen=!removeOpen} disabled={busy||processing} aria-expanded={removeOpen}>{en?'Delete chat':'删除对话'}</button>{/if}</div>
   </div>
   <Modal bind:open={removeOpen} title={en?'Delete conversation':'删除对话'}><p>{en?'Project and work records will be retained.':'项目和工作记录会完整保留。'}</p>{#snippet footer()}<button onclick={()=>removeOpen=false}>{en?'Cancel':'取消'}</button><button onclick={remove}>{en?'Confirm deletion':'确认删除'}</button>{/snippet}</Modal>
   <!-- Keyboard focus allows Page Up/Down to scroll the independently bounded conversation. -->
   <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
   <div class="qa-transcript" data-testid="qa-transcript" bind:this={transcript} onscroll={(event)=>{if(!disposed&&!sessionLoading&&!scrollRestoring){const el=event.currentTarget;followBottom=el.scrollHeight-el.scrollTop-el.clientHeight<90;if(sessionId)scrollPositions.set(sessionId,{top:el.scrollTop,follow:followBottom});}}} role="region" aria-label={conversationTitle} tabindex="0">
    {#if sessionLoading||!ready}<div class="qa-loading" role="status" data-testid="qa-loading"><span>{en?'Loading conversation…':'正在读取对话…'}</span><i></i><i></i><i></i></div>{:else if !turns.length}<div class="qa-welcome"><div class="qa-spark" aria-hidden="true">✦</div><h2>{en?'What would you like to work through?':'今天，想把哪件事想清楚？'}</h2><p>{en?'Ask about progress, prepare for a conversation, or connect the dots across projects.':'梳理进展、准备下一次交流，或找出项目之间的关联。'}</p><div class="qa-starters">{#each (en?['Which projects need my attention?','Help me prepare for my next meeting.','What is blocking progress this week?']:['哪些项目需要我优先跟进？','帮我准备下一次专家交流。','本周有哪些事项卡住了？']) as prompt}<button onclick={()=>{question=prompt;textarea?.focus();}}>{prompt}<span aria-hidden="true">↗</span></button>{/each}</div><small>{en?'Type @ to focus on one or more projects.':'输入 @，可以聚焦一个或多个项目。'}</small></div>{/if}
    {#each turns as turn(turn.id)}
     {@const answer=json<Answer|null>(turn.answer_json,null)}{@const pack=json<Pack|null>(turn.evidence_json,null)}
     <article class="qa-turn" data-testid={'qa-turn-'+turn.id}>
      <div class="qa-user"><div class="qa-user-text">{turn.question}</div><small>{scopeName(json(turn.scope_json,[]))} · {new Date(turn.created_at*1000).toLocaleTimeString($locale,{hour:'2-digit',minute:'2-digit'})}</small></div>
      <div class="qa-assistant"><div class="qa-avatar" aria-hidden="true">✦</div><div class="qa-answer-body"><strong class="qa-speaker">{en?'Workbench assistant':'工作台助手'}</strong>
       {#if turn.document||answer?.document}<div data-testid="qa-answer"><AiDocument document={(turn.document??answer?.document)!} {pack}/></div>{:else if answer}<div data-testid="qa-answer">{#each answer.claims as claim}<section class="qa-claim">{#if claim.basis==='inference'}<span class="qa-inference">{en?'Inference':'分析建议 · 推断'}</span>{/if}<p>{claim.text}</p><KnowledgeSources citations={claim.citations} {pack}/></section>{/each}{#if answer.gaps.length}<details class="qa-gaps" data-testid="qa-gaps"><summary>{en?'Still to clarify':'还需要确认'} · {answer.gaps.length}</summary><ul>{#each answer.gaps as gap}<li>{gap}</li>{/each}</ul></details>{/if}</div>
       {:else if turn.error}<div class="k-error">{turn.error}</div>{:else}<p class="qa-working" role="status">{['running','pending'].includes(turn.status)?(en?'Checking your records and preparing an answer. You can leave this page.':'正在结合工作记录思考，切换页面后会继续完成。'):(en?'This question was interrupted. You can retry it.':'此轮问答已中断，可以重新发起。')}</p>{/if}
       <div class="qa-answer-actions">{#if answer}<button onclick={()=>copyAnswer(turn,answer)}>{copied===turn.id?(en?'Copied':'已复制'):(en?'Copy answer':'复制回答')}</button>{/if}{#if ['failed','interrupted','pending'].includes(turn.status)}<button disabled={busy||!modelReady} onclick={()=>ask(turn)}>{en?'Retry':'重试本轮'}</button>{/if}{#if pack}<details class="qa-coverage"><summary>{en?'Retrieval scope':'检索范围'} · {pack.sources.length} {en?'sources':'条依据'}</summary><p>{en?'Snapshot':'数据截至'}：{new Date(pack.as_of*1000).toLocaleString($locale)}</p><p>{en?'Relevant excerpts were selected. Omitted candidates':'按相关性选取片段，未纳入候选记录'}：{pack.omitted}</p>{#each pack.notes as note}<p>{note}</p>{/each}<p>{en?'Sources support verification; interpretations still need judgment.':'可展开依据核对原始记录；推断仍需结合实际情况判断。'}</p></details>{/if}</div>
      </div></div>
     </article>
    {/each}
   </div>
   <div class="qa-composer-wrap" data-testid="qa-composer">
    <div class="qa-status-line" aria-live="polite">{#if error}<button class="qa-status-error" title={error} onclick={()=>errorOpen=true}>{error}</button>{:else if !modelReady&&ready}<button onclick={()=>navigateTo('settings')}>{en?'Select a model above, or configure one in Settings.':'请在上方选择模型；点击前往设置配置接口。'}</button>{/if}</div>
    <Modal bind:open={errorOpen} title={en?'Error details':'错误详情'}><p style="white-space:pre-wrap;overflow-wrap:anywhere">{error}</p></Modal>
    <div class="qa-composer-box">
     <div class="qa-scope-tags" data-testid="qa-scope">{#if expertScoped}<small data-testid="qa-expert-scope">{en?'Expert: ':'聚焦专家：'}{expertLabel}{selectedExpert?'':(en?' · Deleted':' · 已删除')}</small>{/if}{#each scope as id(id)}<button title={scopeName([id])} disabled={expertScoped&&linkedProjects.includes(id)} onclick={()=>scope=scope.filter(x=>x!==id)} aria-label={(en?'Remove focus: ':'移除聚焦：')+scopeName([id])}>@ {scopeName([id])}{expertScoped&&linkedProjects.includes(id)?'':' ×'}</button>{/each}{#if scope.length&&!expertScoped}<button onclick={()=>scope=[]}>{en?'All information':'恢复全局'}</button>{:else if !scope.length}<small>{expertScoped?(en?'This expert’s records':'当前范围：该专家记录'):(en?'Searching all workbench information':'当前范围：全部工作台信息')}</small>{/if}</div>
     {#if picker||mention}<div class="qa-project-picker" data-testid="qa-project-options"><div class="qa-picker-heading"><strong>{en?'Focus a project':'聚焦项目'}</strong><button onclick={()=>{picker=false;suppressMention=true;}} aria-label={en?'Close project picker':'关闭项目选择'}>×</button></div>{#each options as p,i(p.id)}<button class:highlighted={i===highlight} data-testid={'qa-project-'+p.id} onclick={()=>pick(p)}>{p.title}{p.status==='archived'?' · '+(en?'Archived':'已归档'):''}</button>{:else}<p>{en?'No matching projects':'没有匹配的项目'}</p>{/each}</div>{/if}
     <textarea data-testid="qa-question" bind:this={textarea} bind:value={question} maxlength="6000" rows="2" placeholder={en?'Ask anything about your work. Use @ to focus a project.':'有什么想了解的，直接问。输入 @ 聚焦项目。'} aria-label={en?'Your question':'输入问题'} oninput={e=>{caret=e.currentTarget.selectionStart;suppressMention=false;highlight=0;copied=null;}} onclick={e=>caret=e.currentTarget.selectionStart} onkeyup={e=>caret=e.currentTarget.selectionStart} onkeydown={onKey}></textarea>
     <div class="qa-composer-actions"><button class="qa-focus" onclick={()=>{picker=!picker;suppressMention=true;highlight=0;}} aria-expanded={picker}>@ {scope.length||expertScoped?(en?'Add project':'添加项目'):(en?'All information':'全部工作信息')}</button><span class="qa-enter-hint">{en?'Enter to send · Shift + Enter for a new line':'Enter 发送 · Shift + Enter 换行'}</span><button class="primary qa-send" data-testid="qa-send" disabled={!canSend} onclick={()=>ask()} aria-label={en?'Send question':'发送问题'}>{busy||processing?(en?'Thinking…':'思考中…'):(en?'Send ↑':'发送 ↑')}</button></div>
    </div>
    <div class="qa-composer-footnote">{en?'Answers use workbench records. Check the sources for important decisions.':'回答基于工作台记录，重要判断请核对依据。'}</div>
   </div>
  </main>
 </div>
</section>
