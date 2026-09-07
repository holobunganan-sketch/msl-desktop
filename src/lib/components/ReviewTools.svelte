<script lang="ts">
  import { command } from "$lib/services/api";
  import { locale, translateKind } from "$lib/i18n";
  import Modal from "$lib/components/ui/Modal.svelte";
  import { invalidate } from "$lib/stores/dataRevision";
  let {onchange=()=>{},feedbackCount=0}:{onchange?:()=>void;feedbackCount?:number}=$props();
  type Card={id:number;cue:string;suggested_kind:string;preferred_kind:string|null;preferred_work_id:number|null;accepted:number;corrected:number;rejected:number;updated_at:number};
  type Receipt={id:string;proposal_ids:number[];created_at:number;undone_at:number|null};
  let open=$state(false);let busy=$state(false);let error=$state("");
  let cards=$state<Card[]>([]);let receipts=$state<Receipt[]>([]);let works=$state<{id:number;title:string}[]>([]);
  let tab=$state<"memory"|"undo">("memory");let en=$derived($locale==="en-US");
  const kinds=["work","task","waiting","calendar","inbox","resume_point"];
  function kindLabel(kind:string){return ({work:["工作","Work"],task:["任务计划","Task"],waiting:["等待事项","Waiting"],calendar:["日历","Calendar"],inbox:["收件箱","Inbox"],resume_point:["工作进展","Progress"]}[kind]??[kind,kind])[en?1:0];}
  async function load(){busy=true;error="";try{[cards,receipts,works]=await Promise.all([command<Card[]>("list_classification_memories"),command<Receipt[]>("list_confirmation_receipts"),command<{id:number;title:string}[]>("list_works",{status:null})]);}catch(e){error=String(e);}finally{busy=false;}}
  async function edit(card:Card,forget=false){busy=true;error="";try{await command("edit_classification_memory",{id:card.id,expectedUpdatedAt:card.updated_at,preferredKind:forget?null:(card.preferred_kind||card.suggested_kind),workId:card.preferred_work_id});await load();onchange();}catch(e){error=String(e);}finally{busy=false;}}
  async function undo(receipt:Receipt){busy=true;error="";try{await command("undo_ai_confirmation",{receiptId:receipt.id});invalidate("analysis","proposals","works","brief");await load();onchange();}catch(e){error=String(e);}finally{busy=false;}}
</script>
<button class="tools-entry" data-testid="review-tools" onclick={()=>{open=true;void load();}}>{en?"Memory & undo":"分类记忆与撤销"}{feedbackCount?` · ${feedbackCount}`:""}</button>
<Modal bind:open title={en?"Your decisions":"您的决定"}>
  <div class="tabs"><button class:active={tab==="memory"} onclick={()=>tab="memory"}>{en?"Classification memory":"分类记忆"}</button><button class:active={tab==="undo"} onclick={()=>tab="undo"}>{en?"Undo confirmations":"撤销确认"}</button></div>
  <p>{tab==="memory"?(en?"Only explicit acceptance and category corrections influence preferences. Deferring, duplicates and completed items do not count as category errors.":"确认和明确的分类纠正会影响偏好。暂缓、重复和已完成的事项不会被当作分类错误。可更正或忘记这些记忆。"):(en?"Undo restores the selected confirmation group, including feedback and inbox links. Later changes are protected; source files stay unchanged.":"撤销会还原该组确认前的记录、反馈和收件箱关联。已有后续修改的事项会受到保护，源文件始终保持原样。")}</p>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if tab==="memory"}
    {#each cards as card(card.id)}
      <article class="memory-card" data-testid={`memory-${card.id}`}><strong>{card.cue}</strong><small>{en?"Confirmed / corrected / rejected":"确认 / 纠正 / 判断有误"}：{card.accepted} / {card.corrected} / {card.rejected}</small>
        <div class="fields"><label>{en?"Preferred category":"偏好分类"}<select bind:value={card.preferred_kind}><option value={null}>{kindLabel(card.suggested_kind)}</option>{#each kinds as kind}<option value={kind}>{kindLabel(kind)}</option>{/each}</select></label>
        <label>{en?"Project":"工作归属"}<select bind:value={card.preferred_work_id}><option value={null}>{en?"Temporary":"临时事务"}</option>{#each works as work}<option value={work.id}>{work.title}</option>{/each}</select></label></div>
        <div class="actions"><button disabled={busy} onclick={()=>edit(card,true)}>{en?"Forget this memory":"忘记此条记忆"}</button><button disabled={busy} onclick={()=>edit(card)}>{en?"Save correction":"保存纠正"}</button></div>
      </article>
    {:else}<p>{en?"No classification memories yet.":"暂无分类记忆。"}</p>{/each}
  {:else}
    {#each receipts as receipt(receipt.id)}<article class="memory-card"><strong>{en?"Confirmed suggestions":"确认事项"} {receipt.proposal_ids.map(id=>`#${id}`).join("、")}</strong><small>{new Date(receipt.created_at*1000).toLocaleString($locale)}</small><button data-testid={`undo-${receipt.id}`} disabled={busy||!!receipt.undone_at} onclick={()=>undo(receipt)}>{receipt.undone_at?(en?"Undone":"已撤销"):(en?"Undo this confirmation":"撤销这次确认")}</button></article>{:else}<p>{en?"New confirmations will appear here.":"新版确认记录会显示在这里。"}</p>{/each}
  {/if}
</Modal>
<style>
 button{min-height:36px;font:inherit;font-size:14px;padding:8px 12px;color:var(--color-primary);background:var(--color-primary-soft);border:1px solid var(--color-border);border-radius:9px;cursor:pointer}button:disabled{opacity:.55;cursor:default}.tools-entry{white-space:normal;line-height:1.5}.tabs,.actions{display:flex;gap:8px;flex-wrap:wrap}.tabs .active{color:white;background:var(--color-primary)}p{font-size:14px;color:var(--color-muted);line-height:1.7}.memory-card{display:grid;gap:12px;border:1px solid var(--color-border);border-radius:12px;padding:16px;margin-top:12px;min-width:0}.memory-card strong{font-size:16px;overflow-wrap:anywhere}.memory-card small{font-size:13px;color:var(--color-muted)}.fields{display:grid;gap:12px;grid-template-columns:repeat(auto-fit,minmax(min(170px,100%),1fr))}label{font-size:14px;display:grid;gap:7px}select{min-width:0;width:100%;padding:9px;font:inherit;min-height:40px;border-radius:8px;border:1px solid var(--color-border);background:var(--color-surface);color:var(--color-text)}.error{color:var(--color-danger);overflow-wrap:anywhere}
</style>
