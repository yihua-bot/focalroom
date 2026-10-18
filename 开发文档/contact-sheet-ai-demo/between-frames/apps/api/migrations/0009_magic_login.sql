-- M4 魔法验证码登录：6 位数字验证码（10 分钟有效、一次性、可重发）。
CREATE TABLE IF NOT EXISTS fr_magic_login_codes (
    digest TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    code_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ip TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_magic_login_email ON fr_magic_login_codes(email, created_at DESC);
