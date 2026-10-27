-- G6 正式云端共编（剩余清单 #7）：画布正式命令通道。
-- fr_boards.revision 记录每个 Board 的正式修订号；fr_canvas_commands 保存命令回执，
-- 幂等键沿用项目命令路径的双键约定（command_id 或 (project_id, actor_id, idempotency_key)）。

ALTER TABLE fr_boards ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS fr_canvas_commands (
    command_id UUID PRIMARY KEY,
    board_id UUID NOT NULL REFERENCES fr_boards(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES fr_projects(id),
    actor_id TEXT NOT NULL REFERENCES fr_users(id),
    idempotency_key TEXT NOT NULL,
    payload_digest TEXT NOT NULL,
    envelope JSONB NOT NULL,
    reply JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_fr_canvas_commands_idem
    ON fr_canvas_commands(project_id, actor_id, idempotency_key);

DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT SELECT, INSERT ON fr_canvas_commands TO focalroom_app;
  END IF;
END
$$;
