# 独立共享服务交付与上线核对

这里包含部署草案与本地演练记录，不是腾讯云部署证明。2026-10-18 本机 Docker 29.2.1 可用，已用本地已有的 PostgreSQL 16.13 镜像完成独立容器备份/恢复演练（见下节）。尚未构建或验收本项目 API/Web 业务镜像；不可从 Dockerfile/Compose 推断整栈或生产通过。

## 2026-10-18：磁盘预览备份与空项目恢复

依赖 Bash、Python 3.11+、Docker Compose；不增加 Python 包。脚本从自身目录定位 `compose.yaml`，显式绑定项目、容器标签、实际数据库及预览挂载，不猜卷名；可在任意工作目录执行。也可用 `--compose "/absolute/path/compose.yaml"` 指定已审阅配置。

同一主机、同一用户、同一项目的运维操作使用不阻塞的文件锁，拒绝并发备份/恢复；该锁不能协调其他主机，跨主机维护仍须管理员统一停写。仅支持 macOS/Linux。

只支持本项目独立 PostgreSQL + `/data/previews` 私有磁盘存储。R2、其他写服务、外部数据库或分散的预览挂载会被拒绝，不能混用这个工具假称对象存储已备份。运行配置必须与所选 Compose 一致；操作期间不得有手工 SQL、外部队列或未纳入该栈的写入者。前后表计数核对不能证明不存在同计数的外部并发更新。

### 备份

先安排维护窗口并确认目标项目，再由管理员执行：

```bash
bash "ops/shared/backup.sh" "/secure/focalroom-backups" "focalroom-shared-review"
```

工具暂停当前项目原本运行的 API 副本，以 `pg_dump --format=custom` 和真实预览挂载生成快照；核验数据库引用、全部对象内容 SHA256、迁移和表计数。成功或异常都尝试恢复原运行副本，不启动原先停止的副本、不执行 `compose up`。恢复进程运行不等于业务健康检查，管理员还须检查健康和请求。

校验成功且 API 原状态恢复后，以私有权限和原子重命名发布 `focalroom-backup-*.tar.gz`。其中只有 `manifest.json`、`database.dump`、`previews.tar.gz`。失败不发布完整归档，不自动删除暂存数据。单文件预览最多8MiB、对象最多100000、清单最多32MiB、展开总量最多64GiB；管理员须预留足够暂存和归档空间。

### 验证与恢复

默认只验证归档，不连接 Docker、不停服务、不写数据库：

```bash
bash "ops/shared/restore.sh" "/secure/focalroom-backups/backup.tar.gz"
```

该步验证成员白名单、路径/类型、SHA256、数据库 custom 格式头和全部预览引用；真实 custom dump 的完整解析还会在执行恢复前交给 `pg_restore --list`。旧脚本归档格式不兼容，须独立转换/审阅，不自动猜测。

实际恢复只允许**不同于来源的独立空项目**。先创建该项目数据库与 API 容器及空预览挂载，数据库运行，API 无须启动；确保源/目标 PostgreSQL 主版本一致。构建/创建目标栈仍需按目标环境单独审阅，不能直接指向线上栈。

```bash
bash "ops/shared/restore.sh" "/secure/focalroom-backups/backup.tar.gz" "focalroom-recovery-review" --apply --confirm-project "focalroom-recovery-review"
```

目标非空即拒绝，不提供覆盖/`--clean` 选项。目标停写后再次查空；恢复预览并单事务恢复数据库，核对迁移、每表计数、对象引用、文件 SHA256 与所有权。无论成功还是失败，目标 API 都保持停止；不自动把恢复数据投入服务。失败后的目标和暂存数据保留，继续诊断或另建空目标，不擅自清空重试。

备份含账户哈希、会话和项目内容；暂存文件和测试卷也需要受控保管与保留策略。SHA256 提供一致性校验，不提供来源认证：只恢复自己受控、可信的备份，`pg_restore` 会执行其中 SQL。角色和权限不包含在此逻辑备份中；恢复后管理员还须配置最小权限、核对当时的会话/授权是否应重新启用，验收应用与健康检查后再显式启动 API/Web。不能以本工具代替生产灾备、安全审阅或 RPO/RTO 承诺。

### 回归与证据

```bash
npm run test:ops
npm run test:ops:docker
```

9项标准库边界测试通过；6类真实 Docker/PG 演练通过，22表全部行的规范化内容摘要与2个预览对象恢复一致，另有 UID/GID 1000:1000 的只读进程实际读文件校验。Docker 演练只创建随机前缀项目，网络内部隔离、不发布端口、不拉镜像；API 容器为停写/启动状态的哨兵夹具，不冒充真实 NestJS 业务镜像。测试完成只停止自己新建项目，保留卷和临时数据供复核。30项 API/PG 业务回归另在临时 cluster 通过。完整范围、失败日志与限制见 `../../../docs/V2-M4-BACKUP-RESTORE-TEST-REPORT.md`。

以下首次整栈/生产核对保留有效，尚未因上述存储演练而关闭。

## 拓扑

Web/Nginx 同源代理 `/api/v2` 与 WebSocket 到独立 NestJS API；正式作品唯一权威为 PostgreSQL。私有预览可选择服务进程管理的磁盘或 R2。旧 `cloud/` SQLite 实验库、桌面 SQLite 和其他项目数据库均不复用。公网入口应是 HTTPS，API/数据库端口不直接暴露。

`compose.yaml` 只给新建隔离评审栈使用，Web 仅映射回环 `127.0.0.1:8084`。镜像版本/目标架构须在实际环境核实与锁定 digest。`environment.example` 中没有真实凭据；请在仓库外设置随机 URL 安全数据库密码。

## 首次本地容器评审（尚未执行）

1. 审阅 `.dockerignore`、两个 Dockerfile、`nginx.conf`、`compose.yaml`，确认上下文不含照片、数据库和凭据；构建新隔离镜像。
2. 用独立 Compose project 启动 PostgreSQL；等待健康，核对连接库名和账户。不使用 openyc 的库/账户/网络。
3. 在该全新评审栈显式执行一次迁移：`docker compose run --rm api node apps/api/src/server.ts --migrate`。普通 API 启动不执行 DDL。首次初始化由 `postgres-init/01-create-roles.sh` 创建最小权限角色（需要 `FOCALROOM_APP_PASSWORD` 与 `FOCALROOM_MIGRATOR_PASSWORD`）；迁移自动改用迁移角色连接，运行时 API 使用仅 DML 的应用角色。
4. 启动 API/Web；打开 `/shared`，用生成素材完成注册、邀请、预览、评论、提案、审批、撤权与重连。确认 Web 和 API 的 Origin 完全一致。
5. 关闭评审栈后保留数据库卷；恢复启动验证 session/评论/版本。不要执行带 `-v` 的删除命令，除非已明确批准丢弃整个新评审库。

上述步骤需要 Docker 可用；本轮没有把不可执行项记为通过。

## 生产前必要配置

- 域名与 TLS 终结；精确 `FOCALROOM_WEB_ORIGIN`，同源代理并保留 Origin/Upgrade。HTTPS Origin 会启用 Secure Cookie。公网服务不使用 HTTP Origin。
- PostgreSQL 独立数据库、独立迁移角色与受限应用角色；不使用超级用户。DDL 迁移由管理员单独执行，应用只获得所需读写/序列权限。当前 Compose 超级用户只用于全新本地评审，不用于生产。
- 备份一致性覆盖 PostgreSQL 与预览对象。先在隔离环境完成还原演练，核验不可变版本、命令回执和对象 SHA，再制定保留期/RPO/RTO。
- R2 使用私有桶，禁用公开 r2.dev/公开自定义域名；只给本项目桶的最小 Put/Get 权限。真实配置放仓库外文件，进程指定 `FOCALROOM_R2_CONFIG`，同时取消 `FOCALROOM_PREVIEW_DIRECTORY`。对象通过服务器网关读取，客户端拿到的是 30 秒一次性网关票据，不是可绕过撤权的裸 R2 链接。
- R2 SDK 本轮只对本地 S3 协议服务器验证签名/PUT/GET；真实账户权限、网络、存储区域、费用、跨区域策略、对象保留与灾难恢复必须在目标环境单独验收。
- 认证与邀请：已有 scrypt、数据库持久窗口限流和四语 Web 账户恢复/验证入口，本地回归见账户恢复报告；多实例目标容量、生产 HTTPS/真实邮件交付和会话策略仍需验收。当前邀请由用户手动分享，不发送邮件。
- API 日志不记录请求体、密码、Bearer、Task token、邀请 token 或一次性媒体路径；示例 Nginx 对整个 `/api/v2` 关闭 access log。监控应采集请求类型、状态、耗时、连接/队列/数据库指标，不复制敏感内容。
- 安全头/CSP、Cookie/CSRF、JSON/图片输入边界、WS 断线/撤权、连接容量和数据库压力需在目标反向代理及多实例部署下重测。0.5 秒数据库事件轮询属于当前实现，不等于大规模实时容量已达标。
- 云 Agent 模型/队列未部署。短期评论 Task principal 只支持读原评论，不授予模型发送或作者记忆权限。没有配置任何真实模型密钥。

## 升级与回退

迁移记录校验 SQL SHA，已提交迁移追加新文件，不修改旧文件。`0001` 与仓库已提交版本逐字节一致，新增 `0002_audit_envelopes.sql` 保存后续命令的完整原始载荷并保护审计不可变；旧回执的 envelope 保持 NULL，不编造历史。自动化覆盖仅有 `0001` 的旧临时库升级及回执保留。服务正常启动只核验已应用迁移，不执行 DDL；缺版本或 SHA 不符即拒绝启动。本轮所有迁移只应用于新建临时 cluster，未应用用户或生产库。

上线前保留前一应用镜像、数据库与对象备份，记录版本/hash。正式作品/事件/回执不可变；不得通过删除历史表、手工调整 revision、把 SHARED 改回 LOCAL 或清空幂等键“修复”升级。异常先暂停新写入，检查权威状态，再在隔离恢复库验证修复脚本。回退优先切回兼容镜像；若涉及不可逆数据变化，必须按备份/修复方案独立批准。

没有对用户的腾讯云主机执行命令、生产 DDL、服务重启或部署。本项目与 openyc 的既有资源保持独立。
