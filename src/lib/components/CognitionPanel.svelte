<script lang="ts">
  import { command } from "$lib/services/api";
  import { locale } from "$lib/i18n";
  import { dataRevision } from "$lib/stores/dataRevision";
  let { scope = "global", scopeId = null }: {scope?: "global" | "workspace" | "work"; scopeId?: number | null} = $props();
  type Entry = {scope_key:string; version:number; markdown:string; path:string; document_count:number; ready_count:number; generated_at:number; unavailable_folders:number};
  let entry=$state<Entry|null>(null);
  let expanded=$state(false);
  let busy=$state(false);
  let error=$state("");
  let en=$derived($locale==="en-US");
  let requestToken=0;
  async function load(refresh=false) {
    const token=++requestToken; busy=true; error="";
    try {
      const result=await command<Entry>(refresh?"refresh_project_cognition":"get_project_cognition",{scope,scopeId});
      if(token===requestToken)entry=result;
    } catch(e) {if(token===requestToken)error=String(e);}
    finally{if(token===requestToken)busy=false;}
  }
  $effect(()=>{scope;scopeId;$dataRevision.works;$dataRevision.analysis;entry=null;if(expanded)void load();});
</script>

<details class="cognition-panel" bind:open={expanded} data-testid={`cognition-${scope}`}>
  <summary><span>{en?"Project cognition":"项目认知"}</span><small>{en?"Read-only source files · compact entry":"源文件只读 · 轻量入口"}</small></summary>
  {#if expanded}
    <div class="cognition-body">
      <p>{en?"AI reads this local introduction first, then selects relevant details for follow-up. These generated files live in app storage; your folders stay unchanged.":"AI 先看这份认知入口，再为任务跟进挑选相关资料。认知文件位于应用存储区，您的工作文件夹保持原样。"}</p>
      <div class="cognition-toolbar">
        <button data-testid="refresh-cognition" disabled={busy} onclick={()=>load(true)}>{busy?(en?"Updating in background…":"后台更新中…"):(en?"Refresh index & entry":"更新索引与认知")}</button>
        {#if entry?.path}<button onclick={()=>command("reveal_in_explorer",{path:entry?.path}).catch(e=>error=String(e))}>{en?"Locate Markdown":"定位 Markdown"}</button>{/if}
      </div>
      {#if error}<p role="alert" class="cognition-error">{error}</p>{/if}
      {#if entry}
        <div class="coverage">v{entry.version} · {entry.ready_count} / {entry.document_count} {en?"readable documents":"文件可读"} · {new Date(entry.generated_at*1000).toLocaleString($locale)}</div>
        {#if entry.unavailable_folders}<p class="cognition-error">{en?"Some folders are unavailable; this entry includes historical evidence.":"部分目录暂不可访问，入口含历史索引，请留意资料时效。"}</p>{/if}
        <pre data-testid="cognition-markdown">{entry.markdown}</pre>
      {/if}
    </div>
  {/if}
</details>

<style>
 .cognition-panel{min-width:0;margin:12px 0;border:1px solid var(--color-border);border-radius:12px;background:var(--color-surface)}
 summary{display:flex;gap:10px;flex-wrap:wrap;align-items:center;cursor:pointer;padding:13px 16px;color:var(--color-primary);font-size:15px;font-weight:650;list-style:revert}
 summary small{font-weight:400;color:var(--color-muted);font-size:13px}.cognition-body{min-width:0;padding:0 16px 16px}
 p{font-size:14px;line-height:1.7;margin:0 0 12px;color:var(--color-muted)}.cognition-toolbar{display:flex;flex-wrap:wrap;gap:8px;margin-bottom:12px}
 button{font:inherit;font-size:14px;min-height:38px;border:1px solid var(--color-border);border-radius:9px;padding:8px 12px;background:var(--color-primary-soft);color:var(--color-primary);cursor:pointer}
 button:disabled{opacity:.65}.coverage{font-size:13px;line-height:1.7;color:var(--color-muted);overflow-wrap:anywhere}
 pre{font:14px/1.8 var(--font-sans);white-space:pre-wrap;overflow-wrap:anywhere;max-height:440px;overflow:auto;padding:16px;background:var(--color-surface-muted);border-radius:10px;margin:12px 0 0}
 .cognition-error{color:var(--color-danger);overflow-wrap:anywhere}
</style>
