-- Durable links between accepted advice and the actual item created/updated.
-- Kept separate so older confirmation receipts remain readable and undoable.
CREATE TABLE ai_proposal_outcomes (
    proposal_id INTEGER PRIMARY KEY REFERENCES ai_proposals(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    target_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_proposal_outcomes_target ON ai_proposal_outcomes(kind,target_id);

CREATE TRIGGER proposal_task_resolved AFTER UPDATE OF status ON tasks
WHEN NEW.status='done' AND OLD.status<>'done'
BEGIN
    UPDATE ai_proposals SET status='resolved',decided_at=unixepoch(),updated_at=MAX(updated_at+1,unixepoch())
    WHERE status='confirmed' AND id IN (SELECT proposal_id FROM ai_proposal_outcomes WHERE kind='task' AND target_id=NEW.id);
END;
CREATE TRIGGER proposal_waiting_resolved AFTER UPDATE OF status ON waiting_items
WHEN NEW.status='resolved' AND OLD.status<>'resolved'
BEGIN
    UPDATE ai_proposals SET status='resolved',decided_at=unixepoch(),updated_at=MAX(updated_at+1,unixepoch())
    WHERE status='confirmed' AND id IN (SELECT proposal_id FROM ai_proposal_outcomes WHERE kind='waiting' AND target_id=NEW.id);
END;
CREATE TRIGGER proposal_outcome_already_done AFTER INSERT ON ai_proposal_outcomes
WHEN (NEW.kind='task' AND EXISTS(SELECT 1 FROM tasks WHERE id=NEW.target_id AND status='done'))
  OR (NEW.kind='waiting' AND EXISTS(SELECT 1 FROM waiting_items WHERE id=NEW.target_id AND status='resolved'))
BEGIN
    UPDATE ai_proposals SET status='resolved',decided_at=unixepoch(),updated_at=MAX(updated_at+1,unixepoch())
    WHERE id=NEW.proposal_id AND status='confirmed';
END;

-- Only explicit historical confirmation events establish legacy identity.
-- Never guess a target from a matching title or name.
INSERT OR IGNORE INTO ai_proposal_outcomes(proposal_id,kind,target_id,created_at)
SELECT p.id,json_extract(e.metadata_json,'$.kind'),json_extract(e.metadata_json,'$.target_id'),e.timestamp
FROM ai_proposals p JOIN activity_events e ON e.entity_id=p.id AND e.entity_type='proposal'
WHERE p.status='confirmed' AND e.event_type='ai.proposal.confirmed'
  AND json_valid(e.metadata_json)
  AND json_extract(e.metadata_json,'$.kind')=p.kind
  AND json_type(e.metadata_json,'$.target_id')='integer'
  AND json_extract(e.metadata_json,'$.target_id')>0
ORDER BY e.id DESC;
