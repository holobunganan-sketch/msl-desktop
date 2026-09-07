<script lang="ts">
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import AppCard from "$lib/components/ui/AppCard.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import { command } from "$lib/services/api";
  import { locale, t } from "$lib/i18n";

  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);
  let source = $state("");
  let result = $state("");
  let style = $state<"written" | "spoken">("written");
  let busy = $state(false);
  let error = $state("");
  let direction = $derived(source.split("").filter((c) => /^[\u4e00-\u9fff]$/.test(c)).length * 5 >= Math.max(1, source.replace(/[^\p{L}\p{N}]/gu, "").length) ? "zh-to-en" : "en-to-zh");

  async function translate() { if (!source.trim()) return; busy = true; error = ""; try { result = await command<string>("translate_text", { input: source, style }); } catch (e) { error = String(e); } finally { busy = false; } }
  function clear() { source = ""; result = ""; error = ""; }
  async function copy() { try { await navigator.clipboard.writeText(result); } catch { error = tt("translation.copyFailed"); } }
</script>

<AppCard>
  <div class="translation-head">
    <div><strong>{tt("translation.title")}</strong><span>{tt("translation.hint")}</span></div>
    <div class="style-switch" role="group" aria-label={tt("translation.style")}><button class:active={style === "written"} onclick={() => (style = "written")}>{tt("translation.written")}</button><button class:active={style === "spoken"} onclick={() => (style = "spoken")}>{tt("translation.spoken")}</button></div>
  </div>
  <textarea bind:value={source} maxlength="20000" rows="3" placeholder={tt("translation.placeholder")} onkeydown={(e) => { if ((e.ctrlKey || e.metaKey) && e.key === "Enter") translate(); }}></textarea>
  <div class="translation-meta"><span>{tt("translation.direction", { direction })}</span><span>{source.length}/20000</span></div>
  {#if result}<div class="result"><div class="result-head"><span>{tt("translation.result")}</span><button onclick={copy} aria-label={tt("translation.copy")}><Icon name="copy" size={13} /></button></div><p>{result}</p></div>{/if}
  {#if error}<div class="error" role="alert">{error}</div>{/if}
  <div class="actions"><AppButton onclick={translate} loading={busy} disabled={!source.trim()}>{tt("translation.translate")}</AppButton><AppButton variant="ghost" onclick={clear} disabled={!source && !result}>{tt("common.clear")}</AppButton></div>
</AppCard>

<style>
  .translation-head,.translation-meta,.actions,.result-head{display:flex;justify-content:space-between;gap:8px;align-items:center}.translation-head{margin-bottom:8px}.translation-head>div:first-child{min-width:0;display:grid;gap:2px}.translation-head strong{font-size:13px;font-weight:650}.translation-head span,.translation-meta{color:var(--color-muted);font-size:10px}.style-switch{display:flex;flex:0 0 auto;padding:3px;border-radius:8px;background:#eef2f3}.style-switch button{border:0;border-radius:6px;background:transparent;color:#829198;padding:4px 7px;font-size:11px;cursor:pointer}.style-switch button.active{background:white;color:#536a75;box-shadow:0 2px 6px rgb(46 65 73 / .06)}textarea{width:100%;min-height:58px;max-height:88px;resize:none;padding:9px;border:1px solid var(--color-border);border-radius:10px;background:#f8fafa;color:var(--color-text);font-size:13px;line-height:1.45}.translation-meta{margin:4px 0 6px}.actions{justify-content:flex-start;margin-top:7px}.actions :global(.app-button){min-height:29px;padding:5px 9px;font-size:11px}.result{margin-top:7px;padding:8px 9px;border-radius:9px;background:var(--color-primary-soft);color:#526973}.result-head{font-size:10px;color:#73868e}.result-head button{width:23px;height:23px;display:grid;place-items:center;border:0;border-radius:7px;background:transparent;color:#647d88}.result p{max-height:52px;overflow:auto;white-space:pre-wrap;margin:5px 0 0;font-size:12px;line-height:1.45}.error{margin-top:5px;color:var(--color-danger);font-size:11px}
</style>
