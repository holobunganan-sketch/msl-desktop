<script lang="ts">
  import {locale} from '$lib/i18n';
  import {navigateTo,type MatterSection} from '$lib/services/navigation';
  import InboxView from './InboxView.svelte';
  import AiReviewCenter from './AiReviewCenter.svelte';
  import PlanView from './PlanView.svelte';
  import WaitingView from './WaitingView.svelte';
  let {section='inbox',focusId=null}:{section?:MatterSection;focusId?:number|null}=$props();
  let completedType=$state('task');
  const en=$derived($locale==='en-US');
  const guide=$derived(({
    inbox: en?['01','Capture first','Write what happened. Ask the secretary to organize when ready.']:['01','先记下发生了什么','原话会完整保留。需要整理时交给秘书，不必先填分类和字段。'],
    review: en?['02','Decide what happens next','Check the change, project and time. Adopt, adjust, or leave it for later.']:['02','只确认下一步怎样处理','先看改什么、归哪个项目、何时推进；可以采用、调整，也可以稍后再说。'],
    plan: en?['03','Advance the next step','Work from this list. Confirmed appointments and deadlines appear in Calendar.']:['03','照着下一步推进','完成后点完成，有变化就记一句；已确认的预约和截止时间会进入日历。'],
    waiting: en?['03','Keep follow-ups in view','Record who or what you are waiting for. Follow-up dates appear in Calendar.']:['03','把等待交给跟进提醒','知道在等谁、等什么、何时再问即可。跟进日期自动出现在日历中。'],
    done: en?['04','Keep progress without writing twice','Completed items remain available for review and period reports.']:['04','做完就留下进展','已完成事项会保留在这里，供秘书回顾和生成周报，无需再抄写一遍。']
  })[section]);
</script>
<div class="matters-view">
  <nav class="matter-tabs" data-testid="matters-tabs" aria-label={en?'Work stages':'事项阶段'}>
    <button data-testid="nav-inbox" class:active={section==='inbox'||section==='review'} onclick={()=>navigateTo('inbox')}>{en?'To organize':'待整理'}</button>
    <button data-testid="nav-plan" class:active={section==='plan'} onclick={()=>navigateTo('plan')}>{en?'To advance':'待推进'}</button>
    <button data-testid="nav-waiting" class:active={section==='waiting'} onclick={()=>navigateTo('waiting')}>{en?'Waiting':'等待中'}</button>
    <button data-testid="nav-done" class:active={section==='done'} onclick={()=>navigateTo('done')}>{en?'Completed':'已完成'}</button>
  </nav>
  <section class="stage-guide" data-testid="stage-guide" aria-label={en?'Next step':'这一步怎么做'}><span>{guide[0]}</span><div><strong>{guide[1]}</strong><p>{guide[2]}</p></div></section>
  {#if section==='inbox'||section==='review'}
    <div class="queue-switch"><button data-testid="queue-captures" class:active={section==='inbox'} onclick={()=>navigateTo('inbox')}>{en?'Captured notes':'刚记下的内容'}</button><button data-testid="nav-review" class:active={section==='review'} onclick={()=>navigateTo('review')}>{en?'Secretary suggestions':'秘书准备的建议'}</button></div>
  {/if}
  {#if section==='review'}<AiReviewCenter {focusId}/>
  {:else if section==='inbox'}<InboxView {focusId}/>
  {:else if section==='plan'}<PlanView {focusId}/>
  {:else if section==='waiting'}<WaitingView {focusId}/>
  {:else}
    <div class="queue-switch"><button class:active={completedType==='task'} onclick={()=>completedType='task'}>{en?'Completed tasks':'做完的事项'}</button><button class:active={completedType==='waiting'} onclick={()=>completedType='waiting'}>{en?'Resolved waiting':'已结束的等待'}</button></div>
    {#if completedType==='task'}<PlanView mode="done"/>{:else}<WaitingView mode="done"/>{/if}
  {/if}
</div>
<style>
  .stage-guide{display:flex;gap:14px;align-items:flex-start;padding:16px 18px;min-width:0;border-left:3px solid var(--color-border-strong);background:var(--color-surface)}.stage-guide>span{color:var(--color-primary);background:var(--color-primary-soft);width:36px;height:36px;display:grid;place-items:center;flex-shrink:0;border-radius:50%;font-size:14px;font-weight:600}.stage-guide strong{font-size:16px}.stage-guide p{margin:5px 0 0;line-height:1.65;font-size:14px;color:var(--color-muted);overflow-wrap:anywhere}
  .matters-view{display:grid;gap:20px;min-width:0}
  .matter-tabs{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:5px;padding:5px;background:var(--color-surface);border-radius:10px;border:1px solid var(--color-border)}
  button{padding:10px 16px;min-height:42px;font-size:15px;border:0;border-radius:7px;background:transparent;color:var(--color-muted);cursor:pointer}.matter-tabs .active{background:var(--color-primary-soft);color:var(--color-primary);font-weight:650}
  .queue-switch{display:flex;flex-wrap:wrap;gap:8px}.queue-switch button{font-size:14px;padding:7px 14px;min-height:36px;border:1px solid var(--color-border)}.queue-switch .active{background:var(--color-primary-soft);color:var(--color-primary)}
</style>
