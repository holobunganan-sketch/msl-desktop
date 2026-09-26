<script lang="ts">
 import {untrack} from 'svelte';import DeletionRecordPreview from './DeletionRecordPreview.svelte';
 import {invoke} from '@tauri-apps/api/core';import {locale} from '$lib/i18n';import {invalidate} from '$lib/stores/dataRevision';
 import {deletionRequest,type DeletableRecord} from '$lib/services/manualActions';import Modal from './ui/Modal.svelte';import AppButton from './ui/AppButton.svelte';import StatusLine from './ui/StatusLine.svelte';
 let {record,kind,works=[],onclose,oncomplete}:{record:DeletableRecord|null;kind:'task'|'waiting';works?:{id:number;title:string}[];onclose:()=>void;oncomplete:()=>void|Promise<void>}=$props();
 let snapshot=$state<DeletableRecord|null>(untrack(()=>record?JSON.parse(JSON.stringify(record)):null)),busy=$state(false),error=$state('');let en=$derived($locale==='en-US');
 $effect(()=>{snapshot=record?JSON.parse(JSON.stringify(record)):null;error='';});
 async function refresh(){if(!snapshot)return;busy=true;try{const rows=await invoke<DeletableRecord[]>(kind==='task'?'list_tasks':'list_waiting_items',{status:null,workId:null});const current=rows.find(r=>r.id===snapshot?.id);if(!current)throw Error(en?'This item no longer exists.':'该事项已不存在。');snapshot=current;error=en?'Current version loaded. Please confirm again.':'已载入当前版本，请核对后重新确认。';}catch(e){error=String(e);}finally{busy=false;}}
 async function remove(){if(!snapshot||busy)return;busy=true;error='';try{await invoke(kind==='task'?'delete_task':'delete_waiting',deletionRequest(snapshot));invalidate('tasks','waiting','works','calendar','proposals','analysis','brief');await oncomplete();onclose();}catch(e){error=String(e);}finally{busy=false;}}
</script>
<Modal open={record!==null} title={en?'Delete this item?':'删除这条事项？'} onclose={onclose} dismissible={!busy}>
 <p>{en?'The record and related secretary advice will be removed. Source files remain unchanged. This deletion cannot be undone.':'将删除这条记录，并结束对应的秘书建议。源文件保持不变。此删除无法撤销。'}</p><StatusLine message={error}/>
 {#if snapshot}<DeletionRecordPreview record={snapshot} {works}/>{/if}
 {#snippet footer()}{#if error}<AppButton variant="secondary" disabled={busy} onclick={refresh}>{en?'Reload current version':'刷新当前版本'}</AppButton>{/if}<AppButton variant="secondary" disabled={busy} onclick={onclose}>{en?'Keep it':'保留'}</AppButton><AppButton variant="danger" testid="item-delete-confirm" loading={busy} onclick={remove}>{en?'Confirm deletion':'确认删除'}</AppButton>{/snippet}
</Modal>
