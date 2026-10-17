> 2026-10-17：正式 v2 共享权威为 `../apps/api` 的 NestJS/PostgreSQL 服务。
> 本目录 SQLite 的 `/api/v2` 适配草案存在密码与提交边界缺口，现已安全封闭并返回 501/UNSUPPORTED_AUTHORITY；保留源码供比对，不再自动创建 v2 草案表。旧 v1 协议测试不代表正式作品审批完成。不得把实验服务或其 Dockerfile 当作 PostgreSQL 生产服务。

# M4 本地云协议验证切片

本目录已有 axum HTTP 服务，验证注册/登录、项目所有者授权、项目预览读取和撤销拒绝。**只实现 SQLite 本地协议；没有部署到腾讯云，不是生产共享项目权威。** PostgreSQL 不能只换连接串：当前使用 `SqlitePool`、SQLite 时间函数和 schema，需另行实现迁移、事务、集成测试。

## 运行与测试

在应用根目录执行：

```bash
npm run verify:cloud
# 使用新的独立目录；不要填写桌面应用数据库或已有线上数据库。
BF_CLOUD_DB="/tmp/bf-cloud-review.sqlite" BF_PREVIEW_DIR="/tmp/bf-cloud-previews" cargo run --manifest-path "cloud/Cargo.toml" --locked
```

首次运行创建指定 SQLite 文件和预览目录；父目录必须已存在。默认监听 `127.0.0.1:3001`，`BF_CLOUD_BIND` / `BF_CLOUD_PORT` 可覆盖；不自动开放局域网。测试监听随机回环端口、使用临时目录，覆盖真实二进制首次启动与重启，绝不联系真实 AI 或用户数据库。

## API 与边界

| API | 行为 |
|---|---|
| `POST /auth/register` | email/display_name/password；规范化邮箱；Argon2id PHC；用户与会话同事务 |
| `POST /auth/login` | email/password；返回 token/user_id/display_name |
| `POST /auth/logout` | Bearer；只注销当前令牌 |
| `POST /projects` | Bearer；创建测试用项目元数据，不等于 Promotion |
| `POST /projects/{id}/grants` | owner；grantee_email/role（viewer 或 reviewer）；同收件人稳定 grant_id，恢复授权清空 revoked_at |
| `DELETE /projects/{id}/grants/{gid}` | owner；只撤销该项目该 grant；不存在/重复撤销返回 revoked:0 |
| `GET /projects/{id}/previews` | owner 或 active grant；不产生公开下载 URL |
| `GET /projects/{id}/previews/{sha}.jpg` | 相同 ACL；文件发送前复查，撤销后的后续请求拒绝 |
| `GET /health` | 进程存活，不代表 Postgres/对象存储或生产就绪 |

请求体最多 16 KiB。密码为 8–1024 UTF-8 字节，Argon2id 随机盐/参数包含于 PHC 串，`salt` 旧列保留为空。最多四个并发密码运算，移到阻塞线程；这不是按账户/IP 的防爆破策略。会话有效期 24 小时，不滚动续期；数据库只保存令牌 SHA-256 摘要，令牌只在登录/注册响应返回。所有响应 `private, no-store` / `Vary: Authorization` / `nosniff`，数据库异常不返回 SQL 或路径。

**兼容性变化**：早期切片的 SHA-256 密码账户与明文会话被拒绝，不自动降级或重写。已有实验库应保留原件并另建本地测试库；真实用户密码重置/身份迁移流程尚未实现。不要删除旧库或用此切片处理真实账户。

预览制备/上传 API 尚未实现。测试在独立库登记生成 JPEG；服务只读登记于该项目、64 位小写十六进制命名的 `sha-1400.jpg`，上限 16 MiB、两边均不超过 1400px，文件长度/实际 JPEG 解码尺寸须与记录一致。`sha` 是源照片指纹标识，**不是当前派生 JPEG 的内容校验和**，尚未实现上传内容完整性验证。拒绝普通符号链接；Unix 打开时附加 `O_NOFOLLOW` 防替换和 `O_NONBLOCK` 防特殊文件阻塞。存储根目录和父目录必须由可信服务管理，不支持攻击者控制父目录。不能从 GET 响应倒推素材授权、GPS 清除或上传已完成；这由后续受控媒体管道验收。撤销阻止新请求，无法回收已经下载的像素。

## 仍缺少

Postgres 权威与正式命令/审计、Workspace/Invite、显式 Promotion、私有对象存储与短时签名、受控去 GPS 上传、实际 Web 登录/审稿界面、邮箱验证/密码重置、按账户/IP 限流、HTTPS/CSRF 部署策略、真实运维与容器验收。`Dockerfile` 是既有未验收草案，本轮不修改或据此宣称可部署。M3 真人/竞品质量门槛仍未关闭，M4 只保留工程切片状态。
