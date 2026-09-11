<script lang="ts">
  import type { Snippet } from "svelte";

  type Variant = "primary" | "secondary" | "ghost" | "danger";
  let {
    variant = "primary",
    loading = false,
    disabled = false,
    type = "button",
    label,
    testid,
    children,
    onclick
  }: {
    variant?: Variant;
    loading?: boolean;
    disabled?: boolean;
    type?: "button" | "submit" | "reset";
    label?: string;
    children?: Snippet;
    onclick?: (event: MouseEvent) => void;
    testid?: string;
  } = $props();
</script>

<button data-testid={testid} class={"app-button " + variant} {type} disabled={disabled || loading} aria-busy={loading} {onclick}>
  {#if loading}<span class="spinner" aria-hidden="true"></span>{/if}
  <span class="button-label" class:pending={loading}>{#if children}{@render children()}{:else}{label ?? ""}{/if}</span>
</button>

<style>
  .app-button { position:relative; min-height:40px; display:inline-flex; align-items:center; justify-content:center; border:1px solid transparent; border-radius:var(--radius-sm); padding:8px 14px; font-size:14px; line-height:1.5; font-weight:550; cursor:pointer; transition:background .16s ease,border-color .16s ease,color .16s ease; }
  .button-label.pending { opacity:0; }
  .app-button:disabled { cursor: not-allowed; opacity: .62; }
  .primary { background: var(--color-primary); color: #fff; }
  .primary { box-shadow: 0 4px 12px rgb(77 103 115 / .12); }
  .primary:hover:not(:disabled) { background: var(--color-primary-hover); }
  .secondary { background: var(--color-surface); color: var(--color-text); border-color: var(--color-border); }
  .secondary:hover:not(:disabled) { background: var(--color-surface-muted); }
  .ghost { background: transparent; color: var(--color-muted); }
  .ghost:hover:not(:disabled) { background: var(--color-surface-muted); color: var(--color-text); }
  .danger { background: var(--color-danger); color: #fff; }
  .danger:hover:not(:disabled) { background: #8d5f5f; }
  .spinner { position:absolute; width:14px; height:14px; border:2px solid currentColor; border-right-color:transparent; border-radius:50%; animation:spin .7s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
