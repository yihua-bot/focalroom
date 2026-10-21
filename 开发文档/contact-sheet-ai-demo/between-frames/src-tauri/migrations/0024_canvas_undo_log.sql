-- H0-07：画布操作撤销日志（append-only，不修改 command_events）。
-- canvas_op_log 记录每次可撤销操作的逆操作所需数据。
-- undo_canvas 从此表读取并逆序执行，不触碰审计事件。

CREATE TABLE IF NOT EXISTS canvas_op_log (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    -- move / add_node / delete_node
    op_type TEXT NOT NULL CHECK(op_type IN ('move','add_node','delete_node')),
    -- 操作数据（JSON：node_ids/dx/dy/node_id 等，足以执行逆操作）
    op_data TEXT NOT NULL,
    -- 逆操作数据（JSON：预计算的逆操作参数）
    undo_data TEXT NOT NULL,
    -- 是否已被撤销（撤销后标记，防重复撤销）
    undone INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_canvas_op_log_board
    ON canvas_op_log(board_id, undone, created_at DESC);
