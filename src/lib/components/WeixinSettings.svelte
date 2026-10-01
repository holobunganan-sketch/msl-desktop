<script lang="ts">
  import { onMount } from "svelte";
  import { command, normalizeError } from "$lib/services/api";
  import { locale } from "$lib/i18n";
  import AppButton from "./ui/AppButton.svelte";
  import AppCard from "./ui/AppCard.svelte";

  type Status = { bound: boolean; enabled: boolean; connection_state: string; last_received_at: number | null };
  type Login = { state: string; qr_image: string | null };
  let status = $state<Status>({ bound: false, enabled: false, connection_state: "unbound", last_received_at: null });
  let loading = $state(true);
  let busy = $state(false);
  let error = $state("");
  let qrImage = $state("");
  let loginState = $state("");
  let verifyCode = $state("");
  let disposed = false;
  let loginGeneration = 0;
  const txt = (zh: string, en: string) => $locale === "zh-CN" ? zh : en;
  const labels: Record<string, [string, string]> = {
    unbound: ["未绑定", "Not paired"], disabled: ["已暂停", "Paused"], connecting: ["正在连接", "Connecting"],
    connected: ["已连接，仅接收本人私聊", "Connected · owner-only direct messages"], reconnecting: ["连接中断，正在重试", "Reconnecting"],
    needs_login: ["登录已失效，请重新扫码", "Session expired · scan again"], wait: ["请用本人微信扫码，并在手机上确认", "Scan with your own WeChat account and confirm on your phone"],
    scaned: ["已扫码，请完成手机上的确认", "Scanned · complete confirmation on your phone"], need_verifycode: ["请输入手机上显示的验证码", "Enter the verification code shown on your phone"],
    expired: ["二维码已过期，请重新获取", "QR code expired · request another"], verify_code_blocked: ["验证次数过多，请重新获取二维码", "Too many verification attempts · request another QR code"],
    binded_redirect: ["微信提示已绑定；可尝试启用现有连接，或重新扫码", "WeChat reports an existing pairing; enable it or scan again"],
  };
  const stateLabel = (value: string) => { const pair = labels[value]; return pair ? txt(...pair) : txt("等待连接", "Waiting for connection"); };
  async function load() {
    try { const value = await command<Status>("get_weixin_status"); if (!disposed) status = value; }
    catch (value) { if (!disposed) error = normalizeError(value); }
    finally { if (!disposed) loading = false; }
  }
  onMount(() => {
    void load();
    const timer = setInterval(() => { if (!busy) void load(); }, 5000);
    return () => { disposed = true; loginGeneration += 1; clearInterval(timer); };
  });
  async function begin() {
    busy = true; error = ""; qrImage = ""; verifyCode = "";
    const generation = ++loginGeneration;
    try {
      const value = await command<Login>("begin_weixin_login");
      if (disposed || generation !== loginGeneration) return;
      if (!value.qr_image?.startsWith("data:image/svg+xml;base64,")) throw new Error(txt("二维码显示失败，请重试", "Unable to display QR code; try again"));
      qrImage = value.qr_image; loginState = value.state;
      void pollLogin(generation);
    } catch (value) { if (!disposed) error = normalizeError(value); }
    finally { if (!disposed) { busy = false; void load(); } }
  }
  async function pollLogin(generation: number) {
    while (!disposed && generation === loginGeneration) {
      try {
        const value = await command<Login>("poll_weixin_login");
        if (disposed || generation !== loginGeneration) return;
        loginState = value.state;
        if (value.state === "confirmed") { qrImage = ""; loginState = ""; await load(); return; }
        if (["expired", "verify_code_blocked", "binded_redirect"].includes(value.state)) { qrImage = ""; return; }
      } catch (value) { if (!disposed && generation === loginGeneration) error = normalizeError(value); return; }
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
  }
  async function submitCode() {
    busy = true; error = "";
    try { await command("submit_weixin_verifycode", { code: verifyCode.trim() }); verifyCode = ""; }
    catch (value) { error = normalizeError(value); }
    finally { busy = false; }
  }
  async function toggle() {
    busy = true; error = ""; loginGeneration += 1; qrImage = ""; loginState = "";
    try { status = await command<Status>("set_weixin_enabled", { enabled: !status.enabled }); }
    catch (value) { error = normalizeError(value); }
    finally { busy = false; }
  }
  async function unlink() {
    busy = true; error = ""; loginGeneration += 1; qrImage = ""; loginState = ""; verifyCode = "";
    try { status = await command<Status>("unlink_weixin"); }
    catch (value) { error = normalizeError(value); }
    finally { busy = false; }
  }
</script>

<AppCard title={txt("微信入口", "WeChat input")} subtitle={txt("在手机上随手记录，在桌面端继续处理。", "Capture on your phone and continue on the desktop.")}>
  <div class="weixin-settings" data-testid="weixin-settings">
    <p class="status" role="status">{loading ? txt("正在读取连接状态…", "Loading connection status…") : stateLabel(status.connection_state)}</p>
    <ul>
      <li>{txt("普通文字会原话存入收件箱。", "Ordinary text is saved verbatim to your inbox.")}</li>
      <li>{txt("发送", "Send")} <code>召唤秘书</code>{txt("，收到菜单后回复数字或中文序号。", " to open the menu, then reply with a number.")}</li>
    </ul>
    <ol class="menu-preview" aria-label={txt("微信操作菜单", "WeChat action menu")}>
      {#each [
        ["一", "记一句话", "Record a note"], ["二", "全局整理", "Organize the workspace"],
        ["三", "生成每日简报", "Prepare today's brief"], ["四", "查看整理进度", "Check progress"],
        ["五", "查看待确认数量", "Count pending suggestions"], ["六", "退出菜单", "Exit the menu"],
      ] as item, index}
        <li><span class="menu-number" aria-hidden="true">{txt(item[0], String(index + 1))}</span><span>{txt(item[1], item[2])}</span></li>
      {/each}
    </ol>
    <p class="hint">{txt("菜单有效期 10 分钟。选“一”后，下一条文字按原话记录；选“六”退出。整理和简报在电脑后台进行，建议仍由你在桌面端确认。", "The menu stays active for 10 minutes. Choose 1 to save your next message verbatim, or 6 to exit. Analysis and briefs run on your computer; you confirm suggestions on the desktop.")}</p>
    <p class="hint">{txt("仅接收扫码账号本人的私聊文字；群聊、语音、图片和附件不处理。微信回复操作回执、任务进度或待确认数量，不发送工作正文。", "Only direct text messages from the paired owner are accepted. Groups, voice, images and attachments are ignored. Replies contain acknowledgments, job status or pending counts, never work content.")}</p>
    <div class="actions">
      <AppButton variant="secondary" loading={busy} disabled={loading} onclick={begin}>{txt(status.bound ? "重新扫码绑定" : "扫码绑定并启用", status.bound ? "Pair again" : "Scan to pair and enable")}</AppButton>
      {#if status.bound}
        <AppButton variant="secondary" disabled={busy || loading} onclick={toggle}>{status.enabled ? txt("暂停接收", "Pause") : txt("启用接收", "Enable")}</AppButton>
        <AppButton variant="ghost" disabled={busy || loading} onclick={unlink}>{txt("解除绑定", "Unlink")}</AppButton>
      {/if}
    </div>
    {#if qrImage}
      <div class="qr"><img src={qrImage} width="236" height="236" alt={txt("请用本人微信扫描此绑定二维码", "Scan this pairing QR code with your own WeChat account")} /></div>
    {/if}
    {#if loginState}<p role="status">{stateLabel(loginState)}</p>{/if}
    {#if loginState === "need_verifycode"}
      <form class="verify" onsubmit={(event) => { event.preventDefault(); void submitCode(); }}>
        <label for="weixin-verification">{txt("手机验证码", "Phone verification code")}</label>
        <input id="weixin-verification" inputmode="numeric" autocomplete="one-time-code" pattern={"[0-9]{4,12}"} maxlength="12" bind:value={verifyCode} />
        <AppButton type="submit" loading={busy} disabled={!/^[0-9]{4,12}$/.test(verifyCode)}>{txt("提交验证码", "Submit code")}</AppButton>
      </form>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <p class="hint">{txt("首次使用需由你扫码确认。建议只在一台常开电脑启用接收，并保持工作台运行、网络连接正常；账号能否接入以微信实际返回为准。绑定凭据仅保存在本机安全凭据库，不随工作数据同步或备份。", "You must complete the first scan yourself. Enable receiving on one always-on computer and keep the workbench running and online. Account availability depends on WeChat. Pairing credentials stay in this computer’s secure credential store and are excluded from sync and backups.")}</p>
  </div>
</AppCard>

<style>
  .weixin-settings { display: grid; gap: 12px; }
  p, ul { margin: 0; font-size: 13px; line-height: 1.65; }
  ul { padding-left: 20px; }
  .menu-preview { list-style: none; display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px 20px; margin: 0; padding: 12px; border: 1px solid var(--color-border); border-radius: 8px; }
  .menu-preview li { display: flex; align-items: baseline; gap: 10px; font-size: 14px; line-height: 1.6; overflow-wrap: anywhere; }
  .menu-number { color: var(--color-muted); flex: 0 0 1.4em; }
  @media (max-width: 600px) { .menu-preview { grid-template-columns: minmax(0, 1fr); } }
  .status { font-weight: 600; }
  .hint { color: var(--color-muted); }
  .actions, .verify { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; }
  .qr { padding: 10px; background: white; border-radius: 8px; width: fit-content; }
  .qr img { display: block; max-width: 100%; height: auto; }
  code { background: var(--color-surface-muted); padding: 2px 5px; border-radius: 4px; font-size: 13px; }
  input { max-width: 160px; min-height: 38px; padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-surface); color: var(--color-text); }
  .error { color: var(--color-danger); }
</style>
