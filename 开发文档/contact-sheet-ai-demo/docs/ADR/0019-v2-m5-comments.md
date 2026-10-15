# ADR 0019：v2.0 M5 评论切片（审稿协作数据层）

日期：2026-10-15。

- migration 0011（comment_threads/comments）：锚定照片或项目的讨论线程；同锚点未关闭线程自动聚合，resolve 后同锚点开新线程；锚点照片必须属于本项目（跨项目引用拒绝）。
- 通道语义：评论是协作元数据——经 v2 桥的人类通道写入，不占人工撤销栈、不作为偏好证据、不改正式选片；与 consent/标签的独立通道原则一致。
- Agent 只读：get_comments 工具解锁（M2 的 NOT_IMPLEMENTED 撤销），白名单 + PolicyGate 走原有检查；summarize_review_disagreement 保留拒绝（聚合摘要随 Web 审稿端一起设计）。评论进入任务上下文时带"不构成偏好证据"声明。
- proposal.create/submit/apply 仍 NOT_IMPLEMENTED：operations（add/move/remove + operation_id）与 v1 draft items 的映射、nonce 与 accepted_operation_ids 语义在下一轮 ADR 设计，不偷偷用草案顶替。
- 桥的 comment.resolve 状态枚举大小写不敏感；作者标签 actor_label 放 local-domain（展示用途，不含密钥/端点）。
