# ADR 0007：M4 证据驱动的偏好记忆

状态：已实现并通过自动化验收（2026-10-14）。范围见 [PLAN-V1-M4](../PLAN-V1-M4.md)，测试证据见 [TEST-REPORT](../TEST-REPORT.md)。

L2 证据表 append-only，dedup_key=command_id：幂等重放命中同一命令回执不新增行，撤销/重做命令本身不产生证据（人工原操作保留，不伪造反证据）。证据只在唯一 CommandHandler 的人工命令生效路径记录——Agent 的 create/update 草案工具位于 agent 通道，结构性排除在证据之外，不依赖提示词自律。

L3 假设物化是确定性规则：证据文本（备注/比较理由/关联照片当前备注）做 2-gram 频次聚合，≥2 条独立证据支持才生成 tentative 假设，默认 project_local；跨项目使用为 opt-in 且 v1 不生成。纠正操作将假设置 corrected + disabled_for_retrieval 并保存用户文本；同表述假设不复活（AG-11），合并路径仅扩充证据不重建。ADDENDUM 的 FTS5 检索在本轮以「状态过滤 + 确认优先/证据数/时间排序」实现——v1 假设基数小，LIKE 语义等价；FTS5 在假设量增长后作为性能优化引入，不改变检索语义。

retrieve_preferences 返回假设与证据 ID 的绑定，Agent 引用可回溯到人工操作；ContextBuilder 每轮限量注入（3 条）且带「推断非事实」标注。Memory Studio 提供证据时间线、假设卡、纠正与停用/恢复；不提供证据删除（append-only 审计性质，删除语义留产品评审）。
