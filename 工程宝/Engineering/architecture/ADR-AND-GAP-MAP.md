# Architecture Decision Records & 现有工程差距清单

## ADR-050 · 作品资产取代“单张 Photo”作为中心对象

**Decision：** 启用 `SourceAsset`（母版）/ `EditVariant`（编辑方向）/ `Rendition`（不可变渲染）/ `Representation`（有界代理）分层。

**原因：** 同一胶片扫描件可能对应两版色彩、三个导出、不同画布位置与跨页使用；全部塞入 `photos` 单表必然造成身份漂移和隐式覆盖。资产 ID 与物理路径必须分离。

**Invariant：** 任何 Rendition 修改只创建新 ID；任何外部软件编辑参数的私有文件由外部编辑软件继续解释。选择哪个 Rendition 展示/印刷是有版本的项目操作。

## ADR-051 · Native Quality First

Mac Professional Viewer/Canvas 可优先复用 React/Tauri 外壳，但是源 ICC、16-bit/float、高精度瓦片、显示转换和专业软打样必须 native；先完成 ImageIO/CoreImage/ColorSync 与 Metal 的真实能力验证，再决定 NSView overlay 或原生 Viewer/Canvas。Web 只保证带证据的代理级别，显示能力差异给 UI 品质标签。

## ADR-052 · 场景而非通用“画布”

`Board` 为无限视觉探索画布；`Sequence` 为有序叙事；`Publication` 为有物理尺寸的排版。这三类视图共享影像引用却不能互相覆盖或混用几何语义。跨视图转换产生命名版本。

## ADR-053 · Shared Formal Authority + Draft CRDT

短期多人拖拽、光标、笔迹可以用 Yjs/CRDT 协同；正式项目修订只能由 ACL 权限下的语义命令写入 Postgres。普通 CRDT 消息不应直接产生无审批的正式版本；服务端执行源图/参考资产权限和 schema 验证。

## ADR-054 · Plugin Bridge 不写非公开 Catalog

Lightroom Classic 使用 Lua SDK / Publish；Capture One 使用官方 SDK/AppleScript 可用能力和 Export Recipe；双方均以 manifest + stable asset identity + hash 回流。名义上可以自动化但不在插件声明之外篡改 Catalog/Session。

## ADR-055 · Deliverable By Contract

打印文件不是“PDF 存在就算通过”。根据印厂接受的 PrintContract 配置纸张、裁切、出血、PPI、字体、ICC/OutputIntent、RGB 或 CMYK、PDF/X/TIFF 才能进入 `press_ready`。校样 PDF 和印刷交付属于独立 Job。

## ADR-056 · Task-first Intelligent Model Routing

按具体任务、专业盲评质量、受限区域、预算、能力和用户授权路由；模型变更不会偷偷扩大数据外发；所有 Agent 写操作只能生成草案。评测失败不可被“模型升级”对冲。

## Current vs Target（源码快照基于 2026-10-19 审查，开发前必须复核）

| Area | Current reviewed code | Target work | Severity |
|---|---|---|---|
| CI | 共享端标题与 E2E 预期不一致，本地 API 代理失败 | 稳定远端质量门禁 | P0 |
| Photo identity | 原图路径/hash 与选片状态 | 资产/变体/渲染与旧 Photo alias | P0 |
| Display proxy | 320/1400px RGB8 JPEG | ColorSync+16-bit/float+ICC tiles | P0 |
| Original viewer | 原图有文件/像素上限，RAW JPEG/派生预览 | 100MP/大型胶扫和高质量 RAW Rendition | P0 |
| Sequence | @dnd-kit 线性排序 | 保留并联动 Board 与 Publication | Preserve |
| Canvas | CSS 名称 `canvas-area`；无自由 Node 空间 | 无限 Board + nodes/ink/comments | P0 |
| Agent runtime | Rust tool loop + budget/drafts | 增 Board/Publication 受控工具和质量检验 | Preserve/Extend |
| Long-term memory | 关键词证据和假设 | 多模态带反例审美推断 | P1 |
| Cloud review | NestJS/PG + 评论/提案/权限 | 合作级 Board、Web viewer、实时冲突 | P1 |
| Cloud worker | 评论汇总/分歧分析 | 完整任务证据链，修复 content/task_type | P0 defect |
| Round-trip | JSON/CSV/sidecar custom JSON | 官方插件/导出 manifest/文件关联 | P0 |
| Print | A4 HTML proof | 出版物模型/印前 PDF/X/TIFF/预检 | P1 |
| Release | 有生产部分服务，AI 与正式发行需核验 | 严格签名/公证/升级/备份/收费 | P1 |

## 跨功能迁移依赖

G0 → G1（stable identity）→ G2（truthful display） → G3（Canvas）/G5（External Bridge）/G7（Publishing）。G4 Agent quality 基于 G3 产生可以作用的真实 Board 工具；G6 Shared Collaboration 以 G1/G3 的领域语义和 Native/Web 共享契约为前提。

**拒绝的捷径：** 用 1400px JPEG 放大至 4000px；将自定义 JSON 命名为 XMP；用文件名和时间戳强行配对外部调色；通过 HTML 打印声称 PDF/X；把 React 组件叫 `Canvas` 但不给自由坐标模型；使用 LLM 的 tool_call 参数代替 ACL。
