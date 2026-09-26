<script lang="ts">
  import Icon, { type IconName } from './ui/Icon.svelte';
  import { navigateTo } from '$lib/services/navigation';
  let { en = false, counts }: {
    en?: boolean;
    counts: { projects: number | null; tasks: number | null; waiting: number | null; calendar: number | null; inbox: number | null; decisions: number | null };
  } = $props();
  const items = $derived([
    { id: 'projects', label: en ? 'Projects' : '项目', hint: en ? 'Active projects' : '长期跟进', icon: 'work', target: 'works', tone: 'blue', value: counts.projects },
    { id: 'tasks', label: en ? 'Tasks' : '任务计划', hint: en ? 'Next actions' : '待推进的事项', icon: 'check', target: 'plan', tone: 'teal', value: counts.tasks },
    { id: 'waiting', label: en ? 'Follow-ups' : '等待事项', hint: en ? 'Due for follow-up' : '今天需要跟进', icon: 'clock', target: 'waiting', tone: 'amber', value: counts.waiting },
    { id: 'calendar', label: en ? 'Calendar' : '日历', hint: en ? 'Today’s arrangements' : '今天的安排', icon: 'calendar', target: 'calendar', tone: 'violet', value: counts.calendar },
    { id: 'inbox', label: en ? 'Inbox' : '收件箱', hint: en ? 'Notes to organize' : '待整理的记录', icon: 'inbox', target: 'inbox', tone: 'rose', value: counts.inbox },
    { id: 'decisions', label: en ? 'Decisions' : '秘书建议', hint: en ? 'Awaiting your review' : '等待您确认', icon: 'review', target: 'review', tone: 'cyan', value: counts.decisions }
  ]);
</script>

<nav class="overview-grid" aria-label={en ? 'Work overview' : '工作概览'}>
  {#each items as item (item.id)}
    <button class="overview-tile" data-testid={`overview-${item.id}`} onclick={() => navigateTo(item.target)}>
      <span class="overview-icon {item.tone}"><Icon name={item.icon as IconName} size={21}/></span>
      <span class="overview-label">{item.label}</span>
      <strong class="overview-value" aria-label={item.value === null ? (en ? 'Loading' : '正在加载') : undefined}>{item.value ?? '—'}</strong>
      <small>{item.hint}</small>
    </button>
  {/each}
</nav>

<style>
  .overview-grid{display:grid;grid-template-columns:repeat(6,minmax(0,1fr));gap:12px;min-width:0}
  .overview-tile{display:grid;grid-template-columns:minmax(0,1fr) auto;grid-template-rows:24px auto auto;align-items:center;gap:5px 8px;min-width:0;padding:13px 17px;text-align:left;background:var(--color-surface);color:var(--color-text);border:1px solid var(--color-border);border-radius:10px;box-shadow:var(--shadow-sm);cursor:pointer;transition:border-color .15s,background .15s}
  .overview-tile:hover{border-color:var(--color-border-strong);background:var(--color-surface-muted)}
  .overview-icon{grid-column:1/-1;display:flex;align-items:center;color:var(--color-primary)}
  .blue{color:#527ccd}.teal{color:#168b84}.amber{color:#ae7826}.violet{color:#8772c6}.rose{color:#c26d79}.cyan{color:#287f99}
  .overview-label{font-size:14px;font-weight:600;line-height:1.5;overflow-wrap:anywhere}
  .overview-value{font-size:22px;line-height:1.3;font-weight:650;font-variant-numeric:tabular-nums;min-width:1.2em;text-align:right}
  small{grid-column:1/-1;color:var(--color-muted);font-size:12px;line-height:1.5}
  @container(max-width:980px){.overview-grid{grid-template-columns:repeat(3,minmax(0,1fr))}.overview-tile{grid-template-rows:auto auto;padding:12px 15px}.overview-icon{grid-column:auto;grid-row:1/3;width:28px}.overview-tile{grid-template-columns:28px minmax(0,1fr) auto}.overview-label{grid-column:2}.overview-value{grid-column:3;grid-row:1/3}.overview-tile small{grid-column:2}}
  @container(max-width:490px){.overview-grid{grid-template-columns:repeat(2,minmax(0,1fr))}}
</style>
