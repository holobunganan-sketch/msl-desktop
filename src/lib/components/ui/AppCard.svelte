<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, subtitle, children }: { title?: string; subtitle?: string; children?: Snippet } = $props();
</script>

<section class="app-card">
  {#if title}<div class="card-heading"><h2>{title}</h2>{#if subtitle}<p>{subtitle}</p>{/if}</div>{/if}
  {#if children}{@render children()}{/if}
</section>

<style>
  .app-card { background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-md); box-shadow: var(--shadow-sm); padding: 18px; }
  .card-heading { margin-bottom: 14px; }
  h2, p { margin: 0; }
  h2 { font-size: 14px; font-weight: 650; letter-spacing: .01em; }
  p { color: var(--color-muted); font-size: 12px; margin-top: 4px; }
</style>
