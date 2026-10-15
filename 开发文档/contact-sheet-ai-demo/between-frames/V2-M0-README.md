# v2.0 M0 工程入口

在本目录使用现有 Node/Rust 环境执行 `npm ci`。已验证版本记录于 `.node-version`、`toolchain.json`；第三方版本固定于现有两个 lockfile。

```bash
npm run dev:web           # http://127.0.0.1:1424，内存示例，不提供云协作
npm run dev:desktop       # http://127.0.0.1:1420，桌面前端入口
npm run tauri -- dev --config "src-tauri/tauri.v2-review.conf.json"
npm run build:workspaces  # Desktop/Web 独立生产构建，并检查全部 TS
npm run verify:v2         # v2 基座 + v1 全量回归，跳过 Keychain 写删
```

v2 桌面评审使用独立 identifier `studio.betweenframes.v2foundation`。它不会打开原应用或旧 Review 的数据库；首次启动创建独立评审库，继续使用现有 0001–0010 迁移。

## 目录与状态

| 目录 | M0 内容 |
|---|---|
| apps/desktop | Tauri 使用的 React 入口；复用现有真实本地功能 |
| apps/web | 明示预览模式的 React 入口；示例数据刷新重置 |
| packages/ui | 共享工作台适配入口、基础组件；设计 Tokens 继续来自 src/styles |
| packages/contracts | v2 Schema、TS 类型、权限骨架、TS/Rust 共用 fixtures 与生成 JSON Schema |
| crates/local-domain | 无 I/O 的 Rust 契约和权限检查；作为 src-tauri 的 path dependency |
| apps/api、workers/agent | 明示未实现的命令 skeleton，无监听/认证/队列/模型请求 |
| src、src-tauri | 保留已有成熟 GUI、本地领域、迁移、历史和任务 |

没有 v2 命令会返回写入成功；合法请求也返回 `NOT_IMPLEMENTED`。既有 v1 入口继续承担真实编辑。新增 v2 命令时先改 `packages/contracts/src/schema.ts`，再运行 `npm run contracts:v2:generate`，补齐两端共用样例和执行测试。

数据库调整方案见 `../docs/V2-M0-DATA-COMPATIBILITY.md`；架构见 `../docs/ADR/0016-v2-m0-contracts-and-workspaces.md`。Web 示例与可构建 skeleton 不代表 M4 云共享已实现。
