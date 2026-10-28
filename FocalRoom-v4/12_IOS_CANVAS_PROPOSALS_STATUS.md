# FocalRoom iPhone 画布编辑与提案部分采纳开发记录

日期：2026-10-28，Asia/Shanghai。接续 `11_IOS_SYNC_CENTER_STATUS.md`；目标「继续直到完成所有计划」，本片为 README 后续顺序第 3 项中的手机画布编辑与 AI 草案部分采纳——两项的服务端权威契约（boards 命令通道 schemaVersion 3、项目提案命令 proposal.apply）在既有后端早已就绪，本轮落地 iPhone 消费端。

服务端增量仅一行：`GET /boards?project=` 的 SELECT 增加 `revision` 列，为 Board 修订乐观锁提供客户端输入。零新迁移、零新依赖。

## 已实现

### FocalRoomCore

- `CanvasMoveCommand`/`CanvasCommandReceipt`：与 boards 服务端 canvasCommandSchema 逐字段一致（schemaVersion 3、幂等键=命令号、operations 唯一 opId）；回执校验 `newRevision == baseRevision+1` 且 `affectedNodeIds ⊆ moves`。
- `Proposal` DTO（含操作列表与审批 nonce）与 `ProposalApplyCommand`：构造时校验快照 SHARED、`can_approve`、提案 SUBMITTED 与操作闭包；空采纳即驳回的修订语义（采纳推进一版、驳回去不推进）在客户端回执校验中同样体现。
- `ProjectSnapshot.proposals` 可选解码：服务端始终返回该字段，可选保持旧版本缓存可读；`validate()` 对提案状态机、操作 op 枚举与照片归属做完整校验。
- `Board.revision` 与公开构造器、`BoardNode.x/y` 可变（命令成功后本地立即反映）、`Principal.canPropose`（与服务端 `can_propose` 等价：观察者不可改画布草稿）。

### 画布编辑（BoardReader 升级）

- 草案画布 + 可提案权限 + 在线时以“编辑模式”打开：拖动照片节点把屏幕位移换算为画布坐标，提交 moveNodes 命令；成功后本地更新节点位置并推进 Board 修订。
- 他人先行修改返回 409 REVISION_CONFLICT 时自动重取画布列表与节点、刷新本地修订并提示“画布已被其他设备修改”；其他错误如实显示。
- 正式画布仅接受冻结转换、冻结画布只读（服务端既有生命周期，客户端如实标注）；photo 节点用高优先级手势保证拖动优先于画布滚动。

### 提案审阅（ProposalsView 新增）

- 图库右上角“提案”入口：待审阅（SUBMITTED）与已处理（APPLIED/REJECTED）分区；操作项用按钮行勾选（iOS 26 表单内系统开关命中不可靠，见调试记录）。
- “采纳所选”按勾选操作提交 `proposal.apply`（部分采纳只应用勾选项）；“驳回整个提案”以空采纳提交，原提案保留。基线过期（服务端 409“原提案已保留”）、权限不足（只读提示）均由服务端裁决后如实显示，不做客户端猜测合并。

## 验证结果

| 检查 | 结果与范围 |
|---|---|
| Swift 核心测试 | 41 项（5 项跳过：HEIC 编码器与无环境联调）0 失败；含 3 项新增：画布命令编码键集与回执校验、提案 DTO 解码、提案应用守卫（未知操作/无审批权拒绝）与载荷键集 |
| Swift → 真实隔离后端 | Unified 联调旅程通过：画布移动推进 Board 修订、节点坐标落库核对、过期基线 409；提案一全量采纳（序列 [p1,p2]、正式版本 +1）、提案二驳回（版本不变）、提案三留 SUBMITTED 供 UI |
| 原生界面旅程 | 独立评审模拟器 1 项 0 失败（203 秒）：提案部分采纳（取消 move 项、仅采纳 add、表单收起与待审区清空双断言）→ 选片 → 比较 → 研究 → 同步中心 → 画布编辑模式断言与真实拖拽 → 迁云/设备/离线/导出全链路回归保持 |
| 数据库终局断言 | 正式版本 =4（选片/全量采纳/UI 选片/部分采纳各一次）；序列 = [p1,p2,p3]（部分采纳只应用勾选项）；提案状态 = [APPLIED,APPLIED,REJECTED]；Board 修订 ≥2 |
| 工程检查 | 类型/Lint/契约通过；Release arm64 通过；11 份变更源码无行尾空白、SHA256 归档 |

证据：`apps/mobile-ios/test-results/focalroom-pg-tests-pvUdbk/`（含画布编辑截图的 xcresult 与 ios-verification.json）；日志与源码摘要 `apps/mobile-ios/test-results/canvas-proposals-verification-2026-10-28/`。

## 调试记录（保留为后续参照）

- iOS 26 表单内系统 Switch 的无障碍命中点与控件实际位置不一致，点击不翻转（两次点击标签均不变）；改为按钮行勾选，语义相同且点击稳定。
- 节点视图附加标识符与手势后被提升为独立无障碍元素，内部 `preview.loaded.*` 子元素被吸收；画布段改为断言节点元素本身。
- **提案基线次序**：旅程先做 KEEP 选片（rev2→3）再采纳 base_revision=2 的提案，服务端按契约拒绝（409“基线已变化”）；早期“已处理”断言被 Swift 旅程留下的已决提案假阳性掩盖。修复为旅程次序调整（先在 rev2 采纳 →rev3，再选片 →rev4）并以“表单收起 + 待审区清空”双断言锁定真实结果。
- 测试卫生：复用评审模拟器时，多次失败旅程各自登录不同端口的服务器，离线索引按设计累积多条目；`--ui` 改为安装前卸载旧包，保证每次旅程干净数据容器。

## 边界

- 拖拽编辑的触摸手感、与 Mac 并发移动同一节点（E2E-06 场景）需真机与 Mac 端联调验收；模拟器仅证明命令路径与冲突处理正确。
- 提案创建仍在 Mac/AI 侧，iPhone 本轮为审阅消费端；画布复杂节点（文字/笔迹/编组）仍以类型标记显示。
- 画布命令回执（fr_canvas_commands）尚无 GET 查询入口（项目命令回执查询已有）；画布命令在客户端当前为即时提交，未进离线待提交队列。

## 后续

README 第 3 项剩余：统一 AI 会话/TaskRun 与受控视觉工具接入（Focal AI 完整会话）；随后第 4 项（推送、版本回流/出版审阅、StoreKit、数据权利、真机验收、TestFlight）。本轮未提交、推送、部署、签名或进入 TestFlight；远端 CI 未执行。开发期间共享 HEAD 由外部会话推进至 `a408229`（G2 高分辨率缩放解码），本轮改动保持工作树增量。
