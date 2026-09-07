// Session-only drafts survive navigation. They never become AI input until saved.
const drafts=new Map<string,string>();
export function draftKey(context:{workId?:number|null;entityKind?:string|null;entityId?:number|null}):string {
  return `${context.workId??'independent'}:${context.entityKind??'note'}:${context.entityId??'new'}`;
}
export function readDraft(key:string):string{return drafts.get(key)??'';}
export function writeDraft(key:string,text:string):void{if(text)drafts.set(key,text);else drafts.delete(key);}
export function clearDraft(key:string):void{drafts.delete(key);}
