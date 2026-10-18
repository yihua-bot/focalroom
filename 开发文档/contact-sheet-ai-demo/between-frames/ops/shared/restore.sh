#!/bin/bash
# 使用固定脚本目录；共用标准库实现。恢复默认只验证，不覆盖现有数据。
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"
exec python3 "$SCRIPT_DIR/backup_restore.py" restore "$@"
