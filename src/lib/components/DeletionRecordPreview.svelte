<script lang="ts">
 import {locale,translateStatus} from '$lib/i18n';
 import type {DeletableRecord} from '$lib/services/manualActions';
 let {record,works=[]}:{record:DeletableRecord;works?:{id:number;title:string}[]}=$props();
 const en=$derived($locale==='en-US');
 const dates=[['due_at','截止时间','Deadline'],['scheduled_start','安排开始','Scheduled start'],['scheduled_end','安排结束','Scheduled end'],['follow_up_at','跟进时间','Follow-up'],['started_at','等待开始','Waiting since']] as const;
 const date=(value:number|null|undefined)=>value?new Date(value*1000).toLocaleString($locale):(en?'Not set':'未设置');
</script>
<section class="delete-preview" data-testid="delete-record-preview">
 <h3>{record.title}</h3>
 <dl>
  <dt>{en?'Project':'归属项目'}</dt><dd>{record.work_id?works.find(w=>w.id===record.work_id)?.title??(en?'Linked project unavailable':'关联项目暂不可用'):(en?'Independent item':'独立事项')}</dd>
  {#if record.status}<dt>{en?'Status':'状态'}</dt><dd>{translateStatus(record.status,$locale)}</dd>{/if}
  {#if record.priority}<dt>{en?'Priority':'优先级'}</dt><dd>{record.priority==='high'?(en?'High':'高'):record.priority==='low'?(en?'Low':'低'):(en?'Normal':'普通')}</dd>{/if}
  {#if record.waiting_for!==undefined}<dt>{en?'Waiting for':'等待对象'}</dt><dd>{record.waiting_for||(en?'Not set':'未填写')}</dd>{/if}
  {#each dates as [key,zh,label]}{#if record[key]!==undefined}<dt>{en?label:zh}</dt><dd>{date(record[key])}</dd>{/if}{/each}
  <dt>{en?'Notes':'说明'}</dt><dd class="notes">{record.notes||(en?'No notes':'未填写')}</dd>
 </dl>
</section>
<style>.delete-preview{min-width:0;border:1px solid var(--color-border);background:var(--color-surface-muted);border-radius:8px;padding:14px}.delete-preview h3{margin:0 0 12px;font-size:17px;overflow-wrap:anywhere}.delete-preview dl{display:grid;grid-template-columns:minmax(70px,.4fr) minmax(0,1fr);gap:8px 14px;margin:0;font-size:14px;line-height:1.6}.delete-preview dt{color:var(--color-muted)}.delete-preview dd{margin:0;overflow-wrap:anywhere}.notes{white-space:pre-wrap}</style>
