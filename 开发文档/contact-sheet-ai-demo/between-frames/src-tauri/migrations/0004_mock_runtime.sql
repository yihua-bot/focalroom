
CREATE TABLE agent_tasks (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id),
 goal TEXT NOT NULL, base_revision INTEGER NOT NULL, allowed_ids_json TEXT NOT NULL,
 selected_ids_json TEXT NOT NULL, target_count INTEGER NOT NULL CHECK(target_count BETWEEN 1 AND 100),
 state TEXT NOT NULL CHECK(state IN('queued','planning','running','needs_input','review_ready','completed','paused','stale','failed','cancelled','interrupted')),
 reason TEXT NOT NULL, budget_json TEXT NOT NULL, usage_json TEXT NOT NULL,
 elapsed_ms INTEGER NOT NULL DEFAULT 0, question TEXT, answer TEXT,
 final_ids_json TEXT NOT NULL DEFAULT '[]', pending_json TEXT,
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE INDEX agent_tasks_project ON agent_tasks(project_id,created_at);
CREATE UNIQUE INDEX one_executing_task ON agent_tasks(project_id) WHERE state IN('queued','planning','running');
CREATE TABLE agent_steps (
 id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL REFERENCES agent_tasks(id),
 kind TEXT NOT NULL, body_json TEXT NOT NULL, summary TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TRIGGER steps_no_update BEFORE UPDATE ON agent_steps BEGIN SELECT RAISE(ABORT,'steps are append-only'); END;
CREATE TRIGGER steps_no_delete BEFORE DELETE ON agent_steps BEGIN SELECT RAISE(ABORT,'steps are append-only'); END;
CREATE TABLE tool_calls (
 task_id TEXT NOT NULL REFERENCES agent_tasks(id), call_id TEXT NOT NULL,
 tool_name TEXT NOT NULL, fingerprint TEXT NOT NULL, result_json TEXT NOT NULL,
 PRIMARY KEY(task_id,call_id)
);
CREATE TRIGGER tool_calls_no_update BEFORE UPDATE ON tool_calls BEGIN SELECT RAISE(ABORT,'tool receipts are immutable'); END;
CREATE TRIGGER tool_calls_no_delete BEFORE DELETE ON tool_calls BEGIN SELECT RAISE(ABORT,'tool receipts are immutable'); END;
CREATE TABLE agent_checkpoints (
 id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL REFERENCES agent_tasks(id),
 state TEXT NOT NULL, context_json TEXT NOT NULL, completed_calls_json TEXT NOT NULL,
 budget_json TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE model_usage (
 task_id TEXT NOT NULL REFERENCES agent_tasks(id), request_id TEXT NOT NULL,
 input_estimate INTEGER NOT NULL, output_estimate INTEGER NOT NULL,
 cloud_images INTEGER NOT NULL CHECK(cloud_images=0), cost_minor_usd INTEGER NOT NULL CHECK(cost_minor_usd=0),
 PRIMARY KEY(task_id,request_id)
);
CREATE TABLE task_messages (
 id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL REFERENCES agent_tasks(id),
 kind TEXT NOT NULL, body TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE drafts (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id),
 task_id TEXT NOT NULL REFERENCES agent_tasks(id), title TEXT NOT NULL,
 base_revision INTEGER NOT NULL, revision INTEGER NOT NULL DEFAULT 1,
 state TEXT NOT NULL CHECK(state IN('pending','applied','discarded')),
 summary TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
 FOREIGN KEY(task_id) REFERENCES agent_tasks(id)
);
CREATE TABLE draft_items (
 draft_id TEXT NOT NULL REFERENCES drafts(id), photo_id TEXT NOT NULL REFERENCES photos(id),
 proposed_action TEXT NOT NULL CHECK(proposed_action IN('keep','reject','undecided')),
 position INTEGER NOT NULL CHECK(position>=0), rationale TEXT NOT NULL,
 PRIMARY KEY(draft_id,photo_id), UNIQUE(draft_id,position)
);
CREATE TRIGGER draft_owner BEFORE INSERT ON drafts WHEN NOT EXISTS(SELECT 1 FROM agent_tasks WHERE id=NEW.task_id AND project_id=NEW.project_id) BEGIN SELECT RAISE(ABORT,'draft task project mismatch'); END;
CREATE TRIGGER draft_item_owner BEFORE INSERT ON draft_items WHEN NOT EXISTS(SELECT 1 FROM photos p JOIN drafts d ON d.project_id=p.project_id WHERE p.id=NEW.photo_id AND d.id=NEW.draft_id) BEGIN SELECT RAISE(ABORT,'draft photo project mismatch'); END;
CREATE TRIGGER draft_owner_immutable BEFORE UPDATE OF project_id,task_id ON drafts BEGIN SELECT RAISE(ABORT,'draft owner immutable'); END;
CREATE TRIGGER draft_item_owner_update BEFORE UPDATE OF photo_id,draft_id ON draft_items BEGIN SELECT RAISE(ABORT,'draft item owner immutable'); END;
CREATE TABLE agent_request_receipts (
 request_id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id),
 fingerprint TEXT NOT NULL, result_json TEXT NOT NULL
);
CREATE TRIGGER agent_receipt_no_update BEFORE UPDATE ON agent_request_receipts BEGIN SELECT RAISE(ABORT,'agent receipts immutable'); END;
CREATE TRIGGER agent_receipt_no_delete BEFORE DELETE ON agent_request_receipts BEGIN SELECT RAISE(ABORT,'agent receipts immutable'); END;
