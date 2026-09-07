<script lang="ts">
  import type { Snippet } from "svelte";
  let { id, label, required = false, hint = "", error = "", children }: { id: string; label: string; required?: boolean; hint?: string; error?: string; children?: Snippet } = $props();
</script>

<div class="field">
  <label for={id}>{label}{#if required}<span aria-hidden="true"> *</span>{/if}</label>
  {#if children}{@render children()}{/if}
  {#if hint && !error}<div class="hint" id={id + "-hint"}>{hint}</div>{/if}
  {#if error}<div class="error" id={id + "-error"} role="alert">{error}</div>{/if}
</div>

<style>
  .field { display: grid; gap: 6px; }
  label { color: var(--color-text); font-size: 12px; font-weight: 600; }
  .hint, .error { font-size: 12px; }
  .hint { color: var(--color-muted); }
  .error { color: var(--color-danger); }
</style>
