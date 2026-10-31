-- 项目归档与删除（FocalRoom 项目管理）：
-- 归档 ARCHIVED 是可逆软状态（仅 SHARED 可归档、恢复回到 SHARED）；
-- 删除 DELETED 是深度软删：清空实时数据（预览对象/授权/草稿/评论/票据），
-- 命令/版本/事件审计行按 0033 隐私架构保留不可变，所有入口对 DELETED 返回 404。
ALTER TABLE fr_projects DROP CONSTRAINT IF EXISTS fr_projects_state_check;
ALTER TABLE fr_projects ADD CONSTRAINT fr_projects_state_check CHECK(state IN ('STAGING','SHARED','ABORTED','ARCHIVED','DELETED'));
ALTER TABLE fr_projects ADD COLUMN IF NOT EXISTS archived_at timestamptz;
ALTER TABLE fr_projects ADD COLUMN IF NOT EXISTS archived_by text REFERENCES fr_users(id);
ALTER TABLE fr_projects ADD COLUMN IF NOT EXISTS deleted_at timestamptz;
