<script lang="ts">
  import {command} from '$lib/services/api';
  import {aiText} from '$lib/services/aiText';
  import {locale} from '$lib/i18n';
  import {readableOutput} from '$lib/services/retainedOutput';
  let {runId,updatedAt=0}: {runId:number|null;updatedAt?:number}=$props();
  type Output={run_id:number;raw_output:string;warnings_json:string};
  let result=$state<Output|null>(null);
  let error=$state('');
  let en=$derived($locale==='en-US');
  const warnings=$derived.by(()=>{try{return JSON.parse(result?.warnings_json??'[]') as string[];}catch{return ['核对提示暂时无法读取'];}});
  const sections=$derived(readableOutput(result?.raw_output??''));
  $effect(()=>{
    const id=runId;void updatedAt;let cancelled=false;result=null;error='';
    if(id)void command<Output|null>('get_analysis_output',{runId:id}).then(value=>{if(!cancelled)result=value;}).catch(()=>{if(!cancelled)error=en?'Unable to load the saved answer.':'暂时无法读取已保存的回答，请刷新重试。';});
    return()=>{cancelled=true;};
  });
</script>

{#if error}<p role="alert">{error}</p>{/if}
{#if result}
  <details class="retained-answer" open={warnings.length>0} data-testid="retained-analysis-output">
    <summary>{warnings.length?(en?'Answer saved · review needed':'回答已保留 · 有内容待核对'):(en?'View full analysis':'查看本次完整分析')} <span>#{result.run_id}</span></summary>
    {#if warnings.length}
      <div class="review-warning" role="status">
        <p>{en?'The full answer is available below. Verified suggestions remain in the review queue. Other text will not change your records.':'完整回答在下方。能够核对的建议已进入待确认队列；其余内容保留供查看，不会直接改动项目、事项或专家记录。'}</p>
        <ul>{#each warnings as warning}<li>{aiText(warning)}</li>{/each}</ul>
      </div>
    {/if}
    <div class="answer-reading">{#each sections as section}
      <section>{#if section.title}<h3>{aiText(section.title)}</h3>{/if}<p>{aiText(section.text)}</p></section>
    {/each}</div>
    <details class="raw-output"><summary>{en?'Original response':'查看原始回答'}</summary><pre>{result.raw_output}</pre></details>
  </details>
{/if}

<style>
  .retained-answer{min-width:0;padding:20px 24px;border:1px solid var(--color-border);border-radius:16px;background:var(--color-surface);margin-bottom:20px}
  summary{cursor:pointer;font-weight:650;line-height:1.6;overflow-wrap:anywhere}summary span{font-size:.85em;color:var(--color-muted);margin-left:8px}
  .review-warning{margin:16px 0;padding:14px 18px;border-left:3px solid var(--color-warning);background:var(--color-surface-muted);font-size:14px;line-height:1.7;overflow-wrap:anywhere}.review-warning p{margin:0}.review-warning ul{padding-left:1.3em;margin:8px 0 0}.review-warning li+li{margin-top:8px}
  .answer-reading{max-height:560px;overflow:auto;scrollbar-gutter:stable;margin-top:18px;padding-right:8px}.answer-reading section+section{margin-top:20px}.answer-reading h3{font-size:16px;margin:0 0 8px}.answer-reading p{white-space:pre-wrap;overflow-wrap:anywhere;line-height:1.8;margin:0}
  .raw-output{margin-top:18px;color:var(--color-muted);font-size:14px}pre{white-space:pre-wrap;overflow-wrap:anywhere;max-height:400px;overflow:auto;font-family:inherit;line-height:1.7}
</style>
