-- M4 云 Agent Worker：独立模型发送授权 + 可恢复任务队列。
-- 授权显式记录端点/型号/额度；项目与管理员权限不构成模型发送同意。
-- 任务带租约与检查点：Worker 崩溃后按租约过期重新领取，幂等完成。
CREATE TABLE fr_model_consents (
  id text PRIMARY KEY,
  project_id text NOT NULL REFERENCES fr_projects(id),
  granted_by text NOT NULL REFERENCES fr_users(id),
  endpoint_label text NOT NULL,
  model_id text NOT NULL,
  scope text NOT NULL CHECK(scope IN ('summarize_comments','summarize_disagreement')),
  max_tasks integer NOT NULL CHECK(max_tasks BETWEEN 1 AND 100),
  used integer NOT NULL DEFAULT 0 CHECK(used >= 0),
  state text NOT NULL DEFAULT 'ACTIVE' CHECK(state IN ('ACTIVE','REVOKED','EXHAUSTED')),
  granted_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz
);
CREATE INDEX consent_effective ON fr_model_consents(project_id, state, id);
CREATE TABLE fr_agent_jobs (
  id text PRIMARY KEY,
  project_id text NOT NULL REFERENCES fr_projects(id),
  consent_id text NOT NULL REFERENCES fr_model_consents(id),
  created_by text NOT NULL REFERENCES fr_users(id),
  kind text NOT NULL CHECK(kind IN ('summarize_comments','summarize_disagreement')),
  status text NOT NULL DEFAULT 'QUEUED' CHECK(status IN ('QUEUED','RUNNING','COMPLETED','FAILED','CANCELLED')),
  attempts integer NOT NULL DEFAULT 0 CHECK(attempts >= 0),
  checkpoint jsonb,
  result_id text,
  error text,
  lease_expires timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX job_dispatch ON fr_agent_jobs(status, created_at);
