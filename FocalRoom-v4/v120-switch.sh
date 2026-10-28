#!/bin/bash
# v1.2.0 切换与健康检查（在服务器上运行）
set -e
PSQL="docker exec focalroom-shared-review-postgres-1 psql -U focalroom -d focalroom -tAc"

echo "===VERSIONS==="
$PSQL "SELECT version FROM fr_migrations ORDER BY version" | tr '\n' ' '

echo ""
echo "===SWITCH==="
cd /opt/focalroom/ops/shared
docker compose -f compose.yaml -f ../../releases/v1.2.0-20261028/runtime.compose.private.json up -d --no-deps api web 2>&1 | tail -3

sleep 6
echo "===CONTAINERS==="
docker ps --format "{{.Names}}|{{.Image}}|{{.Status}}" | grep focalroom

echo "===HEALTH_LOCAL==="
curl -sS -m 8 http://127.0.0.1:8084/api/v2/health || echo "(经 web nginx 代理)"
curl -sS -m 8 http://127.0.0.1:3002/api/v2/health 2>/dev/null || docker exec focalroom-shared-review-api-1 sh -c 'wget -qO- http://127.0.0.1:3002/api/v2/health 2>/dev/null || node -e "fetch(\"http://127.0.0.1:3002/api/v2/health\").then(r=>r.text()).then(t=>console.log(t.slice(0,120)))"'

echo ""
echo "===ROUTES_PUBLIC==="
for ep in auth/me devices billing/catalog ai/account; do
  printf '%-18s ' "/api/v2/$ep"
  curl -sS -o /dev/null -w '%{http_code}\n' -m 8 "https://team.focroom.cc/api/v2/$ep"
done

echo "===USERS_INTACT==="
echo "用户数: $($PSQL 'SELECT count(*) FROM fr_users')"
