-- 兼容 0001 的 text 身份与项目主键；不改写未登记的 0003–0005 草案。
CREATE TABLE IF NOT EXISTS fr_agent_task_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id TEXT NOT NULL REFERENCES fr_projects(id) ON DELETE CASCADE,
    task_type TEXT NOT NULL DEFAULT 'summarize_comments',
    created_by TEXT NOT NULL REFERENCES fr_users(id),
    model_id TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    comment_count INTEGER NOT NULL DEFAULT 0,
    tokens_in INTEGER NOT NULL DEFAULT 0,
    tokens_out INTEGER NOT NULL DEFAULT 0,
    cost_estimate_minor_usd INTEGER NOT NULL DEFAULT 0,
    provenance JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_agent_task_results_project
    ON fr_agent_task_results(project_id, created_at DESC);
CREATE TABLE IF NOT EXISTS fr_password_reset_tokens (
    digest TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES fr_users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ip TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_password_reset_user
    ON fr_password_reset_tokens(user_id, created_at DESC);
CREATE TABLE IF NOT EXISTS fr_email_verify_tokens (
    digest TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES fr_users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE fr_users ADD COLUMN IF NOT EXISTS email_verified_at TIMESTAMPTZ;
