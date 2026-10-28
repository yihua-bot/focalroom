#!/bin/bash
# v1.2.0 迁移执行脚本（在服务器上运行）——定向转移 public 业务对象归属后执行迁移
set -e
PSQL="docker exec focalroom-shared-review-postgres-1 psql -U focalroom -d focalroom -tAc"

echo "===REASSIGN_PUBLIC==="
docker exec focalroom-shared-review-postgres-1 psql -U focalroom -d focalroom -c "DO \$\$ DECLARE r record; BEGIN
  FOR r IN SELECT tablename FROM pg_tables WHERE schemaname='public' AND tableowner='focalroom' LOOP
    EXECUTE format('ALTER TABLE public.%I OWNER TO focalroom_migrator', r.tablename);
  END LOOP;
  FOR r IN SELECT sequencename FROM pg_sequences WHERE schemaname='public' AND sequenceowner='focalroom' LOOP
    EXECUTE format('ALTER SEQUENCE public.%I OWNER TO focalroom_migrator', r.sequencename);
  END LOOP;
END \$\$;"
echo "===OWNER_AFTER==="
$PSQL "SELECT tableowner, count(*) FROM pg_tables WHERE schemaname='public' GROUP BY tableowner"

echo "===MIGRATE==="
cd /opt/focalroom/ops/shared
docker compose -f compose.yaml -f ../../releases/v1.2.0-20261028/runtime.compose.private.json run --rm api node apps/api/src/server.ts --migrate 2>&1 | tail -5

echo "===POST==="
echo "登记总数: $($PSQL 'SELECT count(*) FROM fr_migrations')"
echo "新增版本: $($PSQL \"SELECT string_agg(version, ' ' ORDER BY version) FROM fr_migrations WHERE version > '0021'\")"
