-- 智能编辑采纳使用独立语义的命令组，保留既有撤销顺序、状态及历史分组。
-- 不修改不可变的 canvas_command_events。
CREATE TABLE canvas_op_log_v32 (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    op_type TEXT NOT NULL CHECK(op_type IN ('move','add_node','delete_node','resize','set_lock','add_annotation','resolve_annotation','editorial_merge')),
    op_data TEXT NOT NULL,
    undo_data TEXT NOT NULL,
    undone INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    redone INTEGER NOT NULL DEFAULT 0,
    command_group TEXT
);
INSERT INTO canvas_op_log_v32(rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,created_at,redone,command_group)
    SELECT rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,created_at,redone,command_group
    FROM canvas_op_log ORDER BY rowid;
DROP TABLE canvas_op_log;
ALTER TABLE canvas_op_log_v32 RENAME TO canvas_op_log;
CREATE INDEX idx_canvas_op_log_board ON canvas_op_log(board_id,undone,created_at DESC);
CREATE INDEX idx_canvas_op_log_group ON canvas_op_log(command_group);
