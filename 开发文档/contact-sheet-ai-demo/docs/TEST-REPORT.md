# 当前接续：界面文案清理（2026-10-18）

按用户要求移除前端开发自述、重复说明及工程术语，精简项目/检查器/建议/记忆/导出/共享页面；保留演示标记、授权与操作后果。196 项前端测试、Lint、类型及 Desktop/Web 构建、四语包一致性通过；浏览器验收覆盖工作台、共享登录/项目/批注/版本和手机账户页。首轮类型与旧标签断言问题已修复。范围、截图、未执行项及并行提交说明见 [文案清理记录](UI-COPY-CLEANUP.md)。未上线或重打原生包，里程碑状态不变。

# 当前接续：M4 备份与独立空目标恢复（2026-10-18）

修复Compose路径/项目绑定、猜卷、备份不停写、恢复忽略停服失败及直接覆盖等运维问题。新恢复默认仅验证，显式独立空目标方可写入；成功/失败均保持目标API停止。9项归档故障边界、6类真实Docker/PG演练、22表全部行摘要/2预览对象及非root实际读取、30PG业务回归、类型/Lint通过。首轮stdin问题和修复复验保留于[备份恢复报告](V2-M4-BACKUP-RESTORE-TEST-REPORT.md)。本轮不修改GUI/Rust、不重跑或声称旧原生验收；Docker API是启停哨兵，不能当整栈上线。无腾讯云/用户库/真实模型邮件/提交推送，测试卷保留。M3/M4/M5仍partial。

# 当前接续：M4 账户恢复与集中限流（2026-10-18）

新库注册500、密码重置后无法登录、小时配额实际按分钟、验证重发并发绕过等问题已修复；Web补齐四语找回/重置/验证/重发，保持账户存在/不存在的统一受理响应。评论云汇总在专属模型发送授权接通前明确拒绝，不凭管理员权限/Key外发。30项PG/存储与196项前端、类型/Lint/契约、Desktop/Web构建通过，CUA演练账户与恢复页面补验。基线失败、清理测试时区夹具失败、路由/手机页头问题和复验均保留于[本轮报告](V2-M4-ACCOUNT-RECOVERY-TEST-REPORT.md)及evidence。无真实邮件/模型、用户库或生产操作，无应用仓库commit/push；未重新打Tauri包或跑原生迁移。M3/M4/M5整体继续partial。下方2026-10-17及更早记录为历史。

# 当前接续：M4/M5 PostgreSQL 共享协作（2026-10-17）

原生续验已完成真实系统导入/共享包导出、迁移与只读重启，50表/2源图一致，0Agent任务；补修缓存标签和原生浏览器入口。最终188前端、192桌面Rust、16旧云、19PG/存储、23E2E和两共享旅程通过，最后三入口构建及59.78MiB独立debug包通过。默认Quark页面CUA读取超时，系统浏览器画面仍未计通过；同一绑定项目的IAB登录/真实预览/序列v4已验收。最终280源摘要/90起始PNG核验及全部失败记录见报告续验段；阶段未完成清单已按“待编码/待实测”更新，不能理解为只剩部署和真人。

按用户明确范围推进至 M5：NestJS/PostgreSQL 共享权威、真实桌面 Promotion、Web 登录/邀请/私有预览/批注/私有草案修订/部分批准/不可变版本、WS/Presence/重连/撤权、Yjs私人笔记和受限评论 Task 已实现。19项PG/存储、三独立浏览器/54张生成预览及实际Rust HTTP异常旅程的证据、最终全量与打包结果，统一见 [M4/M5 报告](V2-M4-M5-SHARED-AUTHORITY-TEST-REPORT.md)、[ADR0040](ADR/0040-m4-m5-postgres-shared-authority.md)。保留路由、定位、类型和旧云安全测试的首轮失败记录，没有提高超时。旧SQLite /api/v2统一501，不作为正式权威。无用户库/真实Provider/生产操作；M3专业/多Adapter、真实R2/容器/生产和>=5真人双人项目仍未完成。下文为历史，当前计数与验证层级以本轮报告为准。

# 先前切片：M3 同端点模型分工（2026-10-17）

同一服务的策展/观察型号可独立配置与冻结，能力、费率各用各的，图像收件人/缓存/检索按实际观察身份；恢复拒绝角色/能力变化，不新增跨服务 fallback。190桌面Rust+13本地云、183Vitest、16预览/6后端/1Web及严格检查通过；最终界面修改后前端及两套E2E复验通过。独立双角色界面和59.19MiB debug包/元数据/图标/弱链接通过，252源摘要与90起始PNG一致。无新迁移/依赖、真实Provider、用户照片/用户库、部署/Git操作；M3仍partial，专业质量未测。见 [本轮报告](V2-M3-MODEL-ROLES-TEST-REPORT.md)、ADR0039。下文为历史。

# 先前切片：FocalRoom 品牌（2026-10-17）

名称、无文字透明图案、侧栏、favicon 和原生图标已接入。直接从原图像素提取，透明边缘与白/深/灰背景已核验，脚本可重复得到同摘要。180 前端单测、16预览+1Web、类型/Lint、根/Desktop/Web构建和独立 FocalRoom Review.app 打包通过；名称更新时5项Rust导出/册页回归通过。Web favicon 首次路径失败及修复复验均有记录，90起始PNG已恢复。没有启动原生窗口或用户库，M3仍partial。详见 [品牌更新记录](FOCALROOM-BRAND-UPDATE.md)。

# 先前切片：M3 实际服务图像授权（2026-10-17）

图像授权按实际端点/型号隔离，旧通用授权不升级；冻结实例校验、过时界面拒绝、全服务禁止/单方重新批准、HTTP不重定向、回执事务与管理/取消界面已验收。M3仍partial，不宣称完整跨服务 fallback、Creator隔离或质量通过。详见 [本轮报告](V2-M3-PROVIDER-CONSENT-TEST-REPORT.md)、ADR0038。

verify-all --skip-keychain exit=0：181 桌面 Rust +13 本地云、180 Vitest、16 预览/6 后端及契约/类型/Lint/fmt/严格Clippy通过；截图QA只改界面/测试后，再跑全部前端检查与两个E2E套件 exit=0。独立原生debug 58.59MiB、弱链接检查通过，未启动原生窗口/用户库。默认13 ignored及钥匙串写删1 filtered不计通过；243摘要一致，90起始PNG字节保留。无真实Provider、LUMIX外发、依赖/迁移、部署或Git操作。以下为历史。

# 先前切片：M3 本地作者记忆（2026-10-17）

显式授权、离线 Agent 上下文/工具、只读来源列表及来源变更/撤权后的失效保护已接通。默认隔离、旧 true 不继承、支持/反例溯源、来源过滤、事务回滚、ABA/回放拒绝、晚到回复计量与超限关闭均有回归。云模型与共享项目排除，完整 Creator 隔离和 Provider 专属授权仍待完成；M3/M02 保持 partial。详见 [本轮报告](V2-M3-AUTHOR-MEMORY-TEST-REPORT.md)、ADR0037。

最终各项通过：174 桌面 Rust +13 本地云、173 Vitest、16 预览/6 后端、契约/类型/Lint/fmt/严格Clippy及独立debug构建。首轮 verify-all exit=1：新增 checkbox.check 假设同步更新，与持久化后更新的控件不符；只修 E2E 后 Lint 与完整后端套件 exit=0、6/6（1.3分钟），生产代码未变，原 trace 保留。13 ignored 与钥匙串写删1 filtered 未计通过。243 文件摘要一致，90起始截图字节保留。未启动原生窗口/用户库、调用真实模型或部署，Prompt m3-openai-8 / m2-mock-2。以下为历史。

# 先前切片：M3 持久化增量向量索引（2026-10-17）

已有缓存事实的向量可跨连接重开复用；源hash/观察正文/模型配置/范围重验，变化单张只重算该张。派生文件每项目500条/8MiB，损坏拒绝/重建、原子替换、权限边界和清空观察联动均有测试；真实中英文模型确认复用2条/生成1条，正式状态、原图、上传与证据不变。详见 [本轮报告](V2-M3-PERSISTENT-INDEX-TEST-REPORT.md)、ADR0036；最终全量及原生测试/构建/摘要以evidence为准。M3整体仍partial，语言识别误判和专业质量门槛继续单列，无新服务/迁移/部署。以下为历史。

最终各项：165桌面Rust+13云、169Vitest、16预览/6后端、3项显式原生及debug构建通过。全量首轮因打包触发Vite整页刷新，提案界面超时exit=1；结束打包后隔离整个后端套件exit=0、6/6通过，无产品修改/重试/放宽超时。253个最终源摘要、90张起始截图一致；过期预取的cfg(test)增强另已定向补验。

# 先前切片：M3 本地语义候选检索（2026-10-17）

新增已有缓存事实的真实本地句向量检索，保留原关键词工具和明确回退；来源/任务范围/取消/重复回执经过边界验证。中文640维、英文512维 revision1 原生测试和 Agent 工具测试已显式执行。现有云测试复现并修复 Viewer 写评论越权，以及严格 Clippy 问题；原角色测试改为实际 Reviewer/Viewer 分离。详见 [本轮报告](V2-M3-LOCAL-SEMANTIC-TEST-REPORT.md)、ADR 0035；最终全量日志、源码摘要、构建结果以 evidence 为准。无新真实照片或真人样本、依赖/迁移/用户库变更、部署或 Git 操作；M3整体仍 partial。生产部署标题不是验证证据，其他并发新增模块不计本轮功能交付。以下为历史切片。

最终 verify-all exit=0：160 桌面 Rust +13 云、169 Vitest、16 预览/6 后端通过；额外2项真实原生向量/工具测试、独立debug构建和弱链接检查通过。诊断固定17迁移断言已修，当前严格匹配18文件。90张历史截图复核一致，18个源码摘要一致。DeepSeek Prompt7复验因旧凭据图片识别失败未执行、0请求，不计通过。

# 先前切片：M3 模型能力配置（2026-10-16）

已补精确端点/模型能力目录、有界部署声明、组合能力要求和 Adapter 直接入口检查；恢复拒绝能力变化或旧无摘要绑定。界面阻止错误委托/授权但仍允许拒绝图像，纯文本真实任务不误标 Mock。151 桌面 Rust +11 云、169 Vitest、16 预览/6 后端及严格检查通过；独立 debug 构建通过。DeepSeek 新门槛无图任务 review_ready，3轮/4工具/13,907实际token/3美分估算；实际模型账本/checkpoint 一致。Prompt 仍 m3-openai-6，未改缓存键、迁移、用户库或 LUMIX 评审样本。M3 整体 partial，见 [本轮报告](V2-M3-CAPABILITIES-TEST-REPORT.md)、[配置说明](MODEL-CAPABILITIES.md)、ADR 0034。以下为历史切片。

# 先前切片：M3 配置路由、任务恢复与缓存检索（2026-10-16）

本轮 Prompt m3-openai-6、实际模型/端点身份绑定、检查点核对、旧 queued 失败退出、缓存端点隔离、可观察事实关键词检索已验收。144 桌面 Rust + 11 云、164 Vitest、16 预览/6 后端、契约/类型/Lint/fmt/严格 Clippy 全通过；DeepSeek 纯文件名回归 3轮/4工具/13,792 实际 token、3 美分估算、0 图片，独立 debug 构建通过。未启动用户库，未改变原 LUMIX 记录。M3 质量/语义索引/多 Adapter 仍 partial；见 [本轮报告](V2-M3-ROUTING-SEARCH-TEST-REPORT.md)、ADR 0033。以下为历史切片。

# 当前接续：M3 AI 模拟评审和来源分离（2026-10-16）

用户允许模拟真人；本轮真实页面录入/导出 2 条 AI 角色评语，0 真人，CLI 复核原 key/冻结材料并单独统计模拟分数。没有修改原批次、正式作品或审美记忆。15 项定向测试通过；见 [本轮报告](V2-M3-SIMULATED-REVIEW-TEST-REPORT.md) 和 ADR 0032。真实质量门槛仍单列，工程可继续。以下为历史切片。

# 当前接续：M4 既有本地 API 边界修复（2026-10-16）

仓库 9976b16 已新增 cloud/ axum/SQLite 服务，现修复单授权撤销误伤、Argon2id/摘要令牌/时限与注销、注册原子性、有界 JPEG 和新目录/进程重启。全量脚本通过：134 桌面 Rust + 11 云 HTTP/进程测试、159 Vitest、16 预览/6 后端、契约/类型/Lint/fmt/严格 Clippy。没有腾讯云部署、Postgres 权威或真实 Web 审稿；M3/M4 整体仍 partial。详见 [本轮报告](V2-M4-LOCAL-BOUNDARIES-TEST-REPORT.md)、[剩余计划](V2-REMAINING-PLAN.md)。以下为历史切片，原“无 API 代码”不再代表当前仓库。

# 当前增量：M3 授权真实素材与离线评测（2026-10-16）

80 张 LUMIX JPEG 的授权链路已实测，12 张抽样观察形成 8 张 keep 草案；正式项目和 80 源 hash 不变。保留前一失败，1 个探索任务/2 次调试运行，成功 run 84,203 实际 token、9 美分版本化费率估算，尚无真人质量验收。补齐预算内单轮输出、已核对端点费率、视觉档位预留、明确错误反馈及离线盲评/冻结材料。134 Rust、159 Vitest、16 预览、6 后端与严格检查通过，独立 debug 构建通过。完整结果、真实素材外发次数、失败与剩余工程见 [M3 报告](V2-M3-EVALUATION-TEST-REPORT.md)，全阶段缺口见 [剩余计划](V2-REMAINING-PLAN.md)。M3 整体 partial，M4 云 API 确认尚不存在；未修改原用户库或线上数据。以下为历史记录。

# 历史增量：DeepSeek 实测与 token 结算（2026-10-16）

真实文件名选片任务进入 review_ready：3 轮、4 工具、1 份正确草案、13,021 token，正式项目不变；单张生成图的 Provider 协议通过，799 token。原始开放选片仍在预算内停止前未完成，保留失败结果；M3 整体 partial。修复任务身份缺失、工具批次静默丢弃、外部错误正文泄漏、计量结算与有效预算可见性。完整证据、回归与剩余项见 [DeepSeek 报告](V2-DEEPSEEK-TEST-REPORT.md)。下方“未调用真实服务”是上一切片的历史记录。

# 历史增量：Agent 响应与验收门槛修复（2026-10-16）

视觉 Provider 在本地复核严格 schema 和 ID 唯一性；旧缓存同时复核结构与照片身份，错误响应不落观察库，实际上传仍扣配额。真实任务测试改为必须产出预算内可审阅草案，缺 key 不再静默通过；Provider 冒烟与摄影质量验收分开。Rust 默认 128、前端 149、浏览器预览 16、真实后端旅程 6 项通过，契约/类型/Lint/fmt/严格 Clippy 通过；证据见 [本轮报告](V2-AGENT-VALIDATION-TEST-REPORT.md)。新门槛未调用真实服务验收，M3 的 30 项授权任务与专家评测仍待完成。

M1 最新格式交付见 [CR2 报告](V2-M1-CR2-PREVIEW-TEST-REPORT.md)：支持有界 IFD0 内嵌 JPEG 子集，明确非 RAW 全分辨率；原生显示/退出重启已验收。专业色彩、总资源和低端硬件仍 partial。以下均为历史切片，旧“下一步/通过”声明以当前报告为准。

# 历史验收：M1 原生 release 500 图与灯箱修复（2026-10-15）

生成图库的真实系统选择器、500 图浏览/编辑、25 图序列与版本、系统保存导出和完整退出重启通过；50 张表与 500 个源 hash 一致。修复未加载胶片条 CSS 导致大尺寸缩略图溢出的问题。本轮 120 Rust 默认、147 前端、4 M1 GUI 通过；无新迁移/依赖。原生资源采样只覆盖宿主主进程，不能据此证明总内存或低端机达标。M1 整体仍 partial，继续补 RAW 细节/色彩/资源证据，不进入新 M2。见 [V2-M1-NATIVE-500-TEST-REPORT](V2-M1-NATIVE-500-TEST-REPORT.md)。以下为历史结果。

# 历史验收：M1 接续评审三项修复（2026-10-15）

RAW 检视改为校验后的 1400px 派生预览；幂等重放恢复首次完整业务/历史回执；存储基准按实际 1250 命令计算并核对回执行增量。122 Rust 功能测试、1 增长基准、147 前端单测、27 条独立 E2E、严格检查及两端/原生构建通过。独立原生包实测 7000×5000 边界图仅检视 1400×1000，源 hash 不变。报告、证据与最终基准见 [本轮报告](V2-M1-REVIEW-FIXES-TEST-REPORT.md) 和 ADR 0028。旧 29 条 E2E、每 500 命令增长数据及“RAW 嵌入预览”实现声明撤回。M1 仍 partial，总账 17 pass / 7 partial，原用户库未升级，不进入新 M2。以下为历史切片记录。

# 历史验收：M1 原图检视、GUI 内存与原生恢复（2026-10-15）

JPEG/PNG 按需原图、Retina 1:1、同步对比及明确缓存回退通过；精简历史投影减少 91.1% 载荷，40 次编辑后 JS Heap 220.8→74.4MiB。独立原生文件/文件夹选择、保存、完整退出重启通过，49 表与源 hash 前后一致。113 Rust / 146 单测 / 26 E2E、严格检查与两端/原生构建通过。见 [本轮报告](V2-M1-INSPECTION-TEST-REPORT.md) 与 ADR 0025。M1 仍 partial，总账 17 pass / 7 partial；其他原图格式、色彩、原生 500 图和低端性能待补；未升级原用户库，不进入新的 M2。以下保留历史切片结果。

# 历史验收：M1 的 500 张 GUI 切片（2026-10-15）

250 PNG + 250 JPEG 的真实文件通过 GUI 导入、滚动、人工编辑、检查、序列、撤销/重做、版本恢复、导出和新 Rust 进程重启一致性；500 个源 SHA-256 前后一致，异常与取消恢复通过。110 Rust / 142 单测 / 25 E2E 通过，见 [M1 报告](V2-M1-500-GUI-TEST-REPORT.md)。总账为 17 pass / 7 partial；M1 整体仍 partial，原生选择器、原图 1:1/色彩、内存优化和低端设备验收未关闭，不进入新的 M2 开发。以下为历史切片结果。

# 当前验收：v2 评审修复（2026-10-15）

评审八项问题已修复：提案 GUI 契约/错误处理、审批整体事务与单组撤销、位置语义、持久化幂等、Promotion 写入门槛/失败重试、Router 实际执行接线。110 Rust、142 前端单测、16 预览、1 Web、6 条真实后端旅程通过；钥匙串写删跳过，性能测试 ignored。新增 0017 仅在临时库验证，原用户库未升级。见 [本轮报告](V2-REVIEW-FIXES-TEST-REPORT.md)。

**撤回旧版“22 pass / v2 工程侧完成”结论**。当前 [总账](V2-COMPLIANCE.md) 为 16 pass / 8 partial；云服务与 Creator 权限等仍有工程缺口，不能仅归为外部账号条件。以下阶段记录和文末的早期进度保留历史，不作为当前完成声明。

# 当前验收：v2.0 M0 工程与契约切片（2026-10-15）

83 Rust、134 前端单测、17 浏览器 E2E、5 后端旅程通过，类型/契约/Lint/fmt/严格 Clippy、两端前端与独立桌面构建通过。原生 28 图导入/选片/完整退出重启通过，37 表逐行一致，28 源 hash 不变。M0 整体 partial/待架构与 UI/UX 评审；新模式持久化迁移、远端 CI 未实施/未执行，不自动进入 M1。详见 [V2-M0-TEST-REPORT](V2-M0-TEST-REPORT.md)。以下保留历史验收。

# 当前验收：M5 本地存储诊断（2026-10-15）

已完成桌面/harness 共享诊断投影、固定目录存储统计、失败/不完整状态、脱敏 JSON 预览/复制和刷新。全量 **72 Rust、56 前端单测、16 预览、5 真实后端旅程** 通过，契约/typecheck/Lint/fmt/严格 Clippy 通过；原生补验前后 35 表逐行一致、28 源 hash 不变。无新增迁移、依赖或模型任务，原用户库未升级。见 [STORAGE-DIAGNOSTICS-TEST-REPORT](STORAGE-DIAGNOSTICS-TEST-REPORT.md) 与 [ADR 0013](ADR/0013-m5-local-storage-diagnostics.md)。本轮停止等待评审；签名公证、真实模型/计价、真人 Beta、真实大图库性能等边界保留。

# 当前验收：M4 本地记忆控制（2026-10-15）

已完成假设与证据控制、原文/反例、FTS5 项目内检索、幂等回执和任务记忆版本保护。全量 67 Rust、50 前端单测、16 预览及 4 真实后端旅程通过；新增边界与最终构建复验详见 [MEMORY-CONTROLS-TEST-REPORT](MEMORY-CONTROLS-TEST-REPORT.md) 和 [ADR 0012](ADR/0012-m4-local-memory-controls.md)。钥匙串写删与性能基线本轮未执行；解锁后的原生 UI、旧任务恢复拒绝、新任务委托与完整退出重启已补验通过，35 表快照及 28 源 hash 一致。原用户库未升级，跨项目仍关闭。下文保留之前的验收历史。

# 当前评审修复验收（2026-10-14）

评审发现的六项链路缺口已修复并补回归。前一轮 **59 Rust、44 前端单测、16 预览 E2E、3 真实后端旅程通过**，契约/类型/Lint/fmt/Clippy 与独立 Debug 构建通过；钥匙串写删测试未执行。**原生系统选择器、实际 PDF 和退出重启已补验；补验发现的 Safari 页脚分页问题已修复并跑相关回归。真实模型与计价待补验。**详见 [评审修复报告](REVIEW-FIXES-TEST-REPORT.md)、[原生补验报告](NATIVE-REVIEW-FIXES-REPORT.md) 与 [ADR 0011](ADR/0011-review-fixes.md)。以下为原 M5 及版本增量历史报告。

# v1.0 M5 测试与验收（Beta 工程准备）

日期：2026-10-14（Asia/Shanghai）。M5 工程部分已完成并验收：Keychain 密钥存储、诊断命令、性能基线（Release）、Beta 完整旅程 E2E、隐私文档。**发布流程项未完成**：签名/公证（无开发者证书）、DMG 打包、20 位真实用户 Beta 与真实摄影师试用——这些需要证书与真人，工程侧无法替代，留待发布执行。

M4 报告归档于 [V1-M4-TEST-REPORT.md](V1-M4-TEST-REPORT.md)；M3 真实模型 E2E 仍待 API Key 补验。

## 功能验收

| 项目 | 完成内容 | 验证方式 | 结果 | 未解决问题 |
|---|---|---|---|---|
| Keychain 密钥 | store/load/delete + env→钥匙串解析链 + GUI 存入/移除 | keychain 2 项回环测试（独立 service）+ 旅程 | passed | 写入瞬间进程参数可见（PRIVACY 声明）；非 macOS 未集成 |
| 诊断工具 | get_diagnostics（版本/计数/配置，脱敏声明）+ GUI 复制 | Beta 旅程断言诊断内容 | passed | 缓存体积统计未含 |
| 性能基线 | Release：导入 2.9s/200 张（70 张/秒）、决策 p95 23.0ms、投影 3.4ms | perf_baseline（--ignored 手动运行，退化守卫） | passed | 数字为本机（Apple Silicon），非跨硬件承诺 |
| Beta 旅程 | 意图→导入 200→选片+备注→Mock 委托→双草案→假设纠正（AG-11 旅程级）→局部采纳→诊断→重启一致 | beta-journey E2E（真实后端 52.2s） | passed | 真实模型版本待 key |
| 隐私文档 | PRIVACY.md（照片/上传/密钥/导出/诊断/边界） | 文档评审 | — | 证据删除语义待产品决定 |
| 缓存配额清理（M1-09 补） | 孤儿缓存（无 photo 引用）按最旧回收，默认 2GB；引用缓存/原图不动 | cache_quota 2 项测试 + verify-all 全量 | passed | 引用缓存淘汰待懒重建 |
| M0–M4 回归 | 全部能力保留 | 45 Rust + 28 单测 + 16 预览 + 3 集成 | passed | — |

## 自动化结果（2026-10-14 18:30 全量复验）

| 检查 | 结果 |
|---|---|
| cargo test（target-v1，debug） | passed，45 tests（agent_runtime 13、agent_vision 4、agent_memory 4、command_history 10、local_core 10、keychain 2、cache_quota 2） |
| cargo test --release --test perf_baseline | passed（数字见上，--ignored 手动运行） |
| cargo fmt / 严格 Clippy（含 integration-harness） | passed，0 warnings |
| cargo build --release --bin between-frames | passed（unsigned） |
| npm run typecheck / npm run lint | passed，0 warnings（eslint 忽略构建产物目录） |
| npm run test | passed，7 files / 28 tests |
| npm run test:e2e | passed，16 tests |
| CARGO_TARGET_DIR="target-v1" npm run test:integration | passed，3 tests（agent-core、local-core、beta-journey 52.2s） |

## 复现命令

~~~bash
npm run typecheck && npm run lint && npm test && npm run test:e2e
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1"
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --release --test perf_baseline -- --nocapture --ignored
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --release --bin between-frames
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
~~~

## 剩余问题与停止点

签名/公证/DMG 需 Apple 开发者证书；20 位用户 Beta 与 3 位摄影师无指导试用需真人执行（验收脚本以 beta-journey 自动化打底）；真实模型 E2E 仍待 API Key；M4 遗留（跨项目假设、FTS5、证据删除入口）继续有效。

## v1.1 增量（2026-10-14，两轮）

第一轮：版本对比（A vs B / 当前序列的新增/移除/移位/人工判断变化，纯前端复用快照）与项目模板（五个静态模板预填）。第二轮：项目标签——migration 0007（tags_json）、独立通道 set_project_tags（组织元数据不占人工撤销栈、不推动 revision/不使草案 stale）、编辑对话框标签输入（、分隔）、列表标签 chips 与筛选；Rust project_tags 1 项（往返/校验/幂等/revision 不变/归档拒绝）+ 组件 9 项。第三轮：沉浸审片（版式增强）——灯箱内 K/X/U 决策与 0–5 评分（与网格同一正式命令通道）、状态与星级实时反馈、底部胶片条（状态标/星级/当前高亮/点击跳转）；组件测试 4 项。**v1.1 三项全部交付**，verify-all 全量通过（46 Rust、41 前端单测）。见 [PLAN-V1.1](PLAN-V1.1.md)、[ADR 0009](ADR/0009-v1-1-templates-diff.md)。

## v1.2 第一轮增量（2026-10-14）

印刷参考导出（format=print：A4 横版分页 HTML，封面+每页双图+裁切参考，浏览器打印存为 PDF，零新依赖）与元数据侧车（format=sidecar：每照片一 JSON + manifest，目录级只新建不覆盖，路径默认私密可 opt-in）已交付：export_v12 两项测试（3 张作品 2 页 3 plate 结构断言；侧车内容/manifest/重复导出拒绝/路径 opt-in）；导出对话框新增两个按钮。verify-all 全量通过（48 Rust、41 单测）。页跨图编辑已随后交付：册页标记（spread/full/无）经 SetPhotoBookMark 可撤销命令修改并进版本快照；印刷分页尊重标记（独占页/提前收页），book_marks 两项测试（命令往返/撤销/快照恢复 + 分页矩阵）通过。**v1.2 三项全部交付**，verify-all 全量通过（50 Rust、41 前端单测）。见 [PLAN-V1.2](PLAN-V1.2.md)、[ADR 0010](ADR/0010-v1-2-print-sidecar.md)。

## 备份可恢复性巡检增量（2026-10-15，ADR 0015）

诊断升级：最近 5 份升级前备份逐份只读 quick_check，三态报告（ok/corrupt/unreadable，SQLITE_NOTADB 区分）；界面在存储统计下列出巡检结果并明示「应用内暂无自动恢复入口」与人工评估指引。新增 backup_health 测试（三态 + 名称脱敏）；verify-all --skip-keychain exit=0 全绿（73 Rust）。自动恢复替换不做的理由见 ADR 0015。

收尾评审修复（2026-10-15）：墓碑按关键词匹配（证据数漂移不复活，回归钉死）、巡检不跟随符号链接（非普通文件不打开目标）、已纠正假设补删除入口、巡检措辞降级为「完整性通过」。修复后全量 exit=0（75 Rust、57 前端单测、5 集成）。

## v2.0 进展（2026-10-15）

M0 基座切片已提交（257ee6a）并独立复验（verify:v2 exit=0，77 项 v2 契约测试）；M1 Rust 侧复验 3 项（500 张混合集/Undo-Redo 位置唯一/超大拒绝）见 [V2-M1-GUI-REVERIFY](V2-M1-GUI-REVERIFY.md)。M2 执行桥已交付（ADR 0017）：set_selection/sequence 命令经鉴权后接入 v1 唯一通道真实执行（幂等/历史/Undo/证据继承），Agent principal 正式写入纯函数层即拒，proposal/comment 诚实 NOT_IMPLEMENTED 留 M5；v2_bridge 4 项测试。M2 工具别名完成：create_edit_proposal/revise_edit_proposal/evaluate_sequence/retrieve_memory 解析为规范工具，get_comments/summarize_review_disagreement 显式 NOT_IMPLEMENTED（M5）；v2 名字完整任务循环测试（list→proposal→finish→review_ready，正式状态零触碰）。**M2 工程侧关闭**。M3 Router 已交付（ADR 0018）：能力/区域/隐私/预算过滤 + 质量成本排序，三类可读错误（MODEL_CAPABILITY_MISSING/PRIVACY_REGION_REJECTED/BUDGET_EXCEEDED）、图像范围永不自动扩大、explain() 可解释投影不含密钥；6 场景路由测试 + engine 接线。M3 真实质量评估/计费/竞品对比需真模型与真人，不冒充完成。M5 评论切片已交付（ADR 0019）：0011 迁移 + 桥的人类通道写入（同锚点聚合/resolve 重开/跨项目锚点拒绝）+ get_comments 工具解锁（Agent 只读、不构成偏好证据）；summarize 留后续；proposal.create/submit/apply 已交付（ADR 0020）：0012 迁移、nonce 防越权、部分采纳/空集驳回、base 漂移拒绝重建、Agent 仅可创建而人工审批执行、逐操作经 v1 通道可整体撤销。v2_proposals 2 项测试（全流程+陈旧基线）。提案 GUI 已交付：get_proposals/execute_v2_command IPC（principal 为受信入口标识，客户端不可自报角色）、AI 面板旁 ProposalsPanel（列表/展开操作/nonce 表单/部分采纳/空选驳回）、组件测试 3 项。M6 组织骨架已交付（ADR 0021）：0013 迁移 + 席位上限/撤回不可自愈/OWNER 保护/审计留痕 + v2_org 测试。**V2-COMPLIANCE 总账**：24 场景全映射——22 pass（S01/S02/B01/B02 经协议层切片升级，ADR 0022）/ 2 partial（C03/E01 需真实网络两端）/ 0 missing——v2.0 工程侧完成基线。Promotion（0014）、支付账本（0015）、签名媒体链接（0016）落地；hmac 为新依赖。M1 真机性能与人工走查、M2 轨迹映射全量回归、M3+ 未启动。

六阶段工程实现至此全部落地，停止等待整体评审；规格符合性终审见 [V1.0-COMPLIANCE](V1.0-COMPLIANCE.md)。另见 [已知问题](KNOWN_ISSUES.md)、[文件清单](V1-M5-FILES.md)、[ADR 0008](ADR/0008-v1-m5-beta-readiness.md)、[隐私说明](PRIVACY.md)。
