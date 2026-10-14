# 直接复制给编码 Agent 的执行指令

你是本项目的技术负责人、交互工程师与全栈实施 Agent。你的工作不是讨论创意，也不是只写一个 Landing Page，而是逐步交付**可安装、可测试、以真实本地照片为核心数据的 macOS 桌面软件**。

## 项目上下文（必须先读）

1. 阅读 `README.md`。
2. 阅读并实际运行 `demo-standalone.html`（展示交互），阅读 `index.html` 与 `assets/`（可维护 Demo 源码）。
3. 阅读 `docs/PRODUCT-AND-ENGINEERING-SPEC.md`、`docs/schema.sql`、`docs/AI-OUTPUT.schema.json`。
4. 以 Demo 的主要视觉风格、信息架构和交互布局为 UI 基准；以规格书为最终业务逻辑基准。Demo 与规格书冲突时以规格书为准，尤其是 AI 建议 **不得自动改写人工决策**。Demo 已按此行为区分「AI 建议」与「人工保留」，生产代码继续保持数据层分离。
5. 先检查工具链和已有仓库，不要无条件覆盖用户已有代码；缺失的配置或需求在 `DECISIONS.md` 中记录明确假设并继续实现，而不是陷入反复提问。

## 核心开发约束

- 交付目标：Tauri v2 + React + TypeScript + Vite + Rust + SQLite，macOS 先行。
- 基础项目编辑、导入、筛选、比较、排序、存档、导出**必须完全离线工作**。AI 网络服务是附加能力，不是核心流程的依赖。
- 在 UI 中保持「模拟 AI」和「真实 AI」的显著区分；真实 API 未配置时启用 `MockProvider` 并说明。
- 不要为追求技术复杂度引入云同步、用户注册、容器编排、向量数据库或模型微调。
- 不写假数据冒充真实 AI；用于 Demo 的规则与样本必须标记 mock/test。
- 用户原图只能读取，不准移动、重命名、覆写、删除；缩略图存储在 app cache。
- 任何模型输出都作为待审核建议，绝不直接写 `PhotoDecision`。
- 权限最小化：Tauri 的 FS/path capabilities 收紧，系统选择过的路径才能导入；凭据用 OS keychain，不能明文存本地 DB。
- 所有代码、注释、README 面向中文开发者易读，代码标识符保持英文标准命名。

## 按阶段执行，禁止跳过验收

### M0 · 初始化与设计落地

交付：Tauri/React 项目、lint/typecheck/test 脚本、规范组件、项目结构、样式 tokens、基础路由和可运行 Mac 窗口。

必须实现：
- AppShell、SideNav、ProjectHeader、ProjectList、EmptyState、ContactSheetGrid、PhotoInspector、SequenceBoard、CompareModal、AIPanel、PreferencePanel、ExportDialog。
- React 使用可测试的独立组件；样式 token 统一；视觉参考 Demo，但真实软件不能采用一个巨大的单文件。
- `npm run dev`、`npm run test`、`npm run lint`、`npm run tauri dev` 可按文档启动。

### M1 · 本地核心（无 AI）

交付：SQLite migrations、项目 CRUD、批量照片导入、缩略图、状态变更、照片备注、序列排序、决策事件、项目导出。

实现时要验证：
- JPEG/PNG/WebP/TIFF 成功导入；暂不声称支持 HEIC/RAW。
- 导入 200 张用户照片，关闭软件重启后状态仍然存在。
- 照片失联后能看到 missing 状态和重新链接操作。
- 左右箭头/K/X/ESC、对比 modal、拖动顺序运行正常。
- user decision、comparison、sequence change、note change 都有结构化事件日志。
- 原图读前/读后 SHA256 不变。

### M2 · AI Provider 抽象和 Mock 验收

交付：AI Run 状态机与进度、取消、Provider interface、MockProvider、独立建议草案层、accept/ignore 操作。

实现时要验证：
- 同时保留人与 AI 的两套数据，自动建议不改变最终入选集合。
- 任意 AI 中断、失败、无网络，已编辑的序列不受影响。
- `AI-OUTPUT.schema.json` 验证、照片 ID 校验、错误恢复测试。
- AI run 保存 promptVersion/modelId/inputHash/elapsed/status。

### M3 · 真实视觉 AI

交付：经过显式授权的视觉分析管线：batch observation → project shortlist → sequence suggestion。

实现时要验证：
- 首次点击云端分析明确提示将上传多少张压缩预览至哪个 Provider，默认不上传原图、GPS、文件路径。
- 8–12 张为一个可配置批次，单图 ID 映射不会串图；失败可逐批重试。
- AI 输出有结构化依据和相关图片引用，不能捏造照片 ID 或作者生平。
- 使用真实模型时不得继续显示「模拟 AI」，MockProvider 不得标注为真实分析。
- 可以保留全部人工决策和导出完整 AI 建议差异。

### M4 · 审美记忆与 Beta 发布

交付：证据可追溯的偏好假设（基于事件与备注）、序列版本、性能优化、安装包、详细测试记录。

实现时要验证：
- 偏好假设只从确有证据的重复判断中推断；显示证据 ID/用户原话，允许用户删除或改写。
- 200 张测试图库滚动流畅，图像缓存大小可控。
- 自动化测试通过、真实软件执行 1 次完整用户旅程。
- 把已知问题记录在 `KNOWN_ISSUES.md`，不要以口头“已完成”代替测试证据。

## 建议目录树

```text
between-frames/
├── src/
│   ├── app/                  # router, providers, shell
│   ├── components/
│   │   ├── contact-sheet/
│   │   ├── inspector/
│   │   ├── sequence/
│   │   ├── compare/
│   │   └── shared/
│   ├── features/
│   │   ├── project/
│   │   ├── photo-import/
│   │   ├── selection/
│   │   ├── ai-editor/
│   │   ├── preference-memory/
│   │   └── export/
│   ├── services/             # typed IPC, schema, provider client
│   ├── stores/               # Zustand UI state only
│   ├── styles/
│   └── test/
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   ├── db/
│   │   ├── domain/
│   │   ├── image_pipeline/
│   │   ├── ai_gateway/
│   │   │   ├── mod.rs
│   │   │   ├── mock.rs
│   │   │   └── openai_compatible.rs
│   │   └── lib.rs
│   ├── migrations/
│   ├── capabilities/
│   └── tauri.conf.json
├── e2e/
├── scripts/
├── docs/
│   ├── ADR/
│   ├── USER-FLOW.md
│   ├── TEST-REPORT.md
│   └── KNOWN_ISSUES.md
├── README.md
└── package.json
```

## IPC 与数据实现要求

- 不允许将任意 SQL 暴露为前端查询接口：IPC 按业务 use case 设计。
- 所有修改事务必须更新实体及追加 `decision_events`，避免事件和真实状态不一致。
- 排序写入使用 optimistic revision 或事务，避免重复 position；发生冲突返回可恢复错误。
- UI 的 view state（如当前过滤器）与业务持久化 state（照片状态/排序）必须分离。
- AI suggestions 独立表，不借用 human state 字段。
- 事件日志支持人工操作撤回，不允许通过删除事件假装不存在。

## 测试必须由 Agent 自己执行并反馈

每完成一个里程碑，回复一张简明任务表：

| 项目 | 完成内容 | 验证方式 | 结果 | 未解决问题 |
|---|---|---|---|---|
| ... | ... | ... | passed / failed | ... |

并附：
1. 真实文件变更列表、重要架构权衡、必要命令。
2. 截图（至少：工作台、对比、序列、空状态）。
3. `npm run typecheck && npm run lint && npm run test` 结果。
4. Tauri build 或当前机器无法 build 的真实错误和处理方案；不编造成功。
5. 更新后的 `docs/TEST-REPORT.md` 和下一里程碑任务。

**第一步现在执行：** 建立 M0 工程脚手架并复刻 Demo 的 AppShell、ContactSheetGrid、Inspector 和 SequenceBoard，连接 MockRepository（不连接 AI），运行测试并呈现截图，然后等待我的评审再继续 M1。
