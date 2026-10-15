CREATE TABLE memory_evidence_state (
 evidence_id INTEGER PRIMARY KEY REFERENCES preference_evidence(id),
 excluded INTEGER NOT NULL CHECK(excluded IN(0,1)), updated_at TEXT NOT NULL
);
CREATE TABLE memory_commands (
 request_id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id),
 fingerprint TEXT NOT NULL, command_json TEXT NOT NULL,
 changed INTEGER NOT NULL CHECK(changed IN(0,1)), result_json TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE INDEX memory_command_revision ON memory_commands(project_id,changed);
CREATE VIRTUAL TABLE memory_search USING fts5(project_id UNINDEXED, terms, tokenize='unicode61');
ALTER TABLE agent_tasks ADD COLUMN memory_revision INTEGER NOT NULL DEFAULT 0;
