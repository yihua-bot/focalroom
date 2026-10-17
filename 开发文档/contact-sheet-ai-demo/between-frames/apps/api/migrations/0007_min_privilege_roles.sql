-- M4 最小权限数据库账户：迁移角色（DDL）与应用角色（DML only）分离。
-- 由管理员在有超级用户权限时执行一次；应用只获得读写权限。
-- 密码由部署时设置（占位符 <PASSWORD> 需替换）。

-- 迁移角色：可以创建/修改表结构。
CREATE ROLE focalroom_migrator WITH LOGIN PASSWORD '<MIGRATOR_PASSWORD>';
GRANT CONNECT ON DATABASE focalroom TO focalroom_migrator;
GRANT ALL ON SCHEMA public TO focalroom_migrator;
GRANT ALL ON ALL TABLES IN SCHEMA public TO focalroom_migrator;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO focalroom_migrator;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO focalroom_migrator;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO focalroom_migrator;

-- 应用角色：只能 SELECT/INSERT/UPDATE/DELETE，不能 DDL。
CREATE ROLE focalroom_app WITH LOGIN PASSWORD '<APP_PASSWORD>';
GRANT CONNECT ON DATABASE focalroom TO focalroom_app;
GRANT USAGE ON SCHEMA public TO focalroom_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO focalroom_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO focalroom_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO focalroom_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO focalroom_app;

-- 明确拒绝：应用角色不能删表、不能创建/删除角色、不能修改 Schema。
REVOKE ALL ON SCHEMA public FROM focalroom_app;
GRANT USAGE ON SCHEMA public TO focalroom_app;
