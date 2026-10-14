# v1.0 M1 文件与职责

日期：2026-10-14。仅实现统一命令、可恢复人工编辑与私密导出。M0 报告保留为 [V1-M0-TEST-REPORT.md](V1-M0-TEST-REPORT.md)；当前验收见 [TEST-REPORT.md](TEST-REPORT.md)。以下路径相对仓库根目录。

| 文件 | 变更及职责 |
|---|---|
| `between-frames/contracts/gui-command-request.schema.json` | 14 类严格 GUI 命令，必填 revision/因果 ID，无可伪造身份 |
| `between-frames/contracts/domain-envelope.schema.json` | 可信来源/操作者及内部导入、重链 envelope |
| `between-frames/contracts/gui-command-reply.schema.json` | 统一 ok/data/error/revision/requestId 回执 |
| `between-frames/contracts/m1-command-examples.json` | 14 类机器可校验示例 |
| `between-frames/contracts/app-events.json` | 逻辑事件名和 Tauri 合法传输名映射 |
| `between-frames/scripts/generate-command-contracts.mjs` | 从契约生成完整 Rust/TS 请求、命令和事件常量；检查漂移 |
| `between-frames/src/domain/commands.generated.ts`、`between-frames/src-tauri/src/commands_generated.rs` | 生成产物，不手工维护双份命令定义 |
| `between-frames/src-tauri/migrations/0003_command_history.sql` | 六张 M1 表、append-only/归属约束；不修改 0001/0002 |
| `between-frames/src-tauri/src/command_handler.rs` | 唯一正式写入边界，授权、幂等、revision、事务与历史游标 |
| `between-frames/src-tauri/src/edit_commands.rs` | 人工编辑、归档、版本、Undo/Redo 的业务分派 |
| `between-frames/src-tauri/src/edit_state.rs` | 可编辑状态捕获、归属和补偿兼容性校验 |
| `between-frames/src-tauri/src/resource_commands.rs` | 授权导入/重链的资源提交；保持原图只读 |
| `between-frames/src-tauri/src/command_audit.rs` | canonical envelope、指纹、事件、回执和幂等别名 |
| `between-frames/src-tauri/src/db.rs` | 投影查询、旧库一致性备份/升级、委托 Handler 的兼容接口 |
| `between-frames/src-tauri/src/domain.rs` | 版本、历史、归档状态与结构化冲突数据 |
| `between-frames/src-tauri/src/commands.rs`、`lib.rs` | GUI command IPC 与可信身份注入；移除旧 GUI 编辑入口 |
| `between-frames/src-tauri/src/export.rs` | 默认路径字段脱敏、显式迁移导出、CSV/HTML 与拒绝覆盖 |
| `between-frames/src-tauri/src/bin/m1-harness.rs` | 测试专用真实 Store command 桥，生产构建不包含 |
| `between-frames/src/domain/models.ts` | Repository 的历史/版本/事件失效订阅和导出选项 |
| `between-frames/src/services/TauriRepository.ts` | 严格 IPC 校验、同 envelope 传输重试、旧回执重查、事件失效 |
| `between-frames/src/services/MockRepository.ts` | 浏览器示例的内存历史/版本交互，不冒充持久化 |
| `between-frames/src/app/AppController.tsx`、`Workspace.tsx`、`useShortcuts.ts` | 撤销/重做控件、快捷键、版本入口、投影重查与归档导航 |
| `between-frames/src/features/project/VersionsDialog.tsx` | 命名保存、影响范围预览、明确恢复 |
| `between-frames/src/features/export/ExportDialog.tsx`、`src/styles/app.css` | 默认私密导出与布局样式 |
| `between-frames/package.json` | 契约生成/检查脚本，无新增依赖 |
| `between-frames/tauri.m1-review.conf.json` | 独立 M1 Review 名称与 identifier |
| `between-frames/src-tauri/tests/command_history.rs`、`local_core.rs` | 命令/历史/迁移与既有本地核心回归 |
| `between-frames/src/services/TauriRepository.test.ts`、`MockRepository.test.ts` | 回执/事件/内存历史单测 |
| `between-frames/e2e/history-v1.spec.ts`、`local-core.spec.ts` | GUI 历史旅程与真实 Rust/SQLite 综合旅程 |
| `docs/PLAN-V1-M1.md`、`docs/ADR/0004-v1-m1-command-history.md` | 计划、决策与边界 |
| `docs/TEST-REPORT.md`、`docs/KNOWN_ISSUES.md`、两份 README、`DECISIONS.md` | 当前交付状态；旧报告独立保留 |
| `docs/V1-M1-NATIVE-*.json`、`docs/native-m1-exports/` | 重启快照、源文件及导出校验和、实际导出样本 |
| `docs/screenshots/v1-m1/`、`docs/screenshots/v1-m1-native/` | 真实 Store 浏览器及最终原生应用截图 |

SRP：将事务协调、编辑规则、可恢复状态、资源提交和审计拆为各自模块；Store 兼容接口均委托 Handler，不保留第二套正式写入。DIP：UI 依赖 ProjectRepository，桌面和示例遵守相同交互接口。DRY：命令与事件常量从单一契约生成。KISS/YAGNI：只新增 M1 所需六表，未复制整套 24 表或预建 AI、Draft、Memory 空模块；未更换现有视觉体系、拖动库或增加依赖。

原始 Demo、需求包、已应用 0001/0002 migration 保留；本轮未执行 Git commit/push，也未升级用户原应用和 M0 Review 数据库。新 M1 Review 的十二表与临时旧库分别承担原生验收和升级测试。
