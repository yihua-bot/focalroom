#!/usr/bin/env bash
# 独立本地云协议检查；测试仅使用回环 HTTP、临时 SQLite 和生成 JPEG。
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --manifest-path "cloud/Cargo.toml" -- --check
cargo test --manifest-path "cloud/Cargo.toml" --locked
cargo clippy --manifest-path "cloud/Cargo.toml" --locked --all-targets -- -D warnings
