-- PostgreSQL 迁移起点：业务授权校验必须存在于服务层；RLS 用作第二防线。
-- UUID 采用应用生成的 TEXT 标识示例，生产可统一改 UUID 类型，但须由 ADR + 迁移处理。
CREATE TABLE IF NOT EXISTS creators (
 id TEXT PRIMARY KEY, display_name TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS workspaces (
 id TEXT PRIMARY KEY, name TEXT NOT NULL, owner_creator_id TEXT NOT NULL REFERENCES creators(id),
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS memberships (
 workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
 creator_id TEXT NOT NULL REFERENCES creators(id) ON DELETE CASCADE,
 role TEXT NOT NULL CHECK(role IN ('OWNER','ADMIN','EDITOR','REVIEWER','VIEWER')),
 revoked_at TIMESTAMPTZ,
 PRIMARY KEY(workspace_id,creator_id)
);
CREATE TABLE IF NOT EXISTS projects (
 id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
 owner_creator_id TEXT NOT NULL REFERENCES creators(id),
 title TEXT NOT NULL, goal TEXT NOT NULL DEFAULT '',
 revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS project_grants (
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 creator_id TEXT NOT NULL REFERENCES creators(id) ON DELETE CASCADE,
 grant_role TEXT NOT NULL CHECK(grant_role IN ('EDITOR','REVIEWER','VIEWER')),
 can_approve BOOLEAN NOT NULL DEFAULT FALSE,
 expires_at TIMESTAMPTZ,
 revoked_at TIMESTAMPTZ,
 PRIMARY KEY(project_id,creator_id)
);
CREATE TABLE IF NOT EXISTS assets (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 object_key TEXT NOT NULL, preview_object_key TEXT, content_hash TEXT NOT NULL,
 original_uploaded BOOLEAN NOT NULL DEFAULT FALSE,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
 UNIQUE(project_id,content_hash), UNIQUE(id,project_id)
);
CREATE TABLE IF NOT EXISTS photos (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 asset_id TEXT NOT NULL,
 selection_state TEXT NOT NULL DEFAULT 'UNDECIDED' CHECK(selection_state IN ('UNDECIDED','KEEP','REJECT')),
 FOREIGN KEY (asset_id, project_id) REFERENCES assets(id,project_id)
);
CREATE TABLE IF NOT EXISTS versions (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 revision BIGINT NOT NULL, snapshot_json JSONB NOT NULL,
 actor_type TEXT NOT NULL CHECK(actor_type IN ('USER','AGENT','SYSTEM')),
 actor_id TEXT NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
 UNIQUE(project_id,revision)
);
CREATE TABLE IF NOT EXISTS edit_proposals (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 base_revision BIGINT NOT NULL,
 author_type TEXT NOT NULL CHECK(author_type IN ('USER','AGENT')),
 author_id TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'DRAFT' CHECK(status IN ('DRAFT','SUBMITTED','ACCEPTED','PARTIALLY_ACCEPTED','REJECTED','OBSOLETE')),
 operations JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS comment_threads (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 anchor JSONB NOT NULL, status TEXT NOT NULL DEFAULT 'OPEN' CHECK(status IN ('OPEN','RESOLVED','REOPENED'))
);
CREATE TABLE IF NOT EXISTS comments (
 id TEXT PRIMARY KEY, thread_id TEXT NOT NULL REFERENCES comment_threads(id) ON DELETE CASCADE,
 author_id TEXT NOT NULL REFERENCES creators(id), body TEXT NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS agent_tasks (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 initiated_by TEXT NOT NULL REFERENCES creators(id),
 status TEXT NOT NULL CHECK(status IN ('QUEUED','RUNNING','PAUSED','AWAITING_APPROVAL','COMPLETED','FAILED','CANCELLED')),
 limits_json JSONB NOT NULL, checkpoint_json JSONB,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS tool_calls (
 id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
 name TEXT NOT NULL, request_hash TEXT NOT NULL, response_json JSONB,
 status TEXT NOT NULL CHECK(status IN ('STARTED','SUCCEEDED','FAILED','CANCELLED')),
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
 UNIQUE(task_id,id)
);
CREATE TABLE IF NOT EXISTS memory_evidence (
 id TEXT PRIMARY KEY, creator_id TEXT NOT NULL REFERENCES creators(id) ON DELETE CASCADE,
 project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
 source_event_id TEXT NOT NULL, explanation TEXT,
 deleted_at TIMESTAMPTZ,
 UNIQUE(creator_id,source_event_id)
);
CREATE TABLE IF NOT EXISTS preference_hypotheses (
 id TEXT PRIMARY KEY, creator_id TEXT NOT NULL REFERENCES creators(id) ON DELETE CASCADE,
 claim TEXT NOT NULL, scope_project_id TEXT REFERENCES projects(id),
 state TEXT NOT NULL CHECK(state IN ('UNCONFIRMED','CONFIRMED','CONTRADICTED','CORRECTED','ARCHIVED')),
 evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb, deleted_at TIMESTAMPTZ
);
CREATE TABLE IF NOT EXISTS command_receipts (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 idempotency_key TEXT NOT NULL, request_hash TEXT NOT NULL,
 response_json JSONB NOT NULL,
 UNIQUE(project_id,idempotency_key)
);
CREATE TABLE IF NOT EXISTS domain_outbox (
 event_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id),
 project_id TEXT NOT NULL REFERENCES projects(id),
 event_name TEXT NOT NULL, payload JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), published_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_projects_workspace ON projects(workspace_id);
CREATE INDEX IF NOT EXISTS idx_proposals_project_status ON edit_proposals(project_id,status);
CREATE INDEX IF NOT EXISTS idx_comments_thread ON comments(thread_id);
CREATE INDEX IF NOT EXISTS idx_agent_tasks_project ON agent_tasks(project_id,status);
-- IMPORTANT: 生产需对多租户表开启 RLS 且补齐 fail-closed policies，
-- 不应在不设置请求者上下文的情况下简单开启并上线。
