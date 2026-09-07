<script lang="ts">
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import { command } from "$lib/services/api";
  import { locale, t } from "$lib/i18n";
  import { onMount, onDestroy } from "svelte";
  import { aiJobs, refreshJobs, translationDraft } from "$lib/stores/aiJobs";

  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);
  let source = $state(translationDraft.source);
  let result = $state(translationDraft.result);
  let style = $state<"written" | "spoken">(translationDraft.style);
  let busy = $derived($aiJobs.some(job=>job.command==="translate_text" && job.status==="running"));
  let error = $state("");
  let direction = $derived(source.split("").filter((c) => /^[\u4e00-\u9fff]$/.test(c)).length * 5 >= Math.max(1, source.replace(/[^\p{L}\p{N}]/gu, "").length) ? "zh-to-en" : "en-to-zh");

  async function translate() { if (!source.trim() || busy) return; error = ""; try { result = await command<string>("translate_text", { input: source, style }); } catch (e) { error = String(e); } }
  onMount(()=>{void refreshJobs().catch(()=>{}).then(()=>{
    if(!source){const job=$aiJobs.find(j=>j.command==="translate_text");if(job){source=String(job.args.input??"");style=job.args.style==="spoken"?"spoken":"written";}}
  });});
  onDestroy(()=>{translationDraft.source=source;translationDraft.result=result;translationDraft.style=style;});
  $effect(()=>{const job=$aiJobs.find(j=>j.command==="translate_text");if(job && job.args.input===source && job.args.style===style){if(job.status==="completed" && typeof job.result==="string")result=job.result;else if(job.error)error=job.error;}});
  function clear() { source = ""; result = ""; error = ""; }
  async function copy() { try { await navigator.clipboard.writeText(result); } catch { error = tt("translation.copyFailed"); } }
</script>

<div class="translation-page">
  <header class="page-head"><div><div class="eyebrow"><Icon name="languages" size={14} />{tt("translation.workspace")}</div><h1>{tt("translation.title")}</h1><p>{tt("translation.pageHint")}</p></div><div class="style-switch" role="group" aria-label={tt("translation.style")}><button class:active={style === "written"} onclick={() => (style = "written")}>{tt("translation.written")}</button><button class:active={style === "spoken"} onclick={() => (style = "spoken")}>{tt("translation.spoken")}</button></div></header>
  <section class="translation-workspace">
    <article class="panel source-panel"><div class="panel-head"><div><strong>{tt("translation.source")}</strong><span>{tt("translation.direction", { direction })}</span></div><span>{source.length}/20000</span></div><textarea data-testid="translation-source" bind:value={source} maxlength="20000" placeholder={tt("translation.placeholder")} onkeydown={(e) => { if ((e.ctrlKey || e.metaKey) && e.key === "Enter") translate(); }}></textarea><div class="panel-actions"><AppButton onclick={translate} loading={busy} disabled={!source.trim()}><Icon name="sparkles" size={15} />{tt("translation.translate")}</AppButton><AppButton variant="ghost" onclick={clear} disabled={!source && !result}>{tt("common.clear")}</AppButton><span>{tt("translation.shortcut")}</span></div></article>
    <article class="panel result-panel"><div class="panel-head"><div><strong>{tt("translation.result")}</strong><span>{result ? tt("translation.resultReady") : tt("translation.resultEmpty")}</span></div>{#if result}<button class="copy" onclick={copy} aria-label={tt("translation.copy")}><Icon name="copy" size={16} />{tt("translation.copy")}</button>{/if}</div><div data-testid="translation-result" class:empty={!result} class="result">{result || tt("translation.resultPlaceholder")}</div></article>
  </section>
  {#if error}<div class="error" role="alert">{error}</div>{/if}
</div>

<style>
  .translation-page{height:100%;min-height:0;display:grid;grid-template-rows:auto minmax(0,1fr) auto;gap:16px}.page-head{min-height:74px;display:flex;align-items:center;justify-content:space-between;gap:20px}.eyebrow{display:flex;align-items:center;gap:6px;color:var(--color-primary);font-size:11px;font-weight:700;letter-spacing:.1em;text-transform:uppercase}.page-head h1{margin:5px 0 4px;font:500 28px var(--font-serif)}.page-head p{margin:0;color:var(--color-muted);font-size:13px}.style-switch{display:flex;padding:4px;border-radius:11px;background:var(--color-surface-muted)}.style-switch button{min-width:86px;min-height:39px;border:0;border-radius:8px;background:transparent;color:var(--color-muted);font-size:13px;cursor:pointer}.style-switch button.active{background:var(--color-surface-raised);color:var(--color-text);box-shadow:var(--shadow-sm);font-weight:650}.translation-workspace{min-height:0;display:grid;grid-template-columns:1fr 1fr;gap:14px}.panel{min-width:0;min-height:0;display:grid;grid-template-rows:auto minmax(0,1fr) auto;padding:18px;border:1px solid var(--color-border);border-radius:17px;background:var(--color-surface);box-shadow:var(--shadow-sm)}.panel-head{display:flex;align-items:flex-start;justify-content:space-between;gap:12px;margin-bottom:14px;color:var(--color-muted);font-size:12px}.panel-head>div{display:grid;gap:4px}.panel-head strong{color:var(--color-text);font:500 19px var(--font-serif)}textarea{width:100%;height:100%;min-height:300px;resize:none;padding:16px;border:1px solid var(--color-border);border-radius:13px;background:var(--color-surface-muted);color:var(--color-text);font-size:16px;line-height:1.7}.panel-actions{display:flex;align-items:center;gap:8px;margin-top:14px}.panel-actions>span{margin-left:auto;color:var(--color-muted);font-size:11px}.result{min-height:300px;padding:17px;border:1px solid var(--color-border);border-radius:13px;background:linear-gradient(145deg,var(--color-primary-soft),var(--color-surface-muted));white-space:pre-wrap;overflow:auto;color:var(--color-text);font-size:16px;line-height:1.7}.result.empty{display:grid;place-items:center;color:var(--color-muted);text-align:center}.copy{display:flex;align-items:center;gap:6px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface-raised);padding:7px 10px;color:var(--color-primary);cursor:pointer}.error{padding:10px 12px;border:1px solid #ead2d2;border-radius:10px;background:var(--color-danger-soft);color:var(--color-danger)}@media(max-width:850px){.translation-page{height:auto}.translation-workspace{grid-template-columns:1fr}.panel{min-height:430px}.page-head{align-items:flex-start;flex-direction:column}.style-switch{align-self:stretch}.style-switch button{flex:1}}
</style>
