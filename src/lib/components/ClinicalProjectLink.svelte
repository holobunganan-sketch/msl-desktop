<script lang="ts">
  import {invoke} from '@tauri-apps/api/core';
  import {locale} from '$lib/i18n';
  import {invalidate} from '$lib/stores/dataRevision';
  import type {Work} from '$lib/types/domain';
  import AppButton from './ui/AppButton.svelte';
  import StatusLine from './ui/StatusLine.svelte';
  let {entityKind,entityId,onchange=()=>{}}:{entityKind:'task'|'waiting'|'calendar'|'inbox';entityId:number;onchange?:()=>void}=$props();
  type Relation={clinical_work_id:number|null;revision:number};
  let relation=$state<Relation|null>(null),works=$state<Work[]>([]),value=$state<number|null>(null);
  let opened=$state(false),loaded=$state(false),busy=$state(false),error=$state('');
  const english=$derived($locale==='en-US');
  const choices=$derived(works.filter(w=>w.category==='clinical'&&(w.status!=='archived'||w.id===relation?.clinical_work_id)));
  let serial=0;
  async function load(){
    const request=++serial;busy=true;error='';
    try{const [r,projects]=await Promise.all([invoke<Relation|null>('get_clinical_link',{entityKind,entityId}),invoke<Work[]>('list_works',{status:null})]);if(request!==serial)return;relation=r;value=r?.clinical_work_id??null;works=projects;loaded=true;}
    catch(e){if(request===serial)error=String(e);}finally{if(request===serial)busy=false;}
  }
  async function save(){
    if(busy||!loaded)return;busy=true;error='';
    try{relation=await invoke<Relation|null>('set_clinical_link',{entityKind,entityId,clinicalWorkId:value,expectedRevision:relation?.revision??0});invalidate('works','tasks','waiting','calendar','inbox','analysis');onchange();}
    catch(e){error=String(e);}finally{busy=false;}
  }
  $effect(()=>{entityKind;entityId;serial++;loaded=false;relation=null;value=null;error='';});
  $effect(()=>{if(opened&&!loaded&&!busy&&!error)void load();});
</script>

<details class="clinical-link" bind:open={opened}>
  <summary>{english?'Linked clinical study':'关联临床研究'}</summary>
  <div class="clinical-link-controls">
    <label>{english?'Clinical study (optional)':'临床研究项目（可选）'}
      <select aria-label={english?'Linked clinical study':'关联临床研究'} bind:value disabled={busy||!loaded}>
        <option value={null}>{english?'No additional association':'暂不关联'}</option>
        {#each choices as work(work.id)}<option value={work.id}>{work.title}{work.status==='archived'?(english?' · Archived':' · 已归档'):''}</option>{/each}
      </select>
    </label>
    <AppButton variant="secondary" loading={busy} disabled={!loaded||value===(relation?.clinical_work_id??null)} onclick={save}>{english?'Save association':'保存关联'}</AppButton>
  </div>
  <p>{english?'The original project and item remain unchanged. Both projects show the same record.':'保留原项目归属，两个项目查看同一条事项。'}</p>
  <StatusLine message={error}/>
  {#if error}<button type="button" onclick={load}>{english?'Reload association':'刷新当前关联'}</button>{/if}
</details>

<style>
  .clinical-link{flex-basis:100%;min-width:0;margin-top:4px}.clinical-link summary{font-size:14px;color:var(--color-primary);cursor:pointer;padding:5px 0}.clinical-link-controls{display:flex;align-items:end;gap:12px;flex-wrap:wrap;padding-top:10px}.clinical-link-controls label{display:grid;gap:7px;flex:1 1 240px;min-width:0;font-size:14px}.clinical-link select{width:100%;min-width:0;min-height:42px;border:1px solid var(--color-border);border-radius:8px;background:var(--color-surface);color:var(--color-text);padding:8px 12px;font:inherit}.clinical-link p{font-size:13px;color:var(--color-muted);line-height:1.6;margin:8px 0}.clinical-link button{font:inherit}
</style>
