<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, description = "", actionLabel = "", children }: { title: string; description?: string; actionLabel?: string; children?: Snippet } = $props();
</script>

<div class="empty-state">
  <div class="icon" aria-hidden="true">✦</div>
  <h3>{title}</h3>
  {#if description}<p>{description}</p>{/if}
  {#if children}{@render children()}{:else if actionLabel}<button type="button">{actionLabel}</button>{/if}
</div>

<style>
  .empty-state { display: grid; justify-items: center; gap: 8px; padding: 32px 18px; text-align: center; color: var(--color-muted); }
  .icon { display: grid; place-items: center; width: 38px; height: 38px; border: 1px solid #d4e0e4; border-radius: 13px; background: linear-gradient(145deg, #fbfcfc, var(--color-primary-soft)); color: var(--color-primary); }
  h3, p { margin: 0; }
  h3 { color: var(--color-text); font: 500 15px var(--font-serif); }
  p { max-width: 420px; font-size: 12px; }
  button { border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-surface); padding: 7px 12px; cursor: pointer; }
</style>
