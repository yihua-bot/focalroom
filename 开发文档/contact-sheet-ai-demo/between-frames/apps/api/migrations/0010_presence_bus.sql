-- M5 跨实例 Presence 总线：在线状态经 PostgreSQL 共享，多实例 API 互相可见。
-- 心跳由各实例周期 UPSERT；读取只信任新鲜行，断开即删，实例崩溃靠短过期兜底。
CREATE TABLE fr_presence (
  project_id text NOT NULL REFERENCES fr_projects(id),
  instance_id text NOT NULL,
  user_id text NOT NULL,
  user_name text NOT NULL,
  photo_id text,
  seen_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY(project_id, instance_id, user_id)
);
CREATE INDEX presence_live ON fr_presence(seen_at);
