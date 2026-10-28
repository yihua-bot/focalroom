# FocalRoom iPhone Focal AI 文本会话开发记录

日期：2026-10-28，Asia/Shanghai。接续 `12_IOS_CANVAS_PROPOSALS_STATUS.md`；目标「继续直到完成所有计划」，本片为 README 后续顺序第 3 项中的统一 AI 会话（文本对话 + 任务恢复阅读）。服务端代码零改动——复用既有账户 AI 权威链路（`/api/v2/ai/deepseek-flash-v1/*`、项目 agent-jobs 读取）。

## 夹具注入（验证环境）

`verify-live.mjs` 为本地隔离后端注入确定性模型桩：`HostedAiConfig`（stub key + 初始额度）与 `HostedAiTransport`（回显桩回复，真实 usage 计量），并直接把夹具账户标记为已验证邮箱。整套请求仍走真实鉴权、策略头、限流、额度预留与计量结算校验——桩只替代上游模型调用，沿仓库"注入本地 Provider、不调真实模型"的既有模式。

## 已实现

### FocalRoomCore

- `AIAccount`（额度/策略/供应商/恢复能力，客户端核对 available 恒等式）、`AIReply`（宽松 output 子集解码、output_text 聚合、usage 恒等校验）、`AIJob`（PENDING/COMPLETED/REJECTED/UNKNOWN 状态机 + 回执绑定校验）、`AIConversationInput`（与服务端 strictObject 逐字段一致：model 字面量、store 恒 false、input_text/output_text 分角色）、`AgentJob`（项目分析任务读取）。
- `APIClient`：`aiAccount`/`aiSubmit`（异步提交，`sortedKeys` 固定请求字节序）/`aiJob`（回执轮询）/`agentJobs`；`call` 支持自定义请求头（策略头 + 请求编号头）。
- `AIChatStore`：按账户隔离的有界对话文件（100 条、1 MiB 上限、原子写入、损坏拒绝不静默重建）；消息携带 requestID 与 sent/unknown/failed 状态。

### iPhone 应用层

- `AppModel.aiSend`：消息先按“待确认”持久化（进程在响应前中断也不重发），异步提交后轮询至完成；完成/拒绝/未确认分别落 sent/failed/unknown，额度随发随刷。`aiResolvePending` 凭原请求编号逐条查询回执，补齐恢复文本或失败原因——不重复发送、不重复计费。
- `FocalAIView`（替换旧只读汇总页）：额度卡（可用/已用/策略/型号）、对话气泡（待确认/失败状态如实标注 + “查询待确认结果”入口）、输入栏（发送中禁用）、项目分析任务区（agent-jobs 状态 + 既有评论汇总）。退出登录清理对话文件。

## 验证结果

| 检查 | 结果与范围 |
|---|---|
| Swift 核心测试 | 45 项（6 项跳过：HEIC/无环境联调）0 失败；含 3 项新增：对话输入编码键集与分角色 part 类型、回复文本聚合与校验（非完成状态/计量不符拒绝）、对话存储重启恢复/替换/百条裁剪/损坏拒绝/清空 |
| Swift → 真实隔离后端 | AI 联调旅程通过：额度（enabled/verified/available>0）→ 异步提交+轮询至 COMPLETED（桩回复文本与 usage 断言）→ 同号重提返回同结果且 `spent` 恰增 20（不重复计费）→ agent-jobs 空列表 |
| 原生界面旅程 | 1 项 0 失败（223 秒）：Focal AI 额度卡 → 输入并发送 → 桩回复出现 → 键盘收起 → 全链路回归（迁云/研究/画布编辑/提案部分采纳/同步中心/离线浏览/导出/设备）保持通过 |
| 工程检查 | 类型/Lint/契约通过（服务端与前端零改动）；Release arm64 通过；11 份变更源码无行尾空白、SHA256 归档 |

证据：`apps/mobile-ios/test-results/focalroom-pg-tests-50NZAr/`（含 AI 对话截图）；日志与源码摘要 `apps/mobile-ios/test-results/focal-ai-verification-2026-10-28/`。

## 调试记录（保留为后续参照）

- **请求字节序稳定性**：同号重提被服务端判为“另一请求”（409 AI_REQUEST_CONFLICT）——本工具链 `JSONEncoder` 跨实例键序不稳定（12 号记录的清单问题同源）。命令类编码统一 `outputFormatting = .sortedKeys` 固定字节序；服务端摘要按字节计算，键序漂移即视为不同请求。
- **恢复语义在异步路径**：同步 `POST /responses` 按设计不落 `result_json`，轮询只有状态无结果；改用 `POST /responses/async`（结果留存 24 小时）+ 轮询，才是“凭原编号恢复”的正确路径。同号重提在异步路径幂等返回当前状态，不再触发模型调用。
- **键盘遮挡标签栏**：对话段发送后键盘未收，标签栏被遮挡导致后续旅程误入账户页；改为点按 return 键（中文键盘“换行”，否则滚动列表）并以“键盘消失”断言兜底。

## 边界

- 本片为**文本对话**：照片观察（图像输入）需要逐次授权——服务端校验 `x-focalroom-image-consent` 头与 1400px 有界 JPEG 派生预览——未在本片接入，界面明确“照片观察需另行授权，不会自动发送照片”。
- 验证环境使用确定性桩验证协议面与计量；真实 DeepSeek 的回复质量、延迟与额度消耗未在 iPhone 上实测（真机验收清单项）。
- 对话历史是本机有界文件（100 条），不是云端对话存档；换设备不迁移。

## 后续

README 第 3 项剩余：受控视觉工具（照片观察授权与图像输入）；第 4 项（推送、版本回流/出版审阅阅读、StoreKit、数据权利、真机验收、TestFlight）待最终切片收口。本轮未提交、推送、部署、签名或进入 TestFlight；远端 CI 未执行。共享 HEAD 由外部会话推进至 `a1ea760` 期间完成，本轮改动保持工作树增量。
