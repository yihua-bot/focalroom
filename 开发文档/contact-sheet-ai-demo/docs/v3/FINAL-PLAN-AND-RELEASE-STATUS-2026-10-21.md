# FocalRoom 最终计划与上线剩余工作

核对日期：2026-10-21。产品源码基线：main `40c4716fcfe2367c24aaf46549ea7f073f3ebb1e`。核对期间 HEAD 从 24a1451 推进到 40c4716，因此旧复核中的 UUID/TEXT 查询错误不再列为当前未修缺陷。

范围：工程宝 v3 最终蓝图 G0–G8，以及用户随后补充的项目中心、Research & Moodboard 和贯穿创作循环的方向。G9 专业服务为可选后续。原始 Backlog 有 53 个任务编号，其中 FR3-053 属 G9；原 CSV 的 NOT_STARTED_TARGET 是目标基线，不是当前执行状态，不能直接用来统计完成率。

## 1. 当前结论

现有项目导入、人工选片、照片检视、作品序列、版本与导出，已有可用基础；桌面登录、账户资料、官方 AI 账户账本、本地审美记忆、Web 私密评片也有实现和既往验证。v3 增量已有资产表与映射、基础画布、画布 API、外部回流原型和出版页面初稿。

完整 v3 仍未交付。剩余工作同时涉及核心编码、跨模块集成、真实世界验收和正式发行，不能概括成“再修三个 bug”或“只差签名打包”。本报告不提供以测试数量推算的完成百分比。

### 本轮新增核实

- 40c4716 已修复节点 JOIN 的 UUID/TEXT 比较、Agent 的跨项目 Board 读取及 official/frozen 写入限制、嵌套照片范围和 Blob URL 清理。
- 本轮实际执行 `node apps/api/test/run.mjs --test apps/api/test/boards.mjs`：4/4 通过，退出码 0。日志 `/tmp/focalroom-40c4716-boards-tests.log`，隔离 PG 目录 `/var/folders/8v/3tww3vkx10s77klhf8y_7hz80000gn/T/focalroom-pg-tests-KjGprZ`。
- 新提交的远端 CI 为 [39572613518](https://github.com/yihua-bot/focalroom/actions/runs/39572613518)，本次查询时尚未有完整成功结果。旧 dd99781/fc2e873 的绿色记录不替代新候选。
- 公网首页 https://focroom.cc/ 已是新版定位，sitemap.xml 返回 XML，Google 验证文件返回 200 且 53 字节正文与附件一致；团队页面 https://team.focroom.cc/shared 返回 200。此前官网待上传状态已被后续公开发布取代。
- 四语共 16 个公开页面实际请求均返回 200，self-canonical 与各自 URL 一致；[公开页面证据](evidence/readiness-40c4716-2026-10-21/public-http.json)。
- 当前公开下载实际取得 7,263,662 字节，SHA-256 为 `9c9f51a9f85430c7173640f5a2af369ac10531d11c6a356f0316b9a5d50fb6c6`，与既有 1.0.4 相同，不是本次 40c4716 的新构建；本轮没有重新核验其签名/公证。[下载证据](evidence/readiness-40c4716-2026-10-21/public-release.json)
- [本轮 PostgreSQL 画布测试日志](evidence/readiness-40c4716-2026-10-21/pg-boards.txt)已随报告保存。
- 页面可访问不证明 Google 账户已完成验证、搜索引擎已收录或生产 API/数据库已部署当前源码。本轮没有登录 Search Console 或产品账户，没有读取腾讯云运行配置。

## 2. 当前 H0 还需要收口什么

| 项目 | 已推进 | 仍需完成 |
|---|---|---|
| H0-01 基线 | 历史全量和 CI 已恢复过；最新 PG 画布通过 | 同一最终候选的完整 CI、状态快照与独立原生 UI 验收 |
| H0-02 桌面接线 | 实际 Repository、图像引用、导航、重启旅程已补 | 原生 WKWebView 拖拽/快捷键/窗口/系统选择器验证，不能以浏览器桥接替代 |
| H0-03 Agent | project_id、基础 SQL、项目/状态/照片检查已修 | 任务专属草案身份；人工差异采纳；工具注册与条件发送；预算/恢复和嵌套权限回归 |
| H0-04 Web 画布 | 项目入口、列表参数、API ACL、节点别名与票据代码；PG 4/4 | 带真实 alias 和照片的完整浏览器旅程、映射回填、可见错误、权限 UI、撤权/切项目；正式共享命令仍未闭环 |
| H0-05 出版初稿 | 创建、非空分页、布局边界、防覆盖、缩略页面查看 | 可编辑版本与正式命令；不能当作完整出版引擎 |
| H0-06 外部回流 | manifest、匹配/记录函数和插件原型存在 | 文件真实 hash/尺寸/ICC、受控目录和符号链接、项目/Provider/Catalog/Variant 匹配、重复批次与恢复 |
| H0-07 历史 | 审计事件不再 UPDATE；日志错误传播；锁定移动原子失败 | Canvas/Publication envelope、revision、持久幂等回执、追加审计、Undo/Redo/冲突/重启；便签/箭头/笔迹等完整覆盖 |
| H0-08 ICC | 已有有界解析和异常输入测试 | 缺失/无效/不支持的诊断、更多容器与真实 profile 样本；后续原生色彩变换 |

注意：目前 Agent 允许向项目中的 draft Board 放图，但 boards 模型没有任务拥有者绑定。人工正在编辑的 draft 不是天然属于该 Agent 任务的沙盒，仍须建立任务草案隔离和采纳命令。

## 3. 按最终计划逐项剩余

### G0 · 基线与迁移（FR3-001–005）

已有 CI、隔离 PG runner、旧库副本升级/恢复及原图 hash 回归。接续工作：

| 编号 | 剩余或须持续维护的内容 |
|---|---|
| 001 | current-state-v3.json 仍指向 dd99781，应在验证完成后更新为真实候选 SHA 和结果 |
| 002 | 保持登录/评片回归，并增加实际画布、出版与新增导航旅程 |
| 003 | 最新源码完整门禁与独立原生构建通过；只跑单测不能关闭 |
| 004 | 新资产、Board、Rendition、出版引用加入旧库兼容、中断升级与恢复演练 |
| 005 | 架构、迁移 registry、Mock/真实边界及状态文档随候选更新 |

### G1 · 稳定资产与版本（FR3-006–012）

已有本地资产五表、云端资产表、alias、导入双写和旧照片回填。仍需：

| 编号 | 剩余 |
|---|---|
| 006 | 真实元数据/locator/hash 的完整身份链；修回填表达尺寸按 width/4、height/4 猜测的问题 |
| 007 | 多 Variant / 不可变 Rendition、唯一当前指针、历史引用和版本选择完整链路 |
| 008 | 同资产多 Board/节点的引用、分组归属/无环及一致性约束 |
| 009 | 外部软件、Catalog、照片、Variant 稳定身份与批次回执 |
| 010 | 中断/重复回填、旧草案/评论/序列引用兼容；云端 alias 写入与媒体映射的实际接线 |
| 011 | 多路径/security bookmark、离线/移动/重连与内容变化检测 |
| 012 | Rust/TS/云端统一 v3 命令和回执，协议能力与未知版本处理 |

当前云端读取 UUID→TEXT 比较已修复；这不证明云资产映射已经在实际导入/共享时完整生成。节点接口允许无 alias 返回空 photo_id，现有 PG 读取测试不等于照片一定显示。

### G2 · 专业色彩与大图（FR3-013–019）

`src-tauri/src/color_engine.rs` 明确仍以 profile 检测、digest 和管线枚举为主。

| 编号 | 剩余 |
|---|---|
| 013 | ICC v2/v4、灰阶/扫描 profile、未知/无效诊断及真实样本库 |
| 014 | ImageIO/ColorSync/CoreImage 等原生路径；16-bit/float、真实目标空间转换及 provenance |
| 015 | 100MP/巨幅 TIFF 局部解码、瓦片、取消、缓存完整失效与内存边界 |
| 016 | 原生 Fit/100%/A-B、目标显示器变化、软打样、窗口与辅助功能集成 |
| 017 | 多家 RAW 的内嵌/派生/原生/外部显影来源标示和实际支持矩阵 |
| 018 | Web 受权代理的 profile、色彩能力和兼容性边界 |
| 019 | 校准屏/颜色计、多显示器、P3/AdobeRGB/ProPhoto/扫描件验证 |

四个 PipelineTarget 名称和 1400px 代理不足以证明高位深显示或 PrintMaster 已交付。已有原图检视与 CR2 内嵌预览保留，但不是上述全部要求。

### G3 · 完整叙事画布（FR3-020–026）

已有创建 Board、放图、拖动、缩放、便签、箭头、画笔及基础评论。仍需：

| 编号 | 剩余 |
|---|---|
| 020 | culling、大画布与 100+ 图、不同 DPR/缩放下交互精度和性能 |
| 021 | 框选/多选、resize、分组、锁定完整交互；可见且真实工作的 Undo/Redo |
| 022 | 既有便签编辑、矢量笔迹选择/删除、压力和连接器更新的完整历史 |
| 023 | 节点/局部归一化评论、回复/解决/重开及对象变化后的锚点追踪 |
| 024 | Canvas 与独立 Sequence 的双向引用/提取及版本，不破坏原 Board |
| 025 | 对象树、快捷键/辅助技术、画布上下文 Agent 与完整创作任务可用性 |
| 026 | 与 G2 共用可信像素/色彩转换并验证各视图一致性 |

目前工具条没有完整选择/分组/尺寸和 Undo/Redo 工作面；后端 undo 函数存在不等于用户已经可以完成整个交互。

### G4 · 真正的画布 Agent（FR3-027–032）

已有任务引擎、预算、观察、检索、模型 Adapter、部分草案工具、本地记忆和评测工具。仍需：

| 编号 | 剩余 |
|---|---|
| 027 | task-owned Board、完整工具注册、草稿专用 move/group/note/connector、嵌套资源校验与人工采纳 |
| 028 | Board/选区/Rendition/研究资料和观察证据的长链上下文、版本失效与恢复验证 |
| 029 | 条件性工具发送；任务创建/恢复绑定工具集；实际输入预算回归和经摄影质量测量的模型路由 |
| 030 | 现有本地证据记忆继续复用；云作者记忆/Creator 隔离完成实际 schema、路由、检索与权限接线 |
| 031 | Board/页对象引用、变化叠层、逐项采纳/拒绝、旧基线与撤权处理 |
| 032 | 累计至少 30 个授权任务、至少 10 位独立摄影师/编辑，人工基线、失败分母和盲评 |

云作者记忆文件当前仍引用 cloud_projects，未在服务端主路由和 Worker 中找到调用。已有代码不是可用的云作者记忆服务。条件性发送也不能代替真实权限校验。

### G5 · Lightroom / Capture One 往返（FR3-033–037）

| 编号 | 剩余 |
|---|---|
| 033 | 把 Lightroom 原型变成目标版本实际可加载的插件：入口/SDK 回调、JSON/时间、Collection/导出和稳定身份 |
| 034 | 实现并验证 Capture One Catalog/Session/Variant 桥接 |
| 035 | 安全 manifest 状态机、后台监测/恢复/人工确认，真实文件和路径校验，幂等回执 |
| 036 | 原版/调色版/多 Variant 对比、未匹配/歧义/冲突的操作界面 |
| 037 | 回流更新当前预览与冻结出版物引用分离，旧 Edition 不暗改 |

当前 process_manifest 信任声明的 render_sha256/尺寸/ICC，使用 join(render_filename)，没有完成受控真实文件校验；函数未接到完整后台工作流。重复 Rendition 的 INSERT OR IGNORE 仍返回新生成而可能未插入的 ID。插件只有两个原型文件，没有 Capture One 实现。

### G6 · 正式共享工作空间（FR3-038–042）

| 编号 | 剩余 |
|---|---|
| 038 | 桌面 Board 进入共享权威、草稿增量、正式语义命令、同节点冲突和离线重放 |
| 039 | Board/批注/采纳/输出等权限、像素票据、撤权后停止新请求，跨账户回归 |
| 040 | Web 实际画布和照片、Diff/审批、错误状态及切项目完整浏览器旅程 |
| 041 | 已有 PostgreSQL 跨实例 Presence；仍须覆盖 Board 节点状态、游标恢复及真实多实例一致性 |
| 042 | 至少 5 组真实异地摄影师/编辑双人项目 |

既有照片评论、提案审批、Web 登录及 Presence 不是零基础；新增 Board 的正式共编和回执体系仍需完整实现。当前 Board API 四项测试通过，仅覆盖发现、权限、节点/批注和撤权等现有场景。

### G7 · 出版与印前（FR3-043–047）

| 编号 | 剩余 |
|---|---|
| 043 | Edition Snapshot、跨页/母版/文字/页码、可编辑图片框与版式，保存/恢复/版本比较 |
| 044 | 按实际墙面/装框尺寸编排、间距/观看高度、带尺寸安装图与作品清单 |
| 045 | 从指定高分辨率 Rendition 渲染生产母版，不从缩略图或 AI JPEG 放大 |
| 046 | 供应商 PrintContract、PPI/字体/出血/ICC/OutputIntent 等真实预检和审批阻断 |
| 047 | 指定 PDF/X 或微喷 TIFF、独立预检工具、真实印厂/输出方接受及实体样书 |

目前 PublicationPanel 提供创建、每页 1/2/4 图生成和缩略页面查看；尚无上述完整页面编辑和生产输出。pub_type/state 的枚举与 print_contracts 表不构成交付证据。

### G8 · 正式客户端与商业服务（FR3-048–052）

| 编号 | 剩余 |
|---|---|
| 048 | 最终候选 SHA 的正式打包、Developer ID、Hardened Runtime、公证/票据、Gatekeeper、更新与回滚；统一系统版本说明 |
| 049 | v3 新数据/引用/代理/交付文件纳入备份与恢复验证；生产容量、RPO/RTO、报警和故障演练 |
| 050 | 新增画布/回流/出版/账户边界的隐私、安全、撤权和私密内容泄漏验收 |
| 051 | 官方 AI 的真实生产可用性与额度发放、账单核对；完整商业账户/团队计费/收退款/发票与售后 |
| 052 | 干净 Mac 和真实账户完成 500 素材到摄影书/印前交付的全链路 |

已有 Developer ID 配置、历史签名/公证发行记录和公开 1.0.4 下载，不再将证书或下载入口列为从未建立。仍须对包含新 v3 源码的最终候选重新验证，历史包不包含当前所有新功能。

当前 Tauri 配置仍写 minimumSystemVersion=10.15，既有分发包 Mach-O 要求 11.0；官网已改 11+，发行配置也需要统一。存在本地支付事件账本与官方 AI tokens 账本，但未找到完整对外 checkout/订阅/发票闭环，不能以账本代码存在认定商业服务完成。

## 4. 用户补充的 Research & Moodboard

空项目和 brief 已有基础，不重新当作从零创建项目。尚需：

1. 项目内研究笔记、参考链接/参考图、来源/作者及使用范围。
2. 参考资料身份与作品资产分离，避免进入选片数量、正式序列和出版输出。
3. 复用画布的研究板用途、参考节点与受控链接；当前节点 CHECK 只有八种类型。
4. Agent 读取研究资料的授权、来源与版本绑定；外部参考文字作为不可信数据。
5. 人工选择、回流版本、画布/序列、出版引用和讨论持续关联，历史理由不由 AI 补造。
6. 研究资料进入备份/恢复和演示项目，真实验证没有照片也能开始研究。

这部分并入 G1/G3/G4 和最终旅程，不新增一套独立画布引擎，不把十二个创作阶段机械拆成十二个导航模块。

## 5. 必须独立取得的验收

| 类别 | 仍需的证据 |
|---|---|
| HUMAN | ≥30 个真实授权创作任务、≥10 位摄影师/编辑盲评；模拟结果单列 |
| HUMAN | ≥5 组摄影师/编辑异地双人项目，含真实意见和故障处理 |
| DEVICE | 校准显示器/颜色计、多屏、100MP、16-bit TIFF、RAW 及目标 Mac 的性能/可访问性 |
| SUPPLIER | LR/C1 各目标版本的 30 张往返、多次发布/多 Catalog 或 Session |
| SUPPLIER | 32 页摄影书、独立 PDF/X/输出预检、实体印厂/微喷接受 |
| PROD | 新客户端签名安装/登录/演示/升级恢复，真实 AI 额度与账单、云部署和灾备 |

## 6. 官网与上线清单

已在线：新版产品定位、公开主页、sitemap、Google 验证文件、现有 1.0.4 下载入口与团队登录页面。本轮仅核对公开页面，不使用真实账户或生产密钥。

仍需确认：

- 四语内容的独立母语审阅、站长平台的实际所有权验证、sitemap 提交与索引结果；文件可访问不等于 Google 已点验成功，也不保证生成式引用。
- 新候选客户端/云服务与官网能力说明一致；官网没有把 Research/出版等规划写成现有完整能力。
- 腾讯云实际运行的镜像 SHA、迁移 registry、Worker/官方 AI 状态与额度。本轮未核对运行主机，不声称这些已同步到 40c4716。
- 对最终候选做新 Mac 安装、系统最低版本、登录回调/钥匙串、演示项目重启、真实 AI、共享协作和原图保护验收。
- 下载包、签名/公证、checksum、版本号和更新入口一致；保留可用回滚包及数据库恢复策略。
- 生产监控/告警、容量与备份还原、用户支持渠道及实际服务条款/数据处理说明符合当前服务范围。

## 7. 推荐推进顺序

1. 收口 H0：当前候选 CI/原生与真实画布旅程，任务草案/采纳和统一命令历史，安全回流边界。
2. 完成 G1 身份与引用，并开展 G2 原生色彩技术验证；将 Research 的最小模型纳入契约。
3. 完成 G3/G4 创作闭环；同步组织真人评测，不等编码结束才找参与者。
4. 完成 G5 真实调色往返和 G6 正式共享；使用同一个项目反复修改验证。
5. 完成 G7 编辑、母版、预检和供应商打样。
6. G8 固定候选、生产部署、正式客户端和全旅程验收，再将最终能力对外发布。

如果决定先发行现有选片/序列范围，应明确另立较小的版本承诺和验收清单；该发行不关闭 v3 的 G2/G5/G7 等目标。本文没有自动缩减原计划或给出未经测量的工期。

## 8. 关键源码入口

- [资产回填](../../between-frames/src-tauri/src/db.rs)、[导入双写](../../between-frames/src-tauri/src/resource_commands.rs)
- [色彩模块](../../between-frames/src-tauri/src/color_engine.rs)
- [桌面画布](../../between-frames/src/features/canvas/NarrativeCanvas.tsx)、[画布命令](../../between-frames/src-tauri/src/canvas.rs)
- [Agent 工具](../../between-frames/src-tauri/src/agent/tools.rs)、[PolicyGate](../../between-frames/src-tauri/src/agent/policy.rs)
- [外部回流](../../between-frames/src-tauri/src/external_edit.rs)、[LR 原型](../../between-frames/bridges/lightroom-classic/FocalRoomExport.lua)
- [Web 画布](../../between-frames/apps/web/src/shared/BoardViewer.tsx)、[云画布 API](../../between-frames/apps/api/src/boards.ts)
- [出版 UI](../../between-frames/src/features/publishing/PublicationPanel.tsx)、[出版后端](../../between-frames/src-tauri/src/publishing.rs)
- [官方 AI 账户](../../between-frames/apps/api/src/hosted-ai.ts)、[云作者记忆草案](../../between-frames/apps/api/src/author-memory.ts)
- [发行配置](../../between-frames/src-tauri/tauri.conf.json)、[旧版发行与待验记录](../DEVELOPER-HANDOFF-2026-10-19.md)

本文只报告已核对的源码、此次定向测试及公开 HTTP 响应；没有重跑本地全部 Rust/前端套件，没有迁移用户库、真实模型调用、生产发布或 git commit/push。
