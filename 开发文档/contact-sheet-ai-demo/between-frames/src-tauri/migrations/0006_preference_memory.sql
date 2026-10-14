
CREATE TABLE preference_evidence (
 id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL REFERENCES projects(id),
 photo_id TEXT REFERENCES photos(id),
 kind TEXT NOT NULL CHECK(kind IN('keep','reject','comparison','draft_apply','draft_reject','note')),
 detail_json TEXT NOT NULL, dedup_key TEXT NOT NULL, created_at TEXT NOT NULL,
 UNIQUE(project_id, dedup_key)
);
CREATE INDEX evidence_project ON preference_evidence(project_id,kind,id);
CREATE TABLE preference_hypotheses (
 id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL REFERENCES projects(id),
 scope TEXT NOT NULL DEFAULT 'project_local' CHECK(scope IN('project_local','cross_project')),
 statement TEXT NOT NULL, evidence_ids_json TEXT NOT NULL DEFAULT '[]',
 counterexample_ids_json TEXT NOT NULL DEFAULT '[]',
 status TEXT NOT NULL DEFAULT 'unconfirmed' CHECK(status IN('unconfirmed','confirmed','corrected','retired')),
 confidence TEXT NOT NULL DEFAULT 'tentative' CHECK(confidence IN('tentative','moderate')),
 user_correction TEXT, disabled_for_retrieval INTEGER NOT NULL DEFAULT 0,
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE INDEX hypothesis_project ON preference_hypotheses(project_id,status,id);
