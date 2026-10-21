# FocalRoom v3 · H0 修复与后续执行记录

日期：2026-10-21。基线：`4ca66a1fc6ed71622f51b0923f3147c7ad05174a`。
工作分支：`codex/v3-h0-hardening`。

本记录及证据目录保存提交前的本地验收快照；提交后的远端 CI 状态以对应 PR 的检查记录为准。

**本轮交付是修复已有 v3 模块、恢复可信验证基线，不代表 G0–G8 全部完成。**
本文件与[实施交接文档](IMPLEMENTATION-AND-MULTI-AGENT-HANDOFF-2026-10-21.md)配套使用。
后者记录基线审计及全部 FR3 条目的执行顺序；本文件记录这批实际修改、验证和剩余工作。

## 1. 修改范围

### 云画布权限与接口

- `BoardService` 复用 `AuthorityService.scope` 的项目权限、事务锁和 ACL 审计。
- 列表、创建、节点读取、评论读取、添加及解决评论均校验项目访问权；迁移未完成时不开放画布。
- Viewer 只能读取；Reviewer 可创建草稿、评论并处理自己的评论；项目编辑或具备审批权限者可处理他人评论。
- Cookie 写入继续要求 Origin 和 CSRF，撤销项目授权后立即停止既有画布访问。
- 补齐节点、批注与解决批注路由；严格校验 UUID、必填字段、有限坐标、文本长度和布尔值，不做隐式类型转换或静默截断。
- 这些接口只处理草稿/批注，不隐式推进正式作品 revision。
- 新增隔离 PostgreSQL 的真实 HTTP 测试，覆盖匿名、跨项目、角色、撤权、CSRF、输入异常和只读正式状态。

入口：[boards.ts](../../between-frames/apps/api/src/boards.ts)、[server.ts](../../between-frames/apps/api/src/server.ts)、[boards.mjs](../../between-frames/apps/api/test/boards.mjs)。

### 迁移回归

- 精确迁移版本断言补入已登记的 0017/0018，没有移除精确比较或跳过套件。
- 补验七张资产/画布表存在、应用角色具备 DML 且不具备 TRUNCATE。
- 历史 SQL 迁移文件未修改；未连接生产数据库。

入口：[account-security.mjs](../../between-frames/apps/api/test/account-security.mjs)、[min_privilege.mjs](../../between-frames/apps/api/test/min_privilege.mjs)。

### 桌面画布与出版物接线

- 新增类型化 Canvas/Publication Repository，由 TauriRepository 经 AppController/Workspace 注入。
- Rust 的 Board/Publication snake_case 与前端 camelCase 显式转换；IPC 回执通过 schema 校验。
- Canvas 与 Publication 的领域接口独立于 React 组件；共用 IPC 错误转换，避免错误对象被吞成空页面。
- 资产别名关联到项目内缓存路径后显示真实照片；没有任意文件路径输入接口。
- 从选片工作台进入画布保留照片选区；切换项目仍重置，避免跨项目遗留状态。
- 画布、出版物不再同时渲染旧作品序列；共享/迁移中项目的新增写按钮禁用，后端仍校验权限。
- 修复便签/箭头 SQL 参数数量及画笔图层/旋转写入位置；增加跨资产、节点类型、坐标与重启测试。
- 照片、便签、箭头和笔迹分别渲染；连接需明确启用连接模式。
- 便签先输入内容再保存，不再显示无法兑现的“双击编辑”占位提示；新便签避开当前重叠的照片。
- 取消指针不会提交拖拽，移动写入失败后重读实际节点。
- 暂不展示尚未接通的画布撤销按钮；旧选片历史只在选片/作品序列中展示。画布正式 Undo/Redo 仍见 H0-07。

入口：[Workspace.tsx](../../between-frames/src/app/Workspace.tsx)、[NarrativeCanvas.tsx](../../between-frames/src/features/canvas/NarrativeCanvas.tsx)、[TauriCanvasRepository.ts](../../between-frames/src/services/TauriCanvasRepository.ts)、[canvas.rs](../../between-frames/src-tauri/src/canvas.rs)。

### 非空出版生成与页面初稿

- 修复封面/内页 SQL 的列值数量、绑定顺序及重复边距。
- 创建时校验开本，生成仅接受每页 1/2/4 张。
- 先校验完整序列的项目资产；缺失资产不静默跳过成空白页。
- 只有 draft 可生成；已有页面时返回冲突，要求新建另一版，不删除既有页面/文字。
- 增加有照片的封面、分页顺序、框内边距、重启持久化、跨项目、错误规格与失败不落页测试。
- 新增按项目读取页面接口，支持展开封面与内页的缩略布局，使用已关联的缓存照片。
- 页面预览为缩略布局参考，不代表高分辨率母版、校样或 PDF/X 已实现。

入口：[publishing.rs](../../between-frames/src-tauri/src/publishing.rs)、[PublicationPanel.tsx](../../between-frames/src/features/publishing/PublicationPanel.tsx)、[TauriPublicationRepository.ts](../../between-frames/src/services/TauriPublicationRepository.ts)。

### ICC 读取安全

- 用 seek 按段读取替代整文件读取，不把大 TIFF 像素整体装入内存。
- JPEG 校验分片序号、总数、重复和完整性，按序组装。
- PNG 校验 iCCP 名称/压缩方式，压缩输入及解压输出均设上限。
- 经典 TIFF 按字节序定位第一 IFD 的 34675 标签，不搜索像素中的 acsp。
- ICC 校验声明长度、签名、tag table 和数据范围；正确读取 desc 的 ASCII 长度及 mluc UTF-16 文本。
- 测试覆盖截断、恶意长度、分片异常、解压上限、假 acsp、大小端与 2GB 稀疏 TIFF。
- `valid` 只表示所检查的结构可安全读取，不能解释为色彩转换、视觉一致性或印刷合规。
- BigTIFF/HEIF/相机 RAW profile、后续 IFD、多状态诊断和真实参考 profile corpus 仍需后续补齐。

入口：[color_engine.rs](../../between-frames/src-tauri/src/color_engine.rs)。

实现参考：[ICC 官方结构说明](https://github.com/InternationalColorConsortium/iccDEV/blob/master/iccdev-mcp/docs/icc-format-guide.md)、[W3C PNG iCCP 规范](https://www.w3.org/TR/png-3/#11iCCP)。这些参考仅用于元数据解析；本轮没有实现 CMM。

## 2. 验证与证据

最终候选源码冻结后执行 `npm run verify:v2`，退出码 0。验证前后 33 个候选产品源码/配置文件 SHA-256 一致；完整日志与源码清单均已归档。

| 检查 | 实际结果 |
|---|---|
| 契约、typecheck、lint | 通过 |
| 前端单测 | 41 个文件，274 项通过 |
| Rust 默认回归 | 239 通过；15 ignored，另过滤 1 项钥匙串写删测试 |
| Rust fmt / 严格 Clippy | 通过 |
| 浏览器预览 E2E | 18 通过 |
| 真实 Rust Store 旅程 | 7 通过，含新增画布/出版旅程 |
| 隔离 PostgreSQL API | 11 套件，76 通过 |
| 三会话 Web / Rust Promotion | 通过 |
| 500 图与异常/Retina/RAW 边界旅程 | 4 通过；500 原图 hash 一致，重启快照一致，AI task 0 |
| 工作区构建 / verify:v2 | 通过 |
| 独立 Tauri Review.app | debug / --no-sign 构建通过；未执行原生 UI 验收，不能作为正式分发包 |
| 远端 CI / 正式签名发布 | 本轮未执行 |

证据：[完整门禁日志](evidence/h0-2026-10-21/verify-v2.txt)、[结构化结果](evidence/h0-2026-10-21/test-summary.json)、[候选源码指纹](evidence/h0-2026-10-21/candidate-source-manifest.json)、[原生构建日志](evidence/h0-2026-10-21/native-review-build.txt)、[500 图采样](evidence/h0-2026-10-21/m1-500-gui-metrics.json)。

ignored/过滤项没有计入通过数，包括真实模型、系统钥匙串、Apple 原生向量、额外 HEIC/CR2 夹具、性能基准和真实 Polar webhook。500 图旅程是浏览器加真实 Rust/SQLite 桥接，系统选择器由夹具模拟，不等于原生 WKWebView 或总应用/GPU 资源验收。

- 新增真实 Rust Store 浏览器旅程：导入实际夹具、选片、画布照片/便签/连接、生成非空出版页、缩略预览、重复生成拒绝及重启恢复。
- 原有固定 200 张导入断言仍由 local-core 旅程覆盖；新旅程使用 runner 中实际存在的夹具数量，兼容之前旅程有意重定位的一个文件。
- 浏览器内预览测试不运行需要真实 Rust harness 的 v3 旅程。
- 真实图像路径走项目资产别名和缓存，测试使用隔离目录；未打开用户数据库、调用真实 AI 或更改生产服务。
- 自动测试生成的历史截图不作为本轮产品源码变更；本轮新截图保留在独立证据目录。

界面证据：

- [画布](evidence/h0-2026-10-21/canvas.png)
- [页面初稿](evidence/h0-2026-10-21/publication.png)

构建警告须保留：既有大 chunk、语言文件和 Tauri event 的静态/动态导入共存提示，不属于本轮已解决项目。

## 3. H0 的剩余工作

| 交接项 | 本轮实际推进 | 仍需完成 |
|---|---|---|
| H0-01 | 精确迁移回归、资产/画布表与最小权限验证 | 新候选提交的远端 CI；状态快照与原生验收仍需对应同一候选 |
| H0-02 | 真实 Repository 接线、图像、导航和重启旅程 | 独立 WKWebView 原生 UI/拖拽验收；不把浏览器桥接当原生完成 |
| H0-03 | 本轮未修改 | Canvas 工具 strict schema/PolicyGate 同步、任务拥有的草稿、嵌套资源 scope、正式人类采纳 |
| H0-04 | 服务端 ACL、节点/批注/解决接口与真实 PG 回归 | Web BoardViewer 接入项目页面、图片票据、权限 UI；正式画布共享命令与冲突机制 |
| H0-05 | 非空分页、布局边界、防覆盖及缩略页预览 | 出版命令/版本机制仍与 H0-07 联动，不算完整 G7 |
| H0-06 | 本轮未修改 | 真实文件 hash/尺寸/ICC、目录与符号链接边界、项目/Provider/Catalog/Variant 完整匹配、重复批次回执 |
| H0-07 | 隐藏不可用的画布 Undo，未修改审计触发器 | Canvas/Publication envelope、revision、幂等、追加审计、Undo/Redo/冲突与恢复；旧 undo_canvas 的 UPDATE append-only 逻辑仍须替换 |
| H0-08 | 有界解析、异常输入与专项测试 | absent/invalid/unsupported 的明确诊断模型、更多容器与真实 profile corpus；随后才做 G2 原生色彩 |

**H0 未全部关闭。** 不应将这批通过的测试改写成“全部 Gate 验收完成”。

## 4. 全计划后续顺序

1. 收口 H0：先命令/历史与 Agent 草案边界，再完成安全回流和 Web 接线；独立评审包验收后建立新 CI 基线。
2. G1：完成稳定资产身份、不可变 Rendition、兼容引用与恢复验收，不能只以建表计完成。
3. G2：原生 ICC/CMM、16-bit/float、瓦片/大图和跨显示器技术验证；当前模块仍是安全元数据读取。
4. G3：节点选择/尺寸/分组/文字编辑、真正 Undo/Redo、100+ 照片交互验收。
5. G4：任务草案到人类采纳的闭环，工具契约与实际写链路一致；完成策展质量评测。
6. G5：真正的 LR/C1 桥接、受控回流状态机和真实软件往返；当前插件骨架不能当已验收插件。
7. G6：正式共享画布、媒体引用、Web 查看批注、并发冲突、离线/撤权/多实例和双人验收。
8. G7：Edition/页面编辑/文字与版式、PrintContract/预检、真实高分辨率输出及印厂验证；缩略初稿不替代这些要求。
9. G8：最终候选的签名/公证、新 Mac 安装/升级/恢复、账户额度/真实邮件/生产服务和官网分发验证。旧版本验收不能代替新候选。

独立资源验收仍包括真实摄影任务与独立摄影师、真实双人项目、显示器/高像素设备、Lightroom/Capture One、印厂输出。模拟评审只用于调试，不计为独立真人验收。

详细 FR3 编号、边界和验收标准以实施交接文档及工程宝原始蓝图为准。G9 保持可选，不提前扩大范围。

## 5. 工程原则与发布边界

- DRY/SOLID：复用 AuthorityService 与 IPC 错误适配，领域接口与组件、平台适配分离。
- KISS：严格输入/回执，先保证可见、可恢复的初稿流程；已有页面拒绝覆盖。
- YAGNI：未引入新依赖或新 schema；未伪装原生色彩、正式出版输出或不可用的 Undo。
- 变更位于独立 worktree，未覆盖主工作目录；本批不合并主分支、部署生产或分发安装包。
- 用户已在后续消息明确确认本批 git commit、git push 和创建 PR；不复用历史 G0 的确认。候选源码指纹保留提交前状态。
