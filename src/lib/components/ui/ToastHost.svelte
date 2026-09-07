<script lang="ts">
  import { toasts, dismissToast } from "$lib/stores/toast";
  import { t, locale } from "$lib/i18n";
</script>

<div class="toast-host" aria-live="polite" aria-atomic="true">
  {#each $toasts as toast (toast.id)}
    <div class={"toast " + toast.kind} role={toast.kind === "error" ? "alert" : "status"}>
      <div class="toast-message" role="region" aria-label={toast.kind === "error" ? ($locale === "zh-CN" ? "错误详情" : "Error details") : ($locale === "zh-CN" ? "通知" : "Notification")}>{toast.message}</div>
      {#if toast.dismissible}
        <button type="button" onclick={() => dismissToast(toast.id)} aria-label={t("common.cancel", {}, $locale)}>×</button>
      {/if}
    </div>
  {/each}
</div>

<style>
  .toast-host { flex: 0 0 auto; min-width: 0; }
  .toast { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; border-bottom: 1px solid var(--color-border); border-left: 4px solid var(--color-primary); background: var(--color-surface); padding: 10px 22px; font-size: 13px; line-height: 1.5; }
  .toast-message { min-width: 0; max-height: min(96px, calc(var(--viewport-height) * .2)); overflow: auto; overflow-wrap: anywhere; padding-right: 8px; }
  .toast.success { border-left-color: var(--color-success); }
  .toast.error { border-left-color: var(--color-danger); }
  .toast button { flex: 0 0 28px; min-height: 28px; border: 0; background: transparent; color: var(--color-muted); cursor: pointer; font-size: 20px; line-height: 1; }
</style>
