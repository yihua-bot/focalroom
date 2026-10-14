# ADR 0005：M2 受控 Mock Runtime 与审批恢复

状态：实施中，范围见 PLAN-V1-M2。

主规格的 create_selection_draft.items 优先于随包旧 ordered_photo_ids 契约。工程工具契约使用 items（photo_id/proposed_action/position/rationale），后端检查连续位置与 ID scope；原始包不修改。ModelProvider 输出结构化 outcome，每轮依据实际工具结果重新选择，Mock 仅验证闭环与边界，不能证明真实模型自主性。

唯一 CommandHandler 的人类通道负责 ApplyDraft/RebaseDraft/UpdateDraft/DiscardDraft；Agent 通道仅允许受 ToolRegistry/PolicyGate 检查的草案与任务写入。正式 ApplyDraft 的投影、revision、审计、历史、回执同事务。全量采纳明确替换序列；局部采纳仅改变所选条目，保留其他人工状态及彼此相对顺序。rebase 只更新基线，需重新预览并明确采纳，不能静默覆盖。

M2 新增兼容 migration，不覆盖历史 migration。任务生命周期请求有持久化幂等键，工具使用 task/call_id 唯一指纹回执。模型 outcome 在执行前落库，工具效果/回执/步骤/安全检查点同事务；中断恢复重放已有结果，不重复草案。启动只将非终态执行任务标记 interrupted，不自动重启。

ContextBuilder 分层、scope 先过滤、读取字段不包含资源路径、限量人工事实、完整协议关联。文本预算为保守 UTF-8 字节上界估算；若完整恢复协议无法容纳，暂停而不丢关联。M2 无图像/网络接口，费用为 Mock 0，真实 token/图像 consent/成本属 M3。持久化上下文包含人工目标和限量备注，保留在本地应用目录，不作为 telemetry；本轮不新增明文 API key 或图像内容，静态加密与清理属后续交付。
