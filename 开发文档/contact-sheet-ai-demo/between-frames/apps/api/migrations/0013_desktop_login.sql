-- 浏览器只领取一次性授权码；桌面持有的 PKCE verifier 不进入浏览器。
CREATE TABLE fr_desktop_logins (
    id TEXT PRIMARY KEY,
    state_hash TEXT NOT NULL,
    challenge TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    code_hash TEXT,
    code_expires_at TIMESTAMPTZ,
    browser_session TEXT REFERENCES fr_sessions(digest) ON DELETE CASCADE,
    used_at TIMESTAMPTZ
);
CREATE INDEX fr_desktop_login_expiry ON fr_desktop_logins(expires_at);
DO $$ BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname='focalroom_app') THEN
    GRANT SELECT,INSERT,UPDATE,DELETE ON fr_desktop_logins TO focalroom_app;
  END IF;
END $$;
