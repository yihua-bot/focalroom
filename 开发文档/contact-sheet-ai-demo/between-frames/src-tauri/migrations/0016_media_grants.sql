
CREATE TABLE IF NOT EXISTS v2_media_grants (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 photo_id TEXT NOT NULL REFERENCES photos(id),
 token TEXT NOT NULL,
 expires_at_ms INTEGER NOT NULL,
 state TEXT NOT NULL DEFAULT('active') CHECK(state IN('active','revoked')),
 created_at TEXT NOT NULL,
 UNIQUE(project_id, photo_id, token)
);
CREATE INDEX media_grants_token ON v2_media_grants(token, state);
