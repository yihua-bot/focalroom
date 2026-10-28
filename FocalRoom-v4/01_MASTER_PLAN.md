# FocalRoom 全平台最终目标产品与工程实施方案

**版本**：North Star v4.0 / 实施基线 v1.0  
**日期**：2026-10-27  
**目标平台**：macOS、iOS、iPadOS、Android、Web  
**适用对象**：产品、设计、前后端、算法/Agent、图像工程、测试、运维、安全与摄影编辑团队  
**目标约束**：以最佳专业创作体验为导向，不以最低成本、最短工期为产品折中前提；工程仍需按依赖和验收 Gate 实施。

> **一句话定位**：FocalRoom 是摄影创作者与专业摄影 AI Agent 共同完成作品的统一工作空间，从第一条创作灵感、原始影像，到作品编辑、协作、出版与长期归档。
>
> **核心品牌命题**：让照片，成为作品。

## 0. 执行摘要与最终决策

这不是“桌面版适配手机”，而是一个统一产品、五种终端体验：Mac 负责完整专业创作；iPhone 是随身的摄影 Agent 与轻量编辑工作台；iPad 是触控/手写叙事工作台；Android 对齐手机端核心能力；Web 专注跨组织查看、审核、协作和分享。平台之间复用项目、资产、画布、命令、Agent 会话与历史，不强求复用像素级 UI 实现。

**六个产品核心**：（1）Project & Research：创作 Brief、研究资料、参考影像、灵感；（2）Library & Color Engine：影像资产、色彩可信检视、选片和版本；（3）Focal AI：视觉观察、知识、记忆、工具调用、任务与草案；（4）Narrative Canvas：叙事画布、自由拖拽、评论、笔迹与审稿；（5）Creative Pipeline & Publishing：外部调色往返、摄影书/展览/作品集、印前；（6）Collaboration & Sync：项目权限、离线、实时协作、历史、发布。

**架构定案**：移动 UI 采用原生 SwiftUI/UIKit、Kotlin/Compose；抽取可复用的 Rust Domain Core 和统一协议；共享云端以 PostgreSQL 作为正式多设备项目的权威记录，并使用私有对象存储保存用户授权同步的影像派生物。Mac 原图默认不上传。云端 Agent 只有拿到对应项目/照片授权后才能使用影像；本地专有原片仍由受控 Mac 执行器访问。旧本地项目必须可离线使用，不能强迫全部迁云。

**非目标**：不造另一个通用 RAW 调色器；不把手机当成印前签样设备；不做泛社交信息流；不让 LLM 未经确认改写正式作品；不宣称不同品牌屏幕能呈现完全相同颜色；不将单次对话等同于永久审美记忆。

**体验底线**：用户在任一设备中查看同一资产/渲染版本，看到一致的内容身份、叙事关系和操作历史。重大改动可审阅、可追责、可恢复；不以“最后写入获胜”覆盖他人正式修改。

## 1. 现有仓库基线与 Gap（基于 2026-10-27 main 的静态代码核对）

源仓库：https://github.com/yihua-bot/focalroom ；核对时最新提交 `086d8f6536e3`（25MP 解码性能对照），之前提交覆盖 G2 色彩与分块/解码原型。正式审计以提交 SHA 固定，而不能只引用浮动 main。

| 子系统 | 现有确认的基础 | 最终目标缺口 |
|---|---|---|
| 桌面应用 | Tauri v2、React、Rust、SQLite、本地选片和编辑 | 多设备读写、原生高质量影像层进一步集成验收 |
| 画布 | `NarrativeCanvas`、`EditorialReviewWorkspace`、比较与审稿模块、部分合并/历史相关工程 | 端间同义命令、触控/笔支持、Web/移动共同编辑、大规模画布性能 |
| AI | Agent Runtime、工具、记忆与创作上下文；另有 FocalChat | 统一会话/任务/工具循环、跨端任务延续、真实专业质量评价 |
| 色彩 | `color_engine.rs` 已含 ICC 读取、原生转换实现，近期有 ImageIO 解码原型与测试 | 将高精度渲染全面接入主 Viewer/Canvas；设备级色彩和性能验收 |
| 后期桥接 | Lightroom Classic Lua 插件、Capture One macOS Bridge、外部渲染版本导入 | 一键向外部编辑器发送选片；自动回流、版本通知、可靠双向状态 |
| 出版 | Publication 对象、从 Sequence 生成页面/预览 | 专业跨页、字体、色彩软打样、印前检验、PDF/X、输出回执 |
| Web/Cloud | Node API、PostgreSQL、权限、WebSocket、私有预览、Web 审稿 | 本地与云的双向权威/离线操作协议、完整画布同步、高质量派生物管线 |
| 移动端 | 无正式 `apps/mobile-ios` / `apps/mobile-android` | 原生客户端和完整质量门禁 |

**工程注意**：这份 Gap 是代码结构审阅，不等于已经在真实 Mac/iPhone/Android、云环境、校准显示器和印厂完成运行验收。特别是新提交的 Color Engine 与 Editorial Review，不能沿用早期“完全未实现”的判断，也不能自动判定已产品化。

## 2. 目标用户、任务与价值证明

### 2.1 优先服务的人

1. **独立艺术摄影师**：多年/多次拍摄、需要视觉研究、编辑、序列、摄影书、展览、审美反馈。
2. **摄影编辑 / 策展人**：跨项目阅读、照片比较、组照叙事、多轮意见、提案与批准。
3. **摄影工作室与商业创作团队**：导入、客户选片、调色版本、任务、交付、权限和批准。
4. **摄影教育与导师**：布置项目、逐张批注、比较方案、记录改进与成长。
5. **影像爱好者与手机摄影创作者**：低门槛导入与交流，将手机照片发展为严肃作品。

不要同时把所有人作为启动期主叙事。产品首页与第一组真实用户测试以 **艺术摄影师 + 摄影编辑** 为主，商业摄影与学校作为下一条验证线。

### 2.2 五条端到端用户任务（JTBD）

- **灵感→项目**：在散步时语音记录念头，加入参考、创作意图和拍摄计划，Mac 上自动继续。
- **拍摄→选择**：手机或相机素材导入，浏览/标记/对比，AI 检查实际图像，生成可解释候选组照。
- **讨论→叙事**：AI 提出两个叙事方向，将更改放进独立画布草案，用户局部采纳、批注、撤销。
- **选片→调色→回流**：Mac 上送入 Lightroom/Capture One，渲染版本回来后照片身份和画布位置不变，手机收到审片通知。
- **叙事→成品**：Mac/iPad 组织摄影书/展览，手机端审阅和批注；正式印前只由具备专业条件的桌面工作流确认。

## 3. 终端职责与功能级差异

| 能力 | macOS | iPhone | iPad | Android | Web |
|---|---|---|---|---|---|
| 创作 Brief/研究/灵感 | 完整 | 完整、语音优先 | 完整 | 完整 | 共享项目读写 |
| 本地原片/胶扫导入 | 大批量专业 | 系统相册/文件 | 相册/文件 | 系统选图/文件 | 授权上传或受控预览 |
| Contact Sheet/评分/比较 | 专业全量 | 快速选片、双图 | 大屏触控 | 快速选片、双图 | 审核专用 |
| 高保真检视 | 色彩与原片权威检视 | 可信预览和分块 | 可信预览和分块 | 设备能力自适应 | 色彩能力标注 |
| Narrative Canvas | 完整 | 聚焦区域、拖拽、批注 | 完整笔触/多选/分屏 | 聚焦区域、触控 | 授权审阅/协作 |
| AI 对话/视觉工具 | 完整 | 主交互 | 完整 | 主交互 | 授权会话 |
| 部分采纳/撤销 | 完整 | 核心 | 完整 | 核心 | 依权限 |
| 专业修图往返 | 插件/桥接 | 查看和反馈 | 查看和反馈 | 查看和反馈 | 版本审稿 |
| 摄影书/展览设计 | 完整出版 | 审稿 | 设计+审稿 | 审稿 | 外部协作 |
| 印前/PDF-X/软打样 | 权威工作台 | 预览提示 | 可讨论非权威签样 | 预览提示 | 审阅 |

**设计原则**：同一能力跨端不是相同操作密度。Mac 提供专家控制；手机一步一任务；iPad 鼓励空间化直接操纵；Web 最小授权、零安装。

## 4. 信息架构、UI 设计系统与导航

### 4.1 跨端对象层级

`Account → Workspace → Project → SourceAsset / ResearchItem / Sequence / Board / Publication → Revision / Rendition / Comment / AI Task / OutputJob`

一个项目允许同时存在选片序列、多个叙事画布、多个出版物和不同人类/AI 提案。媒体原片与创作对象解耦：一张原片可有多个调色版本，多个画布节点可引用同一资产。

### 4.2 iPhone / Android 一级结构

底部四入口：**项目 / Focal AI / 画布 / 消息**。顶部项目切换器 + 全局搜索 + 个人账户。中间悬浮的“+”由当前场景决定新增灵感、导入照片、语音笔记或评论；绝不引入与任务无关的动态信息流。

- **项目**：最近继续、待审改动、离线状态、文件所在设备、快速导入。
- **Focal AI**：持续会话、拍照/相册引用、语音输入、工具执行步骤、证据卡、草案卡。
- **画布**：最近画布、草案与正式版；手机默认“阅读模式”，进入“编辑模式”才允许拖拽以防误触。
- **消息**：人类评论、AI 完成、采纳请求、冲突、邀请、调色回流与发布状态；可按项目静音。

横屏手机采用照片全屏 + 侧滑控制；iPad 采用左侧项目导航 / 中间画布 / 右侧 Focal AI 或评论三栏，支持拖放多任务与键盘快捷键。Web 延用项目主导航与受邀查看流程。

### 4.3 视觉语言

- 作品优先：深色中性背景，灰阶 UI；照片显示不附加着色遮罩；差异轮廓仅在图像外侧。
- 字体与无障碍：正文具足够字号和动态字级；主要触控目标最少 44pt（iOS）/48dp（Android 设计目标）；所有颜色编码配合图标、文字或形状。
- 状态永远可见：`仅本地 / 已同步 / 离线待提交 / 冲突 / 只读 / 已归档`；每个编辑动作必须有明确完成回执。
- 模式约束：阅读、选片、画布编辑、AI 审稿、出版审稿使用一致但可辨的顶栏；离开有未提交操作的场景时提示保存草稿。

## 5. 手机/平板逐页面 UI 与操作规范

### P01 首次启动与登录

**布局**：第一屏强调“一条创作线索，从手机延续到作品”；三项实际能力示例；登录支持系统安全凭据、适用的第三方登录；允许演示项目只读探索。\
**动作**：登录→选择已有工作区/创建私人工作区→控制同步范围→进入项目。\
**异常**：旧设备恢复、重登、服务不可达、邮箱未验证、邀请到期；不得出现登录后空白或无限转圈。\
**验收**：同账号 Mac 与移动端项目身份一致、注销后密钥失效、本地缓存按策略清理。

### P02 项目首页

**布局**：项目封面、名称、创作 Brief、最近打开位置、设备状态、照片张数、未解决批注、AI 草案数。\
**交互**：点开项目恢复最近浏览对象；长按项目快捷分享/离线下载/归档；“继续创作”回到最近任务；通知卡一键定位。\
**空状态**：创建项目→语音录入意图→导入系统相册/文件→选择同步策略。\
**边界**：尚未入云的纯本地 Mac 项目，不得在手机伪造可访问状态；显示明确的“在 Mac 开启跨设备访问”。

### P03 灵感与视觉研究

**布局**：纵向记录流 + 参考影像网格 + 研究标签；可切为 Moodboard。\
**交互**：短文本/语音转写/相册参考/链接收藏；按创作主题形成草稿卡；拖放到画布中的 Research Group；AI 追问具体拍摄意图，不能自动把“类似某摄影师”当作用户永久风格。\
**数据**：ResearchItem、ReferenceAsset、IdeaRevision、Attribution、Consent。\
**验收**：无网新增、多端排序一致、保留来源及版权说明、手机语音记录在 Mac 可追溯。

### P04 图库与快速选片

**布局**：双列/三列自适应图墙；过滤星级、旗标、日期、相机、项目分组；底部批量操作栏。\
**手势**：点击全屏；长按进入多选；上下滑动切换照片（防止和返回手势冲突）；评分用清晰触控组件，不要求精确小图标；操作可撤销。\
**异步**：缩略图滚动加载、分级缓存、出错占位、原图本机/云端状态；批量导入进度随时可查。\
**验收**：1000 张级别网格顺滑；连续多选不漏选；同一照片在所有端评分/旗标一致。

### P05 单张专业检视与双图比较

**布局**：照片占主屏，信息抽屉列文件、来源、色彩配置、渲染版本、状态；双图比较可选择锁定缩放和分别缩放。\
**手势**：双指缩放、拖动、双击定位 100% / 适合窗口；长按放大镜；手势冲突以当前工具模式确定。\
**色彩**：获取显示用可信派生物；完整显示 ICC 解释、原片/预览标识、无法高精度检视原因；严肃印前判断转 Mac 软打样模式。\
**验收**：旋转与色彩空间不发生静默变化，缓存切换不闪烁成另一种色调；特定 ICC 测试图参考色差达标。

### P06 Focal AI 主对话

**布局**：多模态输入栏、项目上下文标签、当前引用照片卡、工具运行步骤、成本和授权提示、可展开证据。\
**输入**：文字、语音、当前查看照片/选中图集、画布节点、出版跨页引用；最多附图策略由 Agent 与额度/权限共同控制，不应把“用户不懂附图按钮”视为用户责任。\
**回答**：事实问题直接答；图片问题先观察后给证据；操作请求调用受控工具；不确定时说明；输出新草案卡并提供审稿入口。\
**继续**：跨端 Conversation ID / Task ID；恢复未完成任务和执行记录；失败后可按幂等规则继续，不静默重跑收费模型。\
**验收**：自然语言“找出这组中视觉重复的 8 张”能检索并实际引用图像，不是只给通用建议。

### P07 手机 Narrative Canvas

**布局**：全屏可缩放视口；左上画布/版本；右上只读/编辑状态；底部工具托盘（浏览、选择、批注、画笔、AI）。\
**交互**：双指缩放平移，单指在编辑模式拖拽节点，长按弹对象菜单，多选框可见；最小可触控拖拽手柄；右侧浮层显示照片信息。\
**对比**：默认阅读全景，点节点进入对象局部；复杂多节点排列提示“在 iPad/Mac 深度编辑”而不是禁用所有功能。\
**验收**：移动/分组/尺寸修改进入同一领域命令；无网编辑标为未提交；无冲突恢复、存在冲突不覆盖。

### P08 iPad Narrative Canvas Pro

**布局**：左资产抽屉/缩略图堆，中无限画布，右 Focal AI/Inspector，底浮动绘画工具与缩放导航。\
**手势**：Apple Pencil 优先用于笔迹/圈选/箭头，手指平移缩放可区分，外接键盘快捷键、撤销重做、多指及悬停按系统能力适配。\
**复杂操作**：选中框批量移动、组内布局、多节点调整、辅助对齐、叙事连接线、遮挡顺序、章节、锁定、草案对比。\
**笔迹持久化**：PencilKit 输入经规范化保存为跨平台 Stroke 对象（坐标、压力、颜色、笔尖、路径版本），不是只能 iPad 阅读的屏幕截图。\
**验收**：不同缩放级别下笔迹与照片锚定关系不漂移；百至千节点级视口性能达标。

### P09 AI Editorial Review / 部分采纳

**布局**：方案切换、改动树、并排/差异叠层/合并模拟三模式、改动理由/证据、勾选采纳、冲突提示、历史。手机改动树用 Bottom Sheet，iPad 侧栏。\
**交互**：默认只是预览；选择单个 Editorial Change；自动闭合依赖；实时合并预览；用户批准后事务采纳；支持撤销整组采纳和冲突型补偿撤销。\
**语义**：`另存独立方案` 与 `合并到正式画布` 是不同操作；Reviewer 可评论但不得直接采纳。\
**验收**：选 3/20 改动，正式只应用 3 项及不可拆依赖，之后 `撤销本次采纳` 不误删后续人工修改。

### P10 评论与协作

**布局**：对象批注 pin、线程抽屉、@提及、未解决筛选，单独区分正式作品评论与 AI 草案意见。\
**交互**：在图像/画布上圈选、笔划、矩形/箭头、语音评论转写、回复/解决/重开、请求批准。\
**权限**：Viewer/Reviewer/Editor/Owner 独立能力检查；移动系统推送不包含敏感影像。\
**验收**：被撤权后旧下载无法回收，但之后票据/会话/访问被拒绝；评论目标节点修改后仍可定位。

### P11 创作出版预览

**布局**：摄影书封面/跨页横向浏览、页码、留白、裁切比例、内容检视与纸张模拟；展览模式支持墙面照片位置和尺寸查看。\
**交互**：跨页批注、版本切换、批准/退回；iPad 可微调版面，手机不提供全套复杂印前操作。\
**约束**：手机与 iPad 的屏幕软打样不作为专业印刷签样；最终 PDF/X、出血、字形嵌入和 ICC 由专业输出链路校验。

### P12 离线/同步中心

**布局**：同步状态（待提交/上传中/已同步/冲突/失败）、各设备上的原片位置、当前缓存大小和下载选项。\
**交互**：仅 Wi-Fi 上传、高质量缓存下载、暂停/恢复、清理本机副本、撤回同步范围、冲突逐项解决。\
**验收**：后台/杀进程/断网/重新登录后任务恢复；重复重试不产生重复照片或重复命令。

### P13 消息、邀请与账户

**布局**：按项目分组的信息中心、未读聚合、审批通知；账户页管理隐私和 AI 图像授权、设备列表、订阅。\
**交互**：消息直达对象、邀请加入项目、权限申请、设备登出和远程撤权。\
**验收**：审稿按钮权限与服务端一致；离线显示最后已验证权限状态，不把它作为上线写入凭证。

### P14 Mac 第二屏遥控器

Mac 展示全屏 Contact Sheet 或 Gallery Wall，手机连接同一身份授权设备会话。手势评分、语音评论、翻页、比较和指向选中画面；设备发现只在用户允许的范围进行；所有正式操作仍进入同一个命令流。Mac 离线而手机云端在线时明确提示遥控不可用，不能假装实时成功。

## 6. 关键跨设备工作流与状态约束

### W1：Mac 私有项目 → 手机继续

Mac 显示 `在手机上继续`，先生成同步预检：元数据、选择/序列、画布、已授权预览、研究素材、评论、AI 关联、派生物体积。用户选择“私密多设备项目”，明确 RAW/TIFF 不默认上传。服务端创建云项目并确认资产映射，Mac 上传派生物并验证哈希；手机获得基线快照与事件游标；验证无误后开放双向编辑。遇到断网或错账不能形成两个可独立写入的权威项目，必须保持单主状态/显式恢复。

### W2：手机导入 → Mac 正式库

用户从 PhotosPicker / Android Photo Picker 选择内容；App 在授权范围复制/读取媒体，提取方向与源元数据，计算内容 SHA，在手机建立临时资产身份；上传权限明确时传到云端私有对象存储；Mac 获取事件并在项目中显示 `手机导入待入库`，按原片去重和身份映射归档。相同 SHA 不强制合并艺术上的不同使用实例（资产与板上节点不同）。

### W3：跨设备 AI 策展

手机说“从昨天导入的 300 张里找出城市与人之间的孤独感，生成 3 个不同的 20 张序列”。Agent 检查范围与费用/图像授权，按检索→缩略批量观察→重点高分辨率局部观察→候选验证→草案创建→审稿卡的顺序行动；Mac 可继续对话与编辑。没有被授权上传的本地影像必须在 Mac 本地执行器在线且授权时才能被分析，绝不由云端直接遍历 Mac 文件。

### W4：外部调色回流

Mac 筛选结果通过 Lightroom Classic 或 Capture One Bridge 引用原始资产，接收带 manifest/hash/ICC/版本的实际渲染成片；形成新的 Rendition；手机只同步授权派生预览与新版本元数据，收到“调色 A v3 已回流”。画布节点可锁定显示某个版本或跟随被批准的当前版本；不能以文件名自动错配。

### W5：AI 画布部分采纳与撤回

草案绑定源 `boardId + sourceRevision + nodeOrigins`；`simulateMerge` 通过三方差异和依赖计算返回精确预览；审批人选择变更，`applyMerge` 事务合并并记录审计/逆操作。源版本过期则重比；设备断网时选择可存本地草稿，但正式应用必须在权威端确认。补偿撤销不抹去之后不相关的人工改动。

### W6：共同审稿与出版

编辑在 Web 评论第 11–16 张，摄影师手机收到对象级通知，iPad 上通过草案修改；Mac 生成跨页和高质量输出，出版物基于不可变渲染 ID 与版次冻结。交付产物有 outputId、规范、校验结果、可追溯输入母版、真实印厂打样回执。

## 7. 云端与本地的权威模型（最重要的架构改造）

必须明确三种项目状态：

- `LOCAL_ONLY`：Mac SQLite 是权威；云端不得假装可编辑；手机仅可使用已显式导出的副本。
- `PRIVATE_SYNCED`：云端 PostgreSQL 是用户私有多端协作对象（Project、Board、Comment 等）的唯一权威；本地原片仍由 Mac 管理；各端离线操作为未确认草稿。
- `SHARED`：云端同样为正式权威，叠加 Workspace/ProjectGrant、邀请和审批规则。

**状态迁移**：`LOCAL_ONLY → SYNC_ENROLLING → PRIVATE_SYNCED ↔ SHARED`。若现有系统 `LOCAL → PROMOTING → SHARED_CACHE`，必须有向后兼容的迁移适配器；在迁移/回滚期间锁定旧的本地正式写入。任何时候不能出现 Mac SQLite 与 PostgreSQL 同时允许对同一 Board 执行正式写入的“双主”状态。

**权威操作与 CRDT 分界**：项目身份、评分、选片、正式画布、采纳、版本、ACL、出版物、最终批准全部走有条件的版本命令和事务回执；临时光标、presence、输入指示等走实时事件；可选 CRDT 用于明确范围内的协同富文本/草稿笔记，不能绕开正式命令的审批和审计链。

## 8. 目标领域模型与版本策略

| 实体 | 核心字段（示意） | 原则 |
|---|---|---|
| Workspace / Project | id、owner、mode、revision、brief、policy | 项目权限和单主状态 |
| SourceAsset | id、projectId、sha256、kind、locator、originalColorProfile、rights | SHA 辅助去重；稳定业务 ID 才是引用身份 |
| AssetAlias | localPhotoId、sourceAssetId、deviceId | 兼容旧库与跨设备映射 |
| EditVariant | id、assetId、provider、externalId、label | 不把 Lr/C1 调色参数伪装成可跨引擎还原 |
| Rendition | id、variantId、sha256、bitDepth、dimensions、iccDigest、renderRecipe | 不可变版本；不同用途有独立派生 |
| PreviewDerivative | id、renditionId、pipeline、pixelSize、icc、sha256、expiry | AI/检视/分享/印前隔离 |
| Sequence | id、assetRefs、order、revision、intent | 作品顺序与图库筛选分离 |
| Board / CanvasNode | id、projectId、revision、source/variant refs、geometry、style | 同节点跨端一致 |
| EditorialChange / Merge | draftId、baseRevision、changeIds、receipt、inverse | AI 部分采纳可审计可撤回 |
| Annotation / Thread | id、anchor、nodeId、revision、scope、resolution | 评论跟随对象，权限独立 |
| Conversation / TaskRun | id、projectId、scope、model、step、usage、toolEvents | 跨端任务延续、可追溯 |
| MemoryEvidence | assertion、scope、source、confidence、retraction | 主张/偏好分层，不等于聊天记录 |
| Publication / OutputJob | id、pageFrames、renditionLock、icc、preflight、delivery | 出版引用不可变源和验证记录 |
| SyncDevice / Event / Outbox | deviceId、eventSeq、cursor、commandId、state、retry | 可恢复增量同步 |

**禁止**使用照片文件名作为跨软件/跨设备唯一键；禁止把低清预览路径写成印刷母版；禁止把 AI 自然语言解释直接作为已执行的命令凭据。

## 9. 跨端增量同步协议

### 9.1 首次同步

客户端登录→注册设备与能力→`GET /projects/{id}/sync/bootstrap` 得到基线版本、对象 ID 映射、授权范围、媒体 manifest、事件 cursor→校验完整性→写 SQLite 缓存→标记 READY→按优先级下载视口所需媒体。首次同步应该可暂停与续传，不以必须下载所有原片作为阻塞条件。

### 9.2 持续同步

服务器发布持久事件（`eventSeq` 单调递增），实时 WS 只起低延迟通知作用；客户端按游标用 `GET /sync/events?after=` 拉取不可缺失的事实。每条命令带 `commandId`、`idempotencyKey`、`expectedRevision`、`actor`、`operation`；服务端权限校验和写入在同一事务。断线或应用挂起后必须从事件游标重拉，不依赖 WS 恰好投递。

### 9.3 离线 outbox

本机 Outbox 状态：`LOCAL_DRAFT → QUEUED → IN_FLIGHT → ACKED / CONFLICTED / REJECTED / UNKNOWN`。对 `UNKNOWN` 必须按原幂等键查询回执，不能生成新 key 盲重发。断网期间可本地预览命令产生的画布变化，但每个未确认操作有明显状态标签。权限撤销后待提交数据留在本机受控缓存，由用户导出私有草稿或放弃，不能强制上传。

### 9.4 并发与结构冲突

单条评分可选择明确业务规则（例如带实体版本的 compare-and-set）；正式 Sequence/Board 统一 optimistic concurrency；Board AI merge 使用 base/current/draft 三方合并，冲突到属性/节点级。评论的普通追加操作可幂等并发；删除/锁定/更换 Rendition 涉及引用依赖，不能直接 last-write-wins。

### 9.5 可信跨设备验证

永久存储“最后确认同步点”；定期全量快照校验与事件 replay；离线写入、切换账户、重建缓存、撤销授权、迁云中断和旧版本客户端混用均有演练。跨端权限模型不允许依赖前端隐藏按钮。

## 10. 后端服务与 API 模块设计

将现有 `apps/api` 扩展为清晰模块，并保留已有项目/邀请/实时接口兼容：

| 服务模块 | 新增/强化能力 | 高风险点 |
|---|---|---|
| Identity & Device | OIDC/Passkeys（按平台支持）、设备注册/撤销、refresh rotation | 令牌生命周期、安全存储 |
| Project Authority | 私密同步、SHARED、ACL、正式版本命令 | 迁移单主、角色边界 |
| Asset Manifest | 哈希/元数据、派生策略、上传会话、清理 | 原片隐私、错误去重 |
| Color Derivative | 输出设备无关的带 ICC 显示派生、金字塔与派生 hash | 错色/内存/缓存键 |
| Canvas Domain | 同义增量命令、快照、merge/undo/redo、依赖 | 并发与历史不一致 |
| AI Runtime | 统一会话、执行/暂停、图像 consent、工具调用、证据 | 越权、成本、错误自信 |
| Research & Knowledge | 灵感、参考材料、来源信息、跨项目检索 | 版权与内容可信 |
| Collaboration | 评论、审核、presence、通知、组织 | 误推送影像、撤权 |
| External Editors | manifest、变体、调色结果、自动回流通知 | 外部版本错配 |
| Publishing | 版面、渲染锁定、印前、交付状态 | 低质量缓存误入正式输出 |
| Billing & Entitlements | AI 配额、移动商店购买、跨端权益 | 双重计费、商店合规 |
| Observability | tracing、audit、sync lag、queue、隐私日志 | 泄露个人摄影素材 |

建议引入 API 版本与能力协商：`GET /client-capabilities`，实现 `minClientVersion`、schemaVersion、feature flags 与保守降级；服务端可拒绝过旧客户端的破坏性命令，不应以更新失败导致用户失去本地只读能力。

## 11. Agent 平台：Unified Focal AI

### 11.1 必须统一的概念

**一个聊天入口**对应多种内部执行路径：知识问答→直接回复；谈照片→检索并调用 Vision；谈叙事→读取 Board/Sequence→分析证据；要求改变作品→创建 Draft→模拟→用户审批；长耗时任务→云 TaskRun 与设备内执行器分工。用户不需要选择 Chat 模式还是 Task 模式。

### 11.2 推理与工具循环

`User Intent → Project/Snapshot Binding → Permission+Consent Check → Planner → Tool Calls → Evidence Validation → Response/Artifact → Human Approval → Formal Command Receipt → Memory Proposal`。

上下文由系统按需组装：当前照片/画布节点、关联 Rendition、已观察内容、项目 Brief、外部调色版本、近期明确偏好、任务预算；大项目的全图库只能检索，不允许 `LIMIT 200` 后把数量冒充项目全部。视觉分析应具“已实际查看”证据标志。工具结果不可信，应校验对象与权限。

### 11.3 工具集（最终目标）

`search_assets`、`inspect_photos`、`compare_photos`、`get_project_context`、`get_board`、`get_sequence`、`get_edit_variants`、`get_publication_spreads`、`get_editorial_history`、`search_photography_knowledge`、`create_board_draft`、`place_photos`、`move_nodes`、`group_nodes`、`add_editorial_note`、`add_connector`、`simulate_merge`、`prepare_publication_draft`、`request_human_approval`、`read_task_status`。写操作必须在授权的沙盒草案；正式写入只由人工确认后的命令执行。

### 11.4 长任务、多模态与跨端继续

云队列 TaskRun 具排队、运行、暂停、需用户授权、完成、失败、未知/待核账状态。手机退出、来电、断网不导致 Agent 在本地前台线程中丢任务。异步任务可通过事件/推送通知到设备；传给云模型的每张图有精确授权项目、导出版本和保留策略。视觉判断与供应商路由以真实摄影评价集为依据，不能仅由 prompt 复杂程度估算专业水平。

### 11.5 审美记忆

`PreferenceProposal` 必须显示证据、适用范围（当前作品/作者长期）、置信度和保存按钮；用户可以拒绝、修改、撤回。对“喜欢留白”“偏爱 28mm 视角”这类偏好，可用于以后排序推荐；但不能把一次修改自动升级成全局个人特征。永久记忆与对话历史分别建表。

### 11.6 摄影专业评测

至少建立四类真实摄影数据集：城市/街头日常、人物与家庭纪实、胶片扫描及色彩序列、摄影书/展览编辑；盲评维度为观察事实准确性、创作建议质量、叙事连贯性、重复控制、理由可信度、工具执行正确率、编辑者采纳率。人类编辑保有最终判断，A/B 使用相同输入和预算，结果可复现。

## 12. 专业色彩与移动影像引擎

**四条明确分离的处理路径**：缩略图（快速）、AI 预览（隐私/费用优先）、专业检视（ICC/色深/动态分块）、印刷母版（真实高位深源渲染，绝不从 AI 缓存放大）。

macOS 继续完善 ImageIO/Core Image/ColorSync/必要时 Metal；iOS/iPadOS 根据设备颜色空间与显示状态做受控变换；Android 使用原生支持的颜色空间、高位深与 GPU 路径，在目标机型上验证。对于 TIFF/RAW/HEIC/BigTIFF 的覆盖范围写明确支持矩阵，不能泛称所有图像都保真。

**缓存键**：`assetSha + renditionSha + decodeVersion + imageRecipe + sourceProfileDigest + outputIntent + targetColorSpace + dimensions + orientation`。显示器 ICC 变动要触发依赖显示设备的缓存/渲染失效；不可把一个 sRGB 预览误标 P3。

**专业用户 UI**：照片信息显示 source profile、rendition、bit-depth、当前预览类型；颜色管理失败明显提示。默认不施加美化滤镜。P3/HDR 只有在源和设备都明确支持时启用。True Tone、Night Shift、亮度和观看环境影响不得被忽略；手机和 iPad 非最终印前签样。

**质量验收**：以包含 sRGB/P3/Adobe RGB/ProPhoto、带/缺/错误 ICC、16-bit TIFF、100MP 以及胶片扫描件的内部参考集进行色彩变换数值回归、肉眼评审和多设备对照。参考色差阈值按测试条件与测量系统制定，而非随口保证 ΔE=0。缩放和图块切换不得出现可见色跳；出错不得静默退到错色的 8-bit 预览。

## 13. 安全、授权、合规与隐私

- 最小权限：系统相册仅选择用户指定照片；相机、麦克风、通知分别解释用途并按需请求。
- 私有源片不默认上云；手机录音、位置信息、EXIF/GPS 具独立上传和分享策略。默认公开评论预览去除 GPS 等隐私元数据。
- 临时媒体下载票据短期有效，并在真正读取时重查 ACL；撤权无法收回已经下载的像素，产品必须明示。
- 凭据储存在 Keychain / Android Keystore 等系统安全容器，不写前端持久明文；缓存可以加密或依设备安全策略保护；用户能按项目清除离线文件。
- Agent 上传外部模型须有可追溯图像授权，任何角色、项目分享和访问令牌不能自动转化为模型上传同意。
- 法律/条款/数据保留：关注 GDPR 等适用法域、儿童肖像、客户商业保密、摄影作品著作权与第三方参考资料授权；数据删除与备份清理有延迟窗口说明。
- 商业化：iOS App 内解锁数字能力与订阅的购买方式需遵循目标商店地区的 App Store Review Guidelines；网页已购买权益的展示和可购买选项分别设计，不能假设统一外链收款在所有地区允许。
- 安全工程：威胁建模、依赖扫描、签名、供应链 SBOM、审计、API 限流、文件解压/图像解码沙箱、尺寸与内存炸弹防护、日志脱敏、服务端权限回归。

## 14. 平台技术选择与代码库改造

### 14.1 选择原生 UI 的理由

**iOS/iPadOS**：SwiftUI 为 App shell、导航与常规表单；专业照片 Viewer/高性能 Canvas 用 UIKit/Metal 适配；笔迹优先 PencilKit 采集并规范化为跨端路径。\
**Android**：Kotlin/Jetpack Compose 为主；深度自定义画布/大型列表/GPU 走适配组件，WorkManager 承担持久后台同步；系统 Photo Picker 负责授权选图。\
**Rust Domain Core**：共享稳定的资产/命令/版本/权限/合并算法，优先以已存在领域 crate 为基础抽取，使用经过验证的 FFI 方案（例如 UniFFI 或显式 C ABI）提供客户端调用。\
**Tauri Mobile**：官方可以运行，但本项目选择最佳原生摄影交互，作为原型/内部工具备选而不是主交付方案。当前 Mac/Tauri 保持，不贸然重写成熟桌面 GUI。

### 14.2 建议的目标目录（非现状）

```text
between-frames/
  apps/
    desktop/                   # React + Tauri desktop shell
    web/                       # 现有 Web 协作
    mobile-ios/                # SwiftUI, UIKit, PencilKit, Color rendering
    mobile-android/            # Kotlin, Compose, image/gesture adapters
    api/                       # PostgreSQL + object storage + WS
  crates/
    focal-domain/              # Asset/Board/Revision/ACL shared rules
    focal-commands/            # Command validation, merge simulation
    focal-sync/                # Cursors, outbox, conflict state, replay
    focal-media-contracts/     # ICC, rendition/derivative descriptors
    focal-agent-contracts/     # Task/Tool/Consent protocol
  packages/
    contracts/                 # OpenAPI/JSON Schema, generated clients
  services/
    ai-worker/                 # Cloud task orchestration
    derivative-worker/         # Media processing & verification
  tests/
    device-parity/
    sync-chaos/
    visual-color/
    accessibility/
    agent-blind-eval/
```

**迁移策略**：先抽数据契约和“可重复验证的纯领域函数”，不直接把 `src-tauri` 巨型代码复制到移动端；SQLite 迁移与 PostgreSQL 迁移各自独立，靠契约测试保证领域语义一致。不可变历史记录严禁用直接 SQL 脚本伪造旧审批。

## 15. 关键 API / 事件定义示例（目标契约）

| 类型 | 路径或动作 | 作用 |
|---|---|---|
| GET | `/api/v3/projects/:id/sync/bootstrap` | 完整基线、权限、游标、派生 manifest |
| GET | `/api/v3/projects/:id/sync/events?after=cursor` | 可恢复的持久事件 |
| POST | `/api/v3/projects/:id/commands` | 带预期 revision 的正式命令 |
| GET | `/api/v3/projects/:id/boards/:boardId/snapshot` | 画布按版本快照 |
| POST | `/api/v3/projects/:id/drafts/:draftId/simulate-merge` | 部分采纳预演 |
| POST | `/api/v3/projects/:id/drafts/:draftId/apply-merge` | 事务采纳与审计回执 |
| POST | `/api/v3/projects/:id/merges/:mergeId/revert` | 补偿撤销或冲突返回 |
| POST | `/api/v3/projects/:id/agents/conversations` | 创建或恢复跨端对话 |
| POST | `/api/v3/projects/:id/agents/tasks` | 异步 Agent 工具执行 |
| GET | `/api/v3/projects/:id/agents/tasks/:taskId/events` | 执行证据/状态/消费 |
| POST | `/api/v3/projects/:id/assets/derivatives/request` | 根据权限申请派生预览/图块 |
| POST | `/api/v3/projects/:id/transfers/sessions` | 原片或派生物显式上传 |
| WS | `/api/v3/realtime` | 有权限的事件到达通知与 presence |

**版本兼容**：现有 `/api/v2` 不能立即全部替换；v3 API 应和 v2 并行，迁移策略按 feature flag/客户端能力协商实施。任何新接口在 server 与 mobile 生成客户端之间必须来自同一契约，避免历史上 FocalChat 的严格 schema 不一致问题。

### 15.1 正式命令请求示例

```json
{
  "schemaVersion": 3,
  "commandId": "UUID",
  "idempotencyKey": "unique-client-generated-key",
  "projectId": "PROJECT_ID",
  "boardId": "BOARD_ID",
  "expectedRevision": 42,
  "operations": [
    {"opId":"UUID", "type":"MoveNodes", "nodeIds":["NODE_ID"], "dx":24, "dy":-12}
  ]
}
```

### 15.2 事件消息示例

```json
{
  "eventSeq": "9842001",
  "projectId": "PROJECT_ID",
  "kind": "BOARD_COMMAND_COMMITTED",
  "entityId": "BOARD_ID",
  "revision": 43,
  "commandId": "UUID",
  "actorId": "USER_ID",
  "createdAt": "2026-10-27T00:00:00Z"
}
```

正式客户端从事件队列得知 revision 改变后应读取权威回执/增量；事件本身不是给 UI 随意写新状态的指令，客户端还需重新检查已失效的授权。

## 16. 实施阶段：以 Gate 而不是武断工期驱动

| Gate | 实施重点 | 交付物 | 退出标准 |
|---|---|---|---|
| G0 当前基线冻结 | 主分支健康、最新代码映射、恢复与回滚 | 架构盘点/测试快照/合规列表 | CI 关键门禁通过；旧项目恢复演练成功 |
| G1 共享领域契约 | Rust Domain Core、对象 ID、命令 schema | 共享库、OpenAPI、生成客户端 | Mac/iOS/Android 同一命令得到同一结果 |
| G2 云端双向权威 | PRIVATE_SYNCED、迁移、持久事件/快照 | 同步服务、项目模式、权限 | 单主、断点、冲突、撤权测试通过 |
| G3 媒体与色彩 | 派生/分块、ICC、设备显示链路 | Derivative Worker/Native Viewer | 参考色彩与高像素设备级验收通过 |
| G4 原生移动基础 | iOS/Android 登录、项目、图库、缓存 | 可安装内部版 | 真实手机导入选片/离线/恢复闭环 |
| G5 Focal AI Unified | 会话、视觉、工具、跨端任务 | Agent 服务 + 聊天/证据 UI | 300 张真实项目多轮任务可完成与续接 |
| G6 手机画布审稿 | 拖拽、评论、部分采纳、撤销 | Phone Canvas + Review | 跨端同板修改和冲突全部可恢复 |
| G7 iPad Pro | Pencil、无限画布、专业 Viewer | iPad Studio | 大画布真实项目和手写评审完成 |
| G8 Creative Pipeline | Lightroom/C1 回流与跨端版本 | Bridge V2 / Manifest / Notifications | 来源无误匹配、新版本跨端自动可见 |
| G9 Publishing | 摄影书、展览、印前、打样 | Layout / Print Engine | 真实印厂/装订样书测试通过 |
| G10 商业发布 | Store 发行、订阅、数据权利、SRE | 发布包、运营监控、隐私文档 | 安全、稳定性、购买恢复、审核全部通过 |
| G11 专业生态 | 摄影编辑服务、教育/团队高级工作流 | 可选 marketplace/服务标准 | 权限、版权、纠纷与履约闭环，不污染创作核心 |

**并行关系**：G1 和 G0 同步准备；G2/G3 是移动全功能上限；G4 可在 G2 合同稳定后并行 iOS/Android；G5/G6 可与 G3/G4 并行开发但不能在跨端命令/视觉验证前正式发布；G8/G9 与移动 UI 不耦合，但必须使用同一资产/版本契约。

## 17. 研发 Workstreams 与责任边界

**WS-A 产品体验**：用户研究、旅程、原型、设计系统、手机/iPad/桌面一致性、无障碍和 Figma 设计文档。\
**WS-B Core & Sync**：Rust crate、跨端命令/合并、设备身份、outbox、冲突与恢复、旧项目迁移。\
**WS-C Cloud**：PostgreSQL 权威、事件队列、权限、媒体 ticket、WebSocket、后台 worker、运维指标。\
**WS-D iOS/iPad**：原生 App、Media Viewer、Pencil、后台传输、设备安全存储、App Store 发行。\
**WS-E Android**：Compose UI、渲染与手势、Photo Picker、WorkManager、Keystore、Play 发行。\
**WS-F Imaging**：ICC、ImageIO/ColorSync/Core Image/Metal、派生工厂、色差/性能基准。\
**WS-G Agent**：统一 Runtime、模型/工具路由、语音、授权、专业评测和记忆。\
**WS-H Creative Production**：Lightroom/C1 Bridge、排版、输出/印前、印厂规范。\
**WS-I QE/Security/SRE**：契约和端到端回归、设备农场、压力/混沌、安全、数据恢复和发布流水线。

每个任务必须同时具备 UI 设计与服务端合同、迁移脚本、访问控制、错误态、遥测、可复现测试、回滚说明；不能以“页面能点击”作为完成定义。

## 18. 质量门禁与可量化体验目标

| 领域 | 目标门禁（初始产品目标，需真机验证） |
|---|---|
| 正式数据正确性 | 已确认命令 0 丢失；幂等重复 0 双写；迁云 0 双主；事务失败 0 部分应用 |
| 图片保真 | ICC 与渲染路径可追溯；参考图颜色验证；无静默错色；印前不引用低清代理 |
| 同步感知延迟 | 正常网络下同项目正式状态跨设备 P95 < 2 秒；WS 断线后正确补拉 |
| 启动体验 | 已登录并有缓存时温启动目标 P95 < 2 秒（分档真机） |
| 选片体验 | 已缓存图墙滚动在目标机型 60fps 档位稳定；1000 张项目可浏览 |
| 画布体验 | 视口缩放、手写、拖拽在参考机型无明显交互卡顿；大画布虚拟化 |
| AI 可用性 | 图像观察来源可点击；核心工具任务成功率按盲测公布；失败可重试无双扣费 |
| 离线恢复 | 关机/杀进程/断网后草稿与 outbox 可恢复；无静默覆盖 |
| 安全 | 高风险 API 权限用例全部通过；撤权后不能再取得新票据/新媒体 |
| 无障碍 | VoiceOver/TalkBack、动态字体、颜色无障碍、焦点与键盘路径通过人工验收 |
| 商店/运营 | 崩溃、ANR、订阅恢复、退款核对、用户数据导出/删除均可操作 |

测试矩阵覆盖：低/中/高档 Android；LCD/OLED iPhone/iPad；标准/广色域显示；RAW、JPEG、16-bit TIFF、胶扫；Wi-Fi/蜂窝/离线；单人与多人；Owner/Editor/Reviewer/Viewer；已授权/已撤权/expired；旧版本客户端；大量评论和节点；多账号切换；系统后台限制；App 更新失败。

## 19. 发布策略与失败回滚

- **Internal Alpha**：Mac 与两种手机真机接同一项目，读取/写入/撤销基本成功；完全隔离测试环境。
- **Professional Closed Beta**：邀请艺术摄影师和摄影编辑真实项目；强制错误反馈和备份；逐项验证画质/叙事质量。
- **Cross-platform Open Beta**：完整 PRIVATE_SYNCED 与权限、AI、画布审稿；iPad Pro 功能逐步开放。限制无法安全保证的功能，而不是假装成功。
- **Production**：iOS App Store + Android Google Play + Mac 独立发布与 Web；跨版本矩阵、隐私/支付/数据导出删除、告警与值守到位。

渐进迁移：旧本地项目始终可以导出；升级前备份；schema 扩展先于客户端切换；特性开关允许关闭同步/AI 新能力；服务器回滚不能识别新命令时必须改为只读保护；已经写入的新版本不能被旧客户端覆盖。

## 20. 风险、取舍与必须承认的物理限制

1. **高精度色彩并非所有屏幕等价**：校准条件和显示能力无法由 App 完全控制。正式印前签样要求指定设备/环境。
2. **本地原图不上传时云端不可远程渲染原片**：要离线查看某张 100MP 扫描件，应提前下载高质量派生或授权原片传输。
3. **没有自动授权的 Mac 磁盘访问**：移动 Agent 不能绕过隐私；可选可信本地执行器需设备在线和用户授权。
4. **跨端实时编辑不等于 CRDT 万能**：正式摄影编辑的选片/合并/批准有事务与版本语义。
5. **生成更好的摄影作品不能只靠更大模型**：需要真实数据、专业工具、观察证据与摄影师盲评。
6. **Native 双端 UI 增加维护成本**：这是为专业体验作出的明确取舍，用共享领域与契约测试而非复制 UI 缓解。
7. **营销声称不能领先于现实**：“跨端同步”“专业色彩”“完整出版”只在完成对应 Gate 后对外宣称可用。
8. **持续云计算/云存储有实际成本**：虽然产品设计不按成本减配，运行时仍要明确资源配额、计费透明和异常限额。

## 21. 本轮开发任务优先级（可直接下发）

**立即冻结目标协议并启动 G0–G2；不要先复制 desktop UI 搭一个手机壳。**

- **FRM-001** 固定最新 HEAD、完善受影响测试和 CI；列出所有正式源/草案/版本读写接口及回滚路径。
- **FRM-002** 定义单主 ProjectMode 与迁移状态机，建立 v2→v3 兼容表和失败恢复脚本。
- **FRM-003** 统一 Project/Asset/Board/EditorialMerge/AgentTask ID 与共享 schema；制定数据库迁移顺序。
- **FRM-004** 抽取 Rust 纯 Domain Core；Mac 当前 API 适配，保证旧库测试完整通过。
- **FRM-005** 建立 `bootstrap + event cursor + command receipt + outbox` 同步链路与混沌测试。
- **FRM-006** 扩展云端私密多设备与受控媒体派生能力，建立上传授权、票据与撤权检查。
- **FRM-007** 建立 iOS/Android 原生 Shell、登录、系统选图、SQLite 缓存及设备安全密钥。
- **FRM-008** 将 Color Engine 的可信派生与高分辨率 tile 接入 macOS Viewer/Canvas，制定移动对应接口。
- **FRM-009** 统一 FocalChat 与 AgentRuntime 的跨端会话和工具任务、修复调用协议差异。
- **FRM-010** 移动图库/选片/比较与同步状态组件，真机回归。
- **FRM-011** 手机画布/部分采纳/冲突/撤回 UI，复用正式领域合并命令。
- **FRM-012** iPad Pencil/多栏/高保真 Canvas，跨设备位置/笔迹一致性测试。
- **FRM-013** Lightroom/Capture One 版本事件通知和跨端审阅闭环。
- **FRM-014** 出版跨页审稿/印前输出权威流程。
- **FRM-015** Store 购买与权益、隐私导出/删除、SRE、安全、验收、发行。

具体拆分与每个阶段的自动化 Gate 见配套《实施 Backlog 与验收矩阵》文件。

## 22. 工程依据与最终签收

本方案基于 2026-10-27 检查的 [FocalRoom GitHub 主仓库](https://github.com/yihua-bot/focalroom)，重点核对 `apps/api/src/authority.ts`、`realtime.ts`、`src-tauri/src/color_engine.rs`、`agent/creative.rs`、`canvas_review.rs`、`EditorialReviewWorkspace.tsx` 等文件；实施时应冻结具体提交 SHA，并以真机和集成测试结果为准。

官方参考涵盖 [Apple PhotosPicker](https://developer.apple.com/documentation/photosui/photospicker)、[PencilKit](https://developer.apple.com/documentation/pencilkit/pkcanvasview)、[后台传输](https://developer.apple.com/documentation/foundation/downloading-files-in-the-background)、[Android Photo Picker](https://developer.android.com/training/data-storage/shared/photo-picker)、[WorkManager](https://developer.android.com/develop/background-work/background-tasks/persistent)、[Tauri Mobile](https://v2.tauri.app/develop/plugins/develop-mobile/) 和 [App Store 审核规范](https://developer.apple.com/app-store/review/guidelines/)。完整开发清单、接口草案及更多参考链接见配套文件。

**最终签收标准**：真实摄影师能够在手机捕捉想法、拍摄并导入，在 Mac 可信检视原片，通过 Focal AI 观察真实影像和生成草案，在 iPad 与编辑协作画布，通过 Lightroom/Capture One 调色回流，跨端局部采纳或撤回修改，完成摄影书和专业印前，并长期保存可追溯的作品与审美决策。整条链路中照片身份不丢、版本不乱、权限不越界、作品不被静默覆盖。
