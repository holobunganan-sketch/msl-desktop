<script lang="ts">
  import {locale} from '$lib/i18n';
  import {captureNote,organizeCapture,type CaptureContext} from '$lib/services/capture';
  import {draftKey,readDraft,writeDraft,clearDraft} from '$lib/services/captureDrafts';
  import {navigateTo} from '$lib/services/navigation';
  let {context={},placeholder='',label=''}:{context?:CaptureContext;placeholder?:string;label?:string}=$props();
  let text=$state('');let busy=$state(false);let error=$state('');let savedId=$state<number|null>(null);let requestedOrganization=$state(false);
  const key=$derived(draftKey(context));
  $effect(()=>{text=readDraft(key);savedId=null;error='';});
  const en=$derived($locale==='en-US');
  async function save(organize:boolean){
    if(busy||!text.trim())return;busy=true;error='';
    const submittedKey=key;
    try{const note=await captureNote(text.trim(),context);clearDraft(submittedKey);if(key===submittedKey){savedId=note.id;requestedOrganization=organize;text='';}if(organize)organizeCapture(note.id);}
    catch(e){error=String(e);}finally{busy=false;}
  }
</script>
<form class="natural-capture" data-testid="natural-capture" onsubmit={event=>{event.preventDefault();void save(true);}}>
  <label><strong>{label||(en?'What happened at work?':'有什么新进展？')}</strong><textarea data-testid="natural-capture-text" value={text} oninput={event=>{text=event.currentTarget.value;writeDraft(key,text);}} disabled={busy} rows="2" placeholder={placeholder||(en?'A reply arrived, a next step became clear, or something needs following up…':'例如：已收到反馈，等对方补充材料。直接说发生了什么，无需先分类。')}></textarea></label>
  {#if !text && !savedId}<div class="capture-examples" aria-label={en?'Draft starters':'不知道怎么写？从这里开始'}>{#each (en?['Progress: ','Schedule: ','Waiting for: ']:['新进展：','安排时间：','等待反馈：']) as example}<button type="button" data-testid="capture-example" onclick={()=>{text=example;writeDraft(key,text);}}>{example.replace(/[:：] ?$/,'')}</button>{/each}<small>{en?'Examples only fill the draft.':'点选只填入草稿，不会保存或发送。'}</small></div>{/if}
  <div class="capture-actions"><small>{en?'Saved first. The secretary drafts changes for your confirmation.':'先保存原话，秘书在后台整理，您确认后才更新项目与事项。'}</small><button type="button" disabled={busy||!text.trim()} onclick={()=>save(false)}>{en?'Just save':'先记下来'}</button><button class="primary" data-testid="natural-capture-submit" type="submit" disabled={busy||!text.trim()}>{en?'Save & organize':'记下并整理'}</button></div>
  {#if savedId}<p class="capture-success" role="status">{requestedOrganization?(en?'Saved. Organizing results will appear in suggestions.':'已记下，整理结果会出现在建议中。'):(en?'Saved in To organize. Ask the secretary when you are ready.':'已记下，留在待整理；需要时再交给秘书。')} <button type="button" onclick={()=>navigateTo('inbox',savedId!)}>{en?'View note':'查看记录'}</button>{#if requestedOrganization}<button type="button" onclick={()=>navigateTo('review')}>{en?'View suggestions':'查看建议'}</button>{/if}</p>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
</form>
<style>
  .capture-examples{display:flex;flex-wrap:wrap;gap:8px;align-items:center}.capture-examples small{flex-basis:220px}
  .natural-capture{padding:18px;border:1px solid var(--color-border);border-radius:14px;background:var(--color-surface-muted);display:grid;gap:12px;min-width:0}label{display:grid;gap:10px;font-size:15px}textarea{width:100%;padding:12px 14px;line-height:1.65;font-size:15px;resize:vertical;min-height:84px}.capture-actions{display:flex;flex-wrap:wrap;align-items:center;gap:8px}small{flex:1 1 240px;color:var(--color-muted);font-size:13px;line-height:1.6}button{font:inherit;font-size:13px;padding:8px 12px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface);cursor:pointer}.primary{background:var(--color-primary);color:white}button:disabled{opacity:.5;cursor:default}p{margin:0;line-height:1.6;font-size:14px;overflow-wrap:anywhere}.capture-success{color:var(--color-success)}p[role=alert]{color:var(--color-danger)}
</style>
