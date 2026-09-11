<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, description = "", actionLabel = "", children, compact = false, framed = false }: { title: string; description?: string; actionLabel?: string; children?: Snippet; compact?: boolean; framed?: boolean } = $props();
</script>

<div class="empty-state" class:compact class:framed>
  {#if !compact}<div class="icon" aria-hidden="true">✦</div>{/if}
  <h3>{title}</h3>
  {#if description}<p>{description}</p>{/if}
  {#if children}{@render children()}{:else if actionLabel}<button type="button">{actionLabel}</button>{/if}
</div>

<style>
  .empty-state { min-width: 0; display: grid; justify-items: center; align-content: center; gap: 8px; padding: 32px 20px; text-align: center; color: var(--color-muted); line-height: 1.7; overflow-wrap: anywhere; }
  .compact { padding: 28px 20px; min-height: 96px; }
  .framed { border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-surface); }
  .icon { display: grid; place-items: center; width: 38px; height: 38px; border: 1px solid #d4e0e4; border-radius: 13px; background: linear-gradient(145deg, #fbfcfc, var(--color-primary-soft)); color: var(--color-primary); }
  h3, p { margin: 0; }
  h3 { max-width: 100%; color: var(--color-text); font: 500 15px/1.7 "Segoe UI", "Microsoft YaHei", sans-serif; }
  .compact h3 { color: var(--color-muted); font-weight: 400; }
  p { max-width: min(100%, 420px); font-size: 14px; }
  button { border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-surface); padding: 7px 12px; cursor: pointer; }
</style>
