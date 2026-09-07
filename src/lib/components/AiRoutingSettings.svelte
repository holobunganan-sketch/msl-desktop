<script lang="ts">
  import { command, normalizeError } from "$lib/services/api";
  import type { AiTaskRoute, ProviderConnection, ProviderModel } from "$lib/types/domain";
  import { locale, t } from "$lib/i18n";
  import { addToast } from "$lib/stores/toast";

  const taskKinds = ["workspace_analysis", "work_draft", "global_analysis", "daily_brief", "weekly_report", "monthly_report", "translation", "workbench_qa", "kol_analysis", "general"] as const;
  let connections = $state<ProviderConnection[]>([]);
  let models = $state<ProviderModel[]>([]);
  let routes = $state<AiTaskRoute[]>([]);
  let error = $state("");
  let saving = $state<string | null>(null);
  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);
  const taskLabel: Record<string, string> = { workspace_analysis: "settings.routeWorkspace", work_draft: "settings.routeDraft", global_analysis: "settings.routeGlobal", daily_brief: "settings.routeBrief", weekly_report: "settings.routeWeekly", monthly_report: "settings.routeMonthly", translation: "settings.routeTranslation", workbench_qa:"settings.routeQa",kol_analysis:"settings.routeKol",general: "settings.routeGeneral" };

  async function load() {
    try {
      [connections, models, routes] = await Promise.all([
        command<ProviderConnection[]>("list_provider_connections"),
        command<ProviderModel[]>("list_provider_models"),
        command<AiTaskRoute[]>("list_ai_task_routes")
      ]);
    } catch (value) { error = normalizeError(value); }
  }
  function routeFor(kind: string) { return routes.find((route) => route.task_kind === kind)?.provider_model_id ?? null; }
  function modelLabel(id: number | null) { const model = models.find((item) => item.id === id); if (!model) return tt("settings.routeUnconfigured"); const provider = connections.find((item) => item.id === model.provider_id); return `${provider?.display_name ?? "?"} · ${model.display_name}`; }
  function modelOptions() { return models.filter((model) => model.enabled); }
  async function save(kind: string, value: string) {
    saving = kind; error = "";
    try { await command("save_ai_task_route", { taskKind: kind, providerModelId: value ? Number(value) : null }); await load(); addToast(tt("settings.routeSaved"), "success"); }
    catch (value) { error = normalizeError(value); addToast(error, "error"); }
    finally { saving = null; }
  }
  $effect(() => { $locale; if (!connections.length && !models.length) void load(); });
</script>

<section class="routing"><div class="section-head"><div><h2>{tt("settings.aiRouting")}</h2><p>{tt("settings.aiRoutingHint")}</p></div></div>{#if error}<div class="error" role="alert">{error}</div>{/if}<div class="route-list">{#each taskKinds as kind}<div class="route-row"><div><strong>{tt(taskLabel[kind] as Parameters<typeof t>[0])}</strong><small>{modelLabel(routeFor(kind))}</small></div><select value={routeFor(kind) ?? ""} onchange={(event) => save(kind, (event.currentTarget as HTMLSelectElement).value)} disabled={saving === kind} aria-label={tt(taskLabel[kind] as Parameters<typeof t>[0])}><option value="">{tt("settings.routeUnconfigured")}</option>{#each modelOptions() as model}<option value={model.id}>{modelLabel(model.id)} · {model.protocol}{model.available ? "" : ` · ${tt("settings.unavailable")}`}</option>{/each}</select>{#if saving === kind}<span class="muted">{tt("common.loading")}</span>{/if}</div>{/each}</div></section>

<style>
  .routing { display:grid; gap:10px; } .section-head h2 { margin:0 0 4px; font-size:14px; } .section-head p { margin:0; color:var(--color-muted); font-size:10px; } .route-list { display:grid; gap:2px; } .route-row { display:grid; grid-template-columns:minmax(0,1fr) minmax(220px,360px) auto; align-items:center; gap:10px; border-top:1px solid var(--color-border); padding:9px 0; } .route-row strong{font-size:10px}.route-row small { display:block; color:var(--color-muted); font-size:8px; margin-top:3px; } select { width:100%; border:1px solid var(--color-border); border-radius:var(--radius-sm); padding:8px; background:#fbfcfc; color:var(--color-text);font-size:9px } .muted { color:var(--color-muted); font-size:9px; } .error { padding:8px 10px; border:1px solid #ead2d2; background:var(--color-danger-soft); color:var(--color-danger); border-radius:var(--radius-sm); font-size:10px; } @media(max-width:720px){ .route-row { grid-template-columns:1fr; } }
  .section-head h2{font-size:17px}.section-head p,.route-row strong{font-size:12px}.route-row small,.muted{font-size:11px}select{font-size:12px}
</style>
