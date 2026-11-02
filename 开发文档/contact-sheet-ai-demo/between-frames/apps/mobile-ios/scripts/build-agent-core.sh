#!/bin/bash
# 从桌面同一份源码构建原生 AI 核心；不安装工具链、不修改全局设置。
set -euo pipefail
mobile_root="$(cd "$(dirname "$0")/.." && pwd)"
core_root="$(cd "$mobile_root/../../src-tauri" && pwd)"
target="${1:-aarch64-apple-ios}"
output="${2:-$mobile_root/DerivedData/NativeAgent/$target}"
case "$target" in
    aarch64-apple-ios|aarch64-apple-ios-sim|x86_64-apple-ios) ;;
    *) echo "不支持的 iOS 编译目标：$target" >&2; exit 1 ;;
esac
compiler="$(rustup which rustc)"
cargo_binary="$(rustup which cargo)"
sysroot="$("$compiler" --print sysroot)"
native_rustflags="${CARGO_ENCODED_RUSTFLAGS:-}"
if [ ! -d "$sysroot/lib/rustlib/$target/lib" ]; then
    project_sysroot="$mobile_root/DerivedData/NativeAgent/sysroot"
    if [ ! -d "$project_sysroot/lib/rustlib/$target/lib" ]; then
        echo "当前 Rust 工具链缺少 $target 标准库，请先安装对应编译目标。" >&2
        exit 1
    fi
    if [ -n "$native_rustflags" ]; then native_rustflags+=$'\x1f'; fi
    native_rustflags+="--sysroot"$'\x1f'"$project_sysroot"
fi
profile=dev
artifact=debug
if [ "${CONFIGURATION:-Debug}" = Release ]; then
    profile=release
    artifact=release
fi
cd "$core_root"
CARGO_ENCODED_RUSTFLAGS="$native_rustflags" RUSTC="$compiler" "$cargo_binary" rustc --lib --no-default-features --features native-agent \
    --profile "$profile" --target "$target" --target-dir "$mobile_root/DerivedData/NativeAgent/build" --crate-type staticlib
mkdir -p "$output"
cp "$mobile_root/DerivedData/NativeAgent/build/$target/$artifact/libbetween_frames_lib.a" "$output/libFocalRoomAgent.a"
cp "$core_root/native/focalroom_agent.h" "$output/focalroom_agent.h"
echo "已生成原生 AI 核心：$output"
