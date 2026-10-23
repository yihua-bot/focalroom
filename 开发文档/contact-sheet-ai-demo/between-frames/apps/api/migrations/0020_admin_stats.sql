-- 管理员角色 + 产品统计：
-- fr_users.role：user（默认）/ admin（个人中心显示统计、免验证）
-- fr_download_events：下载计数（官网每次下载 ping 一次）
ALTER TABLE fr_users ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'user';

CREATE TABLE IF NOT EXISTS fr_download_events (
    id BIGSERIAL PRIMARY KEY,
    kind TEXT NOT NULL CHECK(kind IN ('dmg','updater')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_download_events ON fr_download_events(kind, created_at);
