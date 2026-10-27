-- G6 正式云端共编（剩余清单 #7 第二批）：画布事件日志与游标。
-- 每条成功命令追加一行事件；客户端断线重连后按 after=<lastSeen> 拉取增量，
-- 是离线重放与多端同步的地基。事件只追加，不修改、不删除。

CREATE TABLE IF NOT EXISTS fr_canvas_events (
    id BIGSERIAL PRIMARY KEY,
    board_id UUID NOT NULL REFERENCES fr_boards(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES fr_projects(id),
    actor_id TEXT NOT NULL REFERENCES fr_users(id),
    revision INTEGER NOT NULL,
    command_id UUID NOT NULL,
    kind TEXT NOT NULL DEFAULT 'canvas.command',
    detail JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_fr_canvas_events_board
    ON fr_canvas_events(board_id, id);

DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT SELECT, INSERT ON fr_canvas_events TO focalroom_app;
  END IF;
END
$$;
