-- H0-07：画布乐观并发控制。boards 增加 revision 列。
-- 每次节点写入（place/move/delete/group/ink/arrow/sticky）递增 revision。
-- 客户端提交时携带 expected_revision，不匹配则冲突拒绝。

ALTER TABLE boards ADD COLUMN revision INTEGER NOT NULL DEFAULT 1;

-- 写入节点时自动递增画布 revision 的触发器
CREATE TRIGGER IF NOT EXISTS board_revision_bump
AFTER INSERT ON canvas_nodes
BEGIN
  UPDATE boards SET revision = revision + 1, updated_at = datetime('now') WHERE id = NEW.board_id;
END;

CREATE TRIGGER IF NOT EXISTS board_revision_bump_update
AFTER UPDATE OF x, y, width, height, visible, locked ON canvas_nodes
BEGIN
  UPDATE boards SET revision = revision + 1, updated_at = datetime('now') WHERE id = NEW.board_id;
END;
