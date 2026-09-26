export type CompletionReceipt={id:string;entity_kind:'task'|'waiting';entity_id:number;title:string;created_at:number;undone_at:number|null};
export type DeletableRecord={id:number;title:string;updated_at:number;work_id?:number|null;status?:string;notes?:string|null;priority?:string;waiting_for?:string;due_at?:number|null;scheduled_start?:number|null;scheduled_end?:number|null;follow_up_at?:number|null;started_at?:number|null};
export function deletionRequest(record:DeletableRecord){return {id:record.id,confirmed:true,expectedUpdatedAt:record.updated_at,expectedRecord:JSON.parse(JSON.stringify(record))};}
export function createManualActions(call:(name:string,args:Record<string,unknown>)=>Promise<unknown>,changed:()=>void){
 const pending=new Map<string,Promise<CompletionReceipt>>();
 const undoing=new Map<string,Promise<void>>();
 return {
  complete(kind:'task'|'waiting',id:number):Promise<CompletionReceipt>{
   const key=`${kind}:${id}`;if(pending.has(key))return pending.get(key)!;
   const promise=call(kind==='task'?'complete_task':'resolve_waiting',{id}).then(value=>{changed();return value as CompletionReceipt;}).finally(()=>pending.delete(key));pending.set(key,promise);return promise;
  },
  undo(receipt:CompletionReceipt):Promise<void>{if(undoing.has(receipt.id))return undoing.get(receipt.id)!;
   const promise=call('undo_manual_completion',{receiptId:receipt.id}).then(()=>{changed();}).finally(()=>undoing.delete(receipt.id));undoing.set(receipt.id,promise);return promise;
  }
 };
}
