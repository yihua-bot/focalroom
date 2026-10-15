
CREATE TABLE proposals (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 task_id TEXT,
 author TEXT NOT NULL,
 title TEXT NOT NULL,
 base_revision INTEGER NOT NULL,
 revision INTEGER NOT NULL DEFAULT 1,
 state TEXT NOT NULL DEFAULT 'draft' CHECK(state IN('draft','submitted','applied','discarded')),
 approval_nonce TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE INDEX proposals_project ON proposals(project_id, state, created_at);
CREATE TABLE proposal_operations (
 proposal_id TEXT NOT NULL REFERENCES proposals(id),
 operation_id TEXT NOT NULL,
 photo_id TEXT NOT NULL REFERENCES photos(id),
 op TEXT NOT NULL CHECK(op IN('add','move','remove')),
 to_index INTEGER,
 reason TEXT NOT NULL DEFAULT '',
 PRIMARY KEY(proposal_id, operation_id)
);
