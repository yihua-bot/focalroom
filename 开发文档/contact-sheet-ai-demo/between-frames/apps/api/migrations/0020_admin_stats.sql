-- 管理员角色 + 产品统计
ALTER TABLE fr_users ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'user';
CREATE TABLE IF NOT EXISTS fr_download_events (id BIGSERIAL PRIMARY KEY, kind TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE INDEX IF NOT EXISTS idx_download_events ON fr_download_events(kind, created_at);
