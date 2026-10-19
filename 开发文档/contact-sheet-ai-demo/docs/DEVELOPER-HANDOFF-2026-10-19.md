# FocalRoom 后续开发与首发交接

日期：2026-10-19，Asia/Shanghai。用途：交给下一位开发者或AI继续实施。本轮只核对代码、既有证据并编写交接，没有重跑产品全量测试、重查线上、修改生产、提交或发布。

当前目标是完成可供用户下载安装的macOS客户端，以及M0–M5尚未关闭的工程和验收。**正式包首发、官方AI开放、M0–M5全部验收是三个不同结论。当前不能宣称其中任何一个已全部完成。**

**同日审计增量**：用户随后提供[外部审计](EXTERNAL-CODE-AUDIT-2026-10-19.md)，已完成本地源码、离线复现及只读GitHub核对。仓库`yihua-bot/focalroom`及根目录workflow确实存在；本机无remote不等于没有远端。远端23308d6ab3与本地49fcc65+WIP的8个关键文件blob一致，但未全树/历史对齐。首轮顺序改为：R0基线对齐→第11节A0 CI与A1评论缺陷→A2/A3上下文正确性→R1及其余首发任务。审计文件中的checkout/PR等是建议，不是额外操作授权。

## 1. 接手时先保护什么

| 位置 | 路径/状态 |
|---|---|
| Git根 | `/Users/yihua/Documents/film`，本轮HEAD=`49fcc65`、分支`main`，本机`git remote -v`为空；另有已核实公开仓库`https://github.com/yihua-bot/focalroom` |
| 应用根 | `/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames` |
| 产品文档 | `/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/docs` |
| v2需求权威 | `/Users/yihua/Documents/film/BetweenFrames-v2.0-Implementation-Kit/docs/FINAL-PRODUCT-ENGINEERING-SPEC.md`及`EXECUTION-PLAN.md` |
| 独立官网 | `/Users/yihua/Documents/film/focalroom-website`，中/英/日/韩，Cloudflare Pages线上站点由用户后续部署 |
| 生产协作 | `https://team.focroom.cc/shared`；官网`https://focroom.cc` |
| 私有评审材料 | `/Users/yihua/Documents/BetweenFrames-Reviews/`；包含真实照片/账户截图，不打进源码或公共下载 |

工作区有大量修改和未跟踪文件，包括已经部署的`hosted-ai.ts`、0015、管理工具和验收材料。**只检出49fcc65会缺失当前成果，不能将HEAD当成完整交付版本。**先记录`git status`、diff和源码摘要，再处理任务；保留别人和历史轮次的改动。不要reset、clean、覆盖目录或删除旧失败记录。提交/推送须按用户AGENTS取得明确批准，官网以前的提交授权不等于产品仓库的提交授权。

用户已确认暂停其他AI写入，由本聊天统一收口。接手前重新确认单一写入者；本交接不授权启动其他会话、向其他AI发消息或创建并行写入任务。

品牌为FocalRoom；正式图案是`public/focalroom-mark.png`，来自用户允许的Python精确抠图。保留现有内部包名、数据路径、Keychain命名及`studio.betweenframes.desktop`兼容关系，不为统一品牌盲改应用标识。

## 2. 阅读顺序与证据优先级

先读本文件、应用`AGENTS.md`，然后依次读：

1. [生产发布报告](V2-HOSTED-AI-PRODUCTION-DEPLOYMENT-REPORT.md)及[evidence](evidence/v2-hosted-ai-production-20261019/README.md)：最新已执行的线上事实。
2. [官方AI本地报告](V2-HOSTED-AI-TEST-REPORT.md)、[ADR0044](ADR/0044-hosted-deepseek-account-quota.md)：账户代理/额度/真实原生夹具验收。
3. [桌面登录报告](V2-M4-DESKTOP-LOGIN-FIX-TEST-REPORT.md)、[ADR0043](ADR/0043-m4-desktop-browser-pkce-login.md)：PKCE、回跳、Keychain、头像。
4. [首发准备清单](PRODUCTION-RELEASE-READINESS-2026-10-19.md)：顶部增量优先，下面保留的只读快照不是最新线上状态。
5. [M4/M5共享报告](V2-M4-M5-SHARED-AUTHORITY-TEST-REPORT.md)、[备份恢复报告](V2-M4-BACKUP-RESTORE-TEST-REPORT.md)、应用`apps/api/README.md`与`ops/shared/README.md`。
6. v2需求权威、`tests/acceptance-matrix.json`，再读旧[V2剩余计划](V2-REMAINING-PLAN.md)和[符合性总账](V2-COMPLIANCE.md)的历史映射。

旧交接中“停在M1”“不存在生产API”“邮件未配置”“Worker完全未写”等是当时状态，不能复述为现在事实。提交标题、文件名、注释中的implemented与测试数量不代替执行链路和验收。

## 3. 已完成到哪一层

| 模块 | 当前代码/实际证据 | 仍不能推导的结论 |
|---|---|---|
| GUI与本地数据 | 选片、检查/比较、序列、Undo/Redo、版本、导出；生成500图原生旅程、源hash保护；有界格式检视 | 专业色彩、高像素真实图库、低端Mac/WebKit总资源已达标 |
| Agent Runtime | 生命周期、工具/权限/预算、检查点、草案与正式版本隔离、引用/幂等；真实入口创建任务已修为绑定引擎身份 | Mock成功就是摄影AI质量通过 |
| 官方AI | 桌面正式入口固定FocalRoom账户代理；API有0015账本、预留/结算/未知结果、图像校验；本地原生账户250000→249985扣15tokens通过 | 生产DeepSeek可用、真实图像/工具/费用已验证 |
| 桌面账户 | 浏览器登录→一次性code/PKCE→Keychain→账户/头像/退出；隔离原生暖/冷启动验收 | 正式安装包/正式标识在干净机器上生产回跳已验 |
| 生产邮件 | API/Web及0014/0015已部署；真实验证码仅1封delivered，用户确认在Gmail收件箱收到；网页登录/刷新/API重启/退出/已用码拒绝通过 | 密码找回、Google、头像真实OAuth等所有账户分支都已生产实测 |
| Promotion/Web协作 | 本地实际Rust→Nest/Postgres冻结/上传/确认/只读；Web评论/草案/审批/版本/WS/Presence/撤权有独立本地旅程 | 生产真实双人协作、R2/多实例/慢网规模验收完成 |
| 模型/记忆 | 质量记录、路由排序、关键词/本地Apple向量及有界缓存；Anthropic/Gemini文本适配；本地作者记忆 | 完整多供应商视觉/授权fallback、全图库观察、云作者记忆/真实Creator隔离完成 |
| 云Worker | `workers/agent`有PostgreSQL领取/租约/重试和评论汇总实现及测试 | 已按Redis+BullMQ规格验收、已接官方token账本或生产可启用 |
| 分发 | Developer ID Application有效，Team=`KXSFCZF4S3` | 最新debug评审包已签名公证、已有官网下载包 |

最近本地报告记录257前端、71PG、224Rust（15 ignored）及严格检查通过；最后部分改动另有定向复验。交接编写轮没有重新跑这些套件，不能把它们写成接手轮测试。生产7关键文件SHA与本地相符，证据见production-final.json。人工收件是后续独立证据，原服务器输出中待确认字段保留，最新`validation.json`与`human-inbox-confirmation.json`已补确认。

## 4. 生产接续必须知道的事实

以下是2026-10-19已执行部署的快照，接手后先只读重核，不当作实时监控值。

- 主机`1.13.251.21`，FocalRoom根`/opt/focalroom`，独立Compose项目`focalroom-shared-review`。不操作openyc容器、数据库、网络、DNS或官网。
- **当前运行配置**：`/opt/focalroom/releases/hosted-ai-20261019/runtime.compose.private.json`，含凭据、0600。不要复制到仓库、打印compose完整config或inspect完整Env。
- 镜像：`focalroom-hosted-ai-api:20261019` / `sha256:7585d079143e4832e679793b2211ad556e1f8b3f964a72334819a4f12ef28fa1`；Web `focalroom-hosted-ai-web:20261019` / `sha256:701567b2c00da025762352d31ede04cbc14e67365f726eb891ef2b960b8bcda3`。
- 单独用旧`/opt/focalroom/ops/shared/compose.yaml`重建会遗漏当前配置或带回旧镜像/迁移凭据；新的本地Compose又含Worker，不能盲目全栈up。当前生产只运行API/Web/Postgres，Worker关闭。
- 已登记版本：0001、0002、0006、0008、0009、0010、0011、0012、0013、0014、0015，共11项、31表。`apps/api/src/database.ts`是登记权威；启动只断言，迁移必须显式执行。
- SQL目录有同号草案：0009_magic_login、0010_author_memory等，不能排序后全量运行。0010当前登记的是presence_bus；不要改旧SQL字节或偷偷补登记同号草案。
- API常驻环境已无迁移URL；一次性DDL读取release下`migrator.private.env`。应用角色无schema CREATE及表所有权。后续发布保持此隔离，不因源码Compose仍有迁移变量而倒退。
- 私有备份在release的`backup/`，含custom dump、预览、环境、有效配置和摘要。检查了TOC/SHA，不等于生产异地全量恢复已演练。不能拿备份覆盖当前库试错。
- 现有PG容器及`focalroom-shared-review_pg-data`、`focalroom-shared-review_preview-data`均保留。禁止以down/prune/删卷排障。
- 最终快照1个已验证用户、1工作区、0摄影项目、0活动会话；AI账户/分配/请求均0。后续可能有真实用户变化，不能硬套这些断言或清回初始数。
- 发件人为`FocalRoom <noreply@openyc.vip>`，该域已验证；`focroom.cc`发件域未在本次验证列表中。邮件Key仅服务器配置；以后每次真实邮件测试另界定数量与收件人。
- DeepSeek密钥未配置，官方AI关闭，初始额度0。历史“DeepSeek真实调用”是旧开发链路，不是新账户托管生产链路。
- 最后剩余1,782,812,672 bytes，约1.66GiB。禁止为腾空间清理其他项目；应明确扩容/对象存储/配额方案。

只在后续具体版本获得批准后，才按新固定配置仅替换API/Web，保留PG。0015后旧镜像会拒绝schema，不能删除迁移登记/表强行回退；用兼容镜像关闭功能或另行批准恢复方案。

## 5. 推荐执行顺序：先完成首发链路

以下R0–R8是任务拆分，不是完成比例。R1、R2的离线测试/发布准备、R3配置与R8的本地CI修复可在缺外部资源时继续；真实调用、公证上传和公开下载必须等各自条件齐备。

### R0 · 冻结可复现源码

**输入**：当前main/49fcc65及全部WIP。**任务**：保存tracked/untracked清单、源码SHA、依赖锁文件和既有证据；核实当前本地关键文件与已部署镜像摘要。获准后按实际改动整理提交，不替别人改写提交历史。

同时核对远端`23308d6ab306778c6e79ab9e96e886e8447aa496`的导入快照；它目前不是本机可解析Git对象。8个重点文件（含根workflow）已做blob比较且相同，不等于整棵树一致，也不能据此checkout/reset覆盖本机WIP。先完成差异/来源清单，再决定接续分支与提交方式。

**完成标准**：下一位开发者拿到完整源码能复现构建；不依赖本聊天/tmp脚本、旧会话变量或开发机秘密。现有SSH辅助脚本只属于临时执行手段，不作为可移交凭据系统。不要从旧文档或聊天提取明文密码作为部署常规入口。

### R1 · 修正已验证账户的重复验证入口

**已定位**：`apps/web/src/shared-api.ts`的Session没有邮箱验证状态；`apps/web/src/shared/AccountRecovery.tsx`的VerificationRequest始终渲染按钮；`apps/api/src/server.ts`的/auth/me未返回验证状态。生产已验证账户仍显示“发送邮箱验证邮件”。

**任务**：从后端权威邮箱状态输出最小字段，经登录/刷新Session解析到Web，已验证时隐藏发送入口或呈现必要状态；不把浏览器自报状态当鉴权。按当前四语机制同步，不额外加说明卡。

**验收**：未验证/已验证/刷新/登录方式切换/请求失败有意义的回归；已验证账户不误发邮件；本地模拟邮件验证即可，不再向真实邮箱发送。API状态字段有一致契约，必要时评估桌面兼容，不无关扩张CloudUser。

### R2 · 完成官方DeepSeek真实链路及额度运营

**核心文件**：`apps/api/src/hosted-ai.ts`、0015、`tools/ai-credit.ts`/`ai-reconcile.ts`、`src-tauri/src/agent/openai.rs`/`runtime.rs`/`lifecycle.rs`/`model_binding.rs`、`cloud_auth.rs`、`src/features/ai-editor/AIPanel.tsx`。

**现状**：代码策略固定`deepseek-flash-v1`、模型`deepseek-flash`、上游`https://api.deepseek.com/responses`。这是代码事实；接手者须重新用官方资料/受控请求验证当前真实可用能力，不因字符串存在就断言供应商支持视觉/工具。

执行顺序：

1. 确认项目可用的服务端密钥安全配置位置和供应商余额；用户确定初始/测试/日常配额与单次预算。不要向桌面塞共享Key，不把旧截图Key直接拿来开生产。
2. 先在隔离服务验证最小文本、严格工具调用/结果、单张已授权1400px JPEG及真实token字段；再跑少量照片的真实Agent任务。端点/模型不支持时如实停在能力缺失，不换兼容网关或静默降级。更换策略要同步身份/授权/缓存/检查点并新委托。
3. 操作者分配用独立grant ID，先只读核对再明确`--apply`。额度单位为input+output tokens，不是人民币/美元，也不等于供应商余额；保守预留可能大于实耗，不能为跑通任意调低。
4. 验额度不足、并发上限、同请求UUID拒重发、正常结算、确定拒绝退款、不确定结果保留、换号/退出禁续跑、会话撤销、图像无授权零外发。账单和估算分开。
5. 生产调用前提供收件人/数据/图像数量/预算/账户/服务范围的可审方案并获批准。使用已有LUMIX授权也必须保持本项目、DeepSeek及选定预览范围，不扩成整卡原图外发；测试结果保存在私有材料目录。

**完成标准**：正式桌面创建的托管任务完成真实观察与工具往返、生成可审草案；服务器账本、客户端用量和供应商计量可核对；原图SHA/正式revision不变；没有明文Key、自动重试不确定请求或伪造质量分。

**故障处理**：PENDING/UNKNOWN不得自动退额重发。`ai-reconcile.ts`只在外部账单确认并有reference后人工处理，默认dry-run；活跃任务不能提前退款。云评论Worker目前不在此账本，不能一并启用。

### R3 · 正式release构建与分发配置

**入口**：`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`src-tauri/src/lib.rs`、`src-tauri/src/login_guard.rs`、`package.json`。

- 主配置仍为0.1.0、`studio.betweenframes.desktop`、仅app目标。确认发布版本及架构，建议先只交付已实际验证的macOS Apple Silicon；其他平台不能因Tauri支持就放下载。
- 增加可审查的正式打包配置/脚本，明确app及DMG/ZIP、签名身份、Hardened Runtime、时间戳、必要entitlements、最低系统版本。最低版本需从实际构建/原生API依赖验证，不凭空承诺。
- **生产构建不能带integration-harness**、评审配置/标识或`FOCALROOM_LOGIN_REVIEW_ORIGIN`的编译期覆盖。默认入口已使用`https://team.focroom.cc`与托管引擎；保留它，不改回from_env或Mock。
- 保留正式标识/现有数据及Keychain兼容；多个评审包争抢`focalroom://`已实际发生，不能靠修改本机默认关联冒充干净机器协议验收。
- 先冻结源码后构建，再记录版本/架构/commit+WIP摘要、feature、服务来源、产物大小/SHA256、Info.plist/协议声明、签名结果。二进制冻结后若改代码必须重建并重新验收。

**完成标准**：新release不依赖终端开发环境、用户从Finder打开仍连接正确生产服务；不是把100.75MiB debug评审app改名或重压缩后分发。

### R4 · 公证凭据、签名与Gatekeeper

**已证实**：Developer ID Application有效，Team KXSFCZF4S3，指纹`EFB01585E32370F1DBCF77907FF4782DBCEB847B`。不需要重新申请证书。

**未就绪**：2026-10-19默认钥匙串中查询FOCALROOM_NOTARY及BETWEENFRAMES_NOTARY均退出69/找不到条目，见[证据](evidence/production-release-readiness-20261019/notary-profile-check.json)。不排除自定义钥匙串，但没有收到路径。用户粘贴了创建方法，尚未回复已创建，不能当成执行成功。

由用户本机交互创建/提供profile或ASC Key路径；不要索要聊天明文密码。省略`--password`，让notarytool安全提示输入。钥匙串/安全代理授权弹窗由用户操作；工具禁止SecurityAgent操作时必须交接，不能绕过。

**顺序**：启用Hardened Runtime的Developer ID签名→严格验证→对冻结产物公证→Accepted记录与staple→Gatekeeper/离线票据检查。上传Apple前，向用户展示具体产物及SHA并确认高风险网络/凭据操作。仅有profile或history成功不能说明某个包已获公证。

**完成标准**：保存签名、公证submission ID/Accepted、staple validate、Gatekeeper检查及最终分发文件SHA；无跳过系统检查、清除quarantine或让用户降低安全设置。

### R5 · 首发生产容量与备份底线

**入口**：`ops/shared/backup_restore.py`/README、`apps/api/src/storage.ts`/`metrics.ts`、Nginx及实际私有runtime配置。

选择并落实磁盘扩容或真实私有对象存储、用户/项目上传及存储配额、满盘前拒绝写入/可读错误、备份保留/异地副本/告警；不要只提高请求body限制或清缓存来掩盖磁盘不足。

在独立空目标恢复生产备份副本，核对表行/权威revision/预览SHA/权限/非root读取，再销毁或保留按具体批准处理。原生产不能当恢复靶场。新runtime是单个JSON Compose，应先核对备份工具对实际服务/挂载/项目绑定，不能拿旧base配置恢复。

另有实际工具链缺口：服务器先前实测Python3.10.12，而标准`backup_restore.py`要求3.11并使用`hashlib.file_digest`。部署时临时脚本已用流式SHA适配3.10，不能据此声称标准运维脚本可直接在服务器运行。交接后选择明确Python3.11运行环境或兼容实现，并实际演练，不擅自全局升级系统Python。

`metrics.ts`未配置token时允许读取，当前生产是否另有限制未重查；上线前核对实际暴露、认证、数据库/磁盘/邮件/AI故障告警，不能因有Prometheus格式就宣称监控已接线。

**完成标准**：容量与配额有数字和压力证据、满盘边界不损坏库；独立恢复能运行且留原始日志；日常运维不把DDL凭据放回API，不清其他项目资源。

### R6 · 官网真实下载入口

**入口**：独立官网`dist/config.js`、`dist/app.js`、`dist/index.html`、`locales/messages.tsv`、`scripts/`。本轮本地config的workspaceUrl仍为空；线上空值来自此前生产审计，发布前再核实。

在真实工作区HTTPS及合格安装包具备后接入：四语工作区/下载入口、版本/日期/架构/最低系统、更新记录、SHA256、实际下载对象。公开安装包与私有摄影预览分开访问策略；大包选择合适对象存储，核对现有托管限制，不能盲目塞进静态托管。

用户明确不要“加入内测”、申请表、邮箱收集；不重新加这些入口。官网演示是本地预设，不调用AI，不伪造成真实客户案例。四语词表是源，`dist/locales.js`由build生成。

**验收**：官网build/check/test、四语及移动端浏览器操作、HTTPS下载文件SHA/文件名/平台匹配，无包时不显示虚假下载成功。官网/DNS/Cloudflare上线是独立外部变更，具体版本另获批准。

### R7 · 从真实下载到生产协作的全新环境验收

使用最终下载对象，在干净Mac或独立测试用户环境验证：

`官网下载→系统默认Gatekeeper打开→Finder启动→新项目/系统选图→真实AI草案→人工批准→浏览器登录→正式scheme暖/冷回跳→共享迁移→另一真实账户Web评片/提案/审批→完整退出重启→撤权/退出`。

记录原图SHA、正式版本、任务/供应商/用量、身份/权限、产物SHA、失败与恢复。旧数据升级只用副本，测试默认不启动正式用户库。不要把1442回环review包、浏览器IPC shim、直接API请求或清除安全属性当作这条验收。

**完成标准**：交付同一个冻结安装包贯穿全程的证据；无严重数据丢失、越权、重复扣费/任务；缺真实AI时不能宣称“官方AI首发可用”。

### R8 · 可用的远端CI与交付归档

本机Git配置无remote，但已存在公开仓库和根`.github/workflows/betweenframes-v2.yml`：macos-14、Node版本文件、Rust1.94.0、正确工作目录均已配置。已核实远端run39224541729失败，步骤“M0与既有能力回归”失败后桌面构建跳过，详细首轮修复见第11节A0。不要再把“完全没有远端CI”作为现状。

产品子目录还存在另一份`.github/workflows/ci.yml`，其中Node22、错误健康路径、类型检查吞错等是旧配置问题，不能与当前根workflow混为一个文件。需确认保留/归档策略和真实覆盖范围。

对齐本地与已知远端仓库，修当前E2E目标和后续覆盖；PG用隔离测试runner，禁真实密钥，让错误真实失败。配置本机remote、提交/推送和PR仍按用户要求获批准，不重复询问已经明确的仓库地址，也不伪造通过链接。

**完成标准**：远端实际run可追溯源码，必要类型/Lint/契约/单测/PG/构建失败会阻断；公开发版产物对应该冻结来源。

## 6. 到M5仍要继续的工程，不被首包公证替代

| 编号 | 已有基础与明确缺口 | 下一步与完成标准 |
|---|---|---|
| M1-A | 原生生成500图/有界CR2预览已做；专业色彩、真实高像素/低端设备/WebKit总资源未闭环 | 按现有M1报告测格式/方向/ICC及设备资源，用实测驱动修复；内嵌预览不冒充完整RAW显影，宿主RSS不当WebKit总内存 |
| M3-A | `author-memory.ts`存在，但查询cloud_projects；旧0010_author_memory外键UUID指向该表，当前登记迁移没有它；API无该模块路由/接线 | 重新设计兼容fr_*权威的Creator归属与显式云模型记忆授权，新增未占用迁移，不改0010_presence；接API/任务上下文/撤销/缓存/恢复，真实两账户跨Workspace测试通过。不能把这个文件存在算完成 |
| M3-B | 本地memory_scope只允许local作者，may_use_author限Mock与LOCAL；登录账户绑定只覆盖当前托管任务 | 不将历史本地偏好自动归给当前登录者；明确本地→Creator归属/迁移同意；共享成员不能默认读作者私有记忆，纠正/排除/墓碑后旧缓存及在途回复不能重引 |
| M3-C | Anthropic/Gemini原生文本/工具适配及测试已写，能力明确无vision；当前正式桌面只走托管DeepSeek | 完整视觉格式/真实协议与能力验收、供应商特定文本/图像/记忆同意、受控fallback需独立切片。现有换供应商须新委托的安全规则不能悄悄解除 |
| M3-D | quality.rs/agent_quality测试与路由排序存在；当前任务冻结单实例，非已部署多候选模型池 | 用真实质量记录验证选择路径、能力/授权/区域/预算过滤；允许切换时同步身份/条款/缓存/恢复，不以模拟高分或目录排序宣称自动选模质量胜出 |
| M3-E | 本地Apple向量只检索已有事实，500条/8MiB缓存；不是全图库后台观察 | 对授权范围内分层观察/索引制定有界队列、增量/撤权/源hash失效/预算/停止策略，避免300原图整批送强模型。无需为了规格字样先加ANN |
| M3-F | Context按工具名覆盖旧结果，并把旧正文截为200字符；输入估算用JSON字节；偏好物化仍主要两字词规则 | 先按第11节A2/A3保住照片/证据引用与合理估算，再做有来源的结构化偏好提取和记忆增益评测；不能只因数据库有完整回执就认定模型上下文没有丢信息 |
| M4-A | Worker已有Postgres租约队列，不是Redis+BullMQ；独立模型Key/配置、与官方token账本分离，生产关闭 | 先审计执行时ACL/项目/创建者/consent收件人和模型匹配、外发前预算、撤销、结果幂等；不确定外部请求不能自动重发/重复扣款。决定复用Postgres并写ADR，或按需要补规格队列；不因源码有服务就启用生产 |
| M4-B | R2适配及本地S3测试存在，真实R2与代理/撤权/灾备/规模证据不足 | 私有桶→网关→单次票据→当前ACL重验，撤权后不能获得新像素/评论；不承诺收回已下载内容；按R5做真实目标备份恢复/容量 |
| M5-A | 本地事务审批/并发冲突/版本/WS重连/Presence已做 | 生产两角色及慢网/断连/多实例/重复请求/并发审批/撤权完整验证，不能只测静态UI；过期提案保留原文，正式版本只由权威事务变更 |
| M5-B | Web当前票据POST与Blob GET按图串行，最后一次性setImages；对象URL已有基础释放 | 有界并发/逐步显示/取消/局部失败/撤权清缓存，保留私有票据与当前ACL检查；真实延迟指标尚需测，不虚构加载秒数 |

云作者记忆的重点测试：A/B不同Creator、同一Creator不同Workspace、协作者角色变化、撤权后任务恢复、证据纠正/删除/排除、旧本地作者身份未绑定、跨项目/缓存/导出泄漏；客户端不能通过自填creator_id获得权力。当前源码的revokeMemoryAccess只验证身份后返回true，没有持久撤权，不可开放为完成接口。

云Worker规格差异和模型fallback都需要ADR/验收更新，不为了“全做完”把安全禁用门槛去掉。M6团队/教育/支付和M7订单不在本轮范围；旧骨架文件不能算它们已上线。

## 7. 真人与外部资源任务

**M3质量**：至少30个获授权创作任务，每项80–300张，目标/分层/模型/Prompt/全部attempt/成本/源hash完整；至少10位真实摄影师，实际作者及独立评审、首次/第三次记忆增益、专家同题/竞品对照按规格记录。一个LUMIX目录不自动等于30个独立任务；用户授权AI模拟可做工程演练，但必须标simulated_ai，不能改为真人、竞品或偏好证据。

评测工具为`tools/agent-eval/README.md`及cli；空模板返回partial/exit2是预期，不是改断言过关。材料/私人映射在仓库外；原失败/暂停不从分母删除。现有私有材料示例：`/Users/yihua/Documents/BetweenFrames-Reviews/m3-lumix-20261016-141130/`，接手先核实存在及授权，不能当新评审已经发生。

**M5真人**：至少5个真实双人项目完成提案→反馈→合并，记录参与者角色、版本、冲突/反馈及复用意愿。多浏览器/生成账户/同一AI扮演两个人不计真人。

**需要用户/管理员提供的资源**：

| 资源 | 当前缺口 | 提供方式 |
|---|---|---|
| DeepSeek | 安全配置位置、有效余额、明确测试/用户额度 | 本机或服务器私有配置路径/secret管理入口；不要聊天粘贴Key |
| Apple公证 | 可用profile或自定义Keychain/ASC Key路径 | 用户本机交互创建后给名称/路径；证书已存在 |
| 分发/容量 | 下载存储、对象存储/扩容方案、配额/告警归属 | 指定环境和受控权限；不擅改DNS、清服务器空间 |
| 源码CI | 已有yihua-bot/focalroom；本机尚未配置remote、基线待对齐及提交推送批准 | 核对已知仓库与现有WIP，取得具体写入批准，不从其他项目借用 |
| 质量/协作 | 摄影师/编辑、授权任务、设备/显示器、双人项目 | 真实素材范围与参与者安排，模拟与真人分开 |

以上外部条件不足时继续R1、离线失败边界、打包脚本、文档/CI本地检查和已授权工程，保持实际未验项，不能无限停在“等资源”或自行编造完成比例。

## 8. 接手后的测试与运行方式

从应用根执行，先核对Node25.8.1≤版本<26、锁文件、Rust/macOS工具链和本地PG。PG runner默认尝试Homebrew PG15或pg_config；生产是16.14，目标版本差异需要另验。

```bash
cd "/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames"
npm run contracts:check
npm run typecheck
npm run lint
npm test
npm run test:cloud:pg
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" -- --skip store_load_delete_roundtrip
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
npm run build:workspaces
```

上面Clippy带harness只是验证测试目标，不是正式构建参数。默认测试不运行ignored；不要添加全局`--ignored`，其中有真实模型/照片上传/Keychain测试。测试子进程清空真实服务Key，不向生产DB写测试数据。PG runner为各套件创建独立临时数据库，已有计数71只是上轮结果，新增测试后按真实输出更新。

按改动追加`npm run test:ops`、获准的独立Docker演练、`test:e2e`/`test:integration`/`test:web`及PG `web-journey.mjs`/`native-promotion.mjs`；先重建真实harness再跑依赖它的旅程。`scripts/verify-all.sh --skip-keychain`是已有聚合入口，先读脚本，不把其尾部“全部通过”当远端CI或原生验收。

E2E/原生构建按顺序跑，避免共享Vite端口/target锁/前端dist互相覆盖；不杀用户已有1430/其他端口服务。隔离UI可用`npm run review:shared`，配置`FOCALROOM_REVIEW_HOSTED_AI=1`只产生本地假AI/假额度，不是生产配置。测试账户/密码由新进程生成，不能拿到公网。

官网单独在其根运行`npm run build`、`npm run check`、`npm test`，不把产品API测试当官网四语测试。

每次交付记录实际命令/退出码/环境/源码SHA、失败尝试与修复、GUI截图/原生操作、DONE/NOT DONE。没有界面通道就明确待原生验收，系统锁定/安全代理阻挡需用户接管；不要以浏览器替代原生。

## 9. 操作授权与回退边界

用户AGENTS要求：提交/推送、删除/批量改动、系统/权限/凭据变更、数据库schema/批量数据、敏感发送/生产API等高风险操作先明确确认。已有批准只覆盖已执行FocalRoom API/Web、备份/0014/0015/Resend及一次测试邮件，不自动授权新生产模型费用、公证上传、官网公开下载、其他项目或新迁移。

按具体版本准备可审结果再请求最后一步批准；不要为已批准的同一步反复确认。秘密不进argv/源码/文档/截图/终端日志；接手只需要路径/名称，不需要复制过去聊天中的密钥。公开证书身份/Team ID不是专用密码。

任何不确定模型结果保留预留和请求ID；任何不确定Promotion确认保留冻结，禁止本地/云双写。LOCAL由本地SQLite权威，SHARED由Postgres权威；Yjs只用于私人笔记，不覆盖正式版本。Agent只提草案，不直接批准正式作品。

## 10. 可直接复制给下一位AI的接手指令

> 请接手FocalRoom当前工作区，先读`/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/docs/DEVELOPER-HANDOFF-2026-10-19.md`和应用AGENTS，再按其阅读顺序核对源码及证据。Git根是film，HEAD49fcc65不包含全部成果，保留全部已修改/未跟踪文件；不要清理、reset、提交推送或自动部署。用户目标是完成官方DeepSeek服务、正式macOS客户端/官网下载链路，并补齐M0–M5；不进入M6/M7。
>
> 第一轮先记录本机49fcc65+WIP与已知GitHub23308d6ab3的基线差异；按第11节A0/A1修CI和评论prompt/task_type，补会在旧代码失败的离线/PG/Web回归；接着按A2/A3修上下文照片证据和token估算，再处理R1验证入口及R2/R3/R4首发准备。不得按审计建议直接checkout远端快照覆盖WIP。真实邮件登录和人工收件已通过，不重复发信。Developer ID有效，但常见公证profile尚未在默认钥匙串找到，不要说证书缺失或假定profile已创建。
>
> 生产已有API/Web和0014/0015，当前用hosted-ai-20261019私有runtime配置，不能用旧base全栈重建；AI关闭/额度0、Worker关闭。云作者记忆文件未正确接线，Worker已有Postgres队列但与账户账本分离；分别按第6节收口，不误报为已完成或完全没写。缺资源时继续离线可验证任务，真人/模拟分开记录。每轮给出实际完成、测试证据、未完成和下一任务；生产变更/真实付费调用/公证上传/公共下载前按具体产物取得确认。

## 11. 外部审计逐项核对与第一批修复单

来源为用户提供的`FocalRoom_Code_Audit_2026-10-19.md`，副本[在此](EXTERNAL-CODE-AUDIT-2026-10-19.md)。该审计针对公开23308d6ab3，不是本机HEAD，也不是生产全量实测。其“未访问公网”只代表审计者能力，不撤销本聊天已有生产/收件证据。

本轮补充证据：[GitHub与8文件对照](evidence/handoff-20261019/github-review-check.json)、[真实模块离线复现](evidence/handoff-20261019/comment-task-offline-reproduction.json)。没有修改产品代码或宣布修好问题；修复留给下轮。以下A编号与前文R编号分开，避免与审计报告R批次混淆。

### A0 · CI失败与测试入口隔离（先做）

已直接读取[run39224541729](https://github.com/yihua-bot/focalroom/actions/runs/39224541729)及job117967524565日志；本轮查询时它也是最近run。commit23308d6ab306778c6e79ab9e96e886e8447aa496，conclusion=failure，桌面构建skipped。

实际错误：`e2e/v2-foundation.spec.ts:11`预期`FocalRoom · Web 工作台预览`，实际`FocalRoom — 团队协作审稿`，并有127.0.0.1:3002拒绝连接。当前`apps/web/src/main.tsx`直接渲染SharedApp，旧测试却继续要求无API的示例工作台。不是修一个标题字符串就算完成。

**任务**：明确独立Preview、登录Shared、真实PG协作的页面与测试入口；要保留Preview就有明确路由/入口并测刷新不持久化，Shared测试则启动受控API。不能删测试、跳过或改成只检查HTTP200。修`verify-v2-m0.sh`的实际依赖，继续跑此前因首错跳过的全部后续套件/桌面构建。

**交付**：原红灯日志、定位和本地复现、新测试证据；获准提交后提供新远端绿灯run。修过本地不等于远端已绿。

### A1 · 评论模型协议与分歧结果类型（与A0同批）

两个确定问题已在本机实际导入模块复现（fetch和pool为内存夹具，零网络/DB写入）：

1. `buildPrompt`返回{system,user}，实际messages把整个对象放进user.content，system固定普通汇总，两种kind都产生object而非所需文本；分歧system没有应用。
2. `storeTaskResult` SQL固定summarize_comments，Worker不传job.kind；分歧结果不能按`AgentAnalysis.tsx`所需类型找到。

**任务**：分别传prompt.system/user字符串，kind从任务贯穿函数/入库/读取/UI；明确缺usage/空结果/无效kind处理，不能通过默认0计量冒充完整结算。

**验收**：旧代码必失败的两种kind请求结构断言；隔离PG验证真实task_type与正确项目/作者；Worker任务COMPLETED且Web展示对应结果的旅程；普通汇总不回归，失败/撤权不外发不写正式版本。修后仍不自动开启生产Worker。

### A2 · 上下文压缩保留照片和证据（第二批）

本地源码确认：超过压缩阈值后按name保留最新工具结果，较早不同照片的inspect_photos也会superseded；旧结果再被截取200字符。完整权威回执仍在库，但模型投影可能没有这些事实。

**任务**：以调用/照片资源范围/意图判断可替代性；仅覆盖范围真正包含、语义更新且有可检索摘要/引用时归并。保留photo IDs、来源/证据、反例、仍待决策引用和能取回原结果的标识，不截断为缺结构的JSON前缀。

**验收**：A/B不重叠照片分批观察、同照片不同工具意图、重叠集、超过阈值/多批次、检查点恢复和撤权时仍可引用正确事实；正式状态/审计不变。审计建议的100张场景是待新增回归，不宣称本轮已经跑过。

### A3 · token估算与费用预留分离（第二批）

本地input_estimate返回序列化字节，PolicyGate把它和token上限/费率直接比较，单位耦合问题成立；它同时承担保守防超账作用，不能简单除4来“修复”。

**任务**：区分模型上下文估算、实际provider usage、文本/图片成本与服务端保守预留；用已支持模型的计数/经测校准方法加裕量，不支持时明确保守回退。修估算不能弱化0015预留/并发/未知结果保护或使预算超支。

**验收**：中文/英文/JSON转义/工具schema/图像的预算边界、估计与实际偏差记录、临界暂停/恢复；超过额度零外发、不确定调用不重复。真实token校准要待R2批准的小额调用，不伪造供应商实测。

### A4 · Web预览加载与协作组件（后续受控切片）

串行票据/Blob及最后统一发布图片状态已静态确认；具体p50/p95和慢网秒数未测。先用有界并发和渐进显示解决瓶颈，取消切页/退出请求并释放对象URL，局部失败不阻塞整页；缓存键必须隔离账户/项目/内容SHA和当前权限，撤权后不能继续取新预览。

验收48图页、200–500图项目、切页/快速换选/慢网/离线/撤权/重登；记录首图/首屏p50/p95、资源与失败重试次数。拆ProjectReview先拆可测试的数据hooks/服务，保留当前领域命令/审批/冲突语义。跨端共享ContactSheet/Inspect/Sequence/ProposalDiff是分阶段UX工程，不为交接而重构整仓。

### A5 · 审美记忆与产品质量（不能用规则数量关门）

materialize确有两字词/重复支持的规则假设，证据、纠正、排除、墓碑的机制保留；已有Apple语义检索不等于语境化偏好提取。增结构化抽取时保留人工证据、项目/作者scope、不确定性和反例，不把Agent草案当作者偏好，并结合第7节首次/第三次真人任务测记忆增益。

审计关于专业色彩、高位深、Web字体/布局、统一视觉工作台及商业成熟度是重要验收/产品判断；当前没有渲染现场、目标设备和专家实验，不能将每一项定性意见写成已复现崩溃。先做对应场景测量，再决定必要实现。自动更新、M6/M7仍不能因审计列出就升成首包必须开发的新范围。
