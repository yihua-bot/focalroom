# v1.0 M4 文件清单

范围为 L2/L3 偏好记忆、检索注入与 Memory Studio。工程路径相对仓库根目录。M5 不在本轮。

| 文件 | 职责与修改 |
|---|---|
| `between-frames/src-tauri/migrations/0006_preference_memory.sql` | preference_evidence（append-only、UNIQUE(project_id,dedup_key)）、preference_hypotheses（scope/status/confidence/纠正/停用） |
| `between-frames/src-tauri/src/agent/memory.rs` | 证据记录（六类人工命令）、确定性 2-gram 假设物化、纠正/停用、检索视图与投影 |
| `between-frames/src-tauri/src/command_handler.rs` | 生效人工命令路径挂钩 record_from_command（幂等回放/撤销天然不增） |
| `between-frames/src-tauri/src/agent/tools.rs` | retrieve_preferences 升级为证据支持检索（假设+证据 ID、排序、限量） |
| `between-frames/src-tauri/src/agent/context.rs` | human_evidence 层注入限量假设摘要 |
| `between-frames/src-tauri/src/commands.rs`、`lib.rs`、`bin/m1-harness.rs` | get_memory_projection / correct_hypothesis / set_hypothesis_enabled 三命令注册与 harness 分支 |
| `between-frames/src-tauri/tests/agent_memory.rs` | 去重与确定性、AG-11 纠正不复活、Agent 草案零证据、检索与白名单拒绝、撤销不影响证据 |
| `between-frames/src/domain/memory.ts`、`services/TauriMemoryRepository.ts` | 记忆类型、zod 校验与 IPC 封装 |
| `between-frames/src/domain/models.ts`、`services/TauriRepository.ts`、`app/AppController.tsx` | 桌面仓库挂载 memory、面板注入 repository |
| `between-frames/src/features/preference-memory/PreferencePanel.tsx` | Memory Studio：证据时间线、假设卡（证据链接/纠正/停用恢复）、边界文案 |
