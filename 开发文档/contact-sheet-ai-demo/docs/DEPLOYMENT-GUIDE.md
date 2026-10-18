# FocalRoom 生产部署指南

更新：2026-10-18。本文不包含连接密码或服务密钥。生产操作须先确认目标、备份和维护窗口；不能把文档存在当成已经部署成功。

## 当前生产发布

用户明确确认后，API/Web 已于2026-10-18替换为验过的amd64镜像。公网HTML200、新登录页和匿名API401经系统curl及IAB通过；原502已恢复。数据库现登记8项迁移，应用/迁移角色分离、.env0600、PG容器及数据卷保留。**邮件暂不启用，不能实际发送验证码，真实登录旅程尚未验收。** 详情与全部边界见 [生产部署报告](V2-M4-PRODUCTION-DEPLOYMENT-REPORT.md)。

运行镜像为 `focalroom-release-api:otp-20261018-amd64` 与 `focalroom-release-web:otp-20261018-amd64`；基础Compose和旧源码保留，实际发布必须同时使用新增覆盖配置：

```bash
compose=(docker compose --project-name "focalroom-shared-review" --env-file "/opt/focalroom/.env" -f "/opt/focalroom/ops/shared/compose.yaml" -f "/opt/focalroom/releases/otp-20261018-amd64/deployment-override.yaml")
"${compose[@]}" ps
```

备份在 `/opt/focalroom/releases/otp-20261018-amd64/backup-verified`，受控保留，不拷贝环境文件或数据库dump到源码仓库。不以单独旧基础Compose重建生产服务；也不对当前已升级库重做下面的首次角色移交。

## 部署前只读核对结果（历史）

本轮通过已知主机密钥校验的 SSH 成功核对 `root@1.13.251.21`，未修改生产文件、角色、数据库或容器。

| 项目 | 实际结果 |
| --- | --- |
| 部署路径 / 项目 | `/opt/focalroom` / `focalroom-shared-review` |
| 架构 / 数据库 | x86_64 / PostgreSQL 16.14，当前仅 plpgsql 扩展 |
| API | exited，退出码 1；日志含 EACCES，路径为 `/app/apps/api/migrations/0006_rate_limits.sql` |
| Web | running，实际标题仍为「FocalRoom · Web 工作台预览」；宿主源码标题已经是新名称 |
| PostgreSQL | running；只登记 0001、0002；用户 0、项目 0 |
| 数据库角色 | focalroom 管理员和 focalroom_app 已有；focalroom_migrator 不存在，现有表/序列/函数属于 focalroom |
| 配置 | API Origin 为 `https://focroom.cc`，应核对为协作域名；.env 和 API 均未配置邮件 key；.env 当前权限为 0644 |
| 资源 | 只读核对时系统盘剩余约 2.19 GiB，部署前必须重新核算容量 |

详细证据及当时待批准范围见 [生产核对记录](V2-M4-PRODUCTION-PREFLIGHT.md)。该范围现已获批并完成，上表保留部署前历史，不代表当前仍然故障。旧版本在服务器上并不等于 Docker 缓存机制失效；先核对源码、镜像和运行容器 ID。

## 架构与隔离

- `focroom.cc`：Cloudflare Pages 静态官网。
- `team.focroom.cc`：Cloudflare Tunnel → 宿主回环 8084 → Nginx → NestJS API；PostgreSQL 为正式作品权威。
- FocalRoom Compose 项目、网络、数据库和预览卷独立；不操作 openyc 的容器或存储。
- API、PG 不直接暴露公网。当前 Compose Web 映射为 `127.0.0.1:8084:80`；Tunnel 的运行位置必须能访问这个回环监听。

Tunnel 的 Service URL 决定本地连接协议，区域 SSL/TLS 模式不会改变它。不把切换 Flexible 当作修复 API 退出、502 或证书错误的步骤。详见 [Cloudflare 官方说明](https://developers.cloudflare.com/tunnel/troubleshooting/https-origins/)。本轮不修改现有 Tunnel、DNS、TLS 或官网。

## 凭据和源码

SSH 使用管理员在受控渠道提供的主机别名/凭据；首次连接先人工核对主机指纹，已知主机保持校验。不使用 `StrictHostKeyChecking=no`，不把 SSH 密码写入命令、指南、归档或仓库。

原指南曾包含实际 SSH 密码；从当前文件移除不代表 Git 历史、聊天或已分发副本中的凭据已经撤回。管理员应安排轮换；本轮未修改登录凭据或重写 Git 历史。

部署来源须冻结到可核对的源码/镜像摘要，排除 .env、数据库、照片和私钥。Web 需要品牌 PNG 等静态资产，不能排除所有 PNG/JPG；API 镜像还需要 `workers/agent/src`。优先使用已审阅的构建上下文，不复制旧指南的通用打包命令。

## 配置要求

模板见 `between-frames/ops/shared/environment.example`；真实配置留在服务器受控文件中，不粘贴到聊天或提交 Git。

| 变量 | 要求 |
| --- | --- |
| FOCALROOM_DB_PASSWORD | 独立 PG 管理员凭据，只用于初始化/维护 |
| FOCALROOM_APP_PASSWORD | 应用角色凭据，应用只获得 DML |
| FOCALROOM_MIGRATOR_PASSWORD | 迁移角色凭据，用于显式 DDL；仅有变量并不等于角色已创建 |
| FOCALROOM_WEB_ORIGIN | `https://team.focroom.cc`，精确 Origin，不含尾部路径 |
| FOCALROOM_RESEND_API_KEY | 验证码交付需要有效值；本轮未配置，缺失会返回 503 |
| FOCALROOM_EMAIL_FROM | 必须使用邮件服务商已验证发送域，不能任意填 noreply 地址 |
| FOCALROOM_AGENT_MODEL_* | 仅在独立授权的云模型功能上线时配置；本轮不启用 Worker/模型 |

API 的 DATABASE_URL 指向 focalroom_app，MIGRATOR_DATABASE_URL 指向 focalroom_migrator；不可把超级用户连接作为长期应用连接。配置文件权限应收紧到 0600；已有生产文件权限变更包含在部署批准范围内，本轮未执行。

## 既有服务器升级顺序

本节用于当前已有卷的服务器。**不能通过重建 PG 容器重新运行首次初始化脚本，也不能删除卷“从零重来”。**

1. 只读核对目标、所有写入者、磁盘、数据库 owner/role、已登记 SQL SHA；保留旧镜像 ID。当前 0001/0002 的登记 SHA 必须与待部署版本一致。
2. 先在本机完成目标 x86_64 镜像构建与非 root 读取验收，或确认服务器有足够构建空间再构建。检查运行镜像里的源码摘要。不要删除所有镜像或全局 builder prune。
3. 取得明确生产批准后，安排停写和一致性备份，覆盖 PG 与私有预览；现有备份工具只自动暂停 API，若 Worker 或其他写入者在运行，须额外协调。
4. 用管理员在独立 FocalRoom 库建立缺失迁移角色，沿用受控配置中的凭据；将已核对的 FocalRoom 表、关联序列和两个触发器函数的所有权交给迁移角色。应用保持非 owner/非超级用户。不对其他数据库使用 REASSIGN OWNED，不扩大应用角色权限。
5. 显式运行新版本迁移，随后启动新 API/Web。正常服务启动只核对 schema，不自动执行 DDL。
6. 验证服务器回环 HTML/静态资源 200、鉴权接口匿名 401、API 正常运行，再验证公网 HTTPS 新页面和 Cookie；不得仅凭首页 200 宣告登录成功。
7. 真实邮件需用户指定接收邮箱并安全配置服务 key，随后验收验证码、刷新会话和退出。没有配置时明确保留邮件验收待办，不向编造邮箱发送真实邮件。

当前部署的具体边界、备份/角色升级/回退顺序见 [本轮执行方案](V2-M4-OTP-DEPLOYMENT-PLAN.md)。上述操作尚未执行。

## 全新隔离服务器的基本顺序

这与现有服务器升级不同：先创建独立 PG 卷，由 postgres-init 首次建立角色，再运行迁移，最后启动业务服务。以下为批准后使用的命令结构，变量/凭据、源码路径、容量和镜像已预先核对；不用于覆盖现有部署。

```bash
cd "/opt/focalroom"
compose=(docker compose --project-name "focalroom-shared-review" --env-file "/opt/focalroom/.env" -f "/opt/focalroom/ops/shared/compose.yaml")
"${compose[@]}" build api web
"${compose[@]}" up -d postgres
# 等待 PostgreSQL 健康，并确认角色和目标数据库之后：
"${compose[@]}" run --rm --no-deps api node apps/api/src/server.ts --migrate
"${compose[@]}" up -d --no-deps api web
```

Worker 另行审阅与启用，不通过默认 `up` 自动开启模型工作。禁止按 SQL 文件名遍历整个迁移目录。

实际登记的迁移：0001、0002、0006、0008、0010、0011、0009_min_privilege_grants、0012_magic_login。0003–0005、0007 和旧 0009_magic_login 是未登记草案，不能混跑。普通 postgres:16.14-alpine 不因文档声称而具备 pgvector。

## 备份和回退

备份与恢复接口以 [运维说明](../between-frames/ops/shared/README.md) 为准。备份暂停目标 API 并检查 PG/预览一致性；恢复默认仅验证，只有显式批准的不同独立空项目可以写入，不覆盖原库。

`assertSchema` 严格匹配版本数量/SHA。应用 0012 后，旧镜像可能因 schema 版本不匹配而拒启；上线前须准备兼容 schema 的备用镜像，或经批准将备份恢复至独立目标后验证切换，不能简单承诺退回旧 tag。

## 验收与排障

- API 退出：检查受控、脱敏日志中的具体错误；不要输出完整环境变量、Token、请求正文或 .env。
- EACCES：镜像内以 node 用户实际检查文件和所有祖先目录。修 Dockerfile 后构建新镜像，不对服务器源码盲目 chmod -R。
- Web 旧标题：分别核对源码标题、镜像 HTML、运行容器 image ID 和公网缓存。本轮已确认源码新、运行镜像旧。
- schema 不匹配：先核对登记版本与 SHA，不删除迁移记录或直接运行所有 SQL。
- 验证码 503：检查邮件 key 是否安全配置且被 Compose 注入，以及发送域是否验证；不把 503 包装成已发送。
- 502：先确认 API 是否存活和代理上游是否可达，再检查 Tunnel 路由。不得通过降低 TLS 校验掩盖服务退出。

本指南的事实来自本轮只读核对；后续部署必须记录新的镜像、迁移和验收结果，不能复用本文的时间点状态冒充上线证明。
