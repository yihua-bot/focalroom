
CREATE TABLE privacy_consents (
 id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL REFERENCES projects(id),
 task_id TEXT REFERENCES agent_tasks(id),
 mode TEXT NOT NULL CHECK(mode IN('task','project','deny')), provider TEXT NOT NULL,
 quota INTEGER NOT NULL CHECK(quota>=0), used INTEGER NOT NULL DEFAULT 0 CHECK(used>=0),
 granted_at TEXT NOT NULL, revoked_at TEXT
);
CREATE INDEX consent_effective ON privacy_consents(project_id,provider,revoked_at,id);
CREATE TABLE photo_observations (
 id INTEGER PRIMARY KEY AUTOINCREMENT, photo_id TEXT NOT NULL REFERENCES photos(id),
 photo_hash TEXT NOT NULL, model_id TEXT NOT NULL, prompt_version TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN('inspect_photos','compare_photos')), observation_json TEXT NOT NULL,
 created_at TEXT NOT NULL
);
CREATE INDEX observation_cache ON photo_observations(photo_hash,model_id,prompt_version,kind);
CREATE TABLE model_usage_v2 (
 task_id TEXT NOT NULL REFERENCES agent_tasks(id), request_id TEXT NOT NULL,
 input_estimate INTEGER NOT NULL, output_estimate INTEGER NOT NULL,
 cloud_images INTEGER NOT NULL CHECK(cloud_images>=0), cost_minor_usd INTEGER NOT NULL CHECK(cost_minor_usd>=0),
 PRIMARY KEY(task_id,request_id)
);
INSERT INTO model_usage_v2(task_id,request_id,input_estimate,output_estimate,cloud_images,cost_minor_usd)
 SELECT task_id,request_id,input_estimate,output_estimate,0,0 FROM model_usage;
DROP TABLE model_usage;
ALTER TABLE model_usage_v2 RENAME TO model_usage;
ALTER TABLE agent_tasks ADD COLUMN provider_id TEXT NOT NULL DEFAULT 'mock';
ALTER TABLE agent_tasks ADD COLUMN consent_id INTEGER REFERENCES privacy_consents(id);
CREATE TABLE consent_receipts (
 request_id TEXT PRIMARY KEY, consent_id INTEGER NOT NULL REFERENCES privacy_consents(id)
);
