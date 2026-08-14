<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  let text = $state("");
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  function save() {
    const content = text.trim();
    if (!content) return;
    invoke("create_inbox_item", { content }).catch((e) => console.error(e));
    text = "";
  }

  // 托盘 Quick Capture 菜单 → 聚焦输入框（指南 §7.8）
  $effect(() => {
    const p = listen("quick-capture", () => {
      inputEl?.focus();
      inputEl?.select();
    });
    return () => {
      p.then((un) => un());
    };
  });
</script>

<input
  bind:this={inputEl}
  bind:value={text}
  class="quick-capture"
  placeholder="Quick Capture：快速记下一件事，Enter 存入 Inbox，Esc 清空"
  onkeydown={(e) => {
    if (e.key === "Enter") save();
    if (e.key === "Escape") text = "";
  }}
/>

<style>
  .quick-capture {
    width: 100%;
    box-sizing: border-box;
    padding: 9px 12px;
    border: 1px solid #c8ccd1;
    border-radius: 8px;
    font-size: 13px;
    outline: none;
  }
  .quick-capture:focus {
    border-color: #4a7ab5;
  }
</style>
