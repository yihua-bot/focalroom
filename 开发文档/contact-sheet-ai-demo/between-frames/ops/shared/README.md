# 独立共享服务交付与上线核对

这里是可评审的部署草案，不是已部署证明。当前机器 Docker daemon 不可连接，未构建/运行这些镜像；已验证的运行环境是 macOS Node 25 + 临时 PostgreSQL 15。不得从 Dockerfile/Compose 推断容器或生产验收通过。

## 拓扑

Web/Nginx 同源代理 `/api/v2` 与 WebSocket 到独立 NestJS API；正式作品唯一权威为 PostgreSQL。私有预览可选择服务进程管理的磁盘或 R2。旧 `cloud/` SQLite 实验库、桌面 SQLite 和其他项目数据库均不复用。公网入口应是 HTTPS，API/数据库端口不直接暴露。

`compose.yaml` 只给新建隔离评审栈使用，Web 仅映射回环 `127.0.0.1:8084`。镜像版本/目标架构须在实际环境核实与锁定 digest。`environment.example` 中没有真实凭据；请在仓库外设置随机 URL 安全数据库密码。

## 首次本地容器评审（尚未执行）

1. 审阅 `.dockerignore`、两个 Dockerfile、`nginx.conf`、`compose.yaml`，确认上下文不含照片、数据库和凭据；构建新隔离镜像。
2. 用独立 Compose project 启动 PostgreSQL；等待健康，核对连接库名和账户。不使用 openyc 的库/账户/网络。
3. 在该全新评审栈显式执行一次迁移：`docker compose run --rm api node apps/api/src/server.ts --migrate`。普通 API 启动不执行 DDL。
4. 启动 API/Web；打开 `/shared`，用生成素材完成注册、邀请、预览、评论、提案、审批、撤权与重连。确认 Web 和 API 的 Origin 完全一致。
5. 关闭评审栈后保留数据库卷；恢复启动验证 session/评论/版本。不要执行带 `-v` 的删除命令，除非已明确批准丢弃整个新评审库。

上述步骤需要 Docker 可用；本轮没有把不可执行项记为通过。

## 生产前必要配置

- 域名与 TLS 终结；精确 `FOCALROOM_WEB_ORIGIN`，同源代理并保留 Origin/Upgrade。HTTPS Origin 会启用 Secure Cookie。公网服务不使用 HTTP Origin。
- PostgreSQL 独立数据库、独立迁移角色与受限应用角色；不使用超级用户。DDL 迁移由管理员单独执行，应用只获得所需读写/序列权限。当前 Compose 超级用户只用于全新本地评审，不用于生产。
- 备份一致性覆盖 PostgreSQL 与预览对象。先在隔离环境完成还原演练，核验不可变版本、命令回执和对象 SHA，再制定保留期/RPO/RTO。
- R2 使用私有桶，禁用公开 r2.dev/公开自定义域名；只给本项目桶的最小 Put/Get 权限。真实配置放仓库外文件，进程指定 `FOCALROOM_R2_CONFIG`，同时取消 `FOCALROOM_PREVIEW_DIRECTORY`。对象通过服务器网关读取，客户端拿到的是 30 秒一次性网关票据，不是可绕过撤权的裸 R2 链接。
- R2 SDK 本轮只对本地 S3 协议服务器验证签名/PUT/GET；真实账户权限、网络、存储区域、费用、跨区域策略、对象保留与灾难恢复必须在目标环境单独验收。
- 认证与邀请：当前密码 scrypt，登录内存并发门槛和短时账户/IP 限流；多实例须补集中限流、邮箱验证/密码恢复/会话管理与邮件交付。当前邀请由用户手动分享，不发送邮件。
- API 日志不记录请求体、密码、Bearer、Task token、邀请 token 或一次性媒体路径；示例 Nginx 对整个 `/api/v2` 关闭 access log。监控应采集请求类型、状态、耗时、连接/队列/数据库指标，不复制敏感内容。
- 安全头/CSP、Cookie/CSRF、JSON/图片输入边界、WS 断线/撤权、连接容量和数据库压力需在目标反向代理及多实例部署下重测。0.5 秒数据库事件轮询属于当前实现，不等于大规模实时容量已达标。
- 云 Agent 模型/队列未部署。短期评论 Task principal 只支持读原评论，不授予模型发送或作者记忆权限。没有配置任何真实模型密钥。

## 升级与回退

迁移记录校验 SQL SHA，已提交迁移追加新文件，不修改旧文件。`0001` 与仓库已提交版本逐字节一致，新增 `0002_audit_envelopes.sql` 保存后续命令的完整原始载荷并保护审计不可变；旧回执的 envelope 保持 NULL，不编造历史。自动化覆盖仅有 `0001` 的旧临时库升级及回执保留。服务正常启动只核验已应用迁移，不执行 DDL；缺版本或 SHA 不符即拒绝启动。本轮所有迁移只应用于新建临时 cluster，未应用用户或生产库。

上线前保留前一应用镜像、数据库与对象备份，记录版本/hash。正式作品/事件/回执不可变；不得通过删除历史表、手工调整 revision、把 SHARED 改回 LOCAL 或清空幂等键“修复”升级。异常先暂停新写入，检查权威状态，再在隔离恢复库验证修复脚本。回退优先切回兼容镜像；若涉及不可逆数据变化，必须按备份/修复方案独立批准。

没有对用户的腾讯云主机执行命令、生产 DDL、服务重启或部署。本项目与 openyc 的既有资源保持独立。
