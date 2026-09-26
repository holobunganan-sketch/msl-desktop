import {writable} from 'svelte/store';
import {invoke} from '@tauri-apps/api/core';
import {createManualActions,type CompletionReceipt} from '../services/manualActions';
import {invalidate} from './dataRevision';
export const latestCompletion=writable<CompletionReceipt|null>(null);
export const completionHistory=writable<CompletionReceipt[]>([]);
const actions=createManualActions(invoke,()=>invalidate('tasks','waiting','works','calendar','proposals','analysis','brief'));
export async function refreshCompletions(){completionHistory.set(await invoke<CompletionReceipt[]>('list_manual_completions'));}
export async function completeManual(kind:'task'|'waiting',id:number){const receipt=await actions.complete(kind,id);latestCompletion.set(receipt);completionHistory.update(rows=>[receipt,...rows.filter(r=>r.id!==receipt.id)].slice(0,100));return receipt;}
export async function restoreManual(receipt:CompletionReceipt){await actions.undo(receipt);const undone={...receipt,undone_at:Math.floor(Date.now()/1000)};latestCompletion.set(undone);completionHistory.update(rows=>rows.map(r=>r.id===receipt.id?undone:r));}
