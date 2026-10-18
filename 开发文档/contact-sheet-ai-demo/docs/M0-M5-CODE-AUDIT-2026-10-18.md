# M0–M5 当前代码审计（2026-10-18）

## 公网入口补验（2026-10-18）

用户报告官网部署至 Cloudflare Pages，协作域名经 Cloudflare Tunnel 接腾讯云。本轮仅打开公开页面，没有登录、提交表单、读取私有项目或修改生产配置。

- https://focroom.cc/：浏览器实际打开官网，标题“FocalRoom — 从照片，到你的作品”，显示中英日韩四个语言选项。
- https://team.focroom.cc/：浏览器实际打开的是“Web 工作台预览”，28张示例照片；并非默认登录页。
- https://team.focroom.cc/shared：实际显示共享评片登录页。
- 线上仍显示“摄影编辑 Agent”“保存我的判断”“每一次选择，都是你的表达”等旧文案；登录页未见当前工作树新增的找回密码和语言切换入口。因此当前公开前端与本地最新工作树不一致，需核对部署产物及缓存。
- 官网与协作公网HTTPS入口标记为“已只读确认可访问”。Cloudflare Pages/Tunnel与腾讯云的具体拓扑仍按用户报告记录，不能通过页面内容推断容器/数据库版本。
- 这次检查不包含真实账户邮件、私有预览、邀请/审批、WebSocket及撤权旅程，不代表整项M4/M5验收完成。
- Tunnel本身不是免备案依据。腾讯云官方“是否需要备案”明确将中国境内源站网站列入备案要求；应按实际源站与业务确认，不能用能访问替代合规结论。参考：https://cloud.tencent.com/document/api/243/19630 。

基准：Git HEAD `fa0d2b7` 加当前未提交工作树；只读核对源码、契约、CI脚本、现存验收记录与本机评审包。没有访问腾讯云、没有调用生产API或真实模型，没有重新跑全量回归，也未修改产品源码。

依据 v2 Implementation Kit 的 EXECUTION-PLAN、FINAL-PRODUCT-ENGINEERING-SPEC（尤其2.3/2.4/2.5、5.4–6.5、13–15章）。旧v1完成状态、17/24场景统计、提交标题不作为v2里程碑完成比例；24场景还含M6/M7内容。

M0底座和M2本地Mock运行时已有实现，未发现需要重建的主流程；M1、M3、M4、M5同时存在细项编码缺口及验收缺口。下表将可编码项和外部验收分开，行数只是工作分组，不是百分比或独立工期估算。

| # | 阶段 | 状态 | 剩余项 | 当前代码/证据与确切边界 | 来源 |
|---|---|---|---|---|---|
| 1 | M0 | 待验收 | 远端 CI 交付证据 | CI workflow 与全量脚本都有；当前 Git 没有 remote，gh 无法取得远端运行结果。不能宣布远端 CI 全绿。 | [scripts/verify-v2-m0.sh](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/scripts/verify-v2-m0.sh:1>)；[scripts/verify-all.sh](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/scripts/verify-all.sh:1>) |
| 2 | M1 | 待编码 | GUI 细项 | 网格目前只有状态筛选、文字搜索、密度与多选。规格要求的分组，以及独立的序列方案复制入口仍缺；已有保存/恢复版本不能写成没有版本功能。 | [src/app/Workspace.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src/app/Workspace.tsx:35>)；[src/domain/models.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src/domain/models.ts:2>)；[src/components/sequence/SequenceBoard.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src/components/sequence/SequenceBoard.tsx:1>) |
| 3 | M1 | 待编码 | 快捷键配置和命令面板 | 快捷键在 handle 中硬编码；没有自定义配置读取，也没有 ⌘K 命令面板分支。 | [src/app/useShortcuts.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src/app/useShortcuts.ts:13>) |
| 4 | M1 | 待编码 | 按操作者浏览历史 | 桌面版本对比/恢复与 Web 版本历史已有，但没有规格所列按人/Agent筛选的统一操作历史入口。 | [src/features/project/VersionsDialog.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src/features/project/VersionsDialog.tsx:1>)；[apps/web/src/shared/ReviewPanels.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/web/src/shared/ReviewPanels.tsx:185>) |
| 5 | M1 | 待验收 | 高像素、色彩和低端 Mac 性能 | 500张生成PNG/JPEG的原生旅程已有；尚缺真实高像素图库、低端设备、WebKit子进程总资源、绘制时延与专业色彩实测。缓存当前显式转RGB8，不能据此宣称16位/ICC专业流程已完成。 | [src-tauri/src/image_pipeline.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/image_pipeline.rs:188>)；[原生500图报告](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/docs/V2-M1-NATIVE-500-TEST-REPORT.md:45>) |
| 6 | M3 | 接线缺口 | 自然语言快速建项目 | QuickStartModal与parseGoal存在，但AppController没有把dialog设置为quickStart的入口；ProjectList只声明onQuickStart，未消费，也没有从AppController传入。创建成功后也没有自动StartTask。 | [src/app/AppController.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src/app/AppController.tsx:132>)；[src/features/project/ProjectList.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src/features/project/ProjectList.tsx:8>)；[src/features/quick-start/QuickStartModal.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src/features/quick-start/QuickStartModal.tsx:37>) |
| 7 | M3 | 待编码 | 免Prompt上下文动作 | 可以选区后填写任务，但没有规格所列“两图AI比较叙事价值”“一组选图生成两版顺序”的独立一键动作。现有C比较是人工CompareModal。 | [src/app/AppController.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src/app/AppController.tsx:95>)；[src/features/ai-editor/AIPanel.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src/features/ai-editor/AIPanel.tsx:125>) |
| 8 | M3 | 待编码 | 多供应商与授权fallback | 实际运行时只有Mock和OpenAiProvider；观察与策展要求同端点。Anthropic/Gemini原生Adapter、跨服务文本/图像授权及条款版本绑定、失败后授权回退未完成。 | [src-tauri/src/agent/runtime.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/runtime.rs:18>)；[src-tauri/src/agent/model_binding.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/model_binding.rs:37>) |
| 9 | M3 | 部分实现 | 质量驱动的自动选模和服务档位 | 能力/区域/预算过滤已有；quality仍unmeasured，候选按默认优先级和估算费率排序。Standard/Pro不是基于实测摄影质量的完整自动路由产品。 | [src-tauri/src/agent/router.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/router.rs:187>)；[src-tauri/src/agent/runtime.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/runtime.rs:56>) |
| 10 | M3 | 部分实现 | 全图库分层观察/索引 | 已有事实的关键词/Apple句向量检索及增量缓存已有；没有完整的全图库后台观察与低价索引到深度策展流水线。当前索引是每项目500条/8MiB的本地平面缓存。 | [src-tauri/src/agent/photo_search.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/photo_search.rs:302>)；[src-tauri/src/agent/semantic_index.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/semantic_index.rs:17>) |
| 11 | M3 | 部分实现 | 作者记忆的身份和云授权 | 项目内证据、纠正/删除、离线跨项目授权已有；跨项目设置仍绑定user_id=local，允许引用其他项目的任务仅Mock/LOCAL。真实Creator隔离、云模型作者记忆独立授权未完成。 | [src-tauri/src/agent/memory_scope.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/memory_scope.rs:34>)；[src-tauri/src/agent/memory_scope.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/memory_scope.rs:81>) |
| 12 | M3 | 部分实现 | 记忆语义检索和上下文压缩 | 偏好检索是FTS/状态/证据数排序，尚不是规格中的embedding混合检索。ContextBuilder已有分层与安全检查，但仅去除重复call字段、限制假设数；没有完整的分层token分配、历史压缩和观察归并机制。超预算暂停已实现。 | [src-tauri/src/agent/memory.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/memory.rs:352>)；[src-tauri/src/agent/context.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/context.rs:96>) |
| 13 | M3 | 部分实现 | 序列评估工具 | evaluate_draft/evaluate_sequence别名实际只从人工备注中的“重复组:”提取组，返回mode=mock_human_annotation_only；没有实际整组视觉/叙事问题评估。 | [src-tauri/src/agent/tools.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/tools.rs:217>) |
| 14 | M3 | 待验收 | 真实成本与模型切换回归 | 已有实际token记录和版本化预算费率；费用仍为估算，需要与真实账单对账，并在模型/Prompt更换后回跑同题质量基线。 | [src-tauri/src/agent/pricing.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/pricing.rs:4>)；[模型分工报告](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/docs/V2-M3-MODEL-ROLES-TEST-REPORT.md>) |
| 15 | M3 | 待外部验收 | 专业质量门槛 | 规格要求至少30个授权创作任务、至少10位目标摄影师、专家盲评/记忆增益及竞品同题（无权限须注明）。现有可核查LUMIX材料为同一80图任务的两次attempt，不能计两个任务；模拟文件2条评语均simulated_ai。 | [评测报告](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/docs/V2-M3-EVALUATION-TEST-REPORT.md:59>)；[模拟报告](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/docs/V2-M3-SIMULATED-REVIEW-TEST-REPORT.md:55>) |
| 16 | M4 | 待编码 | 真正云Agent Worker | workers/agent入口明确implemented:false，无队列/worker进程/checkpoint任务执行。agent-tasks.ts虽有模型调用函数，正式agentSummarize却在任何请求前返回MODEL_CONSENT_REQUIRED；需接独立发送授权、预算、撤权/取消、可恢复队列和草案工具。 | [workers/agent/src/index.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/workers/agent/src/index.ts:3>)；[apps/api/src/service.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/service.ts:1048>)；[apps/api/src/agent-tasks.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/agent-tasks.ts:27>) |
| 17 | M4 | 配置接线缺口 | 应用/迁移数据库角色分离 | 0007仍含密码占位符的管理员模板，正常迁移注册未执行它；Compose里API仍使用Postgres初始化账户focalroom。要完成角色创建、应用连接接线和真实权限回归。 | [apps/api/migrations/0007_min_privilege_roles.sql](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/migrations/0007_min_privilege_roles.sql:1>)；[apps/api/src/database.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/database.ts:26>)；[ops/shared/compose.yaml](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/ops/shared/compose.yaml:21>) |
| 18 | M4 | 待目标环境验收 | 部署版本、HTTPS、邮件与迁移一致性 | 账号/恢复/验证代码已存在；域名和腾讯云部署已有用户报告，不判定为不存在。本轮未访问生产；需核对运行镜像/源码版本、0001/0002/0006/0008注册迁移、HTTPS/WebSocket/Secure Cookie与真实收发件。Compose示例未向API注入邮件配置。 | [apps/api/src/database.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/database.ts:26>)；[apps/api/src/email.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/email.ts:10>)；[ops/shared/compose.yaml](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/ops/shared/compose.yaml:19>) |
| 19 | M4 | 待目标环境验收 | 私有对象存储及生产灾备 | Local与R2 Adapter均已编码，R2测试目标为本机协议服务器；需真实私有桶或经批准的替代方案、撤权与权限验证。磁盘预览/PG备份恢复已经过本地Docker演练，剩生产定时/保留/异地恢复，R2对象备份不在现脚本支持范围。 | [apps/api/test/storage.mjs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/test/storage.mjs:11>)；[ops/shared/README.md](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/ops/shared/README.md:11>) |
| 20 | M4 | 待编码/接线 | 可观测性和告警 | 未找到API分位延迟、任务成功/成本、队列、预览失败、冲突等指标/告警实现；server.ts关闭Nest logger，Nginx对API关闭access log。业务审计记录已有，不能代替运维指标。 | [apps/api/src/server.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/server.ts:224>)；[ops/shared/nginx.conf](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/ops/shared/nginx.conf:12>) |
| 21 | M5 | 部分实现 | 协作交互细项 | 照片坐标批注已有；Web添加批注始终生成type:photo，没有序列item/revision批注入口。Presence传photo_id但UI只显示头像，没有完整的他人焦点/选区展示及隐藏控制。 | [apps/web/src/shared/ProjectReview.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/web/src/shared/ProjectReview.tsx:174>)；[apps/web/src/shared/ProjectReview.tsx](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/web/src/shared/ProjectReview.tsx:268>) |
| 22 | M5 | 待编码/验收 | 跨实例Presence | 在线用户集合是attachRealtime进程内Set，广播只遍历本进程peers；尚未接共享Presence总线。PostgreSQL事件重读和单实例重连已有，不能当作多实例全部同步已完成。 | [apps/api/src/realtime.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/realtime.ts:27>)；[apps/api/src/realtime.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/apps/api/src/realtime.ts:277>) |
| 23 | M5 | 待编码 | AI分析评审分歧并生成折中提案 | 项目ACL只读评论工具已有，来源身份保留；summarize_review_disagreement显式NOT_IMPLEMENTED。完整协作AI流程依赖M4 Worker和独立模型授权，不能以评论读取接口替代。 | [src-tauri/src/agent/policy.rs](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/src/agent/policy.rs:30>)；[workers/agent/src/comments.ts](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/workers/agent/src/comments.ts:25>) |
| 24 | M5 | 待外部验收 | 真人双人项目及真实网络/负载 | 现有生成账号/三浏览器会话覆盖批注、提案、审批、冲突、撤权和重连；仍须至少5个真人双人项目，以及真实慢网/断连、多实例/容量场景。代码和AI模拟不能证明真实参与者完成。 | [共享协作报告](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/docs/V2-M4-M5-SHARED-AUTHORITY-TEST-REPORT.md>)；[v2规格](</Users/yihua/Documents/film/BetweenFrames-v2.0-Implementation-Kit/docs/FINAL-PRODUCT-ENGINEERING-SPEC.md:698>) |
| 25 | 发行 | 待制作/验收 | 当前版本正式分发包 | 本轮codesign只读检查磁盘上的FocalRoom Shared Review.app：Signature=adhoc，TeamIdentifier=not set，Sealed Resources=none。已有Developer ID历史记录不能证明这个包或最新工作树已正式签名/公证；需对最终产物签名、公证及安装/升级验收。不据此断言用户没有证书或其他已签名包。 | [src-tauri/tauri.conf.json](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames/src-tauri/tauri.conf.json:42>) |

## 已存在，不应继续列为“没做”

- Tauri/React monorepo、领域命令、DTO与契约、本地SQLite与GUI。
- 原生500张生成图库的导入/选择器/虚拟网格/选片/评分/对比/序列/Undo/Redo/版本/导出/退出重启；原图hash一致。
- 本地Agent动态循环、预算、受控工具、检查点、取消/恢复、草案与正式版本隔离；真实OpenAI兼容模型链路。
- 项目内记忆证据、纠正、排除/删除与FTS；明确授权的本地离线跨项目记忆。
- NestJS/PostgreSQL共享服务、注册/登录/注销/找回密码/邮箱验证、集中限流、Workspace/Invite/Grant。
- 桌面Promotion单权威切换、私有预览网关、Web批注/回复/解决、提案修订/部分批准/版本、单实例WS/重连与私人Yjs。
- 磁盘预览和PostgreSQL的受控备份与独立空目标恢复。本地证据9项归档边界、6类Docker演练、30项PG业务回归。
- 官网和域名不列为“没有”：存在相关工作及用户部署报告，当前审计没有验证线上生效情况。

完整RAW显影、Windows/Linux发行、支付/席位/教育和专业服务订单不新增为本次M0–M5的必做阻塞项；M6/M7另计。已有HTML Contact Sheet/印刷参考可经浏览器打印PDF，不把“没有原生PDF按钮”判成完全没有导出。

## 本次证据限制

- 当前Git没有remote；`gh run list`返回no git remotes found，所以远端CI为待核验，不判成已经失败。
- 本轮没有重跑测试。最新已有前端日志是26文件/196项通过；不能推出所有Rust/E2E/生产均通过。
- 本机唯一所列Shared Review.app是debug评审产物，签名检查不能代表用户其他位置的发行包。
- 部署配置是仓库模板；线上可能另有手工配置，需与实际运行版本核对，不能凭模板断言公网服务不存在。
- 旧V2-REMAINING-PLAN/V2-COMPLIANCE含大量历史快照；其中“账号恢复/备份脚本完全没做”等旧结论已被后续代码取代。此报告不把历史文字重复算作未完成代码。

建议顺序：先修可达入口/GUI细项及序列评价、上下文/模型/记忆链路；补云Worker与权限配置；统一最终构建与目标环境验收；并行组织M3真实质量和M5双人项目。用户评审或明确修改规格后才能移除规格中的必做项，不用增加测试数量替代完成证据。

## 收口更新（2026-10-18 下午，编码切片一）

详见 [V2-WIRING-SLICE-TEST-REPORT-2026-10-18.md](</Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/docs/V2-WIRING-SLICE-TEST-REPORT-2026-10-18.md>)。验证：前端 Vitest 229/229、API 集成 30/30、契约检查全绿、Preview E2E 16/16。

- 第 6 行（快速开始入口）：**编码完成**——ProjectList「快速开始」按钮 + quickStart 弹窗打开 + 创建后自动开 AI 面板预填目标。
- 第 20 行（可观测性和告警）：**编码完成**——/api/v2/metrics Prometheus 端点（请求计数/延迟直方图/路由模板归一/令牌保护）+ DB 派生指标（任务状态、队列深度、费用与 token 累计）+ 五条告警建议规则（README）。预览错误与协作冲突经 4xx/409 状态码计数覆盖。
- 第 21 行（序列/版本批注）：**编码完成**——契约与 API 新增 revision 锚点；Web 序列条目/当前版本批注入口与分区列表；Presence 光标展示仍在清单。
- 第 3 行（快捷键配置 + ⌘K）：**编码完成**——20 个动作可配置（localStorage + 冲突校验），⌘K 命令面板，帮助内改键。
- 第 4 行（按操作者历史）：**编码完成**——actor 贯通前后端，ActivityDialog 按人工/系统/Agent 草案/版本恢复点筛选。
- 第 2 行（网格分组 + 复制序列方案）：**编码完成**——状态/评分/文件名前缀分组；序列视图一键复制方案副本。虚拟网格在分组模式下按组渲染（单组超 100 张仍虚拟化，跨组滚动为普通流式）。
- 第 7 行（免 Prompt 上下文动作）：**编码完成**——选区「AI 比较叙事价值」「生成两版顺序」自动携带 photo_id 发起；云端图像授权流不变。
- 第 12 行（记忆混合检索/上下文压缩）：**编码完成**——BM25+本地句向量混合检索（0.6/0.4，纯语义命中折减不反超字面命中，语义不可用自动退化关键词）；分层 token 预算（系统/任务/证据/观察/历史）注入；历史压缩（超预算 70% 时旧工具结果摘要化）与观察归并（同名工具 superseded，仅压力下触发）；权威回执不受影响。
- 第 13 行（evaluate_draft）：**编码完成**——结构化本地评估（哈希完全重复防御层、观察事实相邻 Jaccard 相似、人工重复提示、覆盖/评分指标、有界 item_facts），明确叙事判断仍归模型与人工；Mock 循环同步消费新信号。
- 第 17 行（最小权限账户）：**编码完成**——0007 模板删除，init 脚本创建角色 + 0009 迁移幂等授权重放 + compose 应用/迁移角色分离连接 + migrator 迁移入口；权限回归（DML 通过、DDL/提权拒绝）进默认测试集。
- 第 22 行（跨实例 Presence）：**编码完成**——fr_presence PostgreSQL 总线（心跳/新鲜读取/断开删除/TTL），双实例集成测试互相可见；Web 显示协作者浏览照片并支持隐藏。
- 第 9 行（质量驱动自动选模）：**编码完成**——0019 盲评记录表（五维/样本量/理由/hard_fail 硬校验）+ import_quality_score 导入命令；路由注入质量记录：Pro 按均分、Standard 按性价比排序，hard_fail 排除，无记录回退默认优先级且 explain 如实标注；运行时策展与视觉路由均注入。真实盲评数据仍需真人评测产生（第 15 行门槛未变）。
- 第 16 行（云 Agent Worker）：**编码完成**——0011 授权+队列表；runWorkerOnce 租约领取/恢复/取消/失败重试；模型发送授权（额度/撤回即取消）与入队 API；compose agent-worker 服务；集成测试覆盖授权全链路、崩溃恢复、重试上限（模型走本地 mock）。Worker 仍只做评论汇总/分歧分析两类只读任务，不写正式状态。
- 第 23 行（AI 分析评审分歧）：**编码完成**——Worker summarize_disagreement（一致点/分歧/折中要点/待澄清）；Web 授权→发起→结果面板；桌面拒绝消息指向云端授权路径。
- E2E：修复并行文案/语言改动导致的 9 个既有失败（locale 固定 + 新建项目表单定位），Preview E2E 全绿。
- 未动：第 5 行性能验收、第 8/11/14/15/18/19 行（多 Adapter/云端作者记忆/成本对账/真人评测/目标环境/真实桶灾备）、外部验收行（24/25 等）。
