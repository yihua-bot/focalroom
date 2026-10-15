
-- v2.0 M6 组织/席位本地骨架（SHARED 模式的本地镜像子集；ADR 0021）。
-- 本地默认：单一 local-workspace 归 local-owner（迁移幂等插入）。
CREATE TABLE IF NOT EXISTS v2_creators (
 id TEXT PRIMARY KEY,
 display_name TEXT NOT NULL,
 kind TEXT NOT NULL DEFAULT 'individual' CHECK(kind IN('individual','org_service')),
 created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS v2_workspaces (
 id TEXT PRIMARY KEY,
 name TEXT NOT NULL,
 owner_creator_id TEXT NOT NULL REFERENCES v2_creators(id),
 plan TEXT NOT NULL DEFAULT 'standard' CHECK(plan IN('standard','pro','edu')),
 seats_limit INTEGER NOT NULL DEFAULT 1 CHECK(seats_limit BETWEEN 1 AND 500),
 created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS v2_memberships (
 workspace_id TEXT NOT NULL REFERENCES v2_workspaces(id),
 creator_id TEXT NOT NULL REFERENCES v2_creators(id),
 role TEXT NOT NULL CHECK(role IN('OWNER','ADMIN','EDITOR','REVIEWER','VIEWER')),
 state TEXT NOT NULL DEFAULT 'active' CHECK(state IN('active','revoked')),
 created_at TEXT NOT NULL,
 PRIMARY KEY(workspace_id, creator_id)
);
CREATE TABLE IF NOT EXISTS v2_workspace_audit (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 workspace_id TEXT NOT NULL REFERENCES v2_workspaces(id),
 event TEXT NOT NULL,
 detail_json TEXT NOT NULL DEFAULT '{}',
 created_at TEXT NOT NULL
);
INSERT OR IGNORE INTO v2_creators(id, display_name, kind, created_at) VALUES('local-owner','本地所有者','individual','1970-01-01T00:00:00Z');
INSERT OR IGNORE INTO v2_workspaces(id, name, owner_creator_id, plan, seats_limit, created_at) VALUES('local-workspace','本地工作区','local-owner','standard',1,'1970-01-01T00:00:00Z');
INSERT OR IGNORE INTO v2_memberships(workspace_id, creator_id, role, state, created_at) VALUES('local-workspace','local-owner','OWNER','active','1970-01-01T00:00:00Z');
