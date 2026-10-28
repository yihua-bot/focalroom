# FocalRoom v4.0｜跨端架构、同步与 Agent 工程契约（实施草案）

**级别**：DESIGN SPEC / NOT DEPLOYED。示例字段与端点为目标协议，实施前须经类型生成、迁移评审及兼容测试。现有生产 `/api/v2` 与本协议不应混用。

## A. 业务领域不变量

1. 一个正式共享项目在任一时刻只有一个写入权威。`LOCAL_ONLY`、`SYNC_ENROLLING`、`PRIVATE_SYNCED`、`SHARED` 转换必须具可恢复锁与不可变回执。
2. `SourceAsset.id` 是业务身份，内容 SHA 是验证/去重线索；`CanvasNode.id` 是实例身份；同源照片出现在两张画布上仍是两个独立节点。
3. `Rendition` 不可变，每次 Lr/C1 输出生成新版本。显示设置改变只能改变节点引用或当前视图选择，不覆盖旧像素。
4. 所有正式 Board/Sequence/Publication/ACL 更改都经过服务端权威权限校验及乐观版本控制，AI 草案默认不可直接改正式画布。
5. `commit`、`revert`、`transfer`、`billing` 请求的幂等键与请求 digest 必须同时参与唯一性判断。相同 key 不同 body → 409。
6. 同步的 WS 是通知，不是唯一事实日志。事实来自带游标可补读的持久事件/快照。
7. 用户请求删除不等于已下载到成员设备上的复制本自动抹除；必须在权限和隐私政策中明确。
8. 被撤权/未授权外传的图像不能因 Agent 需要观察而隐式发送到云模型。图像可由在线本地 Agent 执行器在明确授权下查看。
9. 所有图片导出保留可追溯的 Source/Rendition/ICC/RenderRecipe；正式印刷不允许引用低清代理。
10. 所有 Review 结果均带草案与源版本；源变化后必须重算，不允许悄悄覆盖当前画布。

## B. 项目迁移状态机

```text
LOCAL_ONLY
    |
    | explicit user approval & manifest
    v
SYNC_ENROLLING --stage/cloud verify--> PRIVATE_SYNCED --share grants--> SHARED
    |                                  ^                         |
    +--failure--> ENROLL_FAILED -------+                         +--revoke share--> PRIVATE_SYNCED

LOCAL_ONLY 内仅 Mac 权威；SYNC_ENROLLING 冻结冲突写入路径；
PRIVATE_SYNCED / SHARED 内云端正式权威，设备只能本地缓存/草稿；
失败恢复按已有 transferId 从确认点续跑，不能重建不同的“同一个云项目”。
```

### ProjectMode 数据契约

```json
{
  "projectId": "p_...",
  "mode": "PRIVATE_SYNCED",
  "authority": "cloud",
  "schemaVersion": 3,
  "formalRevision": 42,
  "syncEnrollmentId": "UUID",
  "sourceDeviceId": "DEVICE_MAC",
  "cloudAuthorityConfirmedAt": "2026-10-27T00:00:00Z",
  "mediaPolicy": { "uploadOriginals": false, "previewScope": "authorized-renditions" }
}
```

## C. 初始化同步与增量同步

### GET /api/v3/projects/:id/sync/bootstrap

响应包含权限快照、正式版本、资产映射、媒体 manifest、event cursor、schema version、可理解能力集合：

```json
{
  "projectId":"p1", "mode":"PRIVATE_SYNCED", "authority":"cloud",
  "schemaVersion":3, "formalRevision":42, "eventCursor":"98420",
  "snapshotDigest":"sha256:...", "snapshotUrl":"short-lived-ticket",
  "assetManifestUrl":"short-lived-ticket",
  "acl":{"role":"EDITOR", "canEditFormal":true, "canApprove":false},
  "requiredClientCapabilities":["canvas-v3","media-icc-v2","editorial-merge-v2"]
}
```

### GET /api/v3/projects/:id/sync/events?after=cursor&limit=500

```json
{
  "from":"98420", "next":"98423", "hasMore":false,
  "events":[
    {"eventSeq":"98421","kind":"BOARD_COMMAND_COMMITTED","entityId":"b1","revision":43,"commandId":"uuid1"},
    {"eventSeq":"98422","kind":"COMMENT_ADDED","entityId":"thread-18","revision":43,"commandId":"uuid2"},
    {"eventSeq":"98423","kind":"ACL_CHANGED","entityId":"p1","revision":43,"commandId":"uuid3"}
  ]
}
```

**游标使用规则**：客户端成功事务应用事件与本地游标后才推进 cursor；若事件缺失/快照 SHA 不匹配/客户端不识别新操作→停止正式写入并拉权威 snapshot。WS 消息只推送 `projectId,eventSeq,kind`，不承载全量图像或复杂业务状态。

### POST /api/v3/projects/:id/commands

```json
{
  "schemaVersion":3,
  "commandId":"UUID", "idempotencyKey":"deviceA:unique-1",
  "projectId":"p1", "entityType":"BOARD", "entityId":"b1",
  "expectedRevision":42,
  "actorContext":{"deviceId":"d1", "clientVersion":"2.0.0"},
  "operations":[{"opId":"UUID","type":"MoveNodes","payload":{"nodeIds":["n1"],"dx":14,"dy":-8}}]
}
```

响应：`commandId, applied, actualRevision, committedEventSeq, effectiveOperationIds, receiptDigest`。`409 REVISION_CONFLICT` 附当前版本和 `resyncRequired`；`409 IDEMPOTENCY_CONFLICT` 同 key 不同 body；`403 PERMISSION_DENIED`；`422 INVALID_OPERATION`；`423 AUTHORITY_TRANSITIONING`；`503 RETRYABLE_UNAVAILABLE`。未知回执必须走 `GET /commands/:id/receipt`。

### Mobile Outbox 数据结构

```text
OutboxOperation(
   localId, projectId, entityId,
   idempotencyKey, commandJson, commandDigest,
   expectedRevision, createdAt,
   state[LOCAL_DRAFT,QUEUED,IN_FLIGHT,UNKNOWN,ACKED,CONFLICTED,REJECTED],
   lastError, retryAfter, committedReceipt
)
```

- 对正式更改展示“等待同步/冲突”标签；退出 App 不丢队列。
- 同项目同实体的依赖操作按序回执，不得仅靠并发 HTTP 到达顺序。
- 应用重启后 UNKNOWN 首先查询回执，再判断是否重发原 key。
- 评论追加允许按 thread 独立并行，正式画布操作需按 revision 串行。
- 撤权后停止发送，对已保存的本机私有草稿提供导出或放弃。

## D. 媒体派生 / 渲染描述

```json
{
  "sourceAssetId":"asset-1", "sourceSha256":"64-lower-hex",
  "variantId":"variant-1", "renditionId":"rendition-9",
  "renditionSha256":"64-lower-hex",
  "pipeline":"CREATIVE_DISPLAY",
  "width":4200, "height":2800, "bitDepth":16,
  "sourceProfileDigest":"sha256:...", "outputColorSpace":"DisplayP3",
  "outputProfileDigest":"sha256:...", "orientation":1,
  "rendererVersion":"color-engine-2", "renderRecipeDigest":"sha256:...",
  "download":{"type":"ticket", "expiresAt":"...", "tileLevel":0}
}
```

允许的 `pipeline`：`THUMBNAIL`, `AI_PREVIEW`, `CREATIVE_DISPLAY`, `PRINT_MASTER`。`PRINT_MASTER` 不应通过普通私人预览 endpoint 颁发；其授权、存储和渲染引擎单独控制。

**导入限制必须与支持矩阵一致**：移动端 iOS/Android 文件系统与相册返回的资源可能是 HEIC/RAW/云照片代理；需要校验真实格式、尺寸、方向、ICC、可读性及资源大小。不能把 URI 字符串当作可长期复用的本地绝对路径。

## E. AI 会话、任务与权限

### Conversation

```json
{
  "conversationId":"uuid", "projectId":"p1", "title":"编辑中段节奏",
  "lastMessageSeq":45,
  "contextBindings":{"boardId":"b1", "boardRevision":42, "selectedAssetIds":["a1","a2"]},
  "memoryPolicy":"project-only", "modelPolicy":"automatic-with-audit"
}
```

### TaskRun

```json
{
  "taskId":"uuid", "conversationId":"uuid", "projectId":"p1",
  "state":"REQUIRES_APPROVAL", "taskType":"EDITORIAL_CANVAS",
  "scope":{"assetIds":["a1","a2"],"boardId":"b1","baseRevision":42},
  "consent":{"cloudImageUpload":false, "allowedLocalExecutor":"d-mac-1"},
  "evidenceRefs":["obs1","obs2"], "draftBoardId":"b-draft-1",
  "usage":{"inputTokens":0,"outputTokens":0,"estimatedCost":null}
}
```

AI Runtime 状态：`QUEUED,PLANNING,WAITING_FOR_DEVICE,WAITING_FOR_CONSENT,OBSERVING,EXECUTING,REQUIRES_APPROVAL,COMPLETED,FAILED,CANCELLED,UNKNOWN_RECONCILE`。对每一步显示真实工具名（可人性化文案）、时间、对象、输入/输出摘要和授权情况。图像处理任务如不可访问源像素，必须请求授权或说明不能观察，而非编造观察结果。

**权限检测序列**：User identity → Project ACL → per-task granted photo IDs → consent and media classification → provider policy → tool scope → immutable tool receipt → optional human approval. 云端拥有用户会话不等于拥有本地 Mac 媒体权限。

### AI 采纳链路

1. Agent 创建不可直接修改源画布的独立 DraftBoard；带源 Board revision、来源节点映射和 Tool/evidence refs。
2. 审稿者获取 `EditorialChange` 列表，选择 changeIds；系统提供依赖闭包。
3. `simulateMerge` 使用 `(base, sourceCurrent, draftCurrent)` 返回精确 merged snapshot 与冲突。
4. `applyMerge` 复检项目 ACL、草案版本、源版本与 simulation hash，在一个正式事务里写入画布/历史/逆操作/审批回执。
5. `revertMerge` 使用补偿操作并检测后续人工更改，不能直接恢复旧快照抹去他人劳动。

## F. 客户端架构接口适配

### Swift (概念接口)

```swift
protocol ProjectRepository {
    func bootstrap(projectId: String) async throws -> ProjectSnapshot
    func observe(projectId: String) -> AsyncStream<ProjectEvent>
    func submit(_ command: FormalCommand) async throws -> CommandReceipt
    func requestPreview(_ renditionId: String, intent: DisplayIntent) async throws -> PreviewHandle
}

protocol FocalAgentRepository {
    func send(_ message: AgentMessage, context: ContextBinding) async throws -> TaskHandle
    func events(taskId: String) -> AsyncStream<AgentEvent>
}
```

### Kotlin (概念接口)

```kotlin
interface ProjectRepository {
    suspend fun bootstrap(projectId: String): ProjectSnapshot
    fun observe(projectId: String): Flow<ProjectEvent>
    suspend fun submit(command: FormalCommand): CommandReceipt
    suspend fun requestPreview(renditionId: String, intent: DisplayIntent): PreviewHandle
}
```

两端界面不应自己写 `boardRevision++`、节点合并或权限“乐观批准”逻辑；共享领域层验证执行前/执行后快照，云端仍是唯一正式回执方。

## G. 需优先建立的基础契约测试

- Swift/Kotlin/TypeScript/Rust 使用同一组 `CommandEnvelope` fixture 序列化/反序列化；大小写、空值、未知枚举、版本不一致均有负例。
- 同一 1000 条随机节点更改，Mac / iOS / Android 的纯领域回放与服务端预期完全一致。
- v2 客户端提交 v3 不认识的操作：明确拒绝/只读而非丢字段。
- 同一 idempotencyKey + 同一 body 返回同一 receipt；相同 key + 不同 body 返回冲突。
- 并发两个同 revision 的正式修改，只能由规则允许的一个成功，其余进入冲突解决。
- 断电重启时本地 outbox 事务不会在 `ACKED` 但 cursor 未更新的状态丢失事件。
- 图片授权/限额/设备撤权/跨项目 assetIds 恶意输入全部拒绝。
- 所有媒体派生文件都能追溯 source sha、rendition sha、ICC 和 pipeline 版本；错误格式被拒绝。
- 未完整确认远端权威状态的 Mac 不得自动进入共享正式写入；冻结态须有恢复路径。

## H. 本规范的代码映射与限制

- 现有真实云端入口：`apps/api/src/server.ts`、`authority.ts`、`realtime.ts`、`boards.ts`、`storage.ts`。
- 现有桌面桥接：`src-tauri/src/v2_bridge.rs`、`agent/creative.rs`、`canvas_review.rs`。
- 现有 Color Engine：`src-tauri/src/color_engine.rs`，仍需实际产品接入与真机验证。
- 现有目标复用的前端 UI：`src/features/canvas/EditorialReviewWorkspace.tsx`（审稿语义），而不是复制 HTML 到原生手机。
- 本文 `api/v3` **没有声明已上线或已通过生产测试**。开发顺序应先生成/验证契约，再迁移数据库和实现界面。
