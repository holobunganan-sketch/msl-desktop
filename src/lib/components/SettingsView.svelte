<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getVersion } from "@tauri-apps/api/app";
  import { locale, t, setLocale } from "$lib/i18n";
  import { normalizeError } from "$lib/services/api";
  import ProviderSettings from "$lib/components/ProviderSettings.svelte";
  import AiRoutingSettings from "$lib/components/AiRoutingSettings.svelte";
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import AnalysisScheduleSettings from "$lib/components/AnalysisScheduleSettings.svelte";
  import StorageSettings from "$lib/components/StorageSettings.svelte";
  import AppearanceSettings from "$lib/components/AppearanceSettings.svelte";
  import { addToast } from "$lib/stores/toast";

  let error = $state("");
  let appVersion = $state("");
  let info = $state("");
  let syncStatus = $state<{ root: string | null; paused: boolean; baseline_count: number; last_scan: number | null; last_warning: string | null } | null>(null);
  let notifEnabled = $state(true);
  let leadMinutes = $state(60);
  let autostartOn = $state(false);
  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);
  function syncTime(ts: number | null) { if (!ts) return tt("workspace.neverScanned"); return new Intl.DateTimeFormat(currentLocale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(ts * 1000)); }
  async function load() { try { syncStatus = await invoke("workspace_sync_status"); const n = await invoke<string | null>("app_settings_get", { key: "notifications_enabled" }); if (n !== null) notifEnabled = n !== "false"; const l = await invoke<string | null>("app_settings_get", { key: "reminder_lead_minutes" }); if (l !== null) leadMinutes = Number(l); autostartOn = await invoke<boolean>("autostart_status"); } catch (value) { error = normalizeError(value); } }
  async function toggleNotif() { try { notifEnabled = await invoke<boolean>("set_notifications_enabled", { enabled: notifEnabled }); addToast(tt("settings.saved"), "success"); } catch (value) { error = normalizeError(value); } }
  async function changeLead() { try { leadMinutes = await invoke<number>("set_reminder_lead_minutes", { minutes: leadMinutes }); } catch (value) { error = normalizeError(value); } }
  async function toggleAutostart() { try { autostartOn = await invoke<boolean>("set_autostart", { enabled: !autostartOn }); info = autostartOn ? tt("settings.autostartEnabled") : tt("settings.autostartDisabled"); } catch (value) { error = normalizeError(value); } }
  $effect(() => { $locale; void load(); void getVersion().then(value => appVersion = value); });
</script>

<div class="settings">
  <div class="page-head"><div><h1>{tt("settings.title")}</h1><p>{tt("settings.pageHint")}</p></div><AppButton variant="secondary" onclick={() => setLocale(currentLocale === "zh-CN" ? "en-US" : "zh-CN")}><Icon name="languages" size={15} />{currentLocale === "zh-CN" ? tt("common.english") : tt("common.chinese")}</AppButton></div>
  {#if error}<div class="status error" role="alert">{error}</div>{/if}{#if info}<div class="status ok">{info}</div>{/if}
  <div class="settings-layout">
    <aside class="settings-nav" aria-label={tt("settings.categories")}>
      <a href="#providers"><Icon name="sparkles" size={15} />{tt("settings.providers")}</a>
      <a href="#appearance"><Icon name="languages" size={15} />{tt("appearance.title")}</a>
      <a href="#routing"><Icon name="review" size={15} />{tt("settings.aiRouting")}</a>
      <a href="#schedule"><Icon name="clock" size={15} />{tt("settings.analysisSchedule")}</a>
      <a href="#storage"><Icon name="archive" size={15} />{tt("storage.title")}</a>
      <a href="#system"><Icon name="settings" size={15} />{tt("settings.system")}</a>
    </aside>

    <main class="settings-content">
      {#if syncStatus?.root}<section class="sync-strip"><div><span class:paused={syncStatus.paused} class="sync-dot"></span><strong>{syncStatus.paused ? tt("workspace.paused") : tt("workspace.watching")}</strong></div><span>{tt("workspace.fileCount", { count: syncStatus.baseline_count })}</span><span>{tt("workspace.lastScan", { time: syncTime(syncStatus.last_scan) })}</span>{#if syncStatus.last_warning}<span class="warning">{tt("workspace.warning", { warning: syncStatus.last_warning })}</span>{/if}</section>{/if}
      <section id="appearance" class="settings-section"><AppearanceSettings /></section>
      <section id="providers" class="settings-section"><ProviderSettings /></section>
      <section id="routing" class="settings-section"><AiRoutingSettings /></section>
      <section id="schedule" class="settings-section"><AnalysisScheduleSettings /></section>
      <section id="storage" class="settings-section"><StorageSettings /></section>
      <section id="system" class="settings-section system-section">
        <div class="section-heading"><h2>{tt("settings.system")}</h2><p>{tt("settings.systemHint")}</p><p class="build-id" data-testid="app-version">MSL Desktop · {appVersion}</p></div>
        <div class="setting-row"><div><strong>{tt("settings.language")}</strong><small>{tt("settings.appearance")}</small></div><AppButton variant="secondary" onclick={() => setLocale(currentLocale === "zh-CN" ? "en-US" : "zh-CN")}>{currentLocale === "zh-CN" ? tt("common.chinese") : tt("common.english")}</AppButton></div>
        <div class="setting-row"><div><strong>{tt("settings.notifications")}</strong><small>{tt("settings.notificationsHint")}</small></div><label class="check"><input type="checkbox" bind:checked={notifEnabled} onchange={toggleNotif} />{tt("settings.enabled")}</label></div>
        <div class="setting-row"><div><strong>{tt("settings.lead")}</strong><small>{tt("settings.notifications")}</small></div><div class="inline-control"><input class="number" type="number" min="1" max="1440" bind:value={leadMinutes} onchange={changeLead} /><span>{tt("settings.minutes")}</span></div></div>
        <div class="setting-row"><div><strong>{tt("settings.autostart")}</strong><small>{tt("settings.autostartHint")}</small></div><label class="check"><input type="checkbox" bind:checked={autostartOn} onchange={toggleAutostart} />{tt("settings.enabled")}</label></div>
        <div class="setting-row"><div><strong>{tt("settings.shortcuts")}</strong><small>{tt("settings.shortcutsSearch")} · {tt("settings.shortcutsCapture")}</small></div></div>
      </section>
    </main>
  </div>
</div>

<style>
  .settings{min-width:0;display:flex;flex-direction:column;gap:16px}.page-head{min-height:54px;display:flex;align-items:center;justify-content:space-between;gap:16px}.page-head h1{margin:0 0 5px;font:500 22px var(--font-serif)}.page-head p{margin:0;color:var(--color-muted);font-size:10px}.settings-layout{min-height:0;display:grid;grid-template-columns:190px minmax(0,1fr);gap:12px}.settings-nav{min-height:0;padding:8px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--color-surface);box-shadow:var(--shadow-sm)}.settings-nav a{min-height:38px;display:flex;align-items:center;gap:9px;padding:0 10px;border-radius:9px;color:#60737c;font-size:10px;text-decoration:none}.settings-nav a:hover{background:var(--color-primary-soft);color:#4f6873}.settings-content{min-height:0;display:grid;gap:11px;overflow:auto;padding-right:3px;scroll-behavior:smooth}.settings-section{padding:15px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--color-surface);box-shadow:var(--shadow-sm);scroll-margin-top:10px}.sync-strip{display:flex;align-items:center;gap:13px;padding:10px 12px;border:1px solid #d9e4e0;border-radius:11px;background:var(--color-success-soft);color:#657d75;font-size:9px}.sync-strip div{display:flex;align-items:center;gap:7px}.sync-strip strong{font-size:10px}.sync-dot{width:7px;height:7px;border-radius:50%;background:var(--color-success);box-shadow:0 0 0 3px rgb(111 141 130 / .12)}.sync-dot.paused{background:var(--color-warning)}.sync-strip .warning{margin-left:auto;color:var(--color-warning)}.section-heading h2{margin:0 0 4px;font-size:14px}.section-heading p{margin:0;color:var(--color-muted);font-size:10px}.setting-row{min-height:55px;display:flex;align-items:center;gap:15px;border-top:1px solid #e7edef}.setting-row>div:first-child{min-width:0;flex:1;display:grid;gap:4px}.setting-row strong{font-size:10px}.setting-row small{color:var(--color-muted);font-size:9px;line-height:1.4}.check,.inline-control{display:flex;align-items:center;gap:6px;color:#667a83;font-size:9px;white-space:nowrap}.number{width:76px;padding:7px}.status{position:static;overflow-wrap:anywhere;padding:8px 10px;border-radius:9px;font-size:9px;box-shadow:var(--shadow-md)}.status.error{border:1px solid #ead2d2;background:var(--color-danger-soft);color:var(--color-danger)}.status.ok{border:1px solid #d9e4e0;background:var(--color-success-soft);color:var(--color-success)}
  .settings-section :global(h2){font-size:14px}.settings-section :global(p){font-size:10px}.settings-section :global(.app-card){border-radius:12px;box-shadow:none}.settings-section :global(input),.settings-section :global(select),.settings-section :global(textarea){border-color:var(--color-border);border-radius:8px;background:#fbfcfc;font-size:10px}.settings-section :global(label),.settings-section :global(.muted){font-size:9px}
  @container(max-width:900px){.settings-layout{grid-template-columns:155px 1fr}.settings-nav a{padding-inline:8px}}
  @container(max-width:760px){.settings{height:auto}.settings-layout{grid-template-columns:1fr}.settings-nav{display:flex;overflow:auto}.settings-nav a{flex:0 0 auto}.settings-content{overflow:visible}.sync-strip{flex-wrap:wrap}.setting-row{align-items:flex-start;flex-direction:column;padding:10px 0}}
  .page-head p{font-size:12px}.settings-nav a{font-size:13px}.sync-strip,.setting-row small,.check,.inline-control,.status{font-size:11px}.sync-strip strong,.setting-row strong{font-size:12px}.section-heading h2,.settings-section :global(h2){font-size:17px}.section-heading p,.settings-section :global(p){font-size:12px}.settings-section :global(input),.settings-section :global(select),.settings-section :global(textarea){font-size:13px}.settings-section :global(label),.settings-section :global(.muted){font-size:11px}
  .build-id { margin-top: 10px; color: var(--color-primary); font-weight: 600; }
  .settings-content { overflow: visible; min-width: 0; padding: 0; }
  .settings-layout { align-items: start; }
  .settings-nav { position: sticky; top: 0; }
  .settings-section,.settings-nav a { min-width: 0; overflow-wrap: anywhere; }
  .page-head,.sync-strip { flex-wrap: wrap; }
  .setting-row { padding: 12px 0; flex-wrap: wrap; }
  @container(max-width:760px) { .settings-nav { position: static; flex-wrap: wrap; } }
</style>
