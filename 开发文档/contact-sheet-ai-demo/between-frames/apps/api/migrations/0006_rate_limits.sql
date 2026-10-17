-- M4 集中限流：PostgreSQL 后端（替代进程内存，支持多实例部署）。
CREATE TABLE IF NOT EXISTS fr_rate_limits (
    key TEXT NOT NULL,
    window_start TIMESTAMPTZ NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (key, window_start)
);
CREATE INDEX IF NOT EXISTS idx_rate_limits_expiry
    ON fr_rate_limits(window_start);
