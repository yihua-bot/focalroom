-- SetNodeText 需要可撤销的历史语义：改写正文既不是 add_node 也不是 resize，必须单列一种 op_type。
-- CHECK 约束无法 ALTER，沿用 0028/0029/0032 的建新表-搬数据-换名做法，保留 rowid 顺序与既有撤销栈。
CREATE TABLE canvas_op_log_v36 (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    op_type TEXT NOT NULL CHECK(op_type IN ('move','add_node','delete_node','resize','set_lock','add_annotation','resolve_annotation','editorial_merge','set_text')),
    op_data TEXT NOT NULL,
    undo_data TEXT NOT NULL,
    undone INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    redone INTEGER NOT NULL DEFAULT 0,
    command_group TEXT
);
INSERT INTO canvas_op_log_v36(rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,created_at,redone,command_group)
    SELECT rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,created_at,redone,command_group
    FROM canvas_op_log ORDER BY rowid;
DROP TABLE canvas_op_log;
ALTER TABLE canvas_op_log_v36 RENAME TO canvas_op_log;
CREATE INDEX idx_canvas_op_log_board ON canvas_op_log(board_id,undone,created_at DESC);
CREATE INDEX idx_canvas_op_log_group ON canvas_op_log(command_group);
