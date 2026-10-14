CREATE TABLE projects (
 id TEXT PRIMARY KEY, title TEXT NOT NULL, brief TEXT NOT NULL DEFAULT '',
 revision INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
 deleted_at TEXT
);
CREATE TABLE photos (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id),
 import_index INTEGER NOT NULL, original_path TEXT NOT NULL, filename TEXT NOT NULL,
 file_size INTEGER NOT NULL, content_sha256 TEXT NOT NULL, width INTEGER NOT NULL, height INTEGER NOT NULL,
 orientation INTEGER NOT NULL DEFAULT 1, thumbnail_path TEXT NOT NULL, preview_path TEXT NOT NULL,
 created_at TEXT NOT NULL, UNIQUE(project_id, import_index), UNIQUE(project_id, content_sha256)
);
CREATE TABLE photo_decisions (
 project_id TEXT NOT NULL REFERENCES projects(id), photo_id TEXT NOT NULL REFERENCES photos(id),
 status TEXT NOT NULL CHECK(status IN ('undecided','keep','reject')) DEFAULT 'undecided',
 user_note TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL,
 PRIMARY KEY(project_id, photo_id)
);
CREATE TABLE sequence_items (
 project_id TEXT NOT NULL REFERENCES projects(id), photo_id TEXT NOT NULL REFERENCES photos(id),
 position INTEGER NOT NULL, updated_at TEXT NOT NULL,
 PRIMARY KEY(project_id, photo_id), UNIQUE(project_id, position)
);
CREATE TABLE decision_events (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id), photo_id TEXT REFERENCES photos(id),
 actor TEXT NOT NULL CHECK(actor IN ('user','system')), event_type TEXT NOT NULL,
 previous_json TEXT, next_json TEXT, reason TEXT, context_json TEXT NOT NULL,
 created_at TEXT NOT NULL
);
CREATE INDEX idx_events_project_time ON decision_events(project_id, created_at);
CREATE TABLE comparison_events (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id),
 left_photo_id TEXT NOT NULL REFERENCES photos(id), right_photo_id TEXT NOT NULL REFERENCES photos(id),
 preferred_photo_id TEXT REFERENCES photos(id), reason TEXT NOT NULL DEFAULT '',
 created_at TEXT NOT NULL, CHECK(left_photo_id <> right_photo_id),
 CHECK(preferred_photo_id IS NULL OR preferred_photo_id IN (left_photo_id, right_photo_id))
);
