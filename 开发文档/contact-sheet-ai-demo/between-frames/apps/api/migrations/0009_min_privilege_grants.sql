-- M4 最小权限账户的应用授权（幂等重放；角色由部署初始化创建，见 ops/shared/postgres-init/01-create-roles.sh）。
-- 每次迁移后重放，确保迁移角色新建的表对应用角色可用；应用角色只获得 DML，迁移角色获得 DDL。
-- 角色缺失时静默跳过：本地与测试库允许单账户运行，不因此失败。
DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT USAGE ON SCHEMA public TO focalroom_app;
    GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO focalroom_app;
    GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO focalroom_app;
  END IF;
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_migrator') THEN
    GRANT USAGE, CREATE ON SCHEMA public TO focalroom_migrator;
    GRANT ALL ON ALL TABLES IN SCHEMA public TO focalroom_migrator;
    GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO focalroom_migrator;
  END IF;
END
$$;
