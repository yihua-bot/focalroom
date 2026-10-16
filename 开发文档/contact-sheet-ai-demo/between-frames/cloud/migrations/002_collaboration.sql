-- M5 云端协作 schema：评论/提案/审批/Presence/快照
CREATE TABLE IF NOT EXISTS project_comments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES cloud_projects(id),
    author_id TEXT NOT NULL REFERENCES users(id),
    photo_sha256 TEXT,           -- NULL = 项目级评论
    anchor_x REAL, anchor_y REAL, -- 归一化坐标
    body TEXT NOT NULL,
    parent_id TEXT REFERENCES project_comments(id), -- 线程回复
    state TEXT NOT NULL DEFAULT 'open' CHECK(state IN('open','resolved')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_comments_project ON project_comments(project_id, state, created_at);
CREATE INDEX IF NOT EXISTS idx_comments_photo ON project_comments(project_id, photo_sha256);

CREATE TABLE IF NOT EXISTS project_proposals (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES cloud_projects(id),
    author_id TEXT NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    base_revision INTEGER NOT NULL,
    state TEXT NOT NULL DEFAULT 'submitted' CHECK(state IN('submitted','applied','rejected','conflict')),
    operations_json TEXT NOT NULL DEFAULT '[]',
    approval_nonce TEXT NOT NULL,
    decided_by TEXT REFERENCES users(id),
    decided_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_proposals_project ON project_proposals(project_id, state);

CREATE TABLE IF NOT EXISTS project_revisions (
    project_id TEXT NOT NULL REFERENCES cloud_projects(id),
    revision INTEGER NOT NULL,
    snapshot_json TEXT NOT NULL,   -- 权威项目快照（序列/选片/评分）
    author_id TEXT NOT NULL REFERENCES users(id),
    cause TEXT NOT NULL,           -- proposal_apply / manual / system
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY(project_id, revision)
);

CREATE TABLE IF NOT EXISTS project_presence (
    project_id TEXT NOT NULL REFERENCES cloud_projects(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    session_id TEXT NOT NULL,
    last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    viewing_sha256 TEXT,
    PRIMARY KEY(project_id, user_id, session_id)
);

CREATE TABLE IF NOT EXISTS acl_audit (
    id SERIAL PRIMARY KEY,
    project_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    allowed BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
