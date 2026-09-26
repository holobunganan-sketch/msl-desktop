<script lang="ts">
  import { locale, t } from "$lib/i18n";
  import { appTheme, fontSize, setFontSize, setTheme, type AppTheme, type FontSize } from "$lib/stores/appearance";

  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0]) => t(key, {}, currentLocale);
  const sizes: FontSize[] = ["compact", "standard", "large", "xlarge"];
  const themes: AppTheme[] = ["mist", "sage", "ivory", "contrast"];
  const sizeLabels: Record<FontSize, Parameters<typeof t>[0]> = { compact:"appearance.compact", standard:"appearance.standard", large:"appearance.large", xlarge:"appearance.xlarge" };
  const themeLabels: Record<AppTheme, Parameters<typeof t>[0]> = { mist:"appearance.mist", sage:"appearance.sage", ivory:"appearance.ivory", contrast:"appearance.contrast" };
</script>

<section class="appearance-settings">
  <div class="section-head"><div><h2>{tt("appearance.title")}</h2><p>{tt("appearance.hint")}</p></div></div>
  <div class="appearance-grid">
    <div><strong>{tt("appearance.fontSize")}</strong><div class="choice-row">{#each sizes as size}<button data-testid={`font-size-${size}`} class:active={$fontSize===size} onclick={() => setFontSize(size)}>{tt(sizeLabels[size])}<span style={`font-size:${size === "compact" ? 12 : size === "standard" ? 14 : size === "large" ? 16 : 18}px`}>Aa</span></button>{/each}</div></div>
    <div><strong>{tt("appearance.theme")}</strong><div class="choice-row theme-row">{#each themes as theme}<button data-testid={`theme-${theme}`} class:active={$appTheme===theme} onclick={() => setTheme(theme)}><i class={`swatch ${theme}`}></i>{tt(themeLabels[theme])}</button>{/each}</div></div>
  </div>
</section>

<style>
  .appearance-settings{display:grid;gap:14px}.section-head h2{margin:0 0 4px;font-size:17px}.section-head p{margin:0;color:var(--color-muted);font-size:12px}.appearance-grid{display:grid;gap:16px}.appearance-grid>div{display:grid;gap:8px}.appearance-grid strong{font-size:13px}.choice-row{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:8px}.choice-row button{min-height:54px;display:flex;align-items:center;justify-content:space-between;gap:8px;padding:9px 11px;border:1px solid var(--color-border);border-radius:11px;background:var(--color-surface-muted);color:var(--color-muted);font-size:12px;cursor:pointer}.choice-row button:hover{border-color:var(--color-border-strong)}.choice-row button.active{border-color:var(--color-primary);background:var(--color-primary-soft);color:var(--color-text);box-shadow:inset 0 0 0 1px var(--color-primary)}.choice-row span{font-weight:650;color:var(--color-primary)}.theme-row button{justify-content:flex-start}.swatch{width:24px;height:24px;border:1px solid rgb(40 50 55 / .12);border-radius:8px}.swatch.mist{background:linear-gradient(135deg,#edf4f9,#147e90)}.swatch.sage{background:linear-gradient(135deg,#f0f4ef,#71887a)}.swatch.ivory{background:linear-gradient(135deg,#fbf7ef,#9b7f65)}.swatch.contrast{background:linear-gradient(135deg,#f7fafb 50%,#314a57 50%)}@media(max-width:760px){.choice-row{grid-template-columns:repeat(2,1fr)}}
</style>
