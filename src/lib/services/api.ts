import { invoke } from "@tauri-apps/api/core";
import { jobCommands, runAiJob } from "$lib/stores/aiJobs";
import type { AiProposal, AnalysisRun, ClassificationMemoryStats, InboxItem, TodayData, Work } from "$lib/types/domain";

export function normalizeError(error: unknown): string {
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  if (typeof error === "object" && error !== null) {
    const record = error as Record<string, unknown>;
    for (const key of ["message", "error", "reason"]) {
      if (typeof record[key] === "string" && record[key]) return record[key] as string;
    }
    try {
      return JSON.stringify(error);
    } catch {
      return "Unknown error";
    }
  }
  return String(error || "Unknown error");
}

export async function command<T>(name: string, payload: Record<string, unknown> = {}): Promise<T> {
  try {
    if (jobCommands.has(name)) return await runAiJob<T>(name,payload);
    return await invoke<T>(name, payload);
  } catch (error) {
    throw new Error(normalizeError(error));
  }
}

export function createInboxItem(content: string): Promise<InboxItem> {
  return command<InboxItem>("create_inbox_item", { content });
}

export function getToday(dayStart: number, dayEnd: number): Promise<TodayData> {
  return command<TodayData>("get_today", { dayStart, dayEnd });
}

export function listWorks(status: string | null = null): Promise<Work[]> {
  return command<Work[]>("list_works", { status });
}

export function listAiProposals(status: string | null = "pending", limit = 50): Promise<AiProposal[]> {
  return command<AiProposal[]>("list_ai_proposals", { status, limit });
}

export function listLatestAnalysisProposals(limit = 20): Promise<AiProposal[]> {
  return command<AiProposal[]>("list_latest_analysis_proposals", { limit });
}

export function listRecentAiProposals(cutoffCreatedAt: number, status: string | null = null, limit = 200): Promise<AiProposal[]> {
  return command<AiProposal[]>("list_recent_ai_proposals", { cutoffCreatedAt, status, limit });
}

export function getClassificationMemoryStats(): Promise<ClassificationMemoryStats> {
  return command("get_classification_memory_stats");
}

export function listAnalysisRuns(limit = 20): Promise<AnalysisRun[]> {
  return command<AnalysisRun[]>("list_analysis_runs", { limit });
}

export function runAnalysisNow(trigger = "manual"): Promise<number> {
  return command<number>("run_analysis_now", { trigger });
}

export function updateAiProposalDraft(id: number, expectedUpdatedAt: number, title: string, payload: unknown): Promise<AiProposal> {
  return command<AiProposal>("update_ai_proposal_draft", { id, expectedUpdatedAt, title, payload });
}

export function updateAiProposalClassification(id: number, expectedUpdatedAt: number, kind: string, workId: number | null, title: string, payload: unknown): Promise<AiProposal> {
  return command<AiProposal>("update_ai_proposal_classification", { id, expectedUpdatedAt, kind, workId, title, payload });
}

export function deferAiProposal(id: number, expectedUpdatedAt: number): Promise<AiProposal> {
  return command<AiProposal>("defer_ai_proposal", { id, expectedUpdatedAt });
}

export function confirmAiProposal(id: number, expectedUpdatedAt: number, editedPayload?: unknown): Promise<{ proposal_id: number; kind: string; target_id: number; receipt_id: string }> {
  return command("confirm_ai_proposal", { id, expectedUpdatedAt, editedPayload: editedPayload ?? null });
}

export function rejectAiProposal(id: number, expectedUpdatedAt: number, reason?: string): Promise<void> {
  return command("reject_ai_proposal", { id, expectedUpdatedAt, reason: reason ?? null });
}

export function startWorkspaceWorkDraft(workspaceId: number | null, workId: number | null = null): Promise<number> {
  return command<number>("start_workspace_work_draft", { workspaceId, workId });
}
