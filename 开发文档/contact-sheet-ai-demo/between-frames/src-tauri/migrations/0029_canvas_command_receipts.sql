-- 正式画布命令与回执追加保存；私有撤销栈可更新，不冒充不可变审计。
CREATE TABLE canvas_command_events (
    command_id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(id),
    board_id TEXT NOT NULL REFERENCES boards(id),
    actor TEXT NOT NULL CHECK(actor='human'),
    base_revision INTEGER NOT NULL CHECK(base_revision>=0),
    revision INTEGER NOT NULL CHECK(revision>=base_revision),
    request_json TEXT NOT NULL,
    receipt_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_canvas_commands_board ON canvas_command_events(board_id,revision);
CREATE TRIGGER canvas_commands_no_update BEFORE UPDATE ON canvas_command_events
BEGIN SELECT RAISE(ABORT,'canvas audit is immutable'); END;
CREATE TRIGGER canvas_commands_no_delete BEFORE DELETE ON canvas_command_events
BEGIN SELECT RAISE(ABORT,'canvas audit is immutable'); END;

CREATE TABLE canvas_operation_ids (
    op_id TEXT PRIMARY KEY,
    command_id TEXT NOT NULL REFERENCES canvas_command_events(command_id)
);
CREATE TRIGGER canvas_ops_no_update BEFORE UPDATE ON canvas_operation_ids
BEGIN SELECT RAISE(ABORT,'canvas operation identity is immutable'); END;
CREATE TRIGGER canvas_ops_no_delete BEFORE DELETE ON canvas_operation_ids
BEGIN SELECT RAISE(ABORT,'canvas operation identity is immutable'); END;

ALTER TABLE board_annotations ADD COLUMN visible INTEGER NOT NULL DEFAULT 1 CHECK(visible IN (0,1));
CREATE TABLE canvas_op_log_v29 (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    op_type TEXT NOT NULL CHECK(op_type IN ('move','add_node','delete_node','resize','set_lock','add_annotation','resolve_annotation')),
    op_data TEXT NOT NULL,
    undo_data TEXT NOT NULL,
    undone INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    redone INTEGER NOT NULL DEFAULT 0,
    command_group TEXT
);
INSERT INTO canvas_op_log_v29(rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,created_at,redone)
    SELECT rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,created_at,redone FROM canvas_op_log ORDER BY rowid;
DROP TABLE canvas_op_log;
ALTER TABLE canvas_op_log_v29 RENAME TO canvas_op_log;
CREATE INDEX idx_canvas_op_log_board ON canvas_op_log(board_id,undone,created_at DESC);
CREATE INDEX idx_canvas_op_log_group ON canvas_op_log(command_group);
