# ADR 0023：v2 评审修复的事务、权威与路由边界

日期：2026-10-15。用户授权继续修复评审列出的八项问题。本决策替代 ADR 0020 的逐操作独立提交与 add 不落位语义，补足 ADR 0018 的实际执行接线及 ADR 0022 的本地写入门槛；不表示这些里程碑整体完成。

## 审批与回执

新增严格 GUI 命令 ApplyProposal（从契约生成 TS/Rust DTO），仍只由人类 CommandContext 执行。CommandHandler 增加 crate 内 execute_tx，保留旧 execute/execute_gui 外部接口与历史回执指纹。审批在同一写事务内依次解释获批操作；后续 move 使用前序结果，add 使用获批 to_index。任一失败全部回滚，成功只推进一次正式 revision、生成一个历史组；Undo/Redo 还原作品状态，审批记录继续标明曾经执行。

空选是显式驳回，不改变作品 revision、历史或偏好证据；因此该命令的 metadata 更新不随“正式状态无变化”保存点回滚。偏好证据只记录人工实际生效的审批 ID/操作 ID，模型 reason 不升级为人工偏好。

v2 桥持有 BEGIN IMMEDIATE，并在同一事务中完成权限重验、业务执行和持久化回执。新增 0017 的 v2_command_receipts 保留完整 envelope + actor 指纹；同请求重放返回原回执，身份相同但 payload/ID/键组合变化拒绝为 IDEMPOTENCY_CONFLICT。不同 command_id 复用幂等键不视为合法别名。v1 已有回执与别名机制保持兼容。新增回执不能追溯恢复旧版未记录 command_id 的评论/提案请求。

前端通过共享 parseCommand 构造 snake_case envelope，传当前项目 revision，校验回执身份、业务错误与审批结果后显示实际执行数量。

## Promotion 与旧库

v2 的权威投影来自数据库状态；事务内再次读取，调用方旧 LOCAL 快照不能放行。统一 CommandHandler 与导入/原图重连通道在同一写锁下检查门槛；PROMOTING 暂停本地新写，completed 后本地只读，failed/aborted 恢复本地写。v1 幂等回执重放允许返回已有结果，不重新产生正式修改。

0017 重建 v2_promotions 并复制全部原行，只限制同项目的活动尝试唯一。失败/中止后同版本可以用新 ID 重试，保留旧尝试；completed 项目不能重新 begin。历史迁移 0001–0016 不改；Store 升级前现有备份机制保留。仅在临时测试库执行迁移，原用户库与现有评审应用库均不升级。

当前完成确认仍为协议层注入字符串；未实现真实云签名、上传、云权威及多租户服务，S01 保持 partial。

## Router

AgentEngine 的每次模型调用前均经过 Router；视觉未命中缓存且授权有效时，在上传前也经过 Router。RegionPolicy/ServiceTier 由可信部署代码注入，不能由模型或任务自报；任务绑定的 provider 在恢复与执行时保持一致，离线任务不会因云适配器可用而自动出网。

v1 的费用单位为美分，换算到 USD，并从用户预算和档位上限中扣减整项任务累计费用。实际预留/结算继续使用既有 cloud/consent 通道，Router 不替代发送前的授权与配额核验。地区拒绝不静默换供应商，预算不足暂停，缺少已绑定提供方返回可读错误。目录估算仍不是实际账单；配置文件/设置 UI、多供应商和真人质量评测均未完成。

## 原则与验证

单一事务边界承载回执、历史、审批与权威检查，复用现有 CommandHandler，避免第二套正式写入流程（SRP、DRY）。纯 Router 与可信执行环境分开，保留注入接口（DIP）；只添加当前需要的一条命令与一份兼容迁移，不扩展云服务 skeleton（KISS、YAGNI）。

回归覆盖原八项缺陷、单组 Undo/Redo、落库故障回滚、重启重放、旧 LOCAL 快照、失败重试保留历史，以及真实执行路径的零 HTTP 拒绝。实际结果与剩余问题见 V2-REVIEW-FIXES-TEST-REPORT.md。
