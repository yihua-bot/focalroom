#!/bin/bash
# M4 数据库备份脚本：PostgreSQL + 预览文件 → tar.gz 归档。
# 用法：./backup.sh <输出目录> [compose项目名]
set -euo pipefail

OUT_DIR="${1:?用法: ./backup.sh <输出目录> [compose项目名]}"
PROJECT="${2:-focalroom-shared-review}"
STAMP=$(date +%Y%m%d-%H%M%S)
BACKUP="$OUT_DIR/focalroom-backup-$STAMP"

mkdir -p "$BACKUP"

echo "=== 1. PostgreSQL 逻辑备份 ==="
docker compose -f "$(dirname "$0")/../compose.yaml" exec -T postgres \
  pg_dump -U focalroom -d focalroom --format=custom \
  > "$BACKUP/database.dump"
echo "  数据库: $(du -sh "$BACKUP/database.dump" | cut -f1)"

echo "=== 2. 预览文件备份 ==="
PREVIEW_VOL="${PROJECT}_preview-data"
docker run --rm \
  -v "$PREVIEW_VOL:/data:ro" \
  -v "$BACKUP:/backup" \
  alpine tar czf /backup/previews.tar.gz -C /data .
echo "  预览: $(du -sh "$BACKUP/previews.tar.gz" | cut -f1)"

echo "=== 3. 校验和 ==="
( cd "$BACKUP" && sha256sum database.dump previews.tar.gz > SHA256SUMS )
echo "  $(cat "$BACKUP/SHA256SUMS" | head -1)"

echo "=== 4. 压缩归档 ==="
tar czf "$OUT_DIR/focalroom-backup-$STAMP.tar.gz" -C "$OUT_DIR" "focalroom-backup-$STAMP"
rm -rf "$BACKUP"
echo "  归档: $(du -sh "$OUT_DIR/focalroom-backup-$STAMP.tar.gz" | cut -f1)"

echo ""
echo "✅ 备份完成: $OUT_DIR/focalroom-backup-$STAMP.tar.gz"
echo "恢复: ./restore.sh $OUT_DIR/focalroom-backup-$STAMP.tar.gz"
