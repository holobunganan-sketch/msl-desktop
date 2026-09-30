<script lang="ts">
  import {onMount} from "svelte";
  import StatusLine from "$lib/components/ui/StatusLine.svelte";
  import { command, normalizeError } from "$lib/services/api";
  import type { AiTaskRoute, ProviderConnection, ProviderModel } from "$lib/types/domain";
  import { locale, t } from "$lib/i18n";
  import { addToast } from "$lib/stores/toast";
  import { chatModels, taskModelId } from "$lib/services/qaChat";

  const taskKinds = ["workspace_analysis", "work_draft", "global_analysis", "daily_brief", "weekly_report", "monthly_report", "translation", "workbench_qa", "kol_analysis"] as const;
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
  let availableModels = $derived(chatModels(connections, models));
  function unavailable(id: number | null) { return id !== null && !availableModels.some(model => model.id === id); }
  function effectiveLabel(kind: string) {
    const id = taskModelId(routes, kind);
    return `${modelLabel(id)}${unavailable(id) ? ` · ${tt("settings.unavailable")}` : ""}`;
  }
  async function save(kind: string, value: string) {
    saving = kind; error = "";
    try { await command("save_ai_task_route", { taskKind: kind, providerModelId: value ? Number(value) : null }); await load(); addToast(tt("settings.routeSaved"), "success"); }
    catch (value) { error = normalizeError(value); addToast(error, "error"); }
    finally { saving = null; }
  }
  onMount(()=>{void load();const timer=setInterval(()=>{if(!saving)void load();},4000);return()=>clearInterval(timer);});
</script>

<section class="routing" data-testid="ai-routing">
  <header class="routing-head">
    <div class="routing-heading"><h2>{tt("settings.aiRouting")}</h2><div class="routing-status"><StatusLine message={error} onclear={() => error = ""}/></div></div>
    <p>{tt("settings.aiRoutingHint")}</p>
  </header>
  {#snippet selector(kind: string)}
    <select data-testid={`route-${kind}`} value={routeFor(kind) ?? ""} onchange={event => save(kind, event.currentTarget.value)} disabled={saving !== null} aria-label={tt(taskLabel[kind] as Parameters<typeof t>[0])} title={effectiveLabel(kind)}>
      <option value="">{tt(kind === "general" ? "settings.routeUnconfigured" : "settings.routeFollowDefault")}</option>
      {#if unavailable(routeFor(kind))}<option value={routeFor(kind) ?? ""} disabled>{modelLabel(routeFor(kind))} · {tt("settings.unavailable")}</option>{/if}
      {#each availableModels as model (model.id)}<option value={model.id}>{modelLabel(model.id)} · {model.protocol}</option>{/each}
    </select>
    <span class="route-saving" aria-live="polite">{saving === kind ? tt("common.loading") : ""}</span>
  {/snippet}
  <div class="route-row global-default" data-testid="global-model-row">
    <div class="route-copy"><strong>{tt("settings.routeGeneral")}</strong><small>{tt("settings.routeDefaultHint")}</small></div>
    {@render selector("general")}
  </div>
  <div class="route-list">
    {#each taskKinds as kind}
      <div class="route-row">
        <div class="route-copy"><strong>{tt(taskLabel[kind] as Parameters<typeof t>[0])}</strong><small>{tt(routeFor(kind) === null ? "settings.routeInherited" : "settings.routeDedicated")} · {effectiveLabel(kind)}</small></div>
        {@render selector(kind)}
      </div>
    {/each}
  </div>
</section>

<style>
  .routing{display:grid;gap:12px;min-width:0;container-type:inline-size}
  .routing-heading{display:flex;align-items:center;gap:20px;min-width:0}
  .routing-head h2{margin:0;font-size:18px;flex-shrink:0}
  .routing-head p{margin:4px 0 0;color:var(--color-muted);font-size:14px;line-height:1.6}
  .routing-status{flex:1;min-width:0;max-width:440px;margin-left:auto}
  .route-list{display:grid;min-width:0}
  .route-row{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1.25fr) 64px;align-items:center;gap:12px;border-top:1px solid var(--color-border);padding:12px 0;min-width:0}
  .route-copy{min-width:0;overflow-wrap:anywhere}.route-copy strong{font-size:15px}.route-copy small{display:block;color:var(--color-muted);font-size:13px;line-height:1.5;margin-top:4px}
  .global-default{border:1px solid var(--color-border);background:var(--color-primary-soft,#edf5f7);border-radius:var(--radius-sm);padding:16px}
  select{width:100%;min-width:0;max-width:100%;min-height:44px;border:1px solid var(--color-border);border-radius:var(--radius-sm);padding:8px 12px;background:var(--color-card,#fff);color:var(--color-text);font-size:14px;text-overflow:ellipsis}
  .route-saving{color:var(--color-muted);font-size:12px;min-width:0}
  @container(max-width:600px){.route-row{grid-template-columns:minmax(0,1fr) 64px;gap:8px}.route-copy{grid-column:1/-1}.routing-heading{gap:12px}.routing-head h2{font-size:17px}}
</style>
