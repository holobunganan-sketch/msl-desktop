<script lang="ts">
  import Modal from "./Modal.svelte";
  import AppButton from "./AppButton.svelte";
  import { t, locale } from "$lib/i18n";
  let { open = $bindable(false), title = "", message = "", onconfirm = () => {}, oncancel = () => {} }: { open?: boolean; title?: string; message?: string; onconfirm?: () => void; oncancel?: () => void } = $props();
  function cancel() { open = false; oncancel(); }
  function confirm() { open = false; onconfirm(); }
</script>

<Modal bind:open title={title} onclose={cancel}>
  <p class="message">{message}</p>
  <div class="actions">
    <AppButton variant="ghost" label={t("common.cancel", {}, $locale)} onclick={cancel} />
    <AppButton variant="danger" label={t("common.delete", {}, $locale)} onclick={confirm} />
  </div>
</Modal>

<style>
  .message { margin: 0; color: var(--color-text); line-height: 1.6; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 22px; }
</style>
