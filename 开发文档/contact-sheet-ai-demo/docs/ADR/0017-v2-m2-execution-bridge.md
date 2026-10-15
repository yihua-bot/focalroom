# ADR 0017：v2.0 M2 执行桥——鉴权后的命令接入 v1 唯一通道

日期：2026-10-15。

- v2 envelope（snake_case、principal、workspace）经 local-domain 纯函数层解析与鉴权（权限/权威/过期），鉴权通过且属本切片的命令映射为 v1 GuiCommand，经唯一 CommandHandler 真实执行——幂等回执、历史、Undo、决策证据全部继承 v1 语义，不产生第二套写入路径。
- 本切片：photo.set_selection（UNDECIDED/KEEP/REJECT 大写枚举映射 v1 状态）、sequence.move_item（重排）、sequence.remove_item（移出序列=置待定，评分备注保留）。
- sequence.add_item 诚实返回引导：需先经 set_selection 置 keep；组合写入（一条命令完成置位+落位）在 M5 的组合命令扩展中实现，不在此偷偷两写。
- proposal.create/submit/apply 与 comment.add/resolve 返回 NOT_IMPLEMENTED 并指向 M5（审批协议与评论锚点），不假装成功。
- 幂等优先级：authorize 的 REVISION_CONFLICT 不在桥层直接拒绝——交 v1 内部裁决，使同命令重放命中幂等回执返回原结果（而非 409）；权限/权威/过期仍在执行前拒绝。
- principal 由可信入口构造（local_owner_principal/agent_principal），客户端不可自报；Agent principal 仅可提案，正式写入在纯函数层即拒（v2 分支轨迹"Agent 停止后无正式写入"的桥级验证）。
- v2→M5 范围的边界由测试钉死（v2_bridge 4 项：owner 写入+幂等重放+v1 历史、move/remove 序列语义、Agent 拒绝且零正式写入、409 与 NOT_IMPLEMENTED）。
