<script lang="ts">
  import { onMount } from "svelte";
  import { locale, t } from "$lib/i18n";
  import {draftKey,readDraft,writeDraft,clearDraft} from "$lib/services/captureDrafts";
  import { invalidate } from "$lib/stores/dataRevision";
  import { addToast } from "$lib/stores/toast";
  import {captureNote,organizeCapture} from '$lib/services/capture';
  import {navigateTo} from '$lib/services/navigation';
  let {workId=null,prominent=false}:{workId?:number|null;prominent?:boolean}=$props();
  let busy=$state(false);
  let savedId=$state<number|null>(null);

  let text = $state("");
  const key=$derived(`quick:${draftKey({workId})}`);
  $effect(()=>{text=readDraft(key);savedId=null;});
  let inputEl = $state<HTMLInputElement | HTMLTextAreaElement | undefined>(undefined);
  let currentLocale = $derived($locale);

  async function save(organize=false) {
    const content = text.trim();
    if (!content||busy) return;
    busy=true;
    const submittedKey=key;
    const submittedText=text;
    try {
      const note=await captureNote(content,{workId,entityKind:workId?'work':null,entityId:workId});
      if(readDraft(submittedKey)===submittedText)clearDraft(submittedKey);
      if(key===submittedKey&&text===submittedText){savedId=note.id;text="";}
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

  function onKeydown(event:KeyboardEvent) {
    if(event.key==='Enter'&&!event.shiftKey&&!event.isComposing&&event.keyCode!==229){event.preventDefault();void save();}
    if(event.key==='Escape'&&!event.isComposing){text='';clearDraft(key);}
  }
  function onInput(event:Event) {
    text=(event.currentTarget as HTMLInputElement|HTMLTextAreaElement).value;
    writeDraft(key,text);
  }
  // The application shell owns native shortcuts; only the visible field listens.
  onMount(() => {
    window.addEventListener('dashboard:focus-capture',focusCapture);
    return ()=>window.removeEventListener('dashboard:focus-capture',focusCapture);
  });
</script>

<div class="capture-control" class:prominent>
{#if prominent}
  <div class="capture-heading-row"><label class="capture-heading" for="homepage-quick-capture">{currentLocale==='en-US'?'Capture a thought':'随手记一句'}</label><span id="homepage-capture-hint" class="capture-hint">{currentLocale==='en-US'?'Enter to save · Shift + Enter for a new line':'Enter 记下 · Shift + Enter 换行'}</span></div>
  <textarea id="homepage-quick-capture" data-testid="quick-capture" bind:this={inputEl} value={text} disabled={busy} oninput={onInput} onkeydown={onKeydown} rows="2" class="quick-capture" placeholder={currentLocale==='en-US'?'A thought, a conversation, or a change of plan…':'一个想法、一次交流、进展或调整，直接写在这里…'} aria-describedby="homepage-capture-hint"></textarea>
{:else}
<input
  data-testid="quick-capture"
  bind:this={inputEl}
  value={text}
  disabled={busy}
  oninput={onInput}
  class="quick-capture"
  title={currentLocale==='en-US'?'Enter saves only. Save & organize asks the secretary.':'按 Enter 只保存原话；点“记下并整理”交给秘书。'}
  aria-label={currentLocale==='en-US'?'Capture a note':'记一件事'}
  placeholder={workId?(currentLocale==='en-US'?'Record something for this project…':'记一下这个项目的新进展…'):(currentLocale==='en-US'?'Capture a note · Enter to save':'先记一下 · Enter 保存原话')}
  onkeydown={onKeydown}
/>
{/if}
<div class="capture-actions">
  <div class="capture-buttons">
    {#if prominent}<button data-testid="quick-capture-save" disabled={busy||!text.trim()} onclick={()=>save()} aria-busy={busy}>{currentLocale==='en-US'?'Save note':'记下'}</button>{/if}
    <button data-testid="quick-capture-organize" class:organize-primary={prominent} disabled={busy||!text.trim()} onclick={()=>save(true)} aria-busy={busy}>{currentLocale==='en-US'?'Save & organize':'记下并整理'}</button>
    <button class="capture-receipt" class:receipt-hidden={!savedId} disabled={!savedId||busy} aria-label={currentLocale==='en-US'?'View saved note':'查看刚记下的内容'} onclick={()=>savedId&&navigateTo('inbox',savedId)}>↗</button>
  </div>
</div>
</div>

<style>
  .capture-control{display:flex;align-items:center;gap:5px;min-width:0;width:100%}.capture-control input{min-width:0;flex:1}.capture-control button{flex-shrink:0;min-height:32px;padding:4px 9px;border:0;border-radius:8px;background:var(--color-primary-soft);color:var(--color-primary);font-size:12px;cursor:pointer}.capture-control button:disabled{opacity:.5}
  .capture-actions,.capture-buttons{display:flex;align-items:center;gap:8px;min-width:0}.capture-control .capture-receipt{width:28px;min-width:28px;padding-inline:0}.receipt-hidden{visibility:hidden}
  .capture-control.prominent{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:10px 16px;padding:14px 18px;border:1px solid var(--color-border-strong);border-left:3px solid var(--color-primary);border-radius:12px;background:var(--color-primary-soft);box-sizing:border-box}
  .capture-heading-row{grid-column:1/-1;display:flex;align-items:center;justify-content:space-between;gap:10px;min-width:0}
  .capture-heading{font-size:17px;font-weight:650;line-height:1.5;color:var(--color-text)}
  .prominent .quick-capture{font:inherit;font-size:16px;line-height:1.65;min-height:82px;height:82px;max-height:82px;resize:none;padding:12px 14px;overflow:auto;scrollbar-gutter:stable;background:var(--color-surface)}
  .prominent .capture-actions{align-self:end;padding-bottom:1px}.capture-hint{font-size:12px;line-height:1.6;color:var(--color-muted)}
  .prominent button{min-height:40px;min-width:88px;font-size:14px;padding:8px 14px;background:var(--color-surface);border:1px solid var(--color-border)}
  .prominent button.organize-primary{min-width:128px;background:var(--color-primary);border-color:var(--color-primary);color:white}.prominent .capture-receipt{min-width:32px;width:32px}
  .capture-control:focus-within{border-color:var(--color-primary)}
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
    box-shadow:0 0 0 2px var(--color-primary-soft);
  }
  @container(max-width:740px){.capture-control.prominent{grid-template-columns:minmax(0,1fr)}.prominent .capture-actions{justify-content:flex-end}.capture-heading-row{flex-wrap:wrap}}
</style>
