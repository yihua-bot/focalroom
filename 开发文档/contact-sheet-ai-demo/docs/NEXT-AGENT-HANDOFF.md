# FocalRoom：接手开发交接说明

## 当前接续：M4/M5 PostgreSQL 共享协作（2026-10-17）

原生续验：系统文件夹导入/共享包导出、真实迁移和只读重启已执行，50表与两张演练源图一致，发现并修复侧栏/列表的LOCAL误标及Tauri原生链接未打开问题。`open_shared_review`仅接projectId，在Rust复核完成态与绑定来源/云UUID后用macOS系统浏览器打开；不接任意URL，不经Shell，不扩大插件权限。其他原生平台显示复制地址提示。最终测试/包SHA/截图以本轮报告的新续验段为准。旧云测试Client已显式no_proxy直连回环夹具，避免系统代理造成502或转发测试凭据；保留原失败日志。

先读 [本轮验收](V2-M4-M5-SHARED-AUTHORITY-TEST-REPORT.md)、[ADR0040](ADR/0040-m4-m5-postgres-shared-authority.md)、`between-frames/apps/api/README.md`、`between-frames/ops/shared/README.md`。本轮按用户要求将可验证工程推进到 M5，M3 的真人/竞品质量门槛保持独立，M6/M7 未开展。正式服务是 `apps/api` NestJS/PostgreSQL，旧 `cloud/` SQLite `/api/v2` 安全封闭；不要将其重开或直接换连接串。保留0001，追加0002，启动只核验不自动迁移。共享命令、版本、原始envelope、事件、回执事务原子，私有草案/修订/部分批准/历史版本与WS重连均有真实本地旅程。

桌面真实迁移保存服务/用户/工作区/清单绑定，不持久化密码/token；不确定保持冻结，已提交不能取消。导出包是独立副本，不是迁移。桌面迁移后只读初始快照，后续版本在Web。私有网关票据短期单次且重查ACL，不能提供绕过撤权的裸桶链接。AI评论Task仅项目评论只读，不等于云模型执行或作者记忆授权。

可运行 `npm run review:shared`：新临时PostgreSQL、Web1430、生成演练账户及8张仓库示例预览；地址/密码由该进程输出，不读用户图库。不要把演练账号用于公网。完整脚本 `bash "scripts/verify-all.sh" --skip-keychain`，覆盖临时PG/S3、三浏览器和真实Rust Promotion；原生构建与Vite测试必须顺序执行，最终包/日志/源码清单见本轮报告。Docker daemon不可用，未构建容器、真实R2或生产部署，不能从配置文件声称已上线。

用户要求本聊天统一收口。工作期间外部HEAD从aa64110推进至24462e0；没有发现可识别的其他活动写进程，未强杀未知进程。本聊天未commit/push/reset；保留外部改动及失败日志，后续按源码SHA核验。待办是M3完整Adapter/授权/质量、生产身份/部署/备份容量、>=5真人双人项目等，详见 [剩余计划](V2-REMAINING-PLAN.md)。以下为历史。

## 最新接续：M3 同端点模型分工（2026-10-17）

先读 [本轮报告](V2-M3-MODEL-ROLES-TEST-REPORT.md)、[ADR0039](ADR/0039-m3-same-endpoint-model-roles.md) 和 [模型配置](MODEL-CAPABILITIES.md)。实际观察型号与策展型号已分别配置/冻结，能力文件不能串用；图像收件人和缓存/事实检索跟随观察身份，两个角色及能力都绑定任务恢复，累计预算仍是原任务。默认复用主实例，不增加端点或跨服务传输。Prompt m3-openai-9 / Mock2 / vision2，旧 Prompt 已执行任务需重新委托。无新 schema 或依赖。

最终190桌面Rust+13本地云、183前端、16预览/6后端/1Web和严格检查通过；独立真实harness配置界面完成两尺寸授权/全局禁止，0任务、正式项目/200测试素材hash不变。最后入口文案同步后前端与两套E2E再通过；FocalRoom Review.app 59.19MiB、标识/图标/弱链接核验通过，原生窗口和用户库未打开。252源摘要/90起始PNG一致。首轮测试定位、锁作用域和标签定位失败日志均保留。

M3仍partial。下一轮可继续完善同端点两角色的实际成本/延迟/质量评测，或另立完整多Adapter与跨服务授权设计；不要把当前能力声明/本地HTTP夹具当成质量胜出，也不要把 provider_fallback::fallback_authorized 的key检查当作用户同意。作者记忆云授权、真实Creator、>=30创作任务/专家竞品/真实摄影师门槛及既有后续阶段仍须完成。保留全部未提交改动，不提交推送或部署。下文为历史。

品牌接续（2026-10-17）：用户确定 FocalRoom 名称，原图去字/去白底的透明图案已在 public/focalroom-mark.png 接入共享侧栏、favicon 和原生图标；详见 [品牌记录](FOCALROOM-BRAND-UPDATE.md)。保留旧内部标识和历史材料。180 单测、16预览+1Web及三入口构建通过，独立 FocalRoom Review.app 打包/元数据/图标摘要通过；原生窗口与用户库未打开。用户最终视觉评审待确认；M3 仍 partial，后续优先级沿用下文。

## 最新接续：M3 实际服务图像授权（2026-10-17）

先读 [供应商授权报告](V2-M3-PROVIDER-CONSENT-TEST-REPORT.md)、[ADR0038](ADR/0038-m3-provider-bound-image-consent.md)、[剩余计划](V2-REMAINING-PLAN.md)。图像收件人绑定实际Adapter/端点摘要/型号，旧通用同意不回填；AiConfig.consentProvider 与冻结实例校验共用，原生与harness不能直接猜环境目标。正向授权须命名空间标识，openai只用于全服务禁止；换key不换目标，换端点/型号必须重授权。管理入口/取消不发起任务，委托路径携带明确consentId。HTTP不自动重定向；预留仍计真实传输和未知结果，不抹账。

全量exit=0：181桌面Rust+13云/180Vitest/16预览/6后端与严格检查；截图QA后完整前端和两套E2E再次通过；独立debug构建58.59MiB，243源码摘要/90起始PNG一致。无新迁移、依赖、真实服务请求或用户库操作；Prompt仍m3-openai-8 / m2-mock-2，缓存版本不变。定向测试夹具失败原始日志、最终日志/截图/构建元数据见evidence/v2-m3-provider-consent。

下一轮仍做M3：受控低价观察/高能力策展分工，或完整多Adapter与跨服务授权/实际fallback。现有provider_fallback::fallback_authorized只检查配置key，不是用户同意；绝不能直接用它开放跨服务数据传输。图像授权边界已有，但跨服务文本/云作者记忆同意、协议历史适配、转换后的预算/缓存/恢复、安全执行仍须独立设计和测试。保持真实Creator隔离、>=30创作任务/专家/竞品与真实摄影师门槛；不跳到新M6/M7。保留所有已有未提交/未跟踪改动，不提交推送或部署。下文为历史。


## 最新接续：M3 本地作者记忆（2026-10-17）

先读 [作者记忆报告](V2-M3-AUTHOR-MEMORY-TEST-REPORT.md)、[ADR0037](ADR/0037-m3-local-author-memory.md)、[剩余计划](V2-REMAINING-PLAN.md)。本地授权开关已接命令回执，作者只读视图与当前项目编辑视图分开，离线上下文和 global_confirmed/v2别名带来源；只读 LOCAL/已确认/有效/相关的来源。任务不可变绑定与全执行门槛重验保护撤权、ABA和来源变化，恢复不能刷新授权；无新schema。旧直接设置函数已移除，不要恢复旁路。默认Mock仍只做元数据策略，不能把接线叫作专业审美提升。

最终174桌面Rust+13云/173Vitest/16预览/6后端、严格检查和58.57MiB debug包通过；首轮UI测试错误使用同步check，修E2E后6/6通过，日志/trace/source摘要见evidence。构建必须在E2E之后，未启动原生窗口/用户库；不批量运行真实API ignored测试。当前Prompt m3-openai-8 / m2-mock-2，vision-2及向量缓存配置不变，无新DeepSeek回归。

下一轮仍收口 M3：先按既定规格实现低价观察/高能力策展的受控模型分工，或完整 Provider Adapter/供应商专属授权及实际 fallback 执行；作者记忆云传输需独立明确授权、不能复用此离线开关。真实 Creator/Workspace 身份隔离保持独立门槛。保留真实任务/专家/竞品与模拟来源分离，M3未全通过不擅自新增M6/M7。当前工作区含前几轮已有未提交/未跟踪文件，全部保留；不提交推送、部署或升级用户库。下文为历史交接。


## 最新接续：M3 持久化增量向量索引（2026-10-17）

先读 [本轮验收](V2-M3-PERSISTENT-INDEX-TEST-REPORT.md)、ADR0036。semantic_index只管有界可重建文件，semantic计算/复用，photo_search在写事务内重验并发布；完整观察/源hash/模型身份绑定，文件不是授权或事实权威。500条/8MiB/单一模型配置，语言/revision/dimensions变更重建；每次查询仍推断。清空观察命令在同一写锁先清空文件，失败不删除观察，迟到预取不可重建旧向量，回执幂等。没有全图库视觉索引/ANN/全局磁盘配额，临时残留/历史不保证擦除；语言识别可能误判，质量unmeasured。Prompt7/vision2不变，无新依赖/迁移/上传/用户库/生产操作。正式报告记录默认回归、实际原生向量/增量/重开/清空测试与源码摘要；下一块仍M3受控索引与策展分工、作者记忆执行接线及供应商特定授权/回退，不能按本地文件缓存或既有提交标题宣布阶段/云生产完成。

## 最新接续：M3 本地语义检索（2026-10-17）

先读 [本轮报告](V2-M3-LOCAL-SEMANTIC-TEST-REPORT.md)、[ADR 0035](ADR/0035-m3-local-semantic-photo-retrieval.md)。新增独立只读 search_photos_semantic：Apple 本机句向量、已有缓存事实、最多500候选/前两条事实、2工作槽/3秒协作预算，先计算后按完整摘要重验。原关键词接口保留，缺模型/非支持语言/超时明确回退；相似度不是置信度，无相关性阈值。真实中文640维/英文512维revision1的精确原生测试已执行，合成缓存不计真实摄影质量。Prompt m3-openai-7，视觉缓存不变；无新依赖/迁移/上传。现有云模块修 Viewer 只读、await 与严格检查，并强化本地13项云测试。共享目录出现其他提交/新回退辅助模块/云页面改动，按最终源码摘要核对，不把提交标题当腾讯云部署或完整功能证据。下文“无API/尚无部署”为当时记录，当前本地HTTP已存在，生产状态本轮未核验。后续仍先补M3增量索引/索引与策展分工、供应商授权执行及评测；M4/M5事务/锚点/WS/生产权威缺口见报告。最终验证与构建结果见 evidence，历史截图按起始备份恢复；未启动用户库、未提交推送或部署。

## 最新接续：M3 模型能力配置（2026-10-16）

先读 [本轮报告](V2-M3-CAPABILITIES-TEST-REPORT.md)、[配置说明](MODEL-CAPABILITIES.md)、ADR 0034。未知模型/代理默认拒绝，16KiB JSON 声明仅来自部署端；显式文件无效不能回退内置目录。来源分官方文档/既有协议冒烟/部署声明，不授予质量、驻留、费用或图片权限。Router/Adapter 共用组合要求，任务恢复绑定能力摘要；旧无摘要须新委托，密钥可轮换。实例冻结、改文件重启；Prompt 仍 m3-openai-6、视觉缓存键不变。配置错误时禁止新增委托/授权，授权卡中拒绝图像仍可用。151 桌面 Rust +11 云、169 Vitest、16 预览/6 后端通过，独立 debug 包已构建；DeepSeek 无图任务 review_ready（3轮/4工具/13,907实际token/3美分估算）。未启动用户库、未部署、无新 LUMIX 上传。M3 仍 partial；下一块继续语义索引/索引与策展分工，随后推进独立 Adapter、供应商专属授权与 fallback；真人/模拟评测门槛继续单列。

## 最新接续：M3 路由/缓存检索（2026-10-16）

先读 [本轮报告](V2-M3-ROUTING-SEARCH-TEST-REPORT.md)、ADR 0033。Prompt m3-openai-6；身份首次执行绑定到步骤/检查点，旧任务不猜绑定、不静默换配置；cached vision namespace 加端点摘要。关键词检索只读项目/task scope、当前源 hash 和版本的可观察事实，非 embedding，不写人工证据。144 桌面 Rust + 11 云、164 Vitest、16 预览/6 后端通过；DeepSeek 纯文件名真实回归 review_ready（13,792 token/3美分估算/0图片），debug 构建通过。下一步继续模型特定能力/质量实测、分层语义检索与授权 fallback；真人/模拟指标继续分开，不以协议声明作为专业质量证明，不推进生产迁移/部署。

## 最新接续：允许 AI 模拟角色继续开发（2026-10-16）

先读 [模拟评审报告](V2-M3-SIMULATED-REVIEW-TEST-REPORT.md)、ADR 0032、评测工具 README。2 AI 角色/0 真人已实际完成 UI 导出→--reviews 汇总，原 83 文件一致。新字段 reviewerKind/reviewerModel，旧缺来源 unverified；真人均值与 AI simulation 分开。模拟材料在仓库外 m3-simulated-2026-10-16T12-02-56-437Z，不能转为真人/采纳率/偏好证据；不回写原批次或正式数据库。可继续 M3 工程，不因缺真人停止全部工作；真实质量、阶段发布与云部署门槛分别保留。

## 最新接续：M4 已有切片修复（2026-10-16）

先读 [V2-M4-LOCAL-BOUNDARIES-TEST-REPORT.md](V2-M4-LOCAL-BOUNDARIES-TEST-REPORT.md)、ADR 0031。当前已有 cloud/ 本地 axum/SQLite 协议，全量通过：134 桌面 Rust + 11 云 HTTP/二进制、159 Vitest、16 预览/6 后端及严格检查；verify-all 包含 verify:cloud。单 grant 撤销、会话失效、注册事务和预览边界已修复；Postgres 需要独立实现而非换连接串，线上未部署。旧 SHA 实验凭证不兼容，禁止清库/修改用户库。未跟踪 Dockerfile 保留原样未验收。M3 真实质量门槛仍开，优先继续评测和既定 M3 工程；不因本地 M4 修复擅自新增云协作功能。

## 当前接续：M3 真实素材与离线评审（2026-10-16）

优先读 [本轮报告](V2-M3-EVALUATION-TEST-REPORT.md)、[剩余计划](V2-REMAINING-PLAN.md)、[ADR 0030](ADR/0030-m3-adaptive-output-and-evaluation.md)。用户授权 `/Volumes/LUMIX/DCIM/` 的选定 1400px 预览到 DeepSeek，并委托决定目标；前 80 JPEG 的 12 张抽样→8 张候选已预算内提交，前一结构失败仍保留。真实素材只在仓库外的 BetweenFrames-Reviews；未评分，不得代写真人结论。Prompt 最终 m3-openai-5 / vision-2；DeepSeek 费率版本与未知配置旧规则分开，不重算旧账。134 Rust / 159 Vitest / 16 预览 / 6 后端通过；原用户库不升级、密钥不落盘、不批量运行 ignored。下一步是 M3 真实分层任务/专家评测和已列工程缺口；云端当前只有 PG 基础设施与空库，没有 Between Frames API，按里程碑推进。以下为历史交接。

接续真实服务验收见 [DeepSeek 报告](V2-DEEPSEEK-TEST-REPORT.md)：文件名任务与单图 Provider 协议通过；原始开放任务仍 paused，优先修复预算内收敛，不得据此宣布 M3 通过。已交付真实任务身份、token 来源/结算、严格工具批次与错误正文隔离，提示版本 m3-openai-2。不要自动提交草案、抬高预算或用明确的元数据任务覆盖开放选片失败；继续保留全部真实运行记录。密钥只曾进入测试子进程内存，不能复制到仓库或文档。

最新接续（2026-10-16）：用户已授权继续推进。先读 [Agent 验证修复报告](V2-AGENT-VALIDATION-TEST-REPORT.md) 与 [CR2 交付报告](V2-M1-CR2-PREVIEW-TEST-REPORT.md)。有界 CR2 内嵌 JPEG 已交付，其他 RAW 仍明确回退；本轮修复现有视觉响应/缓存校验与真实模型验收假通过。不能用历史 needs_input 或非空 JSON 结果宣称 M3 通过；新门槛下真实模型尚待验收，后续重点是授权数据集和五维独立评测。保留 M1 色彩/总资源/低端设备待验项，完整 RAW 显影不扩展为 P0 阻塞项。以下旧任务指令与计数是历史交接，当前状态以上述报告为准。

当前入口：[M1 原生 500 图报告](V2-M1-NATIVE-500-TEST-REPORT.md)。生成 PNG/JPEG 图库的原生功能切片已补验，50 表/500 源 hash 重启一致，胶片条 CSS 已修复。本轮重跑 120 Rust 默认、147 前端、4 M1 GUI；累计基线的额外格式样本与其他 E2E 未重跑。下一步优先 B（真正 RAW 细节与方向/色彩矩阵）及 C（WKWebView 总资源、长期/低端性能），不再把原生 500 图功能当作未做。M1 整体仍 partial，不能进入新 M2。以下旧数字和旧配置说明为历史交接基线，以当前报告及本段为准。

更新：2026-10-15。本文件交接现有工作，不表示下列待办已经实现或验收。

## 可直接交给下一位 AI 的任务

请接手 Between Frames 当前工作区，继续补齐 **v2 M1**。先完整阅读下列入口并检查实际工作区差异，保留所有已有未提交和未跟踪文件。第一轮优先推进 **受控 RAW 细节检视与方向/色彩样本矩阵**，并补 **WKWebView 总资源和低端性能证据**；基于实测修复可复现问题、补充有意义的回归并更新报告。不要进入新的 M2/AI 服务开发。不要升级原用户库、覆盖历史迁移、清理工作区或提交推送。每轮报告实际通过项、证据与剩余问题，环境不足的部分保持 partial，不用浏览器 shim、生成图或软件 GPU 数据冒充原生、真实素材或低端硬件验收。

## 工作目录与阅读入口

真正应用根目录：

```text
/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames
```

以下相对路径均从该目录出发：

1. `AGENTS.md`、`../README.md`、`../docs/TEST-REPORT.md`。
2. `../docs/V2-M1-NATIVE-500-TEST-REPORT.md`：当前结果与剩余项；`V2-M1-REVIEW-FIXES-TEST-REPORT.md` / ADR 0028：不能回退的三项修复；`V2-M1-INSPECTION-TEST-REPORT.md` 保留历史。
3. `../docs/ADR/0025-m1-original-inspection-and-gui-projection.md`。
4. `../docs/V2-M1-500-GUI-TEST-REPORT.md`、ADR 0024：500 图基线和测试方法。
5. `../docs/V2-REVIEW-FIXES-TEST-REPORT.md`、ADR 0023：已有评审修复，禁止回退。
6. `../docs/V2-COMPLIANCE.md`、`../docs/V2.0-ADOPTION.md`：区分当前总账与历史差距快照。
7. 最新需求包 `/Users/yihua/Documents/film/BetweenFrames-v2.0-Implementation-Kit/` 中的 `docs/FINAL-PRODUCT-ENGINEERING-SPEC.md`、`docs/EXECUTION-PLAN.md` 和验收矩阵；按 M1 范围落实。旧 v1 M0–M5 完成记录不能替代新版里程碑验收。

## 历史交接基线（以当前入口为准）

- **v2 M1 仍 partial**，总账 **17 pass / 7 partial**；不是“v2 已完成”。
- 最近通过：**113 Rust、146 前端单测、26 E2E**；契约、类型、Lint、fmt、严格 Clippy、Desktop/Web 和独立原生构建通过。性能基线 1 项 ignored，钥匙串写删回环 1 项跳过。
- 500 图浏览器 GUI 连接真实 release Rust/SQLite，完成导入、虚拟滚动、决策/评分/备注、25 图序列、Undo/Redo、版本恢复、导出和不同 PID 的进程恢复；500 个源 hash 不变。**系统选择器在这组自动化里仍模拟。**
- 500 图素材是 250 PNG + 250 JPEG、1200×800，从仓库 28 张参考图派生；不是独立摄影师原片，也不是大型 RAW 集。
- JPEG/PNG 原图检视已实现：按项目/照片 ID 查所属关系和 SHA-256，限 64 MiB / 3200 万像素；点击后精确授权文件，适合屏幕继续使用缓存。Retina 下原图 100% 是物理屏幕像素 1:1。其他格式明确回退，不能把 1400px 缓存叫原图。
- GUI 的 `compact: true` 仅去掉返回副本中各历史事件的完整前后状态/上下文；数据库审计、持久化幂等回执、Undo/Redo、导出未删减。40 次编辑时返回 JSON 从 5,718,837 降至 510,106 UTF-8 字节。
- M4/10 核/24GiB 的浏览器采样：40 次编辑后 JS Heap 74.4MiB（旧基线 220.8）；最终 Renderer RSS 464.8MiB；按键→下一帧 p95 66.7ms。不是峰值、泄漏排除结论或原生/低端机性能证明；GPU 为 SwiftShader，硬件利用率未获得。
- 原生 **2 张 2400×1600 生成 JPEG/PNG** 已通过真实文件选择、文件夹选择、重复跳过、原图/同步比较、JSON 保存与完整 Quit/重启；49 表内容与源 hash 不变，Agent task 为 0。**原生 500 图功能现已补验，完整 WebView/低端性能仍待验收。**
- 上述实现和证据有大量未提交/未跟踪文件。不要只按 HEAD 判断进度；不得 `git reset`、`git clean` 或覆盖整目录。是否提交由用户另行决定。

## 后续顺序与验收标准

### A：原生 500 图功能已补验；保留复现方法

本节 1–4 的生成图库功能流程已完成，见当前报告；资源只采样到主进程，WKWebView 总资源和低端机仍待验收。修复后的应用标识为 `studio.betweenframes.m1reviewfixes`，配置 `tauri.m1-review-fixes.conf.json`，不要使用下方旧标识。

1. 使用独立评审应用和测试图库，经真实系统文件夹选择器导入 500 张。保留上轮基线，新增独立证据目录。
2. 在原生 WKWebView 检查首/中/尾虚拟网格、快速决策、批量选片/评分、灯箱、原图/双图对比、序列排序、Undo/Redo、版本恢复和导出；完整退出重启后核对状态。
3. 记录构建方式、机器、素材真实尺寸/总量、导入耗时、进程 CPU/RSS、可获得的交互延迟。原生指标若无法直接取得，要写清采样方法和限制，不能套用 Chromium CDP 数值。
4. 操作前后逐张比对 SHA-256；导出回执核对实际文件；重启确认旧进程退出、新 PID、业务数据一致。只移动测试副本来验证失联/重连。
5. 没有摄影师素材或低端设备时，可先完成生成图库的原生验收和瓶颈修复；把真实高像素素材/低端设备标为待外部验证。

### B：补齐其他格式的原图检视

- 按需求确认 TIFF、HEIC、RAW 的支持矩阵，补安全的本地解码与按需资源读取。禁止直接放开任意路径或把整个原图目录加入 asset scope。
- 超过现有检视上限时先设计受控解码/分块/缓存方案，不能仅提高常量导致双图比较无界分配。
- 验证 EXIF 方向、实际解码尺寸、失败回退、取消/翻片迟到响应和原文件不变。RAW 若只能取嵌入预览，明确标注，不能声称是全分辨率解码。
- 生成的本地解码缓存与云视觉资源保持不同用途；不得把新原图能力接入 AI 上传。

### C：色彩、长时间性能与 M1 收口

- 建立带来源说明的 ICC/方向/高位深样本；明确缓存与原图的色彩处理边界。系统 WebView 能显示图片不等于 ICC/HDR/16 位或校准显示器验收通过。
- 实测长期编辑与大项目的 SQLite/回执增长。若确有瓶颈，优先研究只读投影/按需历史，保护完整审计、旧幂等重放和 Undo/Redo；不擅自删除历史或重写旧迁移。
- 低端设备缺失是外部验收条件；不要伪造达标。更新 M1 报告/总账，逐项说明关闭项与未关闭项，经用户评审再进入新的 M2。

## 关键文件

| 范围 | 文件 |
|---|---|
| 原图所属校验/格式和上限 | `src-tauri/src/photo_inspection.rs` |
| Tauri 精确文件授权/精简返回副本 | `src-tauri/src/commands.rs`、`src-tauri/src/lib.rs` |
| 图像导入/方向/1400px 缓存 | `src-tauri/src/image_pipeline.rs` |
| GUI 仓库与延迟原图能力 | `src/services/TauriRepository.ts`、`src/domain/models.ts` |
| 显示/缩放/异步响应 | `src/components/shared/PhotoViewport.tsx`、`Lightbox.tsx`、`CompareModal.tsx` |
| 完整项目/历史/持久化回执 | `src-tauri/src/db.rs`、`command_handler.rs`、`command_audit.rs` |
| 500 图与 Retina GUI 自动化 | `e2e/m1-500.spec.ts`、`playwright.m1.config.ts` |
| 测试素材/真实后端驱动 | `src-tauri/src/harness_fixtures.rs`、`src-tauri/src/bin/m1-harness.rs`、`scripts/integration-server.mjs` |
| 新增回归 | `src-tauri/tests/photo_inspection.rs`、`src/components/shared/PhotoViewport.test.tsx`、`src/services/TauriRepository.test.ts` |
| 独立原生配置 | `src-tauri/tauri.m1-review-fixes.conf.json` |

## 验证命令与环境

在应用根目录执行；选择与改动相匹配的验证。交付里程碑前完成必需回归，记录实际输出。

```bash
npm run contracts:check
npm run typecheck
npm run lint
npm test
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" -- --skip store_load_delete_roundtrip
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
npm run test:e2e
npm run test:web
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
BF_M1_ACCEPTANCE_SLICE="v2-m1-next" npm run test:m1-gui
npm run build:workspaces
CARGO_TARGET_DIR="target-v1" npm run tauri -- build --config "src-tauri/tauri.m1-review-fixes.conf.json" --bundles app
```

- `test:integration` 与 `test:m1-gui` 共用 1420/1422，必须串行。改 Rust 后先重建对应 debug/release harness。
- 1424 是 Web 示例预览，本地测试可复用现有服务，不要杀掉用户的预览。它使用 MockRepository，不能证明原生能力或桌面原图读取。
- 当前原生评审标识为 `studio.betweenframes.m1reviewfixes`（`studio.betweenframes.m1inspection` 为旧标识，不启动），数据库位于其独立 app data；原用户库和旧 `studio.betweenframes.v2foundation` 等评审库未升级，禁止用新二进制启动那些标识。
- 新评审库已 schema 17。旧迁移 0001–0017 不可修改；若确需新迁移，先满足项目与用户的确认要求，使用临时旧库验证备份/升级/回滚，不能直接尝试用户库。
- 自动化显式禁用真实 API Key；不写删钥匙串、不联系生产模型、不伪造 GPU 或真实摄影师验收。
- 最新证据在 `../docs/evidence/v2-m1-native-500/`，浏览器回归截图在 `screenshots/v2-m1-native-500-regression/`；原生截图仅在 CUA 会话内检查，不能声称存在本地 PNG。含结果、原始日志、50 表摘要、500 源 hash 和源码 SHA-256。保留旧证据与历史截图。

## 每轮交付

可运行的实现及适当测试；新增 ADR（涉及架构时）；实际功能/性能验收结果；独立日志/截图；更新 `TEST-REPORT.md`、`V2-COMPLIANCE.md` 和 `AGENTS.md`。明确仍未验证的格式、素材、设备、色彩及外部服务条件，不把“可构建”写成“已验收”。
