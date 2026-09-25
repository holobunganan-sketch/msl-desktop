<script lang="ts">
  import {invoke} from '@tauri-apps/api/core';
  import {locale} from '$lib/i18n';
  import {dataRevision,invalidate} from '$lib/stores/dataRevision';
  import {startWorkspaceWorkDraft,runAnalysisNow} from '$lib/services/api';
  import {navigateTo} from '$lib/services/navigation';
  import Modal from './ui/Modal.svelte';
  import AppButton from './ui/AppButton.svelte';
  let {workId=null,proposalId=null}:{workId?:number|null;proposalId?:number|null}=$props();
  type Round={scope:string;state:string;pending:number;last_run:number|null;epoch:number};
  let rounds=$state<Round[]>([]),busy=$state(false),error=$state(''),selected=$state<Round|null>(null);
  const en=$derived($locale==='en-US');
  $effect(()=>{const w=workId,p=proposalId;void $dataRevision;let active=true;
    void invoke<Round[]>('secretary_round_status',{workId:w,proposalId:p}).then(r=>{if(active)rounds=r;}).catch(e=>{if(active)error=String(e);});
    return()=>{active=false;};
  });
  const labels:Record<string,[string,string]>={ready:['已有进展，可以分析下一步','Ready for the next step'],running:['秘书正在整理本轮内容','This round is being prepared'],waiting_review:['等待您处理本轮建议','Waiting for your decisions'],waiting_progress:['等待您推进事项','Waiting for work to advance']};
  async function complete(){
    const r=selected;if(!r||busy)return;busy=true;error='';
    try {
      await invoke('complete_secretary_round',{scope:r.scope,expectedEpoch:r.epoch});selected=null;
      invalidate('proposals','works','brief');
      if(r.scope.startsWith('work:'))await startWorkspaceWorkDraft(null,Number(r.scope.slice(5)));
      else await runAnalysisNow('manual');
      invalidate('analysis','proposals','works','brief');
    }catch(e){error=String(e);}finally{busy=false;}
  }
</script>

{#if rounds.length}
  <section class="round" data-testid="secretary-round" aria-label={en?'Secretary pace':'秘书跟进节奏'}>
    {#each rounds as r(r.scope)}
      <div class="round-row"><div class="round-copy"><strong>{(labels[r.state]??labels.ready)[en?1:0]}</strong><p>{en?'Accepting suggestions keeps them on hold. After you finish or record progress, the secretary can prepare the next step.':'采用建议后，秘书会继续等待；您完成事项或记下进展后，再整理下一步。文件变化会留待下一轮分析。'}</p></div>
      <div class="round-actions">
        {#if workId&&r.pending>0}<button onclick={()=>navigateTo({view:'matters',section:'review'})}>{en?'Review suggestions':'查看建议'} · {r.pending}</button>{/if}
        {#if r.state==='waiting_review'||r.state==='waiting_progress'}<button data-testid="secretary-complete-round" disabled={busy} onclick={()=>selected={...r}}>{en?'This round is complete':'本轮已完成'}</button>{/if}
      </div></div>
    {/each}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </section>
{/if}
<Modal open={selected!==null} title={en?'Finish this round?':'完成本轮，并分析下一步？'} onclose={()=>selected=null} dismissible={!busy}>
  <p>{en?'Any undecided suggestions in this round will remain in history as completed. Accepted tasks and appointments keep their current status; no files or records will be deleted.':'本轮尚未处理的建议会保留在历史中，标记为“本轮已完成”。已采用的任务和日历保持原状态，不会删除任何文件或记录。'}</p>
  <p>{en?'The secretary will use accumulated information for one new analysis.':'确认后，秘书将结合积累的信息分析一次下一步。'}</p>
  {#snippet footer()}<AppButton variant="secondary" onclick={()=>selected=null} disabled={busy}>{en?'Keep waiting':'继续等待'}</AppButton><AppButton testid="secretary-complete-confirm" onclick={complete} loading={busy}>{en?'Complete and analyse':'完成并分析下一步'}</AppButton>{/snippet}
</Modal>
<style>
  .round{margin:14px 0;padding:16px 18px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--color-surface-muted);min-width:0}
  .round-row{display:flex;flex-wrap:wrap;align-items:center;gap:14px}.round-copy{flex:1 1 280px;min-width:0}.round-copy strong{font-size:1rem}.round-copy p{margin:6px 0 0;color:var(--color-muted);font-size:.92rem;line-height:1.7;overflow-wrap:anywhere}.round-actions{display:flex;flex-wrap:wrap;gap:10px;flex:0 1 auto}.round-actions button{min-height:42px;padding:9px 14px;border:1px solid var(--color-border);border-radius:var(--radius-sm);background:var(--color-surface);color:var(--color-text);font:inherit;cursor:pointer}.error{color:var(--color-danger);overflow-wrap:anywhere}
</style>
