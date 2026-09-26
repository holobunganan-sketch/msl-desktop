<script lang="ts">
 import {locale} from '$lib/i18n';import {completionHistory,refreshCompletions,restoreManual} from '$lib/stores/manualCompletions';
 import type {CompletionReceipt} from '$lib/services/manualActions';import {navigateTo} from '$lib/services/navigation';
 import {paginate} from '$lib/services/pagination';import ListPager from './ui/ListPager.svelte';import StatusLine from './ui/StatusLine.svelte';
 let open=$state(false),page=$state(1),busy=$state(''),error=$state('');const en=$derived($locale==='en-US');const items=$derived(paginate($completionHistory,page,8));
 async function load(){try{await refreshCompletions();error='';}catch(e){error=String(e);}}
 async function undo(receipt:CompletionReceipt){if(busy)return;busy=receipt.id;error='';try{await restoreManual(receipt);}catch(e){error=String(e);}finally{busy='';}}
</script>
<details bind:open data-testid="recent-completions" ontoggle={()=>{if(open)void load();}}><summary>{en?'Recent completions':'最近完成'}</summary>
 <StatusLine message={error}/><p>{en?'Restore recent manual completions. Later edits are protected.':'可恢复最近手动完成的事项；有后续修改的记录会受到保护。'}</p>
 <ListPager view={items} onchange={value=>page=value}/>
 {#each items.items as receipt(receipt.id)}<div class="receipt" data-testid={'completion-history-'+receipt.id}><span><strong>{receipt.title}</strong><small>{new Date(receipt.created_at*1000).toLocaleString($locale)} · {receipt.entity_kind==='task'?(en?'Task':'事项'):(en?'Waiting':'等待')}</small></span>{#if receipt.undone_at}<small>{en?'Restored':'已恢复'}</small>{:else}<button disabled={!!busy} onclick={()=>undo(receipt)}>{receipt.entity_kind==='task'?(en?'Restore unfinished':'恢复为未完成'):(en?'Restore waiting':'恢复为等待中')}</button>{/if}<button onclick={()=>navigateTo(receipt.entity_kind,receipt.entity_id)}>{en?'View':'查看'}</button></div>{:else}<p>{en?'No recent manual completions.':'暂无最近手动完成的记录。'}</p>{/each}
</details>
<style>details{border-bottom:1px solid var(--color-border);padding:12px 0}summary{cursor:pointer;color:var(--color-primary);font-size:15px}p,small{color:var(--color-muted);font-size:13px}.receipt{display:flex;flex-wrap:wrap;align-items:center;gap:10px;padding:12px 0;border-top:1px solid var(--color-border)}.receipt>span{flex:1 1 240px;min-width:0;display:grid;gap:5px}.receipt strong{overflow-wrap:anywhere;font-size:15px}.receipt button{min-height:40px;padding:5px 10px;border:1px solid var(--color-border);background:var(--color-surface);border-radius:6px;color:var(--color-primary);font:inherit}</style>
