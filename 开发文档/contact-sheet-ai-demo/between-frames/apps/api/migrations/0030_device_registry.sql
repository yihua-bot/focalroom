-- G2 设备注册与撤销：fr_devices + 会话绑定。撤销设备经外键级联使其会话一并失效。
-- 会话绑定走独立授权端点（UPDATE fr_sessions.device_id），不改既有 createSession 写入路径。

CREATE TABLE IF NOT EXISTS fr_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL REFERENCES fr_users(id) ON DELETE CASCADE,
    device_key TEXT NOT NULL,
    name TEXT NOT NULL,
    platform TEXT NOT NULL CHECK(platform IN ('ios','mac','web','desktop')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, device_key)
);
CREATE INDEX IF NOT EXISTS idx_fr_devices_user ON fr_devices(user_id);

ALTER TABLE fr_sessions ADD COLUMN device_id UUID REFERENCES fr_devices(id) ON DELETE CASCADE;

DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT SELECT, INSERT, UPDATE, DELETE ON fr_devices TO focalroom_app;
    GRANT UPDATE (device_id) ON fr_sessions TO focalroom_app;
  END IF;
END
$$;
