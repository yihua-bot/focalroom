# ADR 0020：v2.0 M5 提案协议

日期：2026-10-15。

- migration 0012（proposals/proposal_operations）：operations 化提案（add/move/remove + operation_id + to_index + reason），状态机 draft→submitted→applied/discarded，附 approval_nonce（创建时生成、apply 必须匹配——防重放与越权确认）。
- 权限经 local-domain 既有判定：can_propose（create/submit）、can_approve（apply）；Agent principal 仅可 ProposalCreate（纯函数层已拒 submit/apply）——"Agent 提案、人工审批"的核心闭环不依赖提示词。
- apply 语义：accepted_operation_ids 子集校验（空集=驳回 discarded）；base_revision 漂移即拒并要求重建（不静默 rebase）；接受的操作经 v1 唯一 CommandHandler 逐个执行——每操作独立原子、幂等（键绑定提案+操作）、进历史、可整体 Undo；不伪造跨操作事务，失败逐项如实报告。
- add 映射 v1 置 keep（组合落位后续扩展）；move 重排；remove 置待定（评分备注保留）。与 v1 草案机制并存：v1 draft 服务 Mock Agent 通道，v2 proposal 是审批协议层——两者不混用，未来统一在 M5 收尾评审决定。
- Operation.reason 缺省合法（serde default）；payload 字段名对齐契约 snake_case（to_index 不做 camelCase）。
