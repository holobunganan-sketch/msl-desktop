<script lang="ts">
  import {locale} from '$lib/i18n';
  import {command} from '$lib/services/api';
  import {invalidate} from '$lib/stores/dataRevision';
  import type {AiProposal} from '$lib/types/domain';
  import Modal from './ui/Modal.svelte';
  import AppButton from './ui/AppButton.svelte';

  let {item=null,action='delete',onclose,oncomplete,onbusychange=()=>{}}: {
    item?:AiProposal|null;
    action?:'delete'|'resolve';
    onclose:()=>void;
    oncomplete:(action:'delete'|'resolve',id:number)=>void|Promise<void>;
    onbusychange?:(busy:boolean)=>void;
  }=$props();
  let busy=$state(false),error=$state('');
  const en=$derived($locale==='en-US');
  $effect(()=>{item?.id;action;error='';});

  async function apply(){
    const current=item;if(!current||busy)return;
    busy=true;error='';onbusychange(true);
    try{
      await command(action==='delete'?'delete_ai_proposal':'resolve_ai_proposal',{
        id:current.id,expectedUpdatedAt:current.updated_at
      });
      invalidate('works','tasks','waiting','calendar','inbox','proposals','brief','analysis');
      await oncomplete(action,current.id);
      onclose();
    }catch(cause){error=String(cause instanceof Error?cause.message:cause);}
    finally{busy=false;onbusychange(false);}
  }
</script>

<Modal open={item!==null} title={action==='delete'?(en?'Delete this review record?':'删除这条审阅记录？'):(en?'Mark this insight as resolved?':'将这条洞察标记为已解决？')} dismissible={!busy} {onclose}>
  <div class="lifecycle-copy">
    <strong>{item?.title}</strong>
    {#if action==='delete'}
      <p>{en?'This record will be removed from the review list and will no longer be followed up. Tasks, waiting items and calendar entries already created from it will remain.':'这条记录将从审阅列表移除，秘书不再跟进。已采用生成的任务、等待事项和日历会保留。'}</p>
      <p>{en?'The project and original work files will remain unchanged.':'项目及原始工作文件保持原样。'}</p>
    {:else}
      <p>{en?'The insight will stay in history and will no longer be treated as an open issue in future project analysis.':'这条洞察会保留在历史中，后续项目分析不再将它视为待解决问题。'}</p>
      <p>{en?'An associated task or waiting item will also be completed. Other records and original files will remain unchanged.':'关联的任务或等待事项将同步完成；其他记录和原始文件保持原样。'}</p>
    {/if}
    {#if error}<p class="lifecycle-error" role="alert">{error}</p>{/if}
  </div>
  {#snippet footer()}
    <AppButton testid="proposal-lifecycle-cancel" variant="secondary" disabled={busy} onclick={onclose}>{en?'Cancel':'取消'}</AppButton>
    <AppButton testid="proposal-lifecycle-confirm" variant={action==='delete'?'danger':'primary'} loading={busy} onclick={apply}>{action==='delete'?(en?'Delete record':'确认删除'):(en?'Mark resolved':'确认已解决')}</AppButton>
  {/snippet}
</Modal>

<style>
  .lifecycle-copy{display:grid;gap:12px;min-width:0;font-size:1rem;line-height:1.7;overflow-wrap:anywhere}
  .lifecycle-copy strong{font-weight:650;color:var(--color-text)}
  .lifecycle-copy p{margin:0;color:var(--color-muted)}
  .lifecycle-copy .lifecycle-error{padding:12px;border-radius:var(--radius-sm);background:var(--color-danger-soft);color:var(--color-danger)}
</style>
