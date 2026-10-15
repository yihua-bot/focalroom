PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS local_projects (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  mode TEXT NOT NULL DEFAULT 'LOCAL' CHECK (mode IN ('LOCAL', 'SHARED_CACHE', 'PROMOTING')),
  cloud_project_id TEXT,
  goal TEXT NOT NULL DEFAULT '',
  current_revision INTEGER NOT NULL DEFAULT 0 CHECK(current_revision >= 0),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS local_photos (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES local_projects(id) ON DELETE CASCADE,
  source_path TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  preview_path TEXT,
  selection_state TEXT NOT NULL DEFAULT 'UNDECIDED' CHECK(selection_state IN ('UNDECIDED','KEEP','REJECT')),
  imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(project_id, content_hash)
);
CREATE TABLE IF NOT EXISTS local_sequences (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES local_projects(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0)
);
CREATE TABLE IF NOT EXISTS local_sequence_items (
  sequence_id TEXT NOT NULL REFERENCES local_sequences(id) ON DELETE CASCADE,
  photo_id TEXT NOT NULL REFERENCES local_photos(id) ON DELETE CASCADE,
  position INTEGER NOT NULL CHECK(position >= 0),
  PRIMARY KEY(sequence_id, photo_id), UNIQUE(sequence_id, position)
);
CREATE TABLE IF NOT EXISTS local_versions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES local_projects(id) ON DELETE CASCADE,
  revision INTEGER NOT NULL,
  snapshot_json TEXT NOT NULL,
  committed_by TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(project_id, revision)
);
CREATE TABLE IF NOT EXISTS local_proposals (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES local_projects(id) ON DELETE CASCADE,
  base_revision INTEGER NOT NULL,
  status TEXT NOT NULL DEFAULT 'DRAFT' CHECK(status IN ('DRAFT','SUBMITTED','ACCEPTED','PARTIALLY_ACCEPTED','REJECTED','OBSOLETE')),
  operations_json TEXT NOT NULL,
  actor_type TEXT NOT NULL CHECK(actor_type IN ('USER','AGENT')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS local_agent_tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES local_projects(id) ON DELETE CASCADE,
  status TEXT NOT NULL CHECK(status IN ('QUEUED','RUNNING','PAUSED','AWAITING_APPROVAL','COMPLETED','FAILED','CANCELLED')),
  checkpoint_json TEXT,
  max_cost_microusd INTEGER NOT NULL DEFAULT 1000000,
  incurred_cost_microusd INTEGER NOT NULL DEFAULT 0,
  cancelled_at TEXT
);
CREATE TABLE IF NOT EXISTS local_memory_evidence (
  id TEXT PRIMARY KEY,
  creator_id TEXT NOT NULL,
  project_id TEXT NOT NULL REFERENCES local_projects(id) ON DELETE CASCADE,
  evidence_type TEXT NOT NULL,
  event_id TEXT NOT NULL,
  text_reason TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(creator_id, event_id)
);
CREATE TABLE IF NOT EXISTS local_preference_hypotheses (
  id TEXT PRIMARY KEY,
  creator_id TEXT NOT NULL,
  project_id TEXT REFERENCES local_projects(id) ON DELETE CASCADE,
  claim TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'UNCONFIRMED' CHECK(state IN ('UNCONFIRMED','CONFIRMED','CONTRADICTED','CORRECTED','ARCHIVED')),
  evidence_ids_json TEXT NOT NULL DEFAULT '[]',
  deleted_at TEXT
);
CREATE TABLE IF NOT EXISTS local_command_log (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES local_projects(id) ON DELETE CASCADE,
  idempotency_key TEXT NOT NULL,
  command_kind TEXT NOT NULL,
  command_hash TEXT NOT NULL,
  result_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(project_id, idempotency_key)
);
CREATE INDEX IF NOT EXISTS idx_local_photos_project_state ON local_photos(project_id,selection_state);
CREATE INDEX IF NOT EXISTS idx_local_memory_creator ON local_memory_evidence(creator_id,project_id);
