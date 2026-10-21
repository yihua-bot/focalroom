-- H0-03：任务专属草案。boards 增加 owning_task_id 列。
-- NULL = 人工创建的画布；非 NULL = 该 Agent 任务专属的草案沙盒。
-- Agent 工具只能写入 owning_task_id 等于当前任务 ID 的画布。
-- 人工采纳后清空 owning_task_id（画布变为普通 draft，进入人工编辑域）。

ALTER TABLE boards ADD COLUMN owning_task_id TEXT REFERENCES agent_tasks(id);
CREATE INDEX IF NOT EXISTS idx_boards_owning_task ON boards(owning_task_id) WHERE owning_task_id IS NOT NULL;
