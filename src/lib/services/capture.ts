import {command} from './api';
import {invalidate} from '$lib/stores/dataRevision';
import type {InboxItem} from '$lib/types/domain';
export type CaptureContext={workId?:number|null;entityKind?:string|null;entityId?:number|null};
export async function captureNote(content:string,context:CaptureContext={}):Promise<InboxItem>{
  const note=await command<InboxItem>('capture_work_note',{content,workId:context.workId??null,entityKind:context.entityKind??null,entityId:context.entityId??null});
  invalidate('inbox','brief');
  return note;
}
export function organizeCapture(id:number):void {
  // The durable job store owns completion/error reporting across page changes.
  void command('organize_inbox_item',{inboxId:id}).catch(()=>{});
}
