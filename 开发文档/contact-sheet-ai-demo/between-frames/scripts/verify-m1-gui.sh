#!/usr/bin/env bash
# 独立的 500 张混合格式验收；仅创建临时库和临时测试图片。
set -euo pipefail
cd "$(dirname "$0")/.."
export BETWEEN_FRAMES_OPENAI_API_KEY=''
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --locked --release --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npx playwright test --config "playwright.m1.config.ts" "$@"
