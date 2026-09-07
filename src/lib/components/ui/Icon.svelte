<script lang="ts">
  export type IconName =
    | "dashboard" | "folder" | "work" | "check" | "clock" | "calendar"
    | "inbox" | "sparkles" | "review" | "settings" | "search" | "languages"
    | "plus" | "chevron-right" | "file" | "activity" | "capture" | "more"
    | "refresh" | "copy" | "menu" | "user" | "archive" | "edit" | "reports";

  let { name, size = 18, strokeWidth = 1.7, label = "" }: {
    name: IconName; size?: number; strokeWidth?: number; label?: string;
  } = $props();
</script>

<svg class="icon" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" aria-hidden={label ? undefined : "true"} role={label ? "img" : undefined}>
  {#if label}<title>{label}</title>{/if}
  {#if name === "dashboard"}
    <rect x="3" y="3" width="7" height="7" rx="2"/><rect x="14" y="3" width="7" height="4" rx="2"/><rect x="14" y="11" width="7" height="10" rx="2"/><rect x="3" y="14" width="7" height="7" rx="2"/>
  {:else if name === "folder"}<path d="M3.5 7.5h6l2-2h3l2 2h4v11a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/><path d="M3.5 10h17"/>
  {:else if name === "work"}<rect x="3" y="6" width="18" height="14" rx="2"/><path d="M8 6V4h8v2M3 11h18M10 11v2h4v-2"/>
  {:else if name === "check"}<rect x="3" y="3" width="18" height="18" rx="5"/><path d="m7.5 12 3 3 6-7"/>
  {:else if name === "clock"}<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>
  {:else if name === "calendar"}<rect x="3" y="5" width="18" height="16" rx="3"/><path d="M8 3v4M16 3v4M3 10h18"/>
  {:else if name === "inbox"}<path d="M4 4h16v15H4z"/><path d="M4 14h4l2 3h4l2-3h4"/>
  {:else if name === "sparkles"}<path d="m12 3 1.3 4.1L17 9l-3.7 1.9L12 15l-1.3-4.1L7 9l3.7-1.9zM18 15l.7 2.3L21 18l-2.3.7L18 21l-.7-2.3L15 18l2.3-.7zM5 3l.6 1.7L7 5.5l-1.4.8L5 8l-.6-1.7L3 5.5l1.4-.8z"/>
  {:else if name === "review"}<path d="M7 3h10l3 3v15H4V6z"/><path d="M7 11h10M7 15h6M14 3v4h4"/>
  {:else if name === "settings"}<circle cx="12" cy="12" r="3"/><path d="M19 15a2 2 0 0 0 .4 2.2l-2.2 2.2A2 2 0 0 0 15 19l-1 .4V22h-4v-2.6L9 19a2 2 0 0 0-2.2.4l-2.2-2.2A2 2 0 0 0 5 15l-.4-1H2v-4h2.6L5 9a2 2 0 0 0-.4-2.2l2.2-2.2A2 2 0 0 0 9 5l1-.4V2h4v2.6l1 .4a2 2 0 0 0 2.2-.4l2.2 2.2A2 2 0 0 0 19 9l.4 1H22v4h-2.6z"/>
  {:else if name === "search"}<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 4 4"/>
  {:else if name === "languages"}<path d="M4 5h9M8.5 3v2M5 9c2 0 5-1 6-4M6 5c1 3 3 5 6 6M14 20l3.5-9 3.5 9M15.4 17h4.2"/>
  {:else if name === "plus"}<path d="M12 5v14M5 12h14"/>
  {:else if name === "chevron-right"}<path d="m9 6 6 6-6 6"/>
  {:else if name === "file"}<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v5h5"/>
  {:else if name === "activity"}<path d="M3 12h4l2-6 4 12 2-6h6"/>
  {:else if name === "capture"}<path d="M12 20h9M16.5 3.5a2.1 2.1 0 0 1 3 3L8 18l-4 1 1-4z"/>
  {:else if name === "more"}<circle cx="5" cy="12" r="1" fill="currentColor"/><circle cx="12" cy="12" r="1" fill="currentColor"/><circle cx="19" cy="12" r="1" fill="currentColor"/>
  {:else if name === "refresh"}<path d="M20 11a8 8 0 1 0-2.3 5.7M20 5v6h-6"/>
  {:else if name === "copy"}<rect x="8" y="8" width="11" height="12" rx="2"/><path d="M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h3"/>
  {:else if name === "menu"}<path d="M4 7h16M4 12h16M4 17h16"/>
  {:else if name === "user"}<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>
  {:else if name === "archive"}<path d="M4 7h16v13H4zM3 3h18v4H3zM9 11h6"/>
  {:else if name === "edit"}<path d="M13.5 6.5 17.5 10.5M4 20l4.5-1 10-10a2.8 2.8 0 0 0-4-4l-10 10z"/>
  {:else if name === "reports"}<path d="M5 3h12l3 3v15H5z"/><path d="M8 16V11M12 16V8M16 16v-3M17 3v4h4"/>
  {/if}
</svg>

<style>.icon{display:block;flex:0 0 auto}</style>
