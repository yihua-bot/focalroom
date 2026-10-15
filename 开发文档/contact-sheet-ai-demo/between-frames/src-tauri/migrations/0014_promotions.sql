
CREATE TABLE IF NOT EXISTS v2_promotions (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 initiated_by TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN('pending','uploading','verifying','completed','failed','aborted')),
 from_revision INTEGER NOT NULL,
 detail_json TEXT NOT NULL DEFAULT '{}',
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL,
 UNIQUE(project_id, from_revision)
);
