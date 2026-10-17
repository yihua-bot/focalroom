-- M4 邮箱验证：验证令牌（48 小时有效、一次性、可重发）。
CREATE TABLE IF NOT EXISTS fr_email_verify_tokens (
    digest TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES fr_users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE fr_users ADD COLUMN IF NOT EXISTS email_verified_at TIMESTAMPTZ;
