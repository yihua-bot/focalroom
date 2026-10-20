# v3 迁移设计 · G0 评审稿

日期：2026-10-20。来源：工程宝 v3 主方案、ADR-050–056、当前 HEAD `38e9704` 与工作区。**本稿未新增或执行 v3 schema，G1/G2 实施前需技术评审。**

## 已核实的旧数据

- 本地 `src-tauri/src/db.rs` 使用 SQLx migration registry，升级前 `VACUUM INTO` 并校验备份和归属；原始 `photos`、决策、序列、版本、评论、审计与幂等回执必须保留。
- 云端 `apps/api/src/database.ts` 明确登记 11 个迁移，空目标实测 31 张表。SQL 目录另有未登记设计稿，禁止全目录遍历执行。
- 云端共享正式投影在 `fr_projects.snapshot`，版本在 `fr_versions`；不应假设存在与 SQLite 完全同形的 Photo 表。
- 当前 `photos.content_sha256` 和 `original_path` 是旧身份线索；320/1400px 缓存不是新的源资产或印刷 Rendition。
- G0 使用生成 JPEG + v11 SQLite 夹具，包含照片、备注/评分、序列、命名版本、评论。只升级快照副本；源图、快照、升级前备份 SHA256 与恢复结果见 `../evidence/v3-g0-20261020/legacy-snapshot.json`。用户实际数据库未打开，真实用户资料副本兼容性仍待专门授权和验证。

## G1：只增加身份层，先不切换正式写流

1. 新增 SourceAsset、Locator、EditVariant、不可变 Rendition、Representation、ExternalEditLink 与 TransferJob 的独立领域契约；SQLite/PostgreSQL 各自实现同一语义，不能直接执行工程包示例 DDL。
2. 旧 `photo_id` 通过包含权威来源与 project scope 的 alias 映射到 `asset_id`；共享迁移后的 ID 映射显式保存，不按文件名、导入顺序或全局 hash 猜测归属。
3. backfill 只从验证过的旧记录建立映射，记录版本、游标和计数；ICC、位深或外部软件 ID 缺失时保存 unknown，不从 JPEG 预览伪造源信息。同哈希不同位置为可独立验证 Locator，合并须明确。
4. 新读适配器先双读校验，覆盖旧项目、序列、版本、评论、草案、记忆、共享 Promotion、历史回执；未校验一致前保持旧正式写入口。
5. 写入切换需要独立门槛：幂等、revision、ACL、project scope、审计、Undo/Redo、部分失败恢复，以及 500 张源文件 SHA256 不变。Agent 写入仍仅生成 Draft。
6. Board、Sequence 与 Publication 各自保存引用实例；G1 可提供引用基础，G3 才实现自由坐标交互，不能把当前 SequenceBoard 当作自由画布。

## G2：先做色彩原型，再选择渲染实现

- 先比较 ImageIO/CoreImage/ColorSync 与 NSView/Metal 集成，冻结源 ICC、目标 profile、变换参数、位深和参考样本，测数值、窗口合成、输入与可访问性。
- 人类显示、AI 预览、缩略图与印刷母版分管线；保留当前预览作兼容，不能给它贴 Pro Color / print master 标签。
- ICC 缺失/无效、RAW embedded/derived、设备未校准均明确表示；校准仪器、多屏、100MP/巨幅 TIFF 的验收列为 DEVICE 门槛。
- API 返回 quality provenance、实际像素源与处理版本；缓存键绑定源 hash、rendition、ICC、decoder、目标显示、proof 与 HDR 模式。

## 各阶段的回退条件

| 阶段 | 可用回退 | 禁止动作 |
|---|---|---|
| Expand | 保留旧读写；新表只在隔离夹具验收 | 改写历史 migration 或覆盖用户库 |
| Backfill / dual-read | 停止 backfill，保留游标和旧记录；重试须幂等 | 删除旧 Photo/评论/版本以掩盖差异 |
| Switch-write | 停止新写入，校验备份及新操作；设计兼容读取/补偿后回退 | 直接降级旧二进制忽略新命令与版本 |
| Contract | 仅在兼容窗口结束、恢复演练通过并批准后移除旧结构 | 本轮清理旧表、历史和审计 |

G0 的“回退”实测是将升级前备份复制到**另一个临时数据库**，只读检查旧迁移与数据，再升级并比较投影；不证明生产 v3 回退已通过。PostgreSQL 生产灾备和对象存储恢复继续保留独立验收。

## G1/G2 评审需确定的事项

- ID/alias 是否按 local/shared authority 分区；旧字符串 ID 与新 UUID 的兼容策略。
- Rendition 不可变性及当前版本指针；共享后访问控制和引用权限。
- 原图 Locator/security bookmark 与源变更检测策略。
- v3 命令 envelope、旧客户端能力协商、schema 与错误码。
- 色彩样本/设备/容差预登记，以及 Native Viewer 与 Web 代理的明确能力边界。

本轮没有运行 G1+ 的 schema 或外部软件集成，也没有修改生产部署。

## 工程包契约中的待补约束（设计稿，不是已上线缺陷）

实际读取 `DOMAIN-SCHEMA.sql` 与 `api-openapi.yaml` 后，G1 评审还需补齐：

- `fr3_board_nodes.parent_id` 只引用节点 ID，没有限制同 Board，也未阻止父子循环；照片资产的 project scope 也不能仅靠单列 FK 保证。
- `fr3_sequence_items` 的 asset/rendition 是两个独立 FK，尚未保证 rendition 属于该 asset。
- Representation 唯一键包含可空的 target profile；PostgreSQL 普通 UNIQUE 对 NULL 的语义不能直接保证未知 profile 的幂等。
- OpenAPI 的 CommandEnvelope 定义 schemaVersion，但未将其放入 required；Canvas envelope 则要求该字段，需统一兼容协议。
- Canvas schema 当前仅提供放图、移动、缩放、笔迹、删除五种操作样例，尚不是涵盖分组、评论和全部正式命令的完整契约。

这些问题应在新迁移/命令实现前解决，不通过直接运行目标 SQL 来发现于用户资料中。
