-- 用户显式委托固定工具集与画布基线；旧任务不补造新授权。
-- 选片任务保留旧 100 张目标约束；画布任务使用独立目标，不重建审计链上的父表。
ALTER TABLE agent_tasks ADD COLUMN canvas_target_count INTEGER
    CHECK(canvas_target_count IS NULL OR (canvas_target_count BETWEEN 1 AND 500 AND target_count=MIN(canvas_target_count,100)));
CREATE TRIGGER task_target_no_update BEFORE UPDATE OF target_count,canvas_target_count ON agent_tasks
WHEN NEW.target_count IS NOT OLD.target_count OR NEW.canvas_target_count IS NOT OLD.canvas_target_count
BEGIN SELECT RAISE(ABORT,'task target is immutable'); END;
CREATE TABLE agent_task_grants (
    task_id TEXT PRIMARY KEY REFERENCES agent_tasks(id),
    project_id TEXT NOT NULL REFERENCES projects(id),
    grant_json TEXT NOT NULL,
    source_snapshot_json TEXT,
    created_at TEXT NOT NULL
);
CREATE TRIGGER agent_grants_no_update BEFORE UPDATE ON agent_task_grants
BEGIN SELECT RAISE(ABORT,'task grant is immutable'); END;
CREATE TRIGGER agent_grants_no_delete BEFORE DELETE ON agent_task_grants
BEGIN SELECT RAISE(ABORT,'task grant is immutable'); END;

CREATE TABLE board_draft_tasks (
    board_id TEXT PRIMARY KEY REFERENCES boards(id),
    task_id TEXT NOT NULL REFERENCES agent_tasks(id),
    created_at TEXT NOT NULL
);
-- 仅保留旧库中确实存在的任务归属，不补造任何任务授权。
INSERT INTO board_draft_tasks(board_id,task_id,created_at)
    SELECT id,owning_task_id,created_at FROM boards WHERE owning_task_id IS NOT NULL;
CREATE TRIGGER draft_tasks_no_update BEFORE UPDATE ON board_draft_tasks
BEGIN SELECT RAISE(ABORT,'draft task origin is immutable'); END;
CREATE TRIGGER draft_tasks_no_delete BEFORE DELETE ON board_draft_tasks
BEGIN SELECT RAISE(ABORT,'draft task origin is immutable'); END;

CREATE TABLE board_draft_origins (
    board_id TEXT PRIMARY KEY REFERENCES boards(id),
    task_id TEXT NOT NULL REFERENCES agent_tasks(id),
    source_board_id TEXT NOT NULL REFERENCES boards(id),
    source_revision INTEGER NOT NULL,
    snapshot_sha256 TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE board_draft_node_origins (
    node_id TEXT PRIMARY KEY REFERENCES canvas_nodes(id),
    board_id TEXT NOT NULL REFERENCES board_draft_origins(board_id),
    source_node_id TEXT NOT NULL,
    base_json TEXT NOT NULL,
    UNIQUE(board_id,source_node_id)
);
CREATE TRIGGER draft_origins_no_update BEFORE UPDATE ON board_draft_origins
BEGIN SELECT RAISE(ABORT,'draft origin is immutable'); END;
CREATE TRIGGER draft_origins_no_delete BEFORE DELETE ON board_draft_origins
BEGIN SELECT RAISE(ABORT,'draft origin is immutable'); END;
CREATE TRIGGER draft_nodes_no_update BEFORE UPDATE ON board_draft_node_origins
BEGIN SELECT RAISE(ABORT,'draft node origin is immutable'); END;
CREATE TRIGGER draft_nodes_no_delete BEFORE DELETE ON board_draft_node_origins
BEGIN SELECT RAISE(ABORT,'draft node origin is immutable'); END;
