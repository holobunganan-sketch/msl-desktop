<script lang="ts">
  import type { Snippet } from "svelte";
  import {tick} from "svelte";
  import { t, locale } from "$lib/i18n";

  let {
    open = $bindable(false),
    title = "",
    children,
    footer,
    dismissible = true,
    onclose = () => {}
  }: { open?: boolean; title?: string; children?: Snippet; footer?:Snippet; dismissible?:boolean; onclose?: () => void } = $props();
  let firstFocus = $state<HTMLButtonElement | undefined>(undefined);
  const titleId = $props.id();
  function keydown(event:KeyboardEvent) {
    if(event.key==='Escape'){event.stopPropagation();close();}
    if(event.key!=='Tab')return;
    const nodes=[...(event.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),a[href],[tabindex="0"]')].filter(e=>e.getClientRects().length>0);
    const first=nodes[0],last=nodes.at(-1);
    if(!first){event.preventDefault();return;}
    if(event.shiftKey&&document.activeElement===first){event.preventDefault();last?.focus();}
    else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}
  }

  function close() {
    if(!dismissible)return;
    open = false;
    onclose();
  }

  $effect(() => {
    if (!open)return;
    const previous=document.activeElement as HTMLElement|null;
    let active=true;
    void tick().then(()=>{if(active)firstFocus?.focus();});
    return()=>{active=false;void tick().then(()=>{if(previous?.isConnected)previous.focus();});};
  });
</script>

{#if open}
  <div class="modal-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && close()}>
    <div class="modal" role="dialog" aria-modal="true" aria-labelledby={titleId} tabindex="-1" onkeydown={keydown}>
      <header class="modal-header">
        <h2 id={titleId}>{title}</h2>
        <button bind:this={firstFocus} class="close" type="button" onclick={close} aria-label={t("common.cancel", {}, $locale)} title={t("common.cancel", {}, $locale)}>×</button>
      </header>
      <div class="modal-body">{#if children}{@render children()}{/if}</div>
      {#if footer}<footer class="modal-footer">{@render footer()}</footer>{/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop { position: fixed; inset: 0; z-index: 1200; display: grid; place-items: center; padding: 16px; background: rgb(38 54 64 / .28); backdrop-filter: blur(3px); }
  .modal { width: min(560px, 100%); max-height: min(720px, calc(var(--viewport-height) - 32px)); min-height: 0; display: flex; flex-direction: column; overflow: hidden; background: var(--color-surface); border: 1px solid var(--color-border-strong); border-radius: var(--radius-lg); box-shadow: var(--shadow-md); }
  .modal-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 18px 20px; border-bottom: 1px solid var(--color-border); }
  .modal-header { flex-shrink: 0; }
  h2 { min-width: 0; overflow-wrap: anywhere; margin: 0; font: 500 18px var(--font-serif); }
  .close { flex: 0 0 32px; width: 32px; height: 32px; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--color-muted); font-size: 22px; cursor: pointer; }
  .close:hover { background: var(--color-surface-muted); color: var(--color-text); }
  .modal-body { min-height: 0; overflow: auto; padding: 20px; overscroll-behavior: contain; }
  .modal-body{scrollbar-gutter:stable}
  .modal-footer{flex-shrink:0;padding:14px 20px;border-top:1px solid var(--color-border);display:flex;justify-content:flex-end;gap:10px}
</style>
