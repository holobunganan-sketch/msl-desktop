<script lang="ts">
 import {locale} from '$lib/i18n';
 import {sourceLabel,type Citation,type Pack} from '$lib/services/knowledge';
 import {navigateTo} from '$lib/services/navigation';
 import {command,normalizeError} from '$lib/services/api';
 import {json} from '$lib/services/knowledge';
 import Modal from './ui/Modal.svelte';
 let previewOpen=$state(false),previewError=$state(''),preview=$state<{material:{filename:string;error:string};segments:{text:string;locator:string;kind:string}[]}|null>(null);
 async function openMaterial(id:number,segmentId:number,expectedBlobHash:string){previewOpen=true;preview=null;previewError='';try{preview=await command('get_kol_material_preview',{id,segmentId,expectedBlobHash});}catch(e){previewError=normalizeError(e);}}
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
   {#if source?.trust==='deleted'}<p class="removed">{en?'Source deleted · historical reference only':'来源已删除 · 仅供历史回顾'}</p>{:else}<blockquote>{citation.quote}</blockquote>{/if}
   {#if source?.kind==='kol_material'&&source.trust!=='deleted'}{@const file=json<{material_id:number;locator:string;revision:number;blob_hash:string}>(source.text,{material_id:0,locator:'',revision:0,blob_hash:''})}<small>{source.trust==='model_reading'?(en?'Model interpretation · verify the original':'模型解读 · 请核对原资料'):(en?'Extracted document text':'资料提取正文')} · {file.locator} · v{file.revision}</small><small title={file.blob_hash}> · {file.blob_hash.slice(0,12)}</small><button onclick={()=>openMaterial(file.material_id,source.entity_id,file.blob_hash)}>{en?'View material':'查看资料'}</button>{/if}
   {#if source?.timestamp}<small>{new Date(source.timestamp*1000).toLocaleString($locale)}</small>{/if}
   {#if source&&['work','task','waiting','calendar','inbox','resume','proposal'].includes(source.kind)}<button onclick={()=>navigateTo(source.kind,source.entity_id)}>{en?'Open record':'打开记录'} ↗</button>{/if}
  </div>
 {/each}</details>
{/if}
<Modal bind:open={previewOpen} title={preview?.material.filename??(en?'Material evidence':'资料依据')}><div class="evidence-preview">{#if previewError}<p>{previewError}</p>{:else if preview}<p>{preview.material.error}</p>{#each preview.segments as segment}<section><small>{segment.locator} · {segment.kind==='model_interpretation'?(en?'Model interpretation':'模型解读'):(en?'Extracted text':'提取正文')}</small><p>{segment.text}</p></section>{/each}{:else}<p>{en?'Loading material…':'正在读取资料…'}</p>{/if}</div></Modal>
<style>.sources{font-size:14px;color:var(--color-muted);margin-top:10px}.sources summary{cursor:pointer;padding:5px 0}.source{padding:12px;margin:8px 0;border:1px solid var(--color-border);border-radius:12px;overflow-wrap:anywhere}.source strong{font-weight:550;color:var(--color-text)}blockquote{margin:8px 0;white-space:pre-wrap;border-left:3px solid var(--color-border);padding-left:12px;line-height:1.7}button{margin-left:12px;border:0;background:transparent;color:var(--color-primary);cursor:pointer;font:inherit}</style>
