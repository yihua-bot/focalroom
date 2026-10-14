# v1.0 M0 文件变更

日期：2026-10-13。目录均相对于项目根目录；原 Demo 与归档需求包保持原样。

| 文件 | 变化及原因 |
|---|---|
| `between-frames/src/domain/models.ts` | 人工 rating、批量 Repository 接口 |
| `between-frames/src/domain/selection.ts`、`selection.test.ts` | 纯函数范围/切换选择与边界测试 |
| `between-frames/src/stores/uiStore.ts` | UI 选区、搜索、面板折叠，不存业务事实 |
| `between-frames/src/services/MockRepository.ts`、`MockRepository.test.ts` | 明确 K/X/U、预校验批量操作、评分与无操作语义 |
| `between-frames/src/services/TauriRepository.ts` | 类型化批量 IPC、返回评分 Zod 校验 |
| `between-frames/src/app/AppController.tsx`、`Workspace.tsx`、`useShortcuts.ts` | 搜索/多选/批量操作、比较选区、快捷键隔离与 Loupe |
| `between-frames/src/app/AppShell.tsx`、`SideNav.tsx` | 导航与序列带折叠 |
| `between-frames/src/components/contact-sheet/ContactSheetGrid.tsx` | 多选标记、星级、响应式虚拟行重新测量 |
| `between-frames/src/components/inspector/PhotoInspector.tsx` | 人工评分、明确状态入口 |
| `between-frames/src/components/inspector/Lightbox.tsx` | 单图缩放/平移/导航/关闭 |
| `between-frames/src/components/shared/PhotoViewport.tsx` | 两类查看器共用缓存像素缩放、受限平移与错误状态 |
| `between-frames/src/components/shared/Modal.tsx` | 初始焦点与关闭后焦点恢复 |
| `between-frames/src/components/compare/CompareModal.tsx` | 同步/独立缩放平移，比较保持证据语义 |
| `between-frames/src/styles/app.css`、`src/test/setup.ts` | 新控件与响应式样式；jsdom ResizeObserver 替身 |
| `between-frames/src-tauri/migrations/0002_manual_rating.sql` | 兼容增加人工评分，原 0001 不变 |
| `between-frames/src-tauri/src/domain.rs`、`store.rs` | 评分快照、批量事务、无操作、全量预校验及事件 |
| `between-frames/src-tauri/src/commands.rs`、`lib.rs` | 两个业务 IPC 注册；导入授权类型别名降低类型复杂度 |
| `between-frames/src-tauri/src/importer.rs` | 导入任务类型别名，严格 Clippy 通过 |
| `between-frames/src-tauri/src/export.rs` | CSV 包含人工评分；JSON 使用更新后的 Photo DTO |
| `between-frames/src-tauri/src/bin/m1-harness.rs` | feature 限制的测试命令与夹具 hash 查询 |
| `between-frames/src-tauri/tests/local_core.rs` | 200→25、评分、批量回滚/无操作/归属、旧库迁移测试 |
| `between-frames/e2e/gui-v1.spec.ts` | 四个 GUI 回归场景 |
| `between-frames/e2e/local-core.spec.ts` | 真实 Store 200→25、评分、双尺寸几何、hash、阻断外部网络 |
| `between-frames/scripts/integration-server.mjs` | 支持本轮独立 Cargo target 输出目录 |
| `between-frames/tauri.review.conf.json`、`src-tauri/.gitignore` | 独立 Review 数据目录与 target-v1 忽略规则 |
| `README.md`、`between-frames/README.md`、`DECISIONS.md` | 当前阶段、启动/复现与架构决定 |
| `docs/PLAN-V1-M0.md`、`MEMORY-CONTEXT-ADDENDUM.md` | M0 执行范围与后续记忆/上下文约束 |
| `docs/ADR/0003-v1-m0-gui.md`、`TEST-REPORT.md`、`KNOWN_ISSUES.md` | 决策、验收结果与剩余问题 |
| `docs/V0.1-M1-TEST-REPORT.md` | 保留更新前的历史报告 |
| `docs/V1-M0-NATIVE-*.json` | 原生重启前后六表快照与 200 源 SHA256 证据 |
| `docs/screenshots/v1-m0/`、`v1-m0-native/` | 浏览器真实 Store 与原生 WKWebView 截图 |

未创建 Git 提交或推送；没有引入新依赖、Agent Runtime、云服务或记忆引擎。后续 M1 需以统一命令边界替换当前业务入口，不能把当前批量 no-op 当作 commandId 幂等。
