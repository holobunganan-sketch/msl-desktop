<script lang="ts">
  import { command, normalizeError } from "$lib/services/api";
  import type { ProviderConnection, ProviderModel } from "$lib/types/domain";
  import { locale, t } from "$lib/i18n";
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import AppCard from "$lib/components/ui/AppCard.svelte";
  import { addToast } from "$lib/stores/toast";

  let connections = $state<ProviderConnection[]>([]);
  let models = $state<Record<number, ProviderModel[]>>({});
  let keyStatus = $state<Record<number, boolean>>({});
  let loading = $state(false);
  let busy = $state<string | null>(null);
  let error = $state("");
  let customOpen = $state(false);
  let customName = $state("");
  let customUrl = $state("");
  let customAuth = $state("bearer");
  let customKey = $state("");
  let templateKeys = $state<Record<"deepseek" | "opencode_go" | "muse", string>>({ deepseek: "", opencode_go: "", muse: "" });
  let connectionKeys = $state<Record<number, string>>({});
  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);

  async function load() {
    loading = true; error = "";
    try {
      connections = await command<ProviderConnection[]>("list_provider_connections");
      const nextModels: Record<number, ProviderModel[]> = {};
      const nextKeys: Record<number, boolean> = {};
      for (const connection of connections) {
        nextModels[connection.id] = await command<ProviderModel[]>("list_provider_models", { providerId: connection.id });
        nextKeys[connection.id] = await command<boolean>("provider_has_key", { id: connection.id });
      }
      models = nextModels; keyStatus = nextKeys;
    } catch (value) { error = normalizeError(value); }
    finally { loading = false; }
  }

  async function createTemplate(kind: "deepseek" | "opencode_go") {
    const apiKey = templateKeys[kind];
    if (!apiKey.trim()) { error = tt("settings.apiKeyRequired"); return; }
    busy = kind; error = "";
    try {
      const existing = connections.find((connection) => connection.template_kind === kind);
      if (existing) {
        await persistConnectionKey(existing, apiKey);
        addToast(tt("settings.keySaved"), "success");
      } else {
        await command("create_provider_template", { templateKind: kind, apiKey, enabled: true });
        addToast(tt("settings.providerCreated"), "success");
      }
      templateKeys[kind] = "";
      await load();
    } catch (value) { error = normalizeError(value); addToast(error, "error"); }
    finally { busy = null; }
  }

  async function configureMuse() {
    const apiKey = templateKeys.muse;
    if (!apiKey.trim()) { error = tt("settings.apiKeyRequired"); return; }
    busy = "muse"; error = "";
    try {
      let connection = connections.find((item) => item.template_kind === "opencode_go");
      if (connection) {
        await persistConnectionKey(connection, apiKey);
      } else {
        connection = await command<ProviderConnection>("create_provider_template", { templateKind: "opencode_go", apiKey, enabled: true });
      }
      const providerModels = await command<ProviderModel[]>("list_provider_models", { providerId: connection.id });
      const muse = providerModels.find((model) => model.model_id === "muse-spark-1.2-contributor");
      if (!muse) throw new Error(tt("settings.museMissing"));
      if (!muse.enabled) await command("set_provider_model_enabled", { id: muse.id, enabled: true });
      templateKeys.muse = "";
      addToast(tt("settings.museConfigured"), "success");
      await load();
    } catch (value) { error = normalizeError(value); addToast(error, "error"); }
    finally { busy = null; }
  }

  function persistConnectionKey(connection: ProviderConnection, apiKey: string) {
    return command<ProviderConnection>("save_provider_connection", {
      id: connection.id,
      displayName: connection.display_name,
      providerType: connection.provider_type,
      baseUrl: connection.base_url,
      legacyModel: connection.legacy_model,
      templateKind: connection.template_kind,
      authMode: connection.auth_mode,
      modelsEndpoint: connection.models_endpoint,
      enabled: connection.enabled,
      apiKey
    });
  }

  async function saveConnectionKey(connection: ProviderConnection) {
    const apiKey = connectionKeys[connection.id] ?? "";
    if (!apiKey.trim()) { error = tt("settings.apiKeyRequired"); return; }
    busy = `key-${connection.id}`; error = "";
    try {
      await persistConnectionKey(connection, apiKey);
      connectionKeys[connection.id] = "";
      addToast(tt("settings.keySaved"), "success");
      await load();
    } catch (value) { error = normalizeError(value); addToast(error, "error"); }
    finally { busy = null; }
  }

  async function saveCustom() {
    if (!customName.trim() || !customUrl.trim()) { error = tt("common.required"); return; }
    busy = "custom"; error = "";
    try {
      await command("save_provider_connection", {
        id: null, displayName: customName.trim(), providerType: "custom", baseUrl: customUrl.trim(), legacyModel: "",
        templateKind: "custom", authMode: customAuth, modelsEndpoint: null, enabled: true, apiKey: customKey
      });
      customOpen = false; customName = ""; customUrl = ""; customKey = "";
      addToast(tt("settings.saved"), "success");
      await load();
    } catch (value) { error = normalizeError(value); addToast(error, "error"); }
    finally { busy = null; }
  }

  async function refresh(connection: ProviderConnection) {
    busy = `refresh-${connection.id}`; error = "";
    try { await command("refresh_provider_models", { providerId: connection.id }); addToast(tt("settings.modelsRefreshed"), "success"); await load(); }
    catch (value) { error = normalizeError(value); addToast(error, "error"); }
    finally { busy = null; }
  }

  async function test(model: ProviderModel) {
    busy = `test-${model.id}`; error = "";
    try { await command("test_provider_model", { providerModelId: model.id }); addToast(tt("settings.testSuccess"), "success"); }
    catch (value) { error = normalizeError(value); addToast(error, "error"); }
    finally { busy = null; }
  }

  async function toggleModel(model: ProviderModel) {
    busy = `toggle-${model.id}`; error = "";
    try {
      await command("set_provider_model_enabled", { id: model.id, enabled: !model.enabled });
      addToast(model.enabled ? tt("settings.modelDisabled") : tt("settings.modelEnabled"), "success");
      await load();
    } catch (value) { error = normalizeError(value); addToast(error, "error"); }
    finally { busy = null; }
  }

  async function remove(connection: ProviderConnection) {
    if (!window.confirm(tt("settings.deleteMessage", { name: connection.display_name }))) return;
    busy = `delete-${connection.id}`; error = "";
    try { await command("delete_provider", { id: connection.id }); addToast(tt("settings.deleted"), "success"); await load(); }
    catch (value) { error = normalizeError(value); addToast(error, "error"); }
    finally { busy = null; }
  }

  function protocolLabel(model: ProviderModel) {
    return model.capabilities_json.includes("needs_protocol") ? tt("settings.needsProtocol") : model.protocol;
  }

  $effect(() => { $locale; if (!loading && connections.length === 0) void load(); });
</script>

<section class="provider-area">
  <div class="section-head"><div><h2>{tt("settings.providers")}</h2><p>{tt("settings.providerHint")}</p></div><AppButton variant="secondary" onclick={() => (customOpen = !customOpen)}>{tt("settings.customProvider")}</AppButton></div>
  {#if error}<div class="error" role="alert">{error}</div>{/if}
  {#if customOpen}<AppCard><div class="form-grid"><label>{tt("settings.displayName")}<input bind:value={customName} /></label><label>{tt("settings.baseUrl")}<input bind:value={customUrl} placeholder="https://example.com/v1" /></label><label>{tt("settings.authMode")}<select bind:value={customAuth}><option value="bearer">bearer</option><option value="api_key">api_key</option><option value="none">none</option></select></label><label>{tt("settings.apiKey")}<input type="password" bind:value={customKey} autocomplete="off" /></label></div><div class="form-actions"><AppButton loading={busy === "custom"} onclick={saveCustom}>{tt("common.save")}</AppButton><AppButton variant="ghost" onclick={() => (customOpen = false)}>{tt("common.cancel")}</AppButton></div></AppCard>{/if}
  <div class="template-choices">
    <AppCard>
      <div class="template-title">DeepSeek</div><p>{tt("settings.templateDeepSeek")}</p>
      <label class="key-field">{tt("settings.apiKey")}<input data-testid="template-deepseek-api-key" type="password" bind:value={templateKeys.deepseek} placeholder={tt("settings.apiKeyPlaceholder")} autocomplete="new-password" /></label>
      <AppButton loading={busy === "deepseek"} onclick={() => createTemplate("deepseek")}>{connections.some((item) => item.template_kind === "deepseek") ? tt("settings.saveKey") : tt("settings.useTemplate")}</AppButton>
    </AppCard>
    <AppCard>
      <div class="template-title">OpenCode Go</div><p>{tt("settings.templateOpenCode")}</p>
      <label class="key-field">{tt("settings.apiKey")}<input data-testid="template-opencode-go-api-key" type="password" bind:value={templateKeys.opencode_go} placeholder={tt("settings.apiKeyPlaceholder")} autocomplete="new-password" /></label>
      <AppButton loading={busy === "opencode_go"} onclick={() => createTemplate("opencode_go")}>{connections.some((item) => item.template_kind === "opencode_go") ? tt("settings.saveKey") : tt("settings.useTemplate")}</AppButton>
    </AppCard>
    <AppCard>
      <div class="template-title">Muse Spark 1.2 Contributor</div><p>{tt("settings.templateMuse")}</p>
      <label class="key-field">{tt("settings.museApiKey")}<input data-testid="template-muse-api-key" type="password" bind:value={templateKeys.muse} placeholder={tt("settings.apiKeyPlaceholder")} autocomplete="new-password" /></label>
      <div class="muse-template-note">{tt("settings.musePrivacy")}</div>
      <AppButton loading={busy === "muse"} onclick={configureMuse}>{tt("settings.configureMuse")}</AppButton>
    </AppCard>
  </div>
  {#if loading}<div class="muted">{tt("common.loading")}</div>{:else if connections.length === 0}<div class="muted">{tt("settings.noProviders")}</div>{/if}
  <div class="connection-list">{#each connections as connection (connection.id)}<AppCard><div class="connection-head"><div><strong>{connection.display_name}</strong><span class="tag">{connection.template_kind}</span><div class="muted">{connection.base_url}</div></div><div class="connection-actions"><span class:key-ok={keyStatus[connection.id]} class="key-state">{keyStatus[connection.id] ? tt("settings.configuredKey") : tt("settings.noKey")}</span>{#if connection.template_kind !== "custom"}<AppButton variant="secondary" loading={busy === `refresh-${connection.id}`} onclick={() => refresh(connection)}>{tt("settings.refreshModels")}</AppButton>{/if}<AppButton variant="ghost" loading={busy === `delete-${connection.id}`} onclick={() => remove(connection)}>{tt("common.delete")}</AppButton></div></div><div class="connection-key"><label>{tt("settings.apiKeyEdit")}<input data-testid={`connection-${connection.id}-api-key`} type="password" bind:value={connectionKeys[connection.id]} placeholder={tt("settings.apiKeyPlaceholder")} autocomplete="new-password" /></label><AppButton variant="secondary" loading={busy === `key-${connection.id}`} onclick={() => saveConnectionKey(connection)}>{tt("settings.saveKey")}</AppButton><small>{tt("settings.keySecurityHint")}</small></div><div class="model-grid">{#each models[connection.id] ?? [] as model (model.id)}<div class:muse-model={model.model_id === "muse-spark-1.2-contributor"} class="model-row"><div><strong>{model.model_id === "muse-spark-1.2-contributor" ? "Muse Spark 1.2 Contributor" : model.display_name}</strong>{#if model.model_id === "muse-spark-1.2-contributor"}<span class="muse-badge">{tt("settings.museSharedKey")}</span>{/if}<div class="muted">{model.model_id} · {protocolLabel(model)}</div>{#if model.model_id === "muse-spark-1.2-contributor"}<div class="muse-privacy">{tt("settings.musePrivacy")}</div>{/if}</div><div class="model-status"><span class:available={model.available} class="tag">{model.available ? tt("settings.available") : tt("settings.unavailable")}</span><span class:disabled={!model.enabled} class="tag">{model.enabled ? tt("settings.enabled") : tt("settings.disabled")}</span><AppButton testid={`model-toggle-${model.id}`} variant="secondary" loading={busy === `toggle-${model.id}`} onclick={() => toggleModel(model)}>{model.enabled ? tt("settings.disableModel") : tt("settings.enableModel")}</AppButton><AppButton variant="ghost" loading={busy === `test-${model.id}`} onclick={() => test(model)} disabled={!model.enabled}>{tt("settings.test")}</AppButton></div></div>{/each}</div></AppCard>{/each}</div>
</section>

<style>
  .provider-area { display:grid; gap:14px; } .section-head,.connection-head { display:flex; justify-content:space-between; align-items:flex-start; gap:12px; } h2 { margin:0 0 4px; font-size:17px; } p { margin:0; color:var(--color-muted); font-size:12px; line-height:1.5; } .template-choices { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:12px; } .template-choices :global(.app-card){position:relative;overflow:hidden;padding:16px;background:linear-gradient(120deg,#fafcfc,#eff3f4);border-color:#d7e2e5}.template-choices :global(.app-card):after{content:"FIXED TEMPLATE";position:absolute;right:12px;top:12px;color:#80939b;font-size:9px;letter-spacing:.08em}.template-title { font:500 17px var(--font-serif); } .template-choices p { min-height:54px; margin:6px 0 12px; padding-right:72px; } .key-field{margin-bottom:10px}.muse-template-note{min-height:30px;margin:-3px 0 9px;color:#8a7358;font-size:10px;line-height:1.4}.form-grid { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:10px; } label { display:grid; gap:6px; color:var(--color-muted); font-size:12px; } input,select { min-width:0; min-height:38px; border:1px solid var(--color-border); border-radius:var(--radius-sm); padding:9px 10px; color:var(--color-text); background:var(--color-surface); font-size:13px; } .form-actions { display:flex; gap:8px; margin-top:12px; } .connection-list{display:grid;gap:10px}.connection-actions { display:flex; align-items:center; gap:7px; flex-wrap:wrap; justify-content:flex-end; } .connection-key{display:grid;grid-template-columns:minmax(220px,1fr) auto minmax(180px,.8fr);align-items:end;gap:9px;margin-top:13px;padding:11px;border:1px solid var(--color-border);border-radius:11px;background:var(--color-surface-muted)}.connection-key small{align-self:center;color:var(--color-muted);font-size:10px;line-height:1.4}.muted { color:var(--color-muted); font-size:11px; } .tag { display:inline-flex; margin-left:6px; padding:3px 8px; border-radius:999px; background:var(--color-surface-muted); color:var(--color-muted); font-size:10px; } .key-state { color:var(--color-warning); font-size:11px; } .key-state.key-ok { color:var(--color-success); } .model-grid { display:grid; gap:7px; margin-top:13px; } .model-row { display:flex; justify-content:space-between; align-items:center; gap:10px; border-top:1px solid var(--color-border); padding-top:9px; }.model-row.muse-model{margin:4px -6px 0;padding:12px 8px;border:1px solid #cfdde1;border-radius:10px;background:linear-gradient(110deg,#f4f8f9,#eaf1f3)}.muse-badge{display:inline-flex;margin-left:8px;padding:2px 7px;border-radius:999px;background:#dce8ec;color:#55727e;font-size:10px}.muse-privacy{max-width:680px;margin-top:5px;color:#8a7358;font-size:10px;line-height:1.4}.model-status { display:flex; align-items:center; gap:5px; flex-wrap:wrap; justify-content:flex-end; } .available { color:var(--color-success); } .disabled { color:var(--color-warning); } .error { padding:9px 11px; border:1px solid #ead2d2; background:var(--color-danger-soft); color:var(--color-danger); border-radius:var(--radius-sm); font-size:12px; } @media(max-width:1100px){.template-choices{grid-template-columns:1fr 1fr}}@media(max-width:900px){.connection-key{grid-template-columns:1fr auto}.connection-key small{grid-column:1/-1}} @media(max-width:720px){ .template-choices,.form-grid,.connection-key { grid-template-columns:1fr; } .section-head,.connection-head { flex-direction:column; } .connection-actions { justify-content:flex-start; } }
</style>
