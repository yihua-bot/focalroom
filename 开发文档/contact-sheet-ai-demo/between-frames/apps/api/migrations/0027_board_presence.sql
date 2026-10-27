-- G6 正式云端共编（剩余清单 #7 第三批）：画布在线协作 Presence。
-- 心跳是可提案者（编辑/评审）的会话状态广播，观察者只读；
-- 30 秒失活窗口与实时通道一致，心跳时惰性回收过期行。
-- Presence 不属于画布内容：不推进修订、不写事件日志。

CREATE TABLE IF NOT EXISTS fr_board_presence (
    board_id UUID NOT NULL REFERENCES fr_boards(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES fr_users(id),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    selection JSONB,
    PRIMARY KEY (board_id, user_id)
);

DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT SELECT, INSERT, UPDATE, DELETE ON fr_board_presence TO focalroom_app;
  END IF;
END
$$;
