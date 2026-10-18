#!/bin/bash
# 首次初始化创建最小权限角色；密码来自部署环境变量，不落镜像层与仓库。
# 幂等：重复执行不重复创建。默认权限覆盖此后 superuser 新建的表；
# 迁移角色新建的表由 0009 迁移在每次 migrate 后重放授权。
set -euo pipefail
: "${FOCALROOM_APP_PASSWORD:?set FOCALROOM_APP_PASSWORD}"
: "${FOCALROOM_MIGRATOR_PASSWORD:?set FOCALROOM_MIGRATOR_PASSWORD}"
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<EOSQL
DO \$\$ BEGIN
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_migrator') THEN
    CREATE ROLE focalroom_migrator LOGIN PASSWORD '${FOCALROOM_MIGRATOR_PASSWORD}';
  END IF;
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    CREATE ROLE focalroom_app LOGIN PASSWORD '${FOCALROOM_APP_PASSWORD}';
  END IF;
END \$\$;
GRANT CONNECT ON DATABASE "${POSTGRES_DB}" TO focalroom_migrator, focalroom_app;
GRANT USAGE, CREATE ON SCHEMA public TO focalroom_migrator;
GRANT USAGE ON SCHEMA public TO focalroom_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO focalroom_migrator;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO focalroom_migrator;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO focalroom_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO focalroom_app;
EOSQL
