# FocalRoom M4/M5 共享服务

独立 NestJS + PostgreSQL 权威服务，Node 25 原生运行 TypeScript。旧 `cloud/` 的 axum/SQLite 是兼容实验，不是本服务的数据目录或迁移来源。默认不启用云模型、支付或邮件发送。

## 本地验证

应用根目录：

```bash
npm run test:cloud:pg
# macOS 默认使用 Homebrew PostgreSQL 15；其他安装显式指定 bin 目录。
FOCALROOM_TEST_PG_BIN="/path/to/postgresql/bin" npm run test:cloud:pg
node "apps/api/test/run.mjs" "apps/api/test/web-journey.mjs"
# 先构建 target-v1 的 integration harness：
node "apps/api/test/run.mjs" "apps/api/test/native-promotion.mjs"
```

runner 创建新的临时 PostgreSQL cluster、回环端口、私有预览目录；结束关闭 cluster。不会连接 `FOCALROOM_DATABASE_URL` 或用户/线上数据库。测试日志显示临时目录，保留用于失败复核。不要给测试脚本传生产连接串。R2 测试只用本地 S3 协议服务器与测试凭证。

`npm run review:shared` 可启动独立本地评审环境（Web 1430），生成演练账号与仓库示例预览，终端输出评审链接。账号与评论均标记为演练，不是真人评测，也不读取用户图库。结束进程会关闭该临时 PostgreSQL cluster。

## 开发运行

先创建专用于本项目的空数据库与受控预览目录。下列命令只对明确指定的数据库执行迁移；服务正常启动从不自动执行 DDL。

迁移按顺序与 SQL SHA 校验：`0001` 保留已提交内容；`0002` 追加命令 envelope 与不可变审计约束。升级前的回执不补造原始请求，历史 envelope 保持 NULL；新增回执必须存完整原始载荷。服务缺少迁移或 SHA 不符时拒绝启动。

```bash
FOCALROOM_DATABASE_URL="postgresql://.../focalroom_dev" FOCALROOM_WEB_ORIGIN="http://127.0.0.1:1424" npm run migrate --workspace @betweenframes/api
FOCALROOM_DATABASE_URL="postgresql://.../focalroom_dev" FOCALROOM_WEB_ORIGIN="http://127.0.0.1:1424" FOCALROOM_PREVIEW_DIRECTORY="/private/path/previews" npm run dev:api
npm run dev:web
```

打开 Web `/shared`。默认 API `127.0.0.1:3002`；Vite `/api/v2` 同源代理，可用进程级 `FOCALROOM_DEV_API_URL` 覆盖开发目标。注册生成个人工作区；Owner/Admin 可管理工作区项目，普通 Membership 不隐式授予 ProjectGrant。项目邀请只创建项目 Grant，不把受邀者加入整个工作区。邀请返回私密链接，由邀请者自行分享；没有发送邮件。

桌面「导出」提供共享预览包和真实迁移两种操作。包导入是独立副本；真实迁移冻结本地写入，经受控上传与云端确认后进入 SHARED_CACHE。超时、进程重启、错误回执均保持冻结；只有云端确认尚未提交并取消 STAGING 后，才能显式恢复本地写入。已提交不能取消，须继续确认。云端后续版本在 Web 查看，桌面当前仅显示迁移时只读快照。

## 生产部署前

参见 `../../ops/shared/README.md`。本轮只验证临时 PostgreSQL、本地文件存储和模拟 S3 协议；没有执行生产 DDL、云部署、真实 R2 读写或容器构建。生产还需 HTTPS/域名、独立最小权限数据库账户、R2 私有桶/保留策略、邮件身份恢复、备份还原演练、监控限流与负载测试。不得用开发账户/临时 cluster 作为生产服务。

## 协议边界

- 路径 `/api/v2`；账号 scrypt-v1、24 小时 session，数据库只存会话摘要。Cookie HttpOnly/SameSite Strict；Cookie 写请求必须精确 Origin + CSRF，桌面用进程内 Bearer。密码/令牌不写 SQLite、日志或浏览器持久存储。
- `GET /workspaces`、`GET/DELETE /workspaces/:id/members[/user]`、`POST/DELETE /workspaces/:id/invitations[/invite]`、`POST /invitations/accept`。邀请摘要、7 天到期、邮箱绑定、一次性接受和撤销。移除成员与独立 ProjectGrant 撤销分开。
- `POST /promotions` → `PUT /projects/:id/previews/:photo` → `POST /projects/:id/finalize`；相同迁移键/清单幂等，缺预览不可提交。`POST /projects/:id/abort` 只取消未提交副本。源 SHA 与派生 JPEG 内容 SHA 分开。
- 上传只接受 <=8 MiB、长边 <=2048 的单张 JPEG；服务重编码并去 EXIF/XMP/GPS。私有对象内容寻址，不开放桶 URL。`POST /projects/:id/previews/:photo/ticket` 颁发 30 秒一次性网关链接；读取时复查 session/ACL、校验内容 SHA，IO 后再查 ACL。撤销无法收回已经下载的像素。
- `POST /projects/:id/commands` 严格复用 v2 契约；项目行锁 + 版本 + 权限 + 幂等键/载荷一致性。正式序列、不可变版本/事件、命令回执同事务提交。提案逐项人工批准；空选是驳回，不推进正式版本；过期提案保持原文供重建。
- `GET /projects/:id/snapshot`、`history`、`versions/:revision`；历史版本只读，不隐式恢复到当前作品。`PATCH /projects/:id/proposals/:proposal` 仅作者修订自己的 DRAFT，校验提案版本、当前作品基线和幂等载荷；未提交草案仅作者可见。提交后逐项勾选批准，冲突保留原提案，明确重建新草案。修订/回复保存原始载荷及审计，均不修改正式作品版本。
- `POST /projects/:id/comments/:thread/replies` 要求 body/idempotency_key，同项目根线程、同事务审计/回执。Reviewer 可评论/提案，不能批准；Editor 审批/正式编辑分开授权。
- `POST /projects/:id/ws-ticket` → WS `/api/v2/realtime` 首消息 authenticate/ticket。票据不在 URL，短期一次性；0.5 秒查持久事件与当前 ACL，重连重新取权威 snapshot，不重放离线正式编辑。Presence 30 秒失活。单实例本地分发，跨实例依靠同一 PostgreSQL 事件日志轮询。
- Yjs 仅存每用户每项目私人 note（<=4000 字、<=64 KiB），同用户窗口合并；不广播给其他评审者。拒绝正式序列、权限等 root 字段；Yjs 永远不进入正式审批/命令权威。
- `POST /projects/:id/agent-scopes` 颁发 5 分钟只读评论 Task 身份；`GET /agent/comments` 保留 author_id/reviewer/comment/anchor，当前 session/ACL/有效期每次重查，不能访问人类命令或跨项目。`workers/agent/src/comments.ts` 是实际只读工具客户端。此 token 不授予云模型上传或作者记忆权限；云模型 Worker/队列仍未实现。

服务是可本地运行的工程实现，不能替代真人质量评审、真实双人项目、R2/容器/生产运维验收。
