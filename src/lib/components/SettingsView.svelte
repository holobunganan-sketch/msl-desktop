<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type Provider = {
    id: number;
    display_name: string;
    provider_type: string;
    base_url: string;
    model: string;
    enabled: boolean;
    created_at: number;
    updated_at: number;
  };

  let providers = $state<Provider[]>([]);
  let keyStatus = $state<Record<number, boolean>>({});
  let error = $state("");
  let info = $state("");

  // 表单
  let editingId = $state<number | null>(null);
  let showForm = $state(false);
  let fName = $state("");
  let fType = $state("openai_compatible");
  let fUrl = $state("");
  let fModel = $state("");
  let fEnabled = $state(true);
  let fKey = $state("");
  let testing = $state(false);

  // 通知/自启动
  let notifEnabled = $state(true);
  let leadMinutes = $state(60);
  let autostartOn = $state(false);

  const DEEPSEEK_PRESET = {
    name: "DeepSeek",
    type: "openai_compatible",
    url: "https://api.deepseek.com",
    model: "deepseek-v4-flash",
  };

  async function load() {
    try {
      providers = await invoke("list_providers");
      const st: Record<number, boolean> = {};
      for (const p of providers) {
        try {
          st[p.id] = await invoke("provider_has_key", { id: p.id });
        } catch {
          st[p.id] = false;
        }
      }
      keyStatus = st;
    } catch (e) {
      error = String(e);
    }
  }

  function openNew() {
    editingId = null;
    showForm = true;
    fName = "";
    fType = "openai_compatible";
    fUrl = "";
    fModel = "";
    fEnabled = true;
    fKey = "";
  }

  function openEdit(p: Provider) {
    editingId = p.id;
    showForm = true;
    fName = p.display_name;
    fType = p.provider_type;
    fUrl = p.base_url;
    fModel = p.model;
    fEnabled = p.enabled;
    fKey = ""; // 不显示已存 key；留空表示不修改
  }

  function applyPreset() {
    fName = DEEPSEEK_PRESET.name;
    fType = DEEPSEEK_PRESET.type;
    fUrl = DEEPSEEK_PRESET.url;
    fModel = DEEPSEEK_PRESET.model;
  }

  async function save() {
    try {
      await invoke("save_provider", {
        id: editingId,
        displayName: fName.trim(),
        providerType: fType,
        baseUrl: fUrl.trim(),
        model: fModel.trim(),
        enabled: fEnabled,
        apiKey: fKey || (editingId !== null ? "" : null),
      });
      showForm = false;
      info = "已保存";
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(p: Provider) {
    if (!confirm(`删除 Provider「${p.display_name}」？`)) return;
    try {
      await invoke("delete_provider", { id: p.id });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function testConn(p: Provider) {
    testing = true;
    error = "";
    info = "";
    try {
      const msg: string = await invoke("test_provider_connection", { id: p.id });
      info = msg;
    } catch (e) {
      error = String(e);
    }
    testing = false;
  }

  async function loadNotifications() {
    try {
      const v = await invoke("app_settings_get", { key: "notifications_enabled" });
      if (v !== null) notifEnabled = v !== "false";
      const l = await invoke("app_settings_get", { key: "reminder_lead_minutes" });
      if (l !== null) leadMinutes = Number(l);
      autostartOn = await invoke("autostart_status");
    } catch {
      /* 默认值 */
    }
  }

  async function toggleNotif() {
    try {
      notifEnabled = await invoke("set_notifications_enabled", { enabled: notifEnabled });
    } catch (e) {
      error = String(e);
    }
  }

  async function changeLead() {
    try {
      leadMinutes = await invoke("set_reminder_lead_minutes", { minutes: leadMinutes });
    } catch (e) {
      error = String(e);
    }
  }

  async function toggleAutostart() {
    try {
      autostartOn = await invoke("set_autostart", { enabled: !autostartOn });
      info = autostartOn ? "已启用 Windows 自启动（后台模式启动，不弹主窗口）" : "已关闭自启动";
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    load();
    loadNotifications();
  });
</script>

<div class="settings">
  <h1>Settings</h1>

  {#if error}<div class="status error">{error}</div>{/if}
  {#if info}<div class="status ok">{info}</div>{/if}

  <section class="card">
    <div class="card-title">AI Providers</div>
    <div class="muted hint">
      API Key 保存在 Windows 凭据管理器（Credential Manager），不落数据库。
      DeepSeek preset：OpenAI-compatible，base URL
      <code>https://api.deepseek.com</code>。
    </div>

    {#each providers as p (p.id)}
      <div class="row-item">
        <div class="p-main">
          <div class="p-name">{p.display_name} {p.enabled ? "✅" : "⏸"}</div>
          <div class="muted">{p.provider_type} · {p.base_url} · {p.model}</div>
        </div>
        <div class="p-key">{keyStatus[p.id] ? "已配置 Key" : "未配置 Key"}</div>
        <div class="actions">
          <button onclick={() => testConn(p)} disabled={testing}>
            {testing ? "测试中…" : "测试连接"}
          </button>
          <button onclick={() => openEdit(p)}>编辑</button>
          <button onclick={() => remove(p)}>删除</button>
        </div>
      </div>
    {/each}

    {#if providers.length === 0}
      <div class="muted empty">尚未配置 AI Provider（AI 默认关闭，不影响其他功能）</div>
    {/if}

    {#if showForm}
      <div class="form">
        <div class="form-row">
          <button class="preset" onclick={applyPreset}>使用 DeepSeek preset</button>
        </div>
        <div class="form-row">
          <input bind:value={fName} placeholder="显示名称" />
          <select bind:value={fType}>
            <option value="openai_compatible">openai_compatible</option>
            <option value="deepseek">deepseek</option>
          </select>
        </div>
        <div class="form-row">
          <input bind:value={fUrl} placeholder="Base URL（如 https://api.deepseek.com）" />
          <input bind:value={fModel} placeholder="模型（如 deepseek-v4-flash）" />
        </div>
        <div class="form-row">
          <input type="password" bind:value={fKey} placeholder={editingId ? "新 API Key（留空不修改）" : "API Key"} />
          <label class="chk">
            <input type="checkbox" bind:checked={fEnabled} /> 启用
          </label>
        </div>
        <div class="form-row">
          <button onclick={save}>{editingId ? "保存" : "添加"}</button>
          <button onclick={() => (showForm = false)}>取消</button>
        </div>
      </div>
    {:else}
      <button class="add" onclick={openNew}>新增 Provider…</button>
    {/if}
  </section>

  <!-- Notifications -->
  <section class="card">
    <div class="card-title">Notifications</div>
    <div class="row-item">
      <span class="p-main">提醒通知（waiting 跟进 / 任务截止 / 日历日程）</span>
      <label class="chk">
        <input type="checkbox" bind:checked={notifEnabled} onchange={toggleNotif} /> 开启
      </label>
    </div>
    <div class="row-item">
      <span class="p-main">提前提醒</span>
      <input
        type="number"
        min="1"
        max="1440"
        bind:value={leadMinutes}
        onchange={changeLead}
        style="width: 80px; padding: 5px 8px; border: 1px solid #c8ccd1; border-radius: 6px; font-size: 12px;"
      />
      <span class="muted">分钟</span>
    </div>
  </section>

  <!-- Autostart -->
  <section class="card">
    <div class="card-title">Windows 启动</div>
    <div class="row-item">
      <span class="p-main">
        开机自启动（后台模式启动，不弹主窗口，仅托盘常驻；
        点击托盘图标再打开窗口）
      </span>
      <label class="chk">
        <input type="checkbox" bind:checked={autostartOn} onchange={toggleAutostart} /> 启用
      </label>
    </div>
  </section>

  <!-- 快捷键 -->
  <section class="card">
    <div class="card-title">快捷键</div>
    <div class="muted hint">
      <div>Ctrl+K — 全局搜索</div>
      <div>Ctrl+Shift+Space — Quick Capture（快速记入 Inbox）</div>
    </div>
  </section>
</div>

<style>
  h1 {
    font-size: 18px;
    margin: 0 0 12px;
  }
  .card {
    border: 1px solid #e4e7eb;
    border-radius: 8px;
    padding: 12px 14px;
    max-width: 720px;
  }
  .card-title {
    font-weight: 600;
    font-size: 12px;
    letter-spacing: 0.05em;
    color: #4a5568;
    margin-bottom: 8px;
  }
  .hint {
    margin-bottom: 12px;
    line-height: 1.5;
  }
  .hint code {
    background: #f1f3f5;
    padding: 1px 5px;
    border-radius: 4px;
  }
  .row-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    border-bottom: 1px solid #f0f2f4;
    font-size: 13px;
  }
  .p-main {
    flex: 1;
    min-width: 0;
  }
  .p-name {
    font-weight: 600;
  }
  .p-key {
    font-size: 12px;
    color: #2f6db3;
    white-space: nowrap;
  }
  .actions button {
    margin-left: 4px;
  }
  button {
    padding: 5px 10px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    background: #fff;
    font-size: 12px;
    cursor: pointer;
  }
  button:disabled {
    color: #9aa0a6;
    cursor: default;
  }
  .add {
    margin-top: 10px;
  }
  .form {
    margin-top: 12px;
    border-top: 1px solid #e4e7eb;
    padding-top: 10px;
  }
  .form-row {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
    flex-wrap: wrap;
  }
  .form-row input:not([type]),
  .form-row input[type="password"] {
    flex: 1;
    min-width: 160px;
    padding: 6px 8px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    font-size: 12px;
  }
  .chk {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    white-space: nowrap;
  }
  .preset {
    background: #dbe9f7;
    font-weight: 600;
  }
  .muted {
    color: #6b7280;
    font-size: 12px;
  }
  .empty {
    padding: 8px 0;
  }
  .status.error {
    color: #b3261e;
    font-size: 13px;
    margin: 6px 0;
  }
  .status.ok {
    color: #2e7d32;
    font-size: 13px;
    margin: 6px 0;
  }
</style>
