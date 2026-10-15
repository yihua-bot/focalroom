
CREATE TABLE memory_tombstones (
 id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL REFERENCES projects(id),
 kind TEXT NOT NULL CHECK(kind IN('hypothesis','evidence')), ref_key TEXT NOT NULL,
 created_at TEXT NOT NULL, UNIQUE(project_id, kind, ref_key)
);
