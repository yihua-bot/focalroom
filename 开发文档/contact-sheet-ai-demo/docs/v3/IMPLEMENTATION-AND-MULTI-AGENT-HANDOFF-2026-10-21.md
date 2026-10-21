# FocalRoom v3 · 多 Agent 协作实施与交接

版本：2026-10-21，Asia/Shanghai。用途：交给参与 FocalRoom 开发的 Agent，按同一产品目标、源码基线和验收标准协作。

**接手结论：先恢复可信基线、补齐接口与权限，再完成 G1–G8。当前仓库已有多个 v3 模块的首批实现，尚不能将 G0–G7 标记为全部验收完成。** 本文给出的任务状态来自下面列明的源码与 CI 核对；实施建议和待验证事项均单独标示。

后续开发进度见 [H0 修复与执行记录](H0-HARDENING-2026-10-21.md)。本文中的代码缺口描述保留为基线审计，不能替代后续修复记录。

## 1. 使用方式与依据

把本文、`工程宝/Engineering/` 和分配给该 Agent 的任务单一并交接。Agent 领取一个明确任务包，在独立工作目录开发，返回代码、验证证据和剩余问题，由集成人员合入。本文是实施交接，不取代主规格，也不构成任意生产操作或收费调用的授权。

阅读顺序：

1. 用户在当前会话中的产品要求，以及仓库内适用的 `AGENTS.md`。
2. [v3 最终主计划](../../../../工程宝/Engineering/FOCALROOM-V3-FINAL-MASTER-PLAN.md)。
3. [原 Agent 接手入口](../../../../工程宝/Engineering/delivery/AGENT-START-HERE.md)、[ADR 与差距地图](../../../../工程宝/Engineering/architecture/ADR-AND-GAP-MAP.md)。
4. [交互契约](../../../../工程宝/Engineering/ux/SCREEN-AND-INTERACTION-CONTRACT.md)、[验收矩阵](../../../../工程宝/Engineering/quality/ACCEPTANCE-MATRIX.md)、[色彩/印前验证规程](../../../../工程宝/Engineering/quality/COLOR-PRINT-VALIDATION.md)。
5. [53 项原始 Backlog](../../../../工程宝/Engineering/delivery/IMPLEMENTATION-BACKLOG.csv) 与本文件的任务对应表。
6. `工程宝/Engineering/contracts/` 内的 SQL、OpenAPI、Canvas、Agent 和回流 manifest 示例。
7. [G0 历史验收](G0-BASELINE-2026-10-20.md)、[迁移评审稿](migrations-plan-v3.md)，然后核对当前源码和 CI。

主计划和原始 Backlog 中的“目标”“NOT_STARTED_TARGET”、旧 README 中的“尚未实现”，都不是今天的实时状态。遇到冲突，先区分规范要求与旧状态快照，再记录 ADR；不能根据旧交接删掉已经存在的功能。

### 1.1 路径与工具链

| 项目 | 当前位置/值 |
|---|---|
| 仓库根目录 | `/Users/yihua/Documents/film` |
| 产品工程目录，下文记作 APP | `/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames` |
| 产品文档目录，下文记作 DOCS | `/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/docs` |
| 原计划目录 | `/Users/yihua/Documents/film/工程宝/Engineering` |
| GitHub | `https://github.com/yihua-bot/focalroom` |
| 本次核对 HEAD | `4ca66a1fc6ed71622f51b0923f3147c7ad05174a` |
| 当前技术栈 | Tauri 2、Rust、SQLite；React 19、TypeScript、Vite；NestJS、PostgreSQL |
| 锁定工具链 | Node 25.8.1、npm 11.11.0、Rust 1.94.0；见 APP/toolchain.json、.node-version 和锁文件 |

除另有说明，本文代码路径均相对 APP；根工作流路径相对仓库根。新机器的绝对路径可以不同，必须重新确认 APP 与 Git 根目录。

## 2. 当前可核实基线

### 2.1 Git、CI 与验证边界

- 本次读源码时，HEAD 为 `4ca66a1`，工作树未显示未提交修改。接手后必须重新检查，不能沿用这条记录作为未来的干净证明。
- 之前 `088275b` 的 [G0 CI 39423899048](https://github.com/yihua-bot/focalroom/actions/runs/39423899048) 全绿，是历史可复现基线，不是新功能提交的自动背书。
- 当前 HEAD 对应的 [CI 39458087602](https://github.com/yihua-bot/focalroom/actions/runs/39458087602) 为 **failure**。读取失败日志，至少确认 `account-security.mjs` 与 `min_privilege.mjs` 的迁移列表断言仍只期望到 0015，而实际已登记 0017、0018。
- 当前 PostgreSQL registry 在 `apps/api/src/database.ts` 中明确登记 **13 个迁移**；SQLite 目录有 **23 个迁移文件**。这些是文件/登记数，不能据此推断用户数据库或生产数据库已经迁移。
- 汇报中的“265 前端、232 Rust”等单项计数不能替代最新提交的完整门禁。本次交接编写没有重新执行全量产品测试，也没有验收新原生窗口或真实 Lightroom/Capture One。
- 现有 `current-state-v3.json` 和部分 README 是此前 G0 快照，应由基线负责人在新门禁通过后更新，保留历史证据。

验收状态统一使用：`IMPLEMENTED_PARTIAL`（已有部分实现）、`AUTOMATED_PASS`（指定 SHA 自动化通过）、`AUTOMATED_FAIL`、`NOT_TESTED`、`EXTERNAL_BLOCKED`、`ACCEPTED`。只有相应验收矩阵全部满足、证据齐全，才能把 Gate 标为 `ACCEPTED`。

### 2.2 已有实现与真实缺口

| Gate | 已发现的代码 | 当前接续判断 |
|---|---|---|
| G0 | 基线报告、盘点工具、旧格式副本恢复测试、完整验证脚本 | 历史基线曾全绿；最新提交发生 CI 回归，先恢复 |
| G1 | 本地 0020 的 5 张资产域表、云端 0017 的 4 张表；回填与导入资产写入代码 | 已有身份层首批实现；需补旧数据、跨权威 ID、引用归属、双写/重试一致性和实测 |
| G2 | `color_engine.rs` 提取 ICC 信息、digest、四种 PipelineTarget，导入中接入 | 当前文件明确写明是检测阶段；没有完成原生 ColorSync、高位深瓦片及专业显示验证 |
| G3 | `canvas.rs`、0021、React NarrativeCanvas、导航和 IPC；8 种节点类型 | 数据/交互首批实现存在；实际页面 repository、像素引用、命令历史、取消/重做、性能和可访问性仍需补齐 |
| G4 | Canvas 工具 handler、部分机器契约、既有 Agent runtime | 工具注册、schema/PolicyGate、草案归属存在断点；不等于已完成可审批的 Canvas Agent |
| G5 | `external_edit.rs`、0022、Lightroom 导出相关 Lua 文件、manifest 扫描/处理函数 | 不是完整往返；文件真实性校验、持续监测接入、稳定身份、多版本、实际 LR/C1 验收仍缺 |
| G6 | 云端 0018 的 3 张表、BoardService、部分路由、独立 BoardViewer 组件 | 列表/节点/评论接口和页面入口未形成完整闭环；权限与正式命令路径须先补 |
| G7 | `publishing.rs`、0023、创建/列表/生成 IPC、PublicationPanel | 本次实数为 **5 张新增表**；页面生成有 SQL 缺陷，UI 未注入 repository；尚无完整 Edition、印前引擎和交付链 |
| G8 | 既有签名/公证/账户/部署文档及代码可供复用 | 对最终候选 SHA 重新验收；历史发行记录不能证明当前 v3 安装包已经合格 |
| G9 | 目标规格 | 可选后续，不作为 G8 前的功能扩张方向 |

这里没有把“代码存在”记为“可在用户入口完成操作”，也没有把未验收工作统一归因为缺外部资源。下一节列出可以立刻处理的工程缺口。

## 3. 第一轮：H0 基线与纵向链路修复

H0 是本文新增的交接任务编号，不替代原始 FR3 编号。优先完成 H0，再推进后续 Gate 的新增范围。原有代码保留，在小切片中修复。

### H0-01 · 恢复最新 CI 与迁移验证

**负责人：L，A 复核数据库语义。**

证据：`apps/api/src/database.ts` 已登记 `0017_cloud_asset_domain.sql`、`0018_cloud_canvas.sql`；`apps/api/test/account-security.mjs:58` 和 `apps/api/test/min_privilege.mjs:37` 的旧期望尚未包含它们。

交付：同步迁移期望，并验证新表、外键、历史迁移哈希、重复执行、migrator/app 最小权限；检查其他 schema 断言、备份恢复断言和旧库兼容夹具。不能通过删去迁移列表比较、把精确列表改成任意子集或跳过 PG 套件来关闭失败。

退出：候选分支的完整 `verify:v2`、独立 Review 构建和合入后 CI 全部通过；重新生成对应 SHA 的状态快照，明确 ignored/未覆盖项目。

### H0-02 · 接通真实桌面入口

**负责人：C，A 提供 IPC/DTO。**

证据：`src/app/Workspace.tsx:66–67` 使用 PublicationPanel 与 NarrativeCanvas 时没有传 `repository`；两组件的 repository 为可选参数，缺失即显示“需要桌面版”。`TauriRepository.ts` 中未发现已接通的 canvas/publication 适配成员。

交付：沿用现有 Repository 模式实现适配，通过 AppController/Workspace 注入；明确 Rust 返回 snake_case 与前端 camelCase 的转换；照片 Node 通过经过授权的资产/渲染引用获得真实图像。新视图不能同时落入旧“非 grid 就展示 SequenceBoard”的分支。加载/失败必须有可见反馈，不能吞错后伪装为空项目。

退出：在隔离的真实 Rust harness 和独立原生评审包中，从项目入口创建 Board、放入照片、拖动保存、重启恢复；创建出版物并生成非空页面；导航只显示当前视图。浏览器演示也必须区分内存预览与真实桌面能力。

### H0-03 · 修复 Agent 契约、策略与草案写入

**负责人：D，A 审查领域归属。**

静态证据：

- 三个已登记 Canvas 工具 `create_board_draft`、`place_photos_on_board`、`get_board` 的 schema 没有 `project_id`；`agent/policy.rs:102` 却统一要求 `call.args["project_id"] == task.project_id`。不能靠在 strict schema 外加字段调用来绕过。
- `group_nodes`、`create_sequence_draft` 有 handler，但尚未发现对应新契约登记。
- `evaluate_sequence` 已是 `evaluate_draft` 的 v2Alias；另加同名分支会与 canonical 解析冲突。需保留旧任务兼容，给新语义明确名字。
- `get_board` 按 board_id 直接读节点；place 的嵌套 placements、group 的子节点需要项目、Board、任务作用域检查。修复契约使工具可用前，必须同时补齐这些检查。
- Board 仅有 `state='draft'` 不等于属于某个 Agent 任务。不能让模型向用户任意草稿板、正式板或冻结板直接写入。

交付：工具名称/alias/schema/PolicyGate/runtime 同步；任务拥有的草案作用域与正式 Board 分离；人工采纳使用可信命令、revision、幂等、审计与 Undo。序列草案要按当前真实 drafts/draft_items schema 写入，包括 task 关联和状态，而不是假定仅有 title 的旧结构。

预算处理：先记录实际发送的工具 catalog、输入估算、输出预留及供应商计量；必要时按任务暴露工具子集。只有语义确实变化的正常旅程更新夹具预算；零预算拒绝、超额暂停、未知结果不重传、原图不外发等边界断言继续保留。**不把“同步提高所有测试预算”作为接通工具的方法。** 本次读到的远端失败是迁移断言，不应把它描述成已证实的预算失败。

退出：六类期望能力分别经真实 ToolRegistry→PolicyGate→runtime 跑通；跨项目/跨任务/越选区/正式板/冻结板/自我审批均被拒绝；未采纳时正式投影不变，重复工具调用不新增副作用。

### H0-04 · 补齐云画布 ACL、API 与 Web 接线

**负责人：A，C 接入前端。**

证据：`apps/api/src/server.ts:102–112` 的新 Board 路由进行了账户认证，但 BoardService 的 list/create/annotate 没有接入既有项目成员/Grant 校验。BoardViewer 请求 `/boards/{id}/nodes`、`/boards/{id}/annotations` 和 resolve，而当前新增路由只看到列表/创建/添加评论；其列表请求没有带服务端所需 project 参数，且未发现该组件被父页面使用。

交付：认证与项目授权分别验证；Board→Project、Node→Board、Annotation→Board 的归属来自数据库；补齐实际查询/解决评论路由、合法输入和结构化错误；连接 Web 页面入口及带项目范围的列表请求。正式编辑继续经过共享权威，不因新增接口绕过审批和审计。

退出：两用户、两个项目以及 Viewer/评论者/批准者的 API 权限矩阵通过；有效登录但无项目权限也必须拒绝；撤权后禁止新读取/写入；Web 实际显示授权照片、评论并正确反馈失败。该项未完成前，不把新路由发布到生产。

### H0-05 · 修复非空出版生成

**负责人：A，C 验收入口。**

证据：`publishing.rs:143` 封面 INSERT 有 10 个字段、11 个 VALUES 项；当前 `publication_create_generate_and_list` 仅验证创建、列表与空序列错误，没有执行非空生成成功路径。

交付：修复绑定、页框坐标和边距，增加有真实资产 alias 的非空序列成功验证；明确重复生成对人工页面/文字的影响、确认步骤、事务回滚和版本保留。页面/图框的资产与 rendition 必须同属一个源资产。

退出：1/2/4 图布局均可保存读取，页数和照片顺序可核验，封面/内页框不越界；失败保留原版，重复操作幂等；至少一个由 UI 发起、重启后仍可读取的成功旅程。

### H0-06 · 外部回流完整性与目录边界

**负责人：B，A 提供资产/版本接口。**

证据：`external_edit.rs:283` 使用 manifest 声明的 SHA，而不是实际计算并比较文件哈希；当前匹配没有完整绑定 project/provider/catalog/source hash；render_filename 直接 join，需阻止绝对路径、`..` 与越界符号链接。`INSERT OR IGNORE` 后直接返回新生成 ID，在重复记录被忽略时可能返回不存在的 ID。

交付：受控导入目录、规范化路径/越界检查、文件写完判定、大小上限、真实 SHA/尺寸/位深/ICC 校验、完整匹配键、重复批次回执、返回真实存储 ID。失败不得计为 matched，不凭同名文件猜归属；不完整条目进入需处理状态。

退出：篡改 manifest 哈希、跨项目 ID、另一 Catalog 同名/同外部 ID、半文件、重复 manifest、路径穿越与符号链接案例均有真实临时文件测试；原图字节保持不变。

### H0-07 · 将新写入接回命令、历史与恢复体系

**负责人：A，L 复核既有回归。**

证据：`canvas.rs` 的若干写入直接开启事务并改表，未发现与其 undo 所期待的 canvas 事件相配套的统一写链路；`undo_canvas` 查询最近 canvas 事件，并尝试 UPDATE command_events.after_json，但旧 migration 0003 明确对 command_events 配置 append-only 触发器。仅使用 `CommandHandler` 类型名不能证明已实现命令回执、revision 与历史。

交付：Canvas、Publication 和采纳 Agent 草案统一使用经验证的 envelope/正式写入口；事件追加、幂等回执、修订推进与投影同事务；撤销用新的补偿命令或既有历史机制，保留 actor/Board/版本边界。不要删除审计触发器，也不要编辑旧 migration 来迁就新代码。

退出：连续两次 Undo、Redo、重启后继续、重复 envelope、丢失响应重放、无变化命令、共享/迁移中只读、同节点冲突均通过。别人后续写入不被自己的撤销覆盖。

### H0-08 · ICC 检测先达到安全且真实的最低线

**负责人：B。**

证据：`color_engine.rs` 当前以 profile 字节长度判断 valid；TIFF 通过搜索 acsp 猜测 profile 位置；JPEG/PNG 分段、截断和解压上限需要专项边界验证。这属于元数据检测初版，不能显示为专业色彩已完成。

交付：有界读取/解析、JPEG ICC 分片顺序及完整性、PNG iCCP 解压上限、TIFF IFD/tag 正式遍历；区分 absent/invalid/unsupported/valid，异常输入无 panic、无无界分配。检测结果与实际已执行的色彩变换分别建模。

退出：有效参考 ICC、伪 acsp、截断/异常长度/压缩炸弹/缺失分片均有边界测试；界面不会把“有 digest”标为“已做 ColorSync 变换”。

## 4. 参与角色与写入边界

以下是逻辑职责，不要求同时启动全部角色。资源少时一名 Agent 可顺序承担多个角色。首轮建议 L 加 A/C/D；B 可先提交只读诊断与原生验证设计，涉及公共接口后再接入。

| 角色 | 主要责任 | 独占文件/区域 | 首个交付 |
|---|---|---|---|
| L · 集成与 QA，由本聊天承担 | 基线、任务分配、代码评审、合入、完整门禁、验收状态 | 根 CI、公共验证脚本、证据索引、DOCS/v3；测试 stdio 桥接由 L 维护 | H0-01、总体修复顺序和候选集成 SHA |
| A · 领域/数据库/共享权威 | Asset、Canvas/Publication 命令、alias、迁移、ACL、路由 | `db.rs`、`canvas.rs`、`publishing.rs`、`commands.rs`、`lib.rs`；`apps/api/src/{database,authority,boards}.ts`、server 路由；迁移登记与公共领域契约 | H0-04/05/07，G1 领域契约 |
| B · 原生图像/外部编辑 | 有界 ICC、高位深/瓦片 PoC、原图 locator、LR/C1、回流验证、后续原生输出 | `color_engine.rs`、`image_pipeline.rs`、`photo_inspection.rs`、`external_edit.rs`、`bridges/`；新原生服务须先定接口 | H0-06/08，G2 技术验证报告 |
| C · 工作台/画布/出版 UI | Repository 适配、桌面与 Web 视图、交互、国际化、可访问性 | `src/features/canvas/`、`src/features/publishing/`、前端 services/Repository、Workspace/AppController/SideNav；Web BoardViewer 与父页面 | H0-02；G3 最小可用纵向旅程 |
| D · Agent/记忆/评测 | 工具机器契约、PolicyGate、任务草案、上下文、质量路由与评测 | `src-tauri/src/agent/`、`contracts/agent-tool-contracts.json`、提示词、`tools/agent-eval/` | H0-03；G4 越权/草案/预算回归 |

跨边界修改先在任务单列出，由文件负责人实现或明确移交；负责人移交前停止写入。A 的数据库/命令接口发生变化，B/C/D 通过版本化接口消费，不能各自新增同义表/DTO/权限检查。

`package.json`、锁文件和公共样式属于热点：先由 L 指定单一负责人；默认保持依赖。C 调整 `apps/web/src/shared.css` 时，其他 Agent 不同时做 Web 换肤。迁移文件编号由 A 统一分配；0024/0019 仅是当前观察到的下一候选编号，创建前重新盘点。

## 5. 分支、工作目录、任务单与合入

### 5.1 隔离方式

1. 每个写代码的 Agent 使用独立 worktree/checkout 与 `codex/` 前缀任务分支，例如 `codex/fr3-h0-agent-policy`。实际目录由集成人员分配，记录真实路径。
2. 禁止多个 Agent 同时在当前主 checkout 编辑并提交。之前已经出现外部提交卷入他人 WIP 的情况，这不是可接受的常态。
3. 接手先读 `git status --short`、`git rev-parse HEAD`、`git branch --show-current` 和 `git worktree list`。非空工作树先识别归属，不 reset、不把他人的文件一并 add。
4. 保持任务基线 SHA。先核对交接所需的已合入依赖，再选择最新的已验证基线；不要盲目从本文历史 SHA 开始重写新工作。
5. 一个任务一个小切片；只暂存自己列明的路径，提交前检查 staged diff。`git commit`、`git push` 等依照用户 AGENTS 的授权要求执行；既有授权只在原范围内沿用。
6. 只有 L 在获得相应授权后推进集成 main。每个 Agent 回传自己的 branch/SHA、完整依赖和证据，不能只说“已完成”。

本文若尚未提交到任务分支，先从用户附加文件或 DOCS 中的完整路径读取；不要假定新 worktree 自动包含未提交文档。任务派发时应附本文和 BaseSHA，或由 L 在相应授权范围内先将文档纳入已提交的协作基线。

### 5.2 任务认领记录

L 维护唯一任务分配记录。建议字段：

```text
TaskId / 原 FR3 ID：
Owner / Reviewer：
Branch / Worktree / BaseSHA：
目标行为和当前复现：
允许修改的路径：
禁止修改的热点：
依赖任务及其已合入 SHA：
输入/输出契约版本：
验收案例 ID 与证据目录：
已授权的外部资源/操作范围：
状态：READY / RUNNING / REVIEW / ACCEPTED / BLOCKED
阻塞原因、已完成的独立工作、解除条件：
```

同一文件同一时刻只交给一个写入者。接口等待期间，消费方可以写隔离测试、准备 fixtures、审查设计；使用 mock 的结果明确标为契约验证，不能报成真实链路验收。

### 5.3 合入顺序

- L 先完成 CI/基线修复，A 与 D 补最小权限和机器契约；当前新 API 不作为生产候选。
- A 冻结身份/命令/草案契约，先交付兼容适配，再由 C/D 接真实入口。不要把 UI、数据库、工具参数各做成三个互不兼容版本。
- 每次合入重新核对热点、契约生成物、迁移登记与锁文件；解决冲突后重跑受影响验证。
- 候选集成分支完整通过后，再检查合入 main 的对应 CI SHA。分支曾绿、父提交曾绿不等于当前 main 已绿。
- 任务完成状态与证据一起更新。没有 HUMAN/DEVICE/SUPPLIER/PROD 证据的项继续保留，不由工程 Agent 自行签字替代。

### 5.4 推荐批次与可以并行的范围

按依赖安排批次，不承诺未经估算的完成日期。各角色在同一批次内也只领取已分配任务。

| 批次 | 主线 | 可并行工作 | 批次出口 |
|---|---|---|---|
| 0 · 恢复 | L 修 CI；A 补权限/命令/非空出版；C 接入口；D 修工具；B 修解析/回流边界 | 使用冻结的最小 DTO 和隔离 fixtures；冲突接口由 A/L 当日归口 | H0 通过；新候选完整门禁；G1/G2 设计可评审 |
| 1 · 身份与色彩验证 | A 完成 G1；B 完成 G2 原生技术验证 | C 准备手势/对象树测试；D 准备草案/评测 fixtures；不切换未评审写流 | G1 不变量通过；原生路线与质量契约冻结 |
| 2 · 可操作创作链 | B 完成 G2 工程链路；C/A 完成 G3 纵向旅程 | D 在稳定 Board 草案协议上接 G4；B 依身份契约推进 G5 插件/受控回流 | G2/G3 对应工程与设备结果明确；Board 可编辑恢复 |
| 3 · 真实 Agent 与往返、协作 | D/C 完成 G4；B/A 完成 G5；A/C 扩展 G6 | 真人评测、外部编辑器和异地协作可分别组织；不互相替代验收 | G4/G5/G6 独立门槛逐项签字，未验项保留 |
| 4 · 出版闭环 | A/C/B 完成 G7 Edition、排版、原生输出和预检 | 在规格明确后组织供应商打样；L 准备 G8 安装/恢复演练 | 真实出版与交付被接受；具备集成挑战条件 |
| 5 · 毕业与发行 | 全体完成 G8 全旅程和运营验收 | 仅修验收缺陷、完善文档/观测/支持 | 最终 SHA、服务版本、签名包、证据和发布批准一致 |

G2 的校准/多屏验收、G4 的真人质量、G5 的真实软件和 G7 的印厂门槛不能被“其他批次已经开发”自动关闭。可以开发后续的隔离切片，但不能把前置能力的模拟结果作为正式合格证明。

## 6. 正式领域不变量与接口冻结清单

### 6.1 资产/版本身份

- SourceAsset 是稳定的来源身份，locator 是可变位置；同名不同 hash 不合并，同一内容多个位置保留可验证的定位关系。
- 旧 photo_id alias 保留 local/shared authority 与 project 范围。当前本地沿用 photo id、云端另建 UUID 的实际方案需明确映射，不能假定两端 asset_id 相同。
- EditVariant 与 Rendition 分别代表编辑方向与不可变结果；不能用“当前 variant”代替 manifest 指定的变体；重导出产生新 Rendition 或返回已有同一结果回执。
- Board/Sequence/Publication 是不同呈现实例。所有引用校验 project、asset、variant、rendition 归属；锁定 Edition 固定具体 Rendition。
- 未知 ICC、位深、外部版本保存 unknown，不从 1400px JPEG 或文件扩展名推造母版信息。缓存实际尺寸必须来自实际产物，不能按源尺寸除以四估算所有表达。

### 6.2 正式命令与草案

由 A 与 D 评审后冻结，至少规定：`schemaVersion`、`commandId`、`idempotencyKey`、`projectId`、实体范围、`baseRevision`、操作体和错误结构。actor 来自可信身份，不能由客户端字段自行提升权限。

正式写入原子检查 schema→身份/ACL→project/Board/asset scope→可写状态→版本→幂等→变更/审计/回执；失败不留下部分正式修改。Agent 创建 task-owned proposal，只由人类采纳命令转为正式状态。正式事件 append-only，UI preview_move 和 Presence 不提前写正式历史。

冻结具体事项：

| 契约 | 负责人 | 必须讲清楚 |
|---|---|---|
| 资产身份与 alias | A | ID 类型、跨权威映射、重复/重连、未知元数据 |
| Canvas envelope 与回执 | A | revision、幂等、undo group、冲突及 no-op 语义 |
| Agent 草案与采纳 | D + A | task/Board 所属、生命周期、Diff、局部采纳、过期基线 |
| Repository/IPC DTO | C + A | 字段命名、错误码、媒体票据、事件刷新、preview/desktop 差别 |
| Quality Provenance | B | 实际像素源、ICC/变换状态、位深、decoder/显示/校样版本 |
| External manifest | B + A | project/provider/catalog/variant/source/render、路径和文件真实性、幂等回执 |
| Shared snapshot/events | A + C | ACL 过滤、草稿/正式、断网、游标重放、撤权 |
| Edition/PrintContract | A + B + C | 冻结引用、物理单位、预检 blocker、审批及不可变交付 |

### 6.3 迁移规则

保持已提交的历史 migration 字节不变，使用 expand→backfill→dual-read/validate→switch-write→contract。任何 switch-write 都必须有回执、历史、恢复与共享权威验证。当前 0020/0022 等注释里的“DROP 即回滚”不能当作已验证回滚方案：有新数据和下游引用后，直接删表会丢失创作结果。

数据库改动先针对生成 fixture/已授权副本，并验证中断恢复、重复回填、外键/引用、旧版本和源 hash。DDL 示例不能直接在用户库或线上执行；PG 只走实际 registry，目录里仍有未登记的历史设计 SQL。

## 7. 全部 Backlog 的后续实施与退出条件

以下覆盖原始 53 项任务。表中是接续工作，不表示这些项当前已经全部完成；详细通过条件继续关联原验收矩阵。

### 7.1 G0 · 持续保持基线

| ID | Owner | 接续实施 | 退出条件 |
|---|---|---|---|
| FR3-001 | L | 新 HEAD/未提交改动、registry、模块和能力重盘点 | 当前快照指向真实源码 SHA，而非旧汇报 |
| FR3-002 | L/C | 保留真实登录/标题/语义选择器回归，加入新 UI 路径 | A001 所需 Web 旅程可复现 |
| FR3-003 | L/A | 临时 API/PG、权限与新迁移回归；修当前红 CI | 完整门禁及独立构建在同一候选 SHA 通过 |
| FR3-004 | A/L | 将旧 Photo/Sequence/Version/Comment 副本覆盖扩展到新资产引用 | A002；源/备份哈希不变，无孤儿，恢复实测 |
| FR3-005 | L | 更新架构映射、实际边界和迁移设计 | 新模块、Mock/真实路径、剩余门槛可查 |

G0 是持续基线；G1+ 发生回归时恢复它，不删除此前历史通过记录。

### 7.2 G1 · Asset Domain

依赖 H0 的契约/命令修复。先评审，再扩展；优先复用已经存在的 0020/0017 和 alias 代码。

| ID | Owner | 接续实施 | 退出条件 |
|---|---|---|---|
| FR3-006 | A/B | SourceAsset 元数据、hash、真实定位信息与 project scope | A003/A004；500 源 hash 不变，同名异源不误合并 |
| FR3-007 | A | Variant/Rendition 不可变性、唯一当前指针和可靠引用 | A005；A/B 变体互不覆盖，历史引用可打开 |
| FR3-008 | A | Board/CanvasNode 引用约束、分组归属/无环 | 同资产多节点、多 Board 互不串数据 |
| FR3-009 | A/B | 外部稳定身份、批次/状态/幂等回执 | A006；切编辑器/重复回流不变源身份 |
| FR3-010 | A | 回填游标、事务双写、alias、兼容双读/校验 | 中断/重试/重复打开无遗漏，旧草案/评论/版本可追溯 |
| FR3-011 | B/A | 多 locator、security bookmark、失联/重连与修改检测 | A007；重连必须验证 hash，不删原 Node |
| FR3-012 | A/L | Rust/TS 共享语义、客户端能力/协议版本、错误与回执 | 相同操作跨端投影一致；未知版本安全拒绝 |

G1 退出：A003–A007 及旧库兼容通过；不能只证明五张表创建成功。G2 PoC 可依据已冻结接口并行，正式读写切换由 A/L 控制。

### 7.3 G2 · Color Engine 与专业显示

| ID | Owner | 接续实施 | 退出条件 |
|---|---|---|---|
| FR3-013 | B | 修 H0-08；ICC v2/v4、灰阶/缺失/无效 profile | 输入边界测试通过；检测与变换状态分别真实表达 |
| FR3-014 | B | ImageIO/ColorSync/CoreImage 原型；16-bit/float 转换及 provenance | 与预登记参考变换数值对照；不经过低质 JPEG 中间图 |
| FR3-015 | B | 真正局部解码、瓦片金字塔、完整缓存键和取消 | C005/C009；100MP/巨幅 TIFF 真实压力，无 OOM |
| FR3-016 | B/C | Native Viewer Fit/100%/AB、多屏、软打样状态 | 实际像素缩放准确；窗口合成/输入/VoiceOver 验证 |
| FR3-017 | B | embedded/derived/native/external RAW 来源身份 | C006；所有 RAW 降级真实标示 |
| FR3-018 | B/C | Web 带 profile 的受权代理及能力提示 | 不冒充原生专业显示；浏览器/OS 兼容矩阵 |
| FR3-019 | B/L + 设备负责人 | 校准屏、颜色计、多屏、P3/AdobeRGB/ProPhoto/扫描件 | C001–C010 适用项有数值、设备/方法及预登记容差 |

先完成原生层与 WebView 合成技术验证，再选正式实现。`PipelineTarget::PrintMaster` 枚举存在不代表 print master 已能生成。没有仪器时可以推进原型和自动化，但 DEVICE 项保持未验；不能由 Agent 自拟宽松 ΔE 阈值关闭。

### 7.4 G3 · Narrative Canvas

| ID | Owner | 接续实施 | 退出条件 |
|---|---|---|---|
| FR3-020 | C/B | 独立逻辑坐标、光标锚定、pan/zoom、可见区域裁剪 | N001/N003；100+ 图、不同 DPR/滚动仍准确 |
| FR3-021 | C/A | 多选/框选、resize、锁定、分组、语义 Undo/Redo | N002；Esc/失焦取消不提交，重启与冲突可恢复 |
| FR3-022 | C/A | 矢量 ink/pressure、笔迹边界、箭头端点绑定、便签编辑 | 缩放和节点移动后不漂移；每节点正确渲染一次 |
| FR3-023 | C/A | Node/局部归一化/Board 评论锚点、回复/解决/重开 | N005；移动/裁切/隐藏后历史可追溯 |
| FR3-024 | A/C | Canvas→独立 Sequence，保留原 Board 和 rendition 引用 | N007；顺序/章节/引用及历史完整 |
| FR3-025 | C | 项目上下文导航、Agent 抽屉、对象树与键盘/辅助技术 | N006；真实用户可完成 Board 探索任务 |
| FR3-026 | B/C | Canvas 与 G2 共用高保真输出与转换定义 | Native Viewer/Canvas 同源、同参数可测一致 |

先交付“选区→Board→移动/取消→保存→重启→评论→序列”的纵向旅程。当前界面单击连线、全节点统一照片渲染、PointerLeave 当提交等行为需逐项对照交互契约，不能以类型枚举八种节点作为验收。

### 7.5 G4 · Agent on Canvas

| ID | Owner | 接续实施 | 退出条件 |
|---|---|---|---|
| FR3-027 | D/A | 接通工具、task-owned Board 草案、move/group/note/connector 等受控操作 | AI01/AI02/AI08；所有 ID 校验，正式状态未审批不变 |
| FR3-028 | D | 按资产/版本/调用 ID 保存观察证据和长链上下文 | AI03；同名工具不同照片证据不被覆盖 |
| FR3-029 | D | 能力/区域/授权/实测质量/费用路由，预算前置检查 | AI04；不扩大授权，不隐藏跨供应商切换 |
| FR3-030 | D | 多模态证据检索、反例、纠正、跨项目授权 | AI05；Agent 自产建议不变成人类偏好证据 |
| FR3-031 | C/D/A | 对话引用选区、Diff/ghost overlay、逐项采纳/拒绝 | 未批准不变正式；过期基线与撤权明确处理 |
| FR3-032 | D/L + 评测组织者 | ≥30 授权真实任务、≥10 独立摄影师/编辑、盲评与人工基线 | AI06/AI07；含失败分母、分歧及不确定性 |

元数据平均评分/同名重复检测可以保留作结构诊断，不能标为专业视觉叙事评估。模拟摄影师可用于流程演练；不计入真人样本或签字。既有 LUMIX 素材和授权记录可核对复用，素材存在不等于真人评测已完成。

### 7.6 G5 · Lightroom / Capture One 往返

| ID | Owner | 接续实施 | 退出条件 |
|---|---|---|---|
| FR3-033 | B | 真实 Lightroom 插件包、SDK manifest/回调、稳定 Catalog/Photo/Variant ID、Collection/导出流程 | R001/R002；目标版本实际加载并完成 30 张、多次发布 |
| FR3-034 | B | Capture One SDK/Recipe 能力验证及显式交接实现 | R003；目标 Catalog/Session/Variant 实际验证 |
| FR3-035 | B/A | H0-06 后接后台监测/恢复/确认 UI 和完整状态机 | R004/R005；零误归属，半文件/重放/崩溃可恢复 |
| FR3-036 | C/B | 原版/调色版/变体 A/B、待匹配与冲突人工处理 | 对应版本/ICC/匹配证据可见，失败可操作 |
| FR3-037 | A/C | 当前预览指针与冻结出版引用分开 | R006；新回流不会暗改已批准 Edition |

当前 `bridges/lightroom-classic/Info.plist` 是类 Lua 表文本，目录中没有已经核实可加载的完整插件包；Lua 的 manifest 尚缺接收端要求的渲染 SHA 等信息。交接时不要承诺“复制到 Modules 即完成”。先按官方 SDK 核对插件入口、回调、JSON、时间和真实导出结果；源文件/外部私有 Catalog 不被改写。只存在扫描函数不代表后台 watcher 已启动。

第三方 API 细节实施时查官方文档并在明确目标版本验证，不能从本交接推断函数一定存在。缺外部软件时先交付受控 fixture/manifest 处理，SUPPLIER 验收仍保留。

### 7.7 G6 · Shared Creative Workspace

| ID | Owner | 接续实施 | 退出条件 |
|---|---|---|---|
| FR3-038 | A/C | 桌面 Board→共享权威、草稿增量、正式命令与事件游标 | S001/S002；断线/重放/同 Node 冲突不静默丢写 |
| FR3-039 | A | Board/comment/draft/approve/original/press 等权限分开 | S003；登录不等于项目授权，撤权覆盖代理/命令 |
| FR3-040 | C/A | 真实 Web Board、像素票据、评论/Diff/审批、统一语义 | 完成跨端实际任务，错误/未提交状态可见 |
| FR3-041 | A | 跨实例 Presence、去重、过期连接清理/游标恢复 | S005；多 API 实例得到一致在线状态 |
| FR3-042 | L + 协作组织者 | ≥5 组真实摄影师/编辑异地双人项目 | S004；参与者/版本/反馈及网络故障证据完整 |

H0-04 的 API 闭环通过后再实施实时共编；不要先让未授权 Board 查询接入实时推送。既有 Yjs 私人笔记、旧照片评论 Presence 不能替代新 Board 的协作一致性证明。

### 7.8 G7 · Publishing Studio

| ID | Owner | 接续实施 | 退出条件 |
|---|---|---|---|
| FR3-043 | A/C | Edition/Spread/Page/Frame/文字/母版、冻结版本及真实页编辑 | P001；真实 32 页、跨页/文字/页码保存恢复 |
| FR3-044 | C/A | 展墙物理尺寸、单位、作品比例和安装图 | 尺寸可核验，布局不是普通照片网格 |
| FR3-045 | B/A | 从高分辨率母版/指定 Rendition 做高位深生产渲染 | 不读 AI/UI JPEG 放大，ICC/像素链可追溯 |
| FR3-046 | A/B/C | PrintContract 与 PPI/字体/ICC/OutputIntent/出血等预检、审批 | P002/P003/P006；blocker 阻止 press_ready；冻结输出可复核 |
| FR3-047 | L/B + 供应商 | 指定 PDF/X/微喷 TIFF、独立工具验证、实体样书/样片 | P004/P005；真实印厂/输出方接受目标产物 |

H0-05 的“生成初稿页面”只是起点。必须补 Edition Snapshot、页框指定 Rendition、实际生产渲染与交付清单；SQL 表中的 pdf_x_4 枚举、HTML 打印和预览画面均不作为合规输出证明。

### 7.9 G8 · 专业交付与发布

| ID | Owner | 接续实施 | 退出条件 |
|---|---|---|---|
| FR3-048 | L + 发行负责人 | 最终源码 SHA 的 Developer ID、Hardened Runtime、公证、装订/安装/更新回滚 | L001；Gatekeeper 与真实干净 Mac 接受 |
| FR3-049 | L/A + 运维 | 原件 locator、DB/代理/交付对象、权限和配置的备份恢复及观测 | L003；事先约定 RPO/RTO，实际恢复报告 |
| FR3-050 | A/D/L | 新 Board/回流/发布的授权、票据、撤权、路径/密钥/私密内容泄漏审查 | 未授权读取/外发/审批为 0，包含多账户/过期票据 |
| FR3-051 | L/A + 运营 | 官方 AI 账户额度、费用对账、团队/付费/发票/售后 | L004；明确可对外提供的计费/服务范围 |
| FR3-052 | 全体 + 真实参与者 | 500 素材→30 选片→Board→LR/C1→协作→32 页→印前→交付 | L002 与主计划第 16 节全部证据，开放 P0/P1 为 0 |

最终安装包必须对应通过验收的候选 SHA 和云服务版本。历史上已有签名、公证或官网安装包的记录可核对复用，但不能据此宣称新的 v3 功能已发布；也不能反过来忽略历史证据，重复要求申请已有证书。

### 7.10 G9 · 可选专业服务

| ID | Owner | 接续实施 | 退出条件 |
|---|---|---|---|
| FR3-053 | 后续单独指定 | 专业编辑/策展服务身份、报价、合同、交付和验收 | 一次真实可追溯服务交易；不占用 G1–G8 收口资源 |

## 8. 交互与生产文案的用户决策

以下用户决策优先于工程包中的通用页面样例，Agent 不得在重构时恢复已被否定的交互：

- 产品名 **FocalRoom**；使用已准备好的纯图案 logo 资产。
- 选片工作台属于项目。用户可以先打开演示项目，使用提供的照片试用；演示是正式产品能力，应标明为演示，不能混入验收任务或测试素材名称。
- AI 智能选片位于选片工作台，点击打开自然语言对话抽屉；不新建独立 AI 导航 Tab，不把核心选片功能藏在照片检查器的第二个 Tab。
- AI 设置通过抽屉标题的小设置图标进入；不要恢复“收集任务设置”这种难以理解的入口。开始 AI 选片按产品授权规则处理，普通用户不承担开发者式授权管理流程；后端仍必须执行真实范围/收件人/账户边界。
- 左下角头像/未登录状态作为账户与全局设置入口，包含语言、帮助、联系作者、退出；生产文案使用“登录”，不要“登录云端”。
- 联系邮箱 `sliqiang85@gmail.com`，Instagram `YIHUAPHOTO`。
- 移除“原生验收”“验证离线导入”“测试素材”等开发用语及自我解释段落；保留用户需要做决定的真实错误、数据后果、像素来源与显示限制。
- 原有四语言体验需要维持一致；新增主流程同步中/英/日/韩文案、日期/单位、屏幕阅读器名称。工程包的“中英首发”不能自动降低现有四语产品范围。
- 审美记忆视图的间距、搜索、按钮对齐作为回归项；不要在新增 Canvas 时破坏已认可的账户、记忆和选片交互。

专业像素来源、有效 PPI、丢失文件与同步冲突是决策信息，不能以“去工程味”为由隐藏事实；详细内部 trace 留在诊断/日志。

## 9. 可复现验证与资源协调

### 9.1 当前已存在的入口

以下命令均在 APP 执行。它们是接手时读到的现有入口，实际运行前核对当前 scripts 和工具链。新增套件必须接入验证脚本；不能创建一个 CI 从不执行的测试文件后宣布覆盖。

```bash
npm run contracts:check
npm run typecheck
npm run lint
npm test

cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" -- --skip store_load_delete_roundtrip
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings

npm run test:cloud:pg
node "apps/api/test/run.mjs" "apps/api/test/web-journey.mjs"
node "apps/api/test/run.mjs" "apps/api/test/native-promotion.mjs"

npm run verify:v2
```

独立构建另行、顺序执行：

```bash
CARGO_TARGET_DIR="target-v1" npm run tauri -- build --debug --no-sign --bundles app --config "src-tauri/tauri.v2-review.conf.json"
```

该构建只验证 Review app；正式发行见 FR3-048。Review 标识与 schema 也要在每次启动前确认隔离。

`--skip store_load_delete_roundtrip` 是既有默认避开系统 Keychain 交互的回归方式；不能把被跳过的实际 Keychain/原生登录验收记为通过。样本缺失而 ignored 的 RAW/HEIC/设备测试同理。

### 9.2 并行限制

- 500 图与既有 integration 使用 1420/1422，统一排队；多个 worktree 不等于这些端口已隔离。
- Rust 改动后必须重建 target-v1 对应 harness。不能让旧二进制替新源码通过。
- 原生打包会修改前端构建输出，与浏览器 GUI E2E 顺序执行，避免整页刷新造成假失败。
- PG 使用 `apps/api/test/run.mjs` 的全新 cluster/随机端口/每套件独立 DB；不把生产 URL 交给测试。当前 runner 的 suite 清单是显式列表，新增 Board/Asset 套件要明确登记。
- 每个 Agent 独立 evidence 命名空间，例如 `DOCS/evidence/v3-g1-asset-<date>-<owner>/`；不要让截图覆盖已通过 Gate 的历史文件。
- 重启测试用服务端完成在途 RPC 的方案；避免捕获全部错误后等待/重试，掩盖真实权限或版本失败。
- 导入/重连/移动故障只操作生成 fixture。真实照片默认只读；遗留实际素材授权按原收件人、用途和范围沿用，不泛化给所有新服务。

### 9.3 测试最小范围

| 改动 | 至少验证 |
|---|---|
| 领域/迁移 | schema+旧库副本、回填/幂等/中断、scope、审计/历史、源 hash |
| Repository/新 UI | DTO 错误、真实入口成功/失败、刷新重启、键盘/取消/撤销、截图 |
| Agent | 注册/schema、PolicyGate、真实 runtime mock 协议、草案/正式隔离、预算/幂等 |
| 云端 | 实际 PG+应用角色、两账户两项目、路由/CSRF/ACL、撤权与丢失响应 |
| 图像/回流 | 异常字节/内存上限、真实文件 hash/ICC、路径、重复、版本锁定 |
| 出版 | 非空页面生成、尺寸/引用、冻结 Edition、预检阻断、真实输出校验 |
| 集成候选 | `verify:v2`、独立 Review 构建、相同 SHA 的远端 CI、声明未覆盖项 |

测试按变化影响选择；发现新失败后针对根因补验。通过后不因追求数量反复运行无关套件。

## 10. 外部资源与需要人的验收

| 资源/决策 | 谁提供或确认 | 工程 Agent 可先完成 | 保留门槛 |
|---|---|---|---|
| G1/G2 API、身份映射与原生路线评审 | 技术负责人/集成人员，产品负责人确认影响 | ADR、可执行 PoC、参考 fixtures | 方案未评审不切正式写流 |
| 16-bit/扫描/100MP 参考文件、ICC、校准屏/颜色计 | 摄影/色彩负责人 | 有界解析、自动化、测量程序 | DEVICE 色彩及多屏实测 |
| Lightroom/Capture One 版本、合法软件和隔离 Catalog/Session | 产品/摄影负责人 | manifest/稳定身份/目录故障测试 | 真实 30 张往返与版本矩阵 |
| ≥10 评委、≥30 真实授权任务 | 评测组织者 | 盲评工具、任务冻结、失败采集 | 真人评判，不用模拟人替代 |
| ≥5 组异地双人项目 | 协作组织者 | 多会话自动化、慢网/撤权模拟 | HUMAN 真实协作 |
| 印厂/微喷规格与样书预算 | 产品负责人和供应商 | PrintContract、预检和输出实现 | SUPPLIER 实体接受 |
| 签名/公证、测试机器、发布/云环境 | 发行/运维负责人 | 本地构建、签名诊断、可审部署变更 | PROD 安装、更新、恢复及发布批准 |
| AI 费用/账户额度/真实收发 | 产品/运营负责人 | 离线记账与 mock 协议 | 真实调用按已授权用途和范围执行 |

存在外部资源门槛时，只阻塞依赖它的验收。代码和工具尚未实现的部分继续推进，不统一写成“等待用户资源”。密钥、密码、私有备份和原片不进本文件、证据包、任务提示或 Git。

## 11. 每个任务与 Gate 的交付格式

```text
Task / Gate / 原 FR3 IDs：
BaseSHA / ResultSHA / Branch / Worktree：
问题复现与完成后的用户行为：
改动文件与公共契约版本：
数据变化、兼容与回退：
自动验证：命令、环境、实际通过/失败/ignored、日志路径：
UI/原生验收：实际入口、截图/录屏、是否真实 backend：
源图 SHA 前后、是否触发外发、费用/调用记录：
DEVICE / HUMAN / SUPPLIER / PROD：分别 PASS / NOT_TESTED / BLOCKED：
已知缺陷、未覆盖点、依赖解除条件：
Reviewer / ReviewDate / 合入顺序：
下一位 Agent 的第一条可执行任务：
```

证据行遵循原计划：`Gate, CaseId, CommitSHA, TestDate, Environment, SourceHash, Expected, Observed, PassFail, EvidencePath, Operator, Caveat`。评审不只看测试数，要看测试是否经过真实调用链和成功路径。

Gate 退出需同时满足：范围内产品行为可操作；实际迁移/恢复；自动化；对应实体/真人/供应商/生产证据；公开的剩余问题。若仅工程部分通过，状态写 `ENGINEERING_READY_EXTERNAL_ACCEPTANCE_PENDING`，不得把 Gate 总状态改成全部完成。

## 12. 可直接发给其他 Agent 的启动提示

### 12.1 通用提示

```text
你参与 FocalRoom v3 协作开发。当前 Git 根目录为 /Users/yihua/Documents/film，APP 为该目录下 开发文档/contact-sheet-ai-demo/between-frames，DOCS 为该目录下 开发文档/contact-sheet-ai-demo/docs。
先读本次附加的实施交接文档；若没有附加文件，读 DOCS/v3/IMPLEMENTATION-AND-MULTI-AGENT-HANDOFF-2026-10-21.md 的实际完整路径，再读适用 AGENTS.md 和 Git 根目录下 工程宝/Engineering 的主规格/验收矩阵。本文可能尚未在你的分支中，不要因相对路径缺失就跳过。
核对实际 HEAD、branch、worktree 和未提交文件。基线不固定沿用文档旧 SHA；由集成人员明确给你当前任务路径和依赖 SHA。
只在分配的独立工作目录和任务分支内修改领取的文件；共享热点先向负责人移交，禁止在其他 Agent 的 checkout 或 main 上提交。
先写本任务复现、范围、接口、验收与回退，再完成最小真实纵向切片。保留原图、旧项目、历史 migration、审计和幂等。
遇到状态汇报与源码冲突，以代码和对应提交证据为准，不照抄“全部 Gate 完成”。不自行扩大本任务到下个 Gate。
完成后按本文第 11 节交付分支/SHA、受影响文件、真实测试日志、未验项和下一步。commit/push/生产/真实外发遵循既有明确授权；没有授权时先准备可审结果。
本次任务单：<TaskId、Owner、Worktree、BaseSHA、允许路径、依赖、验收 ID，由集成人员填写>。
```

### 12.2 A · 领域与共享权威

```text
领取 H0-04、H0-05、H0-07，并评审 G1 的资产 alias、Rendition 和命令契约。
优先修 Board 项目 ACL、缺失路由、非空出版生成和 Canvas/Publication 的正式命令/历史断点；保留 append-only 审计，不改历史 migration。
向 C 提供真实 IPC/HTTP DTO 和错误结构，向 D 提供 task-owned draft/人工采纳接口，向 B 提供受限资产匹配/不可变 rendition 记录接口。
迁移编号/registry 由你统一维护；未经明确批准不操作用户库或生产。提交前至少跑真实隔离 PG 权限、非空出版成功、连续 Undo/Redo、重放/丢失响应和旧库副本测试。
```

### 12.3 B · 原生图像与外部回流

```text
先领取 H0-06、H0-08；冻结与 A 交接的 SourceAsset/Variant/Rendition/manifest 边界。
修真实文件 hash 校验、匹配 scope、路径和半文件/重复回执；修有界 ICC 解析。将检测、色彩变换、原生显示和设备校准分别标记。
接着做 G2 原生 ICC/高位深/瓦片 PoC 与量化报告，再实施实际 LR 插件及 C1 版本验证；不把简化 Lua 或目录扫描函数作为已完成插件往返。
任何 schema、核心 IPC 注册变更由 A 接入；不能私改 Catalog 或用户母版。缺设备/软件时交工具和 fixture 证据，保留外部验收。
```

### 12.4 C · 桌面/Web 工作台

```text
先领取 H0-02，并与 A 补齐 H0-04/05 的前端真实链路。修 Workspace 对 Canvas/Publication 的 repository 注入和视图分支，补 typed Tauri Repository/DTO 与真实像素显示。
不要重写现有选片/序列和账户系统。遵循本文第 8 节用户决策：AI 在选片工作台对话抽屉、头像全局设置、正式演示项目、四语、无开发验收文案。
接着逐项实现 G3 多选/拖拽取消/缩放锚点/节点渲染/评论/Undo，补出版物真实页面编辑、Web Board 入口与错误反馈。
测试必须经过真实 backend 成功路径和重启；仅组件 mock 或截图不能宣称功能接通。公共样式/Workspace/Repository 由你单写，领域/权限交 A。
```

### 12.5 D · Agent 与质量评测

```text
先领取 H0-03：统一机器契约、project_id、canonical alias 和工具 handler；消除 evaluate_sequence 名称冲突。补 board/asset/node/placements 与 task scope，保证模型只能写 task-owned 草案。
接入 A 的人工采纳/正式命令接口；保持预算/授权前置，未知调用结果不自动重传。先测真实请求 catalog 和估算，再决定正常夹具预算，保留全部预算拒绝边界。
接着推进 G4 上下文、观察证据、多模态检索、反例记忆、Diff 和盲评工具。模拟真人只用于流程演练，不计入 30 任务/10 评委。
提交工具契约变更与上下文成本影响报告；不绕过 PolicyGate 或直接改用户正式 Board，不替产品负责人签署质量验收。
```

## 13. 本轮交接文档的生成边界

本文根据 2026-10-21 当前仓库、原计划、机器契约和已读取的远端失败日志编写。源码中可以静态确定的断点已明确列出；权限/回流/原生显示的完整运行结果由后续任务按验收复现。本文不宣称完成全量安全审计或新 Gate 验收。

本轮交付为实施文档，没有据此直接改产品代码、数据库或生产部署。下一次进入开发，应从 H0 任务单与角色文件边界开始，完成后再按第 7 节逐 Gate 推进。
