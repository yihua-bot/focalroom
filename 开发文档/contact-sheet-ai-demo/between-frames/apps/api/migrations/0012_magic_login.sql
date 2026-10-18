-- 保留未登记的 0009_magic_login.sql 草案；新版本明确登记兼容结构。
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
ALTER TABLE fr_magic_login_codes ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 5);
-- 新表由迁移角色拥有；增量升级也必须授予现有应用角色，而非修改旧迁移。
DO $$ BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT SELECT, INSERT, UPDATE, DELETE ON fr_magic_login_codes TO focalroom_app;
  END IF;
END $$;
