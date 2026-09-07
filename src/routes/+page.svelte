<script lang="ts">
  import "$lib/styles/app.css";
  import { onMount } from "svelte";
  import QuickCapture from "$lib/components/QuickCapture.svelte";
  import BackgroundJobs from "$lib/components/BackgroundJobs.svelte";
  import PlanView from "$lib/components/PlanView.svelte";
  import WaitingView from "$lib/components/WaitingView.svelte";
  import InboxView from "$lib/components/InboxView.svelte";
  import CalendarView from "$lib/components/CalendarView.svelte";
  import WorksView from "$lib/components/WorksView.svelte";
  import TodayView from "$lib/components/TodayView.svelte";
  import MattersView from "$lib/components/MattersView.svelte";
  import { resolveDestination, navigateTo, type Destination, type View } from '$lib/services/navigation';
  import SearchOverlay from "$lib/components/SearchOverlay.svelte";
  import SettingsView from "$lib/components/SettingsView.svelte";
  import WorkspaceView from "$lib/components/WorkspaceView.svelte";
  import AiReviewCenter from "$lib/components/AiReviewCenter.svelte";
  import TranslationView from "$lib/components/TranslationView.svelte";
  import ReportsView from "$lib/components/ReportsView.svelte";
  import QaView from "$lib/components/QaView.svelte";
  import KolView from "$lib/components/KolView.svelte";
  import ToastHost from "$lib/components/ui/ToastHost.svelte";
  import ConfirmDialog from "$lib/components/ui/ConfirmDialog.svelte";
  import Icon, { type IconName } from "$lib/components/ui/Icon.svelte";
  import { locale, setLocale, t, type TranslationKey } from "$lib/i18n";
  import { initializeAppearance } from "$lib/stores/appearance";

  type NavItem = { view: View; label: TranslationKey; icon: IconName; testid: string };

  const workbenchNav: NavItem[] = [
    { view: "today", label: "nav.today", icon: "dashboard", testid: "nav-today" },
    { view: "works", label: "nav.works", icon: "work", testid: "nav-works" },
    { view: "kol", label: "nav.kol", icon: "user", testid: "nav-kol" },
    { view: "matters", label: "nav.plan", icon: "check", testid: "nav-matters" },
    { view: "calendar", label: "nav.calendar", icon: "calendar", testid: "nav-calendar" },
    { view: "reports", label: "nav.reports", icon: "reports", testid: "nav-reports" },
  ];
  const secretaryNav: NavItem[] = [
    { view: "qa", label: "nav.qa", icon: "search", testid: "nav-qa" },
    { view: "translation", label: "nav.translation", icon: "languages", testid: "nav-translation" },
    { view: "workspace", label: "nav.workspace", icon: "folder", testid: "nav-workspace" },
  ];

  let destination = $state<Destination>({view:'today'});
  let view = $derived(destination.view);
  let routeSerial=$state(0);
  function go(target:Destination) { destination=target;routeSerial++; }
  let searchOpen = $state(false);
  let currentLocale = $derived($locale);
  let formattedDate = $derived(new Intl.DateTimeFormat(currentLocale, { month: "long", day: "numeric", weekday: "long" }).format(new Date()));

  function tt(key: TranslationKey, params: Record<string, string | number> = {}): string {
    return t(key, params, currentLocale);
  }

  function viewTitle(value: View): string {
    const en=currentLocale==='en-US';
    const names:Partial<Record<View,string>>={today:en?'Today':'今天',works:en?'Projects':'项目',matters:en?'Matters':'事项',reports:en?'Review & reports':'回顾'};
    if(names[value])return names[value]!;
    const key: Record<View, TranslationKey> = {
      today: "nav.today", workspace: "nav.workspace", works: "nav.works", matters: "nav.plan",
      calendar: "nav.calendar", settings: "nav.settings", translation: "nav.translation", reports: "nav.reports", qa: "nav.qa", kol: "nav.kol"
    };
    return tt(key[value]);
  }

  function onSearchSelect(kind: string, id: number) {
    navigateTo(kind,id);
  }

  async function toggleLocale() {
    await setLocale(currentLocale === "zh-CN" ? "en-US" : "zh-CN");
  }

  onMount(() => {
    void initializeAppearance();
    const handler = (event: Event) => {
      const raw=(event as CustomEvent<string|Destination>).detail;
      const next=typeof raw==='string'?resolveDestination(raw):raw&&resolveDestination(raw.view)?raw:null;
      if(next)go(next);
    };
    window.addEventListener("dashboard:navigate", handler);
    return () => window.removeEventListener("dashboard:navigate", handler);
  });
</script>

<div class="app-shell">
  <aside class="sidebar" aria-label={tt("shell.primaryNavigation")}>
    <div class="brand">
      <span class="brand-mark" aria-hidden="true"><svg viewBox="0 0 64 64"><path class="logo-tile" d="M14 5h36a9 9 0 0 1 9 9v36a9 9 0 0 1-9 9H14a9 9 0 0 1-9-9V14a9 9 0 0 1 9-9Z"/><path class="logo-ribbon" d="M17 45V20c0-2 1-3 3-3h2l10 12 10-12h2c2 0 3 1 3 3v25M32 29v16"/><path class="logo-spark" d="m50 8 1.5 4 4 1.5-4 1.5-1.5 4-1.5-4-4-1.5 4-1.5L50 8Z"/></svg></span>
      <span class="brand-copy"><strong>MSL DESKTOP</strong><small>{tt("app.name")}</small></span>
    </div>

    <nav class="nav-groups">
      <section class="nav-group" data-testid="primary-navigation">
        <div class="nav-group-label">{tt("shell.workbench")}</div>
        {#each workbenchNav as item (item.view)}
          <button data-testid={item.testid} class="nav-item" class:active={view === item.view} onclick={() => go({view:item.view})} title={viewTitle(item.view)} aria-current={view === item.view ? "page" : undefined}>
            <span class="nav-icon"><Icon name={item.icon} size={17} /></span><span class="nav-label">{viewTitle(item.view)}</span>
          </button>
        {/each}
      </section>
      <section class="nav-group" data-testid="tools-navigation">
        <div class="nav-group-label">{currentLocale==='en-US'?'Tools':'工具'}</div>
        {#each secretaryNav as item (item.view)}
          <button data-testid={item.testid} class="nav-item" class:active={view === item.view} onclick={() => go({view:item.view})} title={tt(item.label)} aria-current={view === item.view ? "page" : undefined}>
            <span class="nav-icon"><Icon name={item.icon} size={17} /></span><span class="nav-label">{tt(item.label)}</span>
          </button>
        {/each}
      </section>
      <section class="nav-group system-group">
        <div class="nav-group-label">{tt("shell.system")}</div>
        <button data-testid="nav-settings" class="nav-item" class:active={view === "settings"} onclick={() => go({view:'settings'})} title={tt("nav.settings")} aria-current={view === "settings" ? "page" : undefined}>
          <span class="nav-icon"><Icon name="settings" size={17} /></span><span class="nav-label">{tt("nav.settings")}</span>
        </button>
      </section>
    </nav>

    <BackgroundJobs />
  </aside>

  <main class="main-content">
    <header class="topbar">
      <div class="topbar-title"><strong>{viewTitle(view)}</strong><span>{formattedDate}</span></div>
      <div class="capture-wrap"><Icon name="capture" size={15} /><QuickCapture workId={view==='works'?destination.id??null:null}/></div>
      <div class="topbar-actions">
        <button class="icon-button search-button" type="button" onclick={() => (searchOpen = true)} aria-label={tt("common.openSearch")} title={tt("common.openSearch")}>
          <Icon name="search" size={17} /><span class="shortcut">Ctrl K</span>
        </button>
        <button class="icon-button locale-button" type="button" onclick={toggleLocale} aria-label={tt("common.toggleLanguage")} title={tt("common.toggleLanguage")}>
          <Icon name="languages" size={17} /><span>{currentLocale === "zh-CN" ? "中" : "EN"}</span>
        </button>
        <div class="avatar" title="MSL"><Icon name="user" size={15} /></div>
      </div>
    </header>

    <ToastHost />
    <div class:dashboard-shell={view === "today"} class:qa-shell={view === "qa"} class="content-scroll">
      <div class="view-body">
        {#key routeSerial}
        {#if view === "today"}<TodayView />
        {:else if view === "workspace"}<WorkspaceView />
        {:else if view === "works"}<WorksView focusId={destination.id??null} resumeId={destination.resumeId??null} onprojectchange={id=>{destination={...destination,id};}}/>
        {:else if view === "matters"}<MattersView section={destination.section??'inbox'} focusId={destination.id??null}/>
        {:else if view === "calendar"}<CalendarView focusId={destination.id??null}/>
        {:else if view === "settings"}<SettingsView />
        {:else if view === "reports"}<ReportsView />
        {:else if view === "qa"}<QaView focusId={destination.id??null} workId={destination.workId??null}/>
        {:else if view === "kol"}<KolView focusId={destination.id??null}/>
        {:else if view === "translation"}<TranslationView />{/if}
        {/key}
      </div>
    </div>
  </main>

  <SearchOverlay bind:open={searchOpen} onSelect={onSearchSelect} />
  <ConfirmDialog open={false} />
</div>

<style>
  /* The shared stylesheet owns zoom AND compensated viewport dimensions. */
  .app-shell { display: flex; min-width: 0; color: var(--color-text); background: var(--color-bg); }
  .sidebar { width: var(--sidebar-width); flex: 0 0 var(--sidebar-width); padding: 18px 13px 13px; display: flex; flex-direction: column; background: var(--color-sidebar); border-right: 1px solid #d4dde1; }
  .brand { min-height: 48px; flex-shrink: 0; display: flex; align-items: center; gap: 10px; padding: 0 9px; margin-bottom: 12px; }
  .brand-mark { width: 34px; height: 34px; display: grid; place-items: center; flex: 0 0 auto; }
  .brand-mark svg{width:34px;height:34px;overflow:visible}.logo-tile{fill:#f7fafb;stroke:#8ea5ae;stroke-width:1.4}.logo-ribbon{fill:none;stroke:#567481;stroke-width:5.5;stroke-linecap:round;stroke-linejoin:round}.logo-spark{fill:#78939e}
  .brand-copy { display: grid; min-width: 0; }
  .brand-copy strong { font-size: 13px; letter-spacing: .08em; white-space: nowrap; }
  .brand-copy small { margin-top: 2px; color: var(--color-muted); font-size: 9px; }
  .nav-groups { display: grid; gap: 13px; min-height: 0; flex:1; align-content:start; overflow-y:auto; margin-bottom:12px; scrollbar-width:thin; }
  .nav-group { display: grid; gap: 3px; }
  .nav-group-label { padding: 0 11px 5px; color: #87969d; font-size: 9px; font-weight: 650; letter-spacing: .12em; text-transform: uppercase; }
  .nav-item { width: 100%; height: 38px; display: flex; align-items: center; gap: 10px; border: 0; border-radius: 10px; padding: 0 11px; background: transparent; color: #566971; font-size: 12px; text-align: left; cursor: pointer; transition: background .16s ease, color .16s ease, box-shadow .16s ease; }
  .nav-item:hover { background: rgb(255 255 255 / .38); color: var(--color-text); }
  .nav-item.active { background: rgb(255 255 255 / .74); color: #2e424c; box-shadow: 0 4px 13px rgb(51 73 82 / .05); font-weight: 650; }
  .nav-icon { width: 18px; display: grid; place-items: center; flex: 0 0 auto; color: #78909a; }
  .nav-item.active .nav-icon { color: #587582; }
  .system-group { margin-top: 1px; }
  .main-content { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; overflow: hidden; background: var(--color-content); }
  .topbar { height: var(--topbar-height); flex: 0 0 var(--topbar-height); display: grid; grid-template-columns: minmax(150px, 1fr) minmax(260px, 430px) auto; align-items: center; gap: 18px; padding: 0 24px; background: rgb(251 252 252 / .94); border-bottom: 1px solid var(--color-border); }
  .topbar-title { min-width: 0; display: flex; align-items: baseline; gap: 9px; white-space: nowrap; overflow: hidden; }
  .topbar-title strong { font-size: 14px; font-weight: 650; }.topbar-title span{color:var(--color-muted);font-size:10px;overflow:hidden;text-overflow:ellipsis}
  .capture-wrap { min-width: 0; height: 36px; display: flex; align-items: center; gap: 8px; padding-left: 11px; border: 1px solid var(--color-border); border-radius: 10px; background: #f8fafa; color: #7e9097; }
  .capture-wrap :global(.quick-capture) { min-width: 0; height: 34px; padding: 0 10px 0 0; border: 0; background: transparent; box-shadow: none; font-size: 11px; }
  .topbar-actions { display: flex; align-items: center; gap: 7px; }
  .icon-button { height: 34px; display: inline-flex; align-items: center; justify-content: center; gap: 7px; border: 1px solid var(--color-border); border-radius: 10px; background: #fafcfc; color: #71848c; cursor: pointer; }
  .icon-button:hover { border-color: var(--color-border-strong); color: #526b76; background: white; }
  .search-button { padding: 0 8px 0 10px; }.locale-button{min-width:42px;padding:0 8px;font-size:10px}.shortcut{border-left:1px solid var(--color-border);padding-left:7px;color:#8b999f;font-size:9px}
  .avatar { width: 32px; height: 32px; display: grid; place-items: center; border-radius: 10px; background: #dbe5e8; color: #58717c; }
  .content-scroll { min-height: 0; flex: 1; overflow: auto; padding: 20px 22px 24px; }
  .dashboard-shell { padding: 16px 20px 24px; }
  .view-body { width: min(100%, 1440px); margin: 0 auto; min-height: 100%; }
  .view-body { container-type: inline-size; }
  .content-scroll.qa-shell { display:flex; overflow:hidden; }
  .qa-shell .view-body { flex:1; height:100%; min-height:0; }
  @media(max-height:560px){.content-scroll.qa-shell{overflow:auto}.qa-shell .view-body{height:620px;min-height:620px}}
  @media (max-width: 760px) {
    .sidebar { width: var(--sidebar-collapsed-width); flex-basis: var(--sidebar-collapsed-width); padding-inline: 8px; }
    .brand { justify-content: center; padding-inline: 0; }.brand-copy,.nav-label,.nav-group-label{display:none}.nav-item{justify-content:center;padding-inline:0}.topbar{grid-template-columns:minmax(130px,1fr) minmax(220px,350px) auto;padding-inline:18px}.shortcut{display:none}
  }
  @media (max-width: 760px) {
    .topbar { grid-template-columns: 1fr auto; gap: 10px; padding-inline: 14px; }.capture-wrap{grid-row:2;grid-column:1/-1;margin-bottom:8px}.topbar{height:102px;flex-basis:102px}.topbar-title span,.avatar{display:none}.content-scroll{padding:14px}.dashboard-shell{overflow:auto}.search-button{width:34px;padding:0}
  }
  @media (max-width: 1180px), (max-height: 720px) {
    .dashboard-shell { overflow: auto; }
    .dashboard-shell .view-body { height: auto; min-height: 100%; }
  }
  /* C1 readability pass */
  .brand-copy small,.nav-group-label,.shortcut{font-size:11px}
  .nav-item{font-size:13px}
  .topbar-title strong{font-size:16px}.topbar-title span{font-size:12px}
  .capture-wrap :global(.quick-capture){font-size:13px}.locale-button{font-size:12px}

  .nav-item{height:auto;min-height:43px;font-size:15px;line-height:1.5;padding-block:8px}.nav-label{white-space:normal}.topbar{min-height:76px;height:auto;flex-basis:auto;grid-template-columns:minmax(90px,.65fr) minmax(240px,1.4fr) auto;gap:12px;padding-block:10px}.topbar-title{white-space:normal;flex-wrap:wrap;gap:3px 10px}.topbar-title span{white-space:normal}.capture-wrap{height:auto;min-height:38px;padding-right:5px}.view-body{max-width:1380px}.brand-copy strong{font-size:12px}.brand-copy small{font-size:12px}
  @media(max-width:760px){.topbar{grid-template-columns:1fr auto}.capture-wrap{grid-column:1/-1}.nav-item{justify-content:center}}
</style>
