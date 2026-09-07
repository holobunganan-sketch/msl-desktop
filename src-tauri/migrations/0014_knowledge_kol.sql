-- Durable knowledge, never a cache; source workspaces remain read-only.
CREATE TABLE qa_sessions (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 title TEXT NOT NULL,
 scope_json TEXT NOT NULL DEFAULT '[]',
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
);
CREATE TABLE qa_turns (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 session_id INTEGER NOT NULL REFERENCES qa_sessions(id) ON DELETE CASCADE,
 question TEXT NOT NULL,
 scope_json TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','completed','failed','interrupted')),
 answer_json TEXT,
 evidence_json TEXT,
 error TEXT,
 created_at INTEGER NOT NULL,
 finished_at INTEGER
);
CREATE INDEX qa_turn_session ON qa_turns(session_id,id);
CREATE UNIQUE INDEX qa_turn_active ON qa_turns(session_id) WHERE status IN ('pending','running');
CREATE TABLE kol_experts (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 name TEXT NOT NULL CHECK(length(trim(name))>0),
 institution TEXT NOT NULL CHECK(length(trim(institution))>0),
 specialty TEXT NOT NULL DEFAULT '',
 summary TEXT NOT NULL DEFAULT '',
 archived INTEGER NOT NULL DEFAULT 0,
 revision INTEGER NOT NULL DEFAULT 1,
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
);
CREATE INDEX kol_expert_name ON kol_experts(name,institution);
CREATE TABLE kol_projects (
 expert_id INTEGER NOT NULL REFERENCES kol_experts(id) ON DELETE CASCADE,
 work_id INTEGER NOT NULL REFERENCES works(id) ON DELETE CASCADE,
 PRIMARY KEY(expert_id,work_id)
);
CREATE TABLE kol_notes (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 expert_id INTEGER NOT NULL REFERENCES kol_experts(id),
 work_id INTEGER REFERENCES works(id) ON DELETE SET NULL,
 inbox_id INTEGER UNIQUE REFERENCES inbox_items(id) ON DELETE SET NULL,
 content TEXT NOT NULL,
 occurred_at INTEGER NOT NULL,
 created_at INTEGER NOT NULL
);
CREATE INDEX kol_note_expert ON kol_notes(expert_id,occurred_at DESC);
CREATE INDEX kol_note_project ON kol_notes(work_id);
CREATE TABLE kol_drafts (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 expert_id INTEGER REFERENCES kol_experts(id),
 purpose TEXT NOT NULL CHECK(purpose IN ('organize','prepare','synthesize')),
 status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','confirmed','rejected')),
 payload_json TEXT NOT NULL,
 evidence_json TEXT NOT NULL,
 revision INTEGER NOT NULL DEFAULT 1,
 created_at INTEGER NOT NULL,
 decided_at INTEGER
);
CREATE TABLE kol_insights (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 draft_id INTEGER NOT NULL REFERENCES kol_drafts(id),
 expert_id INTEGER REFERENCES kol_experts(id),
 title TEXT NOT NULL,
 categories_json TEXT NOT NULL,
 observation TEXT NOT NULL,
 implication TEXT NOT NULL,
 uncertainty TEXT NOT NULL,
 next_question TEXT NOT NULL,
 citations_json TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'hypothesis' CHECK(status IN ('hypothesis','reviewed','revised','dismissed')),
 review_note TEXT NOT NULL DEFAULT '',
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
);
CREATE INDEX kol_insight_expert ON kol_insights(expert_id,status);
CREATE TABLE kol_actions (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 draft_id INTEGER NOT NULL REFERENCES kol_drafts(id),
 expert_id INTEGER REFERENCES kol_experts(id),
 entity_kind TEXT NOT NULL CHECK(entity_kind IN ('task','waiting','calendar','inbox')),
 entity_id INTEGER NOT NULL,
 created_at INTEGER NOT NULL,
 UNIQUE(entity_kind,entity_id)
);
DROP INDEX idx_ai_task_routes_model;
ALTER TABLE ai_task_routes RENAME TO ai_task_routes_v13;
CREATE TABLE ai_task_routes (
 task_kind TEXT PRIMARY KEY CHECK(task_kind IN ('workspace_analysis','work_draft','global_analysis','daily_brief','weekly_report','monthly_report','translation','general','workbench_qa','kol_analysis')),
 provider_model_id INTEGER REFERENCES provider_models(id) ON DELETE SET NULL,
 updated_at INTEGER NOT NULL
);
INSERT INTO ai_task_routes SELECT * FROM ai_task_routes_v13;
DROP TABLE ai_task_routes_v13;
CREATE INDEX idx_ai_task_routes_model ON ai_task_routes(provider_model_id);
