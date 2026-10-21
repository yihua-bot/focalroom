-- H0-07 补充：Redo 支持。canvas_op_log 增加 redone 标记。
-- undo 将 undone=1；redo 将 undone=0（重新消费）。
-- 撤销栈语义：最后一条 undone=0 的记录可撤销；其前一条 undone=1
-- 且 redone=0 的记录可重做（保持栈序）。

ALTER TABLE canvas_op_log ADD COLUMN redone INTEGER NOT NULL DEFAULT 0;
