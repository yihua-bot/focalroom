# ADR 0016：v2.0 M0 增量工作区与契约边界

日期：2026-10-15。状态：M0 实施中，待本轮评审。

## 决策

- 在现有 `between-frames/` 建 npm workspaces。`apps/desktop` 与 `apps/web` 有独立 Vite 入口和构建；`packages/ui` 复用现有工作台、设计 Tokens 和控件。暂保留 `src/` 与 `src-tauri/` 路径，避免同时重写既有 IPC、资源路径、测试与 Tauri bundle。
- Web M0 明确为示例工作台，使用内存 MockRepository，不读取本地库，不连接云端，不宣称登录、保存或协作。桌面继续使用既有真实 Rust 通道。
- `apps/api` 和 `workers/agent` 仅提供 fail-closed dispatcher skeleton，无网络监听、认证、数据库或模型调用。NestJS、Postgres、队列与供应商路由留对应阶段。
- `packages/contracts` 定义 v2.0 wire Schema 与 TS 类型；生成 JSON Schema 供 Rust 读取。v2 envelope 明确带 `version: "2.0"`，拒绝客户端 actor；不改变 v1 命令、fingerprint 或回执。
- `crates/local-domain` 为纯 Rust 契约/权限基座，作为既有 Tauri crate 的 path dependency。只使用已有 serde/serde_json 版本；不搬动旧锁文件、不引入另一套数据库。
- TS/Rust 共享命令和权限 fixture；Rust DTO 的 kind 枚举由 Schema 生成，payload 通过同一 JSON Schema 校验后才可获得 ValidatedCommand。双方统一 `accepted_operation_ids`，不接受旧样例的歧义字段。
- workspace/project 归属、可信 user/task principal、任务到期、角色、revision、LOCAL/SHARED 单权威均在代码中检查。M0 合法命令最终返回 `NOT_IMPLEMENTED`，不伪造成功；这些入口尚不替代现有已验收 v1 dispatcher。
- 项目业务模式为 LOCAL/SHARED；PROMOTING/SHARED_CACHE 是本地副本状态。云权威命令不能回退本地写入；真实 Promotion 在 M4 实施。
- 旧库默认保持原有单机权威。M0 不新增生产 migration，不执行新需求包的 initial SQL。旧库到 Creator/Workspace/ProjectMode 的回填与单主写保护需单独迁移审查。

## 兼容与安全范围

保留全部 0001–0010 历史迁移、照片/版本/证据 ID、Undo/Redo、检查点和原图引用。不得直接为已有用户建立并行 local_* 权威表。模式与身份在 M0 fixture 中测试，不将未持久化的类型标成实际共享能力。

现有 Schema 参数校验器提取到纯 Rust crate 复用；v1 的包装错误码/消息保持兼容。v2 只使用已支持并有测试的 Schema 子集，不支持的关键字明确拒绝，不能静默忽略约束。

## 工程与回滚

依赖保持现有 lockfile 的已安装版本，记录 Node/Rust/npm 工具链，不以本轮为由升级核心依赖。CI 在 Git 根目录 `.github/workflows`，工作目录指向既有应用。

回滚可停用新增 workspace 入口和 v2 路由，继续原 `npm run dev`/Tauri/v1 命令。无新增数据库格式需要降级。删除或搬动旧目录不属于本轮操作。

## 验收

新旧契约校验、TS/Rust 共用正反例、ACL/单权威、前端及既有本地回归、两端生产构建、独立桌面构建、Web 双尺寸截图。CI 文件已生成不等于远端 CI 已运行。

M0 数据迁移仅交付方案与临时旧库兼容证据；未批准生产结构变更前不得标记完整迁移通过。完成本轮停在 M0，不自动进入 M1。
