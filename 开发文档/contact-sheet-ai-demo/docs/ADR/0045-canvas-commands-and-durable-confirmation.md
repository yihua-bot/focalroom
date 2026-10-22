# ADR 0045：画布正式命令、历史组与中断确认

日期：2026-10-22。范围为本地画布的 H0-07/G3 可靠性，以及 H0-03 的整张任务画布采纳入口；不代表 Shared 共编或完整出版版本已完成。

## 决策

正式桌面写入统一为 `canvas_execute`。请求固定 `schemaVersion=3`、`commandId`、`idempotencyKey`、`projectId`、`boardId`、`baseRevision`、带唯一 `opId` 的 operations；身份不从请求反序列化，由可信 Rust `CommandContext` 注入。旧无版本的 mutation IPC 不再注册，保留的 Store 兼容方法仅供内部和测试，正式 GUI 不使用。

目标包的 Canvas JSON Schema 是设计样例，本实现保留其版本/范围/幂等/操作身份语义，并采用 `action: {type,data}` 复用现有领域 DTO，覆盖创建、节点、评论、历史及采纳 15 类操作。创建节点和评论的稳定 ID 使用 opId；创建 Board 使用请求 boardId。当前 PlacePhoto 仍绑定 SourceAsset，未据此宣称完成 Rendition 选择功能。

共享 TypeScript 契约在 `packages/contracts/src/canvas.ts`，Rust 类型在 `canvas_commands.rs`；两端共用 `contracts/canvas-command-examples.json` 的 15 类样例进行精确往返及未知字段拒绝测试。创建、撤销、重做、采纳须单独提交，其余命令每次最多 300 项、序列化请求最多 2 MiB；现有单项节点/笔迹限制继续生效。

`BEGIN IMMEDIATE` 中依次完成身份、完整请求幂等匹配、项目存续/本地写权威、Board 归属/冻结/任务归属、baseRevision 校验、所有领域操作、历史组、版本及不可变审计回执。失败全部回滚；同 ID 或键改内容、同 opId 跨请求复用均拒绝。回放仅返回当时保存的回执，不能用当前投影替换，也不再次修改历史或偏好证据。

同一个语义命令只推进一次 Board revision；旧节点触发器继续兼容旧内部写入，正式事务内将中间计数合并为一次推进。评论也推进 Board revision。未变化的 Resize/Lock、无可用历史的 Undo/Redo 保存无变化回执。画布版本与项目选片版本分开，画布审计不伪造审美偏好证据。

批量操作的历史行共享 command_group，Undo 逆序恢复整组、Redo 正序重放整组；任一锁定或冲突使整笔恢复失败。NULL group 的旧记录仍逐项处理，旧插入顺序、undone/redone 状态保留。新增操作作废旧 redo 分支但不删除旧记录。评论通过 visible 和前后 resolved 状态恢复，不删除审计。采纳是不可逆的人工决定；其正式回执持久化，任务须属于本项目、基线仍有效且处于 review_ready/completed，Board 须为任务 draft。采纳后进入 official，Agent 不再拥有写入权。

审计 `canvas_command_events`、操作身份 `canvas_operation_ids` 只追加，由数据库触发器拒绝 UPDATE/DELETE。可更新的私有 `canvas_op_log` 与不可变审计分开，不修改旧 command_events。新迁移 0029 保留旧撤销 rowid 与状态、旧评论默认可见；仅隔离库验证。升级沿用 Store 的自动备份及完整性检查，回退必须恢复相应旧库备份，不让旧程序直接打开新库。

## 客户端恢复

界面读取 Board/节点/评论/缓存来源的同事务快照，再使用该版本提交。列表刷新不能悄悄替换已显示节点的编辑基线；发生冲突时刷新界面、保留输入，由用户显式重试，不自动 rebase。缓存版本只按已确认回执单调推进，历史回放不能把当前版本降回旧值。

发送前，客户端将完整请求写入本地 outbox；存储失败不发送。结果不明时保留请求、停止同 Board 新编辑，并显示“确认上次操作”。刷新或重启后用户确认时重发完全相同的请求，让后端幂等识别。只有核对命令/项目/画布/基线/结果数与版本范围的回执，或明确的业务拒绝，才移除 outbox。未知错误、存储故障、响应格式异常不当作失败后免费重做。

## 取舍与余项

复用原有事务内领域操作和一份共享协议，避免另写一套节点 SQL（DRY/SRP）；一个正式入口和一个只读快照入口，未引入通用总线或 CRDT（KISS/YAGNI）。

这不是完整 v3 收口：云端正式画布命令与 ACL/共编、Agent 差异及部分采纳、Publication Edition/布局版本、原生色彩与大图、受控 LR/C1 回流仍需分别完成。此批浏览器旅程连接真实 Rust Store，但摄影师/编辑者是 SIMULATED，不计独立真人或真实设备色彩验收。公证、生产迁移、部署和分发不在本批执行范围。
