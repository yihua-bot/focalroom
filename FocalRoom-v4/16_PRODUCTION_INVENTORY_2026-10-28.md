# FocalRoom 生产只读盘点报告（升级前置）

日期：2026-10-28 晚，Asia/Shanghai。对 `1.13.251.21` 执行三批只读盘点（系统/容器、数据库/迁移、磁盘/配置），未修改任何生产状态。SSH 密码由用户会话提供，未写入任何仓库文件；密钥值未读取，仅记录键名存在性。

## 一、系统与容器

| 项 | 值 |
|---|---|
| 主机 | VM-0-10-ubuntu（Ubuntu 22.04，2C/4G/50G，到期 2027-07-15） |
| 磁盘 | `/` 50G 已用 46G，**仅剩 1.9G（97%）——升级硬风险** |
| FocalRoom 栈 | api `focalroom-api:v1.1.0`（Up 5d）、web `focalroom-web:v1.1.0`（Up 6d）、postgres `postgres:16.14-alpine`（Up 10d，healthy） |
| 镜像 digest | `focalroom-api@sha256:e6943d36…0468ec` |
| Compose | `/opt/focalroom/ops/shared/compose.yaml` + `/opt/focalroom/releases/v1.1.0-20261022/runtime.compose.private.json`，项目名 `focalroom-shared-review` |
| 卷 | `focalroom-shared-review_pg-data`、`focalroom-shared-review_preview-data` |
| 同机其他 | openyc 全家桶 10 容器运行中（api/embedding/im-core/orchestrator/lightfm/edge-tts/redis/falkordb/qwen-proxy/qdrant/postgres）——**升级全程不得触碰** |

## 二、数据库与迁移

- PostgreSQL 16.14（与本地回归版本一致）。
- 线上 `fr_migrations` 登记 16 条：0001, 0002, 0006, 0008, 0009, 0010, 0011, 0012–0015, 0017–0021。
- 最新代码 `database.ts` 白名单与线上的差集（**待执行集合，12 条**）：
  `0022_ai_durable_queue` → `0023_author_memory` → `0024_billing_refunds` → `0025_canvas_commands` → `0026_canvas_events` → `0027_board_presence` → `0028_team_seats` → `0029_research_references` → `0030_device_registry`（iPhone 设备）→ `0031_private_sync` → `0032_account_data_rights`（iPhone 数据权利）→ `0033_audited_private_erasure`。
- 已登记 16 条的 sha256 一致性核对**尚未执行**（升级前必做：不一致则严格校验拒绝启动）。
- 0003–0005/0007 维持历史决策不登记；0009/0010 在线上为白名单语义（presence/agent worker + 最小权限重放），无冲突。

## 三、磁盘构成与清理潜力

`/var` 28G（Docker 为主）+ `/tmp` 7.1G + `/usr` 4.2G + `/root` 3.6G + swap 2.1G。

`docker system df`：镜像 53 个 23.49GB（**可回收 13.79GB**）、容器层 4.23GB（可回收 2.45GB，为 12 个 11 天～3 个月前的 Exited 残留）、构建缓存 476MB。

**清理潜力 ≈ 16.7GB**，清后可用空间约 18.6G，满足备份 + 双镜像 + 迁移余量。清理候选（需批准，逐项执行）：
1. `docker image prune`（仅悬空镜像层，不动任何打标签镜像）
2. `docker container prune`（仅 Exited 容器；openyc 运行容器不受影响）
3. `docker builder prune`（构建缓存 476MB）
4. `/tmp` 7.1G 内容逐项核对后清理（清单另出）

## 四、配置与密钥

`runtime.compose.private.json` 键名（值未读取）：`FOCALROOM_DEEPSEEK_API_KEY`、`FOCALROOM_RESEND_API_KEY`、`FOCALROOM_EMAIL_FROM`、`FOCALROOM_GOOGLE_CLIENT_ID`、`FOCALROOM_AI_INITIAL_TOKENS`、数据库双角色密码、`FOCALROOM_WEB_ORIGIN`、预览目录。**DeepSeek 凭据在生产已配置**（是否有效/启用待登录后验证）。升级沿用该文件，不重写。

`releases/v1.1.0-20261022/` 含 `backup-db.sql`（上一版发布时备份）。`ops/shared/.env` 不存在（密钥在 private.json）。

## 五、升级执行顺序（待批准后执行）

1. 磁盘清理（上述候选，逐项记录前后 `df -h`）
2. 已登记迁移 sha256 全量核对（线上 vs 本地文件，16/16 必须一致）
3. 固定源码快照 → 本地构建 linux/amd64 api/web 镜像（排除密钥/归档/iOS 产物）→ 导入服务器
4. PG custom dump 备份 + 隔离恢复验证
5. 一次性 migrator 执行 0022–0033（最小权限角色不持迁移凭据）
6. 切换镜像至新版本，保留旧镜像与回退路径
7. 上线只读检查（健康/鉴权/devices/events/receipts/私有权限/旧端兼容）
8. AI 真实调用验收单独执行（用户指定测试账号与费用上限后）

回退原则沿用 `IOS_1_0_0_PRODUCTION_UPGRADE_PLAN.md`：保留原容器/卷/旧镜像/备份；不 down/prune 数据资源；新增迁移后旧镜像可能拒绝启动，回退需用兼容镜像。

## 六、本次盘点未做

未执行迁移、未切换镜像、未清理磁盘、未读取密钥值、未动 openyc、未发真实模型请求、未在仓库留存任何凭据。
