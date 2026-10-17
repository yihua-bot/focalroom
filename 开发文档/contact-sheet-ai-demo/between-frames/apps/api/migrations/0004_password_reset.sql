-- M4 密码找回：重置令牌（30 分钟有效、一次性使用）。
CREATE TABLE IF NOT EXISTS fr_password_reset_tokens (
    digest TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES fr_users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ip TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_password_reset_user
    ON fr_password_reset_tokens(user_id, created_at DESC);
