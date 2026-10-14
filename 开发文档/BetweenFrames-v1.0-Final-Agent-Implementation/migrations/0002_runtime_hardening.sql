-- BETWEEN FRAMES v1.0: Runtime hardening / opt-in privacy / checkpoints
-- Apply after 0001_init.sql using a migration runner.
-- Baseline requires `PRAGMA foreign_keys=ON` on each DB connection.

CREATE TABLE IF NOT EXISTS domain_idempotency (
    idempotency_key TEXT PRIMARY KEY,
    command_id TEXT NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    response_json TEXT NOT NULL,
    command_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_domain_idempotency_project ON domain_idempotency(project_id);

CREATE TABLE IF NOT EXISTS agent_checkpoints (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    task_state TEXT NOT NULL,
    project_revision INTEGER NOT NULL CHECK(project_revision >= 0),
    safe_context_json TEXT NOT NULL DEFAULT '{}',
    completed_call_ids_json TEXT NOT NULL DEFAULT '[]',
    remaining_budget_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(task_id,ordinal)
);
CREATE INDEX IF NOT EXISTS idx_checkpoints_task ON agent_checkpoints(task_id,ordinal DESC);

CREATE TABLE IF NOT EXISTS task_messages (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    message_kind TEXT NOT NULL CHECK(message_kind IN ('user_goal','clarification_question','clarification_answer','agent_summary','system_event')),
    safe_body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_task_messages_task ON task_messages(task_id,created_at);

CREATE TABLE IF NOT EXISTS model_usage (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    request_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    input_tokens INTEGER NOT NULL DEFAULT 0 CHECK (input_tokens >= 0),
    output_tokens INTEGER NOT NULL DEFAULT 0 CHECK (output_tokens >= 0),
    image_count INTEGER NOT NULL DEFAULT 0 CHECK (image_count >= 0),
    estimated_cost_minor_units INTEGER NOT NULL DEFAULT 0 CHECK(estimated_cost_minor_units >= 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(task_id,request_id)
);

-- project/session consent covers only visual-upload scope, not filesystem access.
CREATE TABLE IF NOT EXISTS privacy_consents (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    task_id TEXT REFERENCES agent_tasks(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL,
    scope TEXT NOT NULL CHECK(scope IN ('single_task','project')),
    status TEXT NOT NULL CHECK(status IN ('granted','revoked')),
    max_image_count INTEGER NOT NULL CHECK(max_image_count >= 0),
    strip_gps INTEGER NOT NULL DEFAULT 1 CHECK(strip_gps IN (0,1)),
    max_preview_edge INTEGER NOT NULL DEFAULT 1536 CHECK(max_preview_edge BETWEEN 128 AND 8192),
    granted_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TEXT,
    CHECK (scope != 'single_task' OR task_id IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS idx_privacy_consents_project ON privacy_consents(project_id, provider_id, status);

-- tool_calls originally has an id/call_id uniqueness pair.
-- Use an extra table for idempotent write call results without breaking old schema.
CREATE TABLE IF NOT EXISTS tool_execution_receipts (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
    call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments_sha256 TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('started','succeeded','failed','rejected')),
    result_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(task_id,call_id)
);

CREATE TABLE IF NOT EXISTS project_preferences_config (
    project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    allow_cross_project_memory INTEGER NOT NULL DEFAULT 0 CHECK(allow_cross_project_memory IN(0,1)),
    enable_proactive_suggestions INTEGER NOT NULL DEFAULT 0 CHECK(enable_proactive_suggestions IN(0,1)),
    photo_cloud_policy TEXT NOT NULL DEFAULT 'deny' CHECK(photo_cloud_policy IN('deny','ask','permit_consented')),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS memory_corrections (
    id TEXT PRIMARY KEY,
    hypothesis_id TEXT NOT NULL REFERENCES preference_hypotheses(id) ON DELETE CASCADE,
    correction_text TEXT NOT NULL,
    actor_type TEXT NOT NULL CHECK(actor_type='human'),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
