CREATE TABLE v2_command_receipts (
    command_id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(id),
    request_fingerprint TEXT NOT NULL,
    result_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- 保留旧尝试记录，移除覆盖终止态的唯一约束；仅限制同项目的活动尝试。
CREATE TABLE v2_promotions_next (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 initiated_by TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN('pending','uploading','verifying','completed','failed','aborted')),
 from_revision INTEGER NOT NULL,
 detail_json TEXT NOT NULL DEFAULT '{}',
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
INSERT INTO v2_promotions_next SELECT * FROM v2_promotions;
DROP TABLE v2_promotions;
ALTER TABLE v2_promotions_next RENAME TO v2_promotions;
CREATE UNIQUE INDEX v2_promotions_active_project ON v2_promotions(project_id)
 WHERE state IN ('pending','uploading','verifying');
