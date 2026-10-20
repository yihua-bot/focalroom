# FocalRoom v3.0｜最终产品与工程实施蓝图

> **文件性质：** 规范性设计文件（Normative Baseline） · **发布日期：** 2026-10-20 · **状态：** 目标架构 / 待实施，不代表现有仓库已交付全部能力  
> **产品定位：** AI-Native Photography Creation Workspace（摄影创作者的 Figma，但以影像保真、叙事编辑和完整交付为专业壁垒）  
> **范围：** 本地 Mac 专业工作台、Web 协作工作台、专业摄影 Agent、外部 RAW 软件往返、摄影书/展览/印刷工作流、云端协作与专业服务  
> **战略约束：** 不以最短开发周期或最低成本为优化目标；以可证明的最佳用户体验、准确性、保真、安全性和可维护性作为设计优先级。

## 00｜执行摘要：最终要打造什么

FocalRoom 不是一款只会“筛选好照片”的 AI 工具，也不是 Lightroom 的替代调色引擎。它是一套从 **原始影像 → 初选与复选 → 编辑判断 → 外部专业调色 → 成片版本回流 → 自由叙事画布 → 摄影书/展览编排 → 软打样 → 印前/交付 → 后续复盘** 的项目级创作系统。摄影师、摄影编辑、策展人、工作室和摄影学校在同一项目语义、同一批影像资产及同一套版本历史上工作；AI Agent 以可审批的创作协作者身份参与，不拥有未经授权的正式修改权。

**最终成功的定义不是功能清单已勾选，而是：** 一位摄影师能以 FocalRoom 为创作入口，完成一个 500 张原始图像的项目，从其中选出 30 张，送到 Lightroom Classic 或 Capture One 调色并准确回流，组织成数版有专业说服力的叙事画布，与编辑远程讨论并修订，制作摄影书或展览版本，最终交付色彩、图像和印前参数经过验证的文件；全过程的创作意图、评论、版本、资产身份和 AI 建议均未丢失。

**项目交付四层次：**（1）工程实现；（2）自动化验证；（3）真实设备、专业摄影师和印厂等外部验收；（4）真实可发布、可收费的运营能力。任何阶段都不得用“代码存在”“模型成功返回 JSON”“生成了 PDF”冒充后两者。

### 00.1 六个产品系统与一个共享内核

| 系统 | 核心价值 | 用户在里面完成什么 |
|---|---|---|
| **Library & Color Engine** | 原始影像可信地被管理、观看 | 发现、导入、打分、对比、原图高精度检视、版本切换 |
| **Photography Agent** | 更高质量的摄影理解与可执行编辑建议 | 检索、批评、选片、叙事建构、解释、学习偏好、执行可审查草案 |
| **Narrative Canvas** | 视觉思考和项目编排的中心 | 自由摆放、拖拽、缩放、分组、比较、便签、箭头、涂鸦、共同评审 |
| **Creative Pipeline** | 外部调色往返闭环 | 同步选片到 Lightroom/Capture One、获取调色版、回流关联、冲突处理 |
| **Publishing Studio** | 从作品序列到可制造的输出 | 摄影书、展览墙、作品集、校样、软打样、印前交付 |
| **Collaborative Workspace** | 与人共同完成作品 | 邀请、角色、在线协同、批注、提案、版本审签、专业服务委托 |

它们必须复用同一组 Asset / Variant / Rendition / Board / Annotation / Publication / ProjectRevision 领域对象。同一幅影像不得在每个功能里变成互不相关的一张“图片”。

### 00.2 十条硬性产品原则

1. **原图绝不修改：** 来源资产只读；原始文件内容哈希保存，任何导出、调色回流与同步不能默默覆盖原始文件。
2. **色彩保真是基础能力：** 高质量人类检视与 AI JPEG 完全分管线；ICC、位深、变体身份及色彩提示不可缺省造假。
3. **工作台优先而非聊天优先：** 选片、画布、排版与历史始终可视，Agent 作为嵌入式能力参加同一个画面。
4. **人类对正式作品有最终控制权：** Agent 只能生成可解释、可预览、可撤销的草案与可审批操作。
5. **自由探索先于线性定稿：** Canvas 不是固定网格；同一照片可在不同 Board 里多次引用，不能因自由拖拽破坏原始资产。
6. **专业外部软件是合作伙伴：** 不依赖破解或逆向编辑 Lightroom/Capture One 私有目录；通过官方 SDK、导出配方和稳定身份契约集成。
7. **完整创作意图可恢复：** 版面、便签、作者判断、AI 证据、评论及版本都属于作品过程，能够查询并撤销。
8. **协作遵循明确权威：** 本地项目本地为准；提升为共享项目后服务器为共享正式状态权威；离线操作采用待提交分支与冲突合并。
9. **专业级交付不可降格：** 校样 PDF、正式印刷 PDF、艺术微喷 TIFF 是不同交付类型，输出时明确色彩/ICC/像素/出血等要求。
10. **每个能力都有真实世界测量门槛：** 摄影师盲评、色彩参考对照、重负载、慢网协作、真实插件往返和印厂检验均为独立验收。

## 01｜用户、任务、目标体验

### 01.1 关键角色

| 角色 | 最重要的任务 | 必备交互 |
|---|---|---|
| 独立艺术摄影师 | 长期个人项目、慢速编辑、审美探索、摄影书 | 跨年资料整理、无压力画布、版本分支、私密创作、个人审美记忆 |
| 商业/人像摄影师 | 数千张快速初选、客户审片、批量回流 | 极速浏览、旗标/评级同步、选片集合、修图外包流、交付集合 |
| 摄影编辑/策展人 | 叙事、并置、删片理由、图文结构 | 自由墙面、评论/箭头/涂鸦、多个提案并排、差异审批 |
| 摄影书设计师 | 跨页、版心、装订、文字、打样 | 规格可配置、布局组件、链接母版、高分辨率校验、PDF/X |
| 工作室/学校 | 审稿、权限、练习指导、组织知识 | 角色、审批、共享项目、版本锁定、教师评语与批注 |
| 印厂/展陈合作方 | 无歧义文件交付 | 输出规格、预检报告、不可变交付包、下载有效期 |

### 01.2 必须跑通的六个端到端旅程

**旅程 A：个人当代摄影创作。** 导入 500 张 JPEG/RAW/胶片扫描 TIFF；按日期、拍摄设备、感受筛选；将 80 张留作候选；Agent 生成三条叙事建议；在画布上手动摆放、加入大片留白、写便签、用箭头标记呼应；生成第一版 30 张线性序列，保存不同方向的分支；返回画布进一步创作。

**旅程 B：Lightroom Classic 往返。** 从 FocalRoom 发送 30 张选中资产到 Lightroom Classic 插件创建 Collection；摄影师调色；Publish/Export 产生包含稳定资产 ID、变体 ID、渲染 ID 和编辑来源版本的元数据清单；FocalRoom 在同一资产下创建 Rendition，不重复建 Photo；预览和画布缩略图原地更新，但已锁定出版物保持旧版本直到审阅。

**旅程 C：Capture One 往返。** 既支持由 FocalRoom 选片进入 Session/Catalog 工作集，也支持已有 Capture One 项目将选片推到 FocalRoom；经 Recipe 输出预览与 16-bit TIFF，使用 manifest/可用 SDK/AppleScript 关联；变体和批次重导出可辨，错误不猜测。

**旅程 D：人与 Agent 共编。** 编辑将四张照片在画布上圈选并评论“右上角高光应形成互文，不要按肤色分类”；Agent 检索相关照片证据，提出两版不同空间关系的布局草案；新草案以 ghost layers 叠在现有 Board；编辑逐项采纳、拒绝、讲理由，Agent 不改原版。

**旅程 E：多人摄影书。** 30 张版面候选→书籍开本和装订参数→自动生成可编辑 32 页布局→页间对照、跨页、章节、文字→编辑批注→印刷前软打样与分辨率/字体/出血/ICC 检查→输出适配指定印厂的 PDF/X 或高质量 TIFF 套件→交付回执。

**旅程 F：跨设备与断网。** Mac 本地使用全分辨率母版；另一位编辑从 Web 以经过 ICC 管理的代理图片批注；掉线时可本地作草案但不可假装已正式提交；恢复连接进行审阅合并；撤权后服务器不能再交付新代理，已经下载的像素无法远程擦除，UI 需明确这一现实边界。

## 02｜产品信息架构、模式与导航

### 02.1 IA 与导航

顶层：**Home / Projects / Library / Canvas / Publications / Activity / Services（未来）**。具体项目内固定左侧：资产集、Saved Views、Boards、Sequences、Publications、Project Members；顶部：项目名、正式状态与分支、协作者、撤销、同步、输出；中央是当前视图；右侧按上下文显示 Inspector / Comments / Agent；底部可收起 Filmstrip、Jobs 和 Versions Timeline。

**三种编辑视图 + 两种生产视图：**

- **Contact Sheet：** 效率优先的筛选、评级、搜索、相似图组与大规模批次。
- **Narrative Canvas：** 无限自由画布，适合无序探索视觉关系，不强制照片先后顺序。
- **Sequence：** 按顺序的摄影叙事序列，适合线性、片段、章节及情绪节奏。
- **Photobook Layout：** 有页面尺寸、出血、装订、跨页和文字的生产画布。
- **Exhibition Wall：** 实际尺寸标尺、展墙、多作品尺寸、安装和观看视线的空间布局。

每种视图都引用共同 Asset 和 Rendition；Canvas Node、Sequence Item、Book Page Frame 是不同的“呈现实例”，不要混作 Photo 数据结构。

### 02.2 桌面主界面布局（设计目标）

以 ≥1440px 宽窗口为参考，左项目栏 240–280px，中间弹性专业画面，右侧上下文检查栏 320–380px，顶栏 56px，底部胶片条可折叠 120–160px。用户可独立折叠左右栏、全屏检视、双屏移动检视窗与面板。窄窗口按优先级收起侧栏而不压缩图像至不可读；支持字体放大与系统辅助功能。

顶部状态区同时表达三组重要信息：**版本 / 权威（Local、Shared、Offline Branch）；显示质量（原图、代理、ICC/软打样）；Agent 状态（空闲、分析中、等待授权、待审查）**。状态不得以含糊的“已同步”代替实际服务器确认。

### 02.3 全局交互规则

鼠标/触控板：单击选中；Cmd 多选；Shift 区间；空格按住临时抓手；双击打开高精度检查；滚轮默认纵向滚动，Cmd/触控板捏合缩放；拖拽素材到板上创建节点而非移动源文件。热区和拖拽手柄在高密度照片布局中可清晰识别。

键盘（可重映射、显示冲突）：`G` Contact Sheet、`B` Board、`S` Sequence、`P` Publications、`C` 比较、`K` 保留、`X` 淘汰、`0–5` 评分、`Space` 抓手、`V` 选择工具、`H` 手形、`N` 便签、`A` 箭头、`D` 画笔、`M` 批注、`Cmd+K` 命令面板、`Cmd+Z` 撤销、`Cmd+Shift+Z` 重做、`Cmd+Enter` 提交草案。输入框聚焦时快捷键不得触发画布命令。

通用状态：空项目提供“导入现有文件夹/从 Lightroom 导入/从 Capture One 导入/打开示例项目”；首次加载给逐层骨架；大图库显示可取消后台任务进度；错误提示必须包含影响范围、是否保留已做操作和可执行恢复动作；系统不允许悄悄降级为低保真显示。网络延迟界面区别“本地已应用”“服务器已提交”“等待合并”“冲突”。

## 03｜界面逐页设计与具体交互

### 03.1 Home / Project Dashboard

内容：最近项目缩略图墙、Library 位置和挂载状态、正在进行的分享/审批、最近出版物、需要关注的代理丢失、待处理的 Agent 草案及外部调色回流。项目卡带创作阶段（筛选/调色/叙事/排版/交付）而不是只有“修改时间”。新建向导：名称、意图、工作媒介、资产位置、是否共享、默认显示策略；均可后改。导入完成自动进入 Contact Sheet。

### 03.2 Library / Contact Sheet

可切换平铺、分组、日期轴、相似图叠栈；支持 RAW+JPEG 配对、胶片扫描同一底片的多扫描版本、相同内容 hash 检测但不自动去重不同曝光/扫描。缩略图标记保留/淘汰/待定、评分、颜色标签、评论数、调色变体数。筛选：状态、星级、标签、相机/焦段/胶片、时间、人工描述、AI 视觉语义、被 Board 使用与否；搜索结果支持“来源证据”查看。

多选操作：标记、加入 Board、生成比较视图、同步外部编辑、创建编辑任务。双击进入 Light Table。滚动到可见位置时仅解码可见缩略图；后台预抓邻近范围；频繁滚动丢弃过期请求，显示监控未发生旧图串位。

### 03.3 Light Table / Professional Inspector

默认黑/中性灰背景，可选择中性亮背景并锁定观看环境；单图、双图同步、最多四图 Light Table；Fit、真实像素 100%、等比缩放、局部 loupe、同步平移；快捷切原始/当前 Rendition/相邻版本；取样器显示源文件数值与经管理的显示值，明确工作空间；显示文件格式、分辨率、位深、源 ICC、目标显示 profile、校样 profile、渲染缓存生成时间。RAW 未高质量显影时给“相机内嵌预览 / 派生代理”明显提示，不能冠以“原图”。

专业显示最关键的交互：**Quality Provenance** 抽屉显示此刻像素源和颜色转换链；源 ICC 缺失时拒绝声称准确；跨显示器移动时重建颜色变换；设备未校准时仅提示“显示器校准状态不可保证”，软件不能假装实现端到端校准。

### 03.4 Narrative Canvas（核心）

画布支持无限坐标，原点可移动；节点类型：照片、文字、便签、形状、箭头、笔迹、分组、章节框、参考图（有版权与权限标识）、参考链接、讨论锚点。照片节点引用 `rendition_ref` 而非复制图片内容；同一资产在多个 Board 可多次出现。可多选、缩放、旋转（图框旋转，不修改像素）、锁定、吸附、对齐、等距分布、自动整理、智能框选、图层、显示/隐藏注释。

画布上部轻量工具条：选择 V、抓手 H、照片 I、框架 F、文字 T、箭头 A、画笔 D、评论 M。右侧 Inspector 按单节点/多节点/分组展示坐标、宽高、裁切方式、透明度（只影响画布预览）、颜色提示和关联版本。支持通过明确命令切回正式源版本。

**拖拽交互状态机：** pointerdown→selected；超过拖拽阈值进入 preview_move，其他协作者看到 ghost presence；pointerup 提交一个 `CanvasMoveNodes` 原子操作；Esc 或丢焦取消 preview；并发编辑同一组节点需字段级冲突处理，不允许静默最后覆盖；撤销只逆转自己可逆的一次领域操作，不吞掉别人随后提交的不同属性修改。

**涂鸦：** 支持鼠标/手写板压感（设备允许时）、钢笔和荧光笔、圈选、箭头；笔迹按分段矢量点+压力+颜色记录，所有点转换到 Board 坐标，不与屏幕像素或画布缩放绑定；可逐段选中、删除、版本化。通过拖拽创建的参考箭头可以连接两个节点 ID，节点移动时保持关联端点。

**评论：** 可锚定 Board 绝对坐标、照片局部归一化坐标、节点/对象、某页书籍及某次作品版本；点击评论 pin 打开线程；评论可回复、解决、重开、@提及、附加前后图像范围；删除/隐藏节点时评论不消失，显示“关联对象已隐藏/已删除”并保留历史。

**Agent 画布：** 选择 24 张照片→右侧输入“建立三条截然不同的叙事路径，其中一条突出颜色与空间关系”→Agent 在沙盒中生成 3 个分支 Board，分别显示变化图层；一键比较视觉差异、按组接受、人工修改后再问；不自动覆盖当前 Board。Agent 生成的空间排布必须解释构图逻辑、使用的视觉证据和置信范围，允许“无法判断”。

### 03.5 Sequence View

线性工作面允许按章节分割、空白停顿、双图并置、序列节奏对比和不同草案差异对照；从 Canvas 选一组照片“生成序列”仅创建有序引用，不移除 Board 位置。序列可包含“占位符”（未确定选片/需要补拍）和旁注。支持拖拽、键盘、粘贴照片 ID 组和 Agent 重新编排。明确同一照片可在多个方案中出现；发布书籍时创建固定的 Edition Snapshot，防止上游重新排序暗改已审版面。

### 03.6 Agent Dock / Task Inspector

默认不是聊天独占页，而是全局 Agent Dock。内有：任务输入、目标作用域（项目/选区/Board/序列/页）、可用外部资源授权、预算高级设置、实时步骤与来源证据。每次任务具有 `plan → observe → tools → candidate → evaluate → draft → review` 流程，显示准备中、正在观察 X/Y、候选草案 N 项、等待确认、失败/可恢复。提供暂停、取消、恢复、再执行、检查工具日志与费用明细。模型名称对普通用户可折叠，企业和专家模式提供诊断而不让普通用户被迫选模型。

Agent 对话支持选中画布对象作为引用；答案中的资产引用可高亮到画布；“应用建议”不是直接写状态，而是显示 Diff（新增节点、删除节点、坐标变化、序列变化、引用版本变化）。任何错误图像 ID、外部工具越权或上传未授权文件均硬失败并留可解释诊断。

### 03.7 Creative Pipeline / External Editors

项目侧栏新增 `Develop` 阶段，显示每张资产的源版本和外部编辑连接：软件、Collection/Session/Catalog、编辑变体、最近检测、导出预设、颜色、分辨率、匹配状态。点击“在 Lightroom Classic 中编辑”显示 30 张将建立/更新 Collection、外部程序是否安装、引用是否有效；支持先在 Lightroom 编辑再导入 FocalRoom，也支持已经存在的 Catalog。

回流面板提供：新渲染发现、对应源文件、编辑变体、色彩 profile、冲突与未知文件。“匹配失败”必须走用户手动映射；“同一文件名”不构成自动覆盖依据。支持选择“只更新预览”“创建版本”“更新当前 Board 引用（需审查）”；已定稿出版物不被静默更新。源文件离线时保留已缓存版本并显示明确状态。

### 03.8 Photobook Studio

进入时创建 Publication（书籍实体），选择开本、页数、纸张、装订、页边距、装订留白、出血、色彩/印厂预设。布局舞台支持单页/跨页、母版、网格、标尺、标线、图片框、文字框、页码、封面/书脊、章节页；可从 Sequence 自动创建初稿；图片框通过 Asset Variant/Rendition 引用，支持裁切框与适配方式，绝不改源文件。逐页评论与改稿、前后版本比较；断链、低 DPI、缺 ICC、文字溢出必须显示显著预检错误。

必须区分 **Review PDF**（便捷屏幕校样）和 **Press Deliverable**（指定输出合同的 PDF/X 或指定格式 TIFF），两者不得共享低质输入。并支持输出单张艺术微喷 TIFF + 参数清单、展览安装图与作品说明清单。

### 03.9 Exhibition Studio

画布按真实墙宽高、挂画尺寸、边距、观看高度和展厅路径；支持成组拖拽、等距分布、模拟观看顺序、标签位置、布展规格单，输出尺寸标注的安装图与作品运输清单。不是伪 3D 渲染优先：首先解决真实尺寸、装框信息和作品间叙事关系。

### 03.10 Shared Workspace / Reviewer UI

与桌面端复用设计 token、资产/Node/Comment/Version 契约；Web 上以管理后的有界色彩代理呈现，显示“Web 代理 / ICC 处理信息”，不冒充与校准专业桌面的视觉绝对一致。评论、提案、角色、Presence、Agent 任务和正式批准在 Web 可用。观看者被撤权应停止获取新像素与新项目状态；离线草案恢复后必须经服务器重新验证权限和版本。

### 03.11 Settings & Preferences

分别提供：色彩管理（工作空间、渲染意图、软打样、缺 ICC 策略）；缓存（总量、源版本、重建、位置）；模型与隐私（每项目/任务授权，供应商、数据存留）；快捷键；外部编辑器/导出预设；团队角色；印厂输出 preset；诊断和导出审计记录。任何可能外发原始文件的设置均需单独高摩擦授权。

## 04｜核心领域数据模型（须先统一再写功能）

### 04.1 领域边界

- `Project`：创作项目，持有成员、创作 brief、分支、工作区、发布状态。
- `SourceAsset`：物理母版，ID 永久不变；可以是 RAW、16-bit TIFF、扫描件和导入的已调色成片；保留源哈希与变更监测。
- `AssetRepresentation`：不同来源/压缩/代理的可显示表达（thumbnail、display_proxy、tile_pyramid、ai_preview、print_master）；每种表明派生算法和 profile。
- `EditVariant`：同一母版的不同审美/调色方向；带外部编辑软件、外部稳定 ID 与原生编辑文档引用。
- `Rendition`：变体在某时刻实际渲染后的不可变文件，包含 SHA-256、尺寸、位深、ICC、时间戳、生成来源、版本、目标用途。
- `Board`：自由画布工作空间；`CanvasNode`：照片/便签/文字/箭头/形状/分组/笔迹；具稳定 ID 和 z-order。
- `Sequence`：独立于 Board 的有序叙事项目；允许章节、占位、同一 asset 的多个引用。
- `AnnotationThread`：锚点、参与人、内容、解决状态；支持图片局部、Board 坐标、Node、Sequence/Revision、Publication 页。
- `Publication`：摄影书/展览/作品集；包含母版、页面、引用的 Rendition、输出预设、冻结版。
- `ReviewProposal`：人工或 Agent 的可审查多命令变更集，带基础版本、每操作批准状态、证据。
- `ProjectRevision`：正式作品变更序号与不可变快照/日志；`SyncBranch`：离线或尝试性分支。
- `ExternalEditLink`：FocalRoom 资产/变体与 Lightroom/Capture One 的版本绑定；`TransferJob` 管理对外/回流批次与核验。
- `RenderJob` / `ProofJob`：高精度渲染与交付，不与普通 UI 预览混同。

### 04.2 核心不变量

1. `SourceAsset` 原文件字节不可由 FocalRoom 覆盖；同哈希不同物理位置可作为不同 Locator，资产合并必须明确。
2. `CanvasNode` 引用永不依赖“照片当前文件名”；引用 Rendition/Variant/Source 的策略显式保存。
3. `Rendition` 内容不变；外部再次输出创建新 `rendition_id`，可更新 `is_current` 指针但不得修改旧文件身份。
4. 跨 Node 移动、序列 reorder、评论和提案必须引用稳定 ID，并带 `expected_revision` 或 CRDT 版本向量。
5. 正式版本变动只经受权领域命令；Canvas CRDT 原始增量不能直接越过审批发布正式作品。
6. 输出冻结版锁定具体 Rendition、布局、字体、ICC/输出配置和修订号；上游编辑后默认显示“有新版本可用”，不自动替换。
7. 外部编辑软件同步先建立映射再导出；无法识别冲突不允许猜测、静默合并或重写对方目录。
8. 所有文本和外部元数据均是不可信输入；可显示但不可作为修改权限、隐私授权或事实来源。

### 04.3 推荐存储

本地：SQLite + 事务命令日志（现有）+ 内容寻址预览/瓦片缓存；文件访问由 macOS 安全书签与用户授权文件/目录范围管理，记录原始位置与备用 Locator；专业图像解码由 native service；备份仅在明示授权时含原图副本。共享：PostgreSQL（关系、权限、事件、任务、合同）+ S3 兼容对象存储（代理/可选原件/输出包）+ Redis 或专门消息总线用于跨实例实时广播和短生命周期 Presence；`PostgreSQL` 是正式版本权威，不是浏览器本地状态。

注：增量同步可以用 CRDT（如 Yjs）解决高频自由画布坐标、笔迹和光标，但**审批、资产授权、版本发布、出版冻结、对象引用变更**必须经服务器语义命令和事务校验。不能误将 CRDT 最终一致性等同于所有业务规则正确。

## 05｜Color Engine：高保真专业影像管线

### 05.1 不允许共享低质中间图作为专业显示源

输入：摄影 RAW、JPEG/HEIC、16-bit TIFF、扫描件、外部调色母版。逻辑上分为 4 个输出目标：缩略图库、专业 Viewer/Canvas 高质量显示、AI 观察低成本代理、印刷生产母版。各路径可以共享解码缓存/索引但不得共享“8-bit 1400px JPEG”作为唯一像素来源。

### 05.2 原生解码与颜色路径

`SourceAsset → Format Decoder (ImageIO/原生支持/经验证 RAW 引擎) → retain ICC & high bit depth → linear/high-precision scene/display working space → crop/scale/composite → display profile transform (ColorSync) → calibrated display`。

macOS 首选 ImageIO + Core Image + ColorSync，复杂高分辨率视图可用 Metal `RGBA16Float`（或经过测量选择更合适纹理格式）；候选跨平台后端 libvips + LittleCMS，但需要和 macOS 渲染进行参考样本像素比对。图像处理保留适当的 16-bit 或 float 精度，在最后显示阶段按真实输出设备与显示模式量化。HDR/EDR 另建渲染策略，不把“更亮”误当作 SDR 色准。

**ICC 策略：** 识别源 ICC v2/v4、profile 头/标签、EXIF 方向；记录源 profile digest、Rendering Intent、BPC 和目标 profile；嵌入缺失、无效或不匹配时显示状态并按用户指定的保守策略处理。跨显示器移动或显示配置变化时需要重新设置目标 profile 及可见瓦片缓存。CI 的“通过”必须在参考颜色样本上比较变换结果（ΔE00、暗部、肤色、超色域），不可凭浏览器看起来正常。

**RAW 路线：** 多相机 RAW 不应为了“好看”去假定某个通用引擎与 Lightroom/Capture One 显影一致。用于检查的 RAW 要么来自 FocalRoom 自身明确标识的 RAW 显影，要么优先查看来自外部编辑软件的高质量 Rendition。需要支持相机嵌入 JPEG 作为临时预览，但必须标明并允许用户一键切换可信 Rendition。

**胶片扫描特殊需求：** TIFF 巨幅（如 10000px+）、16-bit、嵌入或指定 scanner ICC、灰阶 ICC、正负片扫描、方向与边框可见；对使用色彩管理扫描仪母版和后期调色 TIFF 分别记录。不擅自自动降噪/锐化/改变扫描色调；作者可控制显示锐化与输出锐化。

### 05.3 图像金字塔和瓦片缓存

缓存键应至少包含：`source_hash + rendition_id + decoder_version + color_transform_version + source_profile_hash + target_display_profile_hash + proof_profile_hash + rendering_intent + tile_level + tile_xy + HDR_mode`。尺寸变化、源文件变化、输出配置或 ICC 变化自动失效。显示用级联质量策略：先瞬时低分辨率定位，再加载正确 ICC 的屏幕分辨率瓦片，静止时继续完善细节；用户 100% 检视时切到原始/高质量 Rendition 像素瓦片，不插值冒充真实分辨率。

性能目标（待真实设备测量，非现有数据）：10,000 张已有索引项目初次可操作 ≤3秒，常见滑动 60fps 目标（重负载可降但有清晰响应），100MP 本地照片 Fit-to-window 首帧 ≤1秒，100% 平移 95 分位请求≤150ms；超大 TIFF 处理过程不因一次性全解码导致 OOM。性能目标需按芯片、文件大小、显示分辨率和缓存冷热分组报告，不强造单一通过值。

### 05.4 Viewer/Canvas/Web 色彩一致性边界

Native Viewer、Native Canvas、出版物预览使用同一色彩变换参数和 Render Service；浏览器 Web 端受操作系统/浏览器和用户显示器限制，采用带 profile 的高质量代理、浏览器色彩管理兼容策略、来源 badge 和 QA，不能虚称在所有屏幕上实现像素级同色。Web 专业校样默认提示“请在已校准的专业显示设备或本地高保真 Viewer 进行最终色彩确认”。

### 05.5 软打样

明确区分 display view 与 paper proof。按打印机+纸张 ICC、Rendering Intent、黑点补偿、纸白模拟和光泽/观看条件配置独立 Proof Session。提供超色域警告、前后差异、墨色/黑位参考与输出参数；在用户未指定印厂和纸张时不得自动声称“已完成印刷校样”。

## 06｜Narrative Canvas：真正的无限视觉创作空间

### 06.1 画布模型与渲染

Board 的坐标系用浮点逻辑单位（独立于设备像素）；视图变换 `screen = pan + zoom × board_coordinate`；所有拖拽/笔迹在逻辑坐标存储；支持 zoom 2%–3200% 目标，超大 Board 按可见节点/图层做 culling。图片来源经 Color Engine 提供正确色彩的纹理/代理，Node 存几何属性不存图片副本。画布布局可以共享模型但 Native 与 Web 呈现使用平台兼容 renderer；重要的截图对比在颜色、比例、文字换行、箭头端点分别验收。

### 06.2 工具和交互表

| Tool | 行为 | 取消/撤销/协作 |
|---|---|---|
| Select | 点选、多选、框选、改 z-order、锁定/对齐 | Esc 取消；每次已提交移动一个 undo group |
| Pan/Zoom | 触控板捏合和手形、指定点锚定缩放 | 非持久化 viewport，可共享 presenter mode |
| Place Photo | 从 Library 拖入或 Board 复制链接 | 生成引用 Asset/Rendition 的节点，不移动/复制原图 |
| Group/Frame | 组节点、章节、色系、时间段 | 嵌套循环禁止；权限及 z-order 明确 |
| Resize/Rotate | 八手柄缩放框、支持裁切与 Fit/Fill | 图框裁切可逆、原图不变 |
| Text/Sticky | 文字和便签、富文本范围受控 | 历史和评论锚点保持稳定 |
| Ink | 自由笔迹、压感/颜色/粗细、多段擦除 | 矢量路径、不污染源影像，协作增量可恢复 |
| Connector | 箭头连接节点/具体锚点 | 节点移动时端点跟随 |
| Comment | pin 与线程、区域批注、提及 | 保留作者、版本、解决/重开 |
| Presentation | 只读画布导览、聚焦某一区域、序列展示 | Presenter 视角可选同步，不改作品 |

### 06.3 版本与冲突

区分 Draft Board（探索状态）、Submitted Proposal（可审查）、Official Board Revision（正式状态）和 Edition Snapshot（出版定稿）。多人实时移动不同节点可并行；同节点同属性同时修改采用显式冲突提示/语义合并（不是无脑 LWW）；被评论锚定的对象发生移动时评论随对象移动，但原始评论 anchor 和当时快照可回看。撤销仅作用于当前用户最后一笔可逆命令，不能撤销其他协作者的后来改动。

## 07｜Photography Agent：以专业质量和共享工作台为核心

### 07.1 真实 Agent 运行链路

`Goal & Selection → Policy/Consent Check → Task Context Builder → Plan → Candidate Retrieval/Index → Tools → Multi-resolution Visual Observation → Evaluate/Refine → Draft Board/Sequence/Layout → User Review → Approved Commands → Memory Evidence`。

Agent 拥有任务生命周期、规划、工具选择、观察反馈、预算、检查点、暂停/恢复、失败重试、权限和幂等；不是一次模型调用加一段模板文字。**模型可以建议，但只有服务器/本地可信 Policy Engine 决定调用权限。**

### 07.2 工具集（最小标准）

检索/观察：`search_assets`, `retrieve_project_context`, `inspect_assets`, `compare_assets`, `inspect_render_variants`, `get_sequence`, `get_board_graph`, `get_comments`, `retrieve_preference_evidence`。

创作草案：`create_board_draft`, `place_assets`, `move_nodes`, `group_nodes`, `create_connector`, `add_editorial_note`, `create_sequence_draft`, `evaluate_sequence`, `propose_layout`, `compare_layout_drafts`。

流程与审阅：`request_human_feedback`, `summarize_review_disagreement`, `compare_color_versions`, `plan_external_edit_batch`, `read_output_preflight`, `finish_task`。所有写工具都只能写 proposal/draft 命名空间；`publish`, `overwrite_source`, `grant_cloud_upload`, `approve_print` 等高风险操作绝不可由模型直接调用。

### 07.3 多模型路由

默认平台托管、任务驱动：Embedding 索引和相似检索；轻量视觉分析；高质量视觉叙事模型；工具编排模型；记忆/摘要模型；可选 OCR/元数据专用模块。Region、用户授权、任务上下文、模型能力、经测量的摄影质量、预算/响应时延共同决定候选；不能按“模型价格低所以默认最佳”配置。路由需保留 provider/model/prompt/版本/费用/人工反馈与失效记录，支持 fallback 但不允许在授权/身份/区域改变时无提示回退。

### 07.4 长期审美记忆：从词频升级为可检验假设

记忆三层：L1 当前任务/Board 上下文；L2 用户人工动作与说明（保留、拒绝、前后对比、纠正、接受/撤销草案，因果去重）；L3 审美假设（含语境、反例、证据和置信度、确认/暂停/删除）。跨项目记忆默认关闭，对每次模型调用附“授权+来源+版本”绑定。采用多模态视觉特征与语义证据检索，不以词频直接代表人格或审美；用户可看到“系统为什么这么认为”，还可点击纠正。Agent 的自我生成草案不自动变成作者喜欢的证据。

### 07.5 专业摄影评测

评测从真实项目中预登记不同任务：500→30 选片、两张深度比较、30 张节奏排序、Board 的并置/留白、跨项目偏好进化、出版页建议、外部回流后的重新评审。由至少 10 位目标摄影师/编辑参与，累计至少 30 个真实授权创作任务（作为首个门槛，后续扩大），专家盲评五维：视觉事实准确度、审美判断、叙事结构、执行可靠性、修改后的适应能力。比较人工基线与可获得的 Photoreka 任务表现；明示不同工具能力差异、缺失评测和评委分歧。硬失败：杜撰照片 ID、无授权外发图像、正式状态越权写入、原图损坏，容错率 0。

## 08｜Lightroom Classic / Capture One Round-trip

### 08.1 双向同步原则

FocalRoom **绝不直接写第三方不公开的 Catalog/Session 数据库**。源资产身份采用 `focalroom_asset_id` + `source_sha256` + `external_catalog_id` + `external_photo_id`/变体引用；不能只靠文件名或顺序。转换为 XMP 时，严格区分“可互通的元数据”与“只能由编辑软件解释的非破坏性调色设置”；不能假装跨引擎视觉一致。

### 08.2 Lightroom Classic Bridge

使用官方 Lua SDK 建立插件，面向两个方向：① Lightroom 中选中照片→“Send to FocalRoom”创建资产引用与候选 Board；② FocalRoom 中项目/序列→“Edit in Lightroom”创建/更新对应 Collection 或任务集合。使用 Lr Catalog 可用操作时，按官方 SDK 的事务与线程约束实现；不承诺 SDK 未支持的后台实时控制。回流使用 Publish Service 或导出插件，产生渲染文件 + `.focalroom-manifest.json`（插件稳定 ID 与源变体、尺寸、ICC、hash），本地 watcher 二次验证并创建不可变 Rendition。外部设置留在 Lightroom；对外只接收渲染后的色彩真实结果。

### 08.3 Capture One Bridge

采用 Capture One Developer SDK 能力 + macOS AppleScript/JXA 辅助 + Export Recipes，先验证当前版本实际支持的 Album/Variant 访问与调用模式。Recipe 可以一次输出 Reviewer JPEG、Print 16-bit TIFF 与后续生产版本；都必须用 manifest 关联资产与变体。若 SDK 不提供某个确定性外部 ID 或 Collection 操作，则降级为显式的交接/导出流程并在 UI 中告知，而不是使用脆弱的私有数据库改写。

### 08.4 回流状态机

`PREPARED → DISPATCHED → EDITING_EXTERNAL → RENDER_DETECTED → VERIFYING → MATCHED → REVIEW_PENDING → ACCEPTED/REJECTED`。冲突分支：`AMBIGUOUS_MATCH`, `SOURCE_CHANGED`, `WRONG_PROFILE`, `BROKEN_LINK`, `EXPORT_INCOMPLETE`；只有人工确认/可靠 manifest 能完成匹配。回流后更新派生预览和可选 Board 工作副本，不影响冻结出版物；导出重复提交按 `transfer_batch_id + rendition_hash` 幂等。

## 09｜Publishing Studio：从编排到印前交付

### 09.1 书籍模型

`Publication → Edition → Spreads → Pages → Frames → RenditionRefs`。每页有实际物理尺寸、边距、出血、裁切、网格、文字样式、章节与装订参数，单位明确 mm/pt 与 px；同一 Rendition 可被多个 frame 引用但不复制字节。版面草案与正式 Edition 分开，面向校样与生产有不同输出合同。

### 09.2 生产与色彩

出厂前针对每个目标印厂的规范创建 `PrintContract`，规定 PDF/X 目标（如 PDF/X-4，仅当印厂接受）、OutputIntent、CMYK 或特定 RGB 工作流、嵌入字体、透明度、出血、成品裁切线、图片有效 PPI、关键文字与图像越界、图像锐化、黑点/纸白/墨量控制；无法确定要求时阻止“印刷就绪”标记。艺术微喷可能需要 RGB 16-bit TIFF + 指定 ICC，而非 PDF/X；也有专业激光冲印或装框输出不同规范。

### 09.3 可复现渲染

Print Engine 从冻结的高分辨率 Rendition/母版读取；基于 output profile 和裁切坐标在高位深完成渲染；禁止从 1400px UI proxy 放大。输出包包含文件哈希、版本 ID、源资产映射、字体与 ICC 清单、预检结果、创建者审批、尺寸数量与时间戳。出版版本必须支持回滚和再版，印厂问题能从交付文件定位到素材与版本。

### 09.4 Agent 在出版中的权能

可以提出不同跨页节奏、空白和图文密度的草案；可以报告低 PPI、错位、潜在重复、颜色不一致，但**不得将审美主张伪装成客观印前错误**；最终出版确认必须是具批准权限的人类动作。完整排版专业能力包括母版、文字排版、网格、导出等，不能用简单“HTML 转 PDF”替代。

## 10｜前后端系统架构与技术选型

### 10.1 总体逻辑拓扑

```
Mac FocalRoom (Tauri/React shell + Rust domain + Native Color/Canvas Renderer)
    |  local SQLite, SourceAsset locators, native high-fidelity tile cache
    |  cmd/events & signed manifest & resumable transfers
    v
Cloud Edge/API (NestJS, auth, ACL, REST command API, Realtime WS)
    |--> PostgreSQL (authoritative shared projects, commands, revisions, grants)
    |--> Object Storage (encrypted-at-rest previews, explicit-original uploads, publish outputs)
    |--> Job Queue / Workers (AI tasks, tile/proxy render, export/preflight, thumbnails)
    |--> Model Router / Provider Adapters (capability/consent/region/quality/cost)
    +--> Collaborator Web App (React project shell, Board, comments, proposals, proxy viewer)
External Bridges: Lightroom Classic Plugin / Capture One Plugin + Export Recipe Adapter
```

### 10.2 本地客户端决策

保留仓库已有 Tauri 2 + Rust + React 的领域命令、导入、Agent 和 GUI 投资，但专业图像 Viewer/Canvas 要通过**技术验证关口**决定最终呈现形式。两条候选：A）React shell + 原生 macOS `NSView/Metal/CoreImage` 图像渲染层，菜单/Inspector 用 React；B）若原生层与 WebView 的变换/输入法/Accessibility/窗口合成产生不可接受的问题，将高保真 Canvas/Viewer 改为原生 AppKit/SwiftUI 组件，并复用 Rust Core 与协议。产品最佳体验高于保留原框架的惯性，选择以色彩样本、交互延迟、可访问性、窗口及多屏测试数据确定。

### 10.3 Cloud & Web

以现有 NestJS/PostgreSQL 为基础升级，REST 命令写入、查询投影读取、WebSocket 事件与 Presence 增量；长耗时工作交后台任务/队列。开放可替换的对象存储与模型 Provider 层，但不把所有逻辑包装为难调试的微服务。Web 和 Desktop 尽量复用 TypeScript 契约、页面信息架构、命令语义、设计 tokens、键盘映射和可用的业务组件；不能要求 Web 运行 macOS 专用色彩 API。

### 10.4 任务与权限

用户身份/工作区成员/项目 ACL/资产权限/内容外发授权/模型账户/输出授权/服务市场交易角色分别建模。照片代理和评论的访问票据短期单次、服务端每次验证权限，启用撤权审计；外部服务插件拥有独立精确 scope，撤销后不能继续同步。每次 Agent 写入产生提案和追溯；模型提示词永远不是权限系统。

### 10.5 异步后台作业

统一 Job 状态：`queued / running / waiting_user / paused / completed / failed / cancelled / unknown_external_result`。包含 idempotencyKey、checkpoint、resourceBudget、progress、attempts、retryPolicy、auditRefs、userVisibleSummary。处理 RAW/100MP/大型图书 PDF 的 Job 可以暂停并恢复，失败不能留下被误标为完成的文件。

## 11｜协议、API、事件与错误设计

### 11.1 调用分层

- `GET /v3/projects/{id}/snapshot`：返回当前共享权威版本、已授权资产与 Board/Publication 投影，不返回无授权的原图路径或密钥。
- `POST /v3/projects/{id}/commands`：统一提交经过 JSON Schema 校验的领域命令，带 `command_id`、`idempotency_key`、`base_revision`、`actor`、`operations`；服务端验证 ACL、范围、基线、版本与操作内容。
- `GET /v3/projects/{id}/events?cursor=` 与 `/v3/realtime`：事件版本及 WebSocket 游标，断线重放并按项目授权过滤。
- `POST /v3/assets/import-sessions`：可恢复/校验的资产导入任务；源文件本地时不强制上传。
- `POST /v3/external-edit/batches`、`POST /v3/external-edit/renditions`：软件交接/回流；采用签名或受保护 manifest 与内容哈希校验。
- `POST /v3/agent/tasks`、`POST /v3/agent/tasks/{task}/approve`：Agent 请求、流进度和明确审批，模型不能自己 approve。
- `POST /v3/publications/{id}/proof`、`POST /v3/publications/{id}/press-jobs`：预检与生产输出任务，输出不等于 HTTP 同步请求。

接口详细样例与事件 schema 见 `contracts/`。需要版本化 API 与兼容窗口，线上强制声明客户端版本与服务能力，以防桌面旧客户端错误执行新类型命令。

### 11.2 事件结构

`EventEnvelope`：`eventId, projectId, revision, actorId, opId, causationId, entityType, entityId, eventType, payload, createdAt, schemaVersion`。敏感字段按权限过滤；服务端为顺序关键域统一版本推进；实时光标/hover 等短期 Presence 不写正式历史，笔迹和节点位置提交应进持久增量及快照。

### 11.3 失败 UX 标准

错误类必须可机器识别：`SOURCE_OFFLINE`, `ICC_MISSING`, `COLOR_TRANSFORM_FAILED`, `RENDITION_UNMATCHED`, `EXTERNAL_VARIANT_CONFLICT`, `REVISION_CONFLICT`, `ROLE_DENIED`, `CONSENT_REQUIRED`, `BUDGET_EXCEEDED`, `MODEL_CAPABILITY_MISSING`, `PRINT_PREFLIGHT_BLOCKED`, `SYNC_PARTIAL`, `REPLAY_REJECTED`。每类错误对应 UI 状态、用户可执行动作、是否可重试和是否已经外发数据，不允许“未知错误请稍后”掩盖文件完整性状态。

## 12｜协作、版本与一致性细节

正式作品变化采用语义命令事务；自由画布实时中间态采用短期 Presence/可恢复协同草稿；持久化 Node geometry 和 ink 路径增量。共享资产编辑需要分清 `source asset permission`, `can_comment`, `can_edit_draft`, `can_submit`, `can_approve`, `can_publish`, `can_share_original`, `can_export_press`，不能只有 Viewer/Editor 两档完成所有管理。

协作者编辑时看得到他人选区、正在看哪张照片和讨论位置；显示的是审稿辅助，不强制他人画布跟随。Presenter mode 可选同步。离线个人草案重新上线时先检查权限、正式修订、资产是否仍可访问；冲突在 Diff 面板处理；撤权后的离线草案保持本地私有但不能提交或取新代理。

版本快照分：资产身份（稳定）、Rendition（不可变）、Board draft（可变）、Official revision（正式）、Edition release（冻结）、Delivery artifact（不可变）。一个评论和图像渲染能追溯到具体版本。生产输出版本不得因后续调色回流自动变化。

## 13｜软件工程、质量与运维

### 13.1 工程组织建议

保持 monorepo；划分 `apps/desktop`、`apps/web`、`apps/api`、`services/render-native`、`workers/ai`、`workers/publishing`、`bridges/lightroom-classic`、`bridges/capture-one`、`packages/domain-contracts`、`packages/ui-system`、`packages/canvas-ops`、`packages/color-spec`、`tools/agent-eval`、`tools/color-qa`、`tools/print-preflight`。与旧工程路径映射执行渐进式迁移，旧数据库不可直接修改历史迁移。

### 13.2 测试金字塔

- Domain：ID 稳定、撤销、版本冲突、AST 合法、审批、幂等、原图不可变、跨项目禁止泄漏。
- Color：参考测试图与 ICC v2/v4、P3、AdobeRGB、ProPhoto、灰阶、16-bit TIFF、HEIC、RAW、扫描件；ΔE00 与像素参考；不同显示器/校准状态。
- Canvas：笔迹压力/密度、变换不漂移、评论锚点跟随、10k Node/culling、拖拽取消、多人冲突、离线重放。
- Round-trip：多 Catalog、同名文件、变体、多次导出、文件掉线、修改 source hash、ICC 变更、失败回退、取消重试。
- Agent：真实任务、专家盲评、工具执行可信度、上下文压缩保留观察证据、模型动态路由/区域/外发授权。
- Publish：印前 preflight、版式出血、字体嵌入、实效 PPI、ICC OutputIntent、文件可被第三方 RIP/Preflight 软件接受。
- Security：ACL、分享票据、私有图片撤权、API 限流、上传校验、插件权限、隐私模型授权、备份恢复。
- Operations：最新 CI 持续通过、日志与可观测性、不泄露 API key/原片路径、签名/公证/更新回滚、灾备演练。

### 13.3 设计级 SLO 与性能验收

建议按真实性分组设目标：工作台连续 4 小时编辑无丢命令；局域网/稳定网络共编操作可见延迟 p95 <250ms（正式确认单独显示），慢网络自动重放且无重复执行；总览 10k 资产库已有索引后快速过滤 p95 <300ms；全分辨率 100MP 100% 平移无卡死；同一参考图经色彩变换的可测 ΔE00 采用 Color Science 团队预先批准的容差，不能临时调阈值通过；发布前 0 P0/P1 开放缺陷。所有数值属于目标，尚未实测。

### 13.4 可观测性

关键业务仪表：导入成功/失败、代理生成耗时、色彩变换错误、Canvas 操作冲突、回流匹配失败率、Agent 任务状态/每张视觉观察费用、真人建议采纳率、出版物预检失败原因、共享权限拒绝、对象存储读写延迟、备份与恢复。事件不包含原图像素、密钥、真实路径和敏感评论明文；高敏感内容仅可审计 ID 与状态。

## 14｜基于现有 GitHub 的保留、修复和重构计划

当前 `yihua-bot/focalroom` 的可验证基线：有 Tauri + Rust + React 的本地编辑、SQLite、Contact Sheet 与 `SequenceBoard` 序列拖拽、Agent 工具循环与草案、NestJS/PostgreSQL 共享审稿与评论。重要缺口：专业自由 Canvas 不存在；专业显示预览主要是 1400px 8-bit RGB JPEG，受限原图检视和 RAW JPEG 代理；`sidecar` 为自定义 JSON 而非通用 XMP；打印输出为 HTML→浏览器 PDF 校样；Lightroom/Capture One 原生双向集成不存在；摄影书/展览生产引擎不存在；云端 Agent 只读评论功能与完整本地 Agent 存在能力边界。最新 CI 有 E2E 标题断言与 API 端口问题。仓库历史文档中的“已完成”仍需以当前提交、CI 和实际运行重新核验。

| 现有模块 | 决策 | 迁移方向 |
|---|---|---|
| `src-tauri/src/command_handler` + SQLite | **保留并扩展** | 将 Project、Photo 命令升级为资产/变体/画布/版本领域命令 |
| `src/components/contact-sheet/*` | **保留并优化** | 复用索引、筛选、虚拟化和高性能渲染，统一色彩代理 |
| `src/components/sequence/SequenceBoard.tsx` | **保留作线性视图** | Sequence 与 Narrative Canvas 并存，互相转换不相互覆盖 |
| `src/components/shared/PhotoViewport.tsx` | **部分替换** | 高保真 Native Viewer 接管专业像素；UI ZoomControls/事件可复用 |
| `src-tauri/src/image_pipeline.rs` | **分拆重构** | thumbnail、high-fidelity display tiles、AI preview、print master 四管线 |
| `src-tauri/src/agent/runtime.rs` | **保留迭代** | 增 Board/Publication 工具、质量评测、上下文与观察证据修复 |
| `src-tauri/src/agent/memory.rs` | **升级** | 从词频假设走向多模态、有反例的语境化审美记忆 |
| `apps/web/src/shared/ProjectReview.tsx` | **渐进拆分** | Share Shell + Board Canvas + Comments + Proposals + Access，统一设计语言 |
| `apps/api/src/agent-tasks.ts` | **优先修复** | 消息结构、任务类型持久化、分歧任务链路与端到端测试 |
| `src-tauri/src/export.rs` | **保留旧格式作兼容** | 另建 External Bridge 和 Print/PDF 引擎，不把 JSON sidecar 当 XMP |
| `src-tauri/src/photo_inspection.rs` | **逐渐取代限制** | 高像素分块、RAW 专业 Rendition、可证色彩 provenance |
| `apps/api/src/authority.ts` 与 realtime | **保留扩展** | 权限/语义操作/CRDT 桥、跨实例 Presence、正式审批 |

**数据库迁移原则：** 执行 expand→backfill→dual-read/validate→switch-write→contract；先只读备份和临时库恢复，不能直接重写已发布 migration；所有 `photo_id` 老引用映射至稳定 `asset_id` 并保留 alias 兼容表，避免把历史评论/草案/版本变成孤儿。

## 15｜实施里程碑：不以工期压缩，而以可证明的完成条件推进

| Gate | 交付结果 | 必须通过的验证与退出条件 |
|---|---|---|
| **G0 现有基线冻结** | 修复当前 CI；全量自动化/架构地图；演练数据与原图备份 | 最新 main CI 全绿，真实图像不丢；旧项目可打开，迁移回滚实测 |
| **G1 Asset Domain 3.0** | SourceAsset、Variant、Rendition、external links、revision | 一张原图多版本、多 Board 引用；重导出不重复/覆盖；旧项目迁移成功 |
| **G2 Color Engine** | Native ICC、16-bit/float、ImageIO/ColorSync/tiles/RAW 真实表示 | 多显示器色彩参考和 100MP/扫描 TIFF 测试通过；无伪“原图” |
| **G3 Narrative Canvas** | 无限 Board、拖拽缩放分组、便签、箭头、画笔、评论 | 人工编辑 100+ 图的完整 Board、缩放锚点准确、离线撤销正确 |
| **G4 Agent on Canvas** | 多模态检索、工具写草案、叙事评估、上下文/审美记忆 | 至少 30 实项目/10 摄影师盲评，权限硬错误 0；能多轮修改 Board |
| **G5 Round-trip** | Lightroom Classic Plugin、Capture One Bridge、manifest/ watcher | 多 Catalog/Session，30 张调色往返零错配，版本保留、ICC 可解释 |
| **G6 Shared Creative Workspace** | Web Board/评论/多人实时、版本提案、权限与离线冲突 | 真实 5 组摄影师/编辑双人项目；慢网、撤权、跨实例不丢写入 |
| **G7 Publishing Studio** | 摄影书/展览、软打样、高质量渲染和交付 | 实际装订样书、印厂/艺术微喷接受目标文件，预检报告可复现 |
| **G8 Professional Delivery & Release** | 签名/公证、安装/更新、付费/团队管理、灾备与运营 | 干净 Mac 安装到印前交付全链路通过，收付费/权限/撤权可用 |
| **G9 Professional Services（可选后续）** | 认证编辑/策展服务、报价、合同、交付、验收 | 服务产生真实可追溯成果，不做社交 Feed / 点赞关系链 |

里程碑存在部分并行可能，但 **G1 数据身份和 G2 色彩** 是 G3/G5/G7 的基础；Agent 质量验证是继续扩大收费的独立硬门槛，不能用工程交付替代。每个 Gate 结束必须有运行产物、完整源代码/迁移、测试数据说明、可复现脚本、已知限制、验收签字和下一 Gate 依赖清单。

## 16｜“完成”的最终验收：一项真实摄影书挑战

从真实摄影师的 500 张素材（含 RAW、16-bit TIFF、扫描件）开始，完成以下全流程：

1. FocalRoom 正确识别 500 个或更多源文件与实际 ICC、位深、尺寸，不修改字节；可在 Native Viewer 观看可信高质量影像。
2. Contact Sheet 快速初选 100 张，Agent 提出 30 张编辑候选；摄影师有权纠正，建议可追溯来源。
3. Narrative Canvas 可自由摆放 100 张候选，至少 20 个分组/便签/涂鸦/箭头，保存两个方向及版本；可转换出 30 张 Sequence。
4. 30 张送往 Lightroom Classic，或另一路送往 Capture One，完成两套非破坏性调色；全部调色版通过 manifest 正确回流至既有资产和变体。
5. 另一城市的摄影编辑通过 Web 审稿，针对照片局部、画布对象、序列和跨页作批注；网络中断、权限撤销及同节点竞争修改均安全处理。
6. Agent 根据评论和上下文产生两版有理由的 Canvas 和摄影书候选草案，摄影师逐项批准，未批准内容不会改变正式作品。
7. 制作真实规格的 32 页摄影书，源图保持高分辨率，完成软打样和印厂合同级预检；生成指定 PDF/X 或微喷 TIFF，并且在独立印前工具/真实供应商检查中通过。
8. 交付包中所有文件、字体、ICC、版面与源 Rendition 映射可追溯；历史版本仍可打开，并能复现同一份印前输出。

此挑战同时是用户价值、色彩可信、Agent 专业质量、工程可靠性和商业产品成熟度的集成验收。任何单点未通过都必须保留为开放事项。

## 17｜架构 ADR：先行确定及须做实验的决策

| 决策 | 默认方向 | 不可忽略的验证 |
|---|---|---|
| GUI | React/Tauri 保留 shell，专业显示层优先原生 NSView/Metal | ICC、透明叠层、窗口移动、多屏、键盘与 Accessibility；不达标时转原生 Canvas |
| Domain | Rust local-core + TypeScript/Zod/JSON Schema 共享语义协议 | Native/Web 对同一命令的回放产生相同语义状态 |
| Source Files | 只读 locator + SHA256 + bookmark | 离线、重命名、迁移、链接丢失、目录更换一致处理 |
| Canvas Sync | 协同 Draft CRDT + 服务端正式语义命令 | 100 人、长离线/撤权、同对象竞争移动、删除评论关联 |
| Model Provider | 平台智能路由，可审计，隐私优先 | 盲评分/区域/授权/计费/无假 fallback |
| Print | 合同驱动高位深 native render + 第三方 preflight | 真实印厂/微喷接受，非 PDF 文件存在即验收 |
| 外部编辑器 | 官方 SDK / Export Recipe / manifest | 插件版本、目录升级、稳定身份、文件改名、重复/冲突 |
| Cloud Storage | 默认共享低损代理；原件仅显式上传 | 最小权限/撤权/失效票据/端到端安全演练 |

## 18｜不可突破的安全与审美边界

- 用户选择共享预览≠授权上传原始 RAW/TIFF；每个模型供应商、范围、图片数量、持续时间、撤回机制须明确。
- 不允许模型对外传输同意范围以外的照片；不允许 Agent 自己批准自己的提案、签署印刷或删除源文件。
- 用户上传的 IPTC、文件名、评论、第三方导出 manifest 都按不可信数据处理，不能作为指令覆盖系统和权限。
- 图像/画布 AI 分析应区分可观察事实、审美解释、创作建议及无法确定；不能把风格关联写成对真实作者/著作权的确定断言。
- 协作不得默认变成公开社交：项目资料、摄影师选择与不发表的照片均为私人创作数据；分享和服务市场由用户明确发起。
- 屏幕显示、印刷硬件和主观审美无法由一个软件保证“绝对一致”；以可验证条件、清晰提示及真实使用场景检验专业可信度。

## 19｜研发团队执行规则与文件索引

**任务启动第一步**读 `delivery/AGENT-START-HERE.md`，再读 `architecture/ADR-AND-GAP-MAP.md`、`ux/SCREEN-AND-INTERACTION-CONTRACT.md`、`contracts/DOMAIN-SCHEMA.sql`、`contracts/api-openapi.yaml`、`contracts/canvas-command.schema.json`、`quality/ACCEPTANCE-MATRIX.md`，再访问当前仓库确认 git HEAD 和运行环境。各包文件是设计契约样例，不代表可直接执行而不做数据库迁移评审；详细接口可在仓库内扩展但不能改变不变量。

**每个里程碑交付模板：** 变更摘要、提交 SHA、变更文件清单、数据迁移验证、契约变更、UI 录屏/真实截图与交互清单、自动化/真实环境测试、性能与色彩结果、已知限制、回滚步骤、开放问题。禁止将 mock 服务器通过当作真实模型质量、将 Web 代理当作专业原图、将校样 PDF 当作可印刷 PDF、将自动化测试通过当作真人审稿通过。

## 附录 A｜外部技术依据（2026-10-20 核对）

- Apple ColorSync： https://developer.apple.com/documentation/colorsync —— ICC profiles/transforms 与显示设备管理。
- Apple HDR/EDR Metal： https://developer.apple.com/documentation/metal/hdr-content —— 专用 HDR 路线，区别于 SDR 色准。
- Adobe Lightroom Classic Developer SDK： https://developer.adobe.com/lightroom-classic —— Lua 插件、导出/发布、元数据扩展。
- Capture One Developer Portal： https://www.captureone.com/en/partnerships/developer —— SDK 插件入口。
- Capture One Export Recipes： https://support.captureone.com/hc/en-us/articles/360021057158-Export-Recipes —— 多格式、尺寸与颜色空间输出。
- Capture One Export ICC： https://support.captureone.com/hc/en-us/articles/360002634337-Specifying-color-space-ICC-profile-for-Export —— 输出 ICC 设置。

> **可执行性提示：** 此文件是最终目标规格，不是“已经完成的软件交付报告”。当前 GitHub 仓库需要先修正 CI 并建立提交 SHA 基线；第三方插件 SDK 的具体函数和行为必须在目标版本中验证，不能凭概念接口猜测。
