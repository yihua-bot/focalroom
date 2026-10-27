-- J5 团队席位（剩余清单 #10）：席位分配挂在工作区，配额来自工作区所有者
-- 有效订阅的价格档位（price.seats）。席位是分配记录：不共享代币额度、
-- 不改成员角色；订阅失效时配额归零，既有分配冻结可查、仍可清理。

CREATE TABLE IF NOT EXISTS fr_team_seats (
    workspace_id TEXT NOT NULL REFERENCES fr_workspaces(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES fr_users(id),
    granted_by TEXT NOT NULL REFERENCES fr_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, user_id)
);

DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT SELECT, INSERT, DELETE ON fr_team_seats TO focalroom_app;
  END IF;
END
$$;
