# FocalRoom v2.0 代码审查报告

审查时间：2026-10-19（针对公开仓库快照）  
仓库：https://github.com/yihua-bot/focalroom  
审查提交：`23308d6ab3`（main）  
审查方式：GitHub 连接器直接读取递归树、核心源码、工程报告、测试源码、最新 GitHub Actions 运行及日志。**未在本地克隆编译运行，也未获得当前产品可交互的渲染截图；公网工作区在本次访问工具中不可达**。因此下文严格区分“静态确认”“CI 证据”“项目自述的先前实测”和“需进一步实验验证”。仓库未修改。

## 1. 总体判断

**已经有真实工程，不是 PPT 产品，也不是单纯聊天机器人；但尚未达到“高质量摄影 Agent + 统一协作工作空间”的可商用成熟度。**

- **强项**：真实桌面 GUI 操作、Rust + SQLite 事务化领域命令、撤销/重做、任务循环/受控工具、草案审批、图像授权、账户/团队服务、Web 评审与版本提案、较丰富自动化测试。
- **短板**：缺乏摄影 Agent 对标竞品的有效外部质量证据；本地审美记忆语义能力仍弱；复杂视觉上下文压缩可能丢失必要证据；云端模型路由在正式托管策略中基本锁定一个模型；云端评论分歧分析存在确定性协议/结果类型缺陷；桌面和 Web 协作不共用统一高效视觉工作台。
- **当前门禁**：latest GitHub CI **failed**。日志表明 TS 类型检查与两个 Vite 构建通过，Web E2E 首个断言失败，后续部分回归及桌面构建未执行，不代表全部测试通过。

建议项目阶段标记为：**“功能性 Alpha；团队协作 Beta 候选；真实摄影 Agent 质量仍在验证；尚未达到商业首发”**。这是审查分级，不是实际发布状态认定。

## 2. 当前实现清单

| 子系统 | 代码事实 | 交付成熟度判断 |
|---|---|---|
| 本地选片 GUI | React 组件、照片网格、虚拟化（>=100张）、对比放大、序列 DnD、键盘命令、版本与 Undo/Redo | 功能已实现；真实工作场景性能/色彩仍待验收 |
| 桌面原生端 | Tauri v2、Rust、sqlx SQLite、图片缓存、项目文件迁移 | 功能实现；发布签名和新用户安装待验收 |
| 摄影 Agent | Rust AgentEngine 真模型多轮调用 + ToolRegistry + PolicyGate + checkpoint + 草案 | 真正 Agent 结构已实现；视觉/策展质量不能凭通过自动化来判定 |
| 模型接入 | OpenAI-compatible/DeepSeek、Anthropic/Gemini 文本适配、能力目录、质量记录 | 多适配器存在；正式账号路径固定 DeepSeek，不能等同“多模型智能路由” |
| 审美记忆 | append-only 人工证据、可纠正/删除假设、作用域与权限 | 规则引擎早期版；非成熟个人审美模型 |
| 云服务 | Node/NestJS、PostgreSQL、授权/账户、云资源、邀请、命令审计 | 多项具体路径已编码并有隔离测试；线上版本需要实测 |
| Web 协作 | 项目图片、评论锚点、序列版本、提案审批、presence、WebSocket | 审稿闭环已有；尚非与本地 GUI 一致的多人主工作台 |
| AI 评论分析 | 授权 + Worker 队列 + DeepSeek Chat Completions 调用 | 已编码，但存在确定性输入结构及结果分类缺陷，需优先修复 |
| 专业服务市场 | 讨论中的未来路线 | 未见支付评片/履约/结算完整闭环 |

## 3. 重大与高优先级缺陷（按业务风险排序）

### P0-1：摄影 Agent 的核心产品质量未通过专家实测验收

**证据**：`docs/V2-M3-EVALUATION-TEST-REPORT.md` 报告仅有一个真实摄影项目，两次运行；80 张照片中仅 12 张被模型看过，第一次失败、第二次生成 8 张草案；无真实摄影师盲评、无竞品同题比较、无跨项目记忆质量增益。`src-tauri/tests/real_model_e2e.rs` 的实际联网测试带 `#[ignore]`，不进默认 CI。

**影响**：不能证明比 Photoreka 更优秀或达到其水平；不能仅按 Agent Runtime 的存在认定有核心产品价值。

**动作**：设计 30 个真实摄影编辑任务、10 位真实目标摄影师、各任务完整图像覆盖记录、专家双盲审阅；固定模型/Prompt/成本版本，跟 Photoreka 同题比较人工采纳率与成品满意度。这个 P0 是上线决策门槛，而不是代码崩溃。

### P1-1：云评论 AI 请求消息体构造有确定性协议缺陷

**文件**：[`apps/api/src/agent-tasks.ts`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/agent-tasks.ts#L20-L105)

`buildPrompt` 返回对象 `{ system, user }`，但 `summarizeProjectComments` 把该**对象**直接放入 Chat Completions 的 `messages[1].content`，而不是使用 `prompt.user` 字符串；同时把 `messages[0].content` 固定写成普通汇总 System Prompt，忽略分歧模式的 `prompt.system`。在标准 Chat Completions 协议中，这类 content 对象不符合 text/多模态 part 的常规结构，可能导致真实模型 400，或至少使分歧策略不生效。

**动作**：用 `{role:'system',content:prompt.system}` 与 `{role:'user',content:prompt.user}`；加针对两种 kind 的真实请求 JSON Schema 和严谨 mock 断言，测试不应仅验证调用次数。

### P1-2：云 AI 分歧结果被永久存成普通汇总，UI 无法定位结果

**文件**：[`apps/api/src/agent-tasks.ts`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/agent-tasks.ts#L104-L151)，[`workers/agent/src/index.ts`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/workers/agent/src/index.ts#L85-L100)，[`apps/web/src/shared/AgentAnalysis.tsx`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/apps/web/src/shared/AgentAnalysis.tsx#L92-L125)

数据库 `storeTaskResult` 插入时 `task_type` 硬编码 `'summarize_comments'`，未接受/保存 Worker 任务 `job.kind`。Web 结果页只查找 `task_type === 'summarize_disagreement'`。即使 Worker 标记任务 COMPLETED，分歧结果也不会作为分歧分析显示。

**动作**：传递并落库 `job.kind`，补齐从授权→队列→模型→结果→Web 呈现的完整 E2E 测试。

### P1-3：ContextBuilder 对不同照片的观察进行“同名工具覆盖”，可能丢失创作证据

**文件**：[`src-tauri/src/agent/context.rs`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/context.rs#L65-L235)

当历史超过预估预算的 70%，旧 `tool` 结果仅按 `name` 去重。两次 `inspect_photos` 分别观察不同的 photo ID，第一次结果也可能被标成 superseded；随后旧结果还会被硬截取成 200 字字符串。模型不一定还能知道先前 A 组照片的关键事实，导致迭代策展不稳定。

**动作**：以 `callId + resourceIds + purpose` 标识可合并对象，仅在覆盖范围真包含旧结果或已有持久化摘要/引用时压缩。保留 photo IDs、证据引用、反例、未完成审批；构建“批量分段观察 100 张后继续选片”的回归测试。

### P1-4：上下文 token 用 JSON 字节数代替 token，易造成错误预算与过早暂停

**文件**：[`src-tauri/src/agent/context.rs`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/context.rs#L120-L150)，[`src-tauri/src/agent/policy.rs`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/policy.rs#L116-L160)

`input_estimate` 返回 `serde_json::to_vec(request).len()` 的字节数，随后 `PolicyGate` 与 `max_context_tokens` 和模型费率做直接比较。字节 ≠ token；尤其中文与结构化工具结果会造成显著偏差。它是保守近似，并非正常的 token 上界实现。

**动作**：采用对应模型 tokenizer 或 provider usage 近似结合安全裕量；记录 `estimated_input_tokens` 与 `actual_input_tokens`，改预算/压缩为按模型计算的 token 逻辑，明确图片成本不等同文本。

### P1-5：所谓“长期审美模型”主要是中文两字关键词规则

**文件**：[`src-tauri/src/agent/memory.rs`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/memory.rs#L140-L230)

`materialize` 从人工备注和比较理由抽词，仅保留 `chars().count() == 2`，相同词出现两次才生成类似“本项目多次人工判断的说明中出现「XX」”的偏好假设；支持/反例与修订机制是真实存在的，但这不是能够理解“克制、疏离、时间感、叙事节奏”及其作用域的个人审美智能。

**动作**：保留 append-only 证据账本不动；新增离线/受控模型的结构化偏好提取与证据链接，区分项目局部/用户全局假设、支持/反例/纠正、跨项目授权；用前后两轮真实摄影师操作度量“记忆增益”。不允许把 AI 草案当人类偏好证据。

### P1-6：生产托管模型路径不等于承诺的“按任务智能多模型路由”

**文件**：[`src-tauri/src/agent/openai.rs`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/openai.rs#L76-L111)，[`apps/api/src/hosted-ai.ts`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/hosted-ai.ts#L1-L38)，[`src-tauri/src/agent/router.rs`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/router.rs#L140-L185)

正式 hosted policy 固定 `deepseek-flash-v1`，后端请求固定 `https://api.deepseek.com/responses`；本地虽提供其他适配器和质量驱动路由，但 `route_for_provider` 仅在已绑定 provider 内选候选，不能把“已有三家 Adapter”当成运行时跨供应商优化。`OpenAiProvider::hosted()` 静态宣告视觉等能力、标注 `official_documentation`；真实图像/函数协议验收仍须与生产模型和端点核实。

**动作**：先用真实生产同款模型验证文字、视觉、工具回执、结构化 JSON 和单位经济；建立多模型质量数据、授权与驻留规则后，才允许自动升级/降级与故障回退；变更模型要更新身份绑定和缓存失效。

### P1-7：Web 协作图片加载是串行网络请求，48 张一页易产生等待

**文件**：[`apps/web/src/shared/ProjectReview.tsx`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/apps/web/src/shared/ProjectReview.tsx#L110-L151)

每一张可见图片先 POST 取一次票据、再 GET 取 Blob；`for...of` 内 `await` 串行执行，当前页面最多 48 张，切页等待按请求链累积。在高延迟网络下可能明显滞后（具体等待秒数需要测量）。

**动作**：并发数 4–8 的受限请求队列、缩略图优先、可取消、URL 对象释放、LruCache、局部失败重试；验收 200–500 图共享项目的 p50/p95 首屏可见时间和滚动体验。

### P1-8：GitHub CI 当前红灯，不具备稳定 release gate

**证据**：[Actions Run 39224541729](https://github.com/yihua-bot/focalroom/actions/runs/39224541729)

最新 CI 的类型检查与两个 workspace Vite 构建成功，`e2e/v2-foundation.spec.ts:11` 预期标题与新版共享审稿页标题不同而失败；预览页面还试图访问不存在的 127.0.0.1:3002 API，表明测试目标没有清晰隔离示例/登录/共享三种模式。后续 Tauri build 被跳过。

**动作**：修改测试为符合真实路由与 UI 意图的断言（不要仅为变绿而随意改产品标题）；E2E fixture 单独启动受控 mock/PG API，明确纯 Preview 和 Shared Login 的测试入口；设置 PR/主分支 CI 强制绿灯。

### P2-1：桌面与协作 Web 两套视觉工作台分裂

**文件**：`src/app/Workspace.tsx`、`src/components/contact-sheet/*`、`apps/web/src/shared/ProjectReview.tsx`（约 975 行）及 `apps/web/src/shared.css`。

桌面端是专业选片工作台，Web 端另建 235px 左图库、中央审稿、340px 右评审区的独立 UI。云端页面能批注、提案和版本审批，但没有与桌面同级别的高效拖拽编排、多图工作面板、标准快捷键和统一视觉组件模型。桌面项目升级为共享后进入云权威状态，Web 承接编辑。

**动作**：定义跨端统一 Domain ViewModel 和至少共享 ContactSheet、Inspect、Sequence、ProposalDiff 等核心组件；保留本地/云端的数据源差异，但统一视觉编辑规则；让人和 Agent 对同一共享项目工作对象形成可见提案。

### P2-2：Web 评审页面过于集中，难以持续迭代

**文件**：`apps/web/src/shared/ProjectReview.tsx` 约 975 行。

同组件承担快照拉取、预览 ticket、WebSocket、评论、协作 presence、提案编辑、审批、分页、UI 布局。高耦合容易在多人并发或移动端变化中产生回归。

**动作**：拆分数据 hooks / 领域控制器 / 展示组件；引入统一项目快照查询缓存、乐观更新和错误分级；给命令与草案独立服务层。

### P2-3：专业照片预览仍是 8bit RGB/JPEG 路线，无法证明专业色彩正确性

**文件**：[`src-tauri/src/image_pipeline.rs`](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/image_pipeline.rs#L151-L210)

现阶段缩略图为 320px，预览图最长边 1400px，使用 `into_rgb8()` 再写 JPEG。对于 16bit TIFF、ICC、广色域/HDR、特殊负片扫描，不能推断能保留专业软打样与色彩准确。只适合快速初筛，尚不能以此代替准确原图检查。

**动作**：保留高性能 proxy 但增加“原图高精度查看”路径、ICC/色彩管理、渲染链自测、RAW/EXIF 方向覆盖以及显示器真实验收。

### P2-4：Web GUI 密度与交互效率需要真人评估

`apps/web/src/shared.css` 的共享视图图库栏宽 235px、两列缩略图、辅助文字多处 9–11px；适合作为早期审稿页，不适合长期高强度摄影编辑。在 1280 与 900 以下断点的变换需要做真实可用性测试。Web 侧以每页48张分页，而桌面侧有 >=100 张虚拟化网格，产品操作范式不一致。

**动作**：建立 13/14px 起的可访问信息密度体系、非阻断 AI 辅助、键盘连续选片、缩略图密度控制、多个对比模式、拖拽序列的键盘后备与批注视觉定位校验。

### P2-5：正式发行和运维证据不齐

项目自述生产 API/Web 和邮件曾通过有限验收，但不等同于本轮可独立确认线上服务；交接文档明确官方 DeepSeek 生产路径尚未开放、正式客户端公证及公开分发未完成。R2 真桶、恢复演练、容量、告警、签名、自动更新及真实跨网验收仍需要独立检查。

### P2-6：代码、历史 Demo 与阶段文档同仓混放

主应用位于 `开发文档/contact-sheet-ai-demo/between-frames/` 深层目录；根目录包含多个代际开发包、独立官网、二进制截图和多份“当前状态”历史快照。容易让编码 Agent 按旧 README 和旧 M0 文档实施，或者误把官网模拟交互当产品功能。

**动作**：设立根目录明确 `apps/`、`packages/`、`crates/`、`docs/` 真实权威入口；设置 `docs/STATUS.md` 单一版本化进度表，标注“代码已实现/通过自动化/通过真人验收/已发布”，旧资料归档；谨慎迁移，优先不改变现有数据库/运行时路径。

## 4. 安全与隐私：已经具有的良好基础、尚未证明的能力

**静态审查确认的设计优点：**

- Agent tool 参数严格 schema 白名单；项目、photoId、草案所有权与当前任务状态在 `PolicyGate` 校验。
- Agent 先写 `draft`，人批准后通过事务提交正式版本，版本/撤销机制是同一底层。
- 向视觉云模型发送照片前，存在显式授权与上传张数/预算约束；预览缩小至约 1400px。
- 云项目有项目成员/角色、读写分离、提案、审计、令牌失效与事件重连逻辑；云评论汇总有独立模型发送授权。
- 登录凭据/密钥有 Keychain/服务端环境变量支持；没有发现要求向浏览器交付服务端统一密钥的设计。

**尚未确认：**

- 生产服务真实的授权撤销传播时间及高并发竞态，跨实例事件全部覆盖；公开项目不等于完成渗透测试。
- 所有个人创作数据的删除传播、对象存储残留与备份保留，以及访客权限边界。
- 生产模型供应商的真实数据驻留/图像协议和服务条款。
- 支付/服务市场财务与合规（功能尚未实现）。

## 5. UI / UX 详细意见（基于代码，非渲染现场评分）

**桌面 GUI 优点**：有保留/淘汰/待定、批量评分、双图比较、Loupe、DnD 顺序、撤销重做、命名版本、键盘快捷键、虚拟网格，满足专业基础操作心智模型。Agent 有独立面板、任务预算折叠区、查看草案/确认机制；非 Agent 操作可离线完成。

**桌面 GUI 重点验证**：真实 300–1000 张大体量的键盘节奏与新旧序列 Diff；“何时 AI 不打断”默认策略；高分辨率原图 100% 检查与色彩；首次导入、项目概念、用户授权文案能否 5 分钟理解。

**Web 协作优点**：预览图片上的 anchored comment、角色存在感、版本/序列批注、评论与 proposal 独立、部分采纳、离线只读状态提醒。

**Web 协作缺口**：与桌面编辑器样式和交互不统一；缩略图串行；大量 9–11px 文案；50张以上分页而非顺滑可虚拟化；Proposal 对用户的操作复杂度高；目前更适合“审稿评片”而不是“多方共编”。

**Figma 类目标验收**：摄影师与编辑在同一云权威项目上同步看到相同选择/序列/批注，任何修改可归属/比较/局部采纳/撤销；Agent 的修改表现为可复核草案而非自动覆盖；页面刷新/断连/重连不丢失未提交工作。

## 6. 建议修复迭代（不推翻现有架构）

| 批次 | 时间要求 | 交付与门禁 |
|---|---|---|
| R0 稳定性修复 | 立即 | 修复 CI 红灯、云评论 prompt + task_type 两个确定性 bug；增完整独立回归，验证真实服务能读取结果 |
| R1 Agent 内核正确性 | 下一迭代 | 修复 context 证据覆盖、token 估计；完善 Observation 引用和失败恢复；10–20 图回归任务 + 重复工具混合测试 |
| R2 Agent 质量 | 决定是否收费之前 | 真实模型同题与 Photoreka 对比：30 个任务、10 位摄影师；测采纳率、完整度、事实准确率、成本、延迟、稳定率 |
| R3 长期记忆 | Agent 质量基线后 | 从两字关键词升级到语境化审美提取；不牺牲证据/纠正/授权；验证跨项目效果 |
| R4 统一协作 GUI | 并行开展设计，R0 后开发 | 跨端共享 ViewModel 与核心视觉组件；修复 48 张串行预览；真双人参与的批注与提案旅程 |
| R5 专业画质与发行 | 封测前 | 色彩/高位深/RAW 路线、低端 Mac 性能、生产签名公证、恢复与权限测试、观测告警 |
| R6 机构与服务商业化 | 在 M3/M5 真正可用之后 | 工作室席位、学校批量审稿；最后才做付费评片、策展、摄影书服务平台 |

**注意**：上述 R0–R6 是依赖关系与交付批次，非已确定工期，修复应以实际测试证明为准。

## 7. 编码 Agent 可直接执行的第一轮任务

> 固定 `main@23308d6ab3`，先创建独立分支，不修改生产数据库、云凭据或用户图库。先复现 GitHub Actions 失败并定位 Preview/Shared 测试入口，不要通过删除测试或人为跳过解决。修复 `apps/api/src/agent-tasks.ts` 的 Chat Completions `prompt.system/user` 结构和任务结果的 `kind` 落库；添加会在旧代码必定失败的测试，覆盖总结与分歧两种模式直到 Web 呈现。修复 Rust `ContextBuilder` 对不同 photo_id 同名工具结果的不当覆盖及输入 token 估算，补充多批次照片观察的可恢复回归。先跑类型检查、lint、隔离数据库测试、Rust 全量与 Web E2E，再提交 PR，附失败前后日志与示例执行证据。未经用户明确授权，不调用生产 DeepSeek，不上传实际摄影作品，不部署或修改在线服务。

## 8. 证据索引

- [GitHub 项目](https://github.com/yihua-bot/focalroom)
- [最新 CI 失败及日志](https://github.com/yihua-bot/focalroom/actions/runs/39224541729)
- [Agent Runtime](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/runtime.rs)
- [模型路由](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/router.rs)
- [上下文工程](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/context.rs)
- [记忆规则](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/memory.rs)
- [模型云代理](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/hosted-ai.ts)
- [云端评论 Agent](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/agent-tasks.ts)
- [多人 Web 工作区](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/between-frames/apps/web/src/shared/ProjectReview.tsx)
- [2026-10-19 接手文档](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/docs/DEVELOPER-HANDOFF-2026-10-19.md)
- [真实摄影任务评测报告](https://github.com/yihua-bot/focalroom/blob/main/开发文档/contact-sheet-ai-demo/docs/V2-M3-EVALUATION-TEST-REPORT.md)
