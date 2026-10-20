# FocalRoom v3.0 · Screen & Interaction Contract

此文档面向 UX 设计、前端/Native/UI 工程师与测试。具体颜色与字体由 Design Tokens 定义，本文件约束**交互语义、状态和可验收行为**。仅做视觉重绘不能满足本契约。

## 1. 全局 Shell

- **主项目导航：** 主页／图库／自由画布／叙事序列／出版物／历史。
- **常驻顶部：** 项目名称、分支或正式修订、当前显示能力（Pro Color/Proxy/Missing ICC）、同步状态、成员/分享、导出。头像与在线人数可点击查看实时协作详情。
- **左侧 260±20px：** 项目资产集、保存筛选、Board tree、Sequence tree、Publication tree、外部编辑连接器。支持手动宽度和完全收起。
- **中心：** 根据工具切 Contact Sheet、Light Table、Narrative Canvas、Sequence、Photobook、Exhibition。
- **右侧 340±30px：** Inspector | Comments | Agent Tab，用户可拆成独立浮窗。
- **底部 Filmstrip：** 预览源状态、批量操作、项目后台任务、进度与警告；用户可隐藏。
- **全局 Command Palette：** 搜索功能/资产/最近操作，快捷键 `Cmd+K`，所有操作在搜索结果中提示权限和需要的当前选择条件。
- **多显示器：** 专业 Viewer 可放到另一屏；目标 ICC 与 display profile 根据其真实显示器重新计算，浏览窗口穿越两屏时采用明确主显示器或可见范围策略并提示。

## 2. Screen state table

| 页面 | 空状态 | 加载 | 可执行动作 | 失败与保护 |
|---|---|---|---|---|
| Projects | 导入、从编辑器新建、示例教程 | 最近作品骨架 | 归档/共享/打开 | Source Offline 不允许重建假文件 |
| Library | 导入或连接现有目录 | 任务进度、可取消 | 选片/分组/过滤/评级 | 损坏文件隔离、哈希重复不默删 |
| Inspector | 未选中图片提示 | 低质定位→高精度瓦片 | Fit/100%/AB/软打样 | 缺 ICC、RAW 代理明确标识 |
| Board | 提示拖放/示范板 | 逐 Node / tile 优先 | 拖拽、笔迹、评论、AI 草案 | 未上传代理、权限失效显示 fallback |
| Sequence | 从 Board 导入或选择照片 | 稳定原顺序骨架 | 按组重排、章节、留白 | 并发冲突后以正式版本差异对比 |
| Develop | 软件未连接向导 | 同步任务、校验进度 | 创建 Collection/Recipe、回流 | Ambiguous match 阻止自动匹配 |
| Photobook | 开本向导 | 布局与高质缩略图加载 | 文本/框架/页码/跨页 | 文本溢出、低 PPI 禁止印刷完成 |
| Shared | 登录/邀请 | ACL、代理、Presence 分层加载 | 评论、提案、审批 | 撤权清屏、离线草案不提交 |

## 3. Narrative Canvas 详细手势

### 3.1 Select Mode

- 单击 Node：单选。Cmd 多选。Shift 保留多选区间；空画布按下拖动为 selection marquee，Shift 增量选择。
- 移动：pointerdown 在选中节点非控制点范围后拖动；阈值 3 逻辑 px，视图开始 ghost preview；drag release 提交一个 move command；Esc 或拖出窗口取消且无正式 revision 增量。
- 放大：滚轮 + modifier 或 trackpad pinch 以光标所在画布点为缩放锚；zoom anchor 下目标稳定（容差≤1px）；手形平移不移动 Node。
- Node 缩放：八个手柄，Shift 等比（可配置），Alt 从中心；旋转可开关 15° 吸附；所有布局属性不改变物理母版。
- 分组：Cmd+G、Ungroup；分组移动后子节点相对位置不漂移；节点不允许自己成为祖先。
- 图层：forward/back/front/back，锁定/隐藏；显示/隐藏对当前草案和正式版本分别有持久策略。
- 丢失代理：Node 仍保留几何/引用，显示原资产信息与重连按钮，不删除 Node。

### 3.2 Annotation Mode

- **Comment：** 点击照片在图片可见范围内取 0–1 归一化坐标，同时保存 `rendition_id`、裁切变换、Board node ID、version；评论 pin 在缩放/裁切后仍准确落点。
- **Ink：** pointerdown 产生 stroke ID、brush/pressure/opacity，pointermove 记录 Board space polyline，pointerup commit；中断时标记 incomplete 并可清理；支持 undo stroke、erase stroke 范围；不是存到 image blob。
- **Arrow：** 从 Board 点或 Node anchor 拉到另一个节点，若吸附为端点绑定，则节点移动时 arrow endpoint 同步。
- **Text/Sticky：** 双击进入编辑态；编辑态禁用普通画布快捷键；文字变更需具语义 merge，避免彼此覆盖。
- **Comments Thread：** 回复、@提及、解决/重开、未读计数；版本历史可重放评论最初关联的 Node/Rendition。删除 Node 不删除 Thread。

### 3.3 AI Draft Overlay

- Agent 任务执行时原 Canvas 可继续使用；右上角显示进度，生成 1–3 个候选草案后以独立 Board/overlay 出现。
- Draft overlay 中新增 Node 虚线边缘、删除/移动半透明幽灵、变化摘要与原因；用户可“采纳单项/采纳此组/全部采纳/拒绝”。
- 接受前校验 latest base revision/ACL 与原片授权；冲突时打开 diff，而不是重试自动覆盖。
- Agent 只解释可见事实、主题假设与表达目的；不可输出“该照片肯定属于某某摄影师作品”之类无证据判断。

## 4. Develop Round-trip 同步对话

- **Edit in Lightroom / Capture One**：显示所选资产数量、原图可达性、编辑器版本/SDK 支持范围、导出回流目录、预计读取/同步动作、是否包含高质量原件（默认不上传）。
- **导出到 Lightroom**：插件成功创建/更新集合后显示真实 collection ID；外部软件拒绝时停在 prepared，不提示已完成。
- **Incoming Renditions Review**：分三栏：正确匹配／需要人工匹配／冲突；中栏提供原片与调色版 A/B、ICC 信息、源文件匹配证据；动作选择创建新变体或新渲染、跳过/稍后处理。
- **已有出版物**：回流后显示“新调色版可用：本摄影书仍引用 v2”；必须显式点“更新此跨页版本”再预检。
- **文件监视器**：写入中/临时文件不显示已完成，等待文件大小稳定+manifest 完整+hash 一致才候选发布。

## 5. Photobook 编辑态

- 左侧页缩略图与章节拖动，中央双页布局和物理 mm 标尺，右侧页面/文字/图片/印前设置。
- 顶栏区分 Editing Draft / Proof Review / Locked Edition。后两者不会因为自由版面操作自动变成正式印刷稿。
- 支持封面、书脊、章节、跨页、单页；插图布局初始可按模板，但全部可人工改动。
- 每个图框显示实际 effective PPI = 使用的有效像素 / 输出英寸，低于合同阈值给明确建议；无法解析 Font/ICC/bleed 时 flag not press-ready。
- 生成 Proof PDF 可快速浏览并评论；提交 Press Job 前运行 preflight，必须通过 Blocker=0 且持批准者的 approval token。

## 6. Accessibility & internationalization

- 全部交互可经键盘与辅助技术访问；Canvas 对象树提供非视觉逻辑列表/有名称节点和可访问评论，不能仅因自定义 Metal/Canvas 而丢失 VoiceOver。
- 中文/英文首发完整本地化设计；日期/数字/单位和书本开本支持本地化。专业数值始终用明确定义单位（mm、pt、px、PPI、nits）。
- 色彩状态图标不仅用颜色区分，文字/图标组合显示；缩放/拖拽操作对应明确 tooltip；减少运动设置影响过渡动画。

## 7. Prototype and usability gate

至少让 10 位摄影师/编辑在可操作版本中完成：500→30、创建两个 Board、批注一次局部色彩问题、调色往返、制作 8 跨页、输出正式交付草案。记录中断点、导航错误、错误匹配与人类可发现性。未经这类测试不能仅靠“UI 看起来像 Figma”宣称 UX 成功。
