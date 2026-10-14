CREATE TABLE command_events (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 command_id TEXT NOT NULL UNIQUE,
 idempotency_key TEXT NOT NULL UNIQUE,
 request_fingerprint TEXT NOT NULL,
 actor_type TEXT NOT NULL CHECK(actor_type IN ('human','system')),
 source TEXT NOT NULL CHECK(source IN ('gui_direct','human_restore','system_import')),
 command_type TEXT NOT NULL,
 command_json TEXT NOT NULL,
 compensates_group_id TEXT REFERENCES command_events(id),
 causation_id TEXT,
 transaction_group_id TEXT,
 before_json TEXT NOT NULL,
 after_json TEXT NOT NULL,
 result_json TEXT NOT NULL,
 project_revision_before INTEGER NOT NULL,
 project_revision_after INTEGER NOT NULL,
 created_at TEXT NOT NULL
);
CREATE INDEX idx_commands_project ON command_events(project_id,created_at);
CREATE TABLE command_receipt_aliases (
 kind TEXT NOT NULL CHECK(kind IN ('command_id','idempotency_key')),
 value TEXT NOT NULL,
 event_id TEXT NOT NULL REFERENCES command_events(id),
 PRIMARY KEY(kind,value)
);
CREATE TRIGGER command_events_no_update BEFORE UPDATE ON command_events BEGIN SELECT RAISE(ABORT,'command_events are append-only'); END;
CREATE TRIGGER command_events_no_delete BEFORE DELETE ON command_events BEGIN SELECT RAISE(ABORT,'command_events are append-only'); END;
CREATE TRIGGER receipt_aliases_no_update BEFORE UPDATE ON command_receipt_aliases BEGIN SELECT RAISE(ABORT,'receipt aliases are append-only'); END;
CREATE TRIGGER receipt_aliases_no_delete BEFORE DELETE ON command_receipt_aliases BEGIN SELECT RAISE(ABORT,'receipt aliases are append-only'); END;
CREATE TABLE command_history (
 project_id TEXT NOT NULL REFERENCES projects(id),
 position INTEGER NOT NULL,
 group_id TEXT NOT NULL UNIQUE REFERENCES command_events(id) DEFERRABLE INITIALLY DEFERRED,
 active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)),
 PRIMARY KEY(project_id,position,group_id)
);
CREATE UNIQUE INDEX idx_active_history_position ON command_history(project_id,position) WHERE active=1;
CREATE TABLE history_heads (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 cursor INTEGER NOT NULL DEFAULT 0 CHECK(cursor>=0)
);
CREATE TABLE sequence_versions (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 label TEXT NOT NULL,
 based_on_revision INTEGER NOT NULL,
 snapshot_json TEXT NOT NULL,
 created_at TEXT NOT NULL
);
CREATE INDEX idx_versions_project ON sequence_versions(project_id,created_at);
CREATE TRIGGER sequence_versions_no_update BEFORE UPDATE ON sequence_versions BEGIN SELECT RAISE(ABORT,'sequence_versions are immutable'); END;
CREATE TRIGGER sequence_versions_no_delete BEFORE DELETE ON sequence_versions BEGIN SELECT RAISE(ABORT,'sequence_versions are immutable'); END;
CREATE TABLE comparison_state (
 comparison_id TEXT PRIMARY KEY REFERENCES comparison_events(id),
 active INTEGER NOT NULL CHECK(active IN (0,1))
);
CREATE TRIGGER decision_events_no_update BEFORE UPDATE ON decision_events BEGIN SELECT RAISE(ABORT,'decision_events are append-only'); END;
CREATE TRIGGER decision_events_no_delete BEFORE DELETE ON decision_events BEGIN SELECT RAISE(ABORT,'decision_events are append-only'); END;
CREATE TRIGGER decision_owner BEFORE INSERT ON photo_decisions WHEN NOT EXISTS(SELECT 1 FROM photos WHERE id=NEW.photo_id AND project_id=NEW.project_id) BEGIN SELECT RAISE(ABORT,'photo project mismatch'); END;
CREATE TRIGGER sequence_owner BEFORE INSERT ON sequence_items WHEN NOT EXISTS(SELECT 1 FROM photo_decisions WHERE photo_id=NEW.photo_id AND project_id=NEW.project_id AND status='keep') BEGIN SELECT RAISE(ABORT,'sequence photo must be kept in project'); END;
CREATE TRIGGER sequence_owner_update BEFORE UPDATE OF project_id,photo_id ON sequence_items WHEN NOT EXISTS(SELECT 1 FROM photo_decisions WHERE photo_id=NEW.photo_id AND project_id=NEW.project_id AND status='keep') BEGIN SELECT RAISE(ABORT,'sequence photo must be kept in project'); END;
CREATE TRIGGER comparison_owner BEFORE INSERT ON comparison_events WHEN (SELECT COUNT(*) FROM photos WHERE project_id=NEW.project_id AND id IN(NEW.left_photo_id,NEW.right_photo_id))<>2 BEGIN SELECT RAISE(ABORT,'comparison project mismatch'); END;
CREATE TRIGGER decision_owner_update BEFORE UPDATE OF project_id,photo_id ON photo_decisions WHEN NOT EXISTS(SELECT 1 FROM photos WHERE id=NEW.photo_id AND project_id=NEW.project_id) BEGIN SELECT RAISE(ABORT,'photo project mismatch'); END;
CREATE TRIGGER photo_owner_immutable BEFORE UPDATE OF project_id ON photos WHEN OLD.project_id<>NEW.project_id BEGIN SELECT RAISE(ABORT,'photo project is immutable'); END;
CREATE TRIGGER comparison_events_no_update BEFORE UPDATE ON comparison_events BEGIN SELECT RAISE(ABORT,'comparison events are append-only'); END;
CREATE TRIGGER comparison_events_no_delete BEFORE DELETE ON comparison_events BEGIN SELECT RAISE(ABORT,'comparison events are append-only'); END;
