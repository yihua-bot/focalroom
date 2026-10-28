-- 保留旧 STAGING/SHARED/ABORTED 权威列；新表明确产品同步模式，旧项目不回填推断私有。
CREATE TABLE fr_project_sync (
  project_id TEXT PRIMARY KEY REFERENCES fr_projects(id),
  mode TEXT NOT NULL CHECK(mode IN ('SYNC_ENROLLING','PRIVATE_SYNCED','SHARED','ENROLL_FAILED')),
  version INTEGER NOT NULL DEFAULT 0 CHECK(version >= 0),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE fr_sync_commands (
  project_id TEXT NOT NULL REFERENCES fr_projects(id),
  command_id UUID NOT NULL,
  actor_id TEXT NOT NULL REFERENCES fr_users(id),
  payload_digest TEXT NOT NULL,
  reply JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY(project_id,command_id)
);
DO $$ BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname='focalroom_app') THEN
    GRANT SELECT,INSERT,UPDATE ON fr_project_sync TO focalroom_app;
    GRANT SELECT,INSERT ON fr_sync_commands TO focalroom_app;
  END IF;
END $$;
