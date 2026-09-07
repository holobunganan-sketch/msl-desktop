import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { invalidate } from "./dataRevision";
import { addToast } from "./toast";
import { locale } from "$lib/i18n";

export type AiJob = { id:number; command:string; args:Record<string,unknown>; status:"running"|"completed"|"failed"|"interrupted"; result:unknown; error:string|null; created_at:number; finished_at:number|null };
export const aiJobs = writable<AiJob[]>([]);
export const translationDraft:{source:string;result:string;style:"written"|"spoken"}={source:"",result:"",style:"written"};
export const jobCommands = new Set(["ask_workbench","analyze_kol","run_analysis_now","start_workspace_work_draft","retry_analysis_run","translate_text","generate_report","retry_report","generate_brief","refresh_project_cognition","organize_inbox_item"]);
let refreshing = false;
const notified = new Set<number>();
function notifyFinished(job:AiJob) {
  if (job.status === "running" || notified.has(job.id)) return;
  notified.add(job.id);
  if(notified.size>200)notified.delete(notified.values().next().value!);
  const en=get(locale)==="en-US";
  addToast(job.status==="completed"?(en?"AI task finished. Results are ready in Background tasks.":"AI 后台任务已完成，可从“后台任务”查看结果。"):(en?"AI task did not finish. Check Background tasks for details.":"AI 任务未完成，请在“后台任务”查看原因。"),job.status==="completed"?"success":"error",6500);
}
export async function refreshJobs() {
  if (refreshing) return;
  refreshing = true;
  try {
    const previous = get(aiJobs);
    const jobs = await invoke<AiJob[]>("list_ai_jobs");
    aiJobs.set(jobs);
    for(const job of jobs)if(previous.find(old=>old.id===job.id)?.status==="running")notifyFinished(job);
    if (jobs.some(job => job.status !== "running" && previous.find(old => old.id === job.id)?.status === "running")) {
      invalidate("analysis","proposals","brief","works");
    }
  } finally { refreshing = false; }
}
export async function runAiJob<T>(command:string,args:Record<string,unknown>):Promise<T> {
  const job = await invoke<AiJob>("start_ai_job",{request:{command,args}});
  aiJobs.update(jobs => [job,...jobs.filter(item=>item.id!==job.id)]);
  // This promise lives in the application module, not in a page lifecycle.
  // The Rust task survives WebView destruction as well; reload reads its receipt.
  let current=job;
  while (current.status === "running") {
    await new Promise(resolve=>setTimeout(resolve,900));
    current=await invoke<AiJob>("get_ai_job",{id:job.id});
    aiJobs.update(jobs=>jobs.map(item=>item.id===job.id?current:item));
  }
  invalidate("analysis","proposals","brief","works");
  notifyFinished(current);
  if(current.status!=="completed") throw new Error(current.error || "后台任务未完成");
  return current.result as T;
}
