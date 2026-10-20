# FocalRoom v3 迁移计划（G0 产出）

日期：2026-10-20。基线 HEAD `8ac1a21`，CI 全绿。本文档为 G1+ 迁移的设计约束，不改既有 migration。

## 现有数据库身份

### 桌面 SQLite（19 个迁移，51 张表）

- `photos` 表是唯一资产身份（`id`, `project_id`, `content_sha256`, `original_path`, `filename`, `file_size`, `width`, `height`, `thumbnail_path`, `preview_path`）。
- `photo_decisions` 保存 status/rating/note/book_mark。
- `sequence_items` 引用 photo_id 排序。
- `agent_steps` / `drafts` / `preference_evidence` 引用 photo_id。
- **没有** Variant、Rendition、Board、CanvasNode、Publication 实体。

### 云端 PostgreSQL（11 个已登记迁移，31 张表）

- `fr_projects` / `fr_previews` / `fr_proposals` / `fr_comments` 为共享权威。
- **没有** 资产/变体/渲染分离，也没有画布/出版实体。

## v3 目标身份（DOMAIN-SCHEMA.sql 核心差异）

| v3 实体 | 现有对应 | 差距 |
|---|---|---|
| `SourceAsset` | `photos` | 需拆分：母版只读、哈希不变、多 Locator |
| `AssetRepresentation` | `thumbnail_path`/`preview_path` 两列 | 需独立表：类型/算法/profile/尺寸 |
| `EditVariant` | 无 | 新增：外部编辑软件绑定、变体身份 |
| `Rendition` | 无 | 新增：不可变渲染文件、SHA-256、位深/ICC/来源 |
| `Board` / `CanvasNode` | 无 | 新增：无限画布、多类型节点 |
| `Sequence` | `sequence_items`（项目内一维） | 升级：独立实体、章节/占位/多方案 |
| `AnnotationThread` | `fr_comments`（云端） | 升级：多锚类型、本地也要有 |
| `Publication` | 无 | 新增：书籍/展览/作品集 |
| `ReviewProposal` | `fr_proposals`（云端） | 升级：Board/画布操作、本地也要有 |
| `ProjectRevision` | `projects.revision` + `command_events` | 升级：正式/分支/冻结分层 |
| `ExternalEditLink` | 无 | 新增：Lightroom/C1 绑定 |
| `TransferJob` | 无 | 新增：外发/回流批次 |

## 迁移策略

### 原则

1. **expand → backfill → dual-read/validate → switch-write → contract**。
2. 旧 `photo_id` 不删除；建 `asset_alias(photo_id TEXT, asset_id TEXT)` 映射表，逐步把引用改走 asset_id。
3. 所有新 migration 前缀从 `0020_` 开始（桌面）/ `0017_` 开始（云端），不碰已登记文件。
4. 先在隔离 fixture 数据库验证，再对生产副本演练；**旧用户库迁移前必须做只读快照备份**。

### G1 第一批（桌面 SQLite）

```
0020_asset_domain.sql
  -- source_assets（从 photos 提取：id 原用 photo id、content_sha256、
  --   original_path→locators JSON、file_size、width、height、format、
  --   source_profile_digest 可空）
  -- asset_aliases(photo_id, asset_id) 映射表
  -- asset_representations（asset_id、kind、algorithm、width、height、
  --   profile_digest、file_path、file_size）
  -- edit_variants（asset_id、name、external_software、external_variant_id、
  --   is_current、created_at）
  -- renditions（variant_id、sha256、width、height、bit_depth、icc_digest、
  --   source_type、created_at、file_path）
  -- 回填：每条 photos → source_assets + alias + 2 条 representation
  --   （thumbnail/display_proxy）
```

### G1 第二批（云端 PG）

```
0017_cloud_asset_domain.sql
  -- fr_source_assets / fr_asset_aliases / fr_renditions
  -- fr_edit_variants
  -- 回填从 fr_previews 提取 object_key→rendition
```

### G1 第三批（代码层重路由）

- `image_pipeline::prepare()` 写入 `asset_representations` 而非 photos 两列。
- `command_handler` 新增 `CreateVariant`/`RecordRendition`/`UpdateAssetLocator` 命令。
- `agent tools` 的 photo_id 参数统一走 `asset_resolver`（内部先查 alias）。
- 导出/预览读取走 `representations` 按需选择。

## 回滚

- expand 阶段：新增表可随时 DROP，不影响旧路径。
- switch-write 后：保留旧列写入（dual-write）至少一个版本，验证后移除。
- 用户库不可回退到无 alias 状态；快照备份是最后防线。

## 风险

1. `photo_id` 引用散布在 agent_steps/drafts/preference_evidence/command_events 中——alias 表必须先于任何 switch-write 上线。
2. 现有 500 图性能基线不能退化——representation 查询需要索引覆盖。
3. 云端 `fr_previews.object_key` 被 Web 票据引用——rendition 迁移不能破坏现有预览下载。
