<script lang="ts">
  import {locale} from '$lib/i18n';
  let {view,onchange,testid='list-pagination'}:{view:{total:number;pages:number;page:number;start:number;end:number};onchange:(page:number)=>void;testid?:string}=$props();
  let en=$derived($locale==='en-US');
</script>
{#if view.pages>1}
<nav class="list-pager" aria-label={en?'List pages':'列表分页'} data-testid={testid} data-total={view.total}>
  <span aria-live="polite">{en?`${view.start}–${view.end} of ${view.total}`:`第 ${view.start}–${view.end} 条，共 ${view.total} 条`}</span>
  <div><button disabled={view.page===1} onclick={()=>onchange(view.page-1)}>{en?'Previous':'上一页'}</button><label><input aria-label={en?'Page number':'页码'} type="number" min="1" max={view.pages} value={view.page} onchange={(event)=>onchange(Number(event.currentTarget.value))}/><span>/ {view.pages}</span></label><button disabled={view.page===view.pages} onclick={()=>onchange(view.page+1)}>{en?'Next':'下一页'}</button></div>
</nav>
{/if}
<style>
 .list-pager{display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:12px;padding:12px 0;color:var(--color-muted);font-size:inherit;min-width:0}
 .list-pager>span{overflow-wrap:anywhere}.list-pager>div{display:flex;align-items:center;gap:12px;flex-wrap:wrap;min-width:0}
 .list-pager button{font:inherit;color:var(--color-text);background:var(--color-surface);border:1px solid var(--color-border);border-radius:10px;min-height:40px;padding:8px 14px;white-space:nowrap;cursor:pointer}.list-pager button:disabled{opacity:.45;cursor:default}.list-pager button:focus-visible{outline:2px solid var(--color-primary);outline-offset:2px}
 .list-pager label{display:flex;gap:8px;align-items:center;white-space:nowrap}.list-pager input{font:inherit;color:var(--color-text);background:var(--color-surface);border:1px solid var(--color-border);border-radius:10px;min-height:40px;width:5.5rem;text-align:center;padding:8px}
</style>
