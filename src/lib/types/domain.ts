export type Workspace = {
  id: number;
  name: string;
  root_path: string;
  enabled: boolean;
  created_at: number;
  updated_at: number;
};

export type Work = {
  id: number;
  title: string;
  status: string;
  summary: string | null;
  created_at: number;
  updated_at: number;
  archived_at: number | null;
};

export type ResumePoint = {
  id: number;
  work_id: number;
  current_state: string;
  next_step: string;
  remember: string;
  source: string;
  created_at: number;
};

export type Task = {
  id: number;
  work_id: number | null;
  title: string;
  status: string;
  priority: string;
  due_at: number | null;
  scheduled_start: number | null;
  scheduled_end: number | null;
  notes: string | null;
  created_at: number;
  updated_at: number;
  completed_at: number | null;
};

export type WaitingItem = {
  id: number;
  work_id: number | null;
  title: string;
  waiting_for: string;
  started_at: number;
  follow_up_at: number | null;
  status: string;
  notes: string | null;
  created_at: number;
  updated_at: number;
  resolved_at: number | null;
};

export type CalendarEvent = {
  id: number;
  work_id: number | null;
  title: string;
  start_at: number;
  end_at: number | null;
  all_day: boolean;
  location: string | null;
  notes: string | null;
  kind: string;
  created_at: number;
  updated_at: number;
};

export type InboxItem = {
  id: number;
  content: string;
  created_at: number;
  processed_at: number | null;
  converted_to_type: string | null;
  converted_to_id: number | null;
};

export type ActivityEvent = {
  id: number;
  timestamp: number;
  event_type: string;
  workspace_id: number | null;
  work_id: number | null;
  entity_type: string | null;
  entity_id: number | null;
  path: string | null;
  display_text: string;
  metadata_json: string | null;
  dedupe_key: string | null;
};

export type TodayData = {
  continue_works: Array<{ work: Work; latest_resume: ResumePoint | null; last_activity_at: number | null; files: Array<{ id: number; work_id: number; workspace_id: number | null; path: string; label: string | null; pinned: boolean; created_at: number }> }>;
  today_tasks: Task[];
  today_calendar: CalendarEvent[];
  waiting_followups: WaitingItem[];
  inbox_pending: InboxItem[];
};

export type Provider = {
  id: number;
  display_name: string;
  provider_type: string;
  base_url: string;
  model: string;
  enabled: boolean;
  created_at: number;
  updated_at: number;
};

export type ProviderConnection = {
  id: number;
  display_name: string;
  provider_type: string;
  base_url: string;
  legacy_model: string;
  enabled: boolean;
  credential_ref?: string;
  template_kind: "deepseek" | "opencode_go" | "custom" | string;
  auth_mode: string;
  models_endpoint: string | null;
  last_models_refresh_at: number | null;
  created_at: number;
  updated_at: number;
};

export type ProviderModel = {
  id: number;
  provider_id: number;
  model_id: string;
  display_name: string;
  protocol: "chat_completions" | "responses" | "anthropic_messages" | string;
  endpoint_path: string;
  capabilities_json: string;
  source: string;
  enabled: boolean;
  available: boolean;
  created_at: number;
  updated_at: number;
};

export type AiTaskRoute = {
  task_kind: "workspace_analysis" | "work_draft" | "global_analysis" | "daily_brief" | "weekly_report" | "monthly_report" | "translation" | "general" | string;
  provider_model_id: number | null;
  updated_at: number;
};

export type BriefSourceCounts = Record<string, number>;

export type BriefResult = {
  id?: number;
  content: string;
  source_counts: BriefSourceCounts;
  source_preview?: unknown[];
  ai_used: boolean;
  warning?: string | null;
  period_start: number;
  period_end: number;
  locale: string;
};

export type AiProposal = {
  id: number;
  analysis_run_id: number | null;
  kind: "work" | "task" | "waiting" | "calendar" | "inbox" | "resume_point" | string;
  suggested_kind: string;
  operation: "create" | "update";
  target_id: number | null;
  work_id: number | null;
  suggested_work_id: number | null;
  workspace_id: number | null;
  dedupe_key: string;
  title: string;
  payload_json: string;
  reason: string;
  source_refs_json: string;
  confidence: number | null;
  user_edited: boolean;
  status: "pending" | "confirmed" | "rejected" | "superseded" | string;
  created_at: number;
  updated_at: number;
  decided_at: number | null;
  deferred_at: number | null;
};

export type ClassificationMemoryStats = {
  pattern_count: number;
  feedback_count: number;
  accepted_count: number;
  corrected_count: number;
  rejected_count: number;
  updated_at: number | null;
};

export type AnalysisRun = {
  id: number;
  trigger: string;
  status: "running" | "completed" | "failed" | string;
  period_start: number | null;
  period_end: number | null;
  provider_model_id: number | null;
  started_at: number;
  finished_at: number | null;
  source_counts_json: string | null;
  snapshot_hash: string | null;
  summary: string | null;
  brief_id: number | null;
  error_code: string | null;
  error_message: string | null;
  created_at: number;
};

export type Report = {
  id: number;
  kind: "weekly" | "monthly";
  period_start: number;
  period_end: number;
  status: "running" | "completed" | "failed";
  provider_model_id: number | null;
  content: string | null;
  snapshot_hash: string | null;
  source_counts_json: string;
  source_report_ids_json: string;
  error_code: string | null;
  error_message: string | null;
  retention_state: "kept" | "superseded";
  generated_at: number | null;
  created_at: number;
  updated_at: number;
};

export type ReportSchedule = {
  id: number;
  weekly_enabled: boolean;
  weekly_weekday: number;
  weekly_hour: number;
  weekly_minute: number;
  last_weekly_period_key: string | null;
  monthly_enabled: boolean;
  monthly_day: number;
  monthly_hour: number;
  monthly_minute: number;
  last_monthly_period_key: string | null;
  updated_at: number;
};
