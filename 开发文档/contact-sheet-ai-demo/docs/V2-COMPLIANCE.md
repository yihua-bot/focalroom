# v2.0 规格符合性总账（24 验收场景映射）

**2026-10-17 M3 同端点模型分工**：两个型号的观察/策展执行、独立能力/费率、观察收件人、角色冻结恢复及缓存/检索已在本地验收，新增9项Rust/3项前端；190桌面Rust+13本地云、183单测、23项E2E、独立界面与打包通过。真实成本优势/摄影质量、自动选模、跨服务Adapter/fallback和真人门槛仍开，17 pass/7 partial层级不提升。见 [本轮报告](V2-M3-MODEL-ROLES-TEST-REPORT.md)、ADR0039；M3仍partial。

**2026-10-17 M3 图像收件人授权**：实际端点/型号隔离、旧授权拒绝、陈旧界面核对、全服务禁止/单方恢复与 HTTP 不重定向已验收，新增7项Rust/7项前端用例并补真实harness禁止/重启旅程。无新真实服务质量证据、多Adapter或跨服务fallback，17 pass/7 partial层级不提升。见 [本轮报告](V2-M3-PROVIDER-CONSENT-TEST-REPORT.md)、ADR0038；M3/M02仍partial。


**2026-10-17 M3 本地作者记忆**：显式授权的 LOCAL 离线跨项目引用已接上下文与工具，来源只读/已确认/有效/相关且有支持/反例溯源；撤权、来源变更、ABA、回放和检查点重验已回归。云模型和共享项目不参与，仍没有多 Creator 身份权威；M02 保持 partial，17 pass/7 partial 的层级不提升。见 [本轮报告](V2-M3-AUTHOR-MEMORY-TEST-REPORT.md)、ADR0037。以下旧“只实现项目内”描述以本段及表内新记录为准。


**2026-10-17 M3增量索引**：已有缓存事实的真实Apple句向量已可持久化/跨连接重开复用并只重新生成变化候选；文件/模型/来源/范围校验、清空联动和迟到预取拒绝有回归。仅有界平面索引，不扩大成全图库视觉索引或高质量策展结论；未增加真人/竞品样本、云生产验证或场景pass数量。M3及17 pass/7 partial层级保持；见 [本轮报告](V2-M3-PERSISTENT-INDEX-TEST-REPORT.md)、ADR0036。下文“不持久化”是历史边界，当前文件是可清空的私密派生缓存。

**2026-10-17 M3 本地语义检索增量**：search_photos_semantic 对任务范围内已有事实做真实 Apple 中英文句向量排序；有来源、快照重验、取消/重放、不可用回退，精确原生测试已执行。无新图像上传/真人质量样本或迁移，持久化/增量语义索引和完整多供应商授权执行仍待完成。新增的本地云协作端点已进入13项云测试，Viewer只读越权已复现修复；生产部署/Postgres/WS/真实Web两端未在本轮核验，提交标题不改变验收状态。来源层级和17 pass/7 partial的总账定义不提升；见 [本轮报告](V2-M3-LOCAL-SEMANTIC-TEST-REPORT.md)、ADR0035及源码摘要。下文“无API代码/未部署”保留历史，不当作当前线上核对结论。

**2026-10-16 M3 能力配置增量**：精确模型能力配置、组合门槛、直接 Adapter 检查、能力变更恢复拒绝与 UI 授权边界已验收；151 桌面 Rust +11 云、169 Vitest、16 预览/6 后端通过，debug 构建通过。DeepSeek 无图回归 review_ready，13,907实际token/3美分估算；无新真实照片/真人质量样本。不把官方文档/协议冒烟/部署声明当专业质量或驻留保证，完整多 Adapter/供应商授权 fallback、语义索引及 M3 整体仍 partial，总账层级不变。见 [本轮报告](V2-M3-CAPABILITIES-TEST-REPORT.md)、ADR 0034。

**2026-10-16 M3 路由/检索增量**：实际实例身份/端点缓存隔离、恢复前配置核对、可观察事实检索带来源已验证；144 桌面 Rust + 11 云、164 Vitest、16 预览/6 后端全通过。DeepSeek 新 Prompt 纯文件名任务 review_ready（13,792实际token、3美分估算、0图片），未改正式项目。M03 保持本地执行层范围，不以 unknown 驻留/静态协议声明冒充能力或区域实测；M3 质量、多 Adapter、语义向量索引仍 partial。见 [本轮报告](V2-M3-ROUTING-SEARCH-TEST-REPORT.md)。

**2026-10-16 AI 模拟评审增量**：按用户授权，2 条明确来源的 AI 作者/编辑角色记录通过实际页面导出与 CLI 汇总；0 真人。真人与模拟分数/事实率严格隔离，旧来源未知不计真人，同 ID 矛盾来源排除；15 项定向测试通过。不增加真实摄影师/竞品或场景 pass 数，不把模拟意见写入偏好记忆。见 [本轮报告](V2-M3-SIMULATED-REVIEW-TEST-REPORT.md)。

**2026-10-16 M4 既有切片修复**：仓库已有 cloud/ axum/SQLite API，11 项本地真实 HTTP/进程测试验证单 grant 撤销隔离、会话/事务、有界预览和重启；全量 134 桌面 Rust + 11 云测试、159 Vitest、16 预览/6 后端和严格检查通过。S02 仍 partial：没有已部署 API/对象存储网关、生产身份或 Postgres 权威；M3 质量门槛仍开。见 [本轮报告](V2-M4-LOCAL-BOUNDARIES-TEST-REPORT.md)。以下“认证/API 完全未实现”仅保留为历史，当前是本地切片。

**2026-10-16 M3 授权素材增量**：80 张真实 JPEG、12 张有限预览观察、8 张草案已在预算内提交，80 源 hash/正式项目一致；失败保留，1 个任务/2 次调试运行，没有真人质量分。134 Rust / 159 Vitest / 16 预览 / 6 后端通过；Prompt m3-openai-5 / vision-2，费用为已核对端点的版本化估算，非账单。新增离线盲评工具不等于 M4 审稿服务。用户核对确认：腾讯云仅基础设施和空库，本项目云 API 尚未实现。M3 仍 partial，总账保持 17 pass / 7 partial 的既有层级定义；见 [本轮报告](V2-M3-EVALUATION-TEST-REPORT.md)、[剩余计划](V2-REMAINING-PLAN.md)。下方原始开放任务暂停、旧费率和旧版本声明为历史切片。

**2026-10-16 真实服务接续**：已在新门槛下执行 DeepSeek。文件名任务提交正确草案、正式项目不变（3 轮/4 工具/13,021 token）；单张生成图 Provider 协议通过（799 token）。原始开放选片仍预算暂停，不能计完成。实际 token 来源、任务 ID 与预算结算已补齐，费用仍为保守估算；见 [实测报告](V2-DEEPSEEK-TEST-REPORT.md)。以下“尚未执行真实服务”保留为上一切片历史，M3 质量与云服务缺口没有因此关闭。

**2026-10-16 复核说明**：文末历史“真实模型双通过”采用的旧测试允许暂停、等待输入、零草案及非空 JSON，不能证明可审阅任务完成或严格视觉结构。现已修复生产响应/缓存校验与验收门槛，见 [本轮报告](V2-AGENT-VALIDATION-TEST-REPORT.md)；新门槛未执行真实服务验收。主循环现有记录是预算估算，非提供方实测 token。总账仍为 17 pass / 7 partial（限各行声明的验证层级），不代表 M3 摄影质量、共享服务或产品整体完成。M1 最新格式证据见 [CR2 报告](V2-M1-CR2-PREVIEW-TEST-REPORT.md)。

日期：2026-10-15。基线：`33e5ff9` + ADR 0023 评审修复与 ADR 0024 的 M1 GUI 验收（未提交工作区）。对照 `BetweenFrames-v2.0-Implementation-Kit/tests/acceptance-matrix.json`。状态：**pass**（自动化证据）、**partial**（本地/模拟层验证，云或真模型层待外部）、**missing**（依赖未建基础设施）。

| ID | 场景 | 状态 | 证据 |
|---|---|---|---|
| G01 | AI 关闭 500 张 GUI 选片导出 | **pass**（GUI 自动化层） | m1-500.spec.ts：250 PNG + 250 JPEG 实际文件，经页面导入/滚动/选片/检查/序列/Undo-Redo/版本恢复/导出/重启；500 个源 hash 前后一致。JPEG/PNG 原图 1:1、Retina、独立原生选择器/退出重启通过；生成图库的独立原生 release 500 图功能旅程已补验，系统导入/保存、序列/版本和完整 Quit/重启通过，50 表及 500 源 hash 一致；RAW 细节、色彩、WKWebView 总资源与低端性能待补，见 [最新 M1 报告](V2-M1-NATIVE-500-TEST-REPORT.md) |
| A01 | 结果不足自主追加检索 | **pass** | agent_runtime ag01（双观察轨迹） |
| A02 | 不同反馈不同路径 | **pass** | agent_runtime ag02/ag03 |
| A03 | 越权 photo_id 拒绝 | **pass** | ag05_06_13_14 + v2_bridge（跨项目操作拒）+ PolicyGate 回归 |
| A04 | Agent 无权改正式版本 | **pass** | v2_bridge（Agent principal 纯函数层拒）+ v2_proposals（Agent 仅可 create） |
| A05 | 取消/恢复不重复草案 | **pass** | ag07（回执重放）+ ag09（检查点） |
| A06 | 同请求重试幂等 | **pass** | v2_bridge + v2_proposals：create/submit/apply/comment 重启重放、冲突指纹拒绝；业务/回执同事务，回执落库故障全部回滚 |
| P01 | 无同意不发送预览图 | **pass** | agent_vision ag04（零外发断言）+ consent 三态配额 + 实际收件人/旧授权/重定向拒绝（ADR0038） |
| P02 | 对抗文本不升权 | **pass** | ag13（注入指令不可信）+ 白名单 schema |
| M01 | 纠正后不再误用 | **pass** | agent_memory ag11 + 物理删除墓碑（ADR 0014 修订） |
| M02 | 私人记忆不跨创作者 | **partial** | 已有默认隔离及显式授权的 LOCAL 离线作者引用、来源/撤权保护；云 Provider 作者记忆授权、真实 Creator 身份与共享项目中的作者私有记忆隔离尚未实现（ADR0037） |
| M03 | 区域拒绝不静默跨区 | **pass**（本地执行层） | agent_vision 的真实 AgentEngine + 本地 HTTP 测试：区域拒绝、预算不足、缺提供方均零 HTTP；离线任务不切云；区域/档位为可信部署代码注入，配置文件与设置界面尚未接入 |
| S01 | LOCAL→SHARED 单权威 | **partial** | 本地写门槛已接入 v1/v2、导入与重连；锁内重读权威，failed/aborted 同版本可重试且保留记录。云权威、上传及真实签名确认尚未实现 |
| S02 | 撤销后 URL 拒绝 | **partial** | v2_media_links：撤销即拒（区别于过期文案）/伪造拒/跨项目签发拒/项目级批量撤销幂等；新增 cloud/ 真实 HTTP 验证单 grant 撤销/不误伤/无认证拒绝与有效 JPEG 读取；仍无生产身份/对象存储签名/CDN 部署（ADR 0022、0031） |
| C01 | Reviewer 不能批准 | **pass** | local-domain 权限样例 21 组 + proposals apply 须 can_approve |
| C02 | 并发提案基线冲突检测 | **pass**（本地领域层） | v2_proposals 的基线漂移拒绝 + v2-proposals.spec.ts 真实后端 GUI 冲突拒绝；审批整体原子并可单组 Undo/Redo |
| C03 | 断线重连版本一致 | **partial** | 本地重启一致性全套通过；CRDT 实时收敛依赖 M4 |
| C04 | 评论锚点重排后仍指原照片 | **pass** | 锚点按 photo_id 持久化（0011），排序变化不影响锚定；照片删除时锚点校验拒新引用 |
| E01 | 教师不能访问学生私人项目 | **partial** | 组织/席位/角色骨架 + 项目归属校验已测（M6 本轮）；教育隔离的云侧 ACL 依赖 M4 |
| B01 | webhook 重放不重复结算 | **partial** | v2_promotion_billing：同 event_id 幂等回执零重复行；真实服务商接入待外部（ADR 0022） |
| B02 | 伪造付款不改账本 | **partial** | v2_promotion_billing：HMAC 签名/篡改拒绝零落行；结算状态机金额匹配（ADR 0022） |
| D01 | 记忆删除后缓存/派生物失效 | **pass** | 物理删除墓碑 + FTS5 全量重建 + 观察缓存清空（ADR 0014/0019） |
| D02 | 版本回滚生成新版本 | **pass** | command_history（恢复为可撤销新编辑）+ gui_v2_m1（undo/redo 位置唯一） |
| A07 | 项目更新后草案标过期 | **pass** | ag08（stale/rebase）+ proposals 基线漂移拒 |

## 汇总

- **pass：17 / 24**（限表中明确列出的自动化层级，不表示真模型质量或云服务验收）。
- **partial：7**（M02、S01、S02、C03、E01、B01、B02）。
- **missing：0 个场景完全无基础证据**，不代表缺失模块为零：认证、Postgres、多租户授权、共享服务、实时同步、支付服务等工程仍未实现。

## 结论与剩余

v2.0 **仍处于部分实现阶段**。撤回先前“22 pass / 工程侧完成”的结论：本地协议单测不能替代远端服务、真实权限边界和指定规模的 GUI 验收。

本轮只修复既有评审问题，详见 [修复报告](V2-REVIEW-FIXES-TEST-REPORT.md) 与 [ADR 0023](ADR/0023-v2-review-transaction-boundaries.md)。M1 的 500 张 GUI 自动化层已补齐，见 [M1 报告](V2-M1-500-GUI-TEST-REPORT.md)。M1 整体仍需原生选择器、原图 1:1 与色彩、低端机和内存性能验收，暂不进入新的 M2 开发；随后再按 M2、M3、M4/M5 顺序推进；云端工程缺口不能仅归为账号或部署条件。

## 收口增量记录（2026-10-15 第二轮）

- B 项格式检视交付：TIFF/HEIC 直通 + RAW 嵌入预览标注（非全分辨率如实标示）；检视测试 7 项。
- C 项存储修复已由 ADR 0028 补正：回执保留首次业务快照并按游标补历史；旧每 500 命令基准撤回。最终基准为 1250 次成功 GUI 命令/1250 回执增量，逻辑数据库 64,733,184 字节，原始输出见当前修复报告。
- 原生 500 图验收：环境侦察完成但无视觉通道可用（无屏幕录制权限、AX 窗口不可达），保持待验收；不冒充。

- C 项色彩边界（ADR 0027）：16 位全流程 + ICC 预览丢失/原图保留行为锁定；校准显示器/HDR 待外部。

## M1 接续评审修复更新（2026-10-15）

R1–R3 已修复，见 [当前报告](V2-M1-REVIEW-FIXES-TEST-REPORT.md) 与 ADR 0028：RAW 不再无界转换或冒充嵌入预览，回执重放不替换为当前项目，基准与 E2E 次数纠正。122 功能 Rust + 1 增长基准、147 前端、27 条独立 E2E 通过。独立原生补验 3500 万像素边界图仅显示 1400×1000 派生缓存，源 hash 不变。原生 500 图、真正 RAW 细节/色彩与低端硬件仍待验收，24 场景总账保持 17 pass / 7 partial，不以本次修复宣布 M1 完成。

## M1 原生 500 图更新（2026-10-15）

生成 PNG/JPEG 图库的原生功能切片已通过，修复胶片条样式未加载的布局缺陷；本轮 120 Rust 默认、147 前端、4 M1 GUI 通过。50 表和 500 源 hash 在完整退出重启前后一致，系统保存的三个导出文件哈希与回执相符。原先“无视觉通道可用”是旧环境侦察记录，本轮通过 CUA 实际完成界面验收。资源采样只覆盖宿主主进程，不证明 WebKit 总内存、帧时延或低端机达标。RAW 细节、专业色彩和真实高像素/低端硬件仍待补，总账保持 17 pass / 7 partial，M1 整体 partial。见 [原生 500 图报告](V2-M1-NATIVE-500-TEST-REPORT.md)。

### 增量记录（2026-10-15 第四轮：原生 500 图 + CR2 内嵌预览）

- **G01 原生证据补齐**：独立 release 应用的 500 图全旅程（系统选择器导入/虚拟网格/批量选片/灯箱比较/序列 Undo-Redo/版本恢复/系统导出/完整退出重启）通过；50 表内容一致、500 源 hash 不变、Agent task=0。
- **CR2 有界内嵌 JPEG 提取**（ADR 0029）：IFD0 0x111/0x117 偏移+长度严格校验，4MiB/1600 万像素上限，八种 EXIF 方向覆盖；kind=`embedded_preview`（dataUrl 返回，不授权整个 RAW）；无 sips、无持久缓存、不冒充全分辨率。
- **回执 eventCursor**（ADR 0028）：历史业务快照+事件游标持久化，重放仅补当时事件；兼容旧完整与无游标瘦回执。
- **灯箱胶片条 CSS 修复**：从未加载的 App.css 迁移到共享 app.css。
- 本轮验证：126 Rust / 149 前端 / 4 M1 GUI（含新增 RAW 格式边界旅程）/ 16+6 E2E / Clippy / 契约全绿。

### 增量记录（2026-10-15 第五轮：真实模型 E2E）

- **M3 真实模型 E2E 通过**（`301f0c1`）：DeepSeek（OpenAI 兼容 Responses API）跑完整 Agent 任务循环——真实工具调用、草案校验、Agent 不改正式序列、API key 零泄漏全部断言通过。测试 `real_model_e2e.rs`（#[ignore]，需真实 key）。
- **工具 schema 兼容修复**：`model_specs` 补 `type: "function"` 字段——标准 Responses API 与 DeepSeek/Qwen 等 OpenAI 兼容端点必需。
- 剩余：真实视觉观察（DeepSeek deepseek-chat 已支持视觉输入，下一轮测试）、token 费用精确校准、竞品对比。

### 增量记录（2026-10-15 第六轮：真实资源接入）

- **macOS Developer ID 签名**：Developer ID Application (KXSFCZF4S3) + 时间戳 + 运行时加固 + strict 验证；zip 分发格式就绪（公证待 Apple ID）。
- **真实模型 E2E 双通过**：Agent 任务循环（needs_input 终态 + 7 工具调用）+ 视觉观察（731 tokens + 严格 JSON schema）。
- **B01/B02 真实 Polar secret**：HMAC 签发/验签/重放幂等/篡改拒绝全部通过真实 Polar webhook secret（polar_whs_...格式）验证。
- **腾讯云 PostgreSQL**：16.14 + pgvector，betweenframes 库就绪于南京服务器。
- 工具 schema 兼容修复：type: function 字段使 OpenAI provider 兼容 DeepSeek/Qwen 等全部 OpenAI 兼容端点。
