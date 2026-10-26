-- 出版版面历史：增量扩展，旧出版物保留原数据并从 revision=1 开始。
ALTER TABLE publications ADD COLUMN revision INTEGER NOT NULL DEFAULT 1 CHECK(revision > 0);

CREATE TABLE publication_versions (
    id TEXT PRIMARY KEY,
    publication_id TEXT NOT NULL REFERENCES publications(id),
    project_id TEXT NOT NULL REFERENCES projects(id),
    revision INTEGER NOT NULL CHECK(revision > 0),
    label TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('working','saved')),
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
    snapshot_sha256 TEXT NOT NULL CHECK(length(snapshot_sha256)=64),
    command_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(publication_id,revision)
);
CREATE INDEX publication_versions_page ON publication_versions(project_id,publication_id,revision DESC);

CREATE TABLE publication_command_events (
    command_id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(id),
    publication_id TEXT NOT NULL REFERENCES publications(id),
    base_revision INTEGER NOT NULL CHECK(base_revision >= 0),
    revision INTEGER NOT NULL CHECK(revision > 0),
    request_json TEXT NOT NULL CHECK(json_valid(request_json)),
    receipt_json TEXT NOT NULL CHECK(json_valid(receipt_json)),
    created_at TEXT NOT NULL
);
CREATE TABLE publication_operation_ids (
    op_id TEXT PRIMARY KEY,
    command_id TEXT NOT NULL REFERENCES publication_command_events(command_id)
);
CREATE TRIGGER publication_versions_no_update BEFORE UPDATE ON publication_versions
BEGIN SELECT RAISE(ABORT,'publication versions are immutable'); END;
CREATE TRIGGER publication_versions_no_delete BEFORE DELETE ON publication_versions
BEGIN SELECT RAISE(ABORT,'publication versions are immutable'); END;
CREATE TRIGGER publication_events_no_update BEFORE UPDATE ON publication_command_events
BEGIN SELECT RAISE(ABORT,'publication events are immutable'); END;
CREATE TRIGGER publication_events_no_delete BEFORE DELETE ON publication_command_events
BEGIN SELECT RAISE(ABORT,'publication events are immutable'); END;
