<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { register as registerShortcut } from "@tauri-apps/plugin-global-shortcut";
  import { locale, t } from "$lib/i18n";
  import {draftKey,readDraft,writeDraft,clearDraft} from "$lib/services/captureDrafts";
  import { invalidate } from "$lib/stores/dataRevision";
  import { addToast } from "$lib/stores/toast";
  import {captureNote,organizeCapture} from '$lib/services/capture';
  import {navigateTo} from '$lib/services/navigation';
  let {workId=null}:{workId?:number|null}=$props();
  let busy=$state(false);
  let savedId=$state<number|null>(null);

  let text = $state("");
  const key=$derived(`quick:${draftKey({workId})}`);
  $effect(()=>{text=readDraft(key);savedId=null;});
  let inputEl = $state<HTMLInputElement | undefined>(undefined);
  let currentLocale = $derived($locale);

  async function save(organize=false) {
    const content = text.trim();
    if (!content||busy) return;
    busy=true;
    const submittedKey=key;
    try {
      const note=await captureNote(content,{workId,entityKind:workId?'work':null,entityId:workId});
      clearDraft(submittedKey);
      if(key===submittedKey){savedId=note.id;text="";}
      if(organize)organizeCapture(note.id);
      addToast(organize?(currentLocale==='en-US'?'Saved. Look for results in Secretary suggestions.':'已记下，整理结果会出现在“秘书准备的建议”中。'):(currentLocale==='en-US'?'Saved in Matters → To organize.':'已记在“事项 → 待整理”，需要时再交给秘书。'), "success");
      invalidate("inbox", "brief");
    } catch (error) {
      addToast(t("feedback.captureFailed", { error: error instanceof Error ? error.message : String(error) }, currentLocale), "error");
    } finally {busy=false;}
  }

  function focusCapture() {
    inputEl?.focus();
    inputEl?.select();
  }

  // 托盘 Quick Capture 菜单 → 聚焦输入框（指南 §7.8）
  $effect(() => {
    const p = listen("quick-capture", () => focusCapture());
    return () => {
      p.then((un) => un());
    };
  });

  // 全局快捷键 Ctrl+Shift+Space → Quick Capture
  $effect(() => {
    registerShortcut("CommandOrControl+Shift+Space", () => focusCapture()).catch(() => {});
  });
</script>

<div class="capture-control"><input
  data-testid="quick-capture"
  bind:this={inputEl}
  value={text}
  disabled={busy}
  oninput={event=>{text=event.currentTarget.value;writeDraft(key,text);}}
  class="quick-capture"
  title={currentLocale==='en-US'?'Enter saves only. Save & organize asks the secretary.':'按 Enter 只保存原话；点“记下并整理”交给秘书。'}
  aria-label={currentLocale==='en-US'?'Capture a note':'记一件事'}
  placeholder={workId?(currentLocale==='en-US'?'Record something for this project…':'记一下这个项目的新进展…'):(currentLocale==='en-US'?'Capture a note · Enter to save':'先记一下 · Enter 保存原话')}
  onkeydown={(e) => {
    if (e.key === "Enter" && !e.isComposing) save();
    if (e.key === "Escape") {text = "";clearDraft(key);}
  }}
/><button data-testid="quick-capture-organize" disabled={busy||!text.trim()} onclick={()=>save(true)}>{currentLocale==='en-US'?'Organize':'记下并整理'}</button>{#if savedId}<button class="capture-receipt" aria-label={currentLocale==='en-US'?'View saved note':'查看刚记下的内容'} onclick={()=>navigateTo('inbox',savedId!)}>↗</button>{/if}</div>

<style>
  .capture-control{display:flex;align-items:center;gap:5px;min-width:0;width:100%}.capture-control input{min-width:0;flex:1}.capture-control button{flex-shrink:0;min-height:32px;padding:4px 9px;border:0;border-radius:8px;background:var(--color-primary-soft);color:var(--color-primary);font-size:12px;cursor:pointer}.capture-control button:disabled{opacity:.5}
  .quick-capture {
    width: 100%;
    box-sizing: border-box;
    padding: 9px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface-raised);
    color: var(--color-text);
    font-size: 12px;
    outline: none;
  }
  .quick-capture:focus {
    border-color: var(--color-primary);
  }
</style>
