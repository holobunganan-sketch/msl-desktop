<script lang="ts">
 import {locale} from '$lib/i18n';
 import {sourceLabel,type Citation,type Pack,type Evidence} from '$lib/services/knowledge';
 import {navigateTo} from '$lib/services/navigation';
 import {sourceDestination} from '$lib/services/workflowContinuity';
 import {command,normalizeError} from '$lib/services/api';
 import Modal from './ui/Modal.svelte';
 let previewOpen=$state(false),previewError=$state(''),preview=$state<{material:{filename:string;error:string};segments:{text:string;locator:string;kind:string}[]}|null>(null);
 let previewRequest=0;
 function materialLocation(source?:Evidence){
  const loc=source?.location;
  return source?.trust!=='deleted'&&loc?.available&&!loc.remote_only&&loc.entity_kind==='kol_material'&&loc.entity_id>0&&loc.material_id&&loc.material_id>0&&loc.blob_hash?loc:null;
 }
 async function openMaterial(source:Evidence){
  const loc=materialLocation(source);if(!loc)return;
  const request=++previewRequest;previewOpen=true;preview=null;previewError='';
  try{const result=await command<typeof preview>('get_kol_material_preview',{id:loc.material_id,segmentId:loc.entity_id,expectedBlobHash:loc.blob_hash});if(request===previewRequest)preview=result;}
  catch(e){if(request===previewRequest)previewError=normalizeError(e);}
 }
 let {citations=[],pack}:{citations?:Citation[];pack:Pack|null}=$props();
 let en=$derived($locale==='en-US');
 let citedNotes=$derived([...new Set(citations.map(c=>c.source_id))].map(id=>pack?.sources.find(s=>s.id===id&&s.kind==='kol_note')).filter(s=>s!=null));
 let expertCount=$derived(new Set(citedNotes.flatMap(s=>s.location?.expert_id?[s.location.expert_id]:[])).size);
</script>
{#if citations.length}
 <details class="sources"><summary>{en?'Evidence':'依据'} · {citations.length}{citedNotes.length?' · '+expertCount+(en?' experts / ':' 位专家 / ')+citedNotes.length+(en?' original notes':' 条原始交流'):''}</summary>
 {#each citations as citation,i (`${i}-${citation.source_id}`)}
  {@const source=pack?.sources.find(s=>s.id===citation.source_id)}
  {@const destination=source?.trust==='deleted'?null:sourceDestination(source?.location)}
  {@const file=materialLocation(source)}
  <div class="source" data-source-id={citation.source_id}><strong>{sourceLabel(source?.kind??'',en)} · {source?.title||(en?'Source record':'来源记录')}</strong>
   {#if source?.trust==='deleted'}<p class="removed">{en?'Source deleted · historical reference only':'来源已删除 · 仅供历史回顾'}</p>{:else}<blockquote>{citation.quote}</blockquote>
    {#if source?.location?.remote_only}<p class="removed">{en?'Source is on another device · available here as a reference only':'来源位于其他设备 · 此处保留引用，无法直接打开'}</p>
    {:else if !destination&&!file}<p class="removed">{en?'No reliable location is available for this source.':'此来源暂无可靠的定位信息，无法直接打开。'}</p>{/if}
   {/if}
   {#if source?.kind==='kol_material'&&source.trust!=='deleted'}<small>{source.trust==='model_reading'?(en?'Model interpretation · verify the original':'模型解读 · 请核对原资料'):(en?'Extracted document text':'资料提取正文')}{file?.locator?' · '+file.locator:''}{file?.revision?' · v'+file.revision:''}</small>{#if file}<button data-testid="view-source-material" onclick={()=>source&&openMaterial(source)}>{en?'View material':'查看资料'}</button>{/if}{/if}
   {#if source?.timestamp}<small>{new Date(source.timestamp*1000).toLocaleString($locale)}</small>{/if}
   {#if destination}<button data-testid="open-source-record" onclick={()=>navigateTo(destination)}>{en?'Open record':'打开记录'} ↗</button>{/if}
  </div>
 {/each}</details>
{/if}
<Modal bind:open={previewOpen} onclose={()=>previewRequest++} title={preview?.material.filename??(en?'Material evidence':'资料依据')}><div class="evidence-preview">{#if previewError}<p role="alert">{previewError}</p>{:else if preview}<p>{preview.material.error}</p>{#each preview.segments as segment}<section><small>{segment.locator} · {segment.kind==='model_interpretation'?(en?'Model interpretation':'模型解读'):(en?'Extracted text':'提取正文')}</small><p>{segment.text}</p></section>{/each}{:else}<p>{en?'Loading material…':'正在读取资料…'}</p>{/if}</div></Modal>
<style>.sources{font-size:14px;color:var(--color-muted);margin-top:10px;min-width:0}.sources summary{cursor:pointer;padding:5px 0}.source{padding:12px;margin:8px 0;border:1px solid var(--color-border);border-radius:12px;overflow-wrap:anywhere}.source strong{font-weight:550;color:var(--color-text)}blockquote{margin:8px 0;white-space:pre-wrap;border-left:3px solid var(--color-border);padding-left:12px;line-height:1.7}.removed{margin:8px 0;line-height:1.65}.source small{display:inline-block;margin-right:8px}.evidence-preview p{white-space:pre-wrap;overflow-wrap:anywhere;line-height:1.75}button{margin:6px 12px 0 0;border:0;background:transparent;color:var(--color-primary);cursor:pointer;font:inherit;min-height:32px;text-align:left}button:focus-visible{outline:2px solid var(--color-primary);outline-offset:2px}</style>
