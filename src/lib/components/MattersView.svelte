<script lang="ts">
  import {locale} from '$lib/i18n';
  import {navigateTo,type MatterSection} from '$lib/services/navigation';
  import InboxView from './InboxView.svelte';
  import AiReviewCenter from './AiReviewCenter.svelte';
  import PlanView from './PlanView.svelte';
  import WaitingView from './WaitingView.svelte';
  import RecentCompletions from './RecentCompletions.svelte';
  let {section='inbox',focusId=null,workId=null,runId=null}:{section?:MatterSection;focusId?:number|null;workId?:number|null;runId?:number|null}=$props();
  function stage(next:MatterSection){navigateTo({view:'matters',section:next,...(workId?{workId}:{})});}
  let completedType=$state('task');
  const en=$derived($locale==='en-US');
</script>
<div class="matters-view">
  <nav class="matter-tabs" data-testid="matters-tabs" aria-label={en?'Work stages':'事项阶段'}>
    <button data-testid="nav-inbox" class:active={section==='inbox'||section==='review'} onclick={()=>stage('inbox')}>{en?'To organize':'待整理'}</button>
    <button data-testid="nav-plan" class:active={section==='plan'} onclick={()=>stage('plan')}>{en?'To advance':'待推进'}</button>
    <button data-testid="nav-waiting" class:active={section==='waiting'} onclick={()=>stage('waiting')}>{en?'Waiting':'等待中'}</button>
    <button data-testid="nav-done" class:active={section==='done'} onclick={()=>stage('done')}>{en?'Completed':'已完成'}</button>
  </nav>
  {#if section==='inbox'||section==='review'}
    <div class="queue-switch"><button data-testid="queue-captures" class:active={section==='inbox'} onclick={()=>stage('inbox')}>{en?'Captured notes':'刚记下的内容'}</button><button data-testid="nav-review" class:active={section==='review'} onclick={()=>stage('review')}>{en?'Secretary suggestions':'秘书准备的建议'}</button></div>
  {/if}
  {#if section==='review'}<AiReviewCenter {focusId} {workId} {runId}/>
  {:else if section==='inbox'}<InboxView {focusId}/>
  {:else if section==='plan'}<PlanView {focusId} {workId}/>
  {:else if section==='waiting'}<WaitingView {focusId} {workId}/>
  {:else}
    <div class="queue-switch"><button class:active={completedType==='task'} onclick={()=>completedType='task'}>{en?'Completed tasks':'做完的事项'}</button><button class:active={completedType==='waiting'} onclick={()=>completedType='waiting'}>{en?'Resolved waiting':'已结束的等待'}</button></div>
    <RecentCompletions/>
    {#if completedType==='task'}<PlanView mode="done" {workId}/>{:else}<WaitingView mode="done" {workId}/>{/if}
  {/if}
</div>
<style>
  .matters-view{display:grid;gap:20px;min-width:0}
  .matter-tabs{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:5px;padding:5px;background:var(--color-surface);border-radius:10px;border:1px solid var(--color-border)}
  button{padding:10px 16px;min-height:42px;font-size:15px;border:0;border-radius:7px;background:transparent;color:var(--color-muted);cursor:pointer}.matter-tabs .active{background:var(--color-primary-soft);color:var(--color-primary);font-weight:650}
  .queue-switch{display:flex;flex-wrap:wrap;gap:8px}.queue-switch button{font-size:14px;padding:7px 14px;min-height:36px;border:1px solid var(--color-border)}.queue-switch .active{background:var(--color-primary-soft);color:var(--color-primary)}
</style>
