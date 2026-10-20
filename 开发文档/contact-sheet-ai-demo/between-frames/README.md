# FocalRoom

摄影项目工作台：本地选片、比较、作品序列、可审阅的 AI 方案，以及 Web 私密协作。当前客户端配置版本为 **1.0.4**；v3.0 是下一阶段目标方案，按 G0–G8 验收推进，G9 为可选专业服务。

v3 首轮范围为 **G0 基线核对与修复**。自由画布、专业 ICC/高位深显示、Lightroom/Capture One 往返与摄影书印前引擎尚未交付。当前状态和后续迁移设计见：

- [G0 代码与测试快照](../docs/v3/current-state-v3.json)
- [v3 迁移计划](../docs/v3/migrations-plan-v3.md)
- [G0 验收记录](../docs/v3/G0-BASELINE-2026-10-20.md)
- [使用说明](../docs/FOCALROOM-USER-MANUAL.md)

## 工程入口

Tauri 2 / Rust / SQLite 提供本地领域命令、导入、缓存、历史、Agent 与导出；React / TypeScript / Vite 提供界面。`apps/web` 为共享审稿入口，`apps/api` 的 NestJS / PostgreSQL 为共享项目正式权威。`cloud/` 是保留的历史协议测试切片，不能代替正式共享 API。

Node 版本见 `.node-version`（25.8.1），Rust 与 Xcode 工具链需满足当前锁文件。安装依赖后：

```bash
npm ci
npm run dev
npm run dev:web
```

浏览器工作台提供内存演示。真实文件导入、SQLite 持久化和本地 Agent 使用桌面客户端；桌面首次启动可创建内置演示项目。开发验收使用独立的 Tauri review 标识与生成素材，不启动用户正式资料目录。

AI 从选片工作台顶部打开对话抽屉；左下角头像菜单提供登录、设置、语言、帮助和联系作者。仅发送消息才产生任务；AI 方案须经人工采纳才改变正式作品。

## 验证

```bash
npm run contracts:check
npm run typecheck
npm run lint
npm test
npm run test:e2e
npm run verify:v2
```

`verify:v2` 串联工作区构建、Web 入口、前端/Rust/旧云协议、本地真实后端、临时 PostgreSQL、共享 Web/Promotion 及 500 图 GUI 测试。原生打包与浏览器验收须顺序执行。完整验证前需安装 Playwright Chromium 和 PostgreSQL 测试工具，CI 配置见仓库根 `.github/workflows/betweenframes-v2.yml`。

G0 旧库副本升级与恢复证据：

```bash
FOCALROOM_G0_EVIDENCE_FILE="/tmp/focalroom-g0-snapshot.json" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --test v3_baseline
node "apps/api/test/run.mjs" "apps/api/tools/g0-inventory.mjs"
```

这些命令只创建临时数据库和生成图片。真实模型、真实摄影质量、校准设备与印厂验收各自保留独立门槛。

## 数据边界

原图保持只读；预览/缓存与原件分别保存。当前缩略图 320px、编辑预览 1400px、RGB8 JPEG；受限原图检视不等于专业 ICC/16-bit 全流程。CSV/JSON/侧车为现有兼容输出，印刷参考为 HTML 校样，不能称为 XMP 双向集成或 PDF/X 印前文件。

历史迁移字节保持不变。SQLite 由 `Store::open` 备份后升级；PostgreSQL 仅执行 `apps/api/src/database.ts` 登记的迁移，不能遍历 SQL 目录全部执行。共享正式写入须经过权限、版本、幂等和审计；Agent 只能创建待审方案。
