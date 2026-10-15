# ADR 0021：v2.0 M6 组织/席位本地骨架

日期：2026-10-15。

- migration 0013（v2_creators/workspaces/memberships/workspace_audit）：SHARED 模式的本地镜像子集；默认幂等插入 local-workspace + local-owner（OWNER 席位）。
- 席位语义：上限 1–500；满员拒绝（BUDGET_EXCEEDED，含当前/上限计数）；撤回释放席位；被撤回者不可自行恢复（PERMISSION_DENIED）；OWNER 席位不可撤回（须先转移所有权，属 M4 云流程）。
- 角色集合与 v2 权限模型一致（OWNER 由创建产生，ADMIN/EDITOR/REVIEWER/VIEWER 可分配）；非法角色拒绝。
- 全部组织变更写 v2_workspace_audit（append-only）；本骨架不冒充云能力——真实多设备/教育隔离（E01 云侧）依赖 M4 认证与共享服务。
- 配套 V2-COMPLIANCE 总账：24 验收场景全映射（18 pass / 2 partial / 4 missing-待外部），作为 v2.0 工程侧完成基线。
