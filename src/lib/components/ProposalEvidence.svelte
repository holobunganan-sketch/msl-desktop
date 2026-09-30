<script lang="ts">
  import {aiText} from '$lib/services/aiText';
  import {locale} from '$lib/i18n';
  let {payloadJson}:{payloadJson:string}=$props();
  const en=$derived($locale==='en-US');
  const payload=$derived.by(()=>{try{const value=JSON.parse(payloadJson);return value&&typeof value==='object'&&!Array.isArray(value)?value:{};}catch{return {};}});
  const labels:Record<string,[string,string]>={title:['标题','Title'],summary:['目标与范围','Objective'],status:['状态','Status'],priority:['优先级','Priority'],notes:['说明','Notes'],current_state:['进展','Progress'],next_step:['下一步','Next step'],remember:['提醒','Remember'],waiting_for:['等待对象','Waiting for'],due_at:['截止时间','Deadline'],follow_up_at:['跟进时间','Follow-up'],scheduled_start:['安排开始','Start'],scheduled_end:['安排结束','End'],start_at:['日程开始','Event start'],end_at:['日程结束','Event end'],location:['地点','Location'],category:['项目分类','Category'],clinical_work_id:['关联临床研究','Related research'],work_id:['所属项目','Owning project']};
  const evidence=$derived.by(()=>{
    if(!payload.field_evidence||typeof payload.field_evidence!=='object'||Array.isArray(payload.field_evidence))return [];
    return Object.entries(payload.field_evidence).flatMap(([field,raw])=>{
      if(!labels[field]||!raw||typeof raw!=='object')return [];
      const entry=raw as Record<string,unknown>;
      if(typeof entry.quote!=='string'||!entry.quote.trim())return [];
      return [{field,quote:entry.quote,suggestion:entry.basis==='suggestion'}];
    });
  });
  const unknowns=$derived(Array.isArray(payload.unknowns)?payload.unknowns.filter((v:unknown):v is string=>typeof v==='string'&&Boolean(v.trim())):[]);
</script>
{#if unknowns.length}<div class="unknowns" data-testid="proposal-unknowns"><strong>{en?'Still to clarify':'仍待明确'}</strong><ul>{#each unknowns as question}<li>{aiText(question)}</li>{/each}</ul></div>{/if}
{#if evidence.length}<details class="field-evidence" data-testid="proposal-evidence"><summary>{en?'Sources for these fields':'查看填写依据'}</summary><dl>{#each evidence as entry}<div><dt>{labels[entry.field][en?1:0]}<span>{entry.suggestion?(en?'Suggestion to review':'待确认建议'):(en?'Source excerpt':'来源摘录')}</span></dt><dd>{aiText(entry.quote)}</dd></div>{/each}</dl></details>{/if}
<style>
  .unknowns,.field-evidence{min-width:0;font-size:14px;line-height:1.7}.unknowns{border-left:2px solid var(--color-warning);padding:5px 12px;color:var(--color-muted)}strong{font-weight:550;color:var(--color-text)}ul{margin:4px 0;padding-left:20px}li,dd{overflow-wrap:anywhere;white-space:pre-line}summary{cursor:pointer;color:var(--color-muted)}dl{margin:9px 0 0;display:grid;gap:12px}dt{display:flex;flex-wrap:wrap;gap:8px;color:var(--color-text)}dt span{font-size:12px;color:var(--color-muted)}dd{margin:3px 0 0;padding-left:12px;border-left:2px solid var(--color-border);color:var(--color-muted)}
</style>
