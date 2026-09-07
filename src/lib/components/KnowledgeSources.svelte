<script lang="ts">
 import {locale} from '$lib/i18n';
 import {sourceLabel,type Citation,type Pack} from '$lib/services/knowledge';
 import {navigateTo} from '$lib/services/navigation';
 let {citations=[],pack}:{citations?:Citation[];pack:Pack|null}=$props();
 let en=$derived($locale==='en-US');
 let citedNotes=$derived([...new Set(citations.map(c=>c.source_id))].map(id=>pack?.sources.find(s=>s.id===id&&s.kind==='kol_note')).filter(s=>s!=null));
 let expertCount=$derived(new Set(citedNotes.flatMap(s=>{try{const id=JSON.parse(s.text).expert_id;return id?[id]:[];}catch{return [];}})).size);
</script>
{#if citations.length}
 <details class="sources"><summary>{en?'Evidence':'依据'} · {citations.length}{citedNotes.length?' · '+expertCount+(en?' experts / ':' 位专家 / ')+citedNotes.length+(en?' original notes':' 条原始交流'):''}</summary>
 {#each citations as citation,i (`${i}-${citation.source_id}`)}
  {@const source=pack?.sources.find(s=>s.id===citation.source_id)}
  <div class="source"><strong>{sourceLabel(source?.kind??'',en)} · {source?.title??citation.source_id}</strong>
   <blockquote>{citation.quote}</blockquote>
   {#if source?.timestamp}<small>{new Date(source.timestamp*1000).toLocaleString($locale)}</small>{/if}
   {#if source&&['work','task','waiting','calendar','inbox','resume','proposal'].includes(source.kind)}<button onclick={()=>navigateTo(source.kind,source.entity_id)}>{en?'Open record':'打开记录'} ↗</button>{/if}
  </div>
 {/each}</details>
{/if}
<style>.sources{font-size:14px;color:var(--color-muted);margin-top:10px}.sources summary{cursor:pointer;padding:5px 0}.source{padding:12px;margin:8px 0;border:1px solid var(--color-border);border-radius:12px;overflow-wrap:anywhere}.source strong{font-weight:550;color:var(--color-text)}blockquote{margin:8px 0;white-space:pre-wrap;border-left:3px solid var(--color-border);padding-left:12px;line-height:1.7}button{margin-left:12px;border:0;background:transparent;color:var(--color-primary);cursor:pointer;font:inherit}</style>
