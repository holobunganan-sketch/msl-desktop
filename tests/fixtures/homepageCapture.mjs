// In-memory native boundary for the real application UI. No files or providers.
import '/scripts/ui-design-fixture.mjs';
import {mount} from 'svelte';
import App from '/src/routes/+page.svelte';

const original=window.__TAURI_INTERNALS__.invoke;
const calls=[];
const jobs=[];
const shortcuts=new Set();
const listeners=new Map();
let holdCapture=false, releaseCapture;
const inferredTime= new URLSearchParams(location.search).has('inferred-time');
const inferredDrafts=new Map();
window.__CAPTURE_TEST__={
  calls, jobs,
  holdCapture(){holdCapture=true;},
  releaseCapture(){holdCapture=false;releaseCapture?.();},
  finishJobs(){for(const job of jobs){job.status='completed';job.result=1;job.finished_at=Math.floor(Date.now()/1000);}},
  shortcutCount(){return shortcuts.size;},
  captureListenerCount(){return [...listeners.values()].filter(name=>name==='quick-capture').length;}
};
window.__TAURI_INTERNALS__.invoke=async(name,args={})=>{
  calls.push({name,args:JSON.parse(JSON.stringify(args))});
  if(name==='plugin:global-shortcut|register'){
    for(const shortcut of args.shortcuts){if(shortcuts.has(shortcut))throw new Error('Duplicate shortcut');shortcuts.add(shortcut);}
    return null;
  }
  if(name==='plugin:global-shortcut|unregister'){for(const shortcut of args.shortcuts)shortcuts.delete(shortcut);return null;}
  if(name==='plugin:event|listen'){const id=await original(name,args);listeners.set(id,args.event);return id;}
  if(name==='plugin:event|unlisten'){listeners.delete(args.eventId);return null;}
  if(name==='capture_work_note'&&holdCapture)await new Promise(resolve=>releaseCapture=resolve);
  if(name==='start_ai_job'&&['run_analysis_now','organize_inbox_item'].includes(args.request.command)){
    const job={id:100+jobs.length,command:args.request.command,args:args.request.args,status:'running',result:null,error:null,created_at:Math.floor(Date.now()/1000),finished_at:null};
    jobs.push(job);return structuredClone(job);
  }
  if(name==='get_ai_job'&&jobs.some(job=>job.id===args.id))return structuredClone(jobs.find(job=>job.id===args.id));
  if(name==='list_ai_jobs')return [...await original(name,args),...structuredClone(jobs)];
  if(['list_latest_analysis_proposals','list_ai_proposals','list_recent_ai_proposals'].includes(name)&&inferredTime){
    const proposals=await original(name,args);
    if(proposals[0]){
      if(!inferredDrafts.has(proposals[0].id))inferredDrafts.set(proposals[0].id,JSON.stringify({...JSON.parse(proposals[0].payload_json),due_at:Math.floor(Date.now()/1000)+86400,time_basis:'inferred',time_reason:'演示：尚未得到用户确认'}));
      proposals[0].payload_json=inferredDrafts.get(proposals[0].id);
    }
    return proposals;
  }
  if(name==='update_ai_proposal_classification'&&inferredTime){
    const row=await original(name,args);inferredDrafts.set(row.id,row.payload_json);return row;
  }
  if(name==='confirm_ai_proposal'&&inferredTime){
    if(args.editedPayload?.time_confirmation!=='user_confirmed')throw new Error('Tentative time requires explicit user confirmation');
    const rows=await original('list_ai_proposals',{status:'pending'});
    const row=rows.find(row=>row.id===args.id);row.status='confirmed';row.updated_at++;
    return {proposal_id:row.id,kind:row.kind,target_id:1,receipt_id:'synthetic-receipt'};
  }
  return original(name,args);
};
mount(App,{target:document.getElementById('fixture-root')});
