#!/usr/bin/env bash
# Between Frames 一键全量验证：与 docs/TEST-REPORT.md 的复现命令保持一致。
# 用法：bash scripts/verify-all.sh [--skip-e2e] [--skip-integration] [--skip-keychain]
# 依赖：Node（npm）、Rust（cargo）、Playwright 浏览器已安装。
set -euo pipefail
cd "$(dirname "$0")/.."

SKIP_E2E=0
SKIP_INTEGRATION=0
SKIP_KEYCHAIN=0
for arg in "$@"; do
  case "$arg" in
    --skip-e2e) SKIP_E2E=1 ;;
    --skip-integration) SKIP_INTEGRATION=1 ;;
    --skip-keychain) SKIP_KEYCHAIN=1 ;;
    *) echo "未知参数：$arg"; exit 1 ;;
  esac
done
# 自动化默认离线；视觉测试使用显式注入的本地 HTTP Provider。
export BETWEEN_FRAMES_OPENAI_API_KEY=''

step() { printf '\n\033[1m== %s ==\033[0m\n' "$1"; }

step "契约检查（GUI + Agent）"
npm run contracts:check

step "前端类型检查"
npm run typecheck

step "前端 Lint"
npm run lint

step "前端单元测试"
npm test

if [ "$SKIP_E2E" = "0" ]; then
  step "浏览器预览 E2E"
  npm run test:e2e
fi

step "Rust 测试（target-v1）"
if [ "$SKIP_KEYCHAIN" = "1" ]; then
  cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" -- --skip store_load_delete_roundtrip
else
  cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1"
fi

step "Rust 格式检查"
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check

step "Rust 严格 Clippy"
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings

step "云协议本地切片（独立 SQLite，不访问线上数据库）"
npm run verify:cloud

step "重建集成 harness（重要：integration 依赖 target-v1 的最新 harness）"
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness

if [ "$SKIP_INTEGRATION" = "0" ]; then
  step "真实后端集成 E2E（含 Beta 旅程）"
  CARGO_TARGET_DIR="target-v1" npm run test:integration
  step "M4/M5 PostgreSQL 与私有存储边界（全新临时 cluster）"
  npm run test:cloud:pg
  step "M4/M5 三会话 Web 与真实 Rust Promotion"
  node "apps/api/test/run.mjs" "apps/api/test/web-journey.mjs"
  node "apps/api/test/run.mjs" "apps/api/test/native-promotion.mjs"
  npm run test:web
fi

printf '\n\033[32m全部通过。\033[0m 性能基线（可选，手动运行）：\n'
printf '  cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --release --test perf_baseline -- --nocapture --ignored\n'
