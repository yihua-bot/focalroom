# FocalRoom 下一阶段实施与验收

2026-11-03 启动。产品主线：导入照片 → 摄影知识与视觉理解 → 共同选片编排 → 画布审稿 → 保存作品 → 跨端继续创作。

## 当前执行决定：专业工作流优先（2026-11-03，用户最新调整）

本节取代此前代码优先计划的排序与范围。用户明确取消旧编号 1（统一自然语言入口）、6（手机与桌面接续）、7（Photography Benchmark 与产品指标工具）。不开展这三项新增开发；现有入口、恢复及跨端能力继续随相关改动做必要回归。

先完成代码开发和本地验证，真实设备、真实模型、专业色彩实测、印前与发布验收集中后置。当前提交基线 8be1e435。各模块先核对当前代码，已有实现以补缺和组合联调为主，不重复建设。

### 新的开发顺序

用户最新调整：Focal AI 专业摄影能力置于第一阶段，Lightroom / Capture One 编辑回流置于第六阶段，其余按下表执行。

| 顺序 | 工作包 | 主要代码范围 | 本地完成条件 |
| --- | --- | --- | --- |
| 1 | Focal AI 专业摄影能力 | 原计划第 2/3/4 项：知识详情与来源引用、分层照片观察、可追溯长期记忆 | 会话可实际调用现有工具；建议关联来源、照片和图像版本，长期偏好由人确认，推断不自动成为永久事实 |
| 2 | 专业色彩与高保真影像 | ICC 读取/保留、色彩空间转换、16-bit/高精度处理、高分辨率分级/瓦片预览；明确原图、编辑版本和显示预览边界 | 颜色元数据、变换和高精度导出可验证；大图处理受控，原图保持只读；真实设备显示一致性另验 |
| 3 | 摄影书制作 | 多图版式、跨页、文字与图注、页面/纸张尺寸、出血与安全区、布局版本、PDF 导出及印前检查 | 选片序列能形成可编辑书稿并导出；检查缺图、低分辨率、溢出与规格问题，保留正式布局版本 |
| 4 | 展览策划 | 展墙尺寸、作品实际尺寸与间距、装框/排列、观看路径、展览草案和清单/布局导出 | 从同一项目建立可编辑展览方案，按比例安排并导出可核对尺寸的布局和作品清单 |
| 5 | 专业协作审稿 | 编辑/策展人/导师角色、版本绑定批注、讨论/回复、建议修改、批准和交付状态 | 建议与正式作品分离；按角色授权，审稿绑定具体版本，接受修改可追溯并可恢复，撤权与冲突有明确结果 |
| 6 | Lightroom / Capture One 编辑回流 | 资产与编辑版本关联、导出/外部编辑/回导、元数据与色彩配置保留、版本比较、重复回导与失败恢复 | 一个项目完整完成往返；回导形成可追溯版本，匹配不明时要求人工确认，不覆盖原片或静默串图 |
| 7 | AI 与作品草案融合 | 原计划第 5 项：独立叙事方案、差异比较、局部修订、部分采纳、三方冲突、Undo/Redo；逐步接入前述书稿与展览草案 | AI 建议能在作品中预览、讨论、修改和部分接受；正式修改始终由人工操作提交，预览与采纳一致 |

当前阶段编号对应本表工作包；此前取消的旧编号 1/6/7 仍分别指统一入口、跨端接续新增开发、独立评测与业务指标平台，不因重新排序恢复。

### 范围与验证

- 已取消：旧计划的统一聊天/任务入口、跨设备接续新增开发、独立摄影 Benchmark/盲评平台与业务指标工具。
- 保留：专业色彩、编辑回流、摄影书、展览、专业协作，以及摄影知识、视觉分析、长期记忆、AI 叙事草案。
- 每个工作包都需通过适用的单元/契约/本机集成测试和构建；取消独立评测平台不等于取消工程测试。
- 真实素材和真实软件往返、专业显示/打印、印前、多人真实审稿、真实模型质量及发布分别验收，代码检查不能替代这些结论。
- 专业协作不包含服务交易市场、支付分成或消费者信息流；Android 扩张和摄影基础模型自研继续暂缓。
- 用户此次调整不授权生产调用、生产 schema 变更、推送或部署。

## 分支与基线

- 用户计划基于 `e16a8e2`；实际启动基线为 `edabaaae`。
- 共享开发分支：`codex/focalroom-v1.5-reliability`。桌面与 iPhone 共用 Rust Agent，先在同一分支修复共享链路，避免两份实现分叉。
- 启动时已有 iOS 多语言、项目配置与测试脚本的未提交改动；保留这些工作，不将它们计入本轮交付。
- 版本号是验收阶段，不是发布日期，也不在启动时修改安装包版本。

## 阶段门槛

| 阶段 | 工作范围 | 验收门槛 |
| --- | --- | --- |
| v1.5 | CI、知识身份、编码、错误恢复、重复哈希、画布回归 | 主分支 CI 全绿；核心流程无预期失败；Mac/iOS 授权知识调用能区分无结果与异常 |
| v1.6 | 统一自然语言入口、摄影知识与引用、分层视觉、可追溯记忆 | 选中照片后实际检索、观察并给出可追溯建议；推断不得自动成为永久偏好 |
| v1.7 | 独立叙事草案、差异比较、部分采纳、事务撤销、三方合并 | 预览与采纳一致；保留人工编辑；未授权不得修改正式画布 |
| v1.8 | 项目与会话接力、离线队列、权威版本、冲突、iPad 创作 | Mac/iPhone 同项目继续讨论；离线恢复无重复副作用；iPhone 聚焦看图、选片和审稿 |
| v1.9 | 专业色彩、编辑回流、摄影书、展览 | 版本关联与回流可靠；色彩及印前通过真实素材和设备验收 |
| v2.0 | 编辑/导师协作、批准、呈现与创作服务 | 协作权限和作品完整性可验证，商业化建立在已有创作闭环上 |

## Sprint 1：可靠性与知识服务

| 任务 | 当前状态 | 后续验证 |
| --- | --- | --- |
| CI lint | 已修复 iOS 验证脚本空 catch；验证结果见下方 | 本地检查不替代远端完整 CI |
| 跨端 KnowledgeClient | 已在真实服务与真实钥匙串上验收（Mac 与手机同一核心） | 注入当前 CloudAuth；绑定任务账户/服务；禁止全局 Keychain 回退及跨来源凭证发送；覆盖登出、换号、撤权；真机界面点选仍待人工一次 |
| 中文检索与流派 | 已完成：UTF-8 编码、SQL 流派参数、空串流派均在真实 PostgreSQL 上复验 | 严格工具契约下模型总带流派字段，空串必须视为"不筛选"，否则服务端参数校验会把正常检索报成异常 |
| iOS 重复内容哈希 | 已完成：同内容不同来源在项目内保留独立记录，重复来源仍跳过 | 迁移 0037 重建照片表去掉项目内内容唯一约束；`source_assets` 项目内唯一保持不变，第二张照片复用同一母版资产；桌面与 iOS 各自新增回归 |
| 四个画布预期失败 | 已完成：四条旅程全部通过并移除 `test.fail` | 根因一是版本冲突时先关便签编辑器丢掉已输入文字；根因二是连接/墨迹的透明加粗命中线压在节点之上吞掉点击；另补可见形状类名，避免把命中线算进节点数量 |
| 知识与 Agent 调用 | 已完成：真实账户会话 → 真实服务 → 真实知识库 → 工具回执全链路，两端各一条 | 剩同账户真机验收与生产服务；修好空串流派后三条真实链路已在同一轮全部重跑通过 |

AI 请求恢复继续沿用已有持久回执与 UNKNOWN 边界：断流或结果不明不自动重发，不猜测退款；补测重复请求、确认中结果及计费对账。

## Sprint 2：专业创作闭环

1. 核对最新桌面/iOS 的聊天和任务入口实际行为，再统一路由；历史文档不能替代现状测试。
2. 摄影知识与来源接入实际会话；优先新彩色、私人日记、街头纪实、类型学景观、当代摄影书。先核实质量和图像展示授权，再扩量。
3. 项目概览 → 批量观察 → 重点检查 → 序列比较；证据绑定照片 ID、图像版本和实际观察。
4. 区分确认偏好、项目上下文、当前反馈及待验证摘要；记忆读写记录来源与适用范围。
5. 建立独立画布草案，展示编辑理由、不确定性和变更列表，验证部分采纳和撤销。
6. 建立 Photography Benchmark 与普通同模型对话的盲评，保留失败任务分母。

## 核心验收与指标

使用获授权的 200 张真实照片和创作意图，生成三个不同方向、每份约 20 张的可审查草案，附摄影史参考和取舍理由。摄影师能够继续讨论、局部修改、部分采纳并撤销；原项目不受损。跨版本验收包含第二天在 iPhone 接续同项目讨论。

记录创作任务完成率、有效建议采纳率、7 日创作回访率、创作闭环率。先冻结事件和分母定义，建立真实基线，再设业务目标。安全门槛为未经授权正式修改、不可恢复丢失、跨账户泄露均为零。

暂缓：未经校审的资料扩量、复杂多 Agent 架构、消费者信息流、服务交易平台、iPhone 桌面功能全复制、摄影基础模型自研。Android 不阻塞核心创作体验。

## 启动增量验证

- 本地隔离 PostgreSQL 知识 API：15/15 通过，包含中文姓名与不同流派的正反例。
- Rust 编码回归：无默认特性及桌面默认特性各 1/1 通过；覆盖中文、保留字符与查询参数边界。
- 前端单元测试：59 个文件、414/414 通过。
- 全仓库 Lint、类型、契约、Rust 格式、git diff 空白检查通过。首次 Lint 暴露已有未提交 i18n 脚本的三个未使用变量错误；本轮仅简化等价替换回调并去掉未使用常量，复验通过。
- 复现命令（在 between-frames 下）：`npm run lint`、`npm run typecheck`、`npm run contracts:check`、`npm test`、`node apps/api/test/run.mjs --test apps/api/test/knowledge.mjs`、`cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target-v1 --lib agent::knowledge::tests`（无默认特性另加 `--no-default-features`）、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`。
- PostgreSQL 临时证据目录：`/var/folders/8v/3tww3vkx10s77klhf8y_7hz80000gn/T/focalroom-pg-tests-9u2usv`。前端运行产生既有 `--localstorage-file` 警告，但测试无失败。
- 尚未完成整个 Sprint 1；尚未执行真实账户/设备、生产知识接口或模型质量验收。
- 本轮无提交、推送、部署、生产数据修改或数据库 schema 改动。

工程原则：复用既有 UTF-8 编码函数（DRY），以最小参数修正保持 API 契约（KISS），不添加新依赖或提前构建后续模块（YAGNI）。共享身份客户端后续单独承担认证与 HTTP 错误映射，工具层继续只处理摄影语义（单一职责）。

## 第二批增量：共享身份与知识工具执行

继续工作期间，外部会话将 HEAD 推进至 28258eb1（桌面版本 1.5.0）；本聊天未执行提交或版本推进。Cargo 校验将锁文件中本包版本同步为 1.5.0，无依赖升级。

- OpenAiProvider 将同一 CloudAuth 注入知识客户端，桌面与原生 iOS 使用相同核心。知识客户端不读取系统全局 Keychain，也不接受环境变量指定的独立知识端点。
- 地址只由已校验的登录服务构造；HTTP 禁止重定向，读取响应有 256 KiB 上限，错误不携带服务原文、查询 URL 或 token。
- 托管新会话冻结三个专业知识工具的授权；无账户会话保留本地方法札记；旧任务不追授新权限。
- 联网前校验任务状态、照片项目范围、工具权限、预算、已存回执和账户/服务绑定；网络在 SQLite 写事务外执行，写回时再次核对当前任务和账户。回放已有回执不发起知识请求。
- available=true 且空列表表示无匹配；401/403/429/5xx、连接失败、非法 JSON/结构或响应过大均返回明确错误。混合检索仍可返回本地札记，并附 cloudAvailable 与 cloudError。
- 搜索摘要保留审校状态，不再统一宣称结果全部已核实；摄影书工具暂只返回检索记录，不冒充已经取得编排结构或来源详情。详情和引用仍属于后续专业化工作。

验证覆盖：四个知识工具的真实本机 HTTP、内存会话认证、中文/特殊字符、无结果与异常区分、禁止重定向、响应上限、登出/换号、任务账户与服务匹配，以及真实 SQLite 工具执行、回执重放、同调用参数冲突和跨项目拒绝。未调用真实模型或生产服务，不能替代 Mac/iPhone 同账户验收。

回归过程中发现并修正：
1. 测试 HTTP 服务的监听器非阻塞状态会影响接受的 socket，已显式切回阻塞读取，避免夹具偶发 WouldBlock。
2. 初次把专业云知识工具放入所有创作会话，使无账户的 300 图回归超出既定上下文预算；已按账户能力限定新工具，不提高预算或删减原有验收断言。
3. 外部新增的两个 iOS 多语言脚本触发 no-control-regex；改用等价的字符串 replaceAll，Lint 复验通过。

剩余工作仍包括重复内容哈希的数据兼容方案、四项画布 E2E 恢复、真实授权双端旅程，以及知识实体详情/引用和真实摄影质量评测。

第二批验证结果：
- 共享原生核心：182 passed / 2 ignored；桌面核心：179 passed / 2 ignored。两项忽略均为既有 Apple 本地语义资源测试。
- 包含 5 项知识定向回归及既有 300 图三草案、原图保护、画布审稿、任务恢复、账户与创作记忆边界测试；并非全仓库 E2E 或真机验收。
- 全仓库 Lint、类型、契约、Rust 格式与差异空白检查通过。
- 测试日志位于 /tmp/focalroom-sprint1-native-tests.log 与 /tmp/focalroom-sprint1-desktop-tests.log。
- 全量回归还暴露工具授权早于模型身份冻结的顺序问题，已在同一事务中先绑定身份再生成授权；复验包含新会话工具可见性与换号后回执拒绝。
- 实现复用 CloudAuth 和现有权限/回执通道，知识客户端集中处理 HTTP 与认证，工具层只组装摄影结果，无新增依赖或数据库迁移。
- 共享原生核心严格 Clippy（-D warnings）通过，日志：/tmp/focalroom-sprint1-clippy.log。

## 第三批增量：重复内容兼容与画布旅程恢复

### 重复内容哈希

- 迁移 0037 只增不改：重建照片表去掉项目内内容唯一约束，保留导入序号唯一约束，改为普通索引支撑按哈希查询；重建前分离、重建后原样恢复引用照片表的触发器，沿用 0034 先例。
- 升级窗口在数据库连接上临时关闭外键检查，迁移完成后无论成败都恢复；一旦重建后仍存在悬挂引用即停止升级，配合既有升级前备份。
- 导入判重从"画面内容相同"改为"同一份来源再次导入"：桌面按原文件路径，手机按系统相册资产标识或同名同字节文件。相同内容但来源不同保留两条独立记录，各有决定与序列位置；重复 ID 与同一文件重复导入仍拒绝。
- 资产母版的项目内内容唯一性不变：第二条照片复用同一母版，派生缓存按哈希共用，删除项目时只清理真正无人引用的文件。
- 手机库清单校验取消"项目内哈希必须唯一"；手机项目上下文同步同样允许同哈希多记录，并按同一母版登记别名。

### 画布四项预期失败

1. 便签提交先关闭编辑器再发命令，版本冲突时已输入文字一起丢失；改为命令确认落地后才关闭。
2. 连接线与墨迹各叠一条透明加粗命中线用于点选，且按节点创建顺序叠放，正好压在自己连入的照片与便签中心，抢走点击；向量节点统一置于节点之下，与组合框节点一致。
3. 端到端断言原本统计所有形状元素，会把命中线算成第二条连线；为可见连线与可见墨迹补稳定类名，断言只针对可见形状，数量与像素视口检查强度不变。

### 验证结果

- 桌面核心：392 passed / 19 ignored；共享原生核心：390 passed / 19 ignored；Rust 格式检查通过。
- 手机共享核心 Swift 套件：75 个用例通过、10 个跳过、0 失败，含新的同内容双记录独立状态与顺序回归。
- 前端单元测试 414/414、Lint、类型、契约检查通过。
- 集成浏览器套件 13/13 通过（1.8 分钟），覆盖 200 图真实导入、既有旅程与四条画布旅程；`e2e/v3-workspace.spec.ts` 的四处预期失败标记已全部移除。
- 本机 1420 端口被无关第三方应用占用，本次集成验证临时使用 1430，未改动仓库默认端口与配置。
- 服务端知识搜索路径没有速率限制，真实 429 仍只能由客户端夹具覆盖；文档证据目录截图由本次集成运行重新生成。
- 剩余：真实账户与设备的同账户 Mac/iPhone 授权知识验收；本轮未提交、推送或部署。

## 第四批增量：真实知识链路双端验收

### 验收方式

- 新增一条只在本机显式运行的验收流程：临时 PostgreSQL 集群 + 真实共享服务 + 只写入本轮临时库的公开摄影史条目，再由测试进程调用发版用的同一份 Rust 客户端。运行命令为 `LC_ALL=en_US.UTF-8 node apps/api/test/run.mjs apps/api/test/knowledge-desktop.mjs`；不调用模型、不上传照片、不接触生产库与生产服务。
- 共享服务必须绑定在自己的真实端口上启动：桌面客户端会逐字节比对服务返回的登录地址，端口不一致会被判成"登录来源不匹配"。
- 验收用例只写自己随机命名的新钥匙串条目，绝不读取或改写用户已有的登录条目。早先一版为了证明"没动用户会话"而先读了那一条，结果触发系统钥匙串授权弹窗并卡住整轮验证，已改为完全不碰。

### Mac 端

- 真实系统钥匙串：PKCE 发起 → 浏览器侧授权 → 深链兑换 → 会话落在钥匙串 → 重启读取 → 登出后条目消失。
- 真实知识库：中文姓名命中、流派数组过滤命中、流派空串等同不过滤、查不到时返回空列表且仍算可用、服务端注销同一会话后报异常码而不是空结果、本地登出后不再发出任何请求。
- 四个知识工具在真实服务上的形状检查：检索摘要不作为指令、不作为视觉证据、结果里不出现会话令牌；本地札记与云端记录合并后确实带出云端条目。

### 手机端

- 用与打包手机核心相同的编译开关（不含桌面依赖），走应用真正调用的原生入口：应用侧令牌交给共享核心 → 服务核实身份 → 按账户隔离工作目录 → 真实工具回执、预算与权限链 → 同样的命中、空结果与撤权异常三种情形。
- 这一端证明的是"手机用的就是桌面那套代码与那条服务链路"；真机或模拟器上由人点一次手机界面知识调用、以及生产账户验收，仍未做。

### 验收暴露并修复的问题

- 工具契约是严格模式，`search_photographers` 必须同时给查询、项目与流派三个参数；模型不想过滤时送空串。客户端此前把空串原样拼进请求，服务端按"流派至少一个字符"拒绝整次检索，于是一次正常查询被报成"知识服务不可用"——正是本项要防的"把异常当无结果、把无结果当异常"的反向情形。现在空串（含纯空白）视为不过滤，离线夹具补了"空串不拼参数"的断言，真实库补了"空串结果与不过滤结果一致"的断言。

### 验证结果

- 桌面核心全量单元测试 180 passed / 3 ignored（含新增空串用例），共享原生核心 183 passed / 4 ignored，Rust 格式检查通过，前端 Lint、类型与契约检查通过。
- 显式批准的真实用例：Mac 钥匙串读写删闭环通过；原生登录协议全流程在同一次真实服务上通过；手机侧真实链路通过（第一次因空串流派失败，修复后通过）。
- 修复空串流派之后，三条真实链路在同一次运行里全部重跑通过（`tests 3 / pass 3 / fail 0`，验收器汇总 `"passed":true`）：Mac 端真实钥匙串会话读已播种知识、手机侧原生入口同一链路、原生登录协议与钥匙串条目闭环。此前中断最后一轮的"磁盘写满"已随空间清理消失，该轮重跑不再依赖旧结果。
- 本轮未提交、推送或部署。

## 第五批增量：Sprint 2 入口现状核查与统一路由

### 核查方式

三路并行只读核查（桌面界面与命令层、iOS Swift 与原生入口、共享 Rust 核心），承重结论逐条回读代码确认，不以历史文档或代码注释为准。

### 现状（2026-11-03 实测代码）

- 桌面只有一个 AI 抽屉：`src/features/ai-editor/AIPanel.tsx` 由 `src/app/AppController.tsx:209` 挂载，项目头、选片快捷动作、画布"整理画板"、状态条与快捷键都汇入它，没有第二个界面级入口。
- 但抽屉内部仍是两种模式，走两条不同的推理路。任务模式：`agent_request`（`src-tauri/src/commands.rs:703`）→ `AgentEngine::request`（`agent/runtime.rs:111`）→ `lifecycle::request_bound_with_canvas`，有任务行、回执、预算与账户核对。聊天模式：`agent_chat` / `agent_chat_stream`（`commands.rs:666,675`）→ `chat_turn` / `chat_turn_stream`（`runtime.rs:33,53`）→ 供应商 `chat_text*`，不建任务、不写回执、不占预算、不带项目范围与 `expectedAccountId`，聊天历史只存在前端 `uiStore.aiChat`。
- iOS 同样只有一个入口 `FocalAIView`（`apps/mobile-ios/FocalRoom/RootView.swift:18`），内部 `.task` / `.chat` 分段。`.task` 经 FFI `agentRequest` 进共享核心（`AppModel.swift:693,1391`）；`.chat` 默认打远端流式端点 `POST api/v2/ai/.../responses/stream`（`AppModel.swift:713` → `Sources/FocalRoomCore/APIClient.swift:430-486`），仅当返回 404 且一个增量都没收到时才回退核心 `chat`（`AppModel.swift:738-744`）。也就是说手机上的"聊天"目前由另一套客户端规则执行：核心侧统一 1024 输出上限（`runtime.rs:47,48`），手机请求写的是 `maxOutputTokens: 2048`。
- iOS 侧确认存在的孤儿与缺口：`api.aiSubmit` 只被测试引用；FFI 的 `createProject` Swift 从不调用；`sourceBoard` 恒为 null，`continuation` 与 `allowQueuedRecovery` 从不发送（`AppModel.swift:1418-1429`），因此跨轮接续、排队恢复、画布来源三项只有桌面能用。手机界面没有把知识工具的"查无结果"与"服务异常"分开呈现：观察区为空时整块隐藏，错误统一塌成一条字符串，只有 `AI_RESULT_UNKNOWN` 有专门文案（`FocalAIViews.swift:353,392,399`）。
- 核心的权限口子：桌面 `request()` 硬编码 `canvas = true`（`runtime.rs:112`），原生宿主用 `request_with_canvas_access()` 收紧（`native_agent.rs:217`）。而 `TaskCommand::StartCanvasTask` 原本把画布授权写成恒定 `true`（`agent/lifecycle.rs:143`），于是 C ABI 的任何宿主都能靠"挑一个自带权限的命令"绕过 `canvasAccess=false`。
- 已无生产调用方：`AgentEngine::run`（只由 `dispatch` 与测试触达）、`lifecycle::request`（仅 mock 分支）、`lifecycle::request_bound`（仅测试）、`agent::focal_chat` 是投影而非推理通道。

### 已修

- `StartCanvasTask` 不再自带权限：画布授权改为跟随宿主收紧后的同一判定（`canvas_access` 且项目本机可写）。无法授予画布时当场拒绝并返回权限错误，不留半个任务或已冻结的授权行。`e2e/editorial-review.spec.ts:47` 的非认证路径仍在用这个命令，其项目为本机可写，行为不变。
- 桌面聊天的回复现在带一个"发送前就生成并留存"的请求编号（Sprint 2-1b/1c）：
  - 编号由界面在第一次 `await` 之前用 `crypto.randomUUID()` 生成，随 `agent_chat` / `agent_chat_stream` 一起交给核心，核心不再自己临时造一个（`src-tauri/src/agent/openai/hosted_stream.rs:83`，此前在服务内部生成后即丢弃，回执查无此人）。
  - 服务端确实按这个编号留回执：`apps/api/src/hosted-ai.ts:222` 把每一趟托管请求以 `x-focalroom-request-id` 记入 `fr_ai_requests`，一次性、异步与流式三条路共用同一张表，因此 `GET /responses/async/{编号}` 可以只读核对聊天请求。
  - 网关没有流式路由（404/405/501）而退回提交路径时**沿用同一编号**（`agent/openai.rs` `chat_text_stream`），换编号会让手里那串永远查不到回执；除此之外一律不重发。
  - 新增只读核对通道：`OpenAiProvider::chat_job` → `runtime::chat_job` → `agent_chat_job`（`commands.rs:692`），不要求视觉能力（聊天不挂工具也不带图），输出上限与提交路径同一常量 `CHAT_OUTPUT_TOKENS`（1024），避免发送与核对各定一个门槛。
  - 界面不再邀请盲重发：`AI_RESULT_UNKNOWN` 时保留半成品气泡与原文、只出现"核对结果"按钮；核对到 `completed` 才用服务端正文整条覆盖，`pending`/`unknown` 维持未确认，只有服务端明确 `rejected`/`expired` 才把文字退回输入框。此前任何失败都撤回原文并塞回输入框，等价于请用户为同一条回复付两次。
  - 编号不合法（非 UUID）在联网之前拒绝，四种界面状态各有断言；`m1-harness` 的核对桩如实回"未确认"，不编造正文。
- 桌面聊天与原生聊天合并为同一个入口函数：`AgentEngine::chat_turn` 已删除，两条路都走 `chat_turn_with_id`（`agent/runtime.rs:33`），不存在"一条可核对一条不可核对"的并行实现。

### 统一路由的决定

1. 一个入口只有一个执行者：手机上的纯问答改由共享核心执行，并保持逐字上屏（需给原生入口增加增量回调），远端流式端点不再是手机的默认聊天通道；桌面的聊天模式并入同一生命周期核实（账户、输出上限、可核对回执）。理由是同一功能两套客户端各自定规矩时，计量、撤权与错误语义必然分叉。
2. 权限只由宿主身份与项目状态决定，命令字面不能自带权限。
3. 桌面能传而手机传不出来的字段（画布来源、跨轮接续、排队恢复）要么在手机上补齐，要么明确禁用并说明原因，不允许静默按桌面行为预期却实际丢失。

### 验证结果

- Rust 全量测试 393 passed / 0 failed / 20 ignored（默认特性），托管链路 `tests/hosted_ai.rs` 21 passed / 0 failed（`--features integration-harness`），共享原生核心 `--lib --no-default-features --features native-agent` 184 passed / 0 failed / 4 ignored，`cargo fmt --check` 通过。
- 前端 Lint、类型检查、契约检查全部通过；`npm test` 59 个文件 / 418 个用例通过，含本轮新增的四条：`聊天两条发送路都带上发送前留存的编号，断流后凭它只读核对`、`聊天回执只在已完成时接受正文，其余状态必须空正文`、`断流结果不明时不撤回原文也不重发，只给核对入口`、`核对到确认中与仍不明都保持未确认，只有服务端确认没受理才退还原文`。
- 覆盖到的是契约与界面逻辑：断流、核对、盲重发三类行为由离线夹具模拟服务端回执验证（`hosted_ai.rs` 的流分支按客户端编号记账，与真实网关 `fr_ai_requests` 的记账方式一致）。**没有**在真实托管网关上真的掐断一次流来核对回执，也**没有**装到本机 Make 上做过人工点击验证——这两项仍待办。
- 权限口子（`StartCanvasTask`）的验证在原生宿主测试里：`readonly_host_cannot_open_a_canvas_task_without_canvas_grant` 断言拒绝时 `agent_tasks` 与 `agent_task_grants` 均为 0，即拒绝干净、不留半个任务或冻结授权。
- 统一路由的三条决定只落了第 2 条与第 1 条的一半：桌面聊天已有可核对回执，手机聊天仍打远端流式端点、`maxOutputTokens: 2048` 与核心的 1024 不一致，第 3 条（手机缺失字段）尚未动。
- 本轮未提交、推送或部署。


## 第六批增量：iPhone 聊天接入共享流式与回执核对

基于第五批的工作区继续开发；先重新读取实际 Swift/FFI/Rust 调用路径，未把粘贴的历史结果当作当前验证。

### 实现

- 增加同步回调 C ABI focalroom_agent_chat_stream，复用 AgentEngine::chat_turn_stream。回调仅传 UTF-8 增量，生命周期限定为函数返回前；Swift 复制字符串并通过 AsyncThrowingStream 顺序交付，不在主线程阻塞 Rust 调用。
- 手机 aiSend 的纯聊天发送不再直接调用 APIClient.aiStream，也删除“手机判断 404 后再回退”的分支。请求编号先持久化，提示词、输出上限（1024）、流解析、路由缺失回退规则和账户核对均由共享核心执行。
- 增加原生 chatJob 操作；手机待确认消息通过同一核心只读查询，不再自己调用 APIClient.aiJob 或按另一套输出上限解读回执。
- Swift 仅在 completed 事件后保存正式回答；收到增量而没有最终回执时，原用户消息保留 unknown，不以半截文字冒充完成，也不自动重发。
- 增加 NativeChatJob 严格解析：completed 必须包含非空正文，其余四种状态必须没有正文。expired 的提示独立于 rejected，不再错误声称已释放额度，补齐中英日韩文案。
- Core 网络/身份逻辑、Swift 流事件适配和 UI 持久化各负其责；复用现有执行与计费通道，无新增依赖、数据库 schema 或第二套手机推理规则。

### 本轮验证

- 共享原生核心全库：185 passed / 0 failed / 4 ignored。
- 新增 C ABI 本机网关回归：正常流得到增量和最终正文；截断流返回 AI_RESULT_UNKNOWN；使用相同编号读取完整回执，提交次数不增加；输出上限断言为 1024。
- 原生核对夹具第一次缺少 jobId，被严格协议检查拒绝；补齐与真实服务一致的回执后通过，未削弱客户端校验。
- Swift 核心原有全套：75 executed / 10 skipped / 0 failures，另有 Swift Testing 的 3 项通过；新增 NativeChatJobTests 1 项通过，覆盖五种状态与非法正文组合。
- 共享原生严格 Clippy、Rust 格式、机器契约和 git diff 空白检查通过。
- 首次通用模拟器双架构构建失败；arm64 编译发现调用 aiPayload 缺少 try，已修复并复验。构建日志仍包含既有未使用变量/不可达返回及 Swift driver 的异常诊断，最终状态见后续构建记录。

### 仍需推进

- 本轮保留聊天/任务模式选择；它们都接到共享核心，并不代表已经实现自动 Agent 工具路由。
- 手机 sourceBoard、continuation、allowQueuedRecovery 的产品入口仍待补齐或明确限制。
- 真实托管网关断流、真实手机手动点击、Mac 安装包验收尚未执行。本轮仅本机夹具和模拟器编译，不调用生产模型或扣减用户额度。
- 生产调用属于用户要求显式确认的操作，应在确定测试账户、调用次数和费用边界后再执行。未提交、推送、部署或安装到用户设备。

最终模拟器构建：arm64、Debug、CODE_SIGNING_ALLOWED=NO 复验返回 exit 0，并明确输出 BUILD SUCCEEDED。产物位于 /tmp/focalroom-sprint2-ios-build/Build/Products/Debug-iphonesimulator/FocalRoom.app；未启动或安装到设备。日志：/tmp/focalroom-sprint2-xcode-verify.log。前述通用构建失败与诊断保留，不以成功复验覆盖历史。

## 第七批增量：工作包 1 首项——摄影知识与来源引用接入实际会话

按新的排序先做工作包 1 的"知识详情与来源引用"。核对现有代码后确认这不是重复建设：本地方法札记已经有编号、版本、作者与 sha256，三种云端知识工具却只返回摘要行（id、标题、审校状态），断言正文和出处从来没进过会话，也没有任何 Tauri 命令把知识详情单独交给前端。因此本批只补这一段缺口，不新增工具、不新增命令、不改数据库 schema、不加依赖。

### 实现

- 给 KnowledgeClient 增加一条实体详情出口，并与检索共用同一个传输函数：地址只由已校验的登录服务构造、禁止跟随重定向、响应上限 256 KiB、错误信息不带服务端原文/查询串/令牌、网络前后各核对一次账户。未知实体种类、空 id 和超过 160 字符的 id 在发出请求前拒绝。
- 检索命中只为最前两条补取详情，返回 `citations`（断言正文、claim_type、review_status、disputed、逐条 sources 的 url/机构/核验日期，加实体级 sources）或 `detailError`。裁剪上限：断言 4 条、正文 400 字、每条出处 3 个、url 300、机构 120，单条回执小于 8 KiB。取不到详情时保留已检索到的条目并把原因写进同一条结果，"没有出处记录"和"没有这条实体"在界面上仍是两件事。
- `creative::references()` 把三种云端工具的不可变收据一并投影进任务视图，键为 `cloud:{id}` 与 `cloud:{id}:{断言指纹}`，同一条断言重复检索只留一份；`reviewStatus` 与 `disputed` 原样带上，`visualEvidence` 保持 false。工具回执注明只给最前两条补取详情，imported 只表示已导入。
- 桌面"查看参考来源"新增四件事：审校状态标签、有争议标记、"出处： 机构 · 地址"一行、"出处未取得： 原因"一行。中文标签只覆盖 imported/verified/editor_reviewed 三种已知取值，陌生取值直接露出服务端原文，不替它编一个"已核实"。中英日韩各补 7 条文案。
- 提示词在既有知识段落里加一句约束：引用要说清依据哪条断言，imported 不等于已核实，disputed 要连着争议说明，citations 为空或带 detailError 就直说没取到出处，不补一个看起来权威的来源。
- 前端契约同步：`domain/agent.ts` 类型与 `TauriAgentRepository` 的 zod 校验允许这些新字段，非字符串的错误码仍会被整条投影拒绝，避免把结构错误的"出处"渲染成可信信息。

### 验证结果

- Rust 默认特性全目标：396 passed / 0 failed / 20 ignored（40 个测试二进制）；仅 `--lib` 为 183 passed / 0 failed / 3 ignored。
- 托管链路 `--test hosted_ai --features integration-harness`：21 passed / 0 failed。共享原生核心 `--lib --no-default-features --features native-agent`：188 passed / 0 failed / 4 ignored。`cargo fmt --check` 无差异。
- 本批新增或改写的知识测试：详情失败保留检索结果且不编造出处（403/404 → `detailError`，仍带 reviewStatus，不泄露服务端原文与令牌，不跟随重定向）；未知实体种类在发出请求前拒绝（只发 1 次知识请求）；回执裁剪（7 条断言→4、600 字→400、5 个出处→3、单条回执 < 8 KiB）；注入会话夹具断言 10 次请求里 4 次是 `GET /api/v2/knowledge/photographers/one`；真实工具管线断言账户核对与请求次数（auth + search + detail 共 3 次）并检查 `creative::references()` 投影出的地址带 `reviewStatus=verified`、`visualEvidence=false`，另一条同时是 `disputed=true` 与 `reviewStatus=imported`。
- 前端：59 个测试文件 / 420 项通过（本批新增 2 项）。新增桌面渲染断言"云端知识的出处与审校状态一起上屏，未核实与存疑不伪装成已核实"，以及契约断言"知识出处、审校状态与未取得原因原样过界，字段变形时整条投影被拒"（非字符串错误码会被拒绝）。`npm run lint`、`typecheck`、`contracts:check` 全部通过。
- 过程中修掉一处自己引入的 JSX 括号缺失（版本标签 `</small>` 后漏一个 `}`），它会让整个 AIPanel 模块在打包前解析失败；已定位并复验。

### 一处需要拍板的预算余量

- 300 张照片的画布规模测试在默认预算下曾暂停：第 10 轮上下文估算 48328 字节，超过 `max_context_tokens` 的 48000，引擎按设计不发送请求、保留进度并提示缩小范围（`request_not_sent=true`）。把本批提示词段落从 580 字节压到 194 字节后恢复通过，但它当时只剩 252 字节余量；同一工作区里其他会话的语言跟随提示词同期还在增长（本机当前测得 agent-system +435、canvas +132、focal-agent +378、focal-chat +315、vision-observe +685 字节）。
- 这条断言对本批最终源码连续复验 6 次（桌面特性 3 次、原生特性 3 次）全部通过；此前原生特性出现过一次失败，当时工作区的提示词正被并发编辑，无法把那次失败归到本批源码。这说明它不是稳定的绿，而是卡在阈值上。
- 更深一层：`max_context_tokens` 在分层预算里按"令牌 × 4 字节"使用（48000 → 134400 字节），在请求上限里却直接与字节估算比较（48000 字节），同一个数值两套含义。抬高上限会放大单次大项目分析的花费与延迟，收紧则是明确"300 张一次做完不受支持"。这两条本批都没有擅自改动：默认预算与这条规模断言都保持原样，需要用户决定后再动。

### 仍需推进

- iOS 端没有"参考来源"面板，云端出处目前只在桌面抽屉可见；手机端接入属于后续工作。
- 详情链路只在本地 HTTP 夹具上验过路径、状态码、裁剪、账户核对与不跟重定向；未在真实托管服务和真实账户上实际取过一次实体详情，也未验证真实模型是否按新提示词逐条引用出处。
- 未重新打包安装到本机 Mac，未做人工点击验收；未提交、未推送、未部署。工作区仍含其他会话的未提交改动（iOS、若干提示词、模拟器构建产物与截图），不计入本批。

## 第八批增量：工作包 1 第二项——分层视觉观察与"看的到底是哪一张图"

先核对现有代码再决定做什么：视觉工具已有两个（`inspect_photos` 1–12 张、`compare_photos` 2–4 张加简要意图），观察结果按 `(photo_id, 原图 sha256, 型号, 提示词版本, kind)` 缓存在本地 `photo_observations`，不可变回执写在 `tool_calls.result_json`。缺的不是第三次工具调用，而是缓存键的信任前提：真正发给模型的从来不是原图，而是从 `preview_path` 现压出来的 512px / q75 JPEG，这个派生字节的指纹在整条链路上没有任何地方记录。两条真实路径会造成"原图指纹没变、发出去的图变了"：`resource_commands.rs:145 relink_photo` 只断言原图摘要相同就替换预览文件，以及预览生成管线本身改过解码合成（预乘透明修复）。那时旧观察说的是另一张图，界面却照样显示成已核对的历史观察。因此本批只补这一层证据绑定，不新增视觉工具。加字段需要用户拍板（已选"加字段"）：服务器共享库没有这张表，改动只落在桌面与手机各自的本地 SQLite。

### 实现

- 新增 `0038_observation_preview_hash.sql`：给 `photo_observations` 加可空列 `preview_sha256`，不回填旧行——旧观察的版本是真的没记过，不能用"补一个值"把它伪装成记过。迁移只在本地链（`cloud/migrations` 无此表），升级前仍走 `VACUUM INTO` 备份 + `integrity_check` + `foreign_key_check`。
- `upload_compress` 拆出 `compress_and_fingerprint`：一次压缩同时返回字节与其 sha256，指纹描述的是真正离开机器的字节；`compress_for_upload` 改为复用它，出图行为不变。
- `vision::prefetch` 在缓存判定阶段就为每张候选照片算出当前派生指纹，复用规则分三种：记录过且与当前一致 → `verified`，正常复用；记录过但对不上 → 视为缓存未命中，走原有授权 + 预算重新观察（拿不到授权或提供方不支持视觉时，按既有设计退化成 `mock_metadata_only` 并明确说明没读像素）；记录里没版本，或当前预览读不出来 → 仍然复用、不逼一次重新上传，但状态记成 `unrecorded` / `unreadable`，绝不写成 `verified`。第三种是刻意的：不能因为历史上少记了一列，就悄悄花掉用户的图像配额。
- `tools.rs` 落观察时把 `preview_sha256` 写进新列，不可变回执的 `provenance` 每条补 `preview_sha256` 与 `pixelVersion`；回执新增 `requested_photo_ids`（顺序即判断依据）和一段分层引导：画面细节按"概览 → 抽样检查 → 按顺序比较"逐级收窄，别一次传整批。回执里额外存了请求顺序 ID 集，服务端已校验观察只属于请求集合。
- `focal_chat::inspected` 投影补 `imageVersion { previewSha256, pixelVersion, sourceMatches }`；`sourceMatches` 用一次 SQL 比对当前 `photos.content_sha256`，不重读像素、不调模型。iOS 的 `NativeAIObservation` 用普通 Decodable，新键被忽略，不会崩。
- 桌面"已查看 N 张照片"每条观察下面多一行小字，只有 `verified` 才说"看的是这张图（版本 前8位）"，其余分别说"来自更早的缓存，图像版本没记过""当前预览读不出来，这次没能核对图像""这条观察对应的原图已经换过"。中英日韩各 4 条新文案。
- 前端契约：`domain/agent.ts` 与 `TauriAgentRepository` 的 zod 显式声明新字段（strip 模式下不声明会被静默丢掉），陌生 `pixelVersion` 取值会让整条投影被拒，避免把结构错误的版本信息渲染成可信结论。

### 验证结果

- Rust 默认特性全目标：402 passed / 0 failed / 20 ignored（40 个测试二进制）；仅 `--lib` 为 184 passed / 0 failed / 3 ignored；`--test agent_vision` 52 passed / 0 failed。
- 托管链路 `--test hosted_ai --features integration-harness`：21 passed / 0 failed。共享原生核心 `--lib --no-default-features --features native-agent`：189 passed / 0 failed / 4 ignored（原生构建同样编译新列与迁移）。`cargo fmt --check` 无差异，`cargo clippy --lib` 干净。
- 本批新增测试：`fingerprint_describes_the_bytes_actually_sent_and_moves_with_pixels`（指纹是 64 位十六进制、等于送出字节的 sha256、同图确定、换图像即变）；`regenerated_preview_forces_reobservation_and_binds_sent_pixels`（第一轮所有非空 `preview_sha256` 都等于当时算出的旧指纹 → 覆盖预览文件 → 第二轮图像请求数为 2、旧观察不再复用、新行绑在新指纹上 → 第三轮仍是 2 次且投影显示 `pixelVersion=verified`、`sourceMatches=true`）；`legacy_observations_are_reused_but_never_claimed_as_verified`（把列置空模拟旧库 → 第二轮图像请求数为 1，即不重新上传，投影为 `unrecorded` 且 `previewSha256` 为空 → 再改原图摘要，`sourceMatches` 全部翻假）。
- 修掉两处只钉住迁移条数的断言（`local_core.rs:287`、`diagnostics.rs:68`）从 37 改 38；另外几处 `_sqlx_migrations` 断言针对刻意构造的旧库/备份夹具（2、3、8、11、28），不受影响，保持原值。
- 前端：59 个测试文件 / 422 项通过（本批新增 2 项）。新增契约断言"图像版本证据原样过界：记过的、没记过的、读不出的、原图换过的都不合并，陌生状态整条被拒"，以及桌面渲染断言"图像版本说实话：核对过才说核对，旧缓存、读不出、原图换过都各自点明"。`npm run lint`、`typecheck`、`contracts:check` 全部通过。

### 仍需推进

- 路线图这级的"项目概览"仍未建：它需要第三种 `kind`，而 `photo_observations.kind` 上有 CHECK 约束，等于再来一次迁移。本批只把"抽样检查"和"按顺序比较"两级的证据做扎实。
- 分层引导只出现在真有图像观察的回执里；无授权或提供方不支持视觉时走 `mock_metadata_only`，那条回执不含这段引导。
- 引导文字位于回执 JSON 末尾，而历史压缩只保留最近 6 条工具回执完整、更旧的摘要化为 200 字（`context.rs:421`），所以它是"本轮有效"的提示，不是长期约束；模型是否照做没有评测数据。
- `evaluate_draft` 复用缓存观察的那条 SQL 仍只核对原图指纹、没核对预览版本（`tools.rs:291`），改它要动模型契约字节，本批没动；修订委托（`RevisionObservation`）也还没带预览指纹，接力改方案时的历史观察仍只绑原图。
- 上下文预算那条账没变也没复跑：300 张规模断言（`unified_300_real_photos_three_boards_and_followup`）在默认跑里是 ignored，需要显式批准的端点、凭据与真实授权照片，本批没有擅自运行。每条 12 张的观察回执大约多 160 + 490 + 每张 110 字节，最坏约 6 KiB 落在最近 6 条完整回执上。
- iOS 不显示图像版本（新键被 Decodable 忽略，不崩，但手机端看不到这条证据）；未在真实托管服务与真实照片上验过一次"预览换版本就不复用"；未重新打包安装到本机 Mac，未做人工点击验收；未提交、未推送、未部署。工作区仍含其他会话的未提交改动，不计入本批。

## 第九批增量：工作包 1 第三项——长期创作记忆的来源、适用范围与读取回执

先核对现有代码再决定做什么：记忆层已有的东西比路线图描述的多——本项目假设、跨项目已确认偏好、关键词检索、逐条停用与排除、写入回执、暂停接力都在跑。真正缺的是两件"说不清"的事。第一，四种来路在数据里被压成同一张平铺列表：你自己写明的偏好、你点过"确认"的推断，以及纯粹由关键词推断、还没验证的猜测，界面上和送给模型的摘要都长得一样，只写 status；跨项目那条又直接追加进同一个 `human_evidence.hypotheses` 数组，读者分不清"仅本项目"和"来自其他本地项目"。第二，每一轮到底读了哪几条记忆没有任何地方记录：`store::checkpoint` 只冻结 memoryRevision 与 authorMemoryBinding 两个总数，事后既不能核对"这条结论当时确实看过那条记忆"，也不能证明"没看过"。因此本批只补来源与读取证据，不新建记忆存储、不加迁移、不动提示词、不动模型契约。

### 实现

- 假设视图加 `origin` 三分类：`user_stated`（你在备注/偏好里写明的）、`user_confirmed`（你确认过一条推断）、`rule_inferred`（仅关键词推断，待验证）。判定用一条 SQL 从既有的 `preference_evidence.detail_json.explicitPreference` + `hypothesisId` 反查，并连着 `memory_evidence_state.excluded` 一起算——被排除的证据不再计数，这与 `materialize` 的语义一致；`project_local` 与 `cross_project` 两条读取路径（`load_hypotheses`、`row_view`）都 stamp 同一个 `origin_of(status, stated)`，不会出现"跨项目那条没来源"。
- `summary()` 输出紧跟 status 加一个 `origin` 字段，所以进入上下文的 `human_evidence` 摘要和 `retrieve_preferences` 的工具回执都带来源。数组仍是**单层平铺**：分组只发生在界面。原因是 `preference_proposal::filter_for_task` 逐条改写 `memory["hypotheses"]`，用它核对显式偏好的 owner 是否等于绑定账户、photoIds 是否落在 `task.allowed_photo_ids` 内；把它拆成两个键会让这道安全校验静默失效。
- 每轮的记忆读取变成不可变回执：`memory::read_trace()` 从本轮真正进入上下文的假设里投影出 `memoryId / sourceProjectId / scope / origin`，嵌进 `agent_steps` 已有的 `model_attempt` 那一步。这一步不参与模型协议——`store::protocol()` 只取 `kind IN('model','tool')`——所以这条证据**零上下文字节**、不加行、不加表、不迁移，且和花费、速率卡同一行落库，天然带轮次序号。管道是 `runtime.rs` 把 `input.layers["human_evidence"]["hypotheses"]` 原样交给 `agent_attempt`，再由 `execution::attempt` 计算一次 trace（不在两处各算）。
- 桌面记忆面板把列表分成"已确认（来自你写明或确认过的偏好，可作为本项目的判断依据）"与"待验证（由你的操作推断，确认前不作为偏好使用）"两段，每条卡片加 `data-origin` 与一行来源标签，跨项目那条已经写明来源项目、本项目那条明确标"仅本项目"。原有全部操作（确认/移除/恢复/纠正/停用检索/删除/证据与反例展开/查看更旧）逐条保留，卡片渲染抽成一个 `card` 函数复用两段。
- 前端契约显式声明：`domain/memory.ts` 加 `MemoryOrigin` 联合类型与 `origin` 字段，`TauriMemoryRepository` 的 zod 加 `z.enum([...])` 且不给默认值——Rust 端总会发出，陌生取值应整条投影被拒而不是被解释成某个已知来源。iOS 不解码记忆假设（全仓 grep 无引用），新增键对手机端无影响；`contracts/` 下没有记忆投影 schema，故本批不涉及跨端契约文件。
- 顺带把"当前反馈"和"长期记忆"的边界查清楚了，结论是已经分开、无需再动：`context.rs` 给 `layers["conversation"]` 标了 `trusted_as_instructions:false`，待确认的偏好提案以 `preference_proposal` 步骤/聊天卡片存在，不是被注入成记忆。

### 验证结果

- Rust 默认特性全目标：403 passed / 0 failed / 20 ignored（40 个测试二进制）；托管链路 `--test hosted_ai --features integration-harness` 21 passed / 0 failed；共享原生核心 `--lib --no-default-features --features native-agent` 189 passed / 0 failed / 4 ignored。`cargo fmt --check` 无差异。
- 本批新增 1 个 Rust 测试 `memory_origin_separates_stated_confirmed_and_inferred`：两条备注推断出的假设在投影与检索回执里都是 `rule_inferred` → `ConfirmHypothesis` 后翻成 `user_confirmed` → 直接写一条 `explicitPreference` 证据后是 `user_stated` 且证据计数为 1 → 把那条证据排除后来源不再是 `user_stated`。另外两处现有测试补了断言：跨项目检索钉住 `origin=user_confirmed`；接力改方案的等待提供方夹具钉住 `model_attempt.memoryReads` 恰好 1 条、且 `memoryId / sourceProjectId / scope / origin` 四项都对，并断言这一步的原文里**不含记忆正文**（只记 ID 与来源，不复制文字）。
- 为了让文档化的 `cargo clippy --all-targets -- -D warnings` 门槛真正变绿，修掉 `tests/local_core.rs` 里 3 处 `needless_borrows_for_generic_args`（早先判重那批提交进来的写法在新版 clippy 下报错）；没改任何行为，`--test local_core` 复跑 12 passed / 0 failed。
- 前端：59 个测试文件 / 423 项通过（本批新增 1 项）。新断言"区分写明偏好、确认规则与待验证推断，并标注适用范围"检查 `你写明的偏好 · 仅本项目`、`关键词推断 · 仅本项目`、`已确认（1）`、`待验证（1）`，并且只有 1 张卡片提供"确认此假设"（已确认的那条不该再让你确认一次）。`npm run lint`、`typecheck`、`contracts:check`（"Memory contracts: OK"）全部通过。
- 上下文成本算过账：`origin` 每条约 24–27 字节，`human_evidence` 每轮最多 3 条（≈81 字节内），`retrieve_preferences` 回执按 limit（默认 5、上限 10）最多约 270 字节/条；提示词与模型契约字节未动，所以第八批遗留的 48000 阈值那条账没有变化，也没有被本批推近。
- `git diff --check` 干净。

### 仍需推进

- 读取回执目前只是可查的落库证据，界面上看不到：前端 `TaskProgress.tsx` 只把步骤种类映射成标签，没有任何地方渲染 `memoryReads`；`retrieve_preferences` 面板也没显示 `origin`。用户眼下能看到的只有记忆面板里的分组与标签。
- 没有评测数据证明真实模型会按 `origin` 区别对待"你写明的偏好"和"关键词推断"——本批只保证信息准确过界，行为仍需一次真服务、真账户的人工验收。
- `stated_ids` 每次调用做一次 `json_extract` 全扫（按项目过滤），`materialize` 在单次投影里会调它两次；项目内证据条数量级不大，但这是新读的查询，没有 EXPLAIN QUERY PLAN 记录。
- 云端那张 `fr_author_hypotheses` 仍然既不记来源也不记适用范围。要补就得改服务器共享 schema，属于必须用户点头的改动，本批没有碰；因此跨端（桌面读云端作者记忆）那一路目前只有"已确认"这一种口径。
- 路线图这级还含"项目概览"这一层视觉分级（需要第三种 `kind`，即再来一次带 CHECK 约束的迁移）与 `evaluate_draft` 缓存观察不核对预览版本、`RevisionObservation` 不带 `preview_sha256` 两处遗留，都在第八批"仍需推进"里挂着，未动。
- 300 张规模断言（`unified_300_real_photos_three_boards_and_followup`）仍是 ignored、仍需显式批准，本批未运行；`max_context_tokens` 一个数字两套含义的拍板也还没得到答复。
- 未重新打包安装到本机 Mac，未做人工点击验收；未提交、未推送、未部署。工作区仍含其他会话的未提交改动（iOS、若干提示词、模拟器构建产物与截图），不计入本批。

## 第十批增量：工作包 1 第三项收尾——把"这一轮到底读了哪几条记忆"放到用户眼前

先核对现有代码再决定做什么：上一批已经把每轮真正进入上下文的记忆写进 `model_attempt` 的不可变回执，但它到不了用户眼前——`agent_steps` 投影给前端的只有 `kind / summary / created_at`（`store.rs:191`），聊天进度只把最后一步的种类映射成一句标签（`TaskProgress.tsx:9`），样式表里的 `.agent-steps` 没有任何组件在用。也就是说这批要做的不是再存一份数据，而是把已经存下的证据接进一个已经在的界面。选"参考来源"抽屉而不是新建面板：那里本来就是"这次 AI 用了什么"的落点，四语言文案、zod 契约与测试都现成，且这条投影是纯界面路径，不进模型上下文。因此本批不加迁移、不加 IPC 命令、不加依赖、不动提示词与模型契约字节。

### 实现

- `creative::memory_reads()` 只从 `kind='model_attempt'` 的步骤里取 `memoryReads`，用 `json_type(body_json,'$.memoryReads')='array'` 过滤——上一批之前跑的任务没有这段记录，就一条也不显示，不替历史补写"当时用过什么"。同一轮里同一条只算一次，`rounds` 记的是"带着这条记忆发起过几次请求"，不是"读了几遍正文"。
- 记忆正文按 id 回查 `preference_hypotheses`，并且要求这一行的 `project_id` 与回执里记的 `sourceProjectId` 一致才给出 `passage`；对不上（这条被挪去别的项目）或者已经删除，`passage` 就是 null，界面只报来路。理由很直白：不能拿现在另一行上的文字，冒充"当时那条记忆的内容"。
- 记忆条目排在知识条目之后共用同一个 40 条上限，出处优先占位；每条带 `memoryId / origin / scope / sourceProjectId / rounds`，`visualEvidence` 保持 false（记忆不是看图得出的结论）。
- 抽屉里 `kind==='memory'` 的标题走 `chatMemoryReference`（"本次用到的创作记忆"），正文沿用已有的引用块，脚注一行是"来路 · 适用范围 · 本任务用了 N 轮"。来路标签沿用上一批的三种取值与"陌生取值直接露出原文"的既有约定（`chatMemoryOrigin_*`），缺失时显示"来源未知"而不是替它猜一个；正文读不到时单独说明"这条记忆已被移除，或它的来源项目对不上，正文不在这里显示"。中英日韩各 9 条新文案，四份翻译表键数保持一致（各 279 条）。
- 前端契约同步：`domain/agent.ts` 的 kind 联合加 `'memory'` 与 5 个新字段，`TauriAgentRepository` 的 zod 同步声明（strip 模式下不声明会被静默丢掉）；`rounds` 给成字符串、`passage` 结构错误都会让整条投影被拒，而不是被解释成"用了 2 轮"或"这条读得到正文"。
- iOS 不解析 `creative_references`（全仓只有 Rust 与 TS 引用它），新条目对手机端无影响；这条抽屉目前仍是桌面独有。

### 验证结果

- Rust 默认特性全目标：404 passed / 0 failed / 20 ignored（40 个测试二进制）；仅 `--lib` 为 184 passed / 0 failed / 3 ignored。托管链路 `--test hosted_ai --features integration-harness`：21 passed / 0 failed。共享原生核心 `--lib --no-default-features --features native-agent`：189 passed / 0 failed / 4 ignored。`rustfmt --edition 2024 --check` 对本批两个文件无差异（复跑前后 `creative_tests.rs` 的 2 行差异都是别的会话留下的，本批没碰它）。`cargo clippy --all-targets -- -D warnings` 退出 0。
- 本批新增 1 个 Rust 测试 `memory_reads_surface_in_references_and_stop_showing_text_when_source_moves`：跨项目一条已确认偏好在任务投影里出现为 `kind='memory'`，`origin=user_confirmed`、`scope=cross_project`、`sourceProjectId` 等于来源项目，正文含"留白"；界面显示的轮数与数据库里"带这条记忆的 model_attempt 条数"逐一对齐（不是各自数一遍）；把这条记忆改到另一个项目之后，投影仍有这条（ID 与来路还在）但正文变成 null；同时断言已落库的 `memoryReads` 原文没被改写——改动记忆不会回头重写历史回执。
- 前端：59 个测试文件 / 424 项通过（本批新增 1 项）。新桌面断言"创作记忆的来路与适用范围跟着正文一起上屏，来源对不上就不编正文"检查两条记忆同时出现、`你确认过的规则 · 来自其他本地项目 p-source · 本任务用了 2 轮`、`关键词推断 · 仅本项目 · 本任务用了 1 轮`，以及读不到正文时的那句实话。上一批的契约断言扩成同时覆盖记忆条：`origin/scope/sourceProjectId/rounds/passage` 原样过界，`rounds:'2'`（字符串）会让整条投影被拒。`npm run lint`、`typecheck`、`contracts:check`（M1 command / Agent / Memory contracts 全 OK）通过，`git diff --check` 干净。
- 上下文成本这条账不变：`references()` 只进任务视图，不进 `store::protocol()` 的模型协议，本批一个模型字节都没加。

### 仍需推进

- 抽屉给的是"这个任务总共用过哪些记忆"，不是某一轮的快照：多轮任务里同一条只出现一次并累计轮数，"第 3 轮到底看了什么"目前要回数据库查 `memoryReads`，界面没有按轮展开。
- 上一批之前的历史任务没有读取回执，界面对它们保持空白——那是"没记过"，不是"没用过"；而这两批都还没打包安装，所以本机现在跑的任务都不会出现记忆条目，要等新构建装上才开始有数据。
- 跨项目那条会在本项目的抽屉里显示另一项目的原文。这是同一台机器上用户自己的数据（写入时已按账户与照片范围过滤），但本批没把"这条现在还是已确认状态吗"投影进来：`status` 不在 `memoryReads` 里，被停用或被移除的旧记忆仍会作为历史出现过。
- 真实模型是否因 `origin` 区别对待"你写明的偏好"与"关键词推断"仍没有评测数据；未做人工点击验收。
- 1-B 遗留的"项目概览"分级（需要第三种 `kind`，即再来一次带 CHECK 约束的迁移）、`evaluate_draft` 缓存观察不核对预览版本、`RevisionObservation` 不带 `preview_sha256`，以及云端 `fr_author_hypotheses` 不记来源与适用范围（需要共享 schema DDL）都还没动；300 张规模断言仍是 ignored，`max_context_tokens` 一个数字两套含义的拍板仍未答复。
- 未提交、未推送、未部署。工作区仍含其他会话的未提交改动（iOS、若干提示词、`creative_tests.rs` 的 2 行、模拟器构建产物与截图），不计入本批。

## 第十一批增量：工作包 1 第三项补漏——模型自己查回来的记忆也算"用过"

核对上一批留下的口径缺口：抽屉只数了"每轮被读进上下文"的那三条（`human_evidence` 注入，上限 3 条/轮），但模型还能主动调 `retrieve_preferences`（v2 别名 `retrieve_memory`，经 `contracts/agent-tool-contracts.json` 的 `v2Alias` 与 `ToolRegistry::canonical()` 解析）一次拿回最多 10 条。这些条目确实作为工具结果进过上下文、也确实影响过下一轮的判断，却因为不写在 `memoryReads` 里而在界面上完全不可见——用户会看到"这次一条记忆都没用"，而账本里明明有查询回执。本批把这个来路并入同一个聚合，仍不加迁移、不加 IPC 命令、不加依赖、不动提示词与模型契约字节。

### 实现

- `creative::memory_reads()` 新增第二条聚合：`tool_calls` 里 `tool_name IN('retrieve_preferences','retrieve_memory')`、`json_extract(result_json,'$.ok')=1`、`json_type(result_json,'$.result.hypotheses')='array'` 的不可变回执，按 `memory_id` 去重后并入同一张表；别名一并匹配，避免模型用哪个名字查就漏掉哪个来路。
- 两条来路各记各的数，不互相冒充：`rounds` 仍然只数"带着这条记忆发起过几次请求"，新增 `retrievals` 数"发起过几次带回这条记忆的查询"。同一次查询里同一条只算一次。只被查回来、从没被读进上下文的条目 `rounds` 记 0。
- 来路字段以先到的读取回执为准：`origin / scope / sourceProjectId` 只在读取回执没记的时候才用查询回执里的 `origin / scope / source_project_id` 补齐，两条都记过就保留上下文那份，不会因为后一次查询把历史改写成新的取值。正文回查沿用上一批的同一道闸门——`preference_hypotheses.project_id` 必须等于回执记的来源项目，对不上或已删除就只有 ID 与来路，`passage` 为 null。
- 界面上脚注从"来路 · 适用范围 · 本任务用了 N 轮"扩成"来路 · 适用范围 · 用了 N 轮 · 本任务主动查了 M 次"，两个数字各自只在有值时出现：轮数为 0 就不写"用了 0 轮"，没查过就不写次数。中英日韩各加 1 条文案（四份翻译表各 280 条，键集合完全一致）。
- 契约同步：`domain/agent.ts` 与 zod 各加 `retrievals?: number`（strip 模式下不声明会被静默丢掉），`retrievals` 给成字符串同样让整条投影被拒；Rust 没带这个字段时前端拿到的是 undefined，界面就少写那一段，而不是补一个"查过 0 次"。

### 验证结果

- Rust 默认特性全目标：404 passed / 0 failed / 20 ignored（40 个测试二进制）；仅 `--lib` 184 / 0 / 3；托管链路 `--test hosted_ai --features integration-harness` 21 / 0；共享原生核心 `--lib --no-default-features --features native-agent` 189 / 0 / 4——四个数与上一批逐项相同，本批没新增也没弄坏任何一条。`rustfmt --edition 2024 --check` 对本批两个文件无差异（`creative_tests.rs` 那 2 行仍是别的会话留下的，本批没碰）。`cargo clippy --all-targets -- -D warnings` 退出 0。
- 上一批那条 Rust 测试改跑 `RetrieveScript`（两轮 `retrieve_memory` + 一次 NoAction），现在同时断言两条来路：`rounds` 仍逐一对齐"带这条记忆的 `model_attempt` 条数"，`retrievals` 对齐"用 `json_each(result_json,'$.result.hypotheses')` 数出的、真带回这条 id 的查询回执条数"，两个数都是直接从账本查出来的，界面显示值不可能和账本各说一遍；来源改动后正文变 null、历史回执不被改写这两条断言保持原样通过。
- 前端：59 个测试文件 / 424 项通过。抽屉断言扩到三条记忆，其中一条是"只被主动查过、从没进上下文"的，要求出现 `你写明的偏好 · 仅本项目 · 本任务主动查了 2 次` 且**不得**出现"本任务用了"字样；契约断言新增 `retrievals` 原样过界、`retrievals:'1'`（字符串）整条投影被拒，以及"没查询过就没有这个字段"。`npm run lint`、`typecheck`、`contracts:check`（M1 command / Agent / Memory contracts 全 OK）通过，`git diff --check` 干净。
- 上下文成本这条账仍不变：`references()` 只进任务视图，不进 `store::protocol()` 的模型协议，本批一个模型字节都没加。

### 仍需推进

- 上一批"抽屉只反映上下文注入、不含主动查询"这条缺口本批关闭；"按轮展开（第 3 轮到底看了什么）"仍未做，跨来路的按轮对齐要把查询回执也写进 `agent_steps` 才谈得上。
- 仍是"没记过"不等于"没用过"：第十批之前跑完的历史任务既没有读取回执，其工具回执也在，所以它们会只显示主动查询那一半——本批不替历史补写，界面按各自实际有的来路显示。
- 本批与上两批都还没打包安装，本机现在跑的任务不会有任何记忆条目，要等新构建装上才开始产生可核对的数据。
- 真实模型是否因 `origin` 区别对待"你写明的偏好"与"关键词推断"仍无评测数据；未做人工点击验收。
- 1-B 遗留的"项目概览"分级（需第三种 `kind`，即再来一次带 CHECK 约束的迁移）、`evaluate_draft` 缓存观察不核对预览版本、`RevisionObservation` 不带 `preview_sha256`，以及云端 `fr_author_hypotheses` 不记来源与适用范围（需共享 schema DDL 批准）都还没动；300 张规模断言仍 ignored，`max_context_tokens` 一个数字两套含义的拍板仍未答复。
- 未提交、未推送、未部署。工作区仍含其他会话的未提交改动，不计入本批。

## 第十二批增量：工作包 1 第三项补漏——"当时用过"不等于"现在还算数"

清第十批自己留下的那条口径缺口：抽屉只报"这个任务用过这条记忆"，但记忆是可以事后撤的——用户能在审美记忆面板里纠正或移除一条，也能把它停用不再参与读取。撤过之后老任务的抽屉仍然把那条当成仍然作数的偏好在展示，而且正文照旧显示，用户看不出自己已经不要它了。这不是显示不全，是显示得误导。本批把"这条现在是什么状态"接进同一条投影，仍不加迁移、不加 IPC 命令、不加依赖、不动提示词与模型契约字节。

### 实现

- `memory_reads()` 的回查一次多取 `status` 与 `disabled_for_retrieval`，投影出 `memoryStatus` 与 `memoryDisabled`。"生效中"的取值集合不是猜的：对照检索侧的实际过滤（`memory.rs` 的 `retrievable_for_goal` 用 `!disabled` 且只放行 `unconfirmed / confirmed`，`corrected / retired` 与停用都不再进入检索），所以界面只对不在这个集合里的状态或已停用的那条多说一句，正常情况不打扰。
- 信任闸门沿用上一批那一道，而且明确扩到状态：`passage`、`memoryStatus`、`memoryDisabled` 三个都要求 `preference_hypotheses.project_id` 等于回执记的 `sourceProjectId`。来源对不上（这条被挪去别的项目）时三个一起为 null——不能拿另一行上的文字冒充它当时的正文，同样也不能拿另一行上的状态冒充它现在作不作数。
- 界面把记忆条的三段说明收进一个组件：来路 · 适用范围 · 轮数 · 查询次数（原有脚注），读不到正文时那句实话（原有），以及新增一句警示——复用样式表里已存在的 `.chat-footnote.warning`（橙色），文案是"这条记忆当时用过，现在已不生效："后面接真实原因，多个原因用 ` · ` 并列（"已移除 · 已停用，不再参与读取"）。陌生状态按既有约定直接露出原文，不替它翻译成"已移除"；`memoryDisabled` 为 null（读不到当前状态）时不写"没停用"，也绝不弹这句警示。
- 契约同步：`domain/agent.ts` 加 `memoryStatus?: string | null` 与 `memoryDisabled?: boolean | null`，zod 用 `nullable().optional()`（strip 模式下不声明会被静默丢掉）。中英日韩各加 4 条文案，四份翻译表键数一致（各 284 条）。

### 验证结果

- Rust 默认特性全目标：404 passed / 0 failed / 20 ignored（40 个测试二进制）；仅 `--lib` 184 / 0 / 3；`--test hosted_ai --features integration-harness` 21 / 0；`--lib --no-default-features --features native-agent` 189 / 0 / 4——四个数与上一批逐项相同。`rustfmt --edition 2024 --check` 本批文件无差异（`creative_tests.rs` 那 2 行仍是别的会话留下的），`cargo clippy --all-targets -- -D warnings` 退出 0。
- Rust 那条记忆投影测试按真实状态链走完四步：任务跑完时 `memoryStatus=confirmed`、`memoryDisabled=false` 且没有警示数据；把这条标成停用后 `memoryDisabled=true` 而正文仍在（停用不抹历史）；改成 `retired` 后状态跟着变成 `retired`；最后把这条挪去另一个项目，正文、状态、停用标记三者同时为 null，并且已落库的读取回执原文依旧记着来源项目没被改写。
- 前端：59 个测试文件 / 424 项通过。抽屉断言扩到 5 条记忆，其中两条会被提示、三条不会——`getAllByText(/这条记忆当时用过，现在已不生效/)` 恰好 2 条，分别要求出现 `已移除 · 已停用，不再参与读取` 和陌生的 `archived` 原文；对仍在生效的那条改成整段精确匹配 `你写明的偏好 · 仅本项目 · 本任务主动查了 2 次`，这样"多冒出一句轮数"或"多冒出一句警示"都会让这条断言失败。契约测试补 `memoryStatus/memoryDisabled` 原样过界（含 null 透传），并把 `memoryDisabled:'yes'`（字符串）列为整条投影被拒——"停用"被写成字符串不能解释成"已经停用"。`npm run lint`、`typecheck`、`contracts:check` 全 OK，`git diff --check` 干净。
- 上下文成本仍不变：这条投影只在任务视图里，不进模型协议，一个模型字节都没加。

### 仍需推进

- 检索侧还有一条本批没投影进来的过滤：某条记忆的正文与一条已被纠正/移除的同名条目撞车时，它也会被排除出检索（按语句去重）。界面对这种"内容撞车"仍不提示，只认这条自己的状态与停用标记。
- 警示说的是"现在"，不是"当时"：本批不判断模型当时是否真的按这条走了，也不把状态回写进历史回执（回执仍不可变）。
- 仍未做按轮展开；第十一批之前跑的历史任务仍只显示它们真实有过的那一半来路。
- 三批都还没打包安装，本机现在跑的任务不会出现任何记忆条目，要等新构建装上才开始有可核对的数据。
- 真实模型是否因 `origin` 区别对待"你写明的偏好"与"关键词推断"仍无评测数据；未做人工点击验收。
- 1-B 遗留的"项目概览"分级（需第三种 `kind`，即再来一次带 CHECK 约束的迁移）、`evaluate_draft` 缓存观察不核对预览版本、`RevisionObservation` 不带 `preview_sha256`，以及云端 `fr_author_hypotheses` 不记来源与适用范围（需共享 schema DDL 批准）都还没动；300 张规模断言仍 ignored，`max_context_tokens` 一个数字两套含义的拍板仍未答复。
- 未提交、未推送、未部署。工作区仍含其他会话的未提交改动，不计入本批。

## 第十三批增量：工作包 2 首项——色彩管理预览真的变换，并说清它做了什么

工作包 2「专业色彩与高保真影像」的第一块地基。核查桌面色彩链路时发现一个已经存在但从没生效的东西：`get_color_managed_preview` 这条命令和它的实现早已写好，但前端那个开关组件从来没有接进任何界面，用户无从调用；更要紧的是它自己也不能工作——它把**空的**源色彩配置传进变换函数（源配置文件读出来了却没用上，只把描述文字放进返回值），并且 `display_profile_bytes()` 的失败被 `unwrap_or_else(|_| Vec::new())` 吞成空字节，读不到原文件 ICC 也被 `.ok().flatten()` 悄悄当成 sRGB。结果是这条命令可以在什么都没变换的情况下返回 `transform_applied: true` 和一张图：界面看起来"已按显示器渲染"，像素其实一点没动。本批把这两件事一起修掉：变换按真实源配置做，做不成的每一种原因都自解释；开关接进沉浸审片灯箱。约束沿用本阶段口径——不加迁移、不改 schema、原图只读、默认路径仍是字节级直通、不提交不推送。

### 实现

- `color_engine.rs` 新增两层，全部复用已有的私有件（`extract_embedded_icc` 的 JPEG APP2 / PNG iCCP / TIFF 标签 34675 有界读取、`valid_icc_structure`、`parse_icc_description`、`native_transform::transform_rgb8`），不新开公共 API。第一层 `source_space_for_preview(original)` 决定"源空间到底是什么"，而且是**三态而非两态**：读到有效嵌入配置 → `Profile`；容器可解析（JPEG/PNG/TIFF 魔数）但确实没嵌 → `AssumedSrgb`，这是显式假定而不是猜；容器根本读不出来（HEIF、RAW、BigTIFF、非图片）→ `Rejected{container_unsupported}`，不做任何猜测。这一层刻意向**原文件**要配置而不是向缓存预览要：`image_pipeline` 的缓存是用 device RGB 位图重采样后重新编码的 JPEG，像素数值仍是源空间而 ICC 被丢弃，所以配置的来路只能是原文件。第二层 `render_managed_preview(original, preview, display_icc)` 把缓存预览渲染到显示器色彩空间，`display_icc` 由调用方以 `Result` 注入，于是取不到显示器配置、结构校验不过、解码失败、变换失败、重编码失败每一种都在函数内归到确定状态（`transformed` / `assumed_srgb` / `display_unavailable` / `profile_invalid` / `transform_failed` / `container_unsupported`）并带一句中文原因，不再有任何一处 `unwrap_or_default`。
- `commands.rs` 里那条命令只剩取路径与编码：原本内联的约 80 行试错逻辑搬进上面两层，返回值从 `data_url: String + transform_applied: bool` 改成 `status / detail / source_profile / display_profile / data_url: Option<String>`。**没变换就一个像素都不返回**（`data_url: None`），前端因此没有任何可能显示一张没变换的图却声称做了色彩管理；顺带省下失败情况下约 1 MB 无用的 base64 跨进程拷贝。预览文件读不到时明确报 `RESOURCE_MISSING`。变换仍只在 `#[cfg(target_os = "macos")]` 内（ColorSync），非 macOS 走不到。
- 前端把已有但够不着的 `ColorManagedToggle` 接进灯箱工具条，并把状态说满：`PhotoViewport` 新增可选 `managed`，开启时用变换后的图顶替缓存预览、`data-source` 标成 `color-managed`、角标写"色彩管理预览 · 由缓存预览重渲染，非原图"再加一句状态；`ColorManaged` 类型带 status，未知状态码按 `zh[key] === key` 那条既有约定原样露出，不翻译成猜出来的好听话。
- 两条诚实边界显式画出来：100% 检视时原图直通仍然优先，画面不改，只在下方注明"此刻色彩管理没生效"——变换版本是缓存预览重渲染的，不能冒充全分辨率像素；换片时变换结果按 `photoId` 归属判定自动失效，因为每张图的显示器渲染来自它自己的 ICC，沿用等于张冠李戴。这条用"状态里记 id + 渲染期派生"实现，而不是在 effect 里 setState（`react-hooks/set-state-in-effect` 已在 lint 报错过一次，派生写法也更简单）。后端说没变换时开关保持未按、画面不动，原因直接上屏。
- 中英日韩各加 14 条文案（开关名、状态与失败原因），四份翻译表键数仍一致（各 284 顶层、818 叶子）。

### 验证结果

- Rust：`cargo test --lib` 192 passed / 0 failed / 3 ignored（上一批 184，本批 +8）；默认特性全目标 412 / 0 / 20（上一批 404）；`--test hosted_ai --features integration-harness` 21 / 0；`--lib --no-default-features --features native-agent` 197 / 0 / 4；`color_engine` 单独 27 / 0（原 19 + 本批 8）。`cargo clippy --all-targets -- -D warnings` 退出 0，`rustfmt --edition 2024 --check` 本批两文件无差异，`cargo check` 在 `native-agent` 与 `integration-harness` 下均通过。
- 新增的 Rust 测试把"它现在真的变换了"钉成可核对的事实：带 Display P3 描述的 JPEG 从原文件读出并报告描述；无嵌入配置的 PNG 走 `assumed_srgb`；无法解析的容器走 `container_unsupported` 且不产生任何像素；结构损坏的配置走 `profile_invalid`；显示器配置为 `Err` 或非法字节都走 `display_unavailable`。macOS 专属用例进一步用纯色 JPEG 走完整链路，断言 `transformed` 且真的产出 JPEG 字节，并**用字节级比较证明原文件一个字节都没被改动**（原图只读是这条链路的红线）。
- 前端：59 个测试文件 / 431 项通过（上一批 424，本批 +7）。灯箱用例以 `vi.hoisted` 替掉 Tauri IPC，断言开关按下后 invoke 的参数精确到 `{ projectId: 'p1', photoId: 'a' }`、返回的变换图确实成为检视画面、`data-source` 与角标随之改变，再点一次退回缓存预览；后端返回"没变换"时要求画面仍是缓存预览、开关 `aria-pressed=false`、原因文字上屏；非桌面环境不发 IPC 且说明只桌面可用；换片后上一张的变换结果不被沿用。视口用例覆盖顶替关系、未知状态码原样上屏、以及 100% 检视下原图优先并注明未生效。灯箱里的胶片条每张都渲染缩略图，`getByRole('img')` 会同时命中，故所有画面断言改为从 `photo-viewport` 作用域内取——否则"变换版真的显示了"这条断言其实测不到检视画面。
- `npm run lint`、`typecheck`、`contracts:check` 全 OK，`git diff --check` 干净。本批不动提示词、不动模型协议，上下文成本为零。

### 仍需推进

- 开关目前只在沉浸审片灯箱里；网格检视器与摄影书页面预览尚未接同一套，默认路径仍是字节级直通（这是刻意保留，不是遗漏）。
- 变换的源是缓存预览而非全分辨率，所以它是审片时的判读辅助，不能当作输出交付物；真要按显示器渲染全分辨率原图需要另一条链路（瓦片原图 + 显示管理），本批没做。
- 非 macOS（网页版/Windows）拿不到本机显示器配置，界面明说仅桌面可用；这条边界只有代码路径保证，没有跨平台实测。
- 未做人工点击验收：本批没在本机重打包安装，桌面 App 里现在还看不到这个开关，要等新构建装上才开始可用。
- 未提交、未推送、未部署。工作区仍含其他会话的未提交改动（`agent/*` 等），不计入本批。

## 第十四批增量：工作包 2 第二项——100% 检视也按显示器渲染，并且纠正上一批"变换了但没标注"

两件事。第一件是本批计划内的：色彩管理开关此前只管构图用的缓存预览，放大到 100% 时仍是原图字节直通，界面也只能老实说"此刻色彩管理没生效"——判对焦和噪点时恰恰最需要按显示器的颜色。第二件是核查时发现上一批留的真实缺陷：`transform_to_jpeg` 把像素搬进显示器色彩空间后，写出的 JPEG **不带任何 ICC 配置**。渲染端对无配置的文件一律按 sRGB 处理，于是同一趟转换被做第二遍，屏上结果仍然是错的；上一批说"真的变换了"，像素确实变换了，但没有标注，等于只对了一半。本批把标注补上（预览与全分辨率两条链路同时受益），并把全分辨率那条链路建起来。约束沿用——不加迁移、不改 schema、原图只读、默认路径仍是字节级直通、不提交不推送。

### 实现

- 判定层与渲染层分开：上一批的 `render_managed_preview` 内部先走的 `source_space_for_preview` + 显示器配置读取，被抽成 `plan_managed_render(original, display_icc)`，返回一份"两侧配置字节 + 状态码 + 两侧描述"的 `RenderPlan`，或带回一条已写好中文原因的 `ManagedRender`。缓存预览重渲染与新加的全分辨率渲染**共用同一次判定和同一套状态码**，否则两条链路会对"为什么没变换"各说各话。变换成功后写出的字节一律过 `tag_jpeg_with_icc`：在 SOI 之后插一段 `APP2 / ICC_PROFILE 1/1`，写的是**目标显示器**那份配置（不是源配置）。这段是纯字节手术，两种情况明确失败并拒绝给像素——不是 JPEG 起始（`NOT_JPEG`）、配置大到一个段装不下（`ICC_TOO_LARGE`）；漏写 SOI 会让文件谁都解不开，这条正是新增测试第一次跑挂时暴露的，已在函数里钉住。
- `photo_inspection.rs` 新增 `display_managed_inspection(store, project_id, photo_id)`：**第一步就调 `original_path()`**，于是否在册、原图失联、内容指纹已变、64 MB/512 MB 上限、3200 万像素上限、格式白名单这些既有检视门禁全部自动继承，不必也不可能被这条新入口绕过；来源若是派生或嵌入预览（RAW 走的就是这条路），明确回 `source_not_original` 并说"没有可用的全分辨率原图"，不拿派生图冒充原图渲染。渲染产物写成缓存文件，文件名 `managed-<原图内容 sha256 前 24>-<显示器配置 sha256 前 24>.jpg`：换显示器或原图改动都会自然落到新文件，不需要猜旧缓存还能不能用；写入仍是临时文件加改名，读者不会看到半张图；`unusable()`（不存在或零字节）才重渲染，命中缓存则不重渲染、不换文件。返回值沿用 2-A 的契约形状（`status / detail / source_profile / display_profile / path`），**`path` 为 `None` 就是没变换**，界面继续用缓存预览的重渲染版并显示原因，不会拿一张没变换的原图冒充显示器色彩。
- `commands.rs` 加 `get_display_managed_inspection`，注册进桌面命令表；成功后只把该文件路径加进 asset 协议白名单。回传路径而不是 base64：全分辨率原图的 JPEG 可达约 10 MB，跨进程拷贝没必要，缓存预览那条小图才走 data URL。
- 前端 `Photo` 新增可选 `loadManagedOriginal`，`TauriRepository` 按 `projectId:photoId` 记忆化加载器（函数身份稳定，否则每次刷新项目都会让视口的 effect 重新拉一遍原图）；`kind` 为 `display_managed`。`PhotoViewport` 的选择规则是：开关开着**且**这条通道存在才用它，网页版没有这条通道就仍是原图直通。画面归属跟着改：角标写"原图 · 显示器色彩 · 全分辨率 · 已按显示器 ICC 渲染"，页脚色彩说明改为"已按本机显示器配置渲染并标注"，等待期说"正在按显示器色彩渲染全分辨率原图…"，而"100% 检视仍走原图直通"那句现在只在 `kind === 'original'` 时才出现——已经渲染过还说这句就是骗人。解码失败时提示语用来源标签本身（"原图 · 显示器色彩无法解码"），不笼统报成"原图"。状态码翻译从组件里提到 `app/copy.ts` 的 `colorStatusLabel`，IPC 层与界面共用一份，陌生的码仍原样露出。
- 中英日韩各加 2 条文案（渲染等待语、`source_not_original` 状态说明），四份翻译表键数仍一致（各 284 顶层、820 叶子）。
- 顺手补上这些派生文件的容量归属：`image_pipeline::sweep_orphans` 此前只认 `{hash}-320.jpg` / `{hash}-1400.jpg` 两种名字，`managed-*` 既不计入 2 GiB 配额也永不被回收——看得越多缓存越大且没有上限。现在渲染产物按"可回收"处理：计入总占用、超限即按最旧优先删，下次 100% 检视会按同样的文件名规则重渲染，代价是多花一次渲染而不是涨到没边。写一半崩掉留下的 `managed-*.tmp` 也在回收范围内，但只收存活超过一小时的，避免和仍在进行的渲染争同一个临时文件；同前缀但不是 `.jpg`/`.tmp` 的名字仍视作"不是本应用的文件"不动。

### 验证结果

- Rust：`cargo test --lib` 193 / 0 / 3（上一批 192，本批 +1）；`--all-targets --no-fail-fast` 416 / 0 / 20（上一批 412，本批 +4）；`--test photo_inspection` 10 / 0 / 2（上一批 8）；`--test cache_quota` 3 / 0（新增渲染产物计入配额一例）；`--test hosted_ai --features integration-harness` 21 / 0；`--lib --no-default-features --features native-agent` 198 / 0 / 4（上一批 197）。`cargo clippy --all-targets --all-features -- -D warnings` 退出 0，`rustfmt --edition 2024` 对本批五个改动文件无差异。默认路径的字节级直通与 16-bit 保真、ICC 元数据保真等原图只读回归全部保持通过。
- 全分辨率这条新链路的证明是**尺寸与字节**两头的：测试用 2400×1600、内嵌 Display P3 的 JPEG 导入（缓存预览被压到 1400），断言渲染出的文件确实是 2400×1600——只有取自原图才可能，取自缓存预览就过不了；同时逐字节比较原文件，证明渲染没碰原图；第二次调用要求路径相同且 mtime 不变，证明命中缓存不重渲染。分支由"这台机器能不能取到显示器配置"决定而不是由被测代码的回答决定：无头环境要求它明确报 `display_unavailable` 且不给路径，有显示器的机器上任何"没变换"的回答都算失败——本机走的正是强断言那一支。门禁继承用改过原图（报 `FILE_CHANGED` 而不是渲染一张冒充原图内容的显示器版本）和 `.dng`（报 `source_not_original` 且不给路径）两例钉住。
- "标注"这件事单独钉了三处：单元层断言写出的 JPEG 用同一套分段解析能取回**等于目标显示器配置**的字节、且加标注不改变像素值；`render_display_managed` 的 sRGB→P3 用例断言文件标的是 Display P3 而不是源 sRGB（标错等于让渲染端再管一次）；集成层断言全分辨率渲染文件的内嵌配置逐字节等于本机 `display_profile_bytes()`。
- 前端：59 个测试文件 / 436 项通过（上一批 431，本批 +5）。新增用例覆盖开关开着时 100% 用全分辨率渲染且**不**调用直通原图、`data-source` 与角标随之改变且不再出现"仍走原图直通"、等待语按渲染版而非校验原图来说、后端拒绝时原因上屏并回落到色彩管理预览（同样不退回直通原图）、开关中途关掉与重开时加载器切换正确、按 1:1 检视并在渲染文件解码坏掉时回指渲染版本身。`npm run lint`、`typecheck`、`contracts:check` 全 OK，`git diff --check` 干净。本批不动提示词与模型协议，上下文成本为零。

### 仍需推进

- 双重管理这条判断是按 macOS/WebKit 的既有色彩管理规则写的（无配置文件即按 sRGB 处理），代码层已保证标注的就是它变换到的那份配置；**屏上观感还没在你这台机器上人工比对过**，因为本批没有重打包安装，桌面 App 里现在仍看不到这个开关。
- 渲染产物是 q92 JPEG 的 8-bit 显示器空间图，用途是判读不是交付：16-bit、原图无损与导出仍走原来的直通路，色彩管理版本永不参与导出（导出与缓存清理按数据库记录的路径走，不会把这些派生文件带进导出包）。容量回收本批已接上配额，被删掉的那张下次放大时会自动重渲染，代价只是多等一次；但清理只在启动后跑一次，也没真把缓存压到 2 GiB 去验证，这条只有代码路径与单元/集成测试保证。
- 100% 渲染目前只在灯箱里；网格检视器、摄影书页面预览与手机侧还没接同一套。
- 非 macOS（网页版/Windows）拿不到本机显示器配置，界面仍明说仅桌面可用；只有代码路径保证，无跨平台实测。
- 未提交、未推送、未部署。工作区仍含其他会话的未提交改动（`agent/*`、偏好面板等），不计入本批。

## 第十五批增量：工作包 3 首项——书稿在送印前能自查，且把"为什么不能印"说成有数字的话

工作包 3 核对下来，版式、跨页、纸张与出血、布局版本这些**已经建好并测过**，重复建设没有意义；真正缺的是收尾那一环——一本排好的书在送印前没有任何地方回答"现在能不能印"。原图在不在磁盘、像素够不够印刷分辨率、图有没有压到书脊上、有没有留出裁切余量、色彩配置有没有标注，这些问题原本只在放大检视或导出时才隐约暴露，而且没有一处汇总。本批把这层判断做成一个可复查的报告。约束沿用——不加迁移、不改 schema、检查全程只读、不动导出路径、不提交不推送。

### 实现

- 判定与取数分开，判定是**纯函数**：`publication_preflight::check(layout, min_ppi, offline)` 只吃已经载入的版面加一份"文件还在不在"的事实表，返回带严重级的报告，便于用假版面精确钉每条规则；取数那一层 `Store::publication_preflight` 才碰数据库和磁盘，用 `std::fs::symlink_metadata` 在 `spawn_blocking` 里逐条查存在性——**故意不算内容指纹**，那是检视链路的职责，印前检查只需要回答"这个文件还在不在"，把两条链路的成本混起来会让每次检查都变成一遍全库重算。18 条码分三档（不能送印 / 建议复核 / 提示），阈值合法区间 72–1200 PPI，越界直接回 `VALIDATION_ERROR` 并给出中文原因，报告封顶 200 条、超出置 `truncated` 而不是静悄悄截掉。
- 分辨率这条最容易算错的规则按**实际交付的像素**算，而不是按原图大小自夸：这一格若已绑派生 rendition 就用 rendition 的尺寸（印刷交付的是它），没绑才回落到原图；`crop_zoom` 会同比缩小两个方向的可用像素；毫米换算英寸除 25.4。方向性规则也区分开了——`fit`（整图放进去）取宽、高两向 PPI 的**较大者**，因为短板决定图是否完整；`fill`/拉伸（铺满后裁掉）取**较小者**，因为铺满用的那一边才是实际印出来的密度。屏上显示的数取整，比较用未取整的值，避免"显示 300 其实 299.6"这种自欺。色彩那条与本包已经落库的元数据接上了：没带配置的源图与派生图、色深过浅各自成条，判读影像和判读印刷规格用的是同一份事实。
- 检查**不落库**：`print_contracts` 那张表仍然空转，没有往里写任何"上次检查结论"。理由是这份结论会随版面和磁盘状态立刻过期，存下来反而多一本可能对不上账的台账；界面上的做法是改一次版面就把旧结论清掉，逼它重新检查。
- 界面只信自己数得出来的东西：IPC 回来的报告先过 zod 形状校验，再逐条重数一遍三档计数——**声称的统计与逐条不一致、说被截断却不足上限条数**，都当作不可信直接拒绝并提示重新检查，不拿一份说谎的报告去告诉用户"可以送印"。摄影书工作台新增一个印前检查区，可切 72/150/300 PPI 三档要求（切档立即清空旧结论并作废在途请求，慢回来的那份不能盖住新阈值下的判断）；每条 findings 标明归属整本/封面/第几页，并把实测与要求一起写出来。查看历史版本时这个区不出现——它说的是当前版面，挂在历史版本上是误导。
- 文案与码表分层：`preflightDetails.ts` 负责"码→人话 + 单位 + 该不该给数字"，只有真有量纲的码才拼"实测 X，要求 Y"（PPI/mm/×/bit 四类），没有量纲的码只说问题、不编数字，陌生的码原样露出——这条沿用色彩状态码那批定下的规矩，猜出来的解释比难看的原码更误导。中英日韩各加 34 键，四份翻译表键数仍一致（各 566 顶层）。
- 顺手把既有导出的语义核对了一遍并按现状讲清：序列第一张照片会同时用作封面与第 1 页，这是生成逻辑的既有行为，不是检查器的判断，所以一处原图失联会如实报到两处——界面显示"两个位置都要处理"而不是合并成一条来安慰用户。

### 验证结果

- Rust：`cargo test --lib` 201 / 0 / 3（上一批 193，本批 +8）；`--all-targets --no-fail-fast` 427 / 0 / 20（上一批 416，本批 +11）；`--test publication_preflight` 3 / 0（全部为本批新增）；`--lib --no-default-features --features native-agent` 206 / 0 / 4（上一批 198）。`cargo clippy --all-targets --all-features -- -D warnings` 退出 0，`rustfmt --edition 2024 --check` 对本批改动文件无差异。版面、版本、导出与影像保真等既有回归全部保持通过，检查全程只读——没有任何一条测试路径会写入或改写原图与版面。
- 判定的 18 条码在单元层**逐条带数字断言**：出血缺口多少毫米、图压进书脊带几毫米、文字越出安全线几毫米、拉伸与裁切放大倍数、色深位数，以及三档计数与逐条严格一致、上限截断确实置 `truncated`。
- 真链路用**真数据库 + 真照片文件**证明，不是拿假数据自证：导入两张 3000×4000 的 JPEG 生成整本，断言 0 项阻断；随后删掉其中一张原图，断言只报"原图失联"、不再叠加一条算不出来的低分辨率（同一处问题不重复吓人），并要求它落在封面与第 1 页两处。第二例用一张 600×800 的小图，断言恰好两条低分辨率、要求恒为 300、实测落在 1–300 之间；再把阈值降到 72 重查，断言阻断归零——证明结论是相对印刷要求出来的，不是程序自己拍的。第三例覆盖"选了 3 张只进书 2 张"要提示有几张照片没进书稿（建议复核，不阻断），以及 0/71/1201 三个非法阈值被拒并回中文原因。
- 前端：62 个测试文件 / 448 项通过（上一批 59 / 436，本批 +12）。新增用例覆盖 findings 正确上屏且**全程不触发任何写命令**、切阈值先清空旧结论再以新阈值重查、检查失败时把原因显示出来且绝不出现"可以送印"字样、打开历史版本时该区不渲染；IPC 层单测专门喂篡改过的报告（谎报统计、声称截断却只有一条、阈值越界、非法严重级、负的实测值、大写码、页码越界）要求一律拒绝。四语言完整性有一例独立守护：每条码在四种语言都有说明、且英文表里不混入中日韩字符。`npm run lint`、`typecheck`、`contracts:check` 全 OK，`git diff --check` 干净。本批不动提示词与模型协议，上下文成本为零。

### 仍需推进

- **文字会不会撑破文本框没查**：现有只判断文本框越没越过安全线，要判断"这些字在这个字号下放不放得下"需要真实字体度量，属于另一件事，没在本批硬凑一个近似值。
- **检查没有拦导出**：导出仍是既有的 HTML→浏览器打印路径（架构决议如此），阻断项只做提醒、不会不让点导出。要不要"有阻断项就不让出片"是个产品决定，先摆在这里等你拍板。
- **阈值是行业默认，不是印厂给的规格**：300 PPI、3 mm 出血、15 mm 安全区这些是可讲道理的默认值；印厂实际给的 ICC、书脊宽度、出血规格仍未接入，`print_contracts` 表正是留给"按某家印厂档案检查"这一步的，现在仍空转。
- **图注和页面文字还不能编辑**：数据、校验与只读渲染都在，缺写入命令与界面，这是工作包 3 剩下的第二项，也是下一个最该补的开口。
- 跨页/拼版页的书脊判定按当前几何规则写，没拿真印刷样人工比对过；手机侧没有这个面板（检查依赖桌面端文件与版面数据，网页/手机仍看不到结论）。
- 未在你这台 Mac 上重打包安装，桌面 App 里现在还看不到这个区；未提交、未推送、未部署。工作区仍含其他会话的未提交改动（`agent/*`、iOS、偏好面板等），不计入本批。

## 第十六批增量：工作包 3 第二项——图注和页面文字真的能写进去

上一批留的开口是"数据和只读渲染都在，唯独写不进去"：`publication_texts` 表、版面读写、版本快照、纸面渲染、印前检查里的文字越界判定全都已经建好，但生成器造出来的书**一个文字框都没有**，用户也没有任何入口加字。所以这一批不新造机制，只把缺的那条写入通道和界面接上——照旧不加迁移、不改 schema、不提交不推送。

### 实现

- 命令层加 `AddText`/`UpdateText`/`DeleteText` 三条写命令，完全沿用既有那套：一条命令一个幂等键、`baseRevision` 乐观锁、回执原样可重放、每次改动追加不可变版本快照，文字类与照片/版面类一样允许多条合并成一个批次（版本类动作仍需单独命令，白名单同步扩到这三条）。载荷字段是服务端自己拼的：内容写成 `{"text":…}`、样式写成 `{"sizePt":…,"align":…}`，**客户端传不进任意 JSON**，也就没法把样式或标记塞进版面。限制与 zod 镜像一致：1–300 个字符（Rust 按 `chars()` 计，中文一个字算一次）、字号 6–72 磅、对齐只有左中右、编号必须是 UUID。
- "回传原值"不算改动：`same_text` 比的是逻辑内容而不是 JSON 字符串，同一段字、同一个字号对齐重新提交一遍会得到 `changed=false`、修订号不动、**也不多记一个版本**——否则用户每点一次保存版本历史就多一条假的。
- 越界不留半个框：几何仍然只在写完后重读版面时由既有的 `validate_layout`→`rectangle` 判一次，图注压进出血/书脊带会让整笔事务回滚并回中文原因（测试直接断言原因里含"出血"，并断言版面与版本条数一根毛都没动）。页面没了、文字框没了分别给可行动的 NOT_FOUND 文案，删第二条同一条不会静悄悄成功。
- 前端把"能不能编辑"这件事交给通道本身：领域接口里三个方法都是可选的，只有桌面 Tauri 仓储实现了它们，网页/手机拿不到入口——面板按 `repository.addText` 存在与否出现，而不是假装能写再报错。三条命令都过同一条带重放的持久队列，一次一条，确认前不算改完；越界的载荷（超长、字号越界、非 UUID 编号、缺页编号）在出门前就被契约层拦下，后端连请求都收不到。
- 编辑规则收成一个纯函数模块（`captionDraft.ts`）：从存好的记录读回可编辑的值——内容 JSON 坏掉读回空串而不是替它编一段话，样式缺失或越界退回 10 磅左对齐；新建图注默认落在**下边距之上的整幅横条**（高度取 `min(18, (页高−2×边距)/2)`），用户不必先猜四个坐标；小页面时高度按页高收缩，不会把框顶到出血里。
- 工作台右侧新增图注区：没选文字时告诉你"点页面里的文字框可修改内容"；新增与修改走同一个表单（有原文字走 `UpdateText`，没有走 `AddText`，绝不会因为改了个位置就多出一条）；删除必须先确认，确认文案把"能从版本历史找回"这条退路写在脸上；后端拒绝时原因原样显示、草稿留在屏幕上、**绝不显示"图注已保存"**。
- 纸面这一侧同时补上两处真实缺口：文字框变成可选中且带 `aria-pressed` 的按钮，改动边写边按草稿重画（位置、对齐、文字都实时跟着走）；上一版只把草稿覆盖到"已存在的框"上，**还没保存的新图注在页面上根本看不见**，现在以虚线预览框显示，点它不会选中任何东西（版面里确实没有这条记录）。文字一律按纯文本渲染，内容 JSON 永不当成 HTML。
- 查看历史版本时图注完全只读：没有新增入口、页面上的文字框点不动、也不会把当前草稿盖到历史快照上。
- 文案中英日韩各 17 键，四份翻译表键数仍一致（各 583）。用例专门盯住翻译质量：英文表不许混进中日韩字符，日/韩不许原样照抄中文（首轮就抓到一条日文和中文同形，已改掉）。

### 验证结果

- 真链路（真数据库 + 真照片文件）：一条 Rust 集成用例走完整条路——加图注回执逐条为 `null`，重放同一命令得到等价回执；同值再提交 `changed=false` 且版本条数不变；把框推到出血外整笔回滚（版面与版本计数都不动）；空文字/超长/字号越界/坏对齐/非 UUID 编号一律被拒；往孤儿页面加字回 NOT_FOUND；删完再删回 NOT_FOUND；全程原始照片文件的指纹逐张核对不变。`cargo test --lib` 202 / 0 / 3（上一批 201，本批 +1）。
- 前端：63 个测试文件 / 460 项通过（上一批 62 / 448，本批 +1 文件 +12 项）。新增覆盖：默认落位那一组坐标真的落在安全区里（210×297、边距 15 的页面上是 15/264/180×18，而不是随手拍的数）；没写字时保存是灰的、写了字页面立刻出现预览；改已有图注走 `UpdateText` 且 `AddText` 一次都没被调用；删除不确认就一笔都不发、确认后才发出去；后端拒绝时不出现"图注已保存"、草稿仍在；历史版本下图注只读。仓储层两条新用例证明"三条命令各走一次通道、动作类型与载荷原样送达、确认后不留待重放"以及"越界载荷根本出不了门"。`npm run lint`、`npm run typecheck`、`npm run contracts:check`、`git diff --check` 全部退出 0。
- 其余三条 Rust 门禁：`--all-targets --no-fail-fast` 428 / 0 / 20（上一批 427，本批 +1）、`--lib --no-default-features --features native-agent` 207 / 0 / 4（上一批 206）、`cargo clippy --all-targets --all-features -- -D warnings` 退出 0。格式检查这一条要如实记一笔：**首轮是失败的**——本批新写的 Rust 里若干条链式调用和一条多参数函数签名没按 rustfmt 的换行规则排，`rustfmt --edition 2024 --check` 报出差异。先确认差异全部落在本批自己新增的行区间（不在既有代码里），再只对本批改动的两个文件跑 rustfmt 落盘、逐处比对仅缩进/换行变化、语义未动，重跑四条门禁后 `RUSTFMT CLEAN`。

### 仍需推进

- **纸面字号不是真比例**：版面框按百分比定位，预览用固定 12px 近似，导出/打印链路才按真实磅数排；要真正做到所见即所得得引字体度量，那和"文字会不会撑破框"是同一件事，都还没做。
- **图注编辑只在桌面端有**：三条写命令挂在桌面 Tauri 通道上，手机与网页既没有面板也发不出命令。
- **纸张尺寸、边距和出血还改不了**：核对时发现后端 `UpdatePage` 命令、校验、以及"改小到照片放不下就整笔回滚"的测试**早就在**，契约里也有，唯独前端从领域接口到界面一条通道都没接——这是工作包 3 剩下的最省事的一环，也是下一批要做的。
- **删除没有"撤销"按钮**：找回方式是版本历史（确认文案里写明了），不是在原地反悔。
- 未在你这台 Mac 上重打包安装，桌面 App 里现在还看不到图注面板；未提交、未推送、未部署。工作区仍含其他会话的未提交改动（`agent/*`、iOS、提示词等），不计入本批。

## 第十七批增量：工作包 3 第三项——纸张尺寸、安全边距和出血在界面上真的能改

上一批核对时发现一件很反常的事：`UpdatePage` 这条命令、服务端校验、批次回滚测试、契约里的量程**全都在**，唯独前端从领域接口到界面一条通道都没接——后端早就排好的座位，前端一直没去坐。按"已有实现以补缺和组合联调为主"的规矩，这一批几乎不造新东西，只把这条断头路接上，顺手把接上之后才暴露的两个真问题改掉。约束沿用——不加迁移、不改 schema、不动导出路径、不提交不推送。

### 实现

- 通道补齐：契约导出 `PublicationPageUpdate`，领域接口加可选方法 `editPage`，桌面仓储把它编译成一次只带一条 `UpdatePage` 的幂等命令，走和图注同一条带重放的通道。方法仍是**可选**的——网页与手机没有这条写命令，界面就不显示这个区，而不是显示一个按下去什么也不会发生的表单。
- 客户端规则与服务端 `validate_layout` **同源**，不自己另发明一套：页宽高一律 50–2000mm、出血 0–25mm、边距不小于 0，以及那条最容易被漏掉的"两倍的边距必须小于短边"。量程类的越界（出血 26、短边 40、页面 2001、编号不像编号）在出门前就被契约拦下，不占命令队列；而"边距吃掉整页"这种契约允许、服务端会拒的，本地也拦——省一次往返，也不让用户对着一个转圈的按钮猜。
- 关键的一处是**预览必须跟着草稿走**：纸张一改，页面比例、安全区参考线和每个图框的相对位置都按新的毫米数重画。这样"把 210mm 的页面缩成 148mm，原来 180mm 宽的照片放不下"是**当场看得见**的（预览里图框直接超出边界），而不是点保存后才知道被拒。测试就钉在这条因果上：断言图框的内联宽度百分比从 85.7% 变成大于 100%。
- 常见开本只换尺寸：A4/A5/正方形/横版四个预设只写宽高，**不动**边距和出血——那是作者自己定的成品规格，不该被一个"开本"顺手改掉。预设按输入框里的字面数值比对，手填的 `210.0` 不冒充 A4，宁可显示"自定义"。
- 一次只允许一份未保存的草稿：纸张改动没落地时"新增图注"是禁用的，图注草稿开着时纸张输入是锁着的。理由很直白——保存只发自己那一条命令，另一份草稿会被随后的刷新悄悄顶掉，这种丢失用户完全看不出来。
- 文案与无障碍：纸张的宽高不能沿用图框的"宽度（mm）/高度（mm）"，否则同一个面板里读屏会念出两个同名输入框，所以新增"页面宽度/页面高度"两条标签；纸张的保存按钮叫"保存纸张"、放弃叫"放弃纸张改动"，和图注的"保存调整"区分开。中英日韩各 12 键，四份翻译表键数仍一致（各 595）。
- 顺手补上一批留下的一个自证缺口：检查过的"改小到放不下会整笔回滚"其实**从来没有一条独立测试**证过——既有那条只是批次回滚用例里顺手塞进一个 50×50 的页面，它撞的是边距规则，根本没走到图框越界。本批补一条真链路用例：放大一档开本后断言四个毫米数原样落库、图框一格没动、版本 +1、同命令重放等价且不多记版本、原样回传 `changed=false`；再把页面缩到 60×60，断言报的是"内容超出页面及出血范围"、版面与版本计数都停在原处、原始照片逐张指纹不变。

### 验证结果

- Rust：`cargo test --lib` 203 / 0 / 3（上一批 202，本批 +1）；`--all-targets --no-fail-fast` 429 / 0 / 20（上一批 428，本批 +1）；`--lib --no-default-features --features native-agent` 208 / 0 / 4（上一批 207）；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；`rustfmt --edition 2024 --check` 无差异（新用例落盘前先 `--check` 比对，差异 10 处全部落在本批新增的 833 行之后，确认不碰既有代码才格式化整文件）。
- 前端：64 个测试文件 / 471 项通过（上一批 63 / 460，本批 +1 文件 +11 项）。新增 `pageDraft.test.ts` 把判定规则逐条钉住（含 120mm 页面配 60mm 边距被拒、配 59.9mm 通过这种贴着临界值的比对，以及四个开本预设自身都能过校验），四语言完整性有一例独立守护；界面用例覆盖换开本后页面与图框按新尺寸重画并真的发出 `UpdatePage`、无效值时保存按钮灰着且一条命令都不发（顺带证明"新增图注"这时也被锁住）、后端拒绝时原因原样上屏且不出现"纸张尺寸已保存"、草稿留在输入框里、翻回历史版本时这个区不出现、图注草稿开着时纸张输入禁用。仓储层新用例证明一次 `UpdatePage` 毫米数原样送达、确认后不留待重放，以及越界量程与坏编号根本出不了门。`npm run lint`、`typecheck`、`contracts:check`、`git diff --check` 全部退出 0。

### 仍需推进

- **逐页尺寸不回写整本规格**：纸张是**一页一页**存的，而"从作品序列生成"读的是创建时写在整本规格里的宽高（边距还硬编码 15mm）。所以手工把某几页改过之后再生成一次，会按老规格重排、把改动盖掉。要么加"整本统一开本"的一次性批量，要么让规格跟着页面走——这是个设计决定，没在本批擅自选。
- **一次只能改一页**：多页书稿要换开本得逐页操作，没有"应用到全部页面"。
- **出血改了但没验印前检查的联动**：把某页出血改成 0，印前检查应当新增一条相关项；规则读的就是同一份页面字段，理论上会联动，但本批没写这条联动用例，不当已验证。
- **只有桌面端能改**：`editPage` 是可选方法，网页与手机没有这条写命令，那个区也就不出现。
- **预览仍不是真排版**：沿用上一批的百分比定位，纸面字号与字体度量还是近似值。
- 未在你这台 Mac 上重打包安装，桌面 App 里现在还看不到纸张区；未提交、未推送、未部署。工作区仍含其他会话的未提交改动（`agent/*`、iOS、提示词等），不计入本批。

## 第十八批增量：工作包 4 第一环——展览不再被静默当成摄影书

工作包 3 收完三项后按规矩先核对工作包 4 的现状，结论和第十七批同型、甚至更反常：**展览这条路后端早就修完了，前端连入口都没给**。数据表有 `pub_type` 列、创建命令有 photobook/exhibition/portfolio 白名单、印前检查里专门写着"展览与作品集的序列未入册没有意义"这个分支、契约的 `Create` 也允许带 `pubType/widthMm/heightMm`——可是界面上的创建框只有一个标题输入框，仓储发出去的是 `{title}`。于是每一次"建一本展览册"都被 Rust 的默认值接走，落成 `photobook` + 210×297，用户从头到尾不知道自己建的其实是本 A4 摄影书。这一批仍然只做补路，不加迁移、不改 schema、不新增命令动作、不提交不推送。

### 实现

- 类型与开本成为**创建时的必填选择**，不是可选参数：领域接口收成 `create(projectId, title, spec)`，`spec` 是 `{pubType: PublicationKind, widthMm, heightMm}`。故意不给默认值——默认值正是上一版把展览吃掉的那只手。改签名时另一处调用点被 TypeScript 直接钉住（`TauriWorkspaceRepository.test.ts` 仍按两参调用），这也说明"必填"比"再加一层判空"更靠得住。
- 仓储不做第二次校验、也不补默认：类型与毫米数原样交给契约，越界与陌生类型在 `publicationRequestSchema` 就被拦下，出不了门也不占命令队列。新建那本册的编号仍由前端先定（`crypto.randomUUID()`），回执里的 `id/project_id/revision` 必须与请求一致才算创建成功——这条对齐本来就有，本批给它补了独立用例。
- 创建框多两个选择：类型（摄影书／展览／作品集）与开本（沿用纸张那一套预设，**新增 A3 297×420**——展签与大幅面打样的常见尺寸，只有 A4 的话建完展览册第一件事还是逐页改纸张）。只读项目里这两个下拉和标题框、创建按钮一起禁用；不然用户会先把展览改成别的类型、再发现根本建不了。
- 卡片上的类型标签改走翻译表，不再用界面里硬编码的那份中文映射；库里出现契约之外的陌生值时**原样露出**（`calendar` 就显示 `calendar`），不套一个中文标签把它盖住。中英日韩各 +4 键，四份表键数仍一致（扁平化后各 887）。
- 三种类型的名字在每种语言里都必须互不相同，用例专门断言这一点——选完还是同一个词等于没得选。
- 一条真实库用例把"选了就算数"钉到底：同一段序列、同一张 3000×4000 的原图，分别按摄影书 A4 和展览 A3 创建并生成，断言①页面毫米数就是创建时那两个数（封面与内页都是，不是只有整本规格里记着）；②同一把 300 PPI 尺子在 A3 上判出 2 项阻断、实测落在 260–285 之间，而 A4 那本 0 项阻断——证明展览册没被当成 A4 摄影书算；③册生成后再多选一张进序列时，摄影书收到"漏排"提醒，展览册不收——那个跳过分支此前**只有代码、没有测试**。

### 验证结果

- 前端：64 个测试文件 / 477 项通过（上一批 64 / 471，本批 +6 项）。`PublicationPanel.test.tsx` 从 2 项扩到 4 项，含"选展览 + A3 就真的按这两项创建"与"陌生类型原样露出"；`TauriPublicationRepository.test.ts` 6 → 8 项，新增创建带类型与开本、以及陌生类型／2001mm／49mm／非数字尺寸都到不了 IPC；`pageDraft.test.ts` 5 → 7 项，补类型文案四语言完整性与开本标签可读性。`npm test`、`npm run lint`（--max-warnings 0）、`typecheck`、`contracts:check` 全部退出 0。
- Rust：`cargo test --lib` 203 / 0 / 3（与上一批持平，本批新用例是集成测试）；`--all-targets --no-fail-fast` 430 / 0 / 20（上一批 429，+1）；`--lib --no-default-features --features native-agent` 208 / 0 / 4；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；`cargo fmt --all -- --check` 无差异（新用例所在文件先单独 `--check`，唯一 1 处差异就在本批新增的 `placed` 辅助函数那一行，确认不碰既有代码才格式化该文件；`tests/publication_preflight.rs` 现 4 项全绿）。

### 仍需推进

- **展览目前只是"不是摄影书"，还没有展墙**：类型影响到的行为只有两处——创建时记的类型，和印前检查跳过整本选片核对。页面生成仍然按书的办法排（封面加内页），展墙尺寸、照片间距、观看路径、展签与作品清单导出都还不存在。这几项要不要进数据表、进哪种表，是 schema 决定，等你拍板再动。
- **类型建完不能改**：契约里没有 `UpdateType` 之类的动作，选错只能另建一本。补动作属于新增命令面，本批没擅自加。
- **逐页改动仍不回写整本规格**（第十七批遗留）：现在创建时的开本真的会落到页面上，这条不一致更容易被看见——手工改过纸张后再"从序列生成"，仍按整本规格重排。
- **展览的印前口径要再确认一次**：跳过"选片没入册"是对的，但展览更该判的是"打印尺寸够不够、留没留装裱白"。现在它拿的是和摄影书同一套 PPI 与出血规则，没有展览专属阈值。
- **只有桌面端有这条链路**：手机与网页连出版物面板都不显示（`repository` 为空时只给一句提示），Swift 侧也完全没有 `pub_type` 相关调用；这一环不是"移动端没做完"，是根本没开始。
- **没在你这台 Mac 上重打包安装**：桌面 App 里现在还看不到类型与开本两个选择；交互只用 jsdom 测过，真机运行未验。未提交、未推送、未部署。工作区仍含其他会话的未提交改动（`agent/*`、iOS、提示词等），不计入本批。

## 第十九批增量：工作包 4 第二环——整本一次统一开本

第十七批留下的最刺眼一条是"一次只能改一页"：纸张是一页一页存的，逐页改过的书整本开本不一致，印前检查里那条"页面尺寸不一致"的警告就是这么来的，而要把它抹平得把每页都点开、每页都保存一次。这一批仍然几乎不造新东西——契约早就允许一条命令带多条 `UpdatePage`（只有超版 Save/Restore 那类操作才要求单独成条），Rust 侧的 `validate()` 是同一条规则，多个操作在同一个事务里逐条落库、最后统一 `validate_layout`，天然整笔回滚。缺的还是前端那一步。

### 实现

- 通道：领域接口加**可选**方法 `editPages`，桌面仓储把 N 页编译成 N 条 `UpdatePage`，仍走和图注、逐页纸张同一条带重放的幂等命令；非创建类的回执按契约每条回一个 `null`，整本改完版本只 +1。方法仍是可选的——网页与手机没有这条写命令，那个区就不出现，而不是摆一个按下去什么都不会发生的按钮。
- **目标尺寸从哪来**有两种：输入框里有草稿时用草稿，没有草稿时用当前页面已存的四个毫米数（`pageGeometry`，直接从页面对象取，不绕输入框那层字符串）。后者是关键——"把这本被手工改乱的书拉回一致"本来就不该要求用户先动一遍输入框。
- 客户端**预检要报名字**：`boxFits` 与服务端 `rectangle()` 同源（内容必须落在 `[-出血, 页面+出血]` 内），`wholeBookBlockers` 把按新尺寸放不下照片或图注的页挑出来，界面直接列出"封面、第 2 页在新尺寸下放不下"。这件事只能在出门前做——服务端那句是通用的"内容超出页面及出血范围，请调整位置和尺寸"，**不含任何页码**；新的 Rust 用例专门断言这条消息里没有数字，把"前端必须点名"钉成一条被测的事实，而不是注释里的口头约定。
- 禁用的几种情形都有各自的理由：只读与历史视图不给写；只有一页的书不给"整本"；每页本来就一致时禁着——原样提交不算改动，但按一次按钮就白记一个版本；有页放不下时禁着并点名；页数超过一条命令能带的 100 个操作时禁着并提示逐页改（契约与服务端 `validate()` 都拒 >100，发出去注定失败）；图注或图框草稿未落地时禁着，沿用上一批"一次只允许一份未保存草稿"的规矩——批量改纸会顶掉那份草稿，而且用户完全看不出来。
- 文案 5 键 × 4 语言，四份翻译表键数仍一致（各 892）。列举放不下的页时用标点键（中/日/韩顿号、英文逗号），一致性检查正当豁免标点；同时新增一条"日文串里不得混入谚文"的回归断言——本批写日文那句时真的混进过一个韩文字，正被这条抓住。

### 验证结果

- 前端：64 个测试文件 / 489 项通过（上一批 64 / 477，本批 +12）。`pageDraft.test.ts` 加 3 例：`boxFits` 贴着出血临界值逐边比（`-3/-4`、宽 +1、高 +1 各自的判法，零出血时贴边可用越出即拒）；`wholeBookBlockers` 用"图框超"和"图注超"两种页各点名一次，并断言空页永不误报、按页面自己的原尺寸比对时整本都放得下；`pageGeometry` 取数与 `MAX_WHOLE_BOOK_PAGES=100`。仓储层 1 例证明一次 `editPages` 只发一条 `pub_execute`、N 条操作各自带页面归属、版本 +1、确认后不留待重放，以及空列表和第 101 页根本出不了门（100 页正好通过）。界面 8 例：整本一次点完就发出每页的四个毫米数且不再补逐页命令、按草稿里的新尺寸统一并当场点名放不下的页（换回 A4 后按钮重新可用）、整本已一致时按下去不发命令、单页书不给入口、没有批量写通道时整区不出现、只读视图入口在但按不动、101 页时提示逐页改且不发命令、后端拒掉时原因原样上屏且不出现"整本纸张已统一"。既有那条"一次只改一份草稿"的用例改用多页夹具，顺带证明批量按钮也被同一条规矩锁住。`npm run lint`、`typecheck`、`contracts:check`、`git diff --check` 全部退出 0。
- Rust：`cargo test --lib` 204 / 0 / 3（上一批 203）；`--all-targets --no-fail-fast` 431 / 0 / 20；`--lib --no-default-features --features native-agent` 209 / 0 / 4；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；`cargo fmt --all -- --check` 无差异（新用例落盘前先对单文件 `--check`，4 处差异全部落在本批新增的段落里，确认不碰既有代码后只格式化那一个文件）。新用例 `one_batched_command_unifies_every_page_…` 先把末页单独放大制造真实的不一致，再用**一条命令**把三页统一到 A4：断言每页四个毫米数原样落库、每页图框一格没动、版本只 +1、同命令重放等价且不多记版本；随后提交"第一条放大合法 + 第二条把末页缩到 60×60"的批次，断言报的是内容越界、版面与版本计数都停在原处、原始照片逐张指纹不变——半本新纸张都不许留下。

### 仍需推进

- **统一的是页面，不是整本规格**：`spec_json` 仍是创建时那两个数，逐页改和整本改都不回写。影响面比第十七批描述的小（"从作品序列生成"只允许在没有页面时跑一次），但新建一本书时"上次改过的开本"依然带不过去，这是个设计决定，没在本批擅自选。
- **超 100 页只能逐页改**：批量靠一条命令带 N 条操作，上限就是契约的 100；没有服务端原生的"应用到全部页面"操作。
- **统一后印前的"页面尺寸不一致"警告应当消失**：规则读的就是同一份页面字段，理论上必然联动，但本批没写这条联动用例，不当已验证。出血改动与印前的联动（第十七批遗留）同样仍未验。
- **只有桌面端能改**：`editPages` 是可选方法，网页与手机没有这条写命令；Swift 侧连 `pub_type` 都没有。
- **预览仍不是真排版**：沿用百分比定位，纸面字号与字体度量还是近似值。
- 未在你这台 Mac 上重打包安装，桌面 App 里现在还看不到"应用到全部页面"；交互只用 jsdom 测过，真机运行未验。未提交、未推送、未部署。工作区仍含其他会话的未提交改动（`agent/*`、iOS、提示词等），不计入本批。

## 第二十批增量：工作包 4 第三环——送审、定稿、交付真的能走

### 实现

- **这环真正的缺陷不是"状态没接线"，而是"定稿是一扇单向门"**：执行器原先要求任何写命令都得在草稿上跑，于是状态一旦被改成审阅中或已定稿，连"退回草稿"这条命令都发不出去——想救只能手工改数据库（这正是以前发生过的局面）。现在 `UpdateState` 是唯一绕过"当前必须是草稿"这道前提的动作，其余动作照旧锁在草稿里。
- 后端：新增动作 `UpdateState(StateChange)` 和 `PublicationState` 枚举（序列化沿用库里那四个 snake_case 值，`as_str`/`display`/`label` 三种写法都在同一处定义）。流转表 `next_allowed` 是全项目唯一定义：草稿→审阅中；审阅中→退回草稿、定稿；定稿→审阅中、交付；已交付为终点。跳级报 `CONFLICT` 且消息带"逐级"字样，契约之外的状态字符串在 serde 层就被拒。同一状态再提交一次按空处理：`changed=false`、版本不涨、不记历史。
- 记账顺带做对了：`layout_tx` 的 SELECT 本来就带 `b.state`，所以状态一变 `changed` 自动为真，版本号 +1、历史区自动多一条按动作命名的记录（送审/退回草稿/定稿/交付），不需要为状态另设一条版本命令。恢复历史版本那条路不写 `state`，所以还原一版不会把已定稿的本子悄悄解回草稿。
- 契约与通道：`packages/contracts` 加 `UpdateState` 动作和 `state` 枚举，状态字符串从此只在契约里定义一次（`publicationRecordSchema` 也改用它）；既有的"版本类与状态类动作必须单独成条"超验规则原样适用，前端仓储和 Rust `validate()` 两处都不必放宽。领域接口加**可选**方法 `changeState`，桌面仓储把它编译成一条命令一个操作；网页与手机没有这条写命令时那个区不出现，而不是摆一个按下去什么都不会发生的按钮。
- 界面：版面与版本窗口在纸张区之后、印前检查之前多一段"审校状态"——当前状态、只能逐级走的说明、以及当前状态下合法的那几个下一步按钮；已交付不给按钮只把话说完；不在草稿时额外说明版面已只读。这段刻意**不受** `writable` 影响（否则又是一次单向门），只受只读、历史视图、未确认命令、未保存草稿四项限制：最后一条沿用第十七批的规矩，状态命令会连带刷新版面，不能让它把没落地的图注或纸张草稿顶掉。
- 状态一律取**刚读回来的版面**，不取列表里那份 `publication` 副本。本批之前状态不会在窗口内变化，两者恒等；现在会变了，旧副本会让"恢复此版本"该给的不给、不该给的给（后者点下去必被后端拒）。同一处判断也收进可编辑标志。两头都用新用例钉住。
- 列表卡片的状态徽章原先硬编码四个中文词，现在走翻译表；认不出的值原样露出，不编一个标签把它盖住。前端那份流转表是后端镜像，库里读到没见过的状态就按原样显示、不给任何下一步——迁移 0023 的 `CHECK(state IN …)` 其实已经排除这种数据，这层只算防御，不当作已启用的功能。
- 文案 12 键 × 4 语言，四份翻译表键数从 892 到 904；沿用第十九批那条"日文串里不得混入谚文"和"英文不得残留汉字"的一致性检查。

### 验证结果

- 前端：65 个测试文件 / 501 项通过（上一批 64 / 489，本批 +1 文件 +12 项）。新文件 `publicationWorkflow.test.ts` 3 例：流转表逐格比对服务端那份（草稿只给审阅中、审阅中给退回草稿与定稿、定稿给审阅中与交付、已交付给空；任何状态都不自环）、库里读到陌生状态时原样显示且下一步为空、四套语言的 12 个键都有值且英日韩互不回落。仓储 2 例：`changeState` 只发一条 `pub_execute`、命令里只有一个 `UpdateState` 操作、回执 `[null]`、版本 +1、确认后不留待重放；陌生状态出不了门（连 IPC 都没被调用），而"状态动作 + 版面动作"拼成的批次在契约层就被判无效——前后端两份规则各拦一次，用户不会收到一条注定失败的命令。界面 7 例：草稿只出现"送审"一个按钮且点它不顺手发出任何版面或版本命令、审阅中的本子版面只读但能在状态区退回草稿（并断言"保存纸张"确实按不动）、已交付不给任何按钮、定稿的本子不会凭空多出"退回草稿"、只读打开 / 看历史版本 / 没有 `changeState` 通道三种情况整区不出现、有一条未确认操作时按钮按着、手里有未保存纸张草稿时按钮也按着且不发命令、以及"恢复入口跟着刚读回来的状态走"两头各验一次（列表副本陈旧时不给必被后端拒的恢复；刚退回草稿时恢复立刻可用）。`npm run lint`、`typecheck`、`contracts:check`、`git diff --check` 全部退出 0。
- Rust：`cargo test --lib` 205 / 0 / 3（上一批 204）；`--all-targets --no-fail-fast` 432 / 0 / 20；`--lib --no-default-features --features native-agent` 210 / 0 / 4；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；`cargo fmt --all -- --check` 退出 0（本批新代码里有两处 rustfmt 和一处 `needless_borrows_for_generic_args` 需要先改掉才谈得上绿灯：把越界提示先落成 `let message`，两处折行按格式化器意图手改，都只落在自己新增的段落）。新用例 `review_states_move_one_step_at_a_time_and_keep_a_finalised_book_locked_until_it_is_returned` 一条走完：契约之外的状态字符串在 serde 层就被拒；草稿直接定稿、草稿直接交付都报 `CONFLICT` 且消息含"逐级"；送审成功（`changed=true`、回执 `[null]`、版本 +1、`publications.revision` 与回执一致、历史首条标签是"送审"、版本行数 +1）；送审后的本子上 `UpdateFrame`/`UpdatePage`/`CaptureVersion` 全部 `CONFLICT`，版面与版本计数纹丝不动；同一命令重放得到等价回执；换新编号原样再提交一次是同状态空操作（`changed=false`、版本不涨、不记历史）；"退回草稿"之后一条真实的版面改动能够落库——那扇单向门确实是开的；一路走到已交付后，从已交付回草稿被拒。既有那条整本统一纸张的用例同时证明状态与批量纸张共用同一条幂等通道没有互相踩到。

### 仍需推进

- **状态只是闸门，不是审阅**：`in_review` 现在唯一的含义是"版面不许再改"。它和画布那套成熟的协作评审（`canvas_editorial_comments.rs` 的评论串/回复/解决、`canvas_review_decisions.rs` 的按字段采纳与驳回、云端 `OWNER/ADMIN/EDITOR/REVIEWER/VIEWER` 五种身份）**完全没有连线**——出版页面上没有批注、没有审阅人身份、没有逐图框的同意/驳回。工作包 5 真正的缺口在这里，而不是状态本身。
- **没有交付物出口**：出版物侧只有 `pub_create/pub_execute/pub_generate/pub_layout/pub_list/pub_pages/pub_preflight/pub_version(s)` 这 9 条命令，**没有任何导出**（PDF、印厂包、作品清单都没有）。所以"已交付"目前不可能触发一次真交付，"定稿也不许导出"这种规则无从谈起；交付语义要在有导出通道之后才完整。
- **展览仍然只是"不是摄影书"**：`pub_type` 为 exhibition 时唯一的真实差别是印前跳过"序列里照片没排完"这一条（`publication_preflight.rs:393-404`），其余阈值全套书本页的 PPI；墙位、悬挂高度、作品实际尺寸、观看动线、展签、清单导出都不存在，也不在任何迁移里。这一环需要一次数据模型决定，没在本批擅自定。
- **只有桌面端能改状态**：`changeState` 是可选方法，网页没有这条写命令；手机上不是"缺这条命令"，而是整个出版物界面都没上过（`apps/mobile-ios/FocalRoom` 里检索不到任何 publication 相关代码）。
- **状态流转仍是逐级且无角色**：任何能打开这个窗口的人都能送审、定稿、交付，本地没有"谁能定稿"的区分；这是设计缺口不是缺陷，但它和上一条合起来意味着现在的审校更像锁屏开关，不是评审流程。
- 未在你这台 Mac 上重打包安装，桌面 App 里现在还看不到"审校状态"这一段；交互只用 jsdom 测过，真机运行未验。未提交、未推送、未部署。工作区仍含其他会话的未提交改动（`agent/*`、色彩、偏好面板、iOS 等），不计入本批。

## 第二十一批增量：出版物有了交付出口——作品清单 CSV

### 实现

- **补的是第二十批点名的那条洞**：出版物侧原先只有 9 条命令、没有任何导出，"已交付"只是道闸门。现在多了第 10 条——只读的 `pub_export`，把整本版面摊成一张逐条可核对的作品清单（CSV）写到用户选的位置。
- 新模块 `publication_export.rs`：15 列表头（页码、左右页、类型、格编号、四个毫米数、交付宽高像素、有效 PPI、裁切倍率、适配模式、图注文本、素材编号），版面里每一格出一行——照片格 `frame`、文字格 `text`。
- **像素与 PPI 一律复用印前那套算法**：`delivered`/`effective_ppi` 从 `publication_preflight.rs` 放开成 `pub(crate)` 后直接调用，不在导出侧另算一遍。自检说分辨率不够、清单却写着够，用户就拿着这张纸去开机了。这条复用由真库用例逐格比对钉住（同一格读回的表里的数 == 印前函数算出的数，容差 0.005）。
- 读序：同一页先照片后文字，各自从上到下、从左到右；毫米坐标按整毫米取整后排——版面拖拽产生的 0.001mm 级抖动不该把一条图注甩到下一张照片后面。
- 三处"宁可留空也不假报"：非有限数（`0/0` 会得到 `inf`）留空；帧没有源图时像素两列留空而不是写 0（"分辨率 0"是句假话）；图注里存的结构读不出来时留空，不把原始 JSON 甩进表格冒充文本。
- 表格本身按 RFC4180 转义（图注含逗号、引号、换行都能原样读回），全表加 BOM 与 CRLF——Excel 与 Numbers 缺了 BOM 会把中文图注按本地编码猜，第一列直接花屏。
- 文件名取自书名，路径分隔符等字符换成 `-`，空标题回落 `publication`，限 80 字符；落盘走系统保存对话框（沿用 `export_project` 那条既有模式），要求绝对路径，回执复用导出模块的 `ExportReceipt{path,sha256,bytes}`，SHA 用与既有交付一致的指纹函数现读文件算。
- 前端：领域接口加**可选**方法 `exportWorklist`，仓储只发一条 `pub_export`、不带修订号、不进幂等队列，并用 schema 卡住回执（SHA 必须 64 位）；界面在印前检查之后多一段"作品清单导出"，历史版本视图不给入口。
- **取消不是失败**：后端在对话框取消时回 `Ok(None)` → 前端 `null` → 提示"已取消导出，没有写入文件"，不报错误、不摆一张没写成的回执。
- 导出**绕开 `run()`**：`run()` 在只读视图里直接返回，而"看别人的本子并出货"正是最常见组合；因此导出用自己的锁与忙碌位，只受"连点不重复发"约束，只读视图照样能导。
- 回执会过期：任何一次写命令成功都把屏上那张旧回执清掉——版面已经变了，旧的那张纸不再对应当前这本。
- 文案 7 键 × 4 语言（翻译表键数 904→911），`{items}`/`{path}` 两个占位符四套语言都必须存在并有一致性断言；沿用"英文不得残留汉字、日文不得混入谚文"两条检查。
- 刻意**没做**的三件事：不加"印前不过就不许导出"的闸门（那是产品决定，没擅自定）；不虚构照片↔图注的关联（版面数据里本来就没有这层关系，表里就是两条行）；摄影书与展览目前产出同一张表（有测试记的就是"仍然共用"这个事实，不是成绩）。

### 验证结果

- 前端：65 个测试文件 / 511 项通过（上一批 65 / 501，本批 +10：界面 7、仓储 2、工作流 1）。界面 7 例：导出只发一条命令且不顺手发任何版面/状态/版本命令、条数与落盘路径同时上屏（"已导出 2 条"）、保存对话框取消只提示不摆回执、后端报错与坏回执都不会把路径写成"已导出"、历史版本视图与没有导出通道时整区不出现、**只读视图照样能导**（走的是读通道，这正是"看别人的本子并出货"那条路）、写命令一旦成功旧回执立刻清空、连点两次只发一条命令。仓储 2 例：`pub_export` 只发一条命令、不带修订号、`pendingCommands` 仍为空（没进幂等队列）；`null` 原样透出，五种坏回执（空路径、短 SHA、负字节数等）全在 schema 层被拒。工作流 1 例：7 个键 × 4 语言都有值，`{items}`/`{path}` 两个占位符四套语言一个都不能少，英日韩互不回落。`npm run lint`、`typecheck`、`contracts:check` 全部退出 0；翻译表四份各 911 个扁平键（上一批 904），缺失 0、多余 0，唯一两处空值仍是本批之前就存在的 `copy.editorHint`、`copy.memoryHint`。
- Rust：`cargo test --lib` 218 / 0 / 3（上一批 205，本批 +13 = 12 条单元 + 1 条真库）；`--all-targets --no-fail-fast` 445 / 0 / 20（上一批 432）；`--lib --no-default-features --features native-agent` 223 / 0 / 4（上一批 210）；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0（首轮报的三处都在自己新增段落：`sort_by` 换 `sort_by_key`、测试里 `!= ""` 换 `!is_empty()`）；`cargo fmt --all -- --check` 退出 0（本仓 crate 是 **edition 2024**，用 `--edition 2021` 单文件 `--check` 会给出误导性 diff，别按那个改；两处真实差异都只落在本批新增段落，只格式化那两个文件）。
- 12 条单元用例点名几条关键的：表头与每一行的列数一致；图注里同时含逗号、引号、换行时原样往返（断言那一行确实以 `"2,left,text,t1,` 开头，证明转义没把行拆开）；空书也产出一张只有表头的可打开表格；`fill` 取更紧的方向而 `fit` 取更松的（105×75mm 配 3000×2000、裁切 2 倍，两个方向各自手算再断言比值 > 1.05）；清单里的像素是所选 rendition 而非原图（4000×3000 → 1200×900，PPI 按 900 那一侧算）；帧没有源图时像素两列是空的而不是 0；读序按上→下、左→右且不受亚毫米抖动影响；摄影书与展览产出同一张表（这条记的是**还没做到**）；读不出的文字载荷留空；文件名去掉路径分隔符、空标题回落；非绝对路径直接拒；回执的 SHA 与字节数和盘上文件一致。
- 真库那条 `the_delivery_sheet_repeats_the_stored_layout_and_the_preflight_numbers`：先往库里第二页写一条带逗号引号换行的图注，再从**读回的版面**渲染表格——行数 = 版面格数 + 表头、图注原样往返、每一帧行的页码/左右页/素材编号都与库里一致；关键是**表里的有效 PPI 与交付像素两列等于印前函数对同一帧算出的数**（容差 0.005），把"两份依据不许打架"钉成一条断言而不是注释。随后写进一个中文目录路径，断言回执字节数等于字符串长度、文件内容与字符串逐字节相同、SHA 与独立算出的指纹相同，最后原始照片逐张指纹不变。

### 仍需推进

- **摄影书与展览共用一张表**：清单里没有墙位、悬挂高度、作品实际尺寸、展签编号、画框与装裱数据——展览策划真正需要的字段还不在 `spec_json` 里（第二十/二十一批都记过这条）。
- **这不是印厂包**：不产 PDF、不做拼版与出血/裁切标记、不带 ICC 或色彩配置，也没有打包目录。它只是"逐条可核对的一张纸"，能否开机仍取决于接单方认不认这张纸。
- **导出不受印前拦截**：分辨率不足照样能导出，也不会在表上标"此条不合格"。要不要闸门待定；如果要，规则应写在服务端而不是只在界面藏一个按钮。
- **状态与交付仍未连线**：把状态改成"已交付"不会触发导出，导出也不改变状态；"交付 = 已出货"这条语义还没落进任何一处校验。
- **只有桌面端能导**：`exportWorklist` 是可选方法，网页没有这条 IPC；手机上不是缺这条命令，而是整个出版物界面都不存在。
- 未在你这台 Mac 上重打包安装，桌面 App 里现在还看不到"作品清单导出"这一段；交互只用 jsdom 测过，真机点按未验。未提交、未推送、未部署。工作区仍含其他会话的未提交改动（`agent/*`、色彩、iOS 等），不计入本批。

## 第二十二批增量：工作包 4 第四环——展览按观看距离被判，不再套书本的 300 PPI

### 实现

- **补的是第十八批记下的那条洞**：`pub_type` 为 exhibition 时，原先唯一的真实差别是印前跳过"序列里照片没排完"这一条，分辨率仍按手里画册的 300 PPI 判。一张准备挂在 2.5 米外的放印因此永远报"分辨率不足"，而这条红字是错的——判据用错了，用户只能学会无视它。
- **不建表、不动 schema**：观看距离与悬挂中心线是两个毫米数，落进已有的 `publications.spec_json`（本来就装着页幅宽高，无固定结构）。加列属于生产 schema 变更，本批不越权；两个数的信息量也不值得开一张表。
- **判定是光学不是排版**：新模块 `publication_spec.rs`，要求 PPI = `25.4 / (1 弧分弧度 × 观看距离毫米)`（`ACUITY_RAD = 0.0002908882`）。2.5 米 → 约 34.9 PPI；254 毫米（近到手边）算出 300 以上，被压回基线；100 米 → 落到 25 PPI 的下限。**只会放松、从不收紧**：`clamp(25, baseline)`，摄影书那条 300 一分没动。
- **量程与脏值**：观看距离 300–20000 毫米、悬挂中心线 300–3000 毫米。库里读不出来、非数、越界一律当"未记录"，不把垃圾回填成"用户填过的数"（一显示出来，下次保存就等于替它作保）；写入前在服务端做同一道量程检查，拒在出门前，错误消息带中文范围。
- **新命令 `UpdateSpec`**：契约强制单独成条（不能和改图框拼一条，否则会做出"图框改了、判据没换"的半份结果）；只有展览册能用，摄影书拿到 `CONFLICT`；只有草稿状态能改，送审后同样拒。幂等由**读回来的两个数**判定而不是比字符串——键序和空格不算"用户又改了一次"，同值重放 `changed=false`、不涨修订号、不多记一条历史。历史标签"定展览规格"，恢复那次会把两个数一起带回。
- **印前三条分支**：展览 + 记了距离 → 用距离算出的要求，码换成了 `exhibition_low_resolution`，实测/要求各带自己的数；展览 + 没记距离 → 仍按基线判，并额外给一条 `viewing_distance_missing` 提示"现在用的是书本那把尺"；摄影书与作品集 → 行为完全不变，就算 `spec_json` 里混进一个 `viewing_distance_mm` 也不会被放松（有用例钉住）。规格读炸了也不炸检查，回落基线。
- **前端**：契约里 `UpdateSpec` 两个数各自 nullable + 量程；新 `exhibitionSpec.ts` 负责读回与出门前校验（留空 = 明确送 `null`，一个数越界整条都不出门）；领域接口加**可选** `editSpec`，仓储只发一条命令、不进幂等队列；界面在印前检查之前多一段"展览规格"，只在展览册出现，量程外的值保存按钮按住并当场说清范围，只读视图禁着、看历史版本整段不给。
- **一次只留一份草稿**这条老规矩补齐到三方：规格 ↔ 纸张 ↔ 图注互相挡位，任何一处草稿没落地时另外两处的输入和保存都按住；写成功后规格草稿与导出回执一起清掉。
- 文案 11 键 × 4 语言（翻译表键数 911→922），沿用"英文不得残留汉字、日文不得混入谚文、四套互不回落"三条检查；"定展览规格"这句是后端写进版本标签的原话，用例把它钉住，改一处必得改两处。
- 刻意**没做**的：不虚构墙位、房间、展签编号、动线这些还不存在的实体；不放松摄影书；不把 25 PPI 之下再往下放；导出清单不带这两列；印前仍不拦导出。

### 验证结果

- 前端：66 个测试文件 / 526 项通过（上一批 65 / 511，本批 +15 = 单元 5、界面 8、仓储 2）。新 `exhibitionSpec.test.ts` 5 例：脏值与越界都读成"未记录"、页幅那两键不算规格、留空是 `null` 不是 0、一个数坏掉整条不出门、**界面量程与契约是同一套数**（UI 放行的都能过 `publicationActionSchema`，UI 拦下的契约也拦）。界面 8 例：展览册才出现规格区而摄影书不出现、没有写通道时整段不给入口、改距离只发一条 `editSpec` 且 `editFrame/editPage/editPages/changeState/saveVersion` 一条都没顺手发、保存后新值上屏且按钮回到按住（夹具会随写入变，测的是"真落库再读回来"）、清成空送的是两个 `null`、20001 被按住并说清范围且改回 20000 后恢复、撤销改动回到库里那份、只读与历史版本各按其份、图注草稿开着时两个框读不进、反过来规格草稿挂着时纸张整侧按住、后端拒掉时说清原因不冒充已保存。仓储 2 例：一次 `pub_execute` 只带 `UpdateSpec`、`pendingCommands` 仍为空；四种越界（299 / 20001 / 3001 / `NaN`）到不了后端，`UpdateSpec` 与 `UpdateFrame` 拼一条被契约拒。`npm run lint`、`typecheck`、`contracts:check` 全部退出 0；翻译表四份各 922 个扁平键，缺失 0、多余 0，空值仍只有本批之前就存在的 `copy.editorHint`、`copy.memoryHint`。
- Rust：`cargo test --lib` 231 / 0 / 3（上一批 218，本批 +13 = 规格单元 7 + 印前 5 + 真库 1）；`--all-targets --no-fail-fast` 458 / 0（上一批 445）；`--lib --no-default-features --features native-agent` 236 / 0 / 4（上一批 223）；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；`cargo fmt --all -- --check` 退出 0。
- **clippy 抓到一条真错，值得记下来**：新增的那条真库用例被塞进了上一条用例的函数体里（这个文件是平铺的，没有 `mod tests` 包一层），`#[tokio::test]` 落在内层函数上——rustc 报 `cannot test inner items` 后**静默不收这条用例**，`cargo test --lib publication` 45 条全绿、`--all-targets` 也全绿，只有 clippy 的 `dead_code` 把它露出来。搬出外层并去掉一层缩进后它才真的跑（lib 从 230 涨到 231）。教训：这个仓里"测试数没涨"比"测试失败"更早说明问题。
- 规格 7 条单元点名几条关键的：2.5 米算出的要求远低于 300 且有下限；放松永远不越过更严的基线；没记距离时不给任何判定；页幅两键在合并后原样留着、只有两个展览字段动；超出物理范围的值到不了库；`spec_json` 是垃圾时读成"未记录"而不是让检查崩；一条坏规格仍可被命令修好。
- 印前 5 条：600×400mm 的放印配 3000×2000 像素（约 127 PPI）在 2.5 米判定下一条都不报，而同一帧同一份像素装进摄影书仍是那条 `low_resolution`——书本不看观看距离；仍然不够时报的是这面墙自己的两个数（实测 25 / 要求 34），同一条帧不会被书本口径再报一次；没记距离的展览拿到那条提示；摄影书 `spec_json` 里混进距离也不放松；读不出的规格把书留在印刷基线上而不是崩掉。
- 真库那条 `an_exhibition_keeps_its_own_viewing_distance_and_a_book_refuses_to_borrow_it`：摄影书先被 `CONFLICT` 拒掉且库里没有那个键；改成展览册后一次写入 +1 修订、历史首条是"定展览规格"、`width_mm` 仍在；同一条命令重放 `changed=false` 且不涨版本；新命令送同样的两个数也不涨版本；`Some(2500)/None` 只清掉悬挂中心线、留着观看距离与页幅；三种越界都 `VALIDATION_ERROR` 且库里值没动；送审之后再改规格被 `CONFLICT`。最后原始照片逐张指纹不变。

### 仍需推进

- **展览的全部结构化数据就是这两个数**：没有墙、房间、作品实际尺寸、装裱方式、展签编号、观看动线，一本展墙册和另一本之间也表达不出关系。这一环需要一次数据模型决定，没在本批擅自定。
- **悬挂中心线存了但没被任何判定使用**：它说的是装裱高度，不是分辨率；清单、展签、印前都还没读它。
- **放松是单向的**：只把墙上的大幅从 300 里救出来（最低 25 PPI），不会让手里的画册变严，也不会因为"观众可能凑到 30 厘米外看"而收紧。真要按最近观看距离判，得有那个输入，界面上现在没有。
- **导出清单仍不带展览字段**：那张 CSV 里既没有观看距离也没有悬挂中心线，接单方看到的是同一张书本用的表；印前不过仍能导出，"已交付"状态与导出仍未互相触发。
- **只有桌面端**：`editSpec` 是可选方法，网页没有这条通路；手机上不是缺这段界面，是整个出版物界面都不存在。
- 未在你这台 Mac 上重打包安装，桌面 App 里现在还看不到"展览规格"这一段；交互只用 jsdom 测过，真机点按未验。未提交、未推送、未部署。工作区仍含其他会话的未提交改动（`agent/*`、色彩、iOS 等），不计入本批。

## 第二十三批增量：工作包 5 第一环——审阅意见挂在这一本的版本上，定稿不再无声跨过没处理完的意见

现状核对（先读代码再动手）：这个项目里"讨论"已经有三套互不相通的数据——画布便签（`board_annotations`）、画布编辑线程（从不可变的 `canvas_command_events` 投影，锚在草案 id + 草案修订号上）、以及本地评论线程（`comment_threads` / `comments`，锚点只有 `photo` 与 `project` 两种）。建议通道（`proposals` + nonce 部分采纳）与出版物审校状态机各自完整，但 `publication_versions` 里没有任何一列能放"谁审的"，状态流转记的是"定稿"这个动作本身，无处引用任何一条意见。所以这一环补的不是新界面，是最薄的那条连接：**意见能落到被审的那一版，批准动作不能越过没处理完的意见**。

### 实现

- **不新起一套数据**：给既有评论线程增加第三种锚点 `publication`，带这本的编号与提意见时那一版号。零新表、零迁移、零新列，`comment_threads.anchor_json` 本来就是无结构的 JSON。
- **锚点校验与照片锚点同构**：那本必须真实存在于这个项目（否则 `PERMISSION_DENIED`），版本号必须是 0–100000 的整数（否则 `VALIDATION_ERROR`），类型不认识照样拒。同一道规则在 TS 契约（`packages/contracts/src/schema.ts` 的 anchor 判别联合新增一支）与 Rust 两侧都拦住，生成物已重生成并核对一致（9 commands）。
- **桌面两条命令**：`pub_review` 只读，不带修订号、不进幂等队列；`pub_review_write` 一次一个动作（`add` 或 `resolve`）。审阅者身份不是界面自报的：写侧用与 v2 通道同一份受信本地主体生成作者标签（`user:local-owner`），所以意见带得出"谁说的"。写成功回的是刷新后的整份清单，界面不自己攒列表。
- **定稿闸门落在事务里**：`审阅中 → 已定稿` 时数这本还有几条未解决的审阅意见，非 0 整条拒（`CONFLICT`），消息点名"第 N 版还有 X 条未解决的审阅意见，请先逐条处理或退回草稿"。送审、退回草稿、交付都不受这条闸门影响；同一事务里判定与落库一起成败，不会留下"版本记了、状态没改"。
- **按这本数，不按版本号数**：意见不会因为版面又改了一版就自动失效——绕开闸门的唯一出路是把每条明确标成已处理（或退回草稿）。线程仍按锚点原文复用，所以新版本上的新意见自然另起一条，旧意见仍挂着它当时那一版的号。
- **只管这本**：写侧核对回执线程的锚点，拿照片线程或别本的线程编号来走这条通道整条拒（`PERMISSION_DENIED`），且那次改动不落库。
- **界面**：出版物工作室在"审校状态"下面新增审阅面板——每条意见标着"第几版提的"、作者、原文、处理与否，未处理几条一句话说清；只读视图仍读得到意见，但一个写入口都不给；读不到时明确报错，不把半屏空白当成"都处理完了"。面板在没有这条读通道的仓储上整段不出现（可选方法，与 `editSpec` 同一规矩）。
- **回执守卫**：清单上屏前逐条比对——意见不属于这本、汇总与逐条对不上、同一条线程出现两次、意见版本号比当前还新、状态没见过、线程没有正文，任一命中都不上屏；空白正文与不像编号的编号在出门前就拒。
- **云端不假装支持**：`apps/api` 的评论锚点校验补齐 `publication` 分支并明确拒绝（出版物是本地权威数据，云端没有这本），类型收窄后没有静默走到序列那条分支。
- 文案 13 键 × 4 语言（`copy` 段四套各 647 条，缺失 0、多余 0），并断言英文界面真的走英文、日文不混谚文。

### 验证结果

- Rust：`cargo test --lib` 232 / 0 / 3（上一批 231，本批 +1）；`--all-targets --no-fail-fast` 459 / 0（上一批 458）；`--lib --no-default-features --features native-agent` 237 / 0 / 4（上一批 236）；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；`cargo fmt --all -- --check` 退出 0。上一批记的"测试数没涨比测试失败更早说明问题"这条在本批生效：新增用例跑完后各计数各 +1，确认它真被收进来了。
- 真库那条 `open_review_opinions_are_bound_to_the_book_and_gate_the_finalize_action` 点名的事：意见带着当前版本号与受信作者落库；空正文三种写法与不存在的那本都留不下线程；送审照常通过；定稿被拒之后状态仍是 `in_review`、修订号没涨、历史一条没多；标记已处理后闸门让开、重新打开立刻再关上；**退回草稿改一版版面再送审，同一条意见仍拦着定稿**（意见锚的仍是它那一版，而清单当前版本已经更新）；真处理掉才放行且历史首条是"定稿"；另一本的意见与一条照片意见都不算在这本头上（拿照片线程号走这条通道被 `PERMISSION_DENIED` 且那条线程状态原样未动）。
- 前端：67 个测试文件 / 535 项通过（上一批 66 / 526，本批 +9 = 面板 7、仓储 2）；`npm run lint`、`typecheck`、`contracts:check` 全部退出 0。面板 7 例点名：意见带"第 7 版"与作者上屏且未处理条数说清；没有意见时说的是没有而不是空列表；提交只送正文（首尾空格在出门前去掉）、上屏的是后端刷新回来的那份、输入框清空而旧意见仍在；处理与重开传的是这条线程编号和相反状态；只读视图读得到但写入口一个不给；读失败时报错且不给出"都已处理"的假状态；四套语言文案齐备。仓储 2 例：读侧只发一条 `pub_review`、写侧动作形状原样送达且 `pendingCommands` 仍为空；三种空/超长意见与不像编号的线程号到不了后端，七种坏回执（串本、汇总对不上、线程重复、版本倒挂、陌生状态、无正文、正文全空格）都不上屏。

### 仍需推进

- **审阅记录仍缺"被审版本"那一格的正式落点**：意见锚在版本号上，但 `publication_versions` 里没有可放审阅者身份的列，定稿那次快照仍是"动作 + 版面"。把身份正式写进不可变历史需要一次本地迁移（新列 + 新 CHECK），没有在本批擅自加。
- **多人协作仍是本地骨架**：写通道只有"人类 / 只读"两种上下文，审阅者身份统一是 `user:local-owner`；`v2_memberships` 那套角色没有接进写命令校验，所以"谁能批、谁只能提"还没被强制过。真多人（不同设备/账户）还需要云端评论锚点支持出版物，云端目前没有出版物这张表。
- **意见不进撤销栈、也没有待确认重放**：评论是协作元数据，与画布编辑改动不同，不占人工撤销栈；本地命令在一条事务里提交，崩溃由 SQLite 回滚兜住，但不具备出版物命令通道那套"断链后手动确认"。写失败就是一次失败，需要用户再点一次。
- **两条闸门仍各自独立**：未处理意见拦得住定稿，但印前检查不通过仍拦不住导出；"已定稿"与"可送印"仍未互相触发。
- **只有桌面端有这段界面**：手机端没有出版物界面；网页评审页的评论锚点仍只有照片/序列/版本，看不到出版物线程。
- 面板样式沿用工作室既有排版节奏，未单独写这段 CSS；窄屏下的观感未验。本批未打包、未提交、未推送、未部署；交互只用 jsdom 测过，真机点按未验。工作区仍含其他会话的未提交改动，不计入本批。

## 第二十四批增量：工作包 6 第一环——原图被外部软件就地改写时先报事实，确认后才换记录

现状核对（先读代码再动手）：工作包 6 在这个项目里已经有三块拼图，但没有一块接在这条最常被撞到的现实上——`editor_bridge.rs` 的调色版本已接线，`external_edit.rs` 有函数却没有任何命令注册（休眠），migration 0022 建的 `external_edit_links` / `transfer_jobs` 两张表没人写。而用户在 Lightroom / Capture One 里"保存回原文件"之后，本地库里那条记录的指纹、宽高、文件大小就和磁盘上的对不上了。导入侧原本只有两种结局：指纹不同就当作一张新照片再插一条，指纹相同就跳过；`source_assets.source_hash_verified_at` 这一列从写下的那天起一直是 NULL，等于"核验过"这件事从没发生过。后果是同一张照片在同一个项目里长出第二条记录，而选片、评分、备注、序列位置全留在旧的那条上，新的那条一片空白。所以这一环补的是**检测 + 明确确认**，不是自动替换：先把"文件被就地改写了"和"这是一张新照片"分开说清，落不落库由人点头。

### 实现

- **两种结局变三种**：`ImportOutcome` = 新增 / 重复来源 / 内容已变化。`import_photo` 把原先按 `COUNT(*)` 的判重换成一次按（项目，原图路径）的行查询，同内容优先命中——命中同内容是重复、命中同路径但内容不同是"变化"，两条都不落库。
- **变化既不算新增也不算跳过**：`ImportJob` 新增 `changes`，逐条带回文件名、路径、改动前后的指纹/宽高/大小；计数行改成"新增 N 张 · 重复 N 张 · 内容已变化 N 张 · 失败 N 张"。没人点头之前库里一条不动，也不会静默多出一条空白记录。
- **确认是一条命令，界面递不出路径**：`apply_photo_change` 只收项目、照片、期望修订号；原图路径由 Rust 自己从 `photos.original_path` 读出来再重新解码，前端没有把写操作瞄向任意文件的机会。文件失联回 `ORIGINAL_MISSING`，被移走后指到了别的文件回 `CONFLICT`，内容已经和库里一致回 `CONFLICT`（"无需更新"）。
- **换内容不换身份**：改的是同一行——照片 id、选片状态、评分、备注、序列 `import_index` 全部原样，只换指纹、宽高、大小和两张代理图路径。同一事务里 `projects.revision` 走乐观并发（陈旧回执 `REVISION_CONFLICT`，不落任何改动），并写一条 `content_apply` 内部事件，带改动前后与受信本地主体，版本历史里核对得上。
- **不替别人改写共用的母版**：`resync_asset` 三条分支——项目里已经有这份内容的母版就只把别名改挂过去；旧母版只有这一条在用就地更新指纹/宽高/格式/摘要并**首次写下 `source_hash_verified_at`**；旧母版还有别的照片在用则新建母版与 `original` 版本，收尾统一 `UPDATE asset_aliases … WHERE photo_id=? AND asset_id<>?`（原本没别名的补 INSERT），不留孤儿别名。零新表、零迁移、零新列、零删除。
- **旧内容的 AI 结论自动退场**：`photo_observations` 按（照片，指纹）存，每个读侧都 JOIN `photos.content_sha256`，所以指纹一换，针对旧内容做的分层视觉分析自然读不到——不需要专门写失效代码，也不会把旧分析当成现在这张图的说法。
- **界面内联在导入的两处出口**：`SourceChangeList` 挂在后台导入横幅与导入对话框里（`AppController` 的横幅已带它），一句话说清"库里记的 3000×2000 · 4.0 MB → 文件现在 4000×3000 · 8.0 MB"，逐条一个"更新这条记录"按钮；确认中显示进行中、失败说清原因且不冒充已更新，一条在处理时按住其余按钮，仓储没有这条通道时只报事实不给按钮。
- **回执守卫**：TS 侧 `changes` 是必需字段，id 与两个指纹非空、宽高与大小必须是有限数；**缺 `changes` 字段的旧回执直接报错**，不把它当成"没有变化"而静默吞掉一次外部改写。

### 验证结果

- Rust：`cargo test --lib` 232 / 0 / 3（与上一批持平，本批新增都在集成测试）；`--all-targets --no-fail-fast` **461 / 0**（上一批 459，本批 +2）；`--lib --no-default-features --features native-agent` 237 / 0 / 4；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；`cargo fmt --all -- --check` 退出 0。
- 前端：68 个测试文件 / **543** 项通过（上一批 67 / 535，本批 +8 = 面板 6、仓储 2）；`npm run lint`、`npm run typecheck`、`npm run contracts:check` 全部退出 0；`git diff --check` 干净。
- 真库那条 `an_externally_rewritten_source_is_reported_before_it_lands_in_the_library` 点名的事：把同一个路径的原图就地改写后，回的是"变化"而不是新增，项目里仍只有一条记录；导入任务报 `imported=0 / skipped=0 / changes=1`；拿陈旧修订号确认被 `REVISION_CONFLICT` 拒且指纹没动；正常确认后同一 id、新指纹与新宽高、序列位置和选片/备注原样、两张代理图指向新缓存且文件真在盘上、`content_apply` 事件在；针对旧指纹的观察读不到（JOIN 结果为 0）；母版指纹更新且 `source_hash_verified_at` 不再是空、两条资产表达都指向新指纹、`edit_variants` 仍只有一条；再导入一次判为重复、再确认一次被 `CONFLICT` 拒且修订号没涨；程序从不改写磁盘上的原图。
- 另一条 `updating_one_photos_content_never_rewrites_a_master_another_photo_shares`：两条记录共用一份母版（同内容、不同来源）时，就地改写其中一条不替另一条改历史——另一条的指纹与母版都还是旧的那份，两条母版就此分开，别名总数仍是 2；把第二条也改成同一份新内容时它挂到项目里已有的母版，项目不会长出第三份，`PRAGMA foreign_key_check` 为空。
- 测试真抓到一处逻辑漏洞（不是写完就绿的）：`resync_asset` 的"新建母版"分支原先只建母版、没把别名改挂过去，`assert_ne!` 当场失败；补上收尾的 `UPDATE asset_aliases` + 无别名时补 INSERT 才通过。上一批立的规矩本批继续生效——用 `--test local_core <子串>` 过滤时一开始只跑进一条测试，另一条单独跑才暴露上面这个 bug，计数不涨比失败更早说明问题。

### 仍需推进

- **只做到"检测 + 确认"这一环**：仍然不读 `.xmp` 边车，Lightroom 里的评分/标签/选旗回到本地库这条还没有；`external_edit.rs` 与 `external_edit_links` / `transfer_jobs` 仍未接任何命令，"导出一个可编辑副本 → 在外部软件改完 → 回来对上原记录"这条链没有闭环。下一环打算接 `.xmp` 只读回填。
- **没有"两份都留"的分支**：确认动作是就地换掉这条记录，不保留旧内容的可回退副本（母版只剩新内容，旧内容不再在库里）。真要"外部软件版本 + 本地版本并存"需要新的资产语义和界面选择，本批没擅自加。
- **待确认清单活在导入任务的内存里**：应用重启后这份"内容已变化"名单就没了（库里数据没被改动，重新选一次目录会再列出来）。持久化待确认状态需要新表或新列，本批没有加。
- **界面文案是写在组件里的中文**：`ImportDialog` / `SourceChangeList` 沿用这个组件原有的非本地化写法，没进 `copy` 四套语言，与第二十三批"13 键 × 4 语言"的做法不一致——这是已知欠账，补齐要动这个既有组件的文案体系，不是本批顺路能做完的。
- **只有桌面端，且只在导入时探测**：手机端与网页评审页没有这段；打开项目不主动全盘校验，也没有后台定期复核，改写发生在导入之外时不会被自己发现。
- 本批未打包、未提交、未推送、未部署；交互只用 jsdom 测过，"真的在 Lightroom 里保存回原文件"这条端到端未验；工作区仍含其他会话的未提交改动，不计入本批。

## 第二十五批增量：工作包 6 第二环——外部软件写在原图旁边的判断先当事实列出来，点头才落到本地

现状核对（先读代码再动手）：全仓库搜 `xmp|exiftool|sidecar` 只有 `export.rs` 会**写**一份自有 JSON 边车，没有任何一处读 XMP；`photo_decisions` 的列只有 `status`（undecided/keep/reject）、`user_note`、`updated_at`、`rating`（0002 加，0–5 NOT NULL DEFAULT 0）、`book_mark`（spread/full）。也就是说外部软件里的星级与"选中"旗标目前无处可回，而 `external_edit.rs` 与 `external_edit_links` 仍是休眠代码。本批接的是上一环之后的第二个薄弱点：**摄影师在 Lightroom / Capture One 里的判断如何安全回到本地台账**。

### 实现

- **只读，绝不回写边车**：新模块 `sidecar.rs` 只做一件事——从原图旁边的 XMP 里取出评分、选片旗标、色标签，其余字段（曝光、裁剪、色彩设置）一律不猜、不碰。测试最后一条断言边车文件字节级原样未动。
- **没有新增 XML 依赖**：边车按文本扫，但只在标签内取属性（`<x crs:Rating="4">`）、只在该元素自己的结束标签之间取文本（`<crs:Rating>4</crs:Rating>`），所以备注正文里出现 `crs:Rating="5"` 不会被当成评分。两种写法、以及 `xmp:` / `crs:` / `mp:` / `MicrosoftPhoto:` 四个常见命名空间都认；`.xmp` 的两种常见命名（`名.xmp` 与 `名.扩展名.xmp`）都找。
- **只有能安全落地的才叫"可回填"**：评分必须落在 0–5（255 表示拒绝、-1 表示未评这类写法直接忽略，不当成星级）；旗标只认"选中→keep"这一半，**边车写着没选中绝不映射成淘汰**；两处命名空间评分互相矛盾时 `ambiguous_rating` 报出来、不给可写的值；色标签本地没有列，只如实报出数字，不塞进 `book_mark` 冒充。
- **报事实与落库分开两条命令**：`sidecar_proposals` 只读，不占修订号、不进幂等通道，把"边车说什么 / 台账记什么 / 这条能不能落地"并列出来（文件读放在阻塞线程里，不占界面通道）；`apply_sidecar_judgement` 只收项目、照片、期望修订号，**边车位置由这条记录在册的原图路径推出来，界面递不进任何路径**。
- **落库走同一条事务**：`ensure_local_write` + `UPDATE projects … AND revision=?` 乐观并发（陈旧回执 `REVISION_CONFLICT`，判断一位不动），改 `photo_decisions.rating/status` 与 `updated_at`，若旗标让它变成 keep 且不在序列里就补进序列（与人工点选片同一条路），写一条 `sidecar_apply` 内部事件带前后值与边车路径，`snapshot_tx` 后提交；界面拿的是刷新回来的整份投影。
- **重复确认与失联都不冒充**：已经一致了再点一次回 `CONFLICT`（"本地判断已经和这张照片的边车一致，无需回填"），修订号不涨、不写第二条历史；没有边车回 `SIDECAR_MISSING`；边车超过 2 MiB 不当成一张照片的判断，回 `SIDECAR_TOO_LARGE`；读不到回 `SIDECAR_UNREADABLE`；照片编号不属于这个项目回 `NOT_FOUND`。
- 只加了一条 `EVENT_KIND_LABELS`（`sidecar_apply: '回填外部软件的评分与选片'`，该映射本来就有 `?? event.kind` 兜底），没有动 TS 契约与任何 schema。

### 验证结果

- Rust：`cargo test --lib` **239 / 0 / 3**（上一批 232，本批 +7 全在模块内单元测试）；`--all-targets --no-fail-fast` **469 / 0**（上一批 461，+8 = 7 单元 + 1 集成）；`--lib --no-default-features --features native-agent` **244 / 0 / 4**（上一批 237）；`cargo clippy --all-targets --all-features -- -D warnings` 退出 0；`cargo fmt --all -- --check` 退出 0。
- 前端未受影响：68 个文件 / 543 项通过，`lint`、`typecheck`、`contracts:check`、`git diff --check` 全部退出 0。
- 单元侧点名：属性与元素两种写法都读得到；`xmp:Rating="5"` 与 `crs:Rating="2"` 冲突时报歧义且 `rating` 为空；255/7/99 这类越界值一个都不落成合法判断；正文里的假评分不算数；`名.jpg.xmp` 这种命名也找得到；清单只列可写的事实与歧义项（一致的、没有边车的都不出现），色标签只作字段出现。
- 真库那条 `sidecar_judgements_are_offered_as_facts_and_written_only_on_confirm` 点名的事：台账存的是规范路径（断言到 `canonicalize` 后的路径，边车就从这条路径推）；边车写 4 星 + 选中 + 黄标时清单给出 1 条且 `applies_rating`/`applies_status` 同时成立；陈旧修订号被拒后 `rating/status` 仍是 `0/undecided`；正常确认后 `rating=4`、`status=keep`、`sequence_items` 多出这一条、修订号 +1；再确认一次被 `CONFLICT` 拒且修订号不再涨；删掉边车后清单为空、确认回 `SIDECAR_MISSING`；换成互相矛盾的边雨后仍上报但确认被拒；边车字节未变；`photos` 仍只有 1 条（回填判断不长出新记录）。
- 过程中测试与门禁各抓到一处：`OsString` 不参与 `Display` 格式化（改为 `push(".xmp")` 构造候选路径）；属性扫描的嵌套 `if` 被 clippy `-D warnings` 拒掉（改成布尔 `.then(…).flatten().and_then(…)` 单一表达式）。上一批立的两条规矩继续生效：先跑门禁再看计数、按子串过滤时点名确认新用例真的跑进去了。

### 仍需推进

- **这批只到通道，还没有界面**：命令与守卫都在，但目前没有任何一处 UI 会列出这份待确认清单——用户还看不到"外部软件里 4 星、本地没评"这件事。下一环接一个面板（形状与上一批的 `SourceChangeList` 一致：先报事实、逐条确认），并把 `sidecar_proposals` 挂进照片列表或导入完成处。**在那之前，这条能力对装好包的用户不可用**。
- **只读边车，不读图内 XMP**：评分若被写进 JPEG/CR3 内部的 XMP 包（部分软件的做法）或只存在于 Lightroom 目录数据库里，本批读不到；`.cos` 与目录文件一律没碰。真要覆盖需要解析嵌入包，工作量与边车读取不同量级。
- **色标签与"没选中"无处落地**：本地没有色标签列，也没有"淘汰来自外部"的语义，两者都只报不写。要让它们可回写需要新列（迁移）与明确的品牌语义约定，本批没有擅自加列。
- **一次确认一条**：与上一批同样没有批量按钮；一个几百张的项目里逐条点不现实，但批量回填会一次动很多人工判断，需要先定冲突规则（外部覆盖本地？只在本地未评时填入？），这个决定留给用户。
- **扫描成本随项目规模线性**：`sidecar_proposals` 会逐条照片找文件并读边车（单个上限 2 MiB，跑在阻塞线程）。一万张的项目上这会是一次明显的等待，尚无缓存与增量（例如按边车 mtime 跳过）。
- **本批未打包、未提交、未推送、未部署**；边车内容由测试里手写，真实 Lightroom 导出的 XMP 未在本机跑过一遍；手机端与网页没有这条通道。

## 第二十六批增量：工作包 6 第三环——外部软件的判断回到界面：先按一次才读盘，逐条确认才落库

现状核对：上一批结束时 `sidecar_proposals` / `apply_sidecar_judgement` 两条命令与守卫都在，但全仓库没有任何一处调用——能力对用户不可用。本批只做"接上去"这一件事，不新建第二套界面：形状沿用第二十四批那张确认面板（先报事实、逐条确认、失败不冒充）。

### 实现

- **界面挂在导入对话框里，与"原图被就地改写"同一处**：新增 `SidecarJudgementList`，与 `SourceChangeList` 并列出现在导入完成处。理由是同属"把外部软件的改动带回来"这一件事，用户不必去两个地方。
- **读盘要按一下才发生**：面板打开时只有一个"看看外部软件里的判断"按钮，不自动扫描——上一批记的"扫描成本随项目规模线性"这条欠账没有被界面放大。读与写都走各自的可选通道：仓储没有这两条通道时按钮直接不给（读按钮 disabled），只报事实不谎称已同步。
- **一条一行两句事实并列**：`外部说：4 星 · 已选中 · 色标签 6 → 本地记的：0 星 · 未决`，加一句总说"确认后才会写进这一条记录；不确认什么都不动"。**不能落地的三种情况各说各的原因**：两处评分矛盾说"先在外部软件里统一"；外部写着未选中说"这不等于淘汰，本地不动"；只有色标签说"本地没有地方放，只报事实"。不给这三类按钮，界面也点不动。
- **确认仍然只点名照片**：`applySidecarJudgement(projectId, photoId)`，参数里没有边车路径；成功后该行换成"已回填这条记录"、按钮消失、通知外层刷新（星数与选片在别处立刻同步）；失败时 `role="alert"` 说后端原话，按钮仍可再点。一条在写时按住其余按钮与读按钮，避免同时改两条。
- **回执守卫**：TS 侧新 `proposalSchema`——`photoId`/`sidecarPath` 非空、`sidecarRating` 必须是 0–5 的整数或 null、`colorLabel` 0–8 或 null、`localStatus` 只认 `undecided|keep|reject`、两个 `applies*` 与 `ambiguousRating` 必须是真布尔。**字段整个缺掉（`undefined`）也拒**，不允许后端换形状时被当成"没有评分"。空值走 `null`，不是省字段。
- 没有改 schema、没有改 Rust、没有动契约生成物。

### 验证结果

- 前端：69 个测试文件 / **553** 项通过（上一批 68 / 543，本批 +10 = 面板 8、仓储 2）；`npm run lint`、`typecheck`、`contracts:check`、`git diff --check` 全部退出 0。Rust 侧本批未改动，沿用上一批 469 / 0 的绿灯。
- 面板 8 例点名：不点就不读盘（且读的时候只送项目号）；读到后两句事实与"确认前不动"都上屏、且此时还没有"已回填"；确认只送这一条照片编号并且成功后按钮换成已回填、外层刷新被叫到一次；后端拒掉时 `role="alert"` 是原话且不冒充已回填、按钮还能再点；矛盾那条没有按钮只有"先统一"；未选中那条并列的是"外部说…未选中…→ 本地记的：3 星 · 保留"配"这不等于淘汰"；旁边什么都没有时说的是"没有可以回传的评分或选片旗标"而不是空列表；读失败后还能再读一次；没有通道时读按钮 disabled 且一个写入口都不给。
- 仓储 2 例：读只发一条 `sidecar_proposals` 且参数只有 `projectId`，六种脏回执（空照片号、9 星、字符串星数、陌生状态、字符串布尔、空边车路径）与"字段整个缺掉"都不上屏；写只发一条 `apply_sidecar_judgement`，参数恰好是 `{expectedRevision, photoId, projectId}` 三个键（断言键集合等于此三者，路径进不来）。
- 过程中自己写坏过一次：新增仓储用例时误删了上一条用例的收尾断言与闭合括号，`lint`/`typecheck`/`npm test` 三门同时红在同一个文件（129 行 `'}` expected`）——补回后归绿；另一处是把"本地未决"的默认值当成"保留"断言，测出来的是界面文案与预期不符，改为如实断言默认状态。

### 仍需推进

- **入口仍只有导入对话框一处**：没有在菜单或项目工具栏单开"外部软件判断回流"入口，用户必须先走到导入完成处。要单开入口需要动界面结构与导航，本批没有顺手改。
- **一条一条点，没有一键批量**：批量回填的冲突规则（外部覆盖本地人工判断，还是只在本地未评时填入）仍未定，属用户拍板项；几百张的项目里逐条点不现实。
- **读完不会自动重扫**：面板内已读到的清单是那一次扫描的结果，改完外部软件需要再按一次；没有边车 mtime 缓存或增量扫描（上一批记的成本欠账仍在）。
- **界面文案是写在组件里的中文**：沿用这个组件的非本地化写法，`SidecarJudgementList` 的约十句文案没进 `copy` 四套语言，与第二十四批同属一笔欠账。
- **未做真机验证**：面板只经 jsdom 测过，窄屏观感与 CSS 都没写（沿用 `import-changes` 的排版节奏）；真实 Lightroom 导出的边车未在本机跑过一遍；手机端与网页端没有这段界面。本批未打包、未提交、未推送、未部署。

## 2026-11-05 装机前核对

用户最新推进顺序：先让工作包 6 已完成的三个环节进入可安装版本，再接实 Focal AI 的照片理解与记忆，随后推进颜色链路。七工作包范围仍有效；本轮尚未构建、安装或发布新包。

- 当前开发 HEAD 8be1e435，已安装桌面应用为 1.5.2；发行分支 codex/prompt-language-release-20261104 另有 58909d1e 与 10dc3cde 两个提交，不能直接拿开发树较旧的发行配置覆盖已安装版本。
- 已接回 10dc3cde 中的历史任务展示修复（store/task_scope/对应回归），保留当前工作树其他修改。定向回归 historical_grants_remain_visible_after_prompt_or_contract_updates：1 passed / 0 failed，242 filtered out。日志 /tmp/focalroom-install-preflight-20261105.log。
- 提示词与多语言观察的发行增量 58909d1e 尚需逐项与当前 WIP 合并核对，不能因单个历史展示修复通过就宣称发行基线已完整合入。
- 磁盘仅约 1.5–1.6 GiB 可用。已定位 src-tauri/target/debug/incremental 约 7.3 GiB 可再生构建缓存；未清理。用户在本聊天给出的 AGENTS.md 要求删除目录先取得明确确认，故须确认后才能清理并继续构建。
- 色彩目标是对受支持 ICC/显示/输出条件做有依据的一致性验证，不能承诺不同屏幕和印刷条件下绝对同色。

## 2026-11-05 本地安装包准备结果

- 用户明确确认后，已仅清理 src-tauri/target/debug/incremental，构建前可用空间恢复约 6.9 GiB。未清理其他缓存或业务数据。
- 使用三方比较接回发行分支 58909d1e 的作品集按需方法、上下文压缩及多语言规则，同时保留当前观察版本/记忆等 WIP；提示版本统一为 m3-openai-13 / focal-2.5。10dc3cde 的历史任务修复已接回。未执行 Git 合并提交或推送。
- 合并后 300 图三草案曾因上下文过大暂停：归档布局回执仍携带冗余坐标。现在只缩减发送给模型的归档副本，保留节点 ID、照片 ID、数量与 callId 明细回查入口；权威回执和数据库不变。新增断言验证三份完整回执各有 20 个节点及坐标。新增断言初版误用列 name，按实际 schema 改为 tool_name，复验通过。
- 本轮验证：Rust 默认库测试 241 passed / 0 failed / 3 ignored；前端 69 文件 / 553 passed；Lint、类型、契约、默认库严格 Clippy、Rust 格式及 diff 空白检查通过。未重跑全部集成目标或真实模型。
- 本地候选包：/Users/yihua/Documents/film-builds/FocalRoom-1.5.3-local-20261105/FocalRoom.app；ZIP 同目录 FocalRoom-1.5.3-local.zip，11,067,602 bytes，SHA-256 e34ca26be460517bc25cdbf12d50d79a676f90847ce53688fee59aa848117272。
- 构建使用外部配置覆盖为 1.5.3，未改仓库版本号；454 项构建输入指纹前后核对一致。包含当前未提交代码，不将 8be1e435 单独当作完整包来源；完整来源见 source-manifest.json 与 source-state.json。
- Developer ID 签名与团队 KXSFCZF4S3 核验通过，标识 studio.betweenframes.desktop 与已安装版一致。本地包未做 Apple 公证、未公开发布，未安装/启动。
- Recovery 子目录已备份当前 1.5.2 应用及用户第 37 版项目库；SQLite backup 完整性检查 ok，未改写原库。私有数据库备份不放入应用 ZIP。
- 首次打开新包会从迁移 37 升级到 38（photo_observations 增加可空 preview_sha256；旧记录不回填）；Store 在升级前另做校验备份。按用户本聊天 AGENTS.md，实际用户库 schema 升级需单独明确确认；当前未执行。

## 2026-11-05 安装与用户库升级完成

用户明确确认安装并升级项目库后，已将本地 1.5.3 安装到 /Applications/FocalRoom.app 并启动。数据库由迁移 37 升至 38，preview_sha256 列存在；完整性 ok，外键违规 0。升级前后六张核心业务表全部行摘要一致（3 项目、2189 照片及判断、32 序列项、5 画板、26 节点）。已通过原生 UI 看到既有项目与照片网格；未发起模型请求或回流确认，未改正式作品。没有重新哈希全部原图，不扩大验证结论。

旧应用和安装前即时库备份保留在 /Users/yihua/Documents/film-builds/FocalRoom-1.5.3-local-20261105/Recovery。安装结果与前后校验位于同目录 INSTALL-RESULT.md、install-before.json、install-after.json、artifact.json。本地包未公证、未公开发布；源码仍有 WIP，未提交或推送。

## 2026-11-05 Focal AI：历史视觉与记忆正文的版本可信度

继续工作包 1，先核对源代码，定位两个确实存在的缺口。本批不新增数据库迁移、模型请求、依赖或独立评测平台。

### 照片证据

- 草案 evaluate_draft 原先只按原图内容摘要匹配观察缓存。现在使用已记录的 preview_sha256 核对本轮压缩预览字节，只把 verified 观察用于逐张事实与相邻相似判断；mismatch、unrecorded、unreadable 分别列入 omitted_visual_evidence，不通过重新上传补证据。
- 历史修订观察追加可选 previewSha256 / pixelVersionAtDelegation，原图匹配仍由 matchedAtDelegation 表达，不混淆两种身份。新字段缺失时序列化不追加空字段，保留旧冻结上下文的摘要兼容性。
- GUI 历史投影追加 currentPixelVersion，仅原图与预览均匹配才返回缩略图；不匹配仍保留历史事实/解释与原调用来源，明确标为历史。共享 HistoricalObservations 组件、TS 类型及 IPC schema 一起接线，拒绝未知状态和缺指纹却自称 verified 的回执。
- 复用有界本地压缩函数，放入阻塞线程核对，不调用模型。核对发生在读取时，不保证之后被外部软件再次改写的预览仍保持一致；大范围评估的磁盘扫描性能还需后续测量。
- 回归覆盖：实际预览相同、原图不变而预览替换、旧观察没指纹、预览失联；评估覆盖与界面缩略图必须同步变更；旧冻结结构反序列化后重序列化保持原样。

### 创作记忆

- model_attempt 的 memoryReads 原先只记 ID 与来源项目，正文改写后历史投影会展示当前文字。新读取回执追加 statementSha256，不复制正文；工具查询回执可从当时已经冻结的 statement 取得同一指纹。
- memory_reads 只在来源项目和正文指纹都匹配时展示原文，额外给出 memoryTextVersion（verified / changed / unrecorded / source_unavailable）。当前状态/停用信息保留；不会改写已有读取或查询回执。
- 老读取没有正文指纹时不倒填，也不把现在的文字当作当时依据；来源仍可见。桌面提供正文改写与版本未记录的独立说明，中英日韩文案齐全。
- IPC 守卫拒绝未知正文状态，也拒绝 changed/unrecorded/source_unavailable 却携带当前正文的组合。指纹仅用于本地核对，不加入模型上下文。
- 作者记忆测试扩展同 ID 正文改写/恢复，以及无版本历史回执的投影；既有跨项目、停用、纠正、来源移动与不可变回执测试继续运行。

本批代码尚未重新打包或安装；现有 /Applications/FocalRoom.app 仍为此前冻结来源的 1.5.3，不把源码修改冒充已装机。真实模型质量、完整项目概览与按轮记忆展开仍未完成。

本批最终验证：
- 默认核心库 243 passed / 0 failed / 3 ignored。
- 视觉集成 agent_vision 52 passed / 0 failed；作者记忆 author_memory 12 passed / 0 failed。
- 共享原生核心 248 passed / 0 failed / 4 ignored。
- 前端 70 文件 / 557 passed；Lint、类型、契约、Rust 格式、全目标/全特性严格 Clippy、桌面前端构建及 diff 空白检查通过。
- 日志：/tmp/focalroom-ai-final-rust-20261105.log、/tmp/focalroom-ai-final-native-20261105.log、/tmp/focalroom-ai-final-clippy-20261105.log、/tmp/focalroom-ai-evidence-ui-final-20261105.log、/tmp/focalroom-ai-final-desktop-build-20261105.log。
- 本批未提交、推送、部署或重新安装；安装包构建指纹仍指向安装时冻结来源，不随源码改动更新。

## 2026-11-05 Focal AI：授权范围内的视觉概览与分层观察计划

### 代码交付

- 在已有 read_creative_context 的 section 枚举中增加 visual_overview，复用原工具授权与幂等回执，不增加新的顶层工具、观察 kind 或数据库表。前文关于“项目概览必须新增 kind 和迁移”的判断以此实现替代：本批是根据已有记录派生覆盖情况，不把派生概览当作模型完成的新观察。
- 每次最多读取 500 张本次授权照片的元数据，当前页至多核对 30 张，返回 nextOffset。缓存限定原图摘要、观察型号、服务端点摘要和观察提示版本；正文有 64 KiB 上限，非法正文不当作视觉事实。
- 分开提供整个授权范围的缓存记录数与当前页的像素核对统计。无缓存、版本不符、旧版未记录、文件不可读和非法记录分别标记；未分页复核的区域明确为未核对，不提供虚构的全项目已观察比例。
- 均匀抽样覆盖授权范围首尾和中段，最多 12 张，明确不是审美选片排名；当前页待补看候选最多 12 张。
- 旧版未记录指纹的缓存单列，避免反复调用会复用同一旧缓存的 inspect_photos 却声称完成了新观察；不可读预览单列并要求先恢复文件。
- 重点候选可用 inspect_photos 读取匹配缓存的完整观察，最多列 4 张；明确仍是既有预览规格，并非高分辨率精查。
- 比较候选只来自正式序列中真正相邻且在当前页核对通过的两张，不把过滤后隔开的照片冒充原序列邻居。没有可核对相邻对时不编造候选。
- 当前页重复事实按完整原文及不同照片计数，展示时有界摘录；重复描述不代表画面相似或叙事关系。相同截断前缀的不同事实不会被误合并，同一照片重复写一句话不会被算作多个来源。
- 提示词接通“概览 → 分批观察 → 候选复看 → 序列比较”，但不强制上传、不自动执行。输出仍受原照片范围、图像许可和预算限制。提示版本 m3-openai-14；创作上下文扩展 2.3。旧任务执行仍须通过冻结授权与当前工具契约校验，历史展示沿用此前兼容修复。
- 桌面参考来源补“项目视觉概览”中英日韩名称，不显示内部 section 键。

### 验证范围与限制

新增 5 项共享核心回归，覆盖分页范围与隐藏照片隔离、各类像素状态、更新缓存后复核、仅真实序列邻居可比较、型号/端点绑定、空项目、截断前缀与单照片重复描述，以及真实 AgentEngine 工具循环在无视觉授权时如实停留于元数据、零上传、零新视觉观察。测试序列夹具第一次因照片未设为 keep 被数据库触发器拒绝，按真实选片约束修正夹具后通过，未放松触发器。

本批没有用真实摄影项目证明模型会找到高质量创作方向；全量项目视觉索引、真正高分辨率精查、按轮记忆 UI 仍待推进。未改用户项目库、未调用生产模型，未提交/推送/部署或更新已安装的 1.5.3。

本批验证结果：
- 最终默认核心库 248 passed / 0 failed / 3 ignored，包含 5 项新增概览回归及 300 图三草案既有回归。
- 视觉集成 52 passed；共享原生核心全库在最后一项边界测试前为 252 passed / 4 ignored，最终源码下原生概览定向 5 passed。
- 前端 70 文件 / 558 passed；Lint、类型、契约、Rust 格式、全目标全特性严格 Clippy、桌面前端构建及 diff 空白检查通过。
- 日志：/tmp/focalroom-overview-lib-final-20261105.log、/tmp/focalroom-overview-rust-final-20261105.log、/tmp/focalroom-overview-native-focused-20261105.log、/tmp/focalroom-overview-clippy-final-20261105.log、/tmp/focalroom-overview-ui-20261105.log、/tmp/focalroom-overview-desktop-build-20261105.log。

## 2026-11-05 Focal AI：明确许可的重点照片精查

### 已实现的行为

- 桌面增加默认关闭的“允许所选照片精查（1–4 张）”。换项目、账户或照片选区后清除许可，成功发送后重置；答复旧任务不能追加。托管服务未声明精查能力时禁用，发送前重新核对。
- iPhone 在现有单次照片发送确认页中提供同等选择及中英日韩说明。仅支持已声明能力的服务；不勾选时仍走普通观察。旧版账户响应没有新字段时仍能解码，不推断已支持精查。
- StartConversation 可显式携带 detailPhotoIds，任务授权、原始许可步骤和工具清单同事务冻结。最多四个唯一 ID，必须在本任务照片范围内；每次 inspect_photo_detail 只接受其中一张。模型上下文列出真实许可 ID。篡改任务授权不能绕过不可变的原始许可，失败委托不会留下半个任务。
- 普通观察保持 512px/q75；精查只读派生预览，使用最多 1400px/q90 JPEG 并请求 high detail。不读取或发送原图，不放大小预览，不提供局部裁切或全分辨率承诺。沿用输入 16MiB、解码 1400px/32MiB 边界，派生 JPEG 另限 2MiB。
- 网关原先会统一降为 low，本批增加 visionDetail 能力声明与 preview1400-high-v1 独立许可头。高细节须同时具备普通图片发送许可；服务端再次核验尺寸、重编码去元数据，单次最多四张高细节预览。普通旧请求行为保持 low。高细节许可也绑定请求正文摘要，不能把原编号的低细节结果冒充精查结果。
- 精查在共享核心和网关均采用更高的输入预留，仍受既有图像、费用、轮数和等待上限约束，不增加默认预算。新网关能力失效时在图片预留和发送前停止；本地 BYOK 请求直接使用已准备正文，避免重建正文丢失 high 设置。
- 精查采用独立缓存版本 m3-vision-detail1400-q90-v1@端点摘要，与普通缓存隔离；沿用已有 inspect_photos 数据库 kind，无新迁移或依赖。历史/草案依据核对按当时压缩规格计算像素指纹，精查也投影到本轮已观察照片记录。
- 托管未知结果沿用原编号只读查询与明确继续，恢复后只落一次观察、不重传。已经发送的结果核对不依赖当前服务是否仍开放精查。
- 概览可返回许可内的精查候选及工具；提示版本 m3-openai-15，创作上下文扩展 2.4。普通缓存复看不能表述为精查完成。
- 300 图三草案回归暴露上下文略超既有上限：归档布局回执改为保留画布/调用身份、节点数量和全文回查入口，省略成批 ID/坐标。完整权威回执不改，模型调用与结果仍成对归档；未放宽预算或测试断言。

### 本地验证及边界

- 默认核心库 251 passed / 0 failed / 3 ignored，含 300 图三草案及精查许可、回滚、篡改、压缩/指纹隔离测试。
- 共享原生核心 256 passed / 0 failed / 4 ignored；Rust 格式与全目标/全特性严格 Clippy 通过，diff 空白检查通过。
- 视觉集成 53 passed，托管 HTTP 集成 23 passed；生成色块与回环假模型覆盖 512/1400 实际发送尺寸、high 设置、普通/精查缓存隔离、跨轮缓存零重传、服务能力失效与 UNKNOWN 重启恢复。
- 前端 70 文件 / 561 passed；桌面精查默认关、换选区清许可、发送后重置及服务能力失效均有测试。
- 独立临时 PostgreSQL：托管服务 8、异步 8、队列 15，合计 31 passed。覆盖独立授权头、保留 high、去 EXIF、图片上限、较高预留及未知请求不重放。
- Swift Package：77 项 XCTest，10 skipped，0 failed；另有 3 项 Swift Testing 通过。覆盖旧账户响应兼容及新能力解码。iPhone 模拟器目标编译通过，不等同于真机安装或 UI 人工验收。
- 前端 Lint、类型、契约及桌面前端构建通过；构建仍有既有分包提示。
- 日志统一在 /tmp/focalroom-detail-*-20261105.log；本批未调用生产模型、未部署 API、未提交/推送、未升级用户项目库或重新安装。/Applications/FocalRoom.app 仍是此前冻结来源的 1.5.3；新托管精查须后续部署网关并打包客户端才能使用。

第一阶段专业 AI 仍未整体完成。下一步接实“照片判断 → 可见事实与不确定性 → 摄影方法来源”的解释链，再补按轮记忆依据展示；真实模型摄影质量仍需独立验收。

## 2026-11-05 Focal AI：单张判断的观察与方法依据链

### 代码行为

- 新增 explain_photo_judgment 创作工具，复用不可变 tool_calls 回执，不新增数据库迁移、依赖或正式作品写入通道。普通旧任务不自动取得新工具；提示版本 m3-openai-16，创作上下文扩展 2.5。
- 每条判断引用本任务实际成功取得的 inspect_photos / compare_photos / inspect_photo_detail 回执、照片 ID 和 0 起始事实索引。后端复制原始观察事实，不接收模型另写的“事实引文”；照片须在当前任务范围内，原图记录摘要和当前预览像素须核对一致。元数据回执、缺失记录、越界事实、其他任务的回执及像素不一致均拒绝。
- 方法引用同时指定本轮检索 call_id、知识编号和可选断言索引。本地原创方法只能来自 search_photography_knowledge 的原回执；云端摄影师、摄影书或系列资料只能引用实际取得的对应断言，复用既有实体解析。没有出处可传空数组，不能凭知识编号直接从全库补造已读记录。
- 判断、推理说明、引用事实、观察时的不确定性及本条解释的限制分别保存。inference 固定为 true；回执绑定只证明来路，不认证视觉观察、推理或审美正确。原始回执的像素核对状态标明为 imageStatusAtJudgment，避免重放时冒充现在仍一致。
- 方法保留来源、机构、作者、授权、版本、审校状态和争议标记；缺少外部出处不填造 URL。imported/disputed 不变成 verified；带详情获取失败的记录不能当作已取得的断言。方法正文超过 600 字符时仅摘录，并明确标记 excerpt，全文仍在原检索回执中。
- 桌面及 iPhone 增加可展开的“单张判断与依据”：分别展示模型观察、推理、方法来路与限制。明确标注为 AI 解释；无方法来源时直接说明。中英日韩文案齐全，旧任务不含新字段仍兼容。
- 历史投影每次重新核对预览，像素或原图记录改变时保留当时事实并标记版本未能核对一致，不把当前缩略图附到旧判断上。每任务最多展示最近 20 条成功判断，读取单份回执不超过 64KiB；失败尝试不占成功判断的展示额度。
- 该工具只在具体评价需要时按提示使用，不强制所有对话产生判断卡片，也不把自由回复文字冒充已经绑定的依据。未扩充专业知识语料，不提供画面坐标或局部标注；模型是否采用工具、引用是否充分及摄影判断质量仍需真实模型验收。

### 本地验证

- 默认 Rust 核心 255 passed / 0 failed / 3 ignored；保留既有 300 图三草案与原图摘要回归，不放宽默认预算。
- 共享原生核心 260 passed / 0 failed / 4 ignored；视觉集成 54 passed，托管 HTTP 集成 23 passed。Rust 格式、全目标/全特性严格 Clippy 和 diff 空白检查通过。
- 新增 4 项核心测试覆盖真实事实/来源复制、幂等与正式作品不变、伪造编号/索引/重复事实、跨任务与元数据拒绝、像素改变后的历史标记，以及原创/未核实/争议/空来源的区分。
- 本地真实 AgentEngine → HTTP 假模型 → 视觉观察 → 方法检索 → 判断工具 → 任务投影通过；生成色块，不计真实摄影质量。初次夹具用未命中的关键词却假定必有知识，已改用现有语料确有的检索词，未修改检索器伪造命中。
- 前端 71 文件 / 565 passed；新增 4 项判断组件及 IPC 守卫测试。Lint、类型、契约和桌面前端构建通过，保留既有分包提示。
- Swift Package 78 项 XCTest（10 skipped、0 failed）及 3 项 Swift Testing 通过；新增旧任务兼容、历史版本/争议/无来源解码及未知像素状态拒绝。iPhone 模拟器目标编译通过，尚未真机安装或人工 UI 验收。
- 日志：/tmp/focalroom-judgment-*-20261105.log。本批未运行生产模型，未修改用户项目库，未提交/推送/部署或重新安装；当前已安装 1.5.3 不含本批源码。

下一步继续第一阶段的“本轮实际用了哪些记忆、来自哪次确认、为什么现在适用”展示；专业色彩仍按第二阶段顺序推进，不把本次依据连接当作全部专业 AI 已完成。

## 2026-11-06 Focal AI：按轮记忆来路、确认与当前状态

### 已实现的行为

- 桌面及 iPhone 增加“请求中的记忆来路”。按请求尝试与记忆查询分别展示，保留轮次、时间、来源项目、获得途径、当时状态、检索原因、确认回执及当前状态；旧版任务不含新字段时仍兼容。
- 严格区分可证明的事实：model_attempt 发生在请求发送前，只能证明准备携带；查询成功只能证明返回了条目，不证明后来交给模型，更不证明模型采纳或影响了判断。界面明确说明这一边界，不显示虚构的“记忆实际作用比例”。
- 新请求与查询在原事务内冻结记忆 ID、来源项目、正文指纹、当时状态、排序原因及最多 100 个证据编号。记录人工确认的 requestId、动作与时间，来源只认 changed=1 的 ConfirmHypothesis，或与已保存显式偏好证据匹配的 RememberPreference；不用 updated_at 充当确认时间。
- 排序原因直接来自当时检索路径：关键词/本地语义参与排序、已确认优先、证据优先，或已授权的本地作者记忆排序。语义参与排序不等于语义匹配成立，没有相关性排序时不伪装成匹配。原因只说明为何提供，不能证明此刻适用。
- 记忆查询新增独立 memory_query_trace 步骤，绑定调用编号与轮次；轨迹正文不进入模型协议，不因回执重放再写一条。检索结果仅新增排序原因字段。旧 tool 步骤仅在没有对应轨迹时作为兼容来源，避免重复展示。
- 逐条、逐次核对正文版本。同一记忆 ID 的旧无指纹读取不会被后来查询、当前正文或当前确认补齐；旧查询已有冻结正文时可用该正文计算历史指纹。正文改写、来源移动/不可用时隐藏当前正文；当时确认及状态保持原样，当前停用、移除和证据排除情况单独展示。
- 证据展示只报告当时记录编号的当前现存/排除/缺失数量，不展开原始照片或备注。跨项目记录保留本地授权来源和数量，不增加来源项目的照片、备注或适用说明正文读取范围。本项目明确保存偏好的适用说明仅在正文版本核对通过且原证据仍可读取时展示，不视为当前适用性认证。
- 每任务展示最近 40 条事件，每条最多 10 项记忆；更早事件和证据截断均有提示。单份历史记录最多读取 64KiB，超过或缺字段时显示记录不完整，不当作“没有提供记忆”。证据编号紧凑存储，避免重复项目编号使正常新记录超过上限；相同读取在一次投影中复用解析，双端折叠时不构造全部明细视图。
- 复用既有 agent_steps、memory_commands 与偏好证据，不增加迁移、工具权限或依赖，不复制偏好正文进新轨迹，不修改旧审计记录。旧汇总接口保留兼容；有新轨迹时桌面不重复展示旧记忆汇总。提示版本 m3-openai-17。

### 验证

- 默认 Rust 核心 256 passed / 0 failed / 3 ignored；共享原生核心 261 passed / 0 failed / 4 ignored。既有 300 图三草案回归继续通过，预算未放宽。
- 作者记忆集成 15 passed，新增 3 项按轮轨迹回归：真实 AgentEngine 的请求/两类查询与确认绑定，停用和证据排除后的历史保持，旧读取与当前版本/后来确认隔离，来源移动，以及 40 条/64KiB 边界。既有显式保存偏好测试补 RememberPreference 确认身份断言；新增紧凑证据编号上限单元测试。
- 视觉集成 54 passed，托管 HTTP 集成 23 passed；使用临时库与回环假模型，不调用生产。
- 前端 72 文件 / 570 passed；新增 4 项时间线/IPC 守卫测试及 1 项面板连接测试，覆盖请求与查询区分、当前与当时状态、缺失记录、版本正文、确认与跨项目范围守卫，以及旧汇总不重复展示。
- Swift Package 79 项 XCTest（10 skipped、0 failed）与 3 项 Swift Testing 通过；新增按轮记录、旧缺失字段、确认动作解码与伪造动作拒绝测试。iPhone 模拟器目标编译通过。
- Lint、类型、契约、Rust 格式、全目标/全特性严格 Clippy、桌面前端构建及 diff 空白检查通过；前端构建仍有既有分包提示。
- 主要日志：/tmp/focalroom-memory-trace-*-20261106.log；开始于午夜前的 Swift 与定向日志在 /tmp/focalroom-memory-trace-*-20261105.log。
- 未提交/推送/部署、未重新安装、未修改用户项目库、未调用生产模型。当前已安装 1.5.3 仍不包含后续源码；未执行真机或真人摄影质量验收。

概览、精查、单张判断依据和按轮记忆来路的本地工程路径已连接；模型摄影专业性与知识覆盖的验收仍开。下一步按第二阶段优先级，先核对专业色彩现有显示、预览和导出链路的真实行为，再补代码缺口。

## 2026-11-06 专业色彩第一批：显式检视从原图取像素

### 本轮核对的真实路径

| 路径 | 代码实况与限制 |
|---|---|
| 桌面普通缩略图/预览 | image_pipeline 的普通解码转成 8-bit 后保存 JPEG、丢弃 ICC；大图经过 ImageIO 的 DeviceRGB 位图。旧缓存不能统一假定仍是源配置的数值。本轮不改写这些缓存，也不据此宣称图库已完成色彩管理。 |
| 原有“色彩管理”适合屏幕 | 读取旧预览，再按原图 ICC 变换；缓存有可能已经被平台转换，来源判断不可靠。本轮改为核验原图后重新生成，完全不取旧缓存像素。 |
| 原有色彩管理 100% | image::open 后先降到 RGB8，未应用 EXIF 方向，透明通道直接丢弃；缓存键未含渲染算法版本。本轮与适合屏幕共用新路径。 |
| iPhone 浏览/AI 预览 | LocalLibrary、VisionPreview 已显式绘入 sRGB CGContext 并输出 8-bit JPEG；本轮未改变这条显示链，也未证明真机显示与桌面同色。 |
| 高精度原生变换 | RGB8/16/32f CMM 原型及数值测试早已存在；并非印刷母版、HDR 显示映射或完整瓦片工作流已经接通。 |
| 出版物交付 | 本地布局、印前信息与作品清单已有；worklist CSV 不是带 ICC 的印厂像素包。本轮不修改出版物导出。 |

### 本轮代码

- get_color_managed_preview 通过 photo_inspection 的在册项目、原图指纹、格式与大小门禁，从原图生成最长边 1400px 的显式色彩预览。source_not_original 不拿 RAW 嵌入/派生预览冒充可核验原片；不修改普通预览、原图或项目库。
- 适合屏幕与全分辨率共用 color_engine::render_source_jpeg：有界读取原图，先检查实际尺寸与类型，应用 EXIF 方向，保留 16-bit 中间值执行源 RGB ICC → 目标 RGB ICC 变换，透明区域在目标空间合成白底，到显示 JPEG 编码时才降至 8-bit。不得称为 16-bit 输出或印刷母版。
- 沿用 64MiB 文件、3200 万像素限制；解码器分配限制 256MiB 不等于进程总峰值内存。全分辨率路径仍会解码整图，并未实现区域瓦片或低内存保证。小预览不放大；浮点/HDR 与非 RGB 源配置明确停止，不静默压缩范围后宣称成功。
- 实际用于解码的整份字节指纹必须匹配登记版本，快照内 ICC 必须与已选源 ICC 一致；渲染后再核对文件与数据库当前源版本/项目状态。失败时不发布新缓存文件；已完成的缓存只对应经验证的输入快照。
- 100% 渲染缓存加入 managed-v2 算法版本，仍绑定原图摘要和目标 ICC 摘要，避免沿用旧方向/透明处理结果。旧缓存保留，不执行清理、迁移或静默替换正式数据。
- 大 ICC 按 JPEG ICC 的序号与总数分成多个 APP2 段写入，上限 255 段；每段配置载荷最多 65519 字节。输出完整保留实际目标 ICC，系统仍可将它映射到窗口实际所在的显示器，不把两次有明确来源的映射简单说成错误的“重复管理”。实现依据为 [ICC 嵌入技术说明](https://archive.color.org/files/technotes/ICC-Technote-ProfileEmbedding.pdf) 与 [ICC 规范 JPEG 嵌入章节](https://www.color.org/ICC1V42old.pdf)。
- ICC 扫描区分“确认未嵌入”与“已出现但分段/元数据不完整”。不完整 ICC 不再退化成无配置 sRGB；无法可靠核验时不给变换像素。ICC 结构可读仍不等于 CMM 接受或色彩准确，失败保持显式状态。
- IPC 追加 pixelSource=original，界面区分“从原图生成的受限预览”与全分辨率结果。成功状态必须带有效形状的 JPEG/文件路径，失败状态不能带渲染像素；矛盾回执不给接管画面，显示可读错误，不把 schema 内部错误甩给用户。
- 色彩请求按项目、照片和预览版本隔离；切片/换版本后晚到结果不覆盖当前照片。切换色彩模式时立即隐藏上一模式的 100% 像素，等待新模式结果。四语言说明明确目标目前是主显示器 ICC，输出仍为 8-bit JPEG。
- Apple 对显式/设备色彩空间的接口定义参见 [CGColorSpace 文档](https://developer.apple.com/documentation/coregraphics/cgcolorspace)；本轮数值检查使用本机 Apple sRGB/Display P3 配置，不以此替代真实屏幕校准、跨屏验证或印刷签样。

### 验证与下一步

- 默认 Rust 核心 263 passed / 0 failed / 3 ignored；共享原生核心 268 passed / 0 failed / 4 ignored。Rust 格式、全目标/全特性严格 Clippy 通过。
- 新增 7 项核心回归覆盖多段 ICC 完整回读且像素不变、P3→sRGB 只转换一次、非 RGB 与浮点源拒绝、不完整 ICC 拒绝、EXIF 旋转/透明白底、实际源摘要与尺寸门禁，以及真实 Store 中旧缓存被改色仍只使用原图。旧“无配置”夹具只有 4 字节 JPEG 头，已替换成完整无 ICC JPEG，未放松读取门槛。
- 前端 73 文件 / 574 passed。新增 4 项视口/请求与 IPC 回归，覆盖模式切换立即失效、同 ID 换版本失效、晚到结果隔离及矛盾回执拒绝；灯箱夹具改用 JPEG 形式回执，保留原交互断言。
- Lint、类型、契约、桌面前端构建、diff 空白检查通过；前端构建保留既有分包提示。iPhone 模拟器目标编译通过，仅验证共享核心兼容，未新增手机色彩功能。
- 日志：/tmp/focalroom-color-*-20261106.log。未安装/部署、未提交/推送、未修改用户项目库或真实原片；已安装 1.5.3 不含本批改动。未做真机、测色仪或印厂验收。

下一步先统一普通图库及 AI 预览的像素色彩身份和缓存版本：不能给既有未知像素直接补 ICC，也不能让新旧缓存混用。之后继续高位深导出/印刷母版、分级瓦片预览、窗口所在显示器及校准变化跟随；专业色彩第二阶段仍未整体完成。

## 2026-11-06 专业色彩第二批：图库缓存写出时就说明自己的色彩身份

### 本轮核对的真实路径

| 路径 | 代码实况与限制 |
|---|---|
| 小图标准解码 | image::open 后按数值缩放，源 ICC 被丢弃后另存 JPEG；缓存里那份 P3/sRGB 数值就此失去身份，读侧只能猜。本轮改为写出时带上源自己的配置，像素数值不变。缩放仍在编码后的非线性数值上做，属于既有近似，本轮未改。 |
| 大图缩放解码 | 上一批用 ImageIO 到位图上下文的目标空间，当时接的是 DeviceRGB——等于"当时那块主屏"。不同屏幕导入会产生不同数值且都不标注；换显示器后重新导入，同一张照片颜色就对不上。本轮改为显式 sRGB 并标注 sRGB；系统拿不到 sRGB 时直接失败回退标准解码，不降级到设备空间。 |
| 平台解码（HEIC/RAW 等经 sips 转 PNG） | 说不清这一步做了什么映射，因此缓存保持不标注，也不补一个看起来合理的 ICC。 |
| 分级瓦片 | write_tile_jpeg 直接编 JPEG，既不应用 EXIF 方向也不带配置。本轮按同一规矩标注源配置；方向、区域解码和缓存版本仍未接。 |
| 显式检视（上一批） | 从核验后的原图渲染，不读缓存像素，因此本轮改动不影响它；它也不替缓存背书。 |
| 既有缓存文件 | 已存在的 `{hash}-320.jpg`/`-1400.jpg` 一律不重写，本轮只改新写入的那批。旧库里的错标不会因此自动变好，也不会在被重新生成前被悄悄改动。 |

### 本轮代码

- color_engine 增加两条判定：source_profile_bytes_for_cache 只在源确实嵌入、结构可读、且色彩空间标记就是 RGB 时返回那份配置；灰度、CMYK 或读不懂的一律返回 None。srgb_cache_profile_bytes 取系统 sRGB 配置供缩放解码标注。原则是标签必须等于这条路实际做到的映射，猜不出就留白，不替缓存编一个身份。
- image_pipeline::prepare 按分支选择标签：平台解码与"两张缓存都已存在"不写；缩放解码标 sRGB；标准解码照抄源配置。缩略图与预览共用同一次解码结果，标签必然同一份。
- save_cache 把 JPEG 先编到内存、按需追加 ICC，再 tmp+rename 落盘。写入失败不产出半成品缓存。
- tile_pyramid::write_tile_jpeg 同样改为标注源配置后原子写，替换原先固定质量直写文件的分支。
- 不新增迁移、不改数据库列、不动 photo_observations.preview_sha256 的证据绑定：既有缓存字节不变，所以历史观察摘要仍然对得上；缓存被重新生成时摘要自然不再匹配，按既有规矩回落到"未核验"，不会被静默复用。

### 验证与下一步

- 默认 Rust 核心 269 passed / 0 failed / 3 ignored；native-agent 特性核心 274 passed / 0 failed / 4 ignored。Rust 格式与严格 Clippy 通过。
- 新增 6 项回归：无嵌入配置、结构不完整、非 RGB 三种情况都不给缓存编身份；3600 宽 P3 原图的缓存只声明 sRGB 且明确不等于 P3；800 宽 P3 原图的缓存原样带上 P3 且中心像素仍为源数值；无配置源留白不补；同一照片重复写缓存两次像素不变、只换声明且无 .tmp 残留；瓦片带源配置、无配置源不带。
- 集成契约有意改动一项：photo_inspection 里"预览必须不含 ICC"的旧断言作废，改为"标准解码的预览与缩略图必须原样声明源配置且两份一致"，原图字节仍逐字节校验未被改写。该套件 10 passed / 0 failed / 2 ignored。
- 全目标套件 504 passed / 1 failed（上一批为 503/2）：剩余 1 项是 agent_evaluate 的相邻相似分组用例（tests/agent_evaluate.rs:201，"相邻高相似应命中：[]"），起因是另一会话在 agent/tools.rs 未提交的 preview_sha256 核验门槛，而该测试夹具插入的观察行不含该列。属评测区在飞改动，本轮不改他人代码，仅如实记录。
- 前端本轮未改动；未安装/部署、未提交/推送、未修改用户项目库或真实原片；已安装 1.5.3 不含这两批色彩改动。未做真机、测色仪或跨屏对色。

下一步顺序：先给 AI 预览与上传像素定同一套身份规矩（否则模型看到的仍是无声明数值），再做分级瓦片的方向与区域解码及缓存版本，然后接高位深导出/印刷母版，最后才是窗口所在显示器与校准变化跟随。普通图库缓存的存量迁移仍未做：本轮只保证新写入的缓存可核对。

## 2026-11-06 专业色彩第三批：交给模型的像素也落到确定身份

### 本轮核对的真实路径

| 路径 | 代码实况与限制 |
|---|---|
| 桌面 AI 观察上传 | 读磁盘预览 → 内存缩放 → JPEG 重编码后 base64 交给模型。上一批之前预览从不带配置，这一批之后新预览会带；但重编码仍把标签丢掉，于是一张带 P3 声明的预览交出去时变成"无标签 = 被当成 sRGB"，模型看到的是没管理过的广色域数值，观察指纹还把这个错误固化成可复用的缓存。本轮修这条。 |
| 精查（detail）上传 | 与普通观察同一个函数，只是尺寸与质量不同，因此一并生效。逐字核对与分图判断等复用方也自动跟上。 |
| 手机 AI 预览 | 共享核心把已核验的浏览 JPEG 画进显式 sRGB 上下文再输出 8-bit JPEG，输出不带 ICC 但按约定就是 sRGB——像素身份与实际做的事一致。本轮只核对，未改这行代码；把标签显式写进输出仍属后续。 |
| 观察证据绑定 | 预览指纹算在最终字节上，因此本轮改动只在"预览确实带可核对声明"时改变指纹。存量库里的预览没有声明，历史观察不会被无声判废。 |

### 本轮代码

- color_engine 新增 declared_rgb_profile（从已读进内存的字节里取可核对的 RGB 声明，复用同一套"结构可读且标记是 RGB"判定，不再各写一遍）与 normalize_upload_pixels：拿到声明就在 8-bit 上做一次源→sRGB 变换并交出 sRGB 标签；源本来就是 sRGB 时只声明不再管第二次；拿不到声明时原样返回 None，像素一个不动。
- 非 macOS 平台没有可信的原生变换，因此保留数值、继续声明源自己的配置，绝不只换一个看起来对的标签。
- 上传压缩在编码前完成身份落地，标签写完才量体积、才算指纹；顺序保证"发出去的就是指纹描述的那份字节"。磁盘上的预览仍然一个字节都不改。
- 系统给不出 sRGB 配置时直接报错（COLOR_SRGB_UNAVAILABLE），不静默退回设备空间；结构完整但引擎认不出的配置报错（COLOR_PROFILE_INVALID），不产出"看起来已管理"的上传件。

### 验证与下一步

- 默认 Rust 核心 275 passed / 0 failed / 3 ignored；native-agent 特性核心 280 passed / 0 failed / 4 ignored；全目标套件 510 passed / 1 failed。Rust 格式与严格 Clippy 通过，格式检查只重排本轮改的两个文件。
- 唯一失败仍是上一批记录的那项：agent_evaluate 的相邻相似分组用例，起因是另一会话在 agent/tools.rs 未提交的 preview_sha256 核验门槛与测试夹具不匹配，与本轮色彩改动无关，不改他人代码。
- 新增 5 项回归：身份核对不出时不动像素也不给标签；已经是 sRGB 的源不做第二次管理；P3 声明的源确实变换过一次并声明 sRGB；引擎加载不了的配置如实报错、不留半成品；内存声明判定只接受可核对的 RGB（无声明与非 RGB 都拒绝）。
- 上传边界另加 1 项端到端回归：同一份数值分别按 P3 声明、sRGB 声明、无声明上传，交出去的字节、标签与画面三者关系符合上述规矩，且指纹等于含标签的最终字节，磁盘预览保持不变。
- 前端与手机本轮未改动；未安装/部署、未提交/推送、未修改用户项目库或真实原片；已安装 1.5.3 不含这三批色彩改动。未做真机对色，也未向真实模型发图验证（仅本地字节与标签核对）。

下一步顺序：①把手机 AI 预览输出的 sRGB 身份显式写进文件（现在靠约定，不是靠标签）；②普通图库缓存的存量迁移或版本化，否则老库里的无声明预览会长期停在"留白"这一档；③分级瓦片的方向与区域解码；④高位深导出与印刷母版；⑤窗口所在显示器与校准变化跟随。专业色彩仍未整体完成，本批只代表"新导入照片到模型"这段闭环。

## 2026-11-06 专业色彩第四批：分级瓦片按真实方向与透明规则出图，且能被回收

### 本轮核对的真实路径

| 路径 | 代码实况与限制 |
|---|---|
| 画布取一块瓦片 | `canvas_photo_tile` 按 (源SHA, 算法版本, 层级, x, y) 在预览缓存目录找文件，没有就现场生成。本轮之前生成函数自己开 `image::open` 全量解码：不应用 EXIF 方向（image 0.25 的 `from_decoder` 不会自动转，必须显式 `apply_orientation`），透明像素走 `to_rgb8` 直接丢 alpha。结果是竖幅手机照片在画布上躺倒，PNG 抠图边缘是一块黑或一块刺色。 |
| 取瓦片时的内存 | 每取一块 256×256 都先 `build_levels` 构造整棵金字塔并全部驻留（约原图 RGBA × 4/3），只为读其中一小块。 |
| 瓦片的色彩声明 | 上一批已接入：沿用源 ICC，查不到依据就留白。本轮把它改走的统一解码入口核对了一遍，声明规则与新入口不冲突（新回归覆盖"带声明的源"和"无声明的源"两支）。 |
| 瓦片文件的回收 | `sweep_orphans` 原先只认 `{hash}-320/-1400.jpg` 与 `managed-*`，`v{版本}.{SHA}.{层}.{x}.{y}.jpg` 一律不认识：看过一次大图留下的几十上百个小文件既不进配额统计，也永远不删。 |

### 本轮代码

- 瓦片改走与导入同一个解码入口：应用 EXIF 方向，并受同一套 256MiB 分配与 30000px 像素上限约束；透明像素与图库缓存同一条规矩合成白底（半透明按 alpha 混合，不是丢通道）。
- `PYRAMID_VERSION` 1→2：错方向的 v1 瓦片不可能再被正确命中，缓存键换号让其自然失效。回收侧区分两种情形——版本不是当前这版的属于确定垃圾，额度宽松时也直接清掉；当前版本的按照片是否仍在库决定算引用还是算孤儿，只有超配额才按最旧删。构建崩溃残留的 `v2.{SHA}.{层}.{x}.{y}.{随机号}.tmp` 与渲染临时件共用"刚写出的不动"这一小时规则，避免和仍在进行的任务抢文件。
- 新增 `level_pixels`：只沿到目标层级所需的那一段 2× 缩放链走，逐级释放上一级，不再为读一小块把整棵金字塔同时留在内存里；某一边已缩到 1、缩不下去时如实报 `VALIDATION_ERROR`，不把上一层数据冒充成这一层返回。
- 命名识别只认本应用的形状：SHA 必须是 64 位十六进制、层/坐标必须是数字、结尾只能是 `.jpg` 或 `.{随机号}.tmp`，否则一律不认领，用户自己丢进缓存目录的文件不动。
- 原图仍是一个字节都不写；前端、手机、数据库本轮未改动。

### 验证与下一步

- 默认 Rust 核心 280 passed / 0 failed / 3 ignored（上批 275）；native-agent 特性核心 285 passed / 0 failed / 4 ignored（上批 280）；全目标套件 515 passed / 1 failed / 20 ignored（上批 510/1）。严格 Clippy 与 `cargo fmt --all -- --check` 通过。
- 唯一失败仍是前两批记录的那项：agent_evaluate 的重复/相似分组用例，起因是另一会话在 agent/tools.rs 未提交的 preview_sha256 核验门槛与测试夹具不匹配，与本轮无关，不改他人代码。
- 新增 5 项回归：①瓦片跟随 EXIF 方向——尺寸、画面上下内容、以及"按存储宽度才存在的第 3 列瓦片现在如实越界"三者一起断言，并用同一份不带方向的像素反证报错来自方向而非尺寸门禁；②透明与半透明像素合成白底；③`level_pixels` 与完整金字塔逐级一致且到尽头报错、扁平图不冒充结果；④瓦片命名认领规则（非十六进制 SHA、错扩展名、错段数都拒绝）；⑤配额回收三种情形分开断言，并确认同一片瓦片不会被清两次。既有 `canvas_photo_tile` 项目隔离与缓存命中用例继续通过。
- 仍没做到：①瓦片仍是"整张原图解码后取块"，真正的区域解码（只解目标层级那块像素）没做——45MP 原图取最粗一级仍要一次全量解码；②瓦片不做显示器色彩管理，只如实声明源配置，与 100% 检视那条按显示器渲染的路不同；③本轮只到代码与自动化验证，未打包、未装到本机、未提交/推送，已安装的 1.5.3 不含这四批色彩改动；④未做真机对色，也未测超大图库下瓦片并发请求的真实峰值内存。

下一步顺序：①分级瓦片的区域解码（接 ImageIO 目标长边，去掉整图解码）；②手机 AI 预览把 sRGB 身份显式写进文件；③高位深导出与印刷母版；④窗口所在显示器与校准变化跟随。存量图库缓存的处置见下一节方案，未批准前不动数据。

## 2026-11-06 存量图库缓存的色彩身份：方案（只出方案，未动任何数据）

### 先说清现在是什么状态

- 上三批改的是"写缓存的那一刻"：新导入的照片，预览、缩略图、瓦片、发给模型的字节都带上可核对的声明。
- 之前导入的照片，磁盘上那些 `{hash}-1400.jpg` / `{hash}-320.jpg` 里没有任何声明。读侧现在按"查不到依据"处理——不管理，也不假装管理，所以它们的表现和改动前一模一样。
- 因此这不是数据坏了，也不是显示错了，只是新照片享受到了、老照片停在原地。影响面是可以用一条只读查询数出来的（多少张、多少字节、其中多少张的原图其实带配置）。

### 三条路，代价各不相同

| 做法 | 你会看到什么 | 代价与风险 |
|---|---|---|
| ① 不动（现在的状态） | 老照片继续按数值直出；带广色域配置的看起来偏艳。 | 零写入、零风险；历史 AI 观察全部保持"可核对"，不会因此多花钱。 |
| ② 懒升级（用到哪张补哪张，不批量） | 某张老照片下次被打开检视/被画布用到/被再次问到时，发现缓存没身份而原图有配置，就把那一份缓存重写（先写新再换名，像素数值不变，只补声明）。 | 库里会长期混两种状态；每张被重写后，它的历史观察指纹会变一次，也就是在"参考来源"里从可核对变成对不上，下次问 AI 会重发一次真实图片（花一次模型费用）。需要显式开关，默认关。 |
| ③ 批量迁移（一次点亮全库） | 一个明确入口：先只读预检报数量/字节/预计耗时，确认后分批后台跑，批间可停可续，原图绝不写。 | 一次写几百 MB 到几 GB 缓存；所有带配置照片的历史观察统一变成"对不上"，之后每次问到都可能重发图。若想让老证据继续算数，需要新增一张"旧指纹↔新指纹"映射表——那是数据库改动，要单独批准，本批不做。 |

### 建议的下一步（等你点头，不擅自开始）

1. 先只做只读预检：把"你的库里到底有多少张停在无声明、其中多少张原图其实带配置"变成一个看得见的数字，不动任何文件。
2. 看完数字再选 ②/③。若选 ②，要同时决定"指纹变了算不算证据作废"的呈现口径（只提示，还是加映射表）。
3. 在拿到你的批准之前，这三条都不落地，代码维持"只保证新写入可核对"。

## 2026-11-06 专业色彩第五批：画布瓦片只解它要的那一档，网格口径与前端对齐

### 这批改的两件独立的事

1. **层级尺寸口径对齐。** 画布前端算"这一级有几列几行"用的是逐级向上取整折半（513→257→129），后端算的是向下取整并且直接丢掉对不齐的最外一行一列。两边不一致时，前端认为自己该存在的最后一格瓦片，后端回一句"坐标越界"；前端的处理是整节点退回糊掉的缩略图，用户看到的就是"放大也不清晰"，而且没有任何报错。这条错只发生在奇数尺寸上，所以小图和大图都可能踩到。
2. **粗档位不再整幅全分辨率解码。** 取最粗几档时，先把原图按 4000×2500 那样全分辨率解出来（一张 RGBA 就 40MB 起），再逐级折半，只为拿一小块。现在改成：只读文件头（含 EXIF 方向）算出目标档位的边长，让 ImageIO 边解码边缩到那一档，像素只为这一档付一次。

### 真实路径（本轮之后）

| 情形 | 取像素的方式 | 写进文件的声明 |
|---|---|---|
| 层级 > 0、源图长边超过缩放门槛（2800）、扩展名 jpg/jpeg/png/tif/tiff | ImageIO 边解码边缩到该档长边 → 反前置 → 透明合成白底 → 按网格精确重采一次 | 系统 sRGB：这条路径确实把像素映射进了 sRGB，声明跟着路径走 |
| 层级 0（放大到 100% 那几块） | 标准整图解码，应用 EXIF 方向，受同一套分配/像素上限门禁 | 源图自带配置；没有就留白 |
| 小图（≤ 门槛）、HEIC/RAW、非 macOS、缩放解码失败 | 标准整图解码 + 逐级折半链（只保留当前一级，不留整棵金字塔） | 同上：源配置或留白 |

- 门禁只有一份：头尺寸检查与整图解码调同一个 `decode_limits`（256MiB 单次分配、单边 30000px）。超限的原图在两条路上得到同样的拒绝，不会出现"一条拦一条放行"。
- 层级尺寸改为向上取整后，`PYRAMID_VERSION` 升到 3。旧版（v2）网格算出来的瓦片可能对不齐新格子，缓存键里带版本就永远命中不到；配额扫描认得瓦片命名，遇到过期版本直接删，不会因为额度没满而永久留在盘上。
- 边缘像素的处理：奇数尺寸最外那一行/列按实际存在的像素平均，不再整条丢掉。

### 验证

- 默认 Rust 核心 284 passed / 0 failed / 3 ignored（上批 280）；native-agent 特性核心 289 passed / 0 failed / 4 ignored（上批 285）；全目标套件 519 passed / 1 failed / 20 ignored（上批 515/1）。严格 Clippy 与 `cargo fmt --all -- --check` 通过。
- 唯一失败仍是前几批记录的那项：agent_evaluate 的重复/相似分组用例，起因是另一会话在 agent/tools.rs 未提交的 preview_sha256 核验门槛与测试夹具不匹配，与本轮无关，不改他人代码。
- 新增 4 项回归：①`level_dims` 的闭式（除以 2^level 向上进位）与逐级折半链在 999×801 这种非整除尺寸上逐级一致，且前端 `ceil(边长/256)` 算出的最后一格必须非空；②513×513 的最后一列像素真的能取到（旧口径这一格直接越界）；③同一张 4000×2500 的 Display P3 源图，层级 0 的瓦片声明仍是 P3、层级 1 的瓦片声明是系统 sRGB——这条同时是"粗档确实走了缩放解码"的唯一可核对证据，两条路若塌成一条，断言就会失败；顺带断言 2000×1250 的小图粗档不被谎称成 sRGB；④竖幅存储 + 方向 6 的 2500×4000 大图取粗档：瓦片按转正后的 2000×1250 网格出（未转正时这一格根本不存在），画面上下颜色也确实是转过来的。
- 更新 4 项既有断言到新口径：3×3 降采样现在给 2×2、3×1 边缘平均按实际存在的像素数取、512×1 的层级 1 现在是真实存在的 256×1（改由 1×1 承担"到头如实报错"的断言）、缓存键版本字面量与回收用例改用 `PYRAMID_VERSION`。既有方向、透明、幂等、项目隔离用例全部继续通过。

### 仍没做到

1. **层级 0 仍是整幅全分辨率解码。** 放大到 100% 那几块瓦片要的是原始像素，JPEG 的块级局部取像（按 DCT 系数只解那块区域）没接；粗档省下的内存不覆盖这条最深的路径。
2. **HEIC/RAW 完全没进缩放解码这条路**（按扩展名挡掉），它们仍只能整图取像；非 macOS 也没有变化。
3. **未测真实峰值内存。** 结论来自代码路径的改变与断言，不是 Instruments 上的曲线；原图字节仍要一次性读进内存，超大 JPEG 文件本身（几十 MB）没省掉。
4. **只到代码与自动化验证**：未打包、未装到本机、未提交/推送，已安装的 1.5.3 不含这五批色彩改动。前端一行没改，也不需要改——是后端原来对不齐它的口径。

下一步顺序：①手机 AI 预览把 sRGB 身份显式写进文件；②高位深导出与印刷母版；③窗口所在显示器与校准变化跟随；④（可选，需单独批准）层级 0 的块级局部取像。存量图库缓存的处置仍按上一节方案，未批准前不动数据。

## 2026-11-06 专业色彩第六批：手机交给模型的图片把 sRGB 身份写进字节

### 起点：一个只存在于假设里的声明

手机端把照片交给模型前，`VisionPreview.make` 会重生成一份受限像素：ImageIO 缩到长边 1400、在 sRGB 位图上下文里白底重绘、按 0.75 质量重编码。像素确实落进了 sRGB，界面也直接写着"sRGB JPEG"——但字节里什么都没有。实测（同一台机器、同一套 SDK）：ImageIO 重编码出的 JPEG 只有 JFIF/Exif/Photoshop 段，**没有 APP2 ICC_PROFILE 段**；而 `CGImageSourceCopyPropertiesAtIndex` 报出的 `sRGB IEC61966-2.1` 是读侧对无标签 JPEG 的默认假定，不是文件里的事实。也就是说身份写在文案里、写在管线意图里，唯独没写在交出去的那份字节上。桌面端上一轮已经改成"标签先写完再量体积、算指纹"，这一批把手机对齐到同一条规矩。

### 本轮代码

- `apps/mobile-ios/Sources/FocalRoomCore/VisionPreview.swift`：重编码后按 JPEG 的 ICC 约定把系统 sRGB 配置写成 APP2 段，紧接 SOI，单段上限 65519、序号与总数各占一字节——与桌面端 `color_engine::tag_jpeg_with_icc` 完全同一种形状。**体积门禁移到写完标签之后**，量的是真正交出去的那份字节；标签拿不到时（系统给不出 sRGB 配置字节）原样交出、如实留白，不因为一个标签拦下用户的发送。
- 没动的地方：`LocalLibrary.render` 写出的本机预览缓存同样只有 JFIF 没有标签，但它的字节参与 `previewSHA256`，改一次等于给所有新导入照片换一套指纹，且迁移语义要单独定；本批不碰，列为待拍板项。
- 没动的地方：`uploadPayload`（迁云上传）只在超过 8MiB 时才重编码，那条路本来就不是交给模型的像素。

### 验证

- Swift 侧基线与改后同口径：`swift build` 通过；`swift test` 由 82 executed / 10 skipped / 0 failed 变成 86 executed / 10 skipped / 0 failed，新增 4 项全部通过，既有的 VisionPreview 尺寸上限与 GPS/EXIF 剥离用例继续通过（说明补段没有把来源元数据带回去）。
- 新增用例断言的是四件事：①交出去的字节里 APP2 ICC_PROFILE 段存在、序号总数为 1/1、段内配置与系统 sRGB 配置逐字节相等，且配置第 16–20 字节确实是 `RGB `；②写完标签仍是一张能被 ImageIO 正常读回、尺寸不变的 JPEG；③切段规则按 65519 分片（用 65529 字节的合成配置断言分成 2 段、序号 1/2、两段拼回去等于原配置、原有字节一个不少地紧跟在段表之后）；④不是 JPEG、或配置为空时原样返回，不塞空段也不假定身份。
- 第三方解析器独立核对（Pillow 12.1.1，走真实模块产物）：标签段的配置字节与系统 sRGB 完全相等；同一份文件删掉该段后，两种解读出的**像素逐点相同**——写标签只动段表不动像素这条规矩被外部工具确认；无标签参照读不出 `icc_profile`，带标签的读得出。

### 仍没做到

1. **没有真机/真接口端到端确认。** APP2 放在 SOI 之后、JFIF 之前是 ImageIO 与 Adobe 的通行做法，且桌面端上一批已在同一形状上跑过自动化，但"托管接口那头的模型能正常读带这份标签的图"只有本地解析器与 ImageIO 的证据，真实调用需要单独批准。
2. **手机本机预览缓存（`LocalLibrary.render` 的 2048px JPEG）仍不带标签**，理由同上——它牵动 `previewSHA256`，属于要拍板的迁移，不是顺手改。
3. **没做 iOS 设备编译**：改动只落在 SwiftPM 目标 `FocalRoomCore`，公开 API 签名未变，`copyICCData` 的可用性已从 iPhoneOS SDK 头文件核对（iOS 10+，包下限 iOS 17）。完整 xcodebuild 未跑，真机未装机；工作树里还有另一会话未提交的 iOS 改动，此刻不适合出包。
4. **未提交、未推送。**

下一步顺序：①高位深导出与印刷母版；②窗口所在显示器与校准变化跟随；③（需批准）存量图库缓存处置；④（需批准）层级 0 的块级局部取像。

## 2026-11-06 Focal AI 第四批：判断依据链的拒绝要说清是哪一环断了

### 起点：拒绝理由本身不可行动

`explain_photo_judgment` 的依据链有七八种断法——照片不在本次范围、引用的回执根本不存在、回执是一次失败调用、回执只有文件元数据没看过画面、观察里没有这张照片、事实索引越界、知识编号是编的、云端摘要条目没有断言正文、断言详情没取得、像素版本对不上。改之前它们全部折叠成同一个 `EVIDENCE_UNAVAILABLE` 与同一句话。

折叠的代价不是"话说得不够细"，而是三条实际后果：失败原因会原样写进不可变回执、写进任务时间线那一步的摘要（`execution.rs` 取的是 `error.message`）、并回给模型作为下一轮的观察，所以模型看到的只是"依据未取得"，只能把同一次调用重试一遍，继续吃工具调用与时长预算；用户端只看到"没有判断卡片"，不知道缺的是图像授权还是知识详情；而回执存在但超过单份 64KiB 读取上限这一种，被报成"本任务没有这条回执"，指向了一个根本不存在的下一步。

### 本轮代码

- 每个断点获得自己的编号（`JUDGMENT_ARGS`/`SCOPE`/`RECEIPT_MISSING`/`RECEIPT_TOO_LARGE`/`RECEIPT_FAILED`/`RECEIPT_NOT_VISION`/`METADATA_ONLY`/`NOT_OBSERVED`/`OBSERVATION_INVALID`/`PROVENANCE_MISSING`/`ORIGINAL_CHANGED`/`PREVIEW_CHANGED`/`PIXEL_UNRECORDED`/`PREVIEW_UNREADABLE`/`PHOTO_MISSING`/`FACT_INDEX`/`RECEIPT_NOT_KNOWLEDGE`/`CITATION_UNKNOWN`/`CLOUD_SUMMARY_ONLY`/`METHOD_EMPTY`/`DETAIL_UNAVAILABLE`/`CLAIM_INDEX`/`RECEIPT_SHAPE`），说法只讲这一环缺什么、下一步该调哪个工具。同一条读取失败在"观察依据"与"方法依据"两条来路上给不同的话，因为修法不同。
- 回执读取拆成存在性与大小两步（都是主键查找）：先看这条编号在不在本任务，再看它是否超过单份读取上限；超限不再冒充"没有回执"，并明确告知重复引用同一条不会成功，要改用范围更小的检索或观察。64KiB 这个上限本身没动。
- 越界类拒绝给出那次回执里的真实条数（"共 N 条，序号从 0 到 N-1"），数字取自回执内容而不是模型传入的参数。
- 云端摘要条目与"编号根本不存在"分开：前者提示要先用 `search_photographers`/`get_photobook_structure`/`compare_photography_series` 取详情，后者说明不能凭资料名称编编号。
- 像素这一环新增内部 `Pixel` 口径，把"原图内容被改写""只有本地派生预览的字节变了""旧缓存从未记录预览指纹""现在读不出预览""照片记录已不在本项目"分开说。给界面两侧的 `imageStatus` 仍由这里折叠为已知的 `verified|mismatch|unrecorded|unreadable`（外加 `unavailable`），没有新增客户端枚举值——桌面与手机的 zod/Swift 校验对未知值是直接拒绝的。
- 消息里不回填模型提供的任何字符串，不带文件路径。
- 一条门禁都没放松：照片范围、`vision_observed` 模式、结构校验、像素核对、断言只能来自本轮实际回执、单份回执上限、每任务最近 20 条，全部照旧；失败尝试仍不占成功判断的展示额度。
- 没动提示词文件、工具契约、数据库迁移与依赖，因此提示版本、观察缓存键、已给出的工具授权都不变；本轮只改拒绝的编号与说法。

### 验证

- 默认 Rust 核心 287 passed / 0 failed / 3 ignored（较基线 +3 项）；保留 `native-agent` 292 passed / 0 failed / 4 ignored；全目标 522 passed / 1 failed / 20 ignored，唯一失败仍是另一会话未提交的 `structured_evaluation_reports_duplicates_similarity_and_metrics`（`preview_sha256` 门禁，与本批无关，未触碰）。rustfmt 与 `clippy --all-targets --all-features -D warnings` 退出 0。
- 判断模块 7 项全绿，其中把原有用例从"只断言失败"升级为"断言编号"：范围外/编号不存在/事实越界/`[0,0]` 被严格 schema 拦下/编号编造/引用不存在/跨任务回执/只有元数据，各自对应自己的编号。
- 新增 3 项：方法侧（云端摘要、非知识回执、失败回执、空正文、断言越界带条数、详情未取得、未补取断言、编号不存在）；观察侧（非观察回执、失败回执、超过读取上限、这批没观察、观察结构不合法、没有像素来路、旧缓存未记录指纹，以及历史投影仍展示为 `unrecorded`）；像素侧（预览字节变了→不说不重导原图、预览读不出→不说不一致、原图被改写→另说，且投影状态仍为 `mismatch`）。
- 前端与 Swift 本轮未跑：改动只落在 Rust agent 内部的拒绝文案，客户端对工具错误编号是 `z.string()` 而非枚举，`imageStatus` 取值集合未变。
- 日志：/tmp/focalroom-judgment-lib.log、-native.log、-all.log、-clippy.log、-test-2.log。本批未调用生产模型，未改用户项目库，未提交/推送/部署，也未重新打包安装到本机。

### 仍没做到

1. **模型是否真的因此少重试，没有实测。** 依据链的拒绝会原样回给模型，编号与说法都进了它的观察，但"改了说法之后重试次数下降"需要跑真实模型才能确认，那需要单独批准。
2. **提示词还没教这条规矩**（引用云端摘要要先取详情、越界时按回执条数填序号）——本轮只在被拒时告知。是否值得动提示版本（牵动缓存键与授权口径）留待拍板。
3. `visual_overview` 里"只挑一对相邻照片比较"用的 `LIMIT 1` 没有 `ORDER BY`，同一项目两次运行可能挑到不同一对；本轮记录了口径，没有顺手改。
4. 全项目视觉索引覆盖率、以及分层视觉分析的真实摄影质量验收，仍在第一阶段的待办里。

下一步顺序：①（需批准）真实模型跑一轮判断依据链，看重试与拒绝分布；②把引用口径写进提示词或工具说明的取舍拍板；③第一阶段剩余的分层视觉分析覆盖面。

## 2026-11-07 Focal AI 第五批：传过去的与看过的分开记账，部分漏答不再谎报"已看过"

### 起点

上一批让判断依据链能拒绝"引用一张其实没被观察过的照片"，但上游回执自己还在把两件事混成一个数字：

- 视觉工具只报 `uploaded_images` 与推算出来的 `cache_hits`，命中数用的是"观察条数减上传数"，模型漏答一张就会被算成一次缓存命中——看着像"没花钱也已经看过了"。
- 时间线上那行字按请求条数写"视觉观察了 N 张"，N 里包含传过去却没被回答的照片。
- 只在模型一张都没答时才报错（`MODEL_UNAVAILABLE`，"视觉观察未引用任何请求照片"），部分漏答静默通过，谁都看不出哪几张没被看过。

### 本轮代码

- 观察预取结构新增漏答名单 `dropped`（带 `#[serde(default)]`，上一版写下的回执可直接解析）：响应回来后按"上传集合减去被采纳的观察"算出真正漏答的那几张。三处构造各自如实：全部命中缓存时为空；网络请求发出之前写好的崩溃恢复回执为空（那时还不知道会漏答谁，这个角落记录在下面）；结算后的数据带真实名单。
- 工具回执新增 `observed_photo_ids`、`unobserved_photo_ids` 与一句 `coverageNotice`，明确只有前者实际读过画面；缓存命中改成按真实来源算，即"观察条数减去（上传数减去漏答数）"；`uploaded_images` 保持真实传输数，不因漏答而少报——隐私与配额语义照旧按字节确实离开本机扣减。漏答的照片不写观察缓存行，下一次仍需重新观察。
- 工具摘要在有漏答时追加"另有 N 张传过去了但模型没有给出观察，不算看过"，观察张数口径从"请求了几张"改为"拿到几条观察"；无授权只做元数据对照的那句仍按请求数。

### 验证

- 新增集成用例 `partially_observed_batches_separate_what_was_sent_from_what_was_seen`（真实本地 HTTP 模拟 Responses API、结构化输出走同一校验）。任务一请求 2 张、两张都出了本机、模型只答第一张：回执 `uploaded_images=2`、`cache_hits=0`、观察名单只有第一张、未观察名单只有第二张、第二张零条缓存记录、授权用量按真实传输扣到 2。同一台模拟服务同一项目再跑任务二请求 3 张：第一张命中上一轮缓存、后两张上传、模型又漏答最后一张——`uploaded_images=2`、`cache_hits=1`（旧口径给 0，等于把省下的一次上传抹掉、把丢的那张当成命中）、未观察名单恰是第三张、第三张仍无缓存记录、累计用量 4、图像请求 2 次。两条任务的时间线摘要都含"另有 1 张传过去了但模型没有给出观察，不算看过"，都不含"视觉观察了 3 张"；投影里两个任务的云图用量都是 2。
- 夹具补两种可编排形态：让模型回答列表少答最后一张，以及一次请求三张照片。这样才能让"缓存命中"和"部分漏答"同时出现——缓存键含端点哈希，两台模拟服务永远不共享缓存，单靠多加一台服务器做不出这个组合。
- 观察模块新增 2 项单元测试：上一版程序写下的回执（没有漏答字段）能正常解析且不会凭空多出漏答名单；漏答名单能过序列化往返。
- 门禁：默认核心 289 passed / 0 failed / 3 ignored（较上批 +2）；保留 `native-agent` 294 passed / 0 failed / 4 ignored（+2）；全目标 525 passed / 1 failed / 20 ignored（+3），唯一失败仍是另一会话未提交的 `structured_evaluation_reports_duplicates_similarity_and_metrics`（预览字节门禁，与本批无关，未触碰）。rustfmt 与 `clippy --all-targets --all-features -D warnings` 退出 0。日志：/tmp/focalroom-1e-build.log、-test.log、-test2.log、-lib.log、-native.log、-all.log、-clippy.log。本批未调用生产模型，未改用户项目库，未提交/推送/部署，也未重新打包安装到本机。
- 前端与 Swift 本轮未跑：只新增回执字段与摘要文案，客户端可见的 `image_status` 取值集合不变；回执正文在客户端不做严格 schema 校验（上一批同形状加键已双端绿灯）。

### 仍没做到

1. **崩溃恢复的重放路径退回旧算法**：发送前写好的那份回执里没有漏答名单（响应还没回来，写不出来），进程崩溃后恢复这一次观察时缓存命中数会偏保守。不会把没看过的说成看过，但那个角落没修。
2. **提示词还没教这条规矩**（拿到部分观察应当补传缺的那几张，且不为未观察照片写单张判断依据）——本轮只在回执与摘要里说明。动提示版本牵动缓存键，且第一阶段已有 300 张画布的上下文预算只剩几百字节余量，先跑预算再改。
3. **真实模型会不会部分漏答、比例多少没有实测**，本轮用夹具证明口径；这需要在批准后再排。
4. 一次批量里单张预览读不出仍会中断整批、`evaluate_draft` 取历史观察时不区分层级与提示版本、无指纹旧缓存行按"未记录"沿用——这几条同属分层视觉分析，本轮没顺手改。

下一步顺序：①（需批准）真实模型跑一批部分漏答，看重试分布与覆盖率；②把"部分观察要补传"写进提示词或工具说明的取舍拍板（先跑上下文预算）；③一批中单张预览不可读时只降级那一张。

## 2026-11-07 Focal AI 第六批：一张坏预览不再拖垮整批观察，额度只为真正出得去的字节预留

### 起点

上一批管的是模型那一侧的缺口（传过去了没被回答）；本地这一侧更靠前的一步仍然粗糙：

- 观察预取先按"还没看过的那几张"整体做额度校验与授权，然后才逐张压缩本地预览。任意一张读不出来（预览被清理、被写坏、格式不认）第一个错误就直接上抛，整批观察在发出任何字节之前失败，其余完好照片陪着白跑一次任务。
- 预留与授权用的是"还差几张"，不是"实际能送出几张"：一张坏预览会让它自己那份图像额度先被占住。
- 上一批的 `unobserved_photo_ids` 算法是"请求集合减去拿到观察"，于是"根本没发出去"的那张也被塞进"发出去没人看"这份名单——同一张照片同时被算成两种缺口，时间线摘要会写成"另有 1 张传过去了…、1 张本地预览读不出来"，而请求里只有 2 张。

### 本轮代码

- 逐张压缩挪到额度校验与授权之前，把没命中缓存的照片分成"读得出字节"与"读不出来"两组：只有前者参与 `vision_limits`、`authorize`、`reserve_vision` 和成本上限推算；后者只进新名单，不占额度、不发请求、不写观察缓存行。
- 一张都没读出来时维持原有口径：不发出空批次，把读文件那一步的第一个错误原样上抛（文件不在报"照片预览不可读"，格式不认报解码错误），任务仍按失败记录，`inspect_photos` 一行回执都不留，免得下一次把它当成"这批已经看过了"。
- 观察预取结构新增 `unreadable`（带 `#[serde(default)]`，上一版写下的回执照常解析）。与上一批的漏答名单不同，这份缺口在请求发出之前就已经知道，因此发送前就写好的崩溃恢复回执把它照实带上——进程崩溃后重放这一次观察，仍然说得出哪几张根本没出去。
- 工具回执把两份缺口拆成互不相交：`unobserved_photo_ids` 现在只等于模型漏答集合，`unreadable_photo_ids` 是本地读不出来的；`observed + unobserved + unreadable` 正好覆盖 `requested_photo_ids`，一张不重不漏。缓存命中仍按"观察条数减去（上传数减去漏答数）"回推，未送出的照片不参与这一项。`coverageNotice` 补一句：本地读不出来的那些既没占额度也没被看过，要判断它得先修好本地缓存再单独观察。
- 时间线摘要把两类缺口并列成同一句"另有 …，不算看过"，只出现一类时也只说那一类；上一批混排导致的"同一张被数两次"随之消失。

### 验证

- 新增集成用例 `one_unreadable_preview_degrades_to_that_photo_instead_of_the_whole_batch`（真实本地模拟 Responses API、同一项目、同一份图像授权）。任务一请求 2 张，其中一张的本地预览被写成非图片字节：回执 `ok=true`、`uploaded_images=1`（只有读得出来的那张真的出去了）、观察名单恰为那一张、`unreadable_photo_ids` 为坏的那张、`unobserved_photo_ids` 为空（不再把同一张数两次）、坏的那张在观察缓存里零条记录、授权用量扣到 1、图像请求 1 次；摘要含"另有 1 张本地预览读不出来、没有送出，不算看过"。任务二把剩下那张也写坏：运行返回错误，错误码属本地解码一类且文案说的是预览而不是模型，投影里任务为 `failed`，`inspect_photos` 回执行为 0，图像请求次数不增、用量仍是 1（失败批次不扣额度）。
- 上一批的部分漏答用例同时补断言 `unreadable_photo_ids` 为空——那张是"发出去了没人看"，不是"没发出去"；摘要文案改动后它原有的字符串断言仍按新格式命中。
- 门禁：默认核心 289 passed / 0 failed / 3 ignored（与本批前持平，新增断言都落在集成测试）；保留 `native-agent` 294 passed / 0 failed / 4 ignored；全目标 526 passed / 1 failed / 20 ignored（+1），唯一失败仍是另一会话未提交的 `structured_evaluation_reports_duplicates_similarity_and_metrics`（`preview_sha256` 门禁，与本批无关，未触碰）。rustfmt 与 `clippy --all-targets --all-features -D warnings` 退出 0。日志：/tmp/focalroom-1f-fmt.log、-vision-suite.log、-lib.log、-native.log、-all.log、-clippy.log。本批未调用生产模型，未改用户项目库，未提交/推送/部署，也未重新打包安装到本机。
- 前端与 Swift 本轮未跑：只新增一个回执键与摘要文案，客户端可见的 `image_status` 取值集合不变，回执正文在客户端不做严格 schema 校验（前两批同形状加键已双端绿灯）。
- 上一批"仍没做到"第 4 条里的"一次批量中单张预览读不出仍会中断整批"已闭环；同条其余两项（`evaluate_draft` 不区分层级与提示版本、无指纹旧缓存行按"未记录"沿用）与本批第 1–3 项仍未做。

### 仍没做到

1. **读不出来的照片怎么修好没落地**。本轮只做到如实点名与不占额度；预览重新生成仍要用户自己去图库里重导或重建缓存，工具回执里那句"先修好本地缓存再单独观察"没有一键通道，也没在任务失败时给出可点的修复动作。
2. **真实模型/真实坏缓存下的分布没实测**：坏预览的比例、一次批量里常见几张同时读不出来、以及模型对两份缺口名单的反应（会不会自行补传或改写判断依据），都需要真实数据，牵动配额与提示版本，需单独批准。
3. **提示词仍未教这条规矩**，与上一批同一开口：只在回执与摘要里说明。上下文预算卡在阈值上（300 张画布只剩几百字节余量），动笔前必须先跑预算用例。
4. **"预览读不出来"这个词眼下有两处含义**：本轮的 `unreadable_photo_ids` 说的是"这张根本没送出去"；而照片现在读不出像素、却还留着带版本记录的旧观察时，走的仍是上一批的复用路径，只把图像状态标成 `unreadable`（意思是"当初看过、现在没法核对"），它不进本轮名单。两种含义在界面上会不会被读成同一件事，没有核实。
5. 缓存版本比对用的那次现场压缩会先把每张预览解一遍；解不开的在真正的发送分类里再解一次才拿到错误。多图批次里这一步是重复劳动，只影响耗时不影响口径，本轮没优化。

下一步顺序：①把"读不出来的那张"接一个可执行的修复动作（重新生成预览后再单独观察一次），先核实现有命令能否复用；②（需批准）真实模型与真实坏缓存跑一轮，看两份缺口名单的实际分布；③上一批遗留的 `evaluate_draft` 层级混用与无指纹旧缓存行。

## 2026-11-07 Focal AI 第七批：预览坏了的照片先只读原图重算像素，救不回来的每张说清原因

### 起点

上一批把"本地读不出来的那几张"从整批失败里摘出来，做到不占额度、如实点名，但点名之后就没有下一步了：坏的是缓存文件，照片本身往往完好，用户看到的却是"这张没看过"，只能自己去重导或重建缓存。具体缺口是三条。

- 派生预览被清理、被写坏、格式不认时，这张直接进 `unreadable` 名单，程序没有再试一次"从原图重新算一份像素"。
- 回执里 `unreadable_photo_ids` 只有一份名单，没有区分"为什么"：原图不在这台机器上、原图内容已经变了、原图也解不开、这批没排上重算——这四类用户能做的事完全不同。
- 观察缓存的键含照片内容指纹，所以指纹一变旧观察自动作废；但反过来不成立：旧观察行仍然绑在坏文件上，靠"当初看过"沿用，界面上只标一个含糊的读不出来。

### 本轮代码

- 预取阶段读不出字节时先就地补救：用同一套 `image_pipeline::prepare` 从原图重算一份本次要用的像素，重算目录取系统临时目录下一个随机命名子目录，用完立刻删。原图、图库缓存、数据库、项目文件全程只读，本轮没有新增任何写路径，也没有新增迁移。
- 补救只用指纹核对得上的原图：重算得到的内容指纹必须等于库里记录的 `content_sha256`，不等就一律弃用——那是另一张照片，宁可说读不出来，也不把另一张的画面当成这张的观察。这条是硬门，不做"差不多就用"。
- 一批最多重算 4 次（`MAX_PREVIEW_REBUILDS`）。重算要走完整解码，大 RAW 可能几秒，一次观察不能因为缓存坏了把整轮对话拖住；超出上限的那几张如实报"没排上重算"，而不是不吭声。解码在 `spawn_blocking` 里跑，不卡异步执行器。
- 重算成功的照片与照常读出的照片等价参与本轮：进 `sendable`，占额度、走授权、写观察缓存行，观察缓存绑的是重算后真正交出去的字节指纹（`preview_sha256`），因此下一轮命中缓存时能核对到同一份像素。
- 回执新增两份信息：`preview_rebuilt_photo_ids`（哪几张是重算出来、真的看过的）和 `unreadable_photos`（每张一个原因码：`original_missing` / `original_unreadable` / `original_changed` / `rebuild_failed` / `rebuild_over_limit`），`coverageNotice` 同步解释这五个词各指什么、并写明图库那份缓存本轮没有改写。两份名单仍与上一批的漏答名单互不相交。
- 预取结构新增 `rebuilt` 与 `unreadable_reasons`，都带 `#[serde(default)]`：上一版写下的崩溃恢复回执照常解析，不会凭空多出重算记录；这两份新名单也照上一批 `unreadable` 的做法带进发送前写好的那份回执——缺口在请求发出之前就已确定，崩溃后重放仍说得出哪几张没出去、哪几张是重算来的。
- 时间线摘要分两句说：重算成功时说"其中 N 张的本地预览读不出来、已只读原图重算像素后送出"，仍送不出时按原有句式说"另有 N 张本地预览读不出来、没有送出，不算看过"，两类同时出现就同一行并列。

### 验证

- 新增单元测试 `repair_reads_pixels_from_the_original_only_while_it_is_still_the_same_photo`：原图完好时重算出可送出的字节；原图不在机器上报 `original_missing`；同一路径下换成另一张内容时报 `original_changed`；三种情形跑完，图库那份预览文件字节不变。
- 新增集成用例 `broken_preview_cache_is_recomputed_from_the_read_only_original_when_possible`（真实本地模拟 Responses API，一次请求 6 张、派生预览全部写成非图片字节）。六种原图状况各踩一种结局：第 1、4 张原图完好→重算成功并真的送出；第 2 张原图被删→`original_missing`；第 3 张同一张文件换成另一幅画面→`original_changed`；第 5、6 张已用完 4 次重算额度→`rebuild_over_limit`。断言落在：`ok=true`、`uploaded_images=2`、观察名单与重算名单都恰是那两张、`unreadable_photo_ids` 为另外四张、`unobserved_photo_ids` 为空、四张的原因码逐一对上、重算成功的两张各留一条观察记录而四张没送出的零条、图像额度只扣 2、图像请求 1 次；三条信任底线是六张的原图逐字节比对（被删与被换的两张除外，这两张的改动是夹具自己做的）、六份坏缓存文件内容原样未动、时间线同时含"其中 2 张…已只读原图重算像素后送出"与"另有 4 张…不算看过"。
- 上一批的用例改到"重算也救不回"的那一档：把坏照片的原图一并写成非图片字节，于是它仍然只能走降级路径，`uploaded_images=1`、`preview_rebuilt_photo_ids` 为空、摘要口径不变——这样两条用例分工清楚，一条证明救不回时只降级那一张，一条证明救得回时照常看过。整批都救不回的第二个任务仍按原口径失败（错误说预览、任务记 `failed`、零行回执、额度不增）。夹具另加一种编排形态 `tool:inspect_all_from_history`（一次请求全部照片），才做得出超过重算上限的一批。
- 门禁：默认核心 290 passed / 0 failed / 3 ignored（+1）；保留 `native-agent` 295 passed / 0 failed / 4 ignored（+1）；全目标 528 passed / 1 failed / 20 ignored（+2），唯一失败仍是另一会话未提交的 `structured_evaluation_reports_duplicates_similarity_and_metrics`（`preview_sha256` 门禁，与本批无关，未触碰）。rustfmt 与 `clippy --all-targets --all-features -D warnings` 退出 0。日志：/tmp/focalroom-1g-fmt.log、-unit.log、-vision.log、-lib.log、-native.log、-all.log、-clippy.log。本批未调用生产模型，未改用户项目库，未提交/推送/部署，也未重新打包安装到本机。
- 前端与 Swift 本轮未跑：只新增两个回执键与摘要文案，客户端可见的 `image_status` 取值集合不变，回执正文在客户端不做严格 schema 校验（前三批同形状加键已双端绿灯）。
- 上一批"仍没做到"第 1 条（读不出来的照片没有下一步）就此闭环：能在原图完好时自愈，不能时给出可行动的原因。

### 仍没做到

1. **修好的像素不写回图库缓存**，这是刻意的：本轮坚持"重算只读"，代价是那份坏文件一直是坏的。照片下一次命中本轮写入的观察缓存还能用，但界面上那张的图像状态仍会走含糊的"读不出来"一档，真正的解法是给用户一个"重新生成预览"的可点动作，把坏文件换成好文件——本轮没有做，也不该顺手在观察链路里写图库缓存。
2. **"已核对"这一格的含义变宽了，还没在界面复核**：重算成功那张的像素版本标的是 `verified`，但核对的是原图内容指纹，不是那份仍然坏着的预览文件。同一批里它同时出现在 `preview_rebuilt_photo_ids` 中，模型和回执都能看出它是重算来的；界面只挑一个词显示时会不会被读成"预览好好的"，没核实。
3. **原因码只到回执和模型，没到用户**。五个原因在界面上没有任何一处能看懂：`original_missing` 要的是"重连移动硬盘"，`original_changed` 要的是"这张已经不是当初那张"，`rebuild_over_limit` 要的是"再多等一会儿或分批看"，现在都只体现在"另有 N 张…"这一句计数里。
4. **真实坏缓存下的分布没实测**：一次批量里常见几张同时读不出来、重算平均耗时多少、4 次上限是否合适，都需要真实图库数据。本轮上限是按"不拖住整轮对话"定的一个保守值，没有测量支撑。
5. **临时重算目录的清理没有被断言**。代码里用完即删，但测试没法可靠验证：同一时间别的用例也在触发重算，扫系统临时目录会比出无关目录。这条留作一个更根本的问题——补救产物究竟该放在系统临时目录，还是项目自己的缓存目录里一个明确命名的位置。
6. **提示词仍未教这条规矩**，与上两批同一开口：模型看到 `preview_rebuilt_photo_ids` 与五个原因码时应当怎么做（重算的那些照常引用、`original_changed` 的那张不要试图补猜、`rebuild_over_limit` 的可以再来一次），只在回执文字里说明。上下文预算卡在阈值上（300 张画布只剩几百字节余量），动笔前必须先跑预算用例。
7. `evaluate_draft` 取历史观察时仍不区分层级与提示版本、无指纹旧缓存行仍按"未记录"沿用，与上批相同，未动。

下一步顺序：①把五个原因码接成用户看得懂的下一步（界面一句话加一个可点的"重新生成预览"动作，先核实现有命令能否复用）；②界面图像状态与重算名单对齐，别让"已经真的看过"的照片显示成"读不出来"；③（需批准）真实图库跑一轮，量一次批量里坏预览的常见张数与重算耗时，再回来定 4 次上限；④提示词与上下文预算的取舍；⑤`evaluate_draft` 层级混用与无指纹旧缓存行。

## 2026-11-07 Focal AI 第八批：坏掉的本地预览给出一个真能点的修复动作，结局逐张说实话

### 起点

上一批能在观察前从原图临时重算像素，但刻意不写回图库缓存——那份文件一直是坏的。于是界面上这张照片每过一次都要重算一遍，用户看到的仍然是一句"另有 N 张读不出来"，手边没有一个动作能把坏文件换成好文件。三条缺口很具体。

- 观察链路里"临时重算"只活在这一轮请求的内存里，不落盘：这是刻意的（不能在 AI 链路里悄悄写图库缓存），但代价是坏文件永远坏着。
- 五个原因码（原图不在、原图变了、原图也读不开、没排上重算、重算失败）只进回执和模型，界面上一个字都看不到。
- 真正能修好它的那一步——删掉坏缓存再按同一条工序重算——需要写图库缓存目录，只能是用户点一下才做的事，不能自动发生。

### 本轮代码

- 新增一个"修复本地预览"的命令：调用方只能点名照片 ID，一次最多 200 张，空名单直接拒绝。要读哪个原图、要写哪两份缓存，全部由后端按这条记录在册的字段（`original_path` / `content_sha256` / `thumbnail_path` / `preview_path`）决定，界面给不了路径，也就骗不了这条路去读任意文件。
- 纯逻辑放在图像管线模块而不是命令层：`repair_preview_jobs(缓存目录, 任务名单) -> 逐张结局`。好处有三——命令层只做"查库、转交"；单元测试和集成测试都能直接调用它，不必为了测试把命令模块改成公开；与上一批视觉重算共用同一套原因词表，界面不用再来一遍翻译。
- 判"坏没坏"只读文件头：文件存在且非零字节，再让解码器读一次尺寸（`image_dimensions`），不为判断把整张图解开。因此"本来就是好的"这条能便宜地回答，且能抓出那些有头无尾的半截文件。
- 动手前三道门，任一不过就一张文件都不写：① 原图必须还在这台机器上，否则 `original_missing`；② 原图当前内容指纹必须等于库里记录的 `content_sha256`，不等就 `original_changed`——重算出来的文件会用旧指纹命名，等于把另一张的画面挂到这张的名字上，这不是本步该替用户做的决定，界面上指的是"在照片上确认内容更新"那一步；③ 库里记的两份路径必须正好是"本应用按这份指纹命名的 `-320.jpg` / `-1400.jpg`"且落在本项目缓存目录内，否则 `cache_path_suspicious`。目录比对走规范路径，macOS 上 `/var` 与 `/private/var` 是同一个地方，按字符串比会把该修的每张都误判成可疑。
- 只删坏的那一份，再交给生成派生缓存的同一条工序 `image_pipeline::prepare` 重算：好的一份不动，色彩标注不会被白白重写一遍；文件名由内容指纹决定，所以数据库一行不改，也不需要迁移。重算期间原图又变了（`FILE_CHANGED`）时结论退回 `original_changed`，留下的文件不作数。
- 逐张结局是六个取值之一：`repaired`（并写明重算了哪一档，`thumbnail` / `preview`）、`healthy`、`original_missing`、`original_changed`、`original_unreadable`、`cache_path_suspicious`，各配一句实话说明下一步。命令本身不改项目域数据，因此不占修订号、不进幂等命令通道。
- 界面接在真正说得出"这张没送出"的那一处：AI 面板"已查看 N 张照片"展开后，只有图像状态是"读不出来"的那张后面跟着一个"修复本地预览"按钮，核对得上的那张不给按钮。点下去按照片分别记结局，同一批里每张的结局互不覆盖；文案走四套语言的键表，遇到后端给出的陌生原因码就原样露出，不替它编一句"已修好"。

### 验证

- 新增单元测试 `repair_rebuilds_only_the_corrupt_derived_file_and_never_touches_the_original`：把预览写成非图片字节，结局是 `repaired` 且 `rebuilt` 只有 `preview` 一档；原图与缩略图逐字节不变；重算出来的预览真的解得开且尺寸对得上。再点一次是 `healthy`、`rebuilt` 为空、文件字节与上一次一模一样（不重写就不会重写色彩标注）。
- 新增单元测试 `repair_refuses_to_write_when_the_original_is_not_the_recorded_photo`：原图被换成另一幅画面时报 `original_changed` 且那份坏字节原样留着（一张文件都不写）；原图被删时报 `original_missing`；库里记的缓存文件名对不上指纹时报 `cache_path_suspicious`。
- 新增集成用例 `repairing_the_corrupt_preview_closes_the_gap_the_observation_reported`（真实本地模拟 Responses API、同一项目、同一份图像授权）：两张照片的派生预览都写成非图片字节，先按库里的记录逐张修复——两张都是 `repaired`、都只重算 `preview` 一档，并逐字节比对六份文件：两份原图不变、两份缩略图不变、两份预览从坏字节变成上百字节、读得开的图。随后照常跑一次观察：`ok=true`、`uploaded_images=2`、`preview_rebuilt_photo_ids` 为空（不再需要临时重算）、`unreadable_photo_ids` 与 `unobserved_photo_ids` 都为空、观察缓存两张各一条、图像请求 1 次。这条把上一批的"救得回"与本批的"修得久"接成一个闭环。
- 前端新增两条用例（AI 面板）：一张状态"读不出来"、一张"已核对"时只出现一个"修复本地预览"按钮，点击按 `['p1', ['a']]` 调用仓库方法，成功结局显示"已按原图重新算出本地预览，下次让 AI 看这张就能送上"；后端返回 `original_missing` 时显示"原图不在这台机器上，要先重新定位这张照片"，且不出现那句成功文案——修不成不能装作修好了。四套语言各补 8 个键，键数一致。
- 门禁：rustfmt 退出 0；默认核心 292 passed / 0 failed / 3 ignored（+2）；全目标 531 passed / 1 failed / 20 ignored（+3），唯一失败仍是另一会话未提交的 `structured_evaluation_reports_duplicates_similarity_and_metrics`（`preview_sha256` 门禁，与本批无关，未触碰）；视觉套件 58 passed / 0 failed（+1）；`clippy --all-targets -D warnings` 退出 0。`native-agent` 核心 299 passed / 0 failed / 4 ignored——比上一批记录的 295 多出的不只是本批 2 个，另有并行会话新增的用例，未逐一归因。前端：typecheck 退出 0、`eslint --max-warnings 0` 退出 0、73 个测试文件全绿、命令契约检查 OK。日志：/tmp/t1h.log、/tmp/r1.log、/tmp/r2.log、/tmp/r4.log、/tmp/rc.log、/tmp/ft.log、/tmp/fl.log。本批未调用生产模型，未改任何真实用户图库或项目库（写操作只发生在测试自建临时目录），未提交/推送/部署，也未重新打包安装到本机。

### 仍没做到

1. **没有装机验证**：这是桌面端改动，按本项目的口径要重新打包、换包、重启并在他自己机器上点一次才算真落地。本批只做到门禁全绿，打包需要先打招呼。
2. **入口只有一处**。历史观察抽屉里那条"当前预览读不出来，这次没能核对图像"（意思是"当初看过、现在没法核对"）和照片详情面板都没有同一个动作；一批里坏了多张时只能逐张点，后端支持一次 200 张但界面没有"整批修好"。
3. **修好之后不会自动再看一次**。那条旧观察仍绑旧的文件指纹，界面上这张的状态不会因为修好了就自己变绿——要用户再让 AI 看一次才算看过，这一步本轮没有接（接上就意味着自动发起一次占额度的云端调用，不该默默发生）。
4. **上一批那五个原因码仍未逐字到用户**。本轮到用户的是修复动作自己的六个结局码；观察回执里的 `rebuild_over_limit` / `rebuild_failed` 还是只体现在"另有 N 张…"那句计数里。
5. **坏预览的实际分布与耗时没测**：一次批量里常见几张坏、单张重算平均多少毫秒、200 张一次点下去要等多久、要不要进度条，都要真实图库数据。本轮的上限是按"一次操作别把界面钉住"拍的保守值。
6. **手机侧没有对应动作**：手机不发本地图库缓存文件，图片走托管链路，这台机器上没有"那两份派生文件"可修，因此本批不做手机端；真要在手机上给出等价提示，需要先确认它的读图失败究竟是哪种。
7. **与缓存回收的相互作用没断言**：`sweep_orphans` 按在册指纹保留文件，修复后文件名不变，理论上不会把刚修好的当孤儿收掉，但本轮没有为这条加断言。
8. 上一批"仍没做到"第 2 条（`verified` 一词含义变宽、界面没复核）与第 6 条（提示词没教这条规矩）仍在；第 1、3 条本轮分别闭环与部分闭环。

下一步顺序：①打招呼后打包装机，在他机器上真点一次这个按钮；②把同一个动作补进历史观察抽屉与照片面板，并加"整批修复"入口；③真实图库跑一轮，量坏预览的常见张数与单张耗时，再回来定上限与是否需要进度条；④修好后引导"再看一次"的措辞与是否自动发起（牵动额度）；⑤界面图像状态与 `preview_rebuilt_photo_ids` 对齐；⑥提示词与上下文预算的取舍；⑦`evaluate_draft` 层级混用与无指纹旧缓存行。

## 2026-11-07 Focal AI 第九批：草案评估里"一张照片该听哪一条观察"写成明文规矩，跨口径的比较不再混着报

### 起点

上一批留下的第 ⑦ 条，加上本轮排查时确认的三件事：

1. `evaluate_draft` 给草案里每张照片只复用一条画面观察，但"留哪一条"过去的规矩是**编号最大那条**——也就是最近写入的那条。同一张照片可能同时存着当前页面档（短边 512、质量 75）、更大尺寸的细节档（1400、质量 90）、以及历史提示词版本留下的记录。这几种"画面事实"是在不同问法、不同像素下答出来的，按谁最晚就听谁的，等于悄悄拿两把不同刻度的尺子比字符串。相邻相似判定（Jaccard ≥ 0.6）直接把两侧混着算，读者看不出这一对里左边是细节档、右边是页面档。
2. `read_creative_context` 的 `visual_overview` 只荐一对比较候选（序列里相邻、且本页已核对像素），SQL 里只有 `LIMIT 1`，没写排序。
3. `tests/agent_evaluate.rs` 里那条"相邻高相似应命中"的集成测试从 1-B 之后一直是红的：夹具写观察记录时不填 `preview_sha256`，而评估已要求像素对得上，于是这两张照片双双落在"版本未记录"分支里被剔除。这是夹具没跟上口径，不是产品算错。

### 本轮代码

1. **取哪一条，写在 SQL 里**：`agent/tools.rs` 的观察复用改成固定优先级分组——当前页面档 > 当前细节档 > 同工具的其他提示词版本 > 其他工具，同组内取最新一条。优先级只在 SQL 的一处 `CASE` 里定义，Rust 侧不再复述一套规则，而是把这条 SQL 返回的分组号原样映射成名字（`current_page_tier` / `current_detail_tier` / `other_prompt_version` / `other_tool`），避免出现"选的是 A 口径、报的是 B 口径"。
2. **回执把口径说清楚**：每张入选照片在 `item_facts` 里带上用了哪一档、哪个提示词版本、哪个工具；`metrics.evidence_contracts` 给出四档各占几张；相邻相似对被判为相似时额外带 `contracts` 与 `same_contract`，跨口径的那对在 `metrics.cross_contract_adjacent_pairs` 里单独计数；被剔除的照片除了原有的 `pixel_version` 也带上口径。政策说明与不确定性说明同步改写，明确"跨口径的相同程度不能当成同一把尺子的比较"。跨口径的对**仍然计算并列出**，只是打上标记——把信息删掉不如把信息标出来。
3. **比较候选的排序落到纸面**：`visual_overview` 荐那一对比较候选的查询显式 `ORDER BY position,photo_id`。序列位置本身有唯一约束，因此这更像是把"每次荐同一对"从依赖执行计划变成写在语句里，而不是纠正一次已经发生的错荐。
4. **把那条红灯夹具修好**：`cache_observation` 现在按真实写入方式记下预览字节的指纹，测的才是相似性比对本身，而不是"版本未记录"那条分支。

### 验证

- 新增 `evaluation_keeps_page_and_detail_observations_as_separate_contracts`：两张照片，第一张先有页面档、之后又跑过一次细节档（编号更新），第二张只有细节档。断言第一张用的仍是页面档、报的是页面档那三条事实；跨口径那对相似度照常命中但 `same_contract=false`，四档计数与跨口径计数各自对得上。这条测试是反证过的：按旧的"取最新一条"写法，第一张会被换成细节档、事实只剩一条，那一对也不再达标。
- `tests/agent_evaluate.rs` 由 1 红转 3/3 绿（含之前那条）。
- Rust 五门：lib 293 通过 0 失败 3 忽略；`--all-targets --no-fail-fast` 533 通过 0 失败（本轮之前那一个是唯一失败项）；`--features native-agent` 绿；clippy `--all-targets --all-features` 0 条；改动文件 rustfmt 干净。
- 本轮没有动前端与四套语言文案，因此前端四门维持上一批的绿灯状态，未重跑。
- 没有发起任何真实模型调用；没有改动数据库结构；原图仍只读。

### 仍没做到

1. **界面上看不到口径**：这份回执是给策展模型和取证用的，桌面端没有任何组件渲染 `evaluate_draft` 的字段，因此普通用户在屏幕上现在还看不到"这一对是跨口径比出来的"。要做需要先决定它在哪个界面出现。
2. **跨口径对只是打标，没被排除**：仍然参与相似度计算并出现在列表里。要不要改成只在同口径内比较、跨口径只提示不打分，是一个产品判断，没替他定。
3. **历史版本行的真实分布没测**：一台真实图库里同一张照片通常并存几种口径、页面档缺失有多常见，本轮全为合成夹具，未量。
4. **无指纹的旧缓存行仍未回填**：`preview_sha256` 为 NULL 的历史记录照旧被剔除并如实报"版本未记录"。修它们要么重算像素指纹（本地、便宜，但要定批量入口与灰度），要么引导重看一次（花额度）。这条还挂着。
5. **细节档与页面档的可比性没定义**：现在能区分，但"什么算同口径可比"只到提示词前缀这一层；同一档内不同端点（不同模型）写的行仍会互相顶替，只在同级里按最新取。
6. **手机端不涉及**：这条链路在桌面核心里，本轮未评估手机侧是否复用同一段选择逻辑。
7. **没装机**：本轮为纯后端与测试改动，未重新打包，未在他机器上验证；他机器上的观察缓存分布无法从合成测试推断。
8. 上一批"仍没做到"里第 ①②③④⑤⑥ 条（打包装机、整批修复入口、真实图库耗时、修好后引导再看一次、界面与 `preview_rebuilt_photo_ids` 对齐、提示词与上下文预算）本轮一条都没推进；第 ⑦ 条里的 `evaluate_draft` 层级混用已闭环，无指纹旧缓存行仍在。

下一步顺序：①打招呼后打包装机，把 1-H 与本轮一起装机验证；②决定跨口径对的处置（打标 vs 排除）并把口径显示到某个界面上；③无指纹旧缓存行的批量回填方案；④`visual_overview` 里同口径不同端点的选择粒度；⑤真实图库跑一轮，量口径并存情况；⑥提示词与上下文预算的取舍（仍要先跑预算那条用例）。

## 2026-11-07 Focal AI 第十批：历史观察抽屉里也能修坏预览，并且可以一次修完这一页

### 起点

1-H 把"修复坏掉的本地中间文件"这个动作接到了 AI 面板里，但只在那一处：上一批"仍没做到"第 2、3 条明确写着历史观察抽屉与照片面板没有同一个动作，也没有批量入口。用户在抽屉里能看到"当前预览读不出来，这次没能核对图像"这句话，看到之后却什么都做不了。另一处缺口是词汇：抽屉与面板各自描述同一个结局，将来加一档结局就要改两处，容易一边改一边没改。

### 本轮代码

1. **共用一套结局说法**：把"某个结局对应哪句人话"从 AI 面板里提出来，成为一个小组件外的共用模块（一个映射函数 + 一个把逐张结局整理成 `{照片: 说法}` 的函数）。面板改为引用它，删掉自己那份副本；抽屉也引用它。以后加一档结局只改一处。
2. **抽屉里给出动作**：历史观察列表新增一个可选的修复通道（由上层传入，非桌面端或没有该能力时整个按钮不出现）。这一页里凡是"当前预览读不出来"的照片会汇总成一个按钮：只有一张时是"修复本地预览"，多于一张时是"一次修复这 N 张本地预览"。
3. **逐张结局回到逐张位置**：修复按照片逐张返回结局，界面上把每条结局放回它自己那张照片的状态行下面，而不是给一个"成功 N 张"的合并数字。批量这一路的结局集合与单张完全相同（`repaired` / `healthy` / `original_missing` / `original_changed` / `original_unreadable` / `cache_path_suspicious`），措辞沿用 1-H 已入四套语言的那组，本轮只新增一个批量按钮文案键（四套语言各 1 条）。
4. **失败不谎报**：调用本身抛错时，那几张照片各归到"这一步没能完成，本地文件没有改动"，不会显示成已修复。

### 验证

- 新增三条组件检查：一页里两张坏预览，点一次批量按钮，只传这两个照片号，两张下方分别出现"已重新算出"和"原图不在这台机器上"两种不同结局且不出现失败句；只有一张坏时按钮是单张文案；调用失败时说"没能完成"而不是"已修好"；页面上没有坏预览时压根不出现修复按钮。
- 前端四门全绿：类型检查 0 错、eslint `--max-warnings 0` 通过、73 个测试文件 579 条全通过、接口契约一致性检查通过（四套语言键数仍一致）。
- 后端本轮未改代码，上一批的 Rust 五门结果维持不变。
- 没有新增命令、没有改数据库、没有调用真实模型；修复仍然只写图库自己生成的缓存文件，原图只读这条性质由后端那道闸门保证，本轮没有绕过它。

### 仍没做到

1. **画布审阅那条路没有同一个按钮**：画布侧的历史观察用的是另一套仓库接口，里面没有修复能力，所以编辑审阅抽屉仍然只说明、不动手。要补需要先把能力传到那个仓库上。
2. **照片面板没有入口**：单张照片的检查器里仍然看不到这个动作，用户得先在 AI 对话或抽屉里遇到坏预览才有按钮。
3. **修完不自动刷新状态行**：修复后那张照片的"读不出来"这句话不会当场变绿，需要翻页或重开抽屉重新读取才会更新。这是有意的取舍：重新读取会把这一页整页换掉，逐张结局说明也就跟着被冲掉——而恰恰是"原图不在""原图已改"这些失败结局最需要留在屏幕上。要做的是把两者并存（保留结局、局部重取状态），本轮没做。
4. **跨页批量没有**：按钮只覆盖当前这一页（一页 30 条）。整项目有多少张坏预览、要不要一次全修，仍然没测也没做。
5. **界面仍未显示口径**：上一批 `evaluate_draft` 的口径出处（哪一档、哪个版本）还是只在回执里，屏幕上没有。
6. **没装机**：这两批改动都还没重新打包，没在他机器上真点过。
7. 上一批"仍没做到"第 2、3、5、6 条本轮均未推进；第 1 条（画布/抽屉入口、整批修复）里的抽屉与批量两项本轮闭环，画布与照片面板两项仍在。

下一步顺序：①打招呼后打包装机，把 1-H 到本轮一起装机验证；②把这个动作补进照片面板，并决定跨页/整项目批量；③修完后自动重取该照片的状态；④把口径出处显示到界面上；⑤无指纹旧缓存行的批量回填；⑥画布审阅仓库的修复通道。

## 2026-11-07 Focal AI 第十一批：照片面板里也能就地检查并重算这张的本地预览

### 起点

1-H 的按钮在 AI 对话面板，1-J 补进了历史观察抽屉，但用户最常停留的地方——右侧单张照片检查器——仍然只有"这张图不对"的说明，没有动作。上一批"仍没做到"第 2 条就是这一处。另一处历史遗留是：原图不在这台机器时，检查器已经有"重新定位原图"的按钮，但这两件事（原图不在 vs 派生文件坏了）在界面上是两套说法、两个位置，用户很难判断该点哪个。

### 本轮代码

1. **检查器加一个可选的就地动作**：照片面板新增一个可选的修复通道，由上层在"本地项目 + 该能力存在"时才传入；没有这个能力（云端项目、非桌面端）时整块不出现，连说明文字都不显示，避免出现"看得见但点了没反应"。
2. **把这条命令当成"检查 + 必要时重算"来讲**：修复命令本身对健康的文件返回"这两份本地文件本来就能读，没有改写"，对坏的返回重算了哪一档，所以这个按钮不需要界面先去猜这张坏没坏——点了才知道，知道的是逐张的真话。说明文案写清三件事：AI 看的是这张图算出来的本地预览、可以就地检查并重新算一次、原图不动。
3. **结局说法继续共用**：这一处直接引用 1-J 提出的同一份结局词汇表，没有再复制一份；调用整体失败时归到"这一步没能完成，本地文件没有改动"，不会显示成已修好。
4. **不加自动刷新**：与 1-J 同一个取舍，重取整页会把逐张结局说明冲掉，而失败结局正是最需要留在屏幕上的那部分。

### 验证

- 新增一个此前不存在的检查器测试文件，三条：点一次只动这张照片，并把"已按原图重新算出本地预览"的结局显示在面板里、说明文字里确实写着原图不动；调用失败时出现"这一步没能完成，本地文件没有改动"且没有"已修好"；没有传入这个能力时按钮压根不出现。
- 前端四门全绿：类型检查 0 错、eslint `--max-warnings 0` 通过、74 个测试文件 582 条全部通过、接口契约一致性检查通过。四套语言各新增 1 条说明文案（本轮唯一的文案增量），数量仍一致。
- 后端零改动，Rust 五门沿用上一批结果；没有调用真实模型、没有改结构、原图只读这条性质由后端闸门保证，本轮没有绕过。

### 仍没做到

1. **画布审阅抽屉还是没有这个动作**：那一侧的仓库接口里没有修复能力。可行的补法是把同一能力挂到画布仓库上，这样连传参链路都不用加长，但涉及仓库接口与画布侧测试，本轮没有开始，不想留半截。
2. **三处入口的可达性口径不一样**：对话面板的按钮只在回执点名的那些照片上出现（有依据）；抽屉按整页聚合；检查器任何时候都能点，靠命令返回的结局告诉你这张本来好没坏。三种各有道理，但没有统一成"这张照片的本地文件状态"这一处视图。
3. **修完仍不自动刷新那行的状态说明**（同 1-J 第 3 条）。
4. **跨页/整项目批量没有**；"整项目有多少张坏预览"目前没有任何界面回答得了。
5. **口径出处仍未上屏**（1-I 第 1 条不变）。
6. **没装机**：1-H 到本轮三批改动的本机重打包与真机点击验证仍未做。这一步需要先打招呼——这里是打招呼，不是提问；对外发布线我不会碰，那需要另行确认。
7. 上一批"仍没做到"第 2 条（照片面板入口）本轮闭环；第 1、3、4、5 条原样挂着。

下一步顺序：①打招呼后本机重打包，把 1-H 至本轮一起装机验证；②把修复能力挂到画布仓库，补上审阅抽屉那一处；③统一成"这张照片的本地文件状态"一处视图，顺带解决修完后局部刷新；④整项目坏预览的清点与批量；⑤口径出处上屏；⑥无指纹旧缓存行的回填方案。

## 2026-11-07 Focal AI 第十二批：画布审阅抽屉接上同一个修复通道（能力挂到画布仓库，不再各侧一套）

上一批留下的第 1 条闭环。做法是把修复能力挂到画布侧的仓库接口上，而不是给抽屉再拉一条传参链路。

### 做了什么

1. **画布仓库接口有了这个能力**：与图库侧同为可选成员，签名一致（项目号 + 照片号列表 → 每张一个结局）。因为它是可选的，既有的画布测试夹具与不做修订依据的调用方都不必改。
2. **画布仓库实现调的是同一个后端命令**：命令名、参数名、返回结构与图库侧逐字段相同，并按同一份结构校验——返回缺字段、把 rebuilt 写成字符串、整体不是列表都直接抛错，不会把半截数据当成"修好了"。原图只读、不占修订号这条性质由后端命令本身保证，画布侧没有另开一条写路径。
3. **审阅抽屉的历史观察段落拿到按钮**：只有这一页里"读不出来"的那些照片才进批量名单；抽屉拿不到修复能力时不给按钮，而不是给一个点了没反应的按钮。

### 验证情况

- 仓库层：断言调的命令名与参数与图库侧一致，并把四类畸形返回逐个确认会被拒绝。
- 抽屉层：新增三条——修复确实按"项目号 + 读不出来的那张"发出并显示该张的成功文案；修复报错时显示"这一步没能完成，本地文件没有改动"而不是成功；仓库没有这个能力时页面不出现按钮。
- 前端四门全绿：类型检查 0 错、eslint `--max-warnings 0` 通过、74 个测试文件 586 条全部通过（比上一批多 4 条）、接口契约一致性检查通过。四套语言文案零增量（本轮只接通道，措辞复用前十/十一批那三条）。
- 后端零改动，Rust 五门沿用第九批结果。

### 仍没做到

1. **四处入口的口径仍未统一成一处视图**（对话面板 / 草案抽屉 / 照片面板 / 本轮的画布抽屉）。现在都有能力，但"这张照片的本地文件状态"仍要自己在四处分别看。
2. **修完不自动刷新那行的状态说明**（同 1-J 第 3 条、1-K 第 3 条）：结果说的是这一次修复的结局，页面顶部"版本对不对"那句话仍是修复前的判断。
3. **整项目坏预览清点与跨页批量没有**；画布抽屉同样只有"这一页"。
4. **口径出处仍未上屏**（1-I 第 1 条不变）。
5. **没装机**：1-H 到本轮四批改动的本机重打包与真机点击验证仍未做。这里是打招呼，不是提问；对外发布线不碰，那需要另行确认。

下一步顺序：①打招呼后本机重打包，把 1-H 至本轮一起装机验证；②统一成"这张照片的本地文件状态"一处视图，顺带解决修完后局部刷新；③整项目坏预览的清点与批量；④口径出处上屏；⑤无指纹旧缓存行的回填方案。

## 2026-11-07 Focal AI 第十三批：修完之后那一行改口，"这张的本地文件状态"四处共用一套说法

前十到十二批留下的第 2、3 条（修完不刷新、三处口径各讲一套）这轮合并闭环。

### 做了什么

1. **一句话只在一处生成**：把"这份本地预览文件现在是什么状态"的判断收进修复词汇那一个模块里。后端给的结局里只有两个算"现在读得出来"（按原图重算成功、检查后发现本来就能读），其余结局一律不改口。之前各处是自己拼自己的话。
2. **修好的那张不再挂着旧的坏消息**：抽屉和对话面板里那一行"当前预览读不出来，这次没能核对图像"，在这一张修复确认能读之后换成新的一句——同时明说这条观察仍是按修复前那份字节读出来的，不假装 AI 已经重新看过这张。批量修复时只改点名的那几张，同页其它张的说法一个字不动。
3. **改口靠后端原因码，不靠翻译后的句子**：抽屉和面板现在存的是原始结局，显示时才取文案。原先存的是成品句子，于是"要不要改口"只能靠猜。

### 验证情况

- 抽屉层：批量修两张、一张成功一张原图不在——成功那张换成新说法、失败那张仍挂着"读不出来"，且这样的行只有一行；再加一条"文件本来就能读"也算现在读得出，同时确认同页另一张对不上的说法没被顺手改掉。
- 对话面板：修复成功后旧的坏消息消失、新说法出现、同批里本来好的那张仍显示"看的是这张图（版本 …）"；修复失败（原图不在）时旧的坏消息必须还在——这条专门用来防止把没修成显示成修好了。
- 前端四门全绿：类型检查 0 错、eslint `--max-warnings 0` 通过、74 个测试文件 587 条全部通过、接口契约一致性检查通过（含四套语言键位一致）。四套语言各新增 1 条（"现在读得出来了"那句），数量仍一致。
- 后端零改动，Rust 五门沿用第九批结果。本轮没有新的写路径，原图仍只读。

### 仍没做到

1. **照片面板没有这一行可改口**：那个入口拿不到"这条观察是按哪份字节读的"，它只有修复动作和结局，所以本轮的统一只覆盖对话面板 + 两处审阅抽屉。要把面板也纳入，需要那条回执信息能走到检视器，不是补文案就完事。
2. **"这轮修完之后 AI 到底会看到什么"仍要下一句才生效**：改口那句已经明说了这一点，但界面没有"就按修好的这份重看一次"的动作。那是一次真实的模型调用，本轮没有发起。
3. **整项目坏预览清点与跨页批量没有**；抽屉仍只有"这一页"。
4. **口径出处仍未上屏**（1-I 第 1 条不变）。
5. **没装机**：1-H 到本轮五批改动仍未本机重打包验证。这里继续打招呼，不是提问；另外这棵树里混着另一个会话未完成的改动，干净的打包要么先提交、要么从干净提交拉独立工作树——两条都需要先点头，我不会自己动。

下一步顺序：①先解决装机（需要先点头）；②整项目坏预览的清点与跨页批量；③口径出处上屏；④无指纹旧缓存行的回填方案；⑤把检视器也纳入同一处状态视图（依赖回执信息走到检视器）。

## 2026-11-07 Focal AI 第十四批：口径出处上屏——这份方案的画面事实是按哪一档问法挑出来的

第十三批的第 4 条（口径出处仍未上屏，即 1-I 第 1 条）本轮闭环。1-I 只把"按口径挑观察"这件事做进了评估本身，界面这一侧此前什么也没说。

### 做了什么

1. **核验回执里的计数第一次走到界面**：评估草案那次工具调用本来就在库里留了回执，但任务投影只取"步骤标题"，回执正文从来没上过屏。现在投影从回执里只取六个数——方案照片总数、有可核对画面观察的张数、没有的张数、四档问法各挑中几张、相邻两张跨档的次数、因为原图换过或预览读不出而被排除的张数——同一份方案只留最近一次核验。
2. **只投计数，不投正文**：回执里挨张记录的画面事实一个字都不进这条投影。测试专门把正文写进夹具再断言序列化结果里查不到那句话，用来防止以后有人图省事把整段回执丢给界面。
3. **形状对不上就当没有**：回执缺失、状态是失败、口径不是"结构化本地核验"、六个数里少任何一个、单条回执超过 64KB、内容不是 JSON——一律跳过，界面上这一段干脆不出现。宁可不说，也不摆一段猜出来的摘要。
4. **界面上的四句话**：有观察几张/没有几张一句；四档问法分档一句；只有跨档次数不为零才说"不能当成同一把尺子比"；只有排除张数不为零才单列一句。零计数不写成"另有 0 张…"。
5. **配对按方案走，不按任务走**：这句话按"这条回执是哪份方案的"来配对。测试里方案只有 1 张照片、回执说 2 张，显示出来仍是 2 张——用来证明数字来自后端，不是界面自己数的。

### 验证情况

- 后端五条门全绿：整体编译、单元测试 295 条通过（含本轮新增 2 条：一份方案留最近一次核验 + 坏形状一律跳过）、全部集成测试套件通过、clippy `-D warnings` 通过、格式检查通过。
- 前端四条门全绿：类型检查 0 错、eslint `--max-warnings 0` 通过、74 个测试文件 593 条全部通过、接口契约一致性检查通过（含四套语言键位一致，本轮每套各加 5 条）。新增 6 条用例：审核弹窗 3 条（完整口径 / 一张观察都没有时改口 / 没有回执时不编）、IPC 边界 1 条（缺字段按没有核验处理、负数与文字整条拒收）、工作台链路 2 条（数字来自回执而非界面数、回执对不上这份方案时不借隔壁方案说话）。
- 夹具改动值得一提：一开始测试自己拼回执的键名，和真实写入用的结构体字段拼法不一致，于是"好的回执"也被判成形状不符、投影返回空。改成让夹具直接序列化真实的回执结构体，这条差异才被抓出来并修掉——真实写入路径没受影响。
- 本轮没有新的写路径，原图仍只读；没有生产模型调用；不动数据库结构。

### 仍没做到

1. **只到"几张"，没到"哪几张"**：界面现在说得出四档各几张，说不出具体哪几张走的是大尺寸细节档。逐张的口径其实就在回执里，要上屏得把每张对应的档位也投出来（仍不能带正文）。
2. **没标"截至哪一版"**：这句话对应"这个任务最近一次核验"。核验之后草案又被改过（版本号前进）时，界面上的分档仍是上一次核验的口径，目前不显示它是对着哪一版算出来的。
3. **只有对话工作台这条路能看到**：审核弹窗目前只从 AI 工作台进入；画布侧的方案审核走的是另一套抽屉，本轮没接。
4. **纯人工或外部编辑改出来的草案不显示这一段**：它们没有核验回执，界面按"没有依据"处理，不给解释为什么会没有。
5. **没装机**：1-H 到本轮六批改动仍未本机重打包验证。这里继续打招呼，不是提问；对外发布线不碰。这棵树里混着另一个会话未完成的改动，干净打包要么先提交、要么从干净提交拉独立工作树——两条都需要先点头，我不会自己动。

下一步顺序：①先解决装机（需要先点头）；②把"哪几张走哪档"逐张上屏；③给口径一句"截至哪一版"；④整项目坏预览的清点与跨页批量；⑤无指纹旧缓存行的回填方案；⑥把检视器也纳入同一处状态视图。

## 2026-11-07 Focal AI 第十五批：逐张口径上屏——这张是按哪一档问法看的、那张为什么没用上

第十四批只做到"几张"，本轮把"哪几张"补上（第十四批仍没做到第 1 条）。

### 做了什么

1. **逐张的说法跟着照片一行一行走**：那次本地核验本来就为每张照片记下"用的是哪一档问法"，以及"这条画面记录为什么被排除（图像版本对不上、本地预览读不出、当初没记版本）"。现在这两件事直接出现在方案审核弹窗的照片行里，不再只有上面的汇总数字。
2. **只认写死的几种名字**：问法档位四种、图像版本三种，是后端自己产出的那套枚举；回执里出现任何别的写法，这一条整条丢掉，既不点名也不解释。逐张点名另有 100 条上限，防止一份方案把整段回执搬进投影。
3. **没记过就一个字都不说**：一张照片如果回执里既没有档位也没有排除记录，界面上不加任何评语。关键是**不把"没有记录"当成"没有观察"**——后者是能被数据支持的结论，前者只是没数据。
4. **一句截断说明只在真需要时出现**：核验逐张只扫方案的前 12 张（后端硬上限）。当"有画面观察的张数"比"逐张认得出的条数"多时，补一句"其余只算进总数"；两者一样时不说。
5. **计数摘要多两个字段**：逐张认得出的条数，和逐张点名清单（照片号 + 档位 + 用没用上 + 图像版本）。仍然一个字的观察正文都不带。

### 验证情况

- 后端五门全绿：40 个测试套件全部通过（含本轮扩写的两条断言：陌生档位与陌生图像版本整条丢弃、投影里查不到观察正文和 "verified" 字样），clippy `-D warnings` 通过，格式检查通过。
- 前端四门全绿：类型检查 0 错、eslint `--max-warnings 0` 通过、74 个测试文件 595 条全部通过、接口契约一致性检查通过（四套语言本轮各加 8 条，数量仍一致）。
- 新增/扩写用例覆盖：照片行按档位说话 + 被排除那张说清为什么没用上；记档齐时不出现截断说明；后端没记这张时界面对它一个字都不说；有观察的多于记档的才补那句；IPC 边界收到陌生档位名整条拒收；工作台上从任务投影一路走到照片行（数字与逐张口径都来自回执，方案只有一张照片也照样显示回执给的数）。
- 顺带固定住一件口径差异：汇总的"没采用张数"按回执列出的条数算，逐张只点名认得出图像版本的那种——测试里故意放一条陌生版本，把这个差异写成断言，防止以后有人以为两边一定相等。

### 仍没做到

1. **逐张只覆盖回执扫过的前 12 张**：方案超过 12 张时，后面的照片不会有逐张说法，只有那句"其余只算进总数"。要全覆盖得改后端核验的逐张上限，那会同时加重给模型的提示词负担，本轮没动。
2. **没标"截至哪一版"**：这句话仍对应"这个任务最近一次核验"，核验之后草案被改过时不显示它是对着哪一版算的。
3. **只有对话工作台这条路能看到**：画布侧的方案审核仍走另一套抽屉。
4. **排除原因只到图像版本这一层**：说不到"当初看的是哪份字节、现在这份是什么"这种可以逐条比对的凭据。
5. **没装机、没提交**：1-H 到本轮七批改动都还只在这棵工作树里。装机需要先点头；对外发布线不碰。

下一步顺序：①先解决装机（需要先点头）；②给口径一句"截至哪一版"；③整项目坏预览的清点与跨页批量；④无指纹旧缓存行的回填方案；⑤把检视器也纳入同一处状态视图。

## 2026-11-07 Focal AI 第十六批：口径加上"对着哪一版算的"，改过之后不再拿旧口径冒充新口径

第十五批仍没做到第 2 条（没标"截至哪一版"）本轮闭环。

### 做了什么

1. **核验时就把版本号写进回执**：那次本地评估现在会顺带读出方案当时在库里的版本号，一并记进回执。版本号来自库里那一行，不是界面自己推的——所以界面说的"哪一版"和真正的方案版本不可能各说各话。
2. **界面上三种情况分开说话**：回执记的版本号就是现在这一版 → 明说"就是现在这一版"；不是现在这一版 → 明说"对着第 N 版算的，现在已是第 M 版：中间改过，口径没重算"；老回执根本没记过版本号（升级前留在库里的那批核验） → 这一句整块不出现，绝不猜一个版本号出来。
3. **投影多带一个可空字段**：版本号取不到就是"没记过"，不会被投成 0 或者 1 这种看着合理的数。四套语言各补 2 句。

### 验证情况

- 后端五门全绿。值得一提是这轮把断言加在了两头：真实调用那侧（建方案 + 跑核验的完整链路测试）要求回执里的版本号必须等于库里方案那一行的版本号；投影那侧要求缺字段的老回执投成"没记过"而不是编一个数，同时失败核验仍然不投。两头分开盯，是因为只测投影会出现"字段从来没人真的写进去"这种空转。
- 前端四门全绿：类型检查 0 错、eslint `--max-warnings 0` 通过、74 个测试文件 598 条全部通过、接口契约一致性检查通过（四套语言本轮各加 2 条）。新增 3 条用例：版本一致时讲明白是哪一版、方案改过时必须说"没重算"且不出现"就是现在这一版"、老回执没记版本号时这句话整块不出现而其余口径照常。
- 新字段是必填可空，缺字段会被边界校验挡下来；本轮没有专门为"版本号写成文字/负数"补前端用例，靠的是后端只会投出正整数或空。

### 仍没做到

1. **"要不要按现在这一版重算一次"没有动作**：界面只会说"口径没重算"，点不动。真要重算是再跑一次本地核验（不上传照片），需要一个新的用户触发命令，本轮没做。
2. **逐张口径仍只覆盖回执扫过的前 12 张**（第十五批第 1 条不变）。
3. **画布侧方案审核仍未接**这套口径说明。
4. **没装机、没提交**：1-H 起这八批改动都还只在这棵工作树里；装机需要先点头，发布线不碰。

下一步顺序：①先解决装机（需要先点头）；②加一个"按现在这一版重算口径"的动作（纯本地核验，不上传）；③整项目坏预览的清点与跨页批量；④无指纹旧缓存行的回填方案；⑤把检视器也纳入同一处状态视图。

## 2026-11-07 Focal AI 第十七批：「口径没重算」不再只是一句提示，点一下就按现在这一版重算（纯本地、不上传）

第十六批仍没做到第 1 条（那句"口径没重算"点不动）本轮闭环。

### 做了什么

1. **多了一个用户可点的重算动作**：方案审阅里，只有口径确实可能对不上时才出现「按现在这一版重算口径」——要么回执记的版本号不是现在这一版，要么有照片的画面记录被排除在外（修完坏预览就属于这种）。口径本来就是新的时候不出现，免得看着像还有一件事没做。
2. **重算复用同一条本地核验**：新命令没有另写一套判断，直接把任务里那条本地评估工具再跑一遍——拿库里已经存下的画面事实和现在的预览文件重比。不经过模型、不上传照片，也不改方案版本号。
3. **结果仍旧只写回执表**：算完往回执表新增一条核验记录（旧的那条改不了，库里有触发器挡着），界面再从同一套投影读数字。没有另存一份"重算结果"，那样等于造出两处真相。
4. **命令返回值只有计数和版本号**：观察正文一律不从这条通道回到界面。前端把返回值当确认单看，而且确认单点名的方案必须就是刚点的那一份，对不上直接报错拒收。
5. **算完广播一次**：按方案更新和任务更新各发一条事件，面板重新取投影；面板本来就有一秒一次的轮询兜底，所以不靠这条返回值数字也能对上。
6. 四套语言各补 3 句（按钮、说明、完成回执）。

### 验证情况

- 后端五门全绿，且最后一条按真实退出码复核过：严格模式风格检查退出码 0、全量测试退出码 0、40 个测试套件全部为 ok、0 失败（此前那次是用管道截了尾巴，不足以作为结论）。新增 2 条真实数据库用例：一条先确认口径记的是第 1 版，手动把方案推进到第 2 版后再看投影——它仍停在第 1 版（就是界面上"没重算"那个状态），点一次重算后确认单和投影都变成第 2 版；同一条用例还盯住"重算不新建任务、不把项目记成改过"，因为看一趟口径不应该改动项目的修订记录。另一条要求方案不在指名项目里说"方案不属于这个项目"、已经不在库里说"方案已不在库里，请刷新后重试"，并且这两种失败都不往回执表写任何东西。
- 前端四门全绿：类型检查 0 错、eslint `--max-warnings 0` 通过、74 个测试文件 602 条全部通过、接口契约一致性检查通过（四套语言本轮各加 3 条，密钥数一致）。新增 4 条用例：按钮出现且说明里写明"不重新问模型，也不上传照片"、点一次只调一次重算并出现完成回执、口径本来就新或这份能力不存在时不出现按钮、后端报错时只显示错误且不出现完成回执。另有 1 条边界用例：计数为负、版本号为 0、版本号写成文字、确认单串到别的方案一律拒收，而版本号确实没记过（空值）仍算成功。
- 原始照片没动过；没有生产模型调用，没有库结构改动与新迁移。

### 仍没做到

1. **没装机、没提交**：这批和前面几批改动都还只在这棵工作树里；装机需要先点头，发布线不碰。
2. **重算只在打开的那一份方案里触发**：整项目坏预览的清点、跨页批量重算仍然没有。
3. **逐张口径仍只覆盖回执扫过的前 12 张**（第十五批第 1 条不变）。
4. **画布侧审阅抽屉仍未接**这套口径说明，也就点不到重算。已排查清楚这不是接线活：画布草案存在 `boards` 而不是 `drafts`，本地评估工具也不在画布工具白名单里，所以画布任务的口径投影恒为空、重算只会报"方案已不在库里"。要接必须先动后端（让评估与投影认画布那份草案），再加 3–5 个前端文件；画布仓库接口目前只有修复能力，不含代理仓库。
5. **无指纹旧缓存行的回填方案仍未定**；检视器也还没纳入同一处状态视图。

下一步顺序：①先解决装机（需要先点头）；②整项目坏预览的清点与批量重算（能复用本轮这条通道，纯后端计数 + 前端一次点全）；③把检视器纳入同一处状态视图；④摄影知识与引用的"为什么算这条"原因码上屏；⑤无指纹旧缓存行的回填；⑥逐张口径放开 12 条上限（注意回执有 64KB 长度门槛，直接抬上限会让整份口径从界面消失，得连投影查询一起改）；⑦画布侧口径与重算（要先动后端：画布草案不在方案表里，见上面第 4 条）。

## 2026-11-07 Focal AI 第十八批：坏预览从"这一页几张"升到"整个项目还欠多少"，一次修完并把口径跟着重算

第十七批下一步顺序里的第 ② 项（整项目坏预览的清点与批量重算）本轮落地。

### 做了什么

1. **多了一份只读的点名单**：以前只有"这一页里坏了几张"，跨页的看不见、也数不出总数。现在一次数清整个项目：多少张现在就能读、多少张能只读原图重算、多少张得你先把原图找回来（按原因分开计数），并且逐张点名到照片。这一步一个字节都不改。
2. **一键修完，一轮的边界写在结果里**：修的时候沿用原来那张体检与同一条重算工序（不另写一套判断），一次最多 200 张——每张都要解一次原图，这是原来就有的上限。超过的不会假装做完，结果里明说"还剩几张留到下一轮"。
3. **修完自动把口径跟上**：只要真修好了东西，就把这个项目里最近的各份方案按现在这一版重算一遍（第十七批那条本地通道），并回报重算了几个、有没有失败的。这一步仍旧不经过模型、不上传照片、不改方案版本号。
4. **清点为什么不比对原图内容，写在代码与结果里**：那道门要把原图整张读一遍，按页点名可以，按项目点名会变成几分钟的磁盘扫描。所以外部改过原图的那几张在清点里算"能用"，真去重算时逐张仍旧拦下——清点只会少报，不会多报，两边不互相冒充。
5. **界面上仍是两步**：先"数一下整个项目"，数出来确实有要修或要找回原图的，才出现"一次修完"；一张都不用修就不推销。四套语言各补 4 句。
### 验证情况

- 后端：新增 5 条对着真实数据库与真实缓存文件的用例全部通过（点名不动一个字节、原图没了的按原因拦下且一键修也不替它决定、一次修完真把每张都修好并只写一条口径回执、跨"分页永远凑不到一起"的照片一次够得着、项目不存在时先拒绝对文件都不读）。编译检查 0 错；严格风格检查（带"告警即失败"）、全量测试、格式检查都以真实退出码复核过：clippy 0、全量测试 0（40 个测试套件全部为 ok、0 失败）、格式检查 0 差异。
- 前端四门本轮已确认全绿：类型检查 0 错、eslint 退出码 0、接口契约检查退出码 0、74 个测试文件 608 条用例全部通过（本轮新增 5 条：点名数字来自后端而不是这一页、一次修完要报清修了几张/留下几张/几份口径跟着重算、没这能力时不摆入口且一张都不用修时不推销、点名单只认非负计数与逐张原因（负数、文字计数、缺字段条目一律拒收）、修完的计数缺了就当没修而不拿空值冒充零张）。
- 四套语言各加 4 句后密钥总数仍一致（四份都是同一规模），原始照片一张没动，没有生产模型调用，没有库结构改动与新迁移。

### 仍没做到

1. **没装机、没提交**：这批与前几批都还在这棵工作树里；装机需要先点头，发布线不碰。
2. **入口挂在方案审阅那一格里**：整个项目的点名单其实和某一份方案无关，图库那一侧还没有同一个入口，本轮先接在能看到"画面记录被排除"的地方。
3. **一轮 200 张是硬上限**：大项目要连点几次，界面只如实报"还剩几张下一轮"，没有自动续跑到完。
4. **清点不比对原图内容指纹**（故意的开销取舍）：外部改过原图的那几张在清点里算"能用"，只有真重算时才逐张拦下。
5. **修完只重算该项目最近 20 份方案的口径**：更早的方案要靠第十七批那个按钮自己点。
6. **逐张口径仍只覆盖回执扫过的前 12 张**；画布侧口径仍未接（要先动后端，见第十七批第 4 条）。

下一步顺序：①先解决装机（需要先点头）；②把整项目点名单挪到图库那一侧（与方案无关的事实要有一个与方案无关的入口）；③把检视器纳入同一处状态视图；④摄影知识与引用的"为什么算这条"原因码上屏；⑤修完的口径重算覆盖超出 20 份的老方案；⑥无指纹旧缓存行的回填；⑦逐张口径放开 12 条上限（回执有 64KB 长度门槛，得连投影查询一起改）；⑧画布侧口径与重算（要先动后端）。

## 2026-11-07 Focal AI 第十九批：整项目点名单搬到图库一侧——文件的事不用先打开某份方案才看得到

第十八批下一步顺序里的第 ② 项本轮闭环（第 ① 项装机仍等点头）。

### 做了什么

1. **入口换了地方，不再重复两处**：整项目的"数一遍 / 一次修完"从方案审阅弹窗里摘出来，常驻在图库一侧（网格与作品序列都在），方案弹窗里只留与那份方案口径有关的重算。弹窗里补了一条边界用例，防止以后又把项目级动作塞回方案里。
2. **点名落到具体照片**：名单展开后按原因分组，逐组列出在册文件名；原因的说法沿用 AI 面板、两处审阅抽屉、照片面板共用的那一套词表，不另造一套翻译。回执里的文件名与编号由后端用例钉住：名字必须对得上库里这条记录，否则界面会指错照片。
3. **修不了的不再被推销**：只有确实"能只读原图重算"的那批才出现"一次修好"。坏的全是原图丢了或换过的，就只把原因和下一步说清楚，不摆一个修不了它的按钮。
4. **修完再数一遍**：一次修完之后自动再只读清点一次，状态条说的是现在的文件状态，不把动手之前那一遍的数字留着冒充现状；"已修好几张、还剩几张、几份口径跟着重算"仍按修复回执单独报。
5. **两处"只少不多报"写在脸上**：名单最多列前 50 张，其余明说"都算在上面的总数里"；清点本身有扫描上限，没数到的那部分明说"不算成好的"。一张都不用修时给一句实话，不摆空按钮。
6. **网页预览与只读共享不摆本地文件入口**；画布视图讲的是画布的事，这条项目文件状态不挤进去。四套语言各补 8 句（含清点专用结局的说法）。

### 验证情况

- 前端四门以真实退出码复核：类型检查 0、全量测试 0（76 个测试文件 619 条用例全部通过）、eslint 0、接口契约检查 0。本轮新增 13 条、移除 3 条搬走的旧用例：图库常驻与视图/权限边界 3 条，状态条本身 10 条（数字来自后端且原因沿用同一套词表、名单落到文件名、干净时给实话不推销、修不了的不摆按钮、修完报清三笔计数并按修完之后的数字说话、名单截断说明、扫描上限说明、失败只说错误不编数字、等回执期间挡住重复点击、修完再数一遍不留旧话），另加方案弹窗不再重复项目级入口 1 条、回执缺文件名一律拒收 1 条。
- 后端本轮只补契约断言、未改产品代码：点名回执里的文件名必须与库里这条记录的编号对得上（指错照片就是事故）。五条对着真实数据库与真实缓存文件的用例全部通过；格式检查 0 差异、严格风格检查（告警即失败）退出码 0、全量测试退出码 0（40 个测试套件全部为 ok、0 失败），均以真实退出码复核。
- 四套语言密钥总数仍一致（各 1029 条，逐键比对无缺无多）；原始照片一张没动，没有生产模型调用，没有库结构改动与新迁移。

### 仍没做到

1. **没装机、没提交**：这批与前几批都还在这棵工作树里；装机需要先点头，发布线不碰。界面在真 App 里还没点过。
2. **检视器仍是各说各的**：照片面板那一处只讲当前这张，还没纳入同一处状态视图（下一步第 ② 项）。
3. **一轮 200 张、名单前 50 张、扫描上限、口径重算只覆盖最近 20 份方案**：这些边界都如实报数，但没有自动续跑到完。
4. **清点不比对原图内容指纹**（故意的开销取舍）：外部改过原图的那几张在清点里算"能用"，真重算时才逐张拦下。
5. **逐张口径仍只覆盖回执扫过的前 12 张**；画布侧口径仍未接（要先动后端，见第十七批第 4 条）。

下一步顺序：①装机（需要先点头）；②把检视器纳入同一处状态视图；③摄影知识与引用的"为什么算这条"原因码上屏；④修完的口径重算覆盖超出 20 份的老方案；⑤无指纹旧缓存行的回填；⑥逐张口径放开 12 条上限（回执有 64KB 长度门槛，得连投影查询一起改）；⑦画布侧口径与重算（要先动后端）。

## 2026-11-07 Focal AI 第二十批：照片面板先说清这张能不能读——不再对着好文件推销重算

第十九批下一步顺序里的第 ② 项本轮闭环（第 ① 项装机仍等点头）。

### 做了什么

1. **多了一步只读体检**：照片面板新增"检查这张的本地文件"，只读不改一个字节，回答的是与图库状态条、与真动手重算时同一道门的结论：能读 / 读不出来可重算 / 原图不在 / 原图换过 / 原图也读不出来 / 缓存位置可疑。
2. **修不了的不再摆按钮**：只有体检确认"读不出来、可只读原图重算"时才出现重算按钮。原图丢了或换过的，只把原因与下一步说清楚。旧版是无条件摆着按钮，好文件也被推销一遍。
3. **预览图加载失败就自动体检一次**：面板里那张预览图读不出来时自动走一遍只读检查，用户不用先猜再点。
4. **修完按现在的状态改口**：重算之后再体检一次，状态行说的是现在读不读得出来；重算回执单独一行，两件事不混成一句。体检这一步失败也只说"没能完成、本地文件没有改动"，不谎报成能读。
5. **只读结论不再冒充"已经重算过"**：整项目清点原先把"该重算哪一档"填进了"已经重算了哪一档"那一栏，本轮改成只读结论一律为空，并由后端用例钉住。
6. 四套语言各补 3 句；网页预览与只读共享仍不摆本地文件入口。

### 验证情况

- 前端四门以真实退出码复核：类型检查 0、全量测试 0（76 个测试文件 625 条用例全部通过）、eslint 0、接口契约检查 0。本轮新增 6 条：能读就不摆按钮、检查出坏了才摆按钮且修完按现状改口、原图不在只说原因、预览加载失败自动体检、体检失败不说成能读、单张体检回执缺字段或脏值一律拒收。
- 后端新增 1 条对着真实缓存文件的用例：体检与重算同一套说法、体检一个字节都不写（坏文件原样留着）、只读结论的"已重算档位"一律为空（单张体检与整项目清点都验）、原图换过与原图不在都按重算时的口径回答、好的那一档不动。格式检查 0 差异、严格风格检查（告警即失败）0、全量测试 0（40 个测试套件全部 ok、0 失败），均以真实退出码复核；只格式化本轮自己动过的三个文件，没碰别人的在改文件。
- 四套语言逐键比对无缺无多（各 1032 条）；原始照片一张没动，没有生产模型调用，没有库结构改动与新迁移。

### 仍没做到

1. **没装机、没提交**：这批与前几批都还在这棵工作树里；装机需要先点头，发布线不碰。两个新入口在真 App 里都还没点过。
2. **单张体检要算一次原图指纹**：这是为了与动手时同一道门，代价是每张多读一遍原图；整项目清点仍故意不算指纹，所以两处对"外部改过原图"的口径不同（清点只少不多报）。
3. **一轮 200 张、名单前 50 张、扫描上限、口径重算只覆盖最近 20 份方案**：边界都如实报数，没有自动续跑到完。
4. **逐张口径仍只覆盖回执扫过的前 12 张**；无指纹的旧缓存行仍未回填。
5. **画布侧口径与重算仍未接**（要先动后端，见第十七批第 4 条）。

下一步顺序：①装机（需要先点头）；②摄影知识与引用的"为什么算这条"原因码上屏；③修完的口径重算覆盖超出 20 份的老方案；④无指纹旧缓存行的回填；⑤逐张口径放开 12 条上限（回执有 64KB 长度门槛，得连投影查询一起改）；⑥画布侧口径与重算（要先动后端）。

## 2026-11-07 Focal AI 第二十一批：知识引用说清"为什么算这条"——命中词、命中处、只带回前几条

第二十批下一步顺序里的第 ② 项本轮闭环（第 ① 项装机仍等点头）。

### 做了什么

1. **本机检索的挑选过程不再被丢掉**：原来按命中词数排序，但分数和命中了哪些词一律不带回，界面上只剩标题与正文。现在每条本机方法都带上命中词、命中词数、命中在标题/标签/正文的哪一处；报出来的词数就是排序用的那个数，由后端用例钉住两者一致。
2. **只带回前几条这件事看得见**：命中总数与实际带回条数分开报，界面上明说"一共命中几条、只带回前几条"；命中词最多列八个，多出来的明说只列了前几个。
3. **三种来路分开说，不混成一句"相关资料"**：词面命中、点名取用的产品自带方法、云端知识服务检索命中各有一套说法。云端的相关性原样透传，服务端没给分数就明说没给，本机不重算也不补一个自己编的数。
4. **顺手修掉一处会指错来源的缺陷**：通用知识检索会把云端摘要条目一起带回，那些条目只有实体编号、没有方法编号与正文；原先两种都按方法编号入表，云端那几条会挤成同一条无名引用。现在各按自己的编号入表，并明说"只有摘要、没有断言正文"，与判断引用那边"摘要不能当断言出处"的口径对齐。
5. **两处引用面都接上**：参考来源抽屉与单张判断的方法出处共用同一套说法；每条带理由的引用后面跟一句实话——命中只解释怎么被挑出来，不代表它与这张照片有关，也不代表它正确。老回执没带理由的不替它编一句。
6. 四套语言各补 14 句；回执自相矛盾（列出的词比总数多、带回条数超过命中总数、相关性是文字、陌生来路）一律整份拒收。

### 验证情况

- 前端四门以真实退出码复核：类型检查 0、全量测试 0（76 个测试文件 629 条用例全部通过）、eslint 0、接口契约检查 0。本轮新增 4 条：抽屉里三种来路与截断说明各自上屏且老回执不编理由、判断方法处带理由与那句实话、三种理由原样过界而八种脏值被拒、只有摘要的标记必须是布尔值。
- 后端新增 3 条：本机命中词数与实际排序一致（含命中总数/带回条数、没命中报零、点名取用另算一种）、云端相关性给了就带没给就说没给（本地 HTTP 夹具，不碰生产）、参考来源投影里两条云端摘要不再合并成一条无名引用且各自标明只有摘要（真实数据库）。格式检查 0 差异、严格风格检查（告警即失败）0、全量测试 0（40 个测试套件全部 ok、0 失败），均以真实退出码复核；只格式化本轮自己动过的五个文件。
- 四套语言逐键比对无缺无多（各 1045 条）；没有生产模型调用，没有库结构改动与新迁移，原始照片一张没动。

### 仍没做到

1. **没装机、没提交**：这批与前几批都还在这棵工作树里；装机需要先点头。抽屉里的新说明在真 App 里还没看过。
2. **本机检索仍是词面命中**：这批只把已有的挑选过程如实说出来，没有改进检索本身（没有语义匹配、没有流派权重、没有同义词表），所以"命中了但不对题"依然会发生。
3. **零命中不单独上屏**：抽屉只列真的带回的条目，一次完全没命中时界面上没有一条"这次没找到"的说明（后端已如实报零，缺的是前端入口）。
4. **云端相关性只是服务端给的数**：本机不核对它的算法，也不重排；不同工具之间的分数不可比。
5. **前面记下的老边界未动**：口径重算仍只覆盖最近 20 份方案、逐张口径仍只覆盖前 12 张、无指纹旧缓存未回填、画布侧口径未接。

下一步顺序：①装机（需要先点头）；②零命中在界面上有一句话（后端已报零，补前端入口）；③修完的口径重算覆盖超出 20 份的老方案；④无指纹旧缓存行的回填；⑤逐张口径放开 12 条上限（回执有 64KB 长度门槛，得连投影查询一起改）；⑥画布侧口径与重算（要先动后端）。

## 2026-11-07 Focal AI 第二十二批：查过但空手而归，也在界面上留一句话

第二十一批下一步顺序里的第 ② 项本轮闭环（第 ① 项装机仍等点头）。

### 做了什么

1. **检索台账上屏**：原来"参考来源"抽屉只列真的带回的条目，一次完全没命中时界面上什么都没有，看起来像根本没查过。现在每次知识检索单独留一行：查的是哪个词、取到几条、云端答没答上来。一条参考来源都没有时，助手那一块也必须出现，否则台账无处可挂。
2. **四种结局分开说**：取到了 / 确实没有命中 / 云端这次没答上来（明说不能当成"没有资料"）/ 这次检索根本没跑成（附失败原因，并明说下面不是"没有资料"的结论）。只有通用知识检索才同时查本地札记与云端，另外三种工具没有本地语料，不替它们说"本地也没有"；点名取用自带方法那次只查本地，也明说"这次没查云端"。
3. **问过的词跟着回执一起存**：回执表只留参数指纹、不留参数本身，事后没法从库里还原查的是哪个词，所以问法随结果一起写进不可变回执（限长 200 字；摄影书取书名、系列比较取"A / B"）。失败回执没留下问法时界面明说"这次没留下查的是哪个词"，不编一个。
4. **本地命中数与云端条数分开报**：云端给了两条，不能替本地札记说"这个词有命中"；一次系列比较里只有一边没取到时，照实报取到几条，出错的那一边单独讲，不整次说成失败。
5. **台账末尾跟一句实话**：只说明查过什么、取到几条，不代表已核实的摄影史断言，也不代表与照片有关。
6. 四套语言各补 17 句；回执结论与数字对不上（说取到了却是零条、说没有资料却掩盖云端没答上来、说没跑成却不带原因、跑成了的回执却带着失败原因、陌生工具名、负数条数、超长问法、空错误编号）一律整份投影拒收。

### 验证情况

- 前端四门以真实退出码复核：类型检查 0、全量测试 0（76 个测试文件 632 条用例全部通过）、eslint 0、接口契约检查 0。本轮新增 3 条：抽屉里六种情形各自上屏且没有参考来源时也出现（含"摄影师检索没有本地语料"那句单独的说法）、五种结局原样过界而十三种脏值被拒、旧任务没带台账时补空数组。
- 后端新增 2 条：真实数据库里六次检索按发生顺序留档，四种结局与条数各自正确，云端没答上不等于整次失败、比较只坏一边照实报一条，三次空手而归在参考来源里一条都没有（台账是唯一能看见它们的地方），非知识工具不进台账；另一条钉住问法随回执保存（零命中、云端没授权、本地云端混合时本地命中数单独报、书名、系列比较、超长截断到 200 字）。格式检查 0 差异、严格风格检查（告警即失败）0、全量测试 0（40 个测试套件全部 ok、0 失败），均以真实退出码复核；只格式化本轮自己动过的六个文件。
- 四套语言逐键比对无缺无多（各 1062 条）；没有生产模型调用，没有库结构改动与新迁移，原始照片一张没动。

### 仍没做到

1. **没装机、没提交**：这批与前几批都还在这棵工作树里；装机需要先点头。台账在真 App 里还没看过。
2. **台账只覆盖最近 100 次知识检索**（与参考来源同一上限），更早的不再列出，也没有分页或"还有多少没列"的提示。
3. **失败回执仍不带问法**：这次没跑成时只能说失败原因，说不出查的是哪个词；要补就得改失败回执的形状，本轮没动。这批之前留在库里的老回执同样没有问法字段，界面上只能说"这次没留下"。
4. **检索本身仍是词面命中**：台账只是如实报告查过什么，不会让检索更准（没有语义匹配、没有同义词表），"命中了但不对题"依然会发生。
5. **云端"答上来了"只代表服务返回了列表**：本机不核对服务端是否真的搜全，返回空列表就按"没有命中"说。
6. **前面记下的老边界未动**：口径重算仍只覆盖最近 20 份方案、逐张口径仍只覆盖前 12 张、无指纹旧缓存未回填、画布侧口径未接。

下一步顺序：①装机（需要先点头）；②修完的口径重算覆盖超出 20 份的老方案；③无指纹旧缓存行的回填；④逐张口径放开 12 条上限（回执有 64KB 长度门槛，得连投影查询一起改）；⑤画布侧口径与重算（要先动后端）。

## 2026-11-07 Focal AI 第二十三批：修完之后，老方案的口径也算得上，还欠多少说得清

第二十二批下一步顺序里的第 ② 项本轮闭环（第 ① 项装机仍等点头）。

### 做了什么

1. **欠账按库里的事实点名，不再截断成"最近 20 份"**：判据只有两条——投影读不到这份方案的口径，或者读到的那条记着另一个方案版本号。修完预览后一轮最多补 200 份，更早的老方案同样进名单；原来那 20 份之外的方案修完仍旧口径过期，而回执里看不出还欠着多少，界面上那句"已重算"会被当成全都跟上了。
2. **报清这一轮之后还差多少**：项目一共多少份方案、这次点到多少份、点名时欠多少、补成了几份、几份没算成、还欠多少。没算成的仍旧算欠着，不报成跟上了。
3. **接着算不用重新修图**：新增一条只重算口径的通道与图库侧的按钮，只在确实还欠着（或上一趟口径没跑成）时出现。点一次只算一轮，数字照实更新；欠账清零后按钮自己消失。这条通道不碰照片文件、不问模型、不上传，也不占项目修订号。
4. **都跟上之后不重复算**：再点一轮时欠账为零就不写新回执（测试钉住回执条数不变）。本地改过方案版本号，下一轮重新欠一次——判据看的是事实，不是"上轮算过没有"。
5. **修好文件与口径没跑成分开说**：口径那一趟失败时仍旧报"修好几张"，另附一句失败原因，并保留接着再算的入口，不把已经修好的事实一起吞掉。
6. **顺手修掉一个会让重算白点的顺序问题**：投影原来取同一个任务**最早** 50 条评估回执，而重算会不断往同一个任务追加回执，攒过 50 条之后刚算出来的那条被挡在窗口外——用户点了重算，界面上的口径一动不动。改成取最新 50 条再按时间顺序过一遍，每份方案留下的仍旧是它最新那条。
7. **点名规模也设上限并如实说**：方案 2000 份、回执 5000 条。只点到一部分时明说"没点到的不算成跟上了"，"都跟上了"那句只针对点到的那批，不拿它冒充全项目。
8. 四套语言各补 6 句；欠账的几个数字必须自洽（还差的 = 欠的 − 补成的、补成的加没算成的不超过欠的、点到的不超过总数、欠的不超过点到的、非负整数），对不上就整份拒收；失败原因没有编号也拒收。

### 验证情况

- 前端四门以真实退出码复核：类型检查 0、全量测试 0（76 个测试文件 640 条用例全部通过）、eslint 0、接口契约检查 0。本轮新增 8 条：图库侧 6 条（欠账上屏且接着算只重算口径、不重新修图；没算成的仍留在欠账里并保留按钮；只点到一部分时"跟上的是点到那批"与"没点到的另说"两句分开；文件修好但口径没跑成时两件事分开说且能接着再算；仓库没有这个能力就不摆一个点不动的按钮；重新数一遍放下上一轮欠账），仓库侧 2 条（修完那一轮的欠账跟着点名一起过界、自相矛盾的数字与空编号拒收；接着算走自己的命令、七种脏值整份拒收）。
- 后端新增 6 条：一轮覆盖 25 份方案（远超原来的 20）且投影真能读到 25 份口径、再来一轮欠账为零不写新回执；一轮只算 5 份时如实报还欠 20 份、下一轮从剩下的 20 份接着算；本地改过版本号重新欠一次；点名只限本项目（别的项目的方案不被顺手算、库外的项目拒绝且不留回执）；同一任务攒到 51 条回执后最新那条仍旧上屏（钉住第 6 项的顺序修正）；修复通道端到端覆盖 25 份老方案、文件都好了的一轮不再重算。
- 后端四门以真实退出码复核：编译检查（含测试目标）0、格式检查 0 差异、严格风格检查（告警即失败）0、全量测试 0（314 条单元测试与各集成套件全部 ok、0 失败）；只格式化本轮自己动过的两个文件。
- 四套语言逐键比对无缺无多（各 1068 条），占位符逐键一致；没有生产模型调用，没有库结构改动与新迁移，原始照片一张没动。

### 仍没做到

1. **没装机、没提交**：这批与前几批都还在这棵工作树里；装机需要先点头。新按钮在真 App 里还没点过。
2. **一轮 200 份仍要人再点一次**：修图一轮 200 张、名单前 50 张、口径一轮 200 份这些边界都没有自动续跑到完，只是欠账数现在如实报、也能接着点。
3. **方案超过 2000 份的项目有死角**：点名按更新时间取前 2000 份，每轮取到的都是同一批，剩下的那批本轮进不了名单，也没有游标翻页。界面上明说"没点到的不算成跟上了"，但这不等于覆盖了全项目。
4. **回执超过 64KB 的方案会反复重算**：投影对这种回执整条不投，本轮的欠账判据与投影同一道门，于是它每轮都被算成欠着。要根治得连那道门槛一起改（下一步第 ③ 项），本轮没动。
5. **逐张口径仍只覆盖前 12 张**、无指纹旧缓存未回填、画布侧口径未接（画布草案不在方案表里，这条通道点名不到它们）。
6. **重算不会让观察本身变多**：它只是把库里存着的画面事实与现在的预览再比一遍。从来没观察过的照片仍旧没有画面记录，修好预览也补不出一次没问过的观察。

下一步顺序：①装机（需要先点头）；②无指纹旧缓存行的回填；③逐张口径放开 12 条上限（回执有 64KB 长度门槛，得连投影查询一起改，同时解掉上面第 4 条）；④画布侧口径与重算（要先动后端）；⑤方案超过 2000 份时的点名游标。

## 2026-11-07 Focal AI 第二十四批：当年没记"像素版本"的历史观察，点名与只补能证明的那部分

第二十三批下一步顺序里的第 ② 项本轮闭环（第 ① 项装机仍等点头）。

### 做了什么

1. **给那批旧观察做了一份只读点名单**：加"像素版本"那一列之前留下的观察没记当初交给模型的是哪一份字节，口径便只能把它们报成"版本未记录"、一份方案的画面依据用不上它们。点名比三件本地事实——问法档位今天还认不认得（版本串写成"<档位>@<端点>"，形状不全、缺端点部分也拦下）、这条记录里的原图指纹与库里在册的是不是同一份、从原图整张重算出来的这一档字节与磁盘上那份预览是不是逐字节相同。三条同时成立才算"能证明"，少一条就留在补不了里并给原因码（换过问法档位、原图改过、原图不在、预览读不出、重算压不出、原图读不出、缓存对不上）。
2. **补写只写证明成立的那一份指纹，不写标记也不猜**：写进去的值和口径那道门用的是同一个算法，所以补完之后那道门自己就认了；库里那一列早就在，只是旧行留空——没动库结构，也没新迁移。
3. **补完强制重算收着这些照片的方案**：新加一条"按照片名单点方案"的重算通道。原来那条按欠账点名的通道看不见它们——补指纹不改方案的版本号，欠账判据以为一切都跟得上，界面上的数字就会停在"版本没记过"的那一晚。名单为空就不算，不写新回执。
4. **规模都设了上限并如实说**：一轮最多补 500 条、点名最多点 5000 条、按名单最多重算 2000 份方案；没点到的另说一句"不算成补不了"，不混进补不了那一堆。
5. **界面上在图库一侧多一条常驻**：一条按钮数一下还有几条没记，另一条把能证明的补上——一条都补不了时不摆这个按钮。补完再数一遍，状态条按补完之后那一遍的数字说话；补了几条、留了几条、几份口径跟上、几份没算成、指纹写好了但口径这趟没跑成，分开口径报。这一条与预览状态那条共用同一套图库侧样式，窄屏一起收窄边距。
6. **全程本地、不花钱、原图只读**：不上传照片、不问模型、不占项目修订号、不新增任务、不写模型用量（测试钉住这三样在补写前后不变）。重算只在随机命名的临时目录里做，用完删掉，图库缓存与原图一个字节不动。
7. **数字自洽与文案一处不猜**：四套语言各补 23 句（各 1091 条，占位符逐键一致）；能证明＋补不了＝点到数、点到的≤总数、涉及张数≤点到条数、原因计数之和＝补不了、补上的≤能证明且剩下的＝能证明−补上的，对不上整份拒收；后端给了这边还没译的结局码就原样露出，界面不编一句好听的说法。

### 验证情况

- 后端新增 7 条：点名单把能证明与补不了的分开、原因码各归各；补下去的就是现在这一档还能产出的字节，补完那道门自己判成"已核对"；磁盘上那份缓存不是这一档做出来的按"缓存对不上"拒绝、不猜；一轮补不完如实报还欠几条、下一轮把剩下的补完；端到端——补之前那份方案的画面依据把这条记成"版本未记录"而不采用，补之后同一份方案把它算进看过的那几张，且没有新任务、没改项目修订号、没新增模型用量；点名只限本项目，别的项目不被顺手算；档位前缀不认识时先拦下，不去读文件。
- 前端新增 20 条：图库侧 15 条（数字来自后端、原因分开、名单落到具体哪几条；一条都没记过时说一句实话且不摆补写按钮；全都补不了就不推销做不到的动作；补完报清补了几条、留几条、几份口径重算，状态条改按补完之后那一遍；重新数一遍就放下上一轮结果；一轮补不完说清还欠几条、不把没轮到的算成补不了；口径没算成的单独说；版本号补上了但口径那趟没跑成分开说；只点到一部分时不把没点到的说成补不了；名单截断说清其余算在总数里；没译的结局码原样露出；点名失败只说错误不编数字；等回执期间挡住重复点击）；仓库侧 3 条（两条命令各走各的、九种脏值整份拒收；补的数目不能多于能证明、剩下几条要对得上、缺了就当没补；版本号补上与口径没跑成分开收、失败原因没编号也拒收）；挂载 2 条（与本地预览状态同在图库一侧，网格和序列都常驻；网页预览、只读共享、画布视图不出现，老仓库没这个能力时不多摆一条）。
- 前端四门以真实退出码复核：类型检查 0、全量测试 0（77 个测试文件 660 条用例全部通过）、eslint 0、接口契约检查 0。后端四门：编译检查（含测试目标）0、全量测试 0（321 条单元测试与各集成套件共 561 条全部 ok、0 失败）、严格风格检查（告警即失败）0、格式检查 0 差异（只格式化本轮自己新建的那一个文件）。
- 没有生产模型调用，没有库结构改动与新迁移，原始照片一张没动。

### 仍没做到

1. **没装机、没提交**：这批与前几批都还在这棵工作树里；装机需要先点头。新面板在真 App 里还没点过一次，真实图库里到底有多少条能补、多少条补不了，本机没数过。
2. **"档位字符串换流程时跟着换过"是约定，不是证据**：当年若改过缩放长边或质量却没换版本号，本地无从察觉，会被当成同一档字节而补上指纹。界面上那句"能证明"就建立在这个约定上。
3. **只补版本，不重问模型**：从来没观察过的照片仍旧没有画面记录，这条通道补不出一次没问过的观察；补上版本的那条用的仍是当年按那时问法答出来的内容，不叫重新看过。
4. **一轮补不完要人再点一次**：500 条、5000 条、2000 份这三个边界都没有自动续跑，只是欠账如实报、也能接着点。方案超过 2000 份时按更新时间取前 2000 份，剩下的每轮都进不了名单（沿用上一批第 3 条）。
5. **回执超过 64KB 的方案仍旧每轮被算成欠着**：投影对这种回执整条不投，这条通道同受那道门槛限制。要根治得连门槛一起改（下一步第 ② 项）。
6. **逐张口径仍只覆盖前 12 张**；画布侧口径没接——画布草案不在方案表里，这条按照片点名方案的通道也点不到它们。

下一步顺序：①装机（需要先点头）；②逐张口径放开 12 条上限（连投影 64KB 门槛一起改，顺带解掉上面第 5 条）；③画布侧口径与重算（要先动后端）；④方案超过 2000 份时的点名游标。

## 2026-11-07 Focal AI 第二十五批：逐张口径不再只点到前 12 张，补齐前先跟回执对账

上一批下一步顺序里的第 ② 项做了一半：12 张上限放开了，那道 64KB 门槛本轮没动（原因写在"仍没做到"第 1 条）。

### 做了什么

- 方案审核抽屉里"这张按哪一档问法看的、那张为什么没用上"从前只覆盖回执带得动的前 12 张，第 13 张往后的照片即便有可核对的画面记录，也只能算进总数、说不出自己的来历。现在整份方案都能逐张说到，一次最多 100 张（与界面数据契约里那个 100 是同一个数），超出仍旧由原来那句"逐张口径只点到上面列出的那几张"如实交代。
- 没有把回执撑大：回执那一份是模型看得见的，为了界面把它撑到几十倍会直接挤掉对话预算（三百张画布的测试离上限只剩几百字节）。改成界面投影自己按同一套优先级把缺的那几张补出来，只补"哪一档问法"这一件事。
- 挑观察的优先级（当前页面档 > 大尺寸细节档 > 同工具的其他版本 > 其他工具，同级取最新一条）从评估里抽成一处，评估与界面共用：两处各写一遍时，界面上说的档位和真正拿去比对的档位可能不是同一把尺子。
- 补之前先对账——回执点到的每一张，今天在库里还能按同一档挑出来，并且"库里有画面记录的张数"要正好等于回执说的"用上的"加"排除的"。任何一条对不上（档位名字不同、库里多了一条后来才有的记录、名单里同一张出现两次），一张都不补，逐张口径停在回执自己点到的那几张：宁可少说，也不能把一张没核对过的照片说成按某档问过。
- 补出来的那张不会盖掉回执说"排除"的那张：被排除的仍旧带着自己的原因（原图换过、预览读不出、版本没记过）出现在界面上。
- 全程本地：不上传照片、不问模型、不花额度，没有新迁移、没改库结构，原始照片只读。

### 验证情况

- 后端新增 3 条（该模块从 2 条变 5 条）：一条证明整份方案补齐，且第十三位之后用的就是评估那套优先级（同一张照片同时存着两种问法时，界面说它是页面档问的）；两条证明对不上账时停在回执点到的那几张（档位名字对不上、库里比回执多一条记录）；一条走真正的本地核验通道写回执、再接界面投影——键名或档位名字一旦对不上，这条就会失败，而不是让补齐悄悄失灵。原有那两条口径投影用例一字未改。
- 前端新增 1 条：14 张的方案里第 13、14 张各自带逐张说明，且不再出现"其余只算进总数"。全量测试 0（77 个测试文件 661 条用例全部通过）、类型检查 0、eslint 0、接口契约检查 0。
- 后端四门以真实退出码复核：桌面特性编译检查（含测试目标）0 且 0 警告、全量测试 0（324 条单元测试与各集成套件共 564 条全部 ok、0 失败）、严格风格检查（告警即失败）0、格式检查 0 差异（只格式化本轮改动的两个文件）。
- 本轮没有新增任何用户可见文案，四套语言的条目数与占位符保持一致；没有生产模型调用。

### 仍没做到

1. **那道 64KB 门槛本轮没动**：回执大到装不下时投影仍旧整条不投，那份方案在界面上什么都不说；上一批写的"连门槛一起改"没做到。要根治得先把回执里两处不计条数的内容（相邻相似组、完全重复组）收住，而那两处是模型看得见的内容，属于另一件事，本轮不顺手改。
2. **没装机、没提交**：本轮改的逐张口径在真 App 里还没点开过一份四十张以上的方案看效果。
3. **补齐只说"哪一档"，不重做像素核对**：投影每次加载都为几百张照片重压一遍预览不划算，所以"这张原图换过、那张预览读不出"这类结论仍旧只来自回执本身，库里的新状态不会改口。
4. **一次最多 100 张**：三百张画布的方案仍旧只有前 100 张有逐张说法。这个上限就是界面数据契约里的上限，抬它要连数据契约与界面一起动。
5. 画布侧口径仍旧没接（要先动后端）。

下一步顺序：①装机（需要先点头）；②回执体积与投影那道门槛（先收住相邻相似、完全重复两处的条数并各自留一个总数，再谈放开门槛）；③画布侧口径与重算；④方案超过 2000 份时的点名游标。

## 2026-11-07 Focal AI 第二十六批：核验单大到读不回来时，界面与重算两张嘴不再各说一套

**做了什么**
- 上一批那条"没做到"就是这道长度门。现在把一处规矩拆成两条，各管各的：判欠账只要"这份方案最新的口径记着哪个版本号"，那几个字在库里就地取，整条核验单一次也不搬回内存，因此不看体积脸色；界面要的是计数与逐张名单，得整条读回来，于是单条上限从 64KB 放到 512KB，一个任务再兜一个 2MB 的总量，装不下的整条不读——读不到就什么都不说，绝不拿半截数据编一段口径。
- 投影与重算原先各写一遍筛法（各自一段查询、各自一份"什么才算画面口径"的判断），现在都走同一个读取入口，两份嘴对同一批回执只能给一个答案。
- 顺手修掉一处白干：投影本来就只留每份方案最新的那条回执，却把 50 条整回执全搬回内存再扔掉四十九份。改成先在库里按编号和体积挑（一份方案只挑它最新那条），再只把挑中的几条读回来。最坏内存从"50 条 × 64KB ≈ 3.2MB/任务"收到 2MB/任务，同时单条能看见的大小放宽了八倍。
- 库里存的是文本，早年的行、写了一半的行都可能压根不是 JSON；SQL 里每次就地取值都先过一道有效性判断，一条坏行不能把整个项目的加载问倒。
- 全程本地、免费、不碰模型，原图只读；界面数据契约与文案一个字没动。
- 为什么没直接把那份核验单收小：它同时是模型读的那一份，三百张画布的预算只剩几百字节余量；而撑大它的"相邻相似""完全重复"两份不计条数清单还喂着一条本地兜底去重，收它的条数会改变选片结果。所以这一批只动"读"的这一侧。

**验证情况**
- 后端：新增读取入口自带 4 条测试（每份方案只留最新可读那条、总预算用完就停、体积再大也报得出对着哪一版、坏行与别种工具的结果都不当口径），投影侧 +1 条（垫到约 90KB 的一份方案，头上的计数和五张逐张说法全都投得出来），重算侧 +1 条（大到读不回来的那条不再每轮被算成欠账、不再留多余回执，界面对它照实说没跟上）。单元测试 324 → 330，Rust 全量 570 条通过。
- 前端 77 个文件 / 661 条通过；类型、规范、命令契约、Rust 检查、clippy、格式（含全仓 `cargo fmt --check`）各条退出码均为 0。

**仍没做到**
1. 单条超过 512KB 的方案（大约五百张以上的画布）界面仍旧什么都不说。要它真的小下来，得先收住那两份不计条数的清单，而它们喂着一条本地兜底去重——收条数会改选片结果，这批没动。
2. 判欠账与界面之间还剩一种不一致：那条"记着第 2 版但太大读不回来"的方案不再被算成欠账（不再白跑），但界面上看到的仍是它上一条读得回来的口径，并照实标着"对着第 1 版算的"。要两者完全一致，仍旧回到第 1 条。
3. 一个任务最多仍只扫最近 50 条核验单、整个项目最多 5000 条；超出就少说，不假装看完了。
4. 没装机、没提交。
5. 画布侧口径仍旧没接（要先动后端）。

下一步顺序：①装机（需要先点头）；②把那份核验单本身收小（两份清单各自收条数并各留一个总数，同时保住本地兜底去重的选片结果）；③画布侧口径与重算；④方案超过 2000 份时的点名游标。

## 2026-11-08 Focal AI 第二十七批：把那张核验单本身收小，一条相似对都不少

**做了什么**
- 上一批留的那句"要根治得先收住两份不计条数的清单"，这一批动了，但动的不是条数。撑体积的是每一对身上的赘述：它原先把共用事实的文字抄一遍，还把两边口径名各写一遍——哪怕两边本来就是同一档问法。现在每一对只留四样：位置、哪两张、共用了几条事实、是不是同一口径；口径相同时不再重复点名，只有跨口径才把两边各自是哪档写出来。
- **条数一条都没收**，这是故意偏离上一批写的方案。那条本地兜底去重读的就是这份名单里的照片编号，少列一对就少排除一张，最后选出的片子会跟着变；选片结果不该因为省了几KB 而换。所以省只省在文字上。
- 新的一条最坏情况回归把这两头同时锁住：一百二十张、每对都够像、于是第一百一十九对全都上榜。它逐对核对照片编号（名单必须按方案顺序完整、一对不能少），再锁体积上限（整条要能在旧那道 64KB 门里读回来，每一对平均不超过 260 字节）。同一份方案换个写法实测从 38,334 字节降到 24,173 字节，约省三分之一。
- 顺手改掉一处自己写出来的过头话：回执里那句说明原先写"事实文字在逐张那一份里"，可逐张只带前十二张、每张最多三条——第一百一十九对里的第六十对在回执里根本没有正文。现在这句话照实说清哪些对子只有条数、要读文字得回逐张口径或重新问，并加了一条断言盯着这句说明别再退回去。
- 全程本地、免费、不碰模型，原图只读；界面数据契约与前端文案一个字没动（界面本来就不读这份清单的每一条）。

**验证情况**
- 后端：单元测试 330 → 331（新增那条最坏情况回归）；一条既有集成用例的断言跟着换成新键名（共用几条、同一口径、同口径时不点名）；上一批那条"大到读不回来"的体积夹具照新的清单形状重垫，不再是自造键名。全量 571 条 Rust 用例通过、0 失败。
- 那道本地兜底去重的输入被完整核对过：新回归逐对验证了一百一十九对的照片编号，读的就是去重读的同一份名单，名单没变，选片结果就不会变。
- 前端 77 个文件 / 661 条通过；类型、规范、命令契约、Rust 检查、clippy、格式（全仓 `cargo fmt --check` 与只碰本轮三个文件的 rustfmt）各条退出码均为 0。

**仍没做到**
1. **体积仍随照片数线性增长**：每一对约 200 字节，三百张全像的方案就是六十KB 上下——比原先小了三分之一，但没有变成常数。真要压成常数，只能收条数，而收条数会改选片结果。
2. 界面看不到相似对的事实文字了：以前那一份里能读到共用的是哪几条，现在只剩条数。这是这一批用可读性换体积的代价，逐张口径那一份仍旧带文字。
3. 单条超过 512KB 仍旧整条不投——只是要顶到那么大约需两千五百张全都互为相似，比上一批远得多，但没有消失。
4. 逐张正文的上限（12 张）本轮没动，前 12 张之外没有正文这件事照旧。
5. 没装机、没提交；画布侧口径仍旧没接。（本条原先那句判断写过头了，更正见第二十八批：画布那边那段只看文件名的 `evaluate_sequence` 其实挂在扩展分发口，而模型名字进来先做规范化、落的是真核验单那条路，所以它并没有构成"另一份口径"，界面看不到画布口径的原因是投影只有选片草案这一条通道。）

下一步顺序：①装机（需要先点头）；②画布侧口径上屏（要给画布草案接的是投影与界面通道，不是再写一份评估）；③方案超过 2000 份时的点名游标。

## 2026-11-08 Focal AI 第二十八批：同名两套评估只剩一张嘴，并更正上一批写过头的那句

**做了什么**
- 上一批结尾我写"画布那边有一份 `evaluate_sequence`，只看文件名，读取入口不认它是画面依据"。这句得更正：查证调用路径后，模型名字进工具分发前会先做规范化（`evaluate_sequence` 在接口契约里就是 `evaluate_draft` 的旧别名），所以真跑起来走的是那一份带像素核对的核验单；那段只看文件名的分支挂在画布扩展分发口里，当前没有一条生产路径能把这个名字送到那儿。它不是一嘴在说的另一套口径，而是一段够不着、却顶着"评估"名字的死分支——一旦哪天有个新入口图省事直接调扩展口，就会以结构化评估的名义报出没看过画面的结论。
- 本轮把那段死分支删掉，让这个名字只剩一条路。加了一条测试钉住两头：旧别名规范化后必须等于 `evaluate_draft`；扩展分发口遇到这个名字只能拒绝（权限类错误码 + "未知扩展工具"），不能返回一份看着像评估的结果。
- 顺带把我自己上一批那句过头话在路线图里就地更正，而不是悄悄改掉。
- 全程本地、不碰模型、原图只读；接口契约、界面数据契约与前端文案一字未动（别名与参数要求本来就在契约里）。

**验证情况**
- 画布那条新测试与原有 6 条一起通过；单元测试 331 → 332。上一批最终树上的全量重跑 571 条通过、0 失败（本轮删除与新增测试后另跑一遍，见下方退出码）。
- 本轮补跑并以真实退出码复核：Rust 全量 572 条通过、0 失败（含 40 个单元测试目标）退出码 0；严格风格检查（告警即失败）退出码 0；`cargo fmt --check` 退出码 0；只碰本轮两个文件的 rustfmt 检查退出码 0。前端四道门（类型、规范、接口契约、全量测试 77 个文件 661 条）退出码均为 0。
- 一处要照实说：两道格式门我第一次报绿灯时，树上还差最后一次编辑（那条"前 12 张"的断言换行格式没归齐），rustfmt 检查当场退回非零。已在本文件内单独格式化、重跑两道门并确认退出码 0，不拿第一次的绿灯交付。

**仍没做到**
1. 画布草案的口径仍旧上不了屏。但这一批把缺口位置查清了，比上一批写的窄：**不是后端缺一套评估**。画布那份序列草案本来就落在 `drafts`/`draft_items` 里（`create_sequence_draft` 复用现有草案系统），画布任务的工具清单也含 `evaluate_draft`，所以它一旦被评估就有一份真核验单、并同样进那个任务的口径投影。**这条"并同样进投影"是从代码读出来的判断，还没有一条端到端用例证明画布任务的那份回执真的投到了界面上**——下一批先补这条用例，再动面板；两头都别照着"要先动后端写一份画布评估"去做。
2. 那段分支删掉后，作品序列（项目级那份 `sequence_items`，不是草案）就没有任何结构化评估了——这是照实的状态：没看过像素就不假装有结论。要给它真依据，得让它也产一份带像素核对的核验单。
3. 没装机、没提交。

下一步顺序：①装机（需要先点头）；②画布侧口径面板（读同一份投影，不再新写评估；含四套语言文案）；③方案超过 2000 份时的点名游标。

## 2026-11-08 Focal AI 第二十九批：画布那份草案的画面依据真的走到了界面上，并把"从没算过"这种方案接了出来

**做了什么**
- 上一批留下的那句"从代码读出来的判断，还没有一条端到端用例"这一批补掉了。新用例四个环节全走真实通道，没有一处是手写的库里插行：画布任务先建属于自己的沙盒画布，再由画布那侧的序列草案工具生成草案（草案确实落在 `drafts`/`draft_items`，任务编号就是那个画布任务），口径由**本地核验通道**算出来并只往 `tool_calls` 追加一条回执，最后界面上读的是投影。断言盯着"三张都算在这份方案里、三张都有核对过像素的观察、档位计数与逐张名单三张都点到、口径名和图库侧同一套写法、版本号一起上屏"。任何一环其实是空的——比如画布那份草案压根进不了投影要读的那张表——这条用例就会红，而不是让"接上了"这句话自己圆过去。
- 查授权时把上一批写得太顺的另一处也校正了：画布任务的工具授权本来就带 `evaluate_draft`（那份基础清单是选片与画布共用的，画布只是往上再加自己的工具），会话里工具能不能被模型看到，看的是"这个任务名下有没有草案"，而画布生成的序列草案确实会在草案表里留一行——所以模型是被允许、也被告知可以评估画布草案的。这不是本轮改出来的，本轮只是把它钉成了用例。
- 真正缺的入口在界面上：那个"按现在这一版重算口径"的动作本来挂在"已经有一份口径"这段话里面。从没算过的方案就拿不到这段话，于是连入口也没有——画布那份草案最常落在这一格，用户看着一份判断，界面对它凭哪张画面说话一个字不提，也没有任何办法让它开口。本轮给这一格补了一个动作：先照实说"还没算过，下面的判断只靠评分、状态与顺序"，再摆一个"在本地算一次画面依据"。点下去走的还是那一条本地核验通道，不重问模型、不上传照片、不动原图；算完界面上的口径就是投影出来的那份数字，没有另存一份真相。
- 规矩照旧：仓库没有这个能力时**不给按钮**，也不写那句"还没算过"，宁可不提。四套语言各加三条词条，条目数与占位符一致；新增界面文案没有一条声称"已经看过画面"。
- 没做的事：没有为了画布再写一套评估，没有改画布那份提示词（换它会让所有旧画布任务的授权校验对不上，属于另一件事），没有动接口契约与界面数据契约的字段。

**验证情况**
- 后端 +1 条端到端用例（走真实通道，非夹具插行）；前端 +1 条（没算过的方案给入口并把动作打到正确的方案上），原有那条"没有核验回执时不编一段画面依据"一字未改且仍然通过——它恰好证明没有能力时既不编话也不给按钮。
- 这个入口第一次跑全量时把面板侧一条老用例碰红了，红得对：那条用例原来写的是"页面上不许出现'画面依据'这四个字"，而新入口正好会在"回执是隔壁方案的"这一格老实说一句"还没算过画面依据"。这一句没有借隔壁的数字，是本轮想让界面说的话，所以改的是断言而不是把话咽回去——改成盯住真正要防的东西：隔壁方案那 9 张的数字一张也不许出现，同时这一格要说"还没算过"、要给"在本地算一次"的按钮。改完面板那 75 条单独跑一遍全绿。
- 首轮全量前端有 6 条报"5 秒超时"，全部落在超时而不是断言上，且和后端整套用例并发跑在同一台机器上；这一批的结论以**串行重跑**的退出码为准（见下一行），不拿并发时的红绿当数。
- 本轮最终树上的九道门退出码（Rust 侧跑完之后再没动过后端一行，前端侧最后一次编辑之后重跑）：`cargo check --all-targets --features desktop` 0；`cargo fmt -- --check` 0；对新建/改动的 Rust 文件单独 `rustfmt --edition 2024` 通过；`cargo clippy --all-targets --all-features --features desktop -D warnings` 0；`cargo test --all-targets --no-fail-fast --features desktop` 0，**573 条通过、0 条失败**。前端 `npm run typecheck` 0、`npm run lint` 0、`npm run contracts:check` 0；`vitest` 全量这一趟是 **1**，红的是 2 条 5 秒超时（`src/test/App.test.tsx` 里"项目列表不显示上一个项目的功能"一条、`tools/agent-eval/evaluation.test.ts` 里 CLI 导入模拟评审一条），把这两个文件单独拎出来跑 **0，24 条全过**——即这两条用例本身就卡在超时线上，跟本轮改动无关，本轮改动碰到的文件（面板 75 条、评审弹窗 20 条）单独跑都是绿的。
- 上一趟全量还多出 4 条同类超时，原因是和后端整套用例并发跑在同一台机器上；把 Rust 和前端串行重跑之后超时从 6 条降到 2 条。这条经验已经写进门禁记忆：**Rust 全目标与 vitest 不许并发**，否则红绿都不可信。

**仍没做到**
1. 中间那一环（界面把某个任务的草案对上它那份口径）用的是选片侧已有的那条通用映射，本轮没有再为画布单独写一条界面用例：它靠的是"投影里画布任务的形状和选片任务一样"，这一点由本轮后端用例证明；界面本身在你自己的 App 里还没点开过一份画布草案看效果。
2. 画布那份草案**默认仍旧没有口径**：模型不一定会自己去评估，本轮补的是"用户能一键让它算一次"，不是"每次自动算"。自动算要多写一次本地核验并追加回执，本轮没做，也不拿它冒充已发生。
3. 单条核验单大到读不回来的方案，界面对它仍旧什么都不说（本轮的入口只在"完全没有回执"时出现，两者不是同一格）。
4. 没装机、没提交。
5. 方案超过 2000 份时的点名翻页仍没做。
6. 全量前端这一趟不是干净绿：那 2 条卡在 5 秒超时线上的老用例（整台应用集成一条、评测 CLI 一条）本轮没修，也没为了让它绿去调超时参数或拆用例——单独跑是绿的，说明它们只是没有余量，真要修得先拍板"提高超时"还是"把用例拆小"。

下一步顺序：①装机（需要先点头）；②方案超过 2000 份时的点名游标；③那两条卡在超时线上的用例要不要放宽/拆小；④超大核验单本身再收小（要先动那两份不计条数的清单，会改选片结果，得单独拍板）。

## 2026-11-08 Focal AI 第三十批：欠账点名改成翻页，方案多于 2000 份时更早的那批也点得到名

**做了什么**
- 点名取数原来只有一句 `ORDER BY updated_at DESC,id LIMIT 2000`：一个项目里方案超过 2000 份时，比这更早的方案**永远进不了名单**，界面那句"还欠多少"数的是看得见的这 2000 份，剩下的连"欠着"都算不上——修完预览也追不到它们。这一批把它换成按"改动时间新→旧、同一时刻按编号小→大"往回翻的游标，一轮最多翻 8 页（也就是 16000 份），欠账点名和按照片点名两条入口共用同一份实现，不再各写一遍。
- 游标条件这里有个坑值得记下来：排序是**一降一升**，所以不能图省事写成行值比较 `(updated_at,id) < (游标,游标)`——行值里两个字段同向，同一时刻里编号更大的那份会被整页漏掉，翻过去就再也不回来。写成了 `updated_at < 游标时间 OR (updated_at = 游标时间 AND id > 游标编号)`，并且把这个理由写进了代码注释。
- 规矩没动：一页取多少份留成内部参数只是为了能在小项目里验证翻页，生产入口固定用原来的 2000；那六个数字的口径一个也没改（`recheckRemaining` 仍旧是"该算的减去算成的"），翻到页数头里就按已点到的份数老实报，差额由 `draftsScanned < drafts` 说清是**没轮到**而不是没欠；界面无需改契约，前端也没有一处新的说法。

**验证情况**
- 后端 +2 条用例：25 份方案、页取 2、页数上限 8 → 点名窗口是 16 而不是 2，`draftsScanned=16`、欠账 16、算成 16、库里确实多出 16 条回执，同时断言"只点了 16 份"这件事本身被报出来（不冒充点完 25 份）；第二条专钉按照片点名那条**多了一次连接和去重**的路径，同样从 2 份窗口翻到 16 份。
- 本轮最终树上的 Rust 五道门退出码：`rustfmt --edition 2024`（只跑本轮改的这个文件）通过、`cargo fmt -- --check` 0、`cargo clippy --all-targets --all-features --features desktop -D warnings` 0、`cargo check --all-targets --features desktop` 0、`cargo test --all-targets --no-fail-fast --features desktop` 0，**575 条通过、0 条失败**（上一批 573，本轮 +2）。
- 本轮没动前端一行，也不改接口字段，所以前端那四道门沿用上批次串行重跑的结果（类型/Lint/契约 0，全量 vitest 那 2 条卡在超时线上的老用例照旧）。

**仍没做到**
1. 一轮 16000 份仍是**上限**，不是"再大的项目也点得完"：真超过这个数，界面上只会看到"点了多少份"，本轮没有为这种情况再加一句解释性文案。
2. 欠账判定读的是回执表那份"最新一条记的版本号"，而那张表本身有 5000 行的扫描上限：方案极多、回执更多时，翻页翻到了方案、判定却可能没覆盖到对应的回执——这是上一批就记下、本轮没有消掉的老不一致。
3. 画布草案默认仍旧没有口径、单条核验单大到读不回来时界面不说、那两条卡超时的老用例没修。
4. 没装机、没提交。

下一步顺序：①装机（需要先点头）；②回执侧那道 5000 行扫描上限与翻页对齐（否则第 2 条的老不一致还在）；③那两条卡在超时线上的用例要不要放宽/拆小；④超大核验单本身再收小。

## 2026-11-08 Focal AI 第三十一批：欠账判定不再有"全项目最新几千条"这道窗口，翻页翻到的方案才算真的跟上

**做了什么**
- 上一批把点名能翻到的份数从 2000 抬到 16000，但"这份欠不欠"判的还是那张回执表里**全项目最新 5000 条**：窗口外的方案哪怕刚刚算完，也照样被报成还欠着。后果不是少修几张，而是**永远修不完**——每一轮把 200 份额度花在重算上，"还欠多少"一位不降。这一批把判定改成**只按刚点到的那份名单问**：名单里每一份只认它自己最新那条回执记的版本号，名单外的一条也不必读回来。
- 名单是打包成一个 JSON 数组送进 `json_each` 的，不是每份方案一个问号——几万个方案会把 SQLite 的参数上限撑爆，这是库里已有的同一种写法，不新造机制。
- 原来那道 5000 行的常量删掉了，判定这一条路上不再有任何"扫到哪儿算哪儿"的窗口。体积门、坏行跳过、别的工具名不算口径、同一份取最新一条这四条规矩一个字没减，只是取数范围从"全项目最新几千条"换成"名单里这几份"。
- 界面、契约、文案这一批**一处没动**：六个对外数字的口径不变，前端不需要跟着改。

**验证情况**
- 后端 +1 条用例，专门钉这件事：先给一份方案写一条回执，再拿 5001 条**别人的**回执把它挤到最老——旧写法下扫描窗口只取最新 5000 条，这条刚算完的记录会被判成"没跟上"，新写法必须仍旧认得出它的版本号。这条用例在改动前必红，不是只能跟着绿的断言。
- 两条原有回执用例改成按名单问（断言内容一条没减：体积再大也报得出对着哪一版、没记版本号的老回执不猜一个、坏行不报错），另外原有那条"任务里堆过 50 条仍旧显示最新能读回的那条"照旧通过。
- 本轮最终树上的 Rust 五道门退出码：单文件 `rustfmt --edition 2024` 通过、`cargo fmt -- --check` 0、`cargo clippy --all-targets --all-features --features desktop -D warnings` 0、`cargo check --all-targets --features desktop` 0、`cargo test --all-targets --no-fail-fast --features desktop` 0，**576 条通过、0 条失败**（上批 575，本轮 +1）；那条 5001 次写入的用例整个模块 2.12 秒跑完，没给套件加出可感知的时长。
- 前端本轮无改动，沿用第三十批那四道门的结论。

**仍没做到**
1. 界面读的那份口径是"最新**能读回来**的那条"，欠账判定读的是"最新那条记的版本号"：整条超限读不回来的场合，两边仍旧会分叉（本轮消掉的是 5000 行这道窗口，不是这条）。要并成一张嘴，得先动那两份不计条数的清单，会改选片结果，需要单独拍板。
2. 一轮 16000 份仍是翻页上限，不是"再大也点得完"；真超出去只在"点了多少份"里看得出，没有一句专门的说法。
3. 判定改成按名单问之后，每轮取数从"最多 5000 行"变成"名单那么大一份 JSON 里过滤"——本机 SQLite 上几千份方案的项目 2 秒级跑完，但没有为十万份方案的项目实测过。
4. 画布草案默认仍旧没有口径；那两条卡在 5 秒超时线上的老用例没修；没装机、没提交。

下一步顺序：①装机（需要先点头）；②把"最新能读回来的"和"最新记的版本号"并成一张嘴（先拍板那两份清单）；③那两条卡超时的用例要不要放宽/拆小；④超大核验单本身再收小。

**这一步先停在这里的原因**：画布那份"生成后自动算口径"比看上去更深一层。第三十一批记的是"不能在工具事务里另开连接写回执"，这一批又查到一层——`execution.rs` 里每次工具调用外面还套着一层保存点（`tool_effect`），而那个事务**横跨整轮任务**、不是每条调用一提交：也就是说连"提交之后再补一次"这个落脚点，都要先找到整轮真正结束的位置，否则补的那一次照旧撞在同一把写锁上。这不是改两行能完的事，硬做会在预算里留下半截。

## 2026-11-08 Focal AI 第三十二批：全量前端门禁第一次跑到真绿灯，两条卡超时的用例按实测给余量

**做了什么**
- 前两批的记录里一直挂着同一句实话：全量前端跑不干净，总有 2 到 6 条报"5 秒超时"。这一批先量了再说——本机空载单独跑那两个文件，实测最慢的两条分别是 **881 毫秒**（整台应用起真渲染那条集成用例）和 **1985 毫秒**（评测命令行导入模拟评审那条），而全量 77 个文件并发跑时同一条要翻好几倍，默认 5 秒门在这台机器上就是会随机误红。
- 放宽只给**具体那两组**：`工作台交互` 与 `M3 evidence evaluation` 各自 20 秒，全局默认门**一位没动**——别的用例真变慢照样撞红，这才是想要的敏感度。每条放宽都在代码里写了实测依据和"为什么不是全局调大"，免得下次有人把整个 `testTimeout` 抬上去把敏感度抹平。
- 顺带把第三十一批那条事务约束又往下查了一层（见上面那段），画布自动口径的落脚点比预想的更深，这一批没硬开。

**验证情况**
- 前端四道门**全 0**，这是本轮窗口里第一次全量干净：`npm run typecheck` 0、`npm run lint` 0、`npm run contracts:check` 0、`npx vitest run` 0，**77 个文件 662 条全过、0 失败**。
- Rust 侧本轮一行没动，沿用第三十一批那五道门的结论（576 条通过、0 失败）。
- 放宽没有把任何一条用例改成跳过或删除：条数还是 662，和放宽前那次全量报的总数一致。

**仍没做到**
1. 只在这台机器、这一台机器空载或本机自用负载下验证过；分身会话并发跑 iOS 构建时会不会仍旧翻车，没测。
2. 那两组之外如果还有别的用例也卡在超时线上，本轮没做全量逐条耗时普查（只处理了真报过的两条），下次全量若出现新的超时再按同一套路补。
3. 画布序列草案默认仍没有口径（落脚点比预想的深，见上）。
4. 没装机、没提交。

下一步顺序：①装机（需要先点头）；②把"最新能读回来的"和"最新记的版本号"并成一张嘴（先拍板那两份清单）；③画布自动口径——先定 A（评估跑在同一条事务上）还是 B（找到整轮真正结束的落脚点再补一次）；④超大核验单本身再收小。

## 第三十三批（2026-11-08）：界面不再把"不知道"说成"没算过"

上一批给"没有口径"那一格补了入口，同时把那句话写死成"这份方案还没算过画面依据"。这一批回头核了一遍，发现这句话超出了界面能知道的范围。

**之前什么样。** 投影里读不到一份方案的口径有两种原因：从来没算过；算过但那条核验单整条读不回来——单条超过 512KB，或者同一个任务里反复重算把 50 行的扫描窗口占满，更早那份方案就被挤到窗口外面。后一种在方案多、重算过几轮的项目里是真会发生的，而界面把两种混成一句"没算过"，等于把"不知道"说成"知道"，正好踩在这条产品线的底线上。跟着那句一起的还有"已在本地算过一次，这里马上换成算出来的口径"：窗口被占满那种情形新算的那条确实排在最前、换得回来；整条太大那种情形再算一次还是太大，换不回来，那句就成了不会兑现的承诺。

**做了什么。** 只改话，不改数据通路。那一格改成"这份方案还没有能核对的画面依据，下面的判断只靠评分、状态与顺序"——只说界面确定知道的事；算完那句改成"已在本地算过一次。这份的口径读得回来时，这里会换成算出来的数字"，把承诺换成条件。四套语言同步改，占位符一个没动。代码里那条注释把两种成因写清楚，并注明"界面分辨不了，不许猜成没算过"。两处用例的断言跟着换成新句子：断的是界面说什么，不是拿"某句话不许出现"当代理。

**达到什么效果。** 界面对这一格说的一切都由界面自己掌握得住：没算过，这句成立；算过但读不回来，这句也成立。"在本地算一次"这个动作仍然值得给，因为在救得回来的那种情形里它真救得回来，而救不回来的那种也不再被许诺"马上换成"。

**验证情况。** 本轮 Rust 一行未动（五道 Rust 门的基线仍是上一批的 576 passed / 0 failed）。四条前端门串行跑完、退出码全为 0：类型检查、lint（`--max-warnings 0`）、契约核对、全量用例 **77 个文件 / 662 条全绿**（条数与上一批一致，本轮只改句子，没有新增或删减用例）；改动涉及的那份面板用例单独跑也 0。四语对齐用递归扁平化脚本核：zh/en/ja/ko 各 **1094 键**，缺失 0、多余 0，占位符差异 0。

**仍没做到（实话）。** ①两种情形只是不再被混着说，并没有真正分开：要分开得让后端在库里就地取那几个计数标量（`json_extract` 取 observed、四档计数与版本号，整条核验单不搬回内存），界面对这种方案就能说"这份的逐张档位没读回来"而不是含糊一句——本轮没做，记成 1-AG。②投影那道 50 行扫描窗口本轮没动，反复重算之后更早的方案仍会不投口径；把它换成"每份方案只看它最新两条"要先量代价（那样得扫完整个任务的回执行，每行都要解析一遍 JSON），量完再改。③装机仍未点头：第十七批往后的改动都还在这棵工作树上，未提交、未打包；本轮只做文案，没有新按钮要在真 App 里点，但那句"读得回来时换成算出来的数字"值得在真机上看一眼。

下一步顺序：①装机（需要先点头）；②1-AG：读不回来的口径与从没算过的分开说（先在库里就地取计数，并量一次换掉 50 行窗口的代价）；③把"最新能读回来的"和"最新记的版本号"并成一张嘴（先拍板那两份清单）；④画布自动口径——先定 A 还是 B；⑤超大核验单本身再收小。

## 第三十四批（2026-11-08）：算过但读不回来的方案，界面上不再是哑巴（工作包 1-AG）

上一批把那句越界的话收住了，代价是所有读不到的情形共用一句含糊话。这一批把其中一种情形真的分了出来。

**之前什么样。** 投影里读不到一份方案的画面依据有两种原因：从来没算过；算过但那条核验单整条读不回来（单条超过 512KB，或者被同一个任务里更新的一批挤出 50 行的扫描窗口）。两种在界面上长得一模一样，只能统一说"还没有能核对的画面依据"。可这几个数——凭几张画面事实说话、四档各占几张、排除了几张、对着方案的哪一版算的——**根本不需要把整条核验单搬回内存**：上一批取消欠账判定的那道窗口时已经在库里用 SQL 就地取过版本号了，同一招对这几个计数同样成立。也就是说，"整条读不回来"从来不是"说不出"，只是当时没去取。

**做了什么。** 后端在 `receipts` 里加了一条就地取数的查询：把整条读回来的那几份方案的编号交给 SQL，`NOT IN` 排除掉，剩下的每一份按行号从新往旧取它最新的一条，用 `json_extract` 一次把 9 个数字取出来，整条核验单一个字节也不搬回内存。为了不被老库里"同一个字段写成字符串"的那种行问倒，这 9 个数打包成一行 JSON 取回、在 Rust 侧判形状——形状对不上就这一份不投，绝不猜。排除几张用 `json_array_length` 数出来，数之前先问一句它是不是数组（SQLite 的 JSON 函数在坏数据上是报错，不是返回空）。投影侧把这份降级口径和正常口径合成同一个通道，总条数仍旧封在 50 份以内（前端对这条通道有同样的校验），并给它一个 `basisSummary` 标记：`scanned` 只等于观察数（逐张没读回来就不许装成扫过更多张）、`perPhoto` 留空。界面在这一格的计数下面加一句"这份方案算过，只是那份核验单没能整条读回来，逐张的档位这里就不写了"，四套语言同步；正常读回来的那份不会被贴上这个标记。

**达到什么效果。** "算过但读不回来"这一格从"什么都说不出"变成"数字照说、逐张档位的缺口照标"，界面与"从来没算过"不再是同一句话；而那句含糊话仍然留着兜住剩下的边角（比如老回执里根本没写这些字段的那种）。欠账判定、额度、上传像素、原图一律没动：这一条通道从头到尾只在本地读库。

**验证情况。** 后端加了一条真库用例：把一份方案的核验单垫到 600KB 顶破单条那道门，断言它的观察数、排除张数、版本号仍投得出来、带降级标记、逐张名单为空，且观察正文一个字也没顺路带回来；同时另一份正常大小的方案不被贴标记。四条前端门退出码全为 0：类型检查、lint（零警告）、契约核对、全量用例 **77 个文件 / 663 条全绿**（比上一批多 1 条，就是那条新用例）。Rust 全目标 `--features desktop` 退出码 0，单元测试 337 passed / 0 failed / 3 ignored，各集成套件全部 0 失败（上一批基线 576，本轮 +1）。四语对齐：各 **1095 键**，缺失 0、多余 0，新句子四套语言都在。剩余四道 Rust 门退出码也全为 0：clippy（`--all-targets --all-features -D warnings`）、`cargo check --all-targets --features desktop`、native-agent 那条编译开关（342 passed / 0 failed / 4 ignored）、`cargo fmt -- --check`（0 处差异）。本轮一次编译失败值得记着：`encode(value: &impl Serialize)` 有隐式 `Sized` 要求，把 `&[String]` 直接传进去不过，要写成 `encode(&readable)?`——那一处让五道 Rust 门同时报 101，而 101 是 cargo 自己报错，不是断言不符。格式门第一次跑（在修复之前）是红的，最后一次编辑之后重跑才得 0 差异。

**仍没做到（实话）。** ①50 行扫描窗口本身没动：窗口外的方案现在能报出计数，但一次投影为此最多扫 400 条核验单，这是给解析代价拍的顶，没有实测过超大项目刷新一次的真实耗时——真觉得卡要回来量。②"最新能读回来的那条"和"最新记的版本号"两条嘴仍旧没并成一张：降级那份显示的是就地取的那条，重算按钮出现的条件仍按现有两处判断，整条太大又排除了几张的方案会出现"点了重算、数字不变"的空转（数字没错，只是白点一次）。③逐张档位对降级那份确实没有，界面只能明说缺口，不能补。④装机仍未点头；本轮两个新句子（降级说明、上一批改的条件式回执）都值得在真 App 里看一眼。

下一步顺序：①装机（需要先点头）；②把 50 行窗口换成"每份方案只看它最新两条"并实测一次大项目刷新的耗时；③把"最新能读回来的"和"最新记的版本号"并成一张嘴（先拍板那两份清单）；④画布自动口径——先定 A 还是 B；⑤超大核验单本身再收小。

## 第三十五批（2026-11-08）：扫描窗口不再顺手当份数上限用

上一批下一步顺序里的第 ② 项本轮落地（第 ① 项装机仍等点头）。

**之前什么样。** 投影读核验单时"往回扫 50 行"这一条同时兼着两件事：挡住解析代价，又顺带把份数压在 50 份以内（前端对这条通道有同样的条数校验）。后果是同一份方案反复重算几轮就把行号占满，更早那份方案的**整条口径**连读回来的机会都没有——上一批的降级计数只是给它补了一句"算过、只是没读回来"，数字有了，逐张的档位仍然一起丢了。

**做了什么。** 把两件事拆开：扫描行数上限抬到 400（与就地取数那条同一个量级，都是每次投影在库里读标量），份数由代码明说——挑够 `MAX_ENTRIES=50` 份就停，不再靠行数顶出来。计划里写的是"每份方案只看它最新两条"（用窗口函数按方案分组），实现时没走那条：窗口函数要把这个任务的每一条核验单都解析一遍才能分组，代价随重算轮数无限涨；现在的写法仍然是从最新往旧扫、只是把看得见的行数放宽，单条太大时照旧退回它更早那条读得回来的。**上一批的降级计数没被撤掉**：整条超过 512KB 的方案仍旧走就地取数，只是"被同行号挤出去"这一大类从此不需要降级了。

**达到什么效果。** 反复重算过的老项目里，更早那份方案重新拿到完整的口径（含逐张档位），界面对它说得出"这几张各按哪档问法挑的观察"；投影一次最多 50 份这个约束由代码写死并有用例钉住，不会再出现"多投一条、整份投影被前端判成坏数据、整个面板一起看不见"那种失败形状。

**验证情况。** 两条新用例：①一份方案的核验单先写、再用同一份方案反复重算 120 轮把它挤到最老——断言那份更早的方案仍在投影里，且一份方案只说一条（旧代码只看最新 50 行时这条必红）；②60 份方案各一条核验单——断言投影条数正好等于份数上限。Rust 单元测试 339 passed / 0 failed / 3 ignored，全目标合计 **579 passed / 0 failed**（上一批基线 577，本轮 +2），格式检查 0 差异；clippy 全目标零警告（exit 0），`cargo check --all-targets --features desktop` exit 0；另一套编译开关 `cargo test --lib --no-default-features --features native-agent` 为 344 passed / 0 failed / 4 ignored（exit 0）。前端本轮一行未动，沿用上批全绿的那一组（77 文件 / 663 条），四套语言各 1095 键。

**仍没做到（实话）。** ①400 行这个数是拍的，没实测过大项目刷新一次的真实耗时；真要定死这个数字得先量。②"每份方案只看它最新两条"这个更省的形状没做（理由见上），如果将来库里给方案编号加个索引或生成列，可以再谈。③整条超过 512KB 的方案还是只有计数、没有逐张档位。④"点了重算、数字不变"的空转仍在。⑤装机仍未点头：第十七批往后的改动都还在这棵工作树上，未提交、未打包。

下一步顺序：①装机（需要先点头）；②量一次大项目投影刷新的真实耗时，再定 400 这个数；③把"最新能读回来的"和"最新记的版本号"并成一张嘴（先拍板那两份清单）；④画布自动口径——先定 A 还是 B；⑤超大核验单本身再收小。

## 第三十六批（2026-11-08）：装机——1.5.4 换包重启，三个入口在真 App 里真点真回话

用户点头后执行。第 ① 项落地；其余队列不变。

**之前什么样。** 第十七批到第三十五批的改动全在这棵工作树上，未提交、未打包；他机器上 `/Applications/FocalRoom.app` 还是冻结来源的 1.5.3，这一串新按钮一个都没在真 App 里点过。分支的版本字段停在 1.5.0，直接换包会变成"装一个更低版本"，自带更新还会把它换回去。

**做了什么。** ①版本字段两处（Cargo 与 Tauri 配置）从 1.5.0 抬到 1.5.4；②本机 `tauri build --bundles app` 重打包；③旧包先整份备份（装机前备份与换下的旧包各留一份在临时目录，可原样退回）；④换包、重启；⑤用原生界面在他真实的 1977 张项目里点了三个入口，全程只读。

**达到什么效果（真点到的原话）。** ①整项目预览点名："1977 张的本地预览现在都能读，没有要修的。"②历史观察像素版本点名："4 条历史观察没记像素版本：0 条能证明就是现在这一档字节，4 条补不了，涉及 4 张照片。"并跟着原因行"这一档问法今天已经不认得，补出来的指纹对不上当初发出去的那份 · 4 条"——补不了就照实说补不了，这条诚实规矩在真数据上成立。③单张本地文件检查："这张的本地预览现在读得出来，不用重算。"三次点击都没有写库、没有模型调用、没有上传；点前后库完整性 ok，3 个项目、2189 张照片、3 份草案、迁移停在 38，一个数没变。

**验证情况。** 打包命令退出码 1 的唯一原因是 updater 的 tar.gz 缺签名私钥（本机安装用不到它）；`.app` 本身 Developer ID 签名、`codesign --verify --deep --strict` 通过，换包后再次严格校验通过，进程正常启动。三条 UI 回话取自原生无障碍树原文，不是测试夹具。

**仍没做到（实话）。** ①**草案审阅那几句新文案（第二十九/三十三/三十四/三十五批）在真 App 里点不到**：他仅有的两份草案属于一个老任务，而面板按当前账户过滤任务，那个任务不显示，草案卡片也就跟着不出现。**（第三十八批更正原因：不是"登录前没归属"——那个任务有归属，只是归在另一个账户名下，与当前登录的账户不是同一个；退出登录也看不见，因为未登录时投影直接返回空。）**这是本轮真发现的缺口，没修——改账户过滤的语义会牵动"换号后旧任务能不能续"的既有边界，需要单独拍板。这几批的文案与计数目前仍只有单元/集成用例证明。②400 这个数仍未实测：本轮点的是预览点名，不是口径投影刷新。③未提交、未推送；这份包混着另一个会话约 140 个未提交的桌面/共享后端文件，只是本机验证包，不是干净发版候选。④未公证（本机安装不需要；对外发版是另一条线，仍需另行确认）。

下一步顺序：①老任务的草案在登录后看不见——先拍板要不要让**归在别的账户名下**的本地任务草案继续可见（原因见第三十八批更正）；②量一次大项目投影刷新的真实耗时，再定 400；③并成一张嘴（先拍板那两份不计条数的清单）；④画布自动口径 A/B；⑤超大核验单再收小。

## 第三十七批（2026-11-08）：400 这个数量过了——保留，不改

上一批下一步顺序里的第 ② 项本轮落地（第 ① 项老任务草案可见性仍等拍板）。

**之前什么样。** 第三十五批把"往回扫多少条核验单"从 50 抬到 400，为的是别让反复重算把更早那份方案的逐张口径挤出窗口；但 400 是拍的，没量过。一个没量过的数字写进代码，等于把"界面刷新会不会卡"押在猜测上。

**做了什么。** 加了一条手动基准（平时不跑，要用专门命令跑），在临时库里搭两种形状分别计时：①24 个任务、1446 条小核验单；②再加一个任务，灌进 402 条**每条 66022 字节**的核验单——正好把 400 行那道窗口顶满，而且每条都是当年一眼顶破 64KB 门的那种体积。三处分别量：整次投影刷新、单个任务整条读回、就地补计数。基准只打印耗时、不做时间断言（机器负载会让它随机红）。另外只读查了他真库里这类回执的实际条数与体积。

**达到什么效果（实测原数）。** 小核验单那组：整次投影 **25–26 毫秒**，顶满窗口那个任务整条读回 2 毫秒、补计数 7–8 毫秒。大核验单那组：该任务读回 **17 毫秒**、补计数 **21 毫秒**，25 个任务整次投影 **61–62 毫秒**——最坏的单任务形状只让整次刷新多花约 36 毫秒。**结论：两个 400 都保留。** 他真库里目前只有 **3 条**这类回执、最大 2758 字节（整个工具调用表才 29 条），离窗口远得很：这个顶是给将来反复重算很多轮的项目留的，不是他今天会碰到的形状。

**验证情况。** 格式检查 0 差异；clippy 全目标零警告（exit 0）；`cargo check --all-targets --features desktop` exit 0；Rust 全目标合计 **579 passed / 0 failed / 21 ignored**（与上一批同一个 passed 数——本轮只加了一条平时不跑的手动基准，passed 不涨才是对的）；其中单元测试 339 passed / 0 failed / **4 ignored**（上一批 3 ignored，多的正是这条基准）；另一套编译开关 344 passed / 0 failed / **5 ignored**（同样只多这一条）。前端本轮一行未动，仍是 77 文件 / 663 条，四套语言各 1095 键。基准首轮不是绿的，如实记一笔：先撞"一个项目同时只许一个任务在跑"，再撞"刚建的任务不能直接走结束审核"，最后撞"持有事务期间再调投影会等连接等到超时"——三处都是夹具自己的问题，改完才拿到上面这些数。

**仍没做到（实话）。** ①窗口是"每个任务"的，投影一次最多取 50 个任务；50 个任务都这么重就是约 1.9 秒——这是按单任务增量**推算**的，**没实测**（要灌 1.3GB，本机磁盘常年 99% 满，不能这么干）。②量的是 debug 构建、本机空载，不是他机器上开着 1977 张照片时的真实刷新；release 只会更快，但没量。③"点了重算、数字不变"的空转仍在。④老任务草案登录后看不见那条缺口本轮没动，仍等拍板。⑤未提交、未推送。

下一步顺序：①老任务的草案登录后看不见——先拍板（第三十八批已把原因查清：跨账户，不是没归属）；②并成一张嘴（先拍板那两份不计条数的清单）；③画布自动口径 A/B；④超大核验单再收小。

## 第三十八批（2026-11-08）：把"草案为什么看不见"查清，并更正我自己两个说错的判断

用户选了"新建一份本地草案来验"。本轮先按这个选择去查路，结果是**这条路不存在**，顺带查出上一批对原因的判断也是错的。本轮**没有改一行产品代码，没有往他库里写一个字节，没有调用任何模型**。

**之前什么样。** ①第三十六批装机时发现草案审阅那几句新文案在真 App 里点不到，我把原因写成"那两份草案属于登录之前的老任务、没有账户归属"，并据此在路线图和记忆里留了记录。②为绕开账户边界，我给用户的推荐选项里承诺"我在演示项目里用画布本地生成一份属于当前账户的新草案，不调模型、不上传"。

**做了什么（全是只读查证）。** ①查生产代码里所有往草案表写入的路径：只有模型驱动的 agent 工具会写（工具分发里两处、创作路径里一处），其余命中全在测试专用代码块里，真 App 走不到。画布的"新建看板"造的是看板、不是草案，而且看板上的审阅/采用入口只在"这个看板归属某个画布任务"时才出现——那个任务同样得由模型跑出来。**所以"不调模型造一份草案"做不到，我承诺错了。**②查他真库里那个老任务的账户归属：归属记录存在任务步骤表里（不是单独一张绑定表），那个任务**有**归属，只是归在**另一个账户**名下，与当前登录的账户不是同一个。**"登录前没归属"也是我说错了。**③查退出登录能不能看见：不能——未登录时投影直接返回空任务空草案，比登录时更少。④把路线图第三十六批那条"仍没做到"、两处下一步顺序的措辞，以及两份记忆里的同一句错误原因，全部更正为"跨账户"。

**达到什么效果。** 缺口原因从猜测变成查证过的事实：**不是"老数据没归属"，是"归属在别的账户"**——这两件事的修法完全不同（前者要放宽过滤，后者牵动"换号后能不能看见上一个号的活儿"这条产品边界）。同时把一条我做不到的选项从承诺里撤掉，免得下一轮又照着它去撞墙。真库里那两份草案是完好的、内容真实的，只要用拥有它们的那个账户登录就能看见，不需要花额度、不需要造新数据。

**验证情况。** 本轮无代码改动，所以沿用上批全绿的那一组门禁（Rust 全目标 579 passed / 0 failed / 21 ignored，格式 0 差异，clippy 0 警告，前端 77 文件 / 663 条）；结论全部来自只读读代码与只读打开他的库（`mode=ro`），点前后没有写入。更正后的记录已逐处 grep 复核落地。

**仍没做到（实话）。** ①用户选的那条路没做成，因为它不存在；草案审阅文案**仍然没有在真 App 里点到过**，仍只有单元/集成用例证明。②真正可行的三条路都需要用户先表态：用拥有草案的那个账户登录（凭证操作，必须本人接手）、在当前账户下花一次模型调用（要花额度，得显式批准）、或先拍板改账户过滤语义（改产品边界）。③账户过滤语义本身没动。④未提交、未推送；装机包仍是本机验证包，不是干净发版候选。

下一步顺序：①草案审阅的真机验证——三条可行路里挑一条（都要用户表态）；②账户过滤语义要不要改（换号后旧任务可见性）；③并成一张嘴（先拍板那两份不计条数的清单）；④画布自动口径 A/B；⑤超大核验单再收小。

## 第三十九批（2026-11-08）：重算白点一次，不再假装有了进展

上一批队列里全是要用户表态的事，本轮挑了不需要拍板的那条残留（"点了重算、数字不变"）先做掉，属第 1 项分层视觉分析。

**之前什么样。** 草案审阅里那个"按现在这一版重算口径"按钮，点完一律回一句"已重算一次，口径按现在这一版重新记了"，不管数字有没有变。而被排除的那几张（原图换过、或本地预览读不出来）再算一遍还是被排除，于是屏幕上的数一位不变、按钮照旧摆着，用户白点一次还以为有了进展；每点一次还往库里追加一条回执。

**做了什么。** 点击前先记下**屏幕那一刻**的四个数（有观察几张、没观察几张、被排除几张、对着方案第几版），拿重算回执逐项比对：完全一样就换一句照实的话——"结果和刚才一样，那几张的画面记录还是用不上，再算也是这个数，先把它们的本地预览修好"；只要有一项变了（包括版本号终于对上）就照旧说已按现在这一版重新记过。四套语言同步。**刻意没把按钮藏掉**：用户可能在弹窗开着时去别处修好预览，藏了就没法再算。

**达到什么效果。** 空转不再冒充进展，并且直接告诉用户下一步该做什么（修那几张的本地预览）；真把排除的几张补回来时，仍旧明确说数字变了。

**验证情况。** 前端 typecheck / lint / contracts:check 全 exit 0，全量 **77 文件 / 666 条**（上批 663，本轮 +3），四套语言各 **1096 键**、占位符逐键比对完全一致；草案审阅单文件 24 条全绿。Rust 本轮一行未动，但这棵工作树混着另一个会话的改动，所以仍在当下这棵树上重跑了格式检查（0 差异）与 clippy 全目标（0 警告），都 exit 0；Rust 测试沿用上批 579 passed / 0 failed / 21 ignored。三条新用例里有一条专门钉住最容易写错的地方：**比对必须用点击那一刻的数**——面板每秒重读投影，回执还没回来屏幕上的数就已经换过，拿新数比新回执会把"真有进展"误判成空转。

**仍没做到（实话）。** ①**空转本身没消**：按钮照常出现、照旧写一条回执，本轮只是不再说谎。要真正不白点，得知道每张为什么被排除、并分辨"修预览能救"与"救不了"，那是下一环。②草案审阅那几句文案**仍未在真 App 里点到**（跨账户，等用户换号登录或批准一次模型调用）。③**口径这一整族手机上根本没有**（本轮查证：iOS 只有"画面依据（模型观察）"一段，没有本地重算入口、没有逐张档位、没有排除张数与降级说法），所以第四十五到四十八、六十四与本轮这些成果**只在桌面成立**，不是双端。④未提交、未推送、未装机——本轮改动不在他机器上那个 1.5.4 包里。

下一步顺序：①草案审阅真机验证（等用户换号登录，或批准一次模型调用）；②账户过滤语义要不要改；③把空转真正消掉（按每张的排除原因分辨能不能救）；④并成一张嘴（先拍板那两份不计条数的清单）；⑤画布自动口径 A/B；⑥手机侧的口径缺口。

## 第四十批（2026-11-08）：能证明是白点的那一格，干脆不摆按钮

上一批下一步顺序里的第 ③ 项本轮落地，仍属第 1 项分层视觉分析，不需要用户拍板（不改选片结果、不动后端、不花额度）。

**之前什么样。** 上一批只做到了"白点一次不再假装有了进展"，按钮本身照常摆着、照旧往库里写一条回执。要真正消掉它，得先回答一个问题：被排除的那几张，本地重算到底救不救得回来？

**做了什么。** 先把排除原因查到底：一共只有三类——字节对不上、本地预览读不出来、当初没记下像素指纹。判定函数里"没记指纹"是直接返回的，而重算读的还是同一行记录、指纹仍然是空的，加上既有规矩不许拿新算的指纹冒充旧的，所以**只有"没记指纹"这一类本地永远翻不过来**；另两类修好预览之后有可能翻过来。据此收掉一格：当"版本号已经对得上 + 不是降级那份 + 逐张原因一张不缺地看得到 + 被排除的全是没记指纹"四条同时成立时，这一格不再摆重算按钮，改说照实的话——这几张本地补不出来，再算也还是用不上，要让它们有画面依据只能重新问一次模型。**任何一条证明不成立就保持原样**：降级那份没有逐张原因、逐张超过 100 条会被截、版本号还没对上时重算仍有正事要做、混有另两类原因时修好预览还有救。

**达到什么效果。** 注定白点的那一格不再让人点，也不再往库里追加没用的回执，并且直接说清唯一的出路是重新问模型（本轮没有替用户发起任何模型调用）。还有救的那些格子按钮一个没少。

**验证情况。** 前端 typecheck / lint / contracts:check 全 exit 0，全量 **77 文件 / 674 条**（上批 666，本轮 +8），四套语言各 **1097 键**、占位符逐键比对完全一致；草案审阅单文件 32 条全绿。Rust 一行未动，但仍在当下这棵树上重跑了格式（0 差异）与 clippy（0 警告），两条都 exit 0；全目标测试沿用第三十七批基线 579 passed / 0 failed / 21 ignored。新增 8 条里 5 条分别钉住上面每一道守卫（全是没记指纹→不摆按钮；混一张预览读不出来→照旧摆；逐张被截→照旧摆；降级那份→照旧摆；版本号没对上→照旧摆），另 3 条走英日韩三语确认这一格不残留中文。另外核对了两件容易想错的事：①逐张原因确实带着这三类原因码投到界面，且界面通道同样封在 100 条，所以"原因没看全就不敢下结论"这道守卫是必要的；②**整项目那条"修完预览后补老方案口径"的路没有同样的空转**——它判欠账只看回执记的版本号对不对得上，不把"有照片被排除"当欠账，所以不会每轮白跑还报"仍欠着"（体积门那个变体第三十四批已修）。

**仍没做到（实话）。** ①"字节对不上"与"预览读不出来"这两类里，究竟哪些修好预览真能翻、哪些原图本身换过所以永远翻不了，本轮**没有再往下分**——那需要知道原图有没有变，是下一环；所以这两类的按钮仍可能白点一次（但上一批起至少不会假装有了进展）。②草案审阅那几句文案仍未在真 App 里点到（跨账户，等用户换号登录或批准一次模型调用）。③口径这一整族手机上仍然没有。④未提交、未推送、未装机——本轮与上一批的改动都不在他机器上那个 1.5.4 包里。

下一步顺序：①草案审阅真机验证（等用户表态）；②账户过滤语义要不要改；③把"字节对不上/预览读不出来"再分成"修得好"与"原图换过救不了"；④并成一张嘴（先拍板那两份不计条数的清单）；⑤画布自动口径 A/B；⑥手机侧的口径缺口。

## 第四十一批（2026-11-08）：修不回来的那两种情形，当场说出来

上一批下一步顺序里的第 ③ 项本轮只做"照实说"这一半（逐张分辨要动后端，见仍没做到），仍属第 1 项分层视觉分析，不需要用户拍板（不改选片结果、不动后端、不花额度）。

**之前什么样。** 第三十九批那句空转说明写的是"先把它们的本地预览修好，数字才会变"——这是一句承诺，而不是一句事实。

**做了什么。** 先把承诺查实：本地修复在原图内容变过（`original_changed`）或原图不在本机（`original_missing`）时**一个字节都不写**（已有回归用例钉住），而且就算写了也永远回不到当初记下的那份预览指纹。所以那句承诺对这两类照片是假的。改成带例外的照实说法：先修本地预览、修好再算一次才可能变；原图本身换过或不在本机的那几张修也修不回来，只能重新问一次模型。四套语言同步。顺带查实一件容易想反的事：挑观察那条查询的 join 条件本来就要求"库里记的原图内容指纹等于观察记的指纹"，所以**库里已经更新过指纹的"原图换过"照片根本挑不到观察**，会落进"没看过"那份名单而不是带原因的排除名单；能落进排除名单的"原图换过"只有"盘上换了、库里还记着旧指纹"这一种，而这一种修复恰好拒绝写——新文案的例外正好覆盖它。

**达到什么效果。** 用户不会再被引着去修一份注定修不回来的预览；还有救的那条路（修好再算一次）仍旧说清。本轮没有动后端、没有发起任何模型调用、没有改选片结果。

**验证情况。** 前端 typecheck / lint / contracts:check 全 exit 0，全量 **77 文件 / 677 条**（上批 674，本轮 +3）；草案审阅单文件 **35 条**全绿；四套语言各 **1097 键**，键集合与占位符逐键比对完全一致，并逐套确认新例外那句真写进去了（第一次查报"四套都没有"是我自己按顶层取键查错了——键嵌在同名小节里，改成按后缀找才对，这条坑记忆里已有）。新增 3 条走英日韩三语，确认空转那句连例外一起不残留中文。Rust 一行未动，但仍在最终这棵树上重跑了格式（0 差异）与 clippy（0 警告），两条都 exit 0；全目标测试沿用第三十七批基线 579 passed / 0 failed / 21 ignored。

**仍没做到（实话）。** ①逐张分辨"这张修得好 / 那张原图换过救不了"仍未做：要把原图路径与内容指纹带进排除原因、在本地哈希整张原图比对，代价是在写事务里读原图（大 RAW 可能几秒），得先量再动；所以界面现在只能把两种可能一起说，不能点名到张。②草案审阅那几句文案仍未在真 App 里点到（跨账户，等用户换号登录或批准一次模型调用）。③口径这一整族手机上仍然没有。④未提交、未推送、未装机——本轮与前两批的改动都不在他机器上那个 1.5.4 包里。

下一步顺序：①草案审阅真机验证（等用户表态）；②账户过滤语义要不要改；③逐张分辨"修得好 vs 原图换过"（先量写事务里哈希原图的代价）；④并成一张嘴（先拍板那两份不计条数的清单）；⑤画布自动口径 A/B；⑥手机侧的口径缺口。

## 第四十二批（2026-11-08）：查证批——他真项目的原图现在一张都不在本机

上一批下一步顺序第 ③ 项的前置测量，本轮**一行代码都没改**（只读他的库与本机文件，外加一个自己造、用完即删的探针文件）。

**查到的事实。** ①库里 2189 张照片：**1977 张原图指向 `/Volumes/LUMIX`**（他那个真实项目，SD 卡），12 张演示原图在本机缓存里，200 张是测试夹具（系统临时目录）。当下挂载的卷只有 `HANVIN` 与 `Macintosh HD`，**LUMIX 没挂**——抽 300 张逐张 stat，全部 `No such file or directory`。②这正好落在第四十一批新加的那句例外上：原图不在本机 → 修复一个字节都不写 → 那几张的画面依据本地永远补不回来。与第三十六批装机时"1977 张的本地预览现在都能读"**不矛盾**：预览在本机缓存里，原图在卡上。③哈希代价量过了：原图指纹是流式 SHA-256（64KB 一块读整个文件，`image_pipeline::fingerprint`），本机 64MB 探针两遍各 **0.14 / 0.15 秒**（约 450MB/s）。**这是刚写完的热缓存数、探针又是零填充文件，只能当上界**；SD 卡冷读会慢得多。按这个上界推，一份 100 张的方案逐张哈希原图约 9–20 秒，而这段跑在 `BEGIN IMMEDIATE` 写事务里，会把整库锁住这么久。④顺带一个本机风险（只报不动）：数据卷 **100% 满，只剩 2.0Gi 可用**。

**据此定下的做法（下一批照这个做，别顺手塞）。** "逐张分辨修得好 / 原图换过"不能直接加进现在那条评估：先判文件在不在（不在就零成本判死，他当下 1977 张全落这一格），只有文件在的才哈希；哈希要么挪到写事务之外、要么设每轮上限并照实报"没来得及"，不许把整库锁住十几秒。取数那条 SQL 的 `photos` 表里 `original_path` 与 `content_sha256` 本来就有，不用另开查询。

**验证情况。** 本批无代码改动，故无新门禁数：沿用第四十一批的前端 77 文件 / 677 条、四语各 1097 键、Rust fmt 0 / clippy 0。所有数字都来自只读查询与本机计时，输出留在 `/tmp/b42_*.txt`；探针文件已删（`probe_removed=0`）。

**仍没做到（实话）。** ①逐张分辨仍未实现，界面现在只能把两种可能一起说。②草案审阅那几句文案仍未在真 App 里点到（跨账户，等用户表态）。③口径这一整族手机上仍然没有。④第四十、四十一批未提交、未推送、未装机。⑤磁盘只剩 2.0Gi 这件事只报了，没替他清任何东西。

下一步顺序：①草案审阅真机验证（等用户表态）；②逐张分辨"修得好 vs 原图换过"（先判文件在不在，哈希挪出写事务或设上限）；③账户过滤语义要不要改；④并成一张嘴（先拍板那两份不计条数的清单）；⑤画布自动口径 A/B；⑥手机侧的口径缺口。

## 第四十三批（2026-11-08）：逐张点名"原图不在本机/已换过"，修不回来的不再说有救

上一批下一步顺序第 ② 项，照第四十二批定下的做法实现：先判文件在不在（不在零成本判死），只有文件在的才哈希，每轮设上限并照实报"没来得及"。仍属第 1 项分层视觉分析。

**之前什么样。** 第四十一批把"修也修不回来"的两种可能（原图换过、原图不在本机）合在一句话里说，不能点名到张；"字节对不上/预览读不出来"那两类里谁还有救、谁已经判死，界面分不出来，重算按钮仍可能白点。

**做了什么。** ①评估取数那条 SQL 顺手带上原图路径与在册内容指纹（本来就有这两列，不另开查询）。②排除名单里每张加一格原图状态：文件不在 → missing（零成本，不占名额）；文件在才流式哈希整张原图与在册指纹比对 → matches / changed；每轮最多哈希 8 张（MAX_ORIGINAL_CHECKS），名额用完照实报 unchecked，绝不把整库锁在写事务里十几秒；"当年没记像素版本"那类本地永远翻不过来，不占名额也不查原图。③回执投影只认这四种状态，陌生值整条拒收；老回执没这一栏照常读取。④界面跟着分辨：排除的全是 missing/changed/没记指纹时不再摆那个白点重算按钮，改说通用的"本地补不回来，只能重新问一次模型"；逐张那一行优先点名死因（原图不在本机/原图已经换过，本地修不回来），matches/unchecked/老回执照旧只说预览那一层。四套语言同步新增 3 键。

**达到什么效果。** 他当下 1977 张原图在没挂载的 SD 卡上：评估会逐张点名"原图不在本机"，不再引着去修注定修不回来的预览；原图还在本机的照片照常哈希分辨，还有救的（matches/unchecked）按钮照旧。全程本地只读原图，不上传、不花额度、不改选片结果。

**验证情况。** Rust：lib 344 passed / 0 failed / 4 ignored；全目标 584 passed / 0 failed / 21 ignored；native-agent 349 passed / 0 failed / 5 ignored；check/clippy/fmt 均 exit 0。新增 3 条视觉单测（missing 不占名额、matches/changed 分辨、名额用完报 unchecked）、1 条投影拒收陌生状态、1 条贯穿测试（12 张被排除照片走真实评估链路：8 张需哈希恰好等于上限，逐张断言 matches/changed/missing 与没记指纹不查原图，额度与版本号一个不动）。贯穿测试首两轮因夹具数量错（范围写成 4..16 加成 12 张而非 11 张）红，改对后绿。前端：typecheck/lint/contracts exit 0，全量 77 文件 / 694 条（上批 677，本轮 +17）；草案审阅单文件 51 条（+16：判死不摆按钮、通用文案、逐张死因、matches/unchecked 留按钮、英日韩三语）；仓储 +1（四种状态过界、老回执缺栏可读、陌生值拒收）；四套语言各 1100 键，键集合与占位符逐键一致。前端全量首轮红 1 条：新测试夹具"排除张数 2、逐张只给 1 条"，被组件那道"逐张原因一张不缺才敢判死"的守卫照实拦下；修夹具（omitted 改成 1）后串行重跑全绿。

**仍没做到（实话）。** ①草案审阅那几句文案仍未在真 App 里点到（跨账户，等用户换号登录或批准一次模型调用）。②口径这一整族手机上仍然没有。③未提交、未推送、未装机——第四十至四十三批的改动都不在他机器上那个 1.5.4 包里。④每轮 8 张哈希上限之外的照片只报 unchecked，大方案要再算一轮才查得完（照实说，不假装查过）。

下一步顺序：①草案审阅真机验证（等用户表态）；②账户过滤语义要不要改；③并成一张嘴（先拍板那两份不计条数的清单）；④画布自动口径 A/B；⑤手机侧的口径缺口。

## 第四十四批（2026-11-08）：外部判断回流收口——不用重开导入窗口，本地还空着的可以一次填

收的是工作包 6 那三环（第二十四、二十五、二十六批增量）留下的三条债：入口只有导入对话框一处、文案只有中文、回填只能一条一条点。属用户本轮点定的三件事之一。

**之前什么样。** 边车读到的判断只在导入对话框走完一次导入之后才看得见；想再核对一次外部软件里的评分，得重开导入。所有句子硬写中文，英日韩界面看到的还是中文。一百张里本地空着的，也只能一条一条点"回填这一条"。

**做了什么。**
- **判断"这条能不能一键填"只留一处定义**：`SidecarProposal::batch_fill()` 是唯一真相源——本地评分还是 0 或状态还是未决、且两处评分不矛盾，才算一键够得着；投影里那个 `batchSafe` 和真正落库的写计划都由同一个函数算出来，界面和库不可能各说一套。
- **新命令 `backfill_sidecar_judgements`（只收项目编号与期望修订号）**：读边车仍走只读那条路（不接界面递进来的路径），写计划先在阻塞线程里算好；事务里一次只涨一号修订，每张的 UPDATE 都带 `AND rating=? AND status=?` 的守卫——扫描之后、点下按钮之前被摄影师自己改过的那条会被跳过而不是被覆盖；跳过条数照实报。变成保留的那些走与人工点选片完全同一条序列补入；一条都没落成时整个事务回滚，报"这些判断在你点一键回填之前已经被你自己改过了，一条都没有动"，不留下空事件。回执 `sidecar_apply` 带 `mode:batch`、填了几条、跳过几条与前 50 个照片编号（超出标 `photoIdsTruncated`）。
- **界面拆成两个独立文件并常驻图库一侧**：就地改写与外部判断两段从导入对话框里搬出来，图库侧在本地项目、非只读共享、且这条通道存在时一直挂着，随时"看看外部软件里的判断"，读完还能"再读一遍"，不必重开导入；对话框里那段同时保留。因为这两条是不走实时事件通道的直接命令，落库后由外层刷新台账。
- **一键按钮只报数、不多许诺**：按钮上写的是本地还空着的条数；一条都填不成时不摆按钮，只说"一次填上只碰本地还空着的那些；人工判过的还得逐条点头"；仓储没有这条通道时同样不摆兑现不了的按钮，逐条回填照常。点成之后按现在的台账重读一遍再说"还剩什么"，不拿点之前的清单充数。
- **文案进四套语言**：新增 34 键 × zh/en/ja/ko，星数与状态沿用既有 `starOption`、`status.*` 词汇（未决说成"待定"、keep 说成"已保留"），图库侧的排版与另两块本地文件状态一致。

**达到什么效果。** 在 Lightroom/Capture One 里判完回到本地库的人，不用重开导入就能随时看清"外部写着什么、台账记着什么"；本地还空着的那批可以一次填上，而他自己在别处已经判过的、两处矛盾的、外部只写着"未选中"的，一键一律不碰——最坏情况是跳过并报名字，不会是覆盖。边车文件自始至终只读。

**验证情况。** Rust：`cargo check --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt -- --check` 均 exit 0；lib 345 passed / 0 failed / 4 ignored（+1）；全目标 586 passed / 0 failed / 21 ignored（+2）；另一套编译开关 `--no-default-features --features native-agent` 350 passed / 0 failed / 5 ignored（+1，手机端那条通道也编得过）。新增一条贯穿测试（5 张夹具）：一键填 3 跳过 0、修订号只 +1、外部 4 星补进来、外部写着未选中那条只补星没翻成淘汰、两处都有和两处都空的两条一个字节没动、序列按新增保留补 1 条、再点一次被 `CONFLICT` 拒且修订号不变、边车文件哈希不变、照片总数仍是 5。前端：typecheck / lint（`--max-warnings 0`）/ contracts:check 均 exit 0，全量 **77 文件 / 703 条全绿**（上批 694，本轮 +9）；外部判断面板单文件 13 条（新增一键成功、一键被拒不重读、无可填不摆按钮、无通道不摆按钮、读完可再读），图库侧 3 条（网格与序列都常驻、网页预览与只读共享不出现、仓库没通道不出现），仓储 1 条（一键只发 `{expectedRevision, projectId}` 两个键，负数、小数、字符串计数、缺跳过计数四种脏回执都不认）。四套语言递归扁平各 1134 键，缺失 0、多余 0、占位符差异 0。

**仍没做到（实话）。** ①一键只碰本地还空着的，判过一半的（比如有星没旗）仍旧只能逐条点。②边车读取没有缓存与增量：一万张项目上"看看外部软件里的判断"和一键回填都是整项目读一遍，按 mtime 跳过没做。③图内 XMP 包（JPEG/CR3 里那份）、Lightroom 目录数据库、Capture One `.cos` 依然读不到；色标签本地没有列，仍只报数字不落库。④手机端没有这条回流通道。⑤一键那个按钮在真 App 里还没点过——本批之后随打包一起装机验证。⑥未提交、未推送。

下一步顺序：①手机侧画面依据口径；②提交、打包、装机（用户已点头）；③发官网；④边车扫描缓存；⑤画布自动口径 A/B。

## 第四十五批（2026-11-08）：手机侧把"画面依据"这套口径整套搬过去

用户本轮点定的第三件事：手机侧整套画面依据口径。

**之前什么样。** 手机上翻一份草案，只看得见评分、状态、顺序这些判断结果，一句"这份判断凭几张真看过像素的观察"都没有。桌面侧从第十四批到第四十三批攒下的那套口径——哪档问法挑的观察、对着方案哪一版算的、哪几张被跳过、读不回来和从没算过要分开说——手机一个字都没接。更别扭的是数据其实早就到手机了：手机复用桌面核心，投影里那份 `draft_evidence` 数组跟着任务一起发过来，只是 Swift 侧的结构体没有这个字段，JSONDecoder 一声不响地把它丢掉了。

**做了什么。**
- **模型层只加一个可选字段，不动任何既有解码**。`NativeAITask` 补 `draftEvidence`，配上显式 `CodingKeys` 和手写的 `init(from:)`，原来每个字段的读法一位不改；这一格用 `try?` 兜：口径只是锦上添花，形状不合时不能把整份任务投影一起带走。旧负载（没有这个键）仍旧读得回来，新 App 读旧库、旧 App 读新库都不炸——这是"手机复用桌面核心"那条线唯一的兼容性护栏。
- **口径的形状按桌面那份合同严格收**：`observed/unobserved/pageTier/detailTier/otherVersion/otherTool/crossContractPairs/omitted/scanned` 全是要数字的整数，`draftRevision` 可为空；逐张那几条只认桌面写过的三组值——问法档位四种、像素版本三种（对不上、读不出、没记过）、原图状态四种（缺、换过、一致、没查）。**认不出来就当没说，不猜**：将来桌面加一档新问法，手机宁可这句空着，也不会把没见过的名字翻成"已核对"。
- **草案审阅页加一段"画面依据"**：几款问法各挑了几张、口径是对着第几版算的（就是现在这版 / 中间改过没重算，两种说法分开）、有没有跨口径混着比、被跳过的有几张、逐张只点到列出来的那几张（还欠多少照实说），以及桌面降级成"只有汇总"那种情况下明说"这份只给了汇总，逐张没给"。列表里每张后面补一行小字，说这张是按哪档看的、或者为什么没用上。
- **手机端不放重算按钮**：这条通道在手机侧根本不存在（手机上没有把草案重新评估一遍的操作），摆一个点了没反应的按钮比不说更糟。所以那一格直接写成一句话：要在手机上按现在这一版重算，得回桌面端打开这份草案；手机只读这些数字，不重算，也不会因此改动草案。

**达到什么效果。** 拿手机翻看桌面生成好的草案，现在能和桌面看到同一套"这话凭几张看过像素的观察"的说法：哪几张真是看过画面的、哪档问法挑的、对着哪一版算的、哪几张没用上，全都能当场读出来；从没算过的和读不回来的仍旧分着说，不混成一句"数据不可用"。老任务、老草案照旧翻得开。

**验证情况。** Swift 单元测试新增 5 条：①没有这个键 → 字段为空且整份任务照读；②正常六数 + `draftRevision` 为空 + 逐张行；③把档位写成 `wild_guess`、像素写成 `verified`、原图写成 `whatever`，三条都归为"没说"，而这份口径其余数字仍可读；④降级汇总那条（`perPhoto` 为空、`scanned` 等于 `observed`）读得回来；⑤三种坏形状（数组整体是字符串、缺计数、缺逐张）→ 口径为空但 `goal`/`state`/`needsReview` 全活着。核心套件全量 **91 条通过 / 10 跳过 / 0 失败**（跳过的是需要真机与真后端的那些）。真实 App 目标 `xcodebuild` 编过（BUILD SUCCEEDED）。文案：本批新增 21 句 × 英日韩共 63 条译文写进目录，目录键数 768 → **792**；跑完再复核一次抽取脚本，代码里真正引用的 777 个键**全都在目录里，需要补写的为 0**，占位符与原句不一致的为 0。仍缺译文的是 5 句不属于本批的旧键（网络线路提示、轮次计数、当时证据状态、两处品牌拼接格式）。

**仍没做到（实话）。** ①这段界面只在单元测试与编译里过了一遍，没在模拟器或真机上逐句看过排版——长句在小屏上会不会挤成两行、深色模式对比度如何，都没实测。②手机侧仍然没有重算通道，只能读桌面算剩下的；桌面没算过的草案，手机上那句"还没有能核对的画面依据"就是终点。③这段口径只挂在草案审阅页；手机上的画布与看图板审阅还没有同一格。④逐张那行小字只覆盖回执里列出的那几张，方案很大时靠"还欠多少"那句兜住，不做翻页。⑤依赖桌面先发一次投影——手机自己不会去问模型，也不会上传照片，这是设计如此。⑥未提交、未推送，装机验证留到下一次打包。

## 第四十六批（2026-11-08）：画布生成的序列草案一落地就有一份口径（工作包 1-AF）

收的是第三十九到四十三批留到最后那个 A/B 待拍板项：口径要不要在草案生成时就写好。选了"要"，并按只留一份实现的方式做。属用户本轮点定的"AI 与作品草案融合"那一件。

**之前什么样。** 在画布里把一组照片提成序列草案，草案本身立刻就在库里了，可画面依据那一格要等用户去点"按现在这一版重算"才有数——第一眼看过去是"这份方案还没有能核对的画面依据"，而这份方案恰恰是同一个任务里刚挑完的那组。图省事在画布工具里直接调那次重算也不行：重算自己会另开一条连接写回执，而画布工具此刻正占着外层事务的写锁，WAL 下两边互等，等于把画布点死。

**做了什么。**
- **把重算拆成"开事务的那层"和"什么事务都不开的那层"**：`recheck::run` 仍旧自己起事务、自己提交，界面上那个重算按钮走的就是它；新增 `recheck::record(tx, task, draft_id)` 只做取数、本地评估、写回执，**不提交**，用哪条事务由调用方给。取数、算口径、写回执这套只有一份实现，不存在"按钮那条算得细、画布那条算得粗"的两张嘴。
- **画布工具生成草案后在同一条事务里就地补一次评估**：草案行、条目行、评估回执一道落地，要回滚一起回滚，不会出现"草案在、口径没在"或反过来。工具回执多带一个 `visual_basis`，只说两种话——`recorded` 或 `not_recorded`：没记上就照实说没记上，不拿一次静默失败冒充成功。
- **一处编译层的坎如实记下来**：这条路径是"工具执行 → 画布扩展工具 → 就地评估 → 又回到工具执行"的异步自递归，不加一层 `Box::pin` 断开，futures 的尺寸算不出来，crate 直接拒绝编译。断的是类型递归，不是运行时递归（`evaluate_draft` 这个名字不会再被路由回画布扩展口）。

**达到什么效果。** 从画布提出来的那份序列草案，打开审阅就带着完整的六个数字、版本号与逐张口径，不需要先点一次重算；桌面把这份投影发给手机，手机上（上一批那套口径）也一并有了数。写回执与草案同生同死，最坏情况是明确报"这一份没记上"，不会是界面上静静少一格。

**验证情况。** Rust 六道全 exit 0：`cargo test --lib` 345 passed / 0 failed / 4 ignored；`cargo test --all-targets --no-fail-fast` **586 passed / 0 failed / 21 ignored**；`--no-default-features --features native-agent` 350 passed / 5 ignored；`cargo clippy --all-targets --all-features -- -D warnings` 0 警告；`cargo fmt -- --check` 0 差异；`cargo check --all-targets --features desktop` 0。**测试条数与上一批一致（586、345、350 都没涨），这是有意的**：这一批把既有那条贯穿用例改强——原来它在断言前显式点一次重算，现在全程一次重算都不点，只走"画布生成"这一步，再断言工具回执是 `recorded`、投影里 `draftId` 对得上、`count/observed/pageTier` 都是 3、`unobserved/crossContractPairs/omitted` 都是 0、逐张三条全是"当前页面档"、版本号是 1。真卡死的话这条用例会挂住而不是报错，跑完用时 1.2 秒。

**仍没做到（实话）。** ①`not_recorded` 只说"没记上"，不说为什么没记上（评估失败的原因留在错误里，没进回执），界面那一格仍旧显示"还没有能核对的画面依据"。②"不会互等成死锁"没有独立自动化证明，只有这条用例不挂这一条证据；真项目里画布并发写锁的时长没量过。③手机端跟着有数只是推理自同一份投影，没在手机上再点一次确认。④画布侧另外两种生成（分组、便签）本来就不产草案，没为它们加断言。⑤未提交、未推送，随下一次打包装机再真点一遍。

## 第四十七批（2026-11-08）：草案里逐张写出"AI 自己哪儿拿不准"（Sprint 2-5 收口）

"AI 与作品草案融合"这一件的原计划第 5 项要求四样：独立草案、理由与不确定性与变更列表、部分采纳与撤销、未授权不得改正式画布。先读代码核对：除了"不确定性"这一格，其余三样早就落地并有既有用例压着——**独立性**：Agent 任务的画布带 `owning_task_id`，人工任务拿不到画布改写工具；**部分采纳**：草案审阅可以只勾几张按 `partial` 提交，`full` 必须明确选全部并二次确认；**撤销**：集成用例 `ag08_15_stale_rebase_partial_apply_and_undo_preserve_unapproved_state` 走的是真命令通道——部分采纳两张、撤销后逐张状态与备注与采纳前完全一致、重做又回到采纳后，没勾的那几张一个字节没动；**授权**：`ensure_local_write` 与画布沙盒守卫在写入前把关。唯独"不确定性"从没在方案这一屏露过面，这一批补的就是它。

**之前什么样。** 翻一份 AI 草案，每张只写着"为什么建议留它"和这一张是按哪档画面问法挑的；模型自己承认的两件事——"这张的画面我其实没看清"和"这个取舍我还不敢定"——只在聊天里的判断卡片和历史观察抽屉里看得到。要看准头，得离开方案回聊天里翻，而勾选采纳恰恰就是在方案这一屏做的决定。

**做了什么。**
- **准头从同一轮任务的回执里就地汇总，只有一份取数规则**：`collectDraftDoubts` 先看判断记录（它同时带判断层与观察层两句话，就以它为准），只有画面观察记录的照片退回观察层；判断层写了观察层留空时，那格用观察记录补上。空白句子、没点名照片的记录一律不算一条不准——**回执里没写过的，界面一行都不编**，不替模型补一句客气的犹豫。
- **草案每一条最多两行小字，分层不混**：一行是"这张的画面观察本身也说不准：……"，一行是"这条取舍 AI 自己也留了不准：……"，各说各的来源层，不把观察层的犹豫包装成判断层的、也不反过来。没留下不准的那张两行都不出现。
- **只做展示，不加通道**：不改表结构、不加命令、不再问模型、不上传照片，草案页的勾选、顺序调整、部分采纳、替换序列、丢弃一路照旧。

**达到什么效果。** 摄影师在勾选之前就能在同一屏看到"这套建议里 AI 自己哪几张留了余地、余地是画面看不清还是取舍不敢定"，和已有的理由、画面依据档位凑成"为什么这么建议 / 凭几张看过像素的观察 / 它自己承认哪儿不准"三件事；配合原有的部分采纳与整体撤销，原计划第 5 项那四样这才算都在屏幕上。

**验证情况。** 前端四道全 exit 0：`npm test`、`npm run lint`（`--max-warnings 0`）、`npm run typecheck`、`npm run contracts:check`。全量 **77 文件 / 710 条全绿**（上批 703，本轮 +7），新增 7 条逐条点名跑过：①判断层两层都说了就以它为准、观察记录不重复覆盖；②判断层留空时退回观察记录，只有观察的照片只出观察那一行；③空白句子与没名字记录被丢掉、任务缺失时结果为空；④渲染时两张都写出来且各只出现一次（没留不准的那张不出现）；⑤⑥⑦英日韩三语各一条，界面不残留中文句子。草案审阅单文件 58 条。四套语言递归扁平各 **1136** 键（新增 2 句），缺失 0、多余 0、占位符逐键一致。Rust 这一批一行未动，但仍在当下这棵树上重跑：`cargo fmt -- --check` 0 差异、`cargo clippy --all-targets --all-features -- -D warnings` 0 警告。

**仍没做到（实话）。** ①这两句准头是模型自陈的文本，不像画面依据那样绑着"对着哪一版算的"版本号与可核对编号；改过一版之后旧的自陈句子仍会跟着那份任务回执显示，不重算、不更新。②画布板草案（便签/分组那一侧的独立草案）没有这两行。③手机端的草案页只搬了画面依据口径，这两行没跟过去。④排版只在 jsdom 里断过文字，没在真 App 里看过两行长句在窄栏里的折行。⑤未提交、未推送。

下一步顺序：①提交、打包、装机（用户已点头，本轮四件事已做完）；②发官网；③装机后真点一遍画布生成草案那格与手机上的画面依据；④边车扫描缓存；⑤把"准头"两句绑上版本号，手机侧补同两行。

## 第四十八批（2026-11-08）：四笔提交 + 本机重打包装机 + 官网只发通自动更新那一半

用户这轮的口径是"编辑回流、AI 与作品草案融合、手机侧画面依据这三件做完，其它不要做，然后开始提交、打包、装机、发官网"。前三件已在第三十七到四十七批收口，这一批只做后面四步，不再新加功能。

**之前什么样。** 从第二十一批起，桌面端四十多批改动**一行都没进过版本库**——全在那棵被协作分身共用、同时躺着几千个 iOS 构建产物的工作树里。本机装的是 1.5.3，屏幕上看到的还是旧行为；官网那头的自动更新清单停在更早版本，新用户下载按钮给的是 1.5.2 的公证 DMG，而页面上两句话写着"现有版本 1.5.0"。

**做了什么。**
- **按主题拆成四笔提交**：本地内核（草案与回执同事务、判重、外部编辑回流、迁移 0038、1.5.4 版本字段）/ 桌面端界面（画面依据与判断依据上屏、契约生成物）/ 提示词跟着口径改 / 路线图与选片口径笔记。每笔只精确暂存自己的路径，分身的 iOS 与 apps-build 产物、apps/api、tools/agent-eval、这一轮被测试重拍的截图**一律留在工作树不进提交**。四笔都没推送。
- **本机重打包并换包**：先把版本号抬到已装版本之上（1.5.3 → 1.5.4）再打，避免降级；打包前清掉两份可再生的增量编译缓存腾出磁盘；`npm run tauri build -- --bundles app` 出 `.app`，Developer ID 严格校验过，旧包整份备份后换下、重启。
- **发官网只走通自动更新这一半**：`latest.json` 写成 1.5.4，更新包用禁 AppleDouble 的 tar 结构、带 `--app-version` 签名，产物与清单一起部署；顺手把上一次发版遗留在官网、但仓库里没有对应文件的历史产物（1.5.1/1.5.2 的 DMG 与更新包）抓回本地目录，避免部署把它们冲没；页面两处版本句改成与下载按钮真给的那份一致的 1.5.2。
- **1.5.4 的公证 DMG 没发**：本机钥匙串里 `FOCALROOM_NOTARY`、`BETWEENFRAMES_NOTARY` 两套公证凭证都已不存在，也没有 API 密钥方式，`notarytool submit` 直接报找不到凭证。这属于要他本人出手的凭证操作，我停下来把命令写清楚，改走"新用户仍下载 1.5.2、装上后由自动更新拿到 1.5.4"这条路。

**达到什么效果。** 他每天在用的那台 App 现在是 1.5.4，四十多批改动第一次进了版本库、也第一次真的上了屏；老用户点开"检查更新"能拿到 1.5.4；官网下载与页面文案不再自相矛盾。

**仍没做到（实话）。** ①1.5.4 的公证 DMG 没发，新用户装的是 1.5.2 再靠自动更新跟上——这条路依赖更新通道，不是直接给到最新安装包。②"老 App 点立即更新"这一端到端真点没做：1.5.4 跑过一次后库里迁移版本号到 38，回不去——旧版本启动会在建库阶段直接崩，所以拿他本机真实库做不了这次演练，我只能改成离线验签：用应用内置公钥对着 BLAKE2B-512 预哈希核更新包签名、再核可信注释上的全局签名，两条都过，且先用一对已知正确的旧签名验过这套算法本身。③界面里那个"立即更新"按钮他还没真点过。④画布生成草案那一格、手机上画面依据那两屏，装机后真点的只有本机这几处入口，手机侧本轮没再上机。⑤提示词那批文案的排版长度只过了长度断言，没逐条在真机上读。⑥四笔提交都在本地分支，没推送、没合并，官网那侧的改动只有一笔模板与文档待提交。

下一步顺序：①把 1.5.4 的公证 DMG 发出去（要先在他机器上重新存一次公证凭证）；②装机后真点一遍画布生成草案那格与手机上的画面依据；③把"准头"两句绑上版本号、手机侧补同两行；④边车扫描缓存；⑤推送与合并需要单独点头。

## 第四十九批（2026-11-08）：图库顶上三条本机自查折成一行，话说短

用户看完装机后的界面说了两句：①"这个部分非常影响图片的阅读和界面的交互啊"（附了图库顶部那三张卡的截图）；②"这里的语言描述 AI 味过重，都是一些过度的解释说明"。这一批只改这两件事，不加功能、不动内核。

**之前什么样。** 选片网格上方长期常驻三张卡片——本地预览状态、历史观察的像素版本、外部软件里的判断——每张带一句"为什么在这里"的说明小字，摊开能占掉半屏；每条结果句子写成"整个项目 120 张：5 张能只读原图重算，3 张得你先把原图找回来，其余现在就能读"这种整段解释。

**做了什么。**
- **三张卡收进一个折叠条**：新增 `LocalFileChecks`，标题行只有"本机自查"加一句边界说明（只读本地文件、不上传照片、不问模型），三条面板挪进展开后才显示的身体里。收起用 `hidden` 而不是卸载，所以数过的数字与名单收起再摊开还在，不用重新点一遍；不点这一行仍然一次盘都不读。
- **顶部只留一行**：三条各自的说明小字删掉，标题与按钮并到一行；原来给三张卡分别写的 padding/边框样式收进折叠条，`.local-file-panels` 那条已无用的规则删掉。
- **文案按"只说事实"重写**：四套语言各 34 个键改短（如"检查预览""重生成坏预览""120 张里 5 张可重生成，3 张要先找回原图""5 条没记像素版本：3 条能补，2 条补不了（涉及 4 张）"），删掉 3 条纯解释的键，新增折叠条标题与边界说明 2 个键。**诚实条款一条没删**：没数到的不算好、没点到的不算跟上、只列前 N 条其余仍在总数里、补不了要单独说、一键只填本地空着的——这些句子只是变短，没有一句被"乐观化"。

**达到什么效果。** 图库顶部从三张卡变成一行，选片网格重新拿到那块位置；要看文件事实时点开，数字与名单和原来一样具体到文件名；四语句子长度大致砍掉一半，读起来像台账而不像说明书。

**验证情况。** 前端四道全 exit 0：`npm test`、`npm run lint`（`--max-warnings 0`）、`npm run typecheck`、`npm run contracts:check`。全量 **77 文件 / 710 条全绿**（数量与上批持平，图库那个文件整份重写为 8 条：默认收起、点行才摊开且摊开不读盘、数过的数字收起再摊开还在、历史观察在同一行、序列侧同样只有这一行且外部判断点得开、网页预览与只读共享不摆这一行、画布不摆、三条能力都没有时不摆空行）。四个改动面板的单测文案随新句子重映射，逐条点名跑过。四套语言扁平键数各 **1135**（上批 1136：删 3 条解释句、加折叠条 2 个键），缺失 0、多余 0。Rust 一行未动。

**仍没做到（实话）。** ①这一批没打包、没上机，他屏幕上现在还是三张卡的 1.5.4；干净打包要先点头（工作树里还混着分身的 iOS 改动）。②真 App 里那一行的高度、折叠动画与窄窗折行只在 jsdom 断过文字，没人工看过。③顶部其它入口（历史工具条、筛选栏）本轮没跟着瘦身，占位是否还偏多没人核对。④没推送、没合并。

下一步顺序：①这批装不装机等他一句话；②装机后真点一遍折叠条与三条按钮；③1.5.4 公证 DMG 仍等他补凭证。
