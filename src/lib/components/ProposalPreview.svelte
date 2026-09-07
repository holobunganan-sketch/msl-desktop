<script lang="ts">
  import { locale } from '$lib/i18n';
  import { proposalPresentation } from '$lib/services/proposalPresentation';
  let {item,works}: {item:{kind:string;operation?:string;work_id:number|null;payload_json:string;reason:string};works:Array<{id:number;title:string}>}=$props();
  const preview=$derived(proposalPresentation(item,works,$locale));
</script>
<div class="proposal-preview" data-testid="proposal-preview">
  <p class="preview-action">{preview.action}<span> · {preview.scope}</span></p>
  {#if preview.time||preview.timeDetails.length}<div class="time-preview" data-testid="proposal-calendar-preview">{#if preview.timeBasis}<strong>{preview.timeBasis}</strong>{/if}{#if preview.timeDetails.length}{#each preview.timeDetails as detail}<p>{detail}</p>{/each}{:else}<p class:attention={preview.needsAttention}>{preview.time}</p>{/if}{#if preview.timeReason}<p>{preview.timeReason}</p>{/if}{#if preview.calendarHint}<p class="calendar-hint">{preview.calendarHint}</p>{/if}</div>{/if}
  {#if preview.needsAttention}<small>{$locale==='en-US'?'Please adjust the missing information before accepting.':'请先点“调整”，确认缺少的信息。'}</small>{/if}
  {#if preview.changes.length}<ul>{#each preview.changes as change}<li>{change}</li>{/each}</ul>{/if}
  {#if item.reason}<details><summary>{$locale==='en-US'?'Why this arrangement?':'为什么这样安排'}</summary><p>{item.reason}</p></details>{/if}
</div>
<style>
  .time-preview{padding:10px 14px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-primary-soft);display:grid;gap:4px}.calendar-hint{font-size:13px;color:var(--color-primary)}
  .proposal-preview{display:grid;gap:7px;min-width:0;font-size:14px;line-height:1.65;color:var(--color-muted)}
  ul{padding-left:20px;margin:2px 0;display:grid;gap:5px}li{overflow-wrap:anywhere}
  p{margin:0;overflow-wrap:anywhere}.preview-action{color:var(--color-text);font-weight:550}.preview-action span{font-weight:400;color:var(--color-muted)}
  details{min-width:0}summary{cursor:pointer;font-size:13px}details p{padding-top:6px}.attention,small{color:var(--color-warning)}small{font-size:13px}
</style>
