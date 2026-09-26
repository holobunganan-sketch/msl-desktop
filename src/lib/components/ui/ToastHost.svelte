<script lang="ts">
  import { toasts, dismissToast } from "$lib/stores/toast";
  import { t, locale } from "$lib/i18n";
  import Modal from './Modal.svelte';
  let details=$state(''),open=$state(false);
</script>

<div class="toast-host" aria-live="polite" aria-atomic="true">
  {#each $toasts as toast (toast.id)}
    <div class={"toast " + toast.kind} role={toast.kind === "error" ? "alert" : "status"}>
      <button class="toast-message" title={toast.message} onclick={()=>{details=toast.message;open=true;}}>{toast.message}</button>
      <button class="toast-detail" onclick={()=>{details=toast.message;open=true;}}>{$locale==='zh-CN'?'详情':'Details'}</button>
      {#if toast.dismissible}
        <button type="button" onclick={() => dismissToast(toast.id)} aria-label={t("common.cancel", {}, $locale)}>×</button>
      {/if}
    </div>
  {/each}
</div>
<Modal bind:open title={$locale==='zh-CN'?'通知详情':'Notification details'}><p style="white-space:pre-wrap;overflow-wrap:anywhere;line-height:1.7">{details}</p></Modal>

<style>
  .toast-host { position:fixed;z-index:90;top:82px;right:24px;width:min(540px,calc(100vw - 48px));display:grid;gap:8px;pointer-events:none; }
  .toast { pointer-events:auto;min-height:46px;display:flex;align-items:center;gap:8px;padding:8px 12px;font-size:14px;color:var(--color-text);min-width:0;border:1px solid var(--color-border);border-left:4px solid var(--color-primary);border-radius:8px;background:var(--color-surface);box-shadow:var(--shadow-md); }
  .toast .toast-message{flex:1;min-width:0;text-align:left;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;font:inherit}
  .toast .toast-detail{flex:0 0 auto;font:inherit}
  .toast.success { border-left-color: var(--color-success); }
  .toast.error { border-left-color: var(--color-danger); }
  .toast button { flex: 0 0 28px; min-height: 28px; border: 0; background: transparent; color: var(--color-muted); cursor: pointer; font-size: 20px; line-height: 1; }
</style>
