# FocalRoom iPhone 版本历史与阅读面开发记录

日期：2026-10-28，Asia/Shanghai。接续 `13_IOS_FOCAL_AI_STATUS.md`；目标「继续直到完成所有计划」，本片为 README 后续顺序第 4 项中的版本回流/审阅阅读面：版本历史、双版本差异、提案投影与账单目录。服务端代码零改动——四个读取端点（history / versions diff / proposals diff / billing catalog）均为既有实现，本轮只做 iPhone 消费端。

## 已实现

### FocalRoomCore

- `JSONValue`：任意 JSON 值的宽松解码与展示文本（列表/结构标注为“变更”），承载版本差异中的任意 before/after。
- `VersionSummary`（revision 为身份，倒序列表）与 `VersionDiff`（`{added, removed, changed:[{id, fields:{before→after}}]}` 双实体域投影，端点 revision 与请求参数一致性校验）。
- `ProposalDiff`：stale 分支（sequence=null，采纳会被服务端拒绝、原提案保留）与非 stale 分支（`SequenceChange{id, before, after, added, removed, moved}` 增删移投影）。
- `BillingCatalog`（与 subscriptionCatalogSchema 逐字段一致；salesAvailable=false 时价格恒空）。
- `APIClient` 四个只读方法，均带结构校验；账单目录为公开端点（无会话头）。

### iPhone 应用层

- 图库右上角“版本历史”入口：版本列表（版本号/时间/命令号），选择任意两个版本即显示服务端权威差异——照片与序列两域的增、删、字段变化（字段名 + before → after 展示文本）。客户端不做任何合并重放。
- 提案详情新增“采纳后序列预览（服务端投影）”区：新增/移除照片、移动（从第 N 位 → 第 M 位）与采纳后完整顺序；基线过期时明确提示采纳会被拒绝、原提案保留。投影区固定在采纳/驳回按钮之后，不把主操作挤出首屏。
- 账户页“账单目录”：方案名称与能力说明；销售未开放时如实标注，不显示虚构价格。

## 验证结果

| 检查 | 结果与范围 |
|---|---|
| Swift 核心测试 | 50 项（7 项跳过：HEIC/无环境联调）0 失败；含 4 项新增：JSONValue 全类型解码/展示/往返、版本差异夹具解码、提案投影双分支（含移动断言）、账单目录解码 |
| Swift → 真实隔离后端 | 阅读面旅程通过：历史含迁云初始版/当前版并倒序；0→当前跨版差异含 ios-photo-1 选片字段（UNDECIDED→KEEP）与序列变更；SUBMITTED 提案投影不 stale、added 含 ios-photo-3、before 为当前权威序列；账单目录 salesAvailable=false、双方案、价格恒空 |
| 原生界面旅程 | 1 项 0 失败（220 秒）：版本历史段（入口 → 版本行 → 双选出差异区 → 截图）+ 账户页账单目录断言（含滚动进可视区）；迁云/研究/画布编辑/提案部分采纳/AI 对话/同步中心/离线/导出/设备全链路回归保持通过 |
| 工程检查 | 类型/Lint/契约通过（服务端与前端零改动）；Release arm64 通过；10 份变更源码无行尾空白、SHA256 归档 |

证据：`apps/mobile-ios/test-results/focalroom-pg-tests-ohslir/`（含版本历史差异截图的 xcresult 与 ios-verification.json）；日志与源码摘要 `apps/mobile-ios/test-results/reading-surfaces-verification-2026-10-28/`。

## 调试记录（保留为后续参照）

- **proposalDiff 形状修正**：联调首跑解码失败暴露——非 stale 分支的 `sequence` 不是序列条目数组，而是服务端投影的增删移对象（`{id, before, after, added, removed, moved}`，photo_id 字符串数组）。契约以服务端实现为准，DTO/单测夹具/联调断言/投影 UI 一并修正。
- **投影区把主操作挤出首屏**：投影区插在操作区与采纳/驳回之间后，表单变长导致按钮懒加载不出现（旅程连败两轮同点）；投影区移到表单末尾解决——补充信息不得挤掉主操作。
- **长列表底部内容懒加载**：账户页账单区在列表底部（快照证据标题位于 y≈798 屏幕边缘），方案行不进无障碍树；旅程先滚动再断言方案名称文本。
- **非交互容器标识符不进无障碍树**：方案行是 VStack，identifier 查询不可达；以方案名称文本断言渲染。

## 边界

- 全部为只读权威数据：差异与投影均由服务端计算，客户端展示不做合并、不缓存判定；离线时入口禁用。
- 账单目录在销售未开放时只展示能力说明；不预示价格、不模拟购买入口。
- 版本历史每次取最近 100 版（服务端上限），超长项目翻页后续按需扩展。

## 后续

本片完成后，README 后续顺序中全部可在当前服务端契约与无外部资源条件下开发的 iPhone 功能均已交付（07–14 共八片）。剩余项分类见 `15_IOS_COMPLETION_AUDIT.md` 最终完成审计。本轮未提交、推送、部署、签名或进入 TestFlight；远端 CI 未执行。
