-- SQLite schema baseline v0.1. Apply through ordered sqlx migrations.
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE projects (
 id TEXT PRIMARY KEY, title TEXT NOT NULL, brief TEXT NOT NULL DEFAULT '',
 revision INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE photos (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 import_index INTEGER NOT NULL, original_path TEXT NOT NULL, filename TEXT NOT NULL,
 file_size INTEGER NOT NULL, content_sha256 TEXT, width INTEGER, height INTEGER, orientation INTEGER,
 captured_at TEXT, thumbnail_path TEXT, preview_path TEXT, is_missing INTEGER NOT NULL DEFAULT 0,
 created_at TEXT NOT NULL, UNIQUE(project_id, import_index)
);
CREATE INDEX idx_photos_project_order ON photos(project_id, import_index);
CREATE TABLE photo_decisions (
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 photo_id TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
 status TEXT NOT NULL CHECK(status IN ('undecided','keep','reject')) DEFAULT 'undecided',
 user_note TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL,
 PRIMARY KEY(project_id, photo_id)
);
CREATE TABLE sequence_items (
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 photo_id TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
 position INTEGER NOT NULL, updated_at TEXT NOT NULL,
 PRIMARY KEY(project_id, photo_id), UNIQUE(project_id, position)
);
CREATE TABLE decision_events (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 photo_id TEXT REFERENCES photos(id) ON DELETE SET NULL,
 actor TEXT NOT NULL CHECK(actor IN ('user','ai','system')),
 event_type TEXT NOT NULL, previous_json TEXT, next_json TEXT,
 reason TEXT, context_json TEXT, created_at TEXT NOT NULL
);
CREATE INDEX idx_events_project_time ON decision_events(project_id, created_at);
CREATE TABLE comparison_events (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 preferred_photo_id TEXT NOT NULL REFERENCES photos(id), other_photo_id TEXT NOT NULL REFERENCES photos(id),
 reason TEXT NOT NULL DEFAULT '', context_json TEXT, created_at TEXT NOT NULL,
 CHECK(preferred_photo_id <> other_photo_id)
);
CREATE TABLE sequence_versions (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 version_no INTEGER NOT NULL, photo_ids_json TEXT NOT NULL, label TEXT, created_at TEXT NOT NULL,
 UNIQUE(project_id, version_no)
);
CREATE TABLE ai_runs (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 project_revision INTEGER NOT NULL, provider_id TEXT NOT NULL, model_id TEXT,
 prompt_version TEXT NOT NULL, input_hash TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('queued','running','completed','failed','cancelled')),
 tokens_in INTEGER, tokens_out INTEGER, estimated_cost REAL, error_code TEXT,
 result_json TEXT, created_at TEXT NOT NULL, completed_at TEXT
);
CREATE INDEX idx_runs_project_time ON ai_runs(project_id, created_at);
CREATE TABLE ai_suggestions (
 id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES ai_runs(id) ON DELETE CASCADE,
 project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 photo_id TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
 action TEXT NOT NULL CHECK(action IN ('keep','review','skip')),
 narrative_role TEXT NOT NULL CHECK(narrative_role IN ('opening','transition','anchor','echo','closing','none')),
 reason TEXT NOT NULL, evidence_json TEXT NOT NULL DEFAULT '[]',
 accepted_by_user INTEGER, created_at TEXT NOT NULL,
 UNIQUE(run_id, photo_id)
);
CREATE TABLE photo_observations (
 id TEXT PRIMARY KEY, photo_id TEXT NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
 photo_hash TEXT NOT NULL, provider_id TEXT NOT NULL, model_id TEXT NOT NULL,
 prompt_version TEXT NOT NULL, observation_json TEXT NOT NULL, created_at TEXT NOT NULL,
 UNIQUE(photo_id, photo_hash, provider_id, model_id, prompt_version)
);
CREATE TABLE preference_hypotheses (
 id TEXT PRIMARY KEY, project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
 scope TEXT NOT NULL CHECK(scope IN ('user','project')),
 statement TEXT NOT NULL, evidence_event_ids_json TEXT NOT NULL,
 confidence TEXT NOT NULL CHECK(confidence IN ('low','medium','high')),
 user_corrected INTEGER NOT NULL DEFAULT 0,
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
