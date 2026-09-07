<script lang="ts">
  import { translateStatus, locale, type Locale } from "$lib/i18n";
  let { status = "", tone = "" }: { status?: string; tone?: string } = $props();
  let currentLocale = $derived($locale as Locale);
</script>

<span class={"status-badge " + (tone || status)} role="status">{translateStatus(status, currentLocale)}</span>

<style>
  .status-badge { display: inline-flex; align-items: center; min-height: 23px; border: 1px solid transparent; border-radius: 999px; padding: 3px 8px; background: var(--color-surface-muted); color: var(--color-muted); font-size: 10px; font-weight: 650; }
  .active, .next, .scheduled { background: var(--color-success-soft); color: var(--color-success); border-color: #d8e5e0; }
  .waiting, .paused { background: var(--color-warning-soft); color: var(--color-warning); border-color: #e9dfd2; }
  .done, .resolved { background: #e8edef; color: #5f7179; border-color: #dce4e7; }
  .archived { background: #f0f3f4; color: #7e8c92; border-color: #e1e7e9; }
</style>
