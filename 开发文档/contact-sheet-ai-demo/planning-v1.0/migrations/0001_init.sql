-- BETWEEN FRAMES v0.2 : SQLite initial schema
-- Backend must execute PRAGMA foreign_keys = ON on every connection.
-- Apply with sqlx migration runner; wrap multi-command operations in transactions.

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  brief TEXT NOT NULL DEFAULT '',
  revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS photos (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  file_path TEXT NOT NULL,
  file_size INTEGER NOT NULL DEFAULT 0,
  file_mtime_ns INTEGER,
  file_fingerprint TEXT,
  sha256 TEXT,
  width INTEGER,
  height INTEGER,
  orientation INTEGER,
  import_index INTEGER NOT NULL,
  thumb_path TEXT,
  preview_path TEXT,
  is_missing INTEGER NOT NULL DEFAULT 0 CHECK (is_missing IN (0,1)),
  imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(project_id, id),
  UNIQUE(project_id, import_index),
  UNIQUE(project_id, file_path)
);
CREATE INDEX IF NOT EXISTS idx_photos_project ON photos(project_id, import_index);

CREATE TABLE IF NOT EXISTS photo_decisions (
  project_id TEXT NOT NULL,
  photo_id TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'undecided' CHECK (status IN ('undecided','keep','reject')),
  note TEXT NOT NULL DEFAULT '',
  changed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY(project_id, photo_id),
  FOREIGN KEY(project_id, photo_id) REFERENCES photos(project_id, id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_decisions_project_status ON photo_decisions(project_id, status);

CREATE TABLE IF NOT EXISTS sequences (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL UNIQUE REFERENCES projects(id) ON DELETE CASCADE,
  revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(project_id, id)
);

CREATE TABLE IF NOT EXISTS sequence_items (
  sequence_id TEXT NOT NULL REFERENCES sequences(id) ON DELETE CASCADE,
  photo_id TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
  position INTEGER NOT NULL CHECK (position >= 0),
  PRIMARY KEY(sequence_id, photo_id),
  UNIQUE(sequence_id, position)
);
CREATE INDEX IF NOT EXISTS idx_sequence_order ON sequence_items(sequence_id, position);

CREATE TABLE IF NOT EXISTS drafts (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  task_id TEXT REFERENCES agent_tasks(id) ON DELETE SET NULL,
  title TEXT NOT NULL,
  summary TEXT NOT NULL DEFAULT '',
  base_project_revision INTEGER NOT NULL CHECK (base_project_revision >= 0),
  revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
  status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','stale','applied','archived')),
  created_by TEXT NOT NULL CHECK (created_by IN ('human','agent')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(project_id, id)
);
CREATE INDEX IF NOT EXISTS idx_drafts_project ON drafts(project_id, created_at);

CREATE TABLE IF NOT EXISTS draft_items (
  draft_id TEXT NOT NULL REFERENCES drafts(id) ON DELETE CASCADE,
  photo_id TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
  position INTEGER,
  proposed_action TEXT NOT NULL CHECK (proposed_action IN ('keep','remove','no_change')),
  rationale TEXT NOT NULL DEFAULT '',
  PRIMARY KEY(draft_id, photo_id),
  UNIQUE(draft_id, position),
  CHECK ((proposed_action = 'remove' AND position IS NULL) OR (proposed_action != 'remove' AND position IS NOT NULL))
);

CREATE TABLE IF NOT EXISTS sequence_versions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  label TEXT NOT NULL DEFAULT '',
  based_on_revision INTEGER NOT NULL,
  snapshot_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS command_events (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  command_id TEXT NOT NULL UNIQUE,
  transaction_group_id TEXT NOT NULL,
  actor_type TEXT NOT NULL CHECK (actor_type IN ('human','agent','system')),
  source TEXT NOT NULL,
  command_type TEXT NOT NULL,
  causation_id TEXT,
  before_json TEXT NOT NULL DEFAULT '{}',
  after_json TEXT NOT NULL DEFAULT '{}',
  result_json TEXT NOT NULL DEFAULT '{}',
  project_revision_before INTEGER NOT NULL,
  project_revision_after INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_command_events_project ON command_events(project_id, created_at);

CREATE TABLE IF NOT EXISTS comparison_events (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  command_event_id TEXT REFERENCES command_events(id) ON DELETE SET NULL,
  left_photo_id TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
  right_photo_id TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
  preference TEXT NOT NULL CHECK (preference IN ('left','right','uncertain')),
  reason TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK(left_photo_id != right_photo_id)
);

CREATE TABLE IF NOT EXISTS agent_tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  base_project_revision INTEGER NOT NULL,
  goal TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','planning','running','needs_input','review_ready','completed','paused','stale','failed','cancelled','interrupted')),
  provider_id TEXT NOT NULL DEFAULT 'mock',
  model_id TEXT,
  prompt_version TEXT,
  budget_json TEXT NOT NULL DEFAULT '{}',
  input_summary_json TEXT NOT NULL DEFAULT '{}',
  estimated_cost_minor_units INTEGER NOT NULL DEFAULT 0,
  error_code TEXT,
  error_message TEXT,
  started_at TEXT,
  finished_at TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_agent_tasks_project ON agent_tasks(project_id, created_at);

CREATE TABLE IF NOT EXISTS agent_steps (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
  ordinal INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('planning','tool','observation','message','approval','error')),
  summary TEXT NOT NULL,
  safe_data_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(task_id, ordinal)
);

CREATE TABLE IF NOT EXISTS tool_calls (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES agent_tasks(id) ON DELETE CASCADE,
  call_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  arguments_json TEXT NOT NULL,
  result_summary_json TEXT,
  state TEXT NOT NULL CHECK(state IN ('requested','running','succeeded','rejected','failed','cancelled')),
  error_code TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  completed_at TEXT,
  UNIQUE(task_id, call_id)
);

CREATE TABLE IF NOT EXISTS photo_observations (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  photo_id TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,
  prompt_version TEXT NOT NULL,
  photo_fingerprint TEXT NOT NULL,
  observations_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(photo_id, model_id, prompt_version, photo_fingerprint)
);

CREATE TABLE IF NOT EXISTS preference_evidence (
  id TEXT PRIMARY KEY,
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
  command_event_id TEXT REFERENCES command_events(id) ON DELETE SET NULL,
  comparison_event_id TEXT REFERENCES comparison_events(id) ON DELETE SET NULL,
  evidence_type TEXT NOT NULL CHECK(evidence_type IN ('explicit_keep','explicit_reject','pairwise_comparison','draft_edit','manual_annotation','preference_correction')),
  photo_ids_json TEXT NOT NULL DEFAULT '[]',
  reason TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK(command_event_id IS NOT NULL OR comparison_event_id IS NOT NULL OR reason != '')
);

CREATE TABLE IF NOT EXISTS preference_hypotheses (
  id TEXT PRIMARY KEY,
  scope TEXT NOT NULL CHECK(scope IN ('project_local','global_confirmed','unconfirmed_hypothesis')),
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
  statement TEXT NOT NULL,
  evidence_ids_json TEXT NOT NULL DEFAULT '[]',
  state TEXT NOT NULL DEFAULT 'unconfirmed' CHECK(state IN ('unconfirmed','confirmed','corrected','rejected')),
  user_correction TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK ((scope = 'project_local' AND project_id IS NOT NULL) OR scope != 'project_local')
);

-- app_settings intentionally postponed until product configuration requirements are locked.

-- Cross-project integrity: no photo from project B in project A's sequence or draft.
CREATE TRIGGER IF NOT EXISTS trg_sequence_item_project_guard
BEFORE INSERT ON sequence_items
WHEN (SELECT project_id FROM sequences WHERE id = NEW.sequence_id)
   != (SELECT project_id FROM photos WHERE id = NEW.photo_id)
BEGIN
  SELECT RAISE(ABORT, 'CROSS_PROJECT_SEQUENCE_PHOTO');
END;

CREATE TRIGGER IF NOT EXISTS trg_draft_item_project_guard
BEFORE INSERT ON draft_items
WHEN (SELECT project_id FROM drafts WHERE id = NEW.draft_id)
   != (SELECT project_id FROM photos WHERE id = NEW.photo_id)
BEGIN
  SELECT RAISE(ABORT, 'CROSS_PROJECT_DRAFT_PHOTO');
END;

-- Updating these relational ids is intentionally forbidden; recreate instead.
CREATE TRIGGER IF NOT EXISTS trg_sequence_item_no_reparent
BEFORE UPDATE OF sequence_id, photo_id ON sequence_items
BEGIN
  SELECT RAISE(ABORT, 'SEQUENCE_ITEM_REPARENT_FORBIDDEN');
END;
CREATE TRIGGER IF NOT EXISTS trg_draft_item_no_reparent
BEFORE UPDATE OF draft_id, photo_id ON draft_items
BEGIN
  SELECT RAISE(ABORT, 'DRAFT_ITEM_REPARENT_FORBIDDEN');
END;

-- Official sequence only contains explicitly kept photos.
CREATE TRIGGER IF NOT EXISTS trg_sequence_item_keep_guard
BEFORE INSERT ON sequence_items
WHEN NOT EXISTS (
  SELECT 1 FROM photo_decisions AS pd
  JOIN sequences AS seq ON seq.project_id = pd.project_id
  WHERE seq.id = NEW.sequence_id AND pd.photo_id = NEW.photo_id AND pd.status = 'keep'
)
BEGIN
  SELECT RAISE(ABORT, 'SEQUENCE_ITEM_NOT_KEPT');
END;

-- Changing a kept photo to rejected/undecided must first remove it from official sequence in the same Domain transaction.
CREATE TRIGGER IF NOT EXISTS trg_decision_must_remove_from_sequence_first
BEFORE UPDATE OF status ON photo_decisions
WHEN NEW.status != 'keep' AND EXISTS (
  SELECT 1 FROM sequence_items AS si
  JOIN sequences AS seq ON seq.id = si.sequence_id
  WHERE seq.project_id = NEW.project_id AND si.photo_id = NEW.photo_id
)
BEGIN
  SELECT RAISE(ABORT, 'REMOVE_FROM_SEQUENCE_BEFORE_REJECT');
END;
