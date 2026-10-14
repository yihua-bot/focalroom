# 序间 · BETWEEN FRAMES — v1.0 M1

Tauri v2 + React + TypeScript + Vite + Rust/sqlx/SQLite。离线选片核心之上已完成唯一 CommandHandler、严格生成命令契约、幂等/冲突、持久化 Undo/Redo、命名版本和默认私密导出。**v1.0 M1 已测试，停止等待评审。** AI Runtime、Provider、Draft、ContextBuilder 与记忆引擎留在后续里程碑。

## 启动

需要 Node.js 22.12+、npm、Rust stable、Xcode 工具链。首次安装需下载依赖，依赖就绪后本地工作流不依赖网络。

```bash
cd "/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames"
npm ci
CARGO_TARGET_DIR="target-v1" npm run tauri dev -- --config "tauri.m1-review.conf.json"
```

`npm run dev` 提供浏览器内存示例预览；真实文件导入、存档和导出必须使用桌面版。桌面默认打开项目列表，不注入示例项目。

本机存在的 Debug 应用：

```text
src-tauri/target-v1/debug/bundle/macos/Between Frames M1 Review.app
```

本轮 Review 数据库与缓存：`~/Library/Application Support/studio.betweenframes.m1review/BetweenFrames/`。省略此 config 使用旧 `studio.betweenframes.desktop` 数据目录，启动会应用 M1 migration；本轮未升级用户原 DB 或 M0 Review DB。旧日志保留，升级前操作不生成追溯撤销栈。原图保留在系统选择器授权的原目录，失联可按相同指纹重新定位。

## 本地编辑

1. 创建项目、写创作意图，导入照片或文件夹。
2. 支持 JPEG/PNG/WebP/TIFF；macOS HEIC 与相机 RAW 取决于系统解码器。失败逐张列明，可重试；同一项目的完全相同文件会跳过。
3. 导入显示进度，可取消。取消等待当前照片解码结束，已经提交的部分保留。
4. `K` 保留、`X` 淘汰、`U` 待定，重复设置不切换；`0–5` 人工评分。Cmd/Ctrl 点选切换多选、Shift 点选/方向键连选；Cmd/Ctrl+A 选中当前视图。`← / →` 翻片，`G/S` 切换网格/序列，Cmd/Ctrl+F 搜索文件名/备注；输入时隔离快捷键。
5. 备注草稿可随照片切换保留，点击保存后写入 SQLite。关闭应用前请保存草稿。
6. Space 或检查器大图按钮打开 Loupe，fit/100% 缓存像素、缩放、平移、前后导航，Esc 关闭。选两张后 `C` 比较，支持同步/独立缩放平移与理由；左、右或不确定仅记录偏好，不自动保留/淘汰。
7. 作品序列支持把手拖拽、键盘拖放、上移/下移和 `↑ / ↓`。顺序自动持久化。
8. Cmd/Ctrl+Z 撤销一组人工编辑，Cmd/Ctrl+Shift+Z 重做，重启后历史保留。新有效编辑使 redo 分支失效，审计不删除；导入/重链不通过 Undo 删除照片。
9. 保存命名版本，包含选片、评分、已保存备注和顺序。恢复前预览影响范围，恢复是可撤销的新编辑；版本后导入的照片留在项目中，选片恢复为待定。
10. JSON/CSV 默认省略已知资源路径字段；迁移用途需显式勾选包含本地路径，自填备注路径仍原文保留。接触印相 HTML 含小尺寸缓存，可打开打印。只创建新文件，不覆盖已有文件。
11. 项目可编辑意图、归档与恢复；归档不删除原图、备注或日志。

100 张以上采用虚拟网格；检查器和比较使用最长边 1400px 编辑预览，网格仅使用 320px 缓存。100% 指缓存预览像素，不代表原图逐像素。导航、检查器与 Filmstrip 可折叠；Cmd/Ctrl+Shift+A 切换 AI 占位面板，不启动模型。

## 验证

```bash
npm run contracts:check
npm run typecheck
npm run lint
npm run test
npx playwright install chromium
npm run test:e2e
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --test command_history --test local_core -- --nocapture
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
CARGO_TARGET_DIR="target-v1" npm run tauri build -- --debug --config "tauri.m1-review.conf.json"
```

真实 Rust/SQLite 驱动的浏览器旅程：

```bash
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
```

测试生成自己的数据库和图库，不读取生产项目；失联测试只移动测试生成的照片。集成 harness 不包含在正常 Tauri 产物中。Rust 的原生 HEIC/CR2 用例需要先准备报告中注明的测试夹具，否则明确输出 SKIP。

本机 crates.io 网络受限时，可对 Cargo 命令追加以下参数（不修改全局配置）：

```text
--config 'source.crates-io.replace-with="m0-mirror"'
--config 'source.m0-mirror.registry="sparse+https://rsproxy.cn/index/"'
```

已下载依赖可再加 `--offline`。Tauri build/dev 将这些参数放在第二个 `--` 之后，具体复现命令见父目录 `docs/TEST-REPORT.md`。

架构保持 UI 状态（Zustand）、仓库快照（TanStack Query）、校验 IPC（Zod）、业务事务（CommandHandler）、投影查询（Store）、图像流水线、导入任务与导出服务的职责边界。完整变更、测试结果和阶段限制见父目录 `docs/`。
