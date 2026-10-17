#!/bin/bash
# M4 数据库恢复脚本：从备份归档恢复 PostgreSQL + 预览文件。
# 用法：./restore.sh <备份文件.tar.gz>
set -euo pipefail

ARCHIVE="${1:?用法: ./restore.sh <备份文件.tar.gz>}"
PROJECT="${2:-focalroom-shared-review}"
TEMP_DIR=$(mktemp -d)

echo "=== 1. 解压备份 ==="
tar xzf "$ARCHIVE" -C "$TEMP_DIR"
BACKUP_DIR=$(ls -d "$TEMP_DIR"/focalroom-backup-* | head -1)
cd "$BACKUP_DIR"

echo "=== 2. 校验完整性 ==="
sha256sum -c SHA256SUMS
echo "  ✅ 校验和一致"

echo "=== 3. 恢复 PostgreSQL ==="
# 停止 API 以避免写入冲突。
docker compose -f "$(dirname "$0")/../compose.yaml" stop api 2>/dev/null || true

# 恢复数据库（覆盖现有数据）。
cat database.dump | docker compose -f "$(dirname "$0")/../compose.yaml" exec -T postgres \
  pg_restore -U focalroom -d focalroom --clean --if-exists
echo "  ✅ 数据库已恢复"

echo "=== 4. 恢复预览文件 ==="
PREVIEW_VOL="${PROJECT}_preview-data"
docker run --rm \
  -v "$PREVIEW_VOL:/data" \
  -v "$BACKUP_DIR:/backup" \
  alpine sh -c "cd /data && tar xzf /backup/previews.tar.gz"
echo "  ✅ 预览已恢复"

echo "=== 5. 重启服务 ==="
docker compose -f "$(dirname "$0")/../compose.yaml" up -d

echo ""
echo "✅ 恢复完成"
echo "⚠️  建议验证：登录 → 查看项目 → 检查评论/版本历史"

# 清理
rm -rf "$TEMP_DIR"
