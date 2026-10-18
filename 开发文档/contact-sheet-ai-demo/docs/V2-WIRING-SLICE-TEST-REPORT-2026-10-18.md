# V2 接线级缺口收口：快速开始 / 序列批注 / 命令面板 / 操作历史 / 网格分组 / 免 Prompt 动作（2026-10-18）

基准：`fa0d2b7` + 未提交工作树（含并行的界面文案清理与 magic-login 半成品，不属于本切片）。本切片对应《M0-M5-CODE-AUDIT-2026-10-18》第 6、21、3、4、2、7 行的编码缺口；只收编码项，不含外部验收。

## 第二批（同日下午）：evaluate_draft 真实评估 / 最小权限账户 / 跨实例 Presence

对应审计第 13、17、22 行。验证：Rust 全量 **196/196**（新增 agent_evaluate 3 项；clippy -D warnings 与 fmt 干净）、API 集成 **33/33**（新增跨实例 Presence 与最小权限回归 3 项）、前端 Vitest **231/231**、compose 渲染核对通过。

1. **M3#13 evaluate_draft 结构化本地评估**（`src-tauri/src/agent/tools.rs`）：
   - `identical_groups`：草案内 content_sha256 完全重复（同项目受 UNIQUE 约束常态为空，保留作历史异常数据防御层）；
   - `similar_adjacent`：按草案顺序相邻两张的观察缓存 `visible_facts` Jaccard ≥ 0.6，附共享事实（photo_hash 对齐、正文 ≤64KiB，旧哈希观察不计入）；
   - `human_hint_groups`：保留"重复组:"人工备注通道；
   - `metrics`（观察覆盖/评分/状态分布）与 `item_facts`（≤12 条有界事实）；`mode=structured_local_evaluation`，uncertainty 明示"叙事判断由策展模型结合证据进行，人工评审保留最终决定"。
   - Mock 循环（provider.rs）改为合并 identical/human_hint/similar 三类信号驱动修订；execution 摘要同步。
2. **M4#17 最小权限账户接线**：
   - 删除 0007 占位符模板；新增 `ops/shared/postgres-init/01-create-roles.sh`（首次初始化幂等创建 `focalroom_migrator`/`focalroom_app`，密码来自部署环境变量）；
   - 新增 `0009_min_privilege_grants.sql`（每次迁移后幂等重放授权，角色缺失静默跳过）；
   - compose：postgres 挂载 init 脚本并注入角色密码；api 运行时连接改为 `focalroom_app`（仅 DML），新增 `FOCALROOM_MIGRATOR_DATABASE_URL`；
   - `server.ts --migrate` 优先使用迁移角色连接，未配置时明示回退；README 同步。
   - 回归测试：migrator 执行迁移（含版本表）、app 角色 DML 全通过、CREATE TABLE/DROP/ALTER/CREATE ROLE/ALTER ROLE SUPERUSER 全部 42501 拒绝。
3. **M5#22 跨实例 Presence**：
   - 新增 `0010_presence_bus.sql`（fr_presence 总线表；登记顺序置于 0009 之前使授权重放覆盖新表）；
   - `realtime.ts`：每 tick 整批重写本实例心跳（事务内 DELETE+INSERT）、广播改为读取全部实例 ≤4s 新鲜行、断开即删本实例行、崩溃行按 15s TTL 低频清理；
   - 集成测试：两个 API 实例共享 PostgreSQL，实例一/二互相可见 presence 与浏览照片，断开后从广播消失；
   - Web：Presence 显示协作者正在查看的文件名，可一键隐藏/显示（localStorage 记忆），重复用户去重。

## 第一批（同日）：见下

## 改了什么

1. **M3#6 快速开始入口接通**：ProjectList（本地、非归档）显示「快速开始」；AppController 传入 `onQuickStart` 打开 `quickStart` 弹窗；创建成功后自动打开 AI 面板（预填目标），普通新建项目时清除预填防止串扰。四语词条已存在，未新增。
2. **M5#21 序列 item / version 批注**：
   - 契约 `packages/contracts/src/schema.ts` anchor 增加 `revision` 类型（仅 `revision_id`）；重新生成 `command-envelope.schema.json`，`contracts:check` 通过。
   - `apps/api/src/authority.ts`：`revision` 锚点校验版本存在；`sequence` 锚点校验不变。
   - Web：序列条目「批注」按钮（含未解决批注数徽标）、「批注当前版本 vN」按钮、目标提示与「改回照片批注」、批注面板「序列与版本批注」分区（带锚点标签），发布后自动回到照片目标；`shared-api.ts` anchor schema 补 `sequence_item_id`/`revision_id`。
3. **M1#3 快捷键配置 + ⌘K**：
   - `src/app/shortcutConfig.ts`：20 个可配置动作、默认键位表、localStorage 覆盖（合法性与唯一性校验、损坏回退）、`eventToShortcut` 规范化。
   - `useShortcuts` 改为查表驱动；⌘K 在任何场景（含弹窗/输入焦点）切换命令面板；方向键行为保持固定。
   - `CommandPalette`：过滤、↑↓、Enter 执行；`ShortcutEditor` 嵌入帮助弹窗：捕获式改键、冲突不落盘、恢复默认。
4. **M1#4 操作历史**：`PreviewEvent` 增加 `actor`（user/system）并贯通 Tauri/Mock 仓库；`ActivityDialog` 按操作者（人工/系统/Agent 草案=draft_ 前缀）、类型、版本恢复点三维筛选；版本筛选=该版本保存以来的操作；恢复点徽标。入口：工具栏「操作历史」+ 命令面板。
5. **M1#2 网格分组 + 复制序列方案**：`groupPhotos` 纯函数（状态/评分/文件名前缀，前缀剥离结尾序号）；Workspace 分组下拉与组头分节渲染（不分组时保持原虚拟网格路径）；序列视图「复制序列方案」一键保存带时间戳的方案副本（走既有 saveVersion），命令面板同步收录。
6. **M3#7 免 Prompt 上下文动作**：选区工具条新增「AI 比较叙事价值」（恰选 2 张）与「生成两版顺序」（≥2 张）；AIPanel 支持 `quickAction`（自动携带 photo_id 直接发起 StartTask；视觉预算开启且未授权时先弹授权卡，确认后按快捷参数发任务，授权语义不变）。
7. **E2E 修复（并行会话文案/language 改动导致的既有失败）**：gui-v1/workspace/history-v1 固定 `locale: 'zh-CN'`；新建项目表单改用 `#project-title`/`#project-brief` 定位（ProjectDialog 已被重构为模板选择）；示例项目断言兼容中文。

## 验证

- 前端 Vitest：**229/229 通过**（27→32 文件；新增 QuickStartModal、ProjectList 入口、ProjectReview 序列/版本批注、shortcutConfig、CommandPalette、ActivityDialog、grouping、AIPanel quickAction 等用例）。
- API 集成（隔离 PostgreSQL runner，`LC_ALL=C`）：**30/30 通过**，新增序列/版本锚点成功与 400 边界（错 item、错 revision）。
- 契约：`contracts:generate` 后 `contracts:check` 全绿（9 commands 映射一致）。
- Preview E2E：**16/16 通过**（修复前 7 过 9 败；失败均为 locale/重构后的选择器失配）。
- 类型/Lint：本切片文件 0 错误。当前全仓 typecheck/lint 的报错全部来自并行会话未完成的 magic-login 文件（`apps/api/src/{server,service,email,magic-login}.ts`、`apps/web/src/SharedApp.tsx`），待该切片完成后自然消失，不计入本切片回归。

## 已知边界

- 分组模式下每组内网格未启用虚拟化（组内 <100 张直接渲染；超大单组可切回不分组使用虚拟网格）。
- 免 Prompt 动作不绕过云端图像授权；授权卡语义与手动发起完全一致。
- 本轮未跑 Rust 桌面测试、原生打包与 integration E2E（`--target-dir target-v1` 重建较重）；前端契约生成物已同步。未执行 commit/push。
- 外部验收项（真人评测、真实桶、生产部署、签名公证、远端 CI）不在本切片内。
