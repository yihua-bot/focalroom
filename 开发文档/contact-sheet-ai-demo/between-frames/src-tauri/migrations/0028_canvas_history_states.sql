-- 扩展画布私有撤销栈；保留旧记录、rowid 顺序与 redo 状态。
-- command_events 不变。SQLite CHECK 变更需要事务内重建本表。
CREATE TABLE canvas_op_log_next (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    op_type TEXT NOT NULL CHECK(op_type IN ('move','add_node','delete_node','resize','set_lock')),
    op_data TEXT NOT NULL,
    undo_data TEXT NOT NULL,
    undone INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    redone INTEGER NOT NULL DEFAULT 0
);
INSERT INTO canvas_op_log_next(rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,created_at,redone)
    SELECT rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,created_at,redone
    FROM canvas_op_log ORDER BY rowid;
DROP TABLE canvas_op_log;
ALTER TABLE canvas_op_log_next RENAME TO canvas_op_log;
CREATE INDEX idx_canvas_op_log_board ON canvas_op_log(board_id,undone,created_at DESC);
