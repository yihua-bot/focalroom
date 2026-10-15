#!/usr/bin/env bash
# v2 M0 叠加验证；既有正式能力仍由 verify-all 检查。
set -euo pipefail
cd "$(dirname "$0")/.."
export BETWEEN_FRAMES_OPENAI_API_KEY=''
npm run build:workspaces
npm run test:web
cargo fmt --manifest-path "crates/local-domain/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --locked -p betweenframes-local-domain -- -D warnings
bash "scripts/verify-all.sh" --skip-keychain

# v2 M1 的 500 张真实文件 GUI 回归独立于 200 张旧旅程。
npm run test:m1-gui
