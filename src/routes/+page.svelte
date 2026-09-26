<script lang="ts">
  import "$lib/styles/app.css";
  import { onMount, tick } from "svelte";
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
  let history=$state<{destination:Destination;scroll:number}[]>([]);
  function go(target:Destination) { history=[...history.slice(-19),{destination:{...destination},scroll:document.querySelector('.content-scroll')?.scrollTop??0}];destination=target;routeSerial++; }
  async function back(){const previous=history.at(-1);if(!previous)return;history=history.slice(0,-1);destination=previous.destination;routeSerial++;await tick();const scroll=document.querySelector('.content-scroll');if(scroll)scroll.scrollTop=previous.scroll;}
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

  function onSearchSelect(target:Destination) {
    navigateTo(target);
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
    window.addEventListener('dashboard:back',back);
    return () => {window.removeEventListener("dashboard:navigate", handler);window.removeEventListener('dashboard:back',back);};
  });
</script>

<div class="app-shell">
  <aside class="sidebar" aria-label={tt("shell.primaryNavigation")}>
    <div class="brand">
      <span class="brand-mark" aria-hidden="true"><img src="/brand/msl-loop.svg" alt="" width="42" height="42"/></span>
      <span class="brand-copy"><strong>MSL Desktop</strong><small>{tt("app.name")}</small></span>
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
      <div class="topbar-title">{#if history.length}<button class="icon-button navigation-back" data-testid="navigation-back" onclick={back} aria-label={currentLocale==='en-US'?'Back':'返回上一页'}>←</button>{/if}<div class="topbar-copy"><strong>{viewTitle(view)}</strong><span>{formattedDate}</span></div></div>
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
        {:else if view === "workspace"}<WorkspaceView workspaceId={destination.workspaceId??null} relativePath={destination.relativePath??null} filePath={destination.filePath??null} activityId={destination.activityId??null}/>
        {:else if view === "works"}<WorksView focusId={destination.id??null} resumeId={destination.resumeId??null} onprojectchange={id=>{destination={...destination,id};}}/>
        {:else if view === "matters"}<MattersView section={destination.section??'inbox'} focusId={destination.id??null} workId={destination.workId??null} runId={destination.runId??null}/>
        {:else if view === "calendar"}<CalendarView focusId={destination.id??null}/>
        {:else if view === "settings"}<SettingsView />
        {:else if view === "reports"}<ReportsView focusId={destination.id??null}/>
        {:else if view === "qa"}<QaView focusId={destination.id??null} workId={destination.workId??null} expertId={destination.expertId??null}/>
        {:else if view === "kol"}<KolView focusId={destination.id??null} noteId={destination.noteId??null} insightId={destination.insightId??null} focusDraftId={destination.draftId??null} materialId={destination.materialId??null}/>
        {:else if view === "translation"}<TranslationView />{/if}
        {/key}
      </div>
    </div>
  </main>

  <SearchOverlay bind:open={searchOpen} onSelect={onSearchSelect} />
  <ConfirmDialog open={false} />
</div>
<style>
  .app-shell{display:flex;min-width:0;color:var(--color-text);background:var(--color-bg)}
  .sidebar{width:var(--sidebar-width);flex:0 0 var(--sidebar-width);padding:24px 14px 16px;display:flex;flex-direction:column;background:linear-gradient(180deg,var(--color-sidebar),var(--color-surface-muted));border-right:1px solid var(--color-border)}
  .brand{min-height:80px;display:flex;align-items:center;gap:10px;padding:0 5px 20px;flex-shrink:0}
  .brand-mark{width:42px;height:42px;flex-shrink:0}.brand-mark img{width:100%;height:100%;display:block}
  .brand-copy{display:grid;gap:4px;min-width:0}.brand-copy strong{font-size:16px;font-weight:700;letter-spacing:-.025em;white-space:nowrap}.brand-copy small{font-size:11px;color:var(--color-muted);letter-spacing:.05em}
  .nav-groups{display:grid;gap:22px;align-content:start;min-height:0;overflow:auto;flex:1;margin:12px 0 20px;scrollbar-width:thin}
  .nav-group{display:grid;gap:5px}.nav-group-label{padding:0 13px 7px;font-size:11px;letter-spacing:.12em;color:var(--color-muted)}
  .nav-item{position:relative;display:flex;align-items:center;gap:12px;width:100%;min-height:44px;padding:10px 13px;border:1px solid transparent;border-radius:7px;background:transparent;color:var(--color-muted);font-size:14px;line-height:1.5;text-align:left;cursor:pointer;transition:color .15s,background .15s}
  .nav-item:hover{color:var(--color-text);background:color-mix(in srgb,var(--color-surface) 70%,transparent)}
  .nav-item.active{background:color-mix(in srgb,var(--color-primary) 9%,var(--color-sidebar));color:var(--color-text);font-weight:650}
  .nav-item.active::before{content:"";position:absolute;left:0;top:11px;bottom:11px;width:3px;border-radius:3px;background:var(--color-primary)}
  .nav-icon{width:20px;display:grid;place-items:center;flex-shrink:0;color:var(--color-muted)}.nav-item.active .nav-icon{color:var(--color-primary)}.nav-label{min-width:0;overflow-wrap:anywhere}
  .main-content{flex:1;min-width:0;min-height:0;display:flex;flex-direction:column;overflow:hidden;background:var(--color-content)}
  .topbar{min-height:74px;flex-shrink:0;display:grid;grid-template-columns:minmax(125px,.7fr) minmax(260px,1.3fr) auto;align-items:center;gap:16px;padding:14px 28px;background:var(--color-surface);border-bottom:1px solid var(--color-border)}
  .topbar-title{display:flex;gap:10px;align-items:center;min-width:0}.topbar-copy{display:grid;gap:3px;min-width:0}.topbar-title strong{font-size:17px;font-weight:650}.topbar-title span{font-size:12px;color:var(--color-muted);line-height:1.5}.topbar-title .navigation-back{flex:0 0 32px;width:32px;height:32px;min-width:32px;min-height:32px;padding:0}
  .capture-wrap{display:flex;align-items:center;gap:9px;min-width:0;min-height:40px;padding:0 5px 0 12px;border:1px solid var(--color-border);border-radius:8px;color:var(--color-muted);background:var(--color-surface-muted)}
  .capture-wrap :global(.quick-capture){min-width:0;height:38px;padding-left:0;border:0;background:transparent;box-shadow:none;font-size:13px}
  .topbar-actions{display:flex;align-items:center;gap:10px}.icon-button{height:38px;display:flex;align-items:center;justify-content:center;gap:6px;padding:0 10px;border:1px solid var(--color-border);border-radius:8px;background:var(--color-surface);color:var(--color-muted);cursor:pointer}.icon-button:hover{color:var(--color-primary);border-color:var(--color-border-strong)}
  .locale-button{min-width:54px;font-size:12px}.shortcut{font-size:10px;color:var(--color-muted)}.avatar{width:32px;height:32px;display:grid;place-items:center;border-radius:50%;background:var(--color-primary);color:white}
  .content-scroll{min-height:0;flex:1;overflow:auto;padding:26px 28px 30px}.dashboard-shell{padding-top:10px}
  .view-body{width:min(100%,1540px);margin:0 auto;min-height:100%;container-type:inline-size}
  .content-scroll.qa-shell{display:flex;overflow:hidden}.qa-shell .view-body{flex:1;height:100%;min-height:0}
  @media(max-height:800px){.nav-groups{gap:14px;margin-top:2px}.brand{min-height:64px;padding-bottom:12px}.sidebar{padding-top:16px}}
  @media(max-width:1080px){.topbar{gap:10px;padding-inline:22px}.shortcut,.avatar{display:none}.content-scroll{padding-inline:22px}}
  @media(max-width:760px){.sidebar{width:var(--sidebar-collapsed-width);flex-basis:var(--sidebar-collapsed-width);padding-inline:8px}.brand{justify-content:center;padding-inline:0}.brand-copy,.nav-label,.nav-group-label{display:none}.nav-item{justify-content:center;padding-inline:0}.nav-groups{gap:16px}.topbar{grid-template-columns:1fr auto;padding:12px 16px;gap:8px}.capture-wrap{grid-column:1/-1;grid-row:2}.topbar-title{display:flex;gap:10px;align-items:baseline}.content-scroll{padding:16px}.dashboard-shell{padding-top:6px}.icon-button{height:34px}}
  @media(max-height:560px){.content-scroll.qa-shell{overflow:auto}.qa-shell .view-body{height:620px;min-height:620px}}
</style>
