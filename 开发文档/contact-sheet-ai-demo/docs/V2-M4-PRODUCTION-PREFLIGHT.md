# M4 生产只读核对与待批准部署

后续：用户已明确回复「确认部署」，上述范围已执行，公网200/401和新登录页通过。当前结果、实际镜像/备份、失败和剩余边界见 [生产部署报告](V2-M4-PRODUCTION-DEPLOYMENT-REPORT.md)。本页保留批准前的核对及计划，不作为当前故障状态。

日期：2026-10-18。使用用户明确提供的密码认证到指定主机；保持已知主机密钥校验，凭据仅经进程管道交给 SSH，未写入命令参数、环境变量或证据文件。**本轮没有上传生产文件、修改配置/权限/数据库、重启容器或更改 Cloudflare。**

## 已确认的线上状态

| 项目 | 只读结果 |
| --- | --- |
| 主机 / 路径 / 项目 | x86_64；`/opt/focalroom`；`focalroom-shared-review` |
| API | exited / exit 1；旧镜像 `acc93b7f086f…`；日志 EACCES，涉及 `/app/apps/api/migrations/0006_rate_limits.sql` |
| Web | running；镜像 `b2c5dd710749…`；实际 HTML 标题「FocalRoom · Web 工作台预览」；8084 只映射宿主回环 |
| 宿主源码 | 标题已是「FocalRoom — 团队协作审稿」；宿主文件 0644；多项 API 源码 SHA 与本地修复版不同；缺 0012 SQL。宿主当前权限不能代表旧镜像内权限 |
| PG | 独立 PostgreSQL 16.14，running；未发布宿主端口；仅 plpgsql 扩展，不是已验 pgvector |
| schema | 只登记 0001/0002；两者 SHA 与本地已登记 SQL 完全一致 |
| 数据 | fr_users 0 行、fr_projects 0 行；未读取业务正文或账户标识。零行不免除备份 |
| 角色与 owner | focalroom 为管理员，focalroom_app 非超级用户；没有 focalroom_migrator。18 张 fr_ 表、2 个序列和 2 个函数由 focalroom 拥有 |
| 配置 | API 的应用/迁移连接用户名分别为 focalroom_app / focalroom_migrator，后者角色并不存在；Origin 为官网域名，协作应用应为 team 子域名 |
| 邮件 | .env 与当前 API 均无 Resend key；未读取/打印已有数据库密码，不能据配置名称宣称真实投递可用 |
| 其他 | .env 当前 0644；盘剩余 2,348,494,848 字节（约 2.19 GiB）；该项目没有 Worker 容器 |

数据库读取以 READ ONLY 事务执行；记录元数据、迁移 SHA 和必要总数，没有修改数据。证据中只保留配置是否存在，不保留凭据值。没有证据证明 Docker 缓存机制异常，不使用清镜像/全局 prune 作为处理方式。

## 已在本机准备并验证

沿用前轮已验证的冻结构建上下文，新建目标架构镜像；没有在服务器上编译或加载镜像。

| 镜像 | ID | 架构 |
| --- | --- | --- |
| focalroom-release-api:otp-20261018-amd64 | `sha256:dbf00eca3ba2b9329fb822757b9cba37294835f4d4b962e7ed1439ec726f57d5` | amd64 |
| focalroom-release-web:otp-20261018-amd64 | `sha256:5cc4b4dbed8dc64bc427c097e870e70a96cc13ddd26134054a2944b3e11b2933` | amd64 |

- 两个实际 Docker 构建 exit 0；API 以 UID1000 在 amd64 容器中导入 server.ts，八个关键源码摘要与已验修复版一致；Web 镜像实际 HTML 为新标题。
- 预备离线归档 `/tmp/focalroom-release-otp-20261018-amd64.tar.gz`，119,562,736 字节（约114 MiB），SHA256 `21d14f1345ae00c6fff428004bb9ccc6740e820feaeaeda60b009afc5012b9b7`。尚未上传。这样可以避免在当前空间紧张的服务器上运行 npm 构建；仍须在上传前重新检查容量。
- 隔离临时 PG 复现「0001/0002 为管理员所有」：仅 GRANT ALL 仍不能 ALTER 旧表，迁移失败并回滚；限定 public.fr_ 表/序列/函数所有权转给迁移角色后，八版本迁移及幂等重跑通过。旧测试用户保留、应用 DML 通过、应用建表/改表被拒。不是在生产库上试错。
- 先前业务验证见 [验证码修复报告](V2-M4-OTP-DEPLOYMENT-FIX-TEST-REPORT.md)：52 PG/API、专项 GET 回归、237 前端及实际 arm64 整栈/浏览器验收。本轮 amd64 验证范围为构建、非 root 导入/摘要和静态 HTML；未冒充生产完整旅程。
- 更正 [部署指南](DEPLOYMENT-GUIDE.md)：移除当前文件中的真实 SSH 密码，修正 Origin、迁移顺序、存量角色、静态资产、Worker 与 TLS 排障说明；没有向外发送文档或清理 Git 历史。

## 请求批准的具体操作

只处理上述 FocalRoom 项目，不操作 openyc、官网、DNS、Tunnel、TLS，也不执行 git commit/push、compose down、删除卷/镜像或 prune。

1. 再核对目标容器/挂载、0001/0002 SHA、对象 owner 和磁盘。若状态变化、出现新写入者，先停下重新评估。按归档、镜像导入和备份大小预留容量；无法保留至少 1 GiB 运行余量时不继续，不自动清理磁盘。
2. 暂停该项目写入并生成受控 PG custom dump 与预览卷完整备份，核对清单/摘要、记录原服务状态和镜像 ID。当前 API 已退出、无 Worker，但仍在操作时复核；备份保留。
3. 上传并核验离线镜像归档，加载上表两个固定镜像，记录实际 ID。使用新增部署覆盖配置选择这两个 tag（见 evidence 中的可审阅 YAML），保留服务器旧源码/基础 Compose 和旧镜像；启动禁止自动 build/pull。
4. 保留 .env 其他凭据，修正 Origin 为 `https://team.focroom.cc`，将该文件权限收紧至 0600。新增覆盖文件不含密钥，只补入邮件环境变量传递；邮件 key 继续为空。
5. 用数据库管理员按现有受控配置创建缺失的迁移角色；仅在 FocalRoom 独立库内把已核对的18表、2序列和2函数交给该角色，授予 public 的必要 schema 权限。应用角色继续为非 owner、非超级用户；不重设 SSH 密码、不扩大应用 DDL 权限、不对其他库 REASSIGN OWNED。
6. 用新 API 镜像和迁移角色执行六项尚未应用的登记迁移（0006、0008、0010、0011、0009、0012）；不遍历草案目录、不改旧 SQL/迁移登记。普通 API 启动仍只读校验 schema。
7. 仅重新创建 API/Web，复用现有 PG 与预览卷；不启动 Worker 或模型。先验回环静态页面和匿名鉴权，再验公网新标题、API 鉴权与错误表现；不向真实用户作品写测试内容。

这些操作涉及生产数据库结构/所有权、权限配置和服务替换，须获得用户明确确认后实施。运行中 Web 会短暂中断，不能保证无停机。

## 仍然没有的条件与回退边界

真实邮件 key/已验证发送域、接收测试邮箱尚未提供。部署本修复后未配置邮件时预期明确返回 503；**批准上述更新不等于验证码邮件已可用**。真实发送须另有明确收件人和交付验收。

0012 后旧镜像可能因严格迁移清单而拒绝启动，不能承诺只切回旧 tag 即恢复。若迁移失败，事务回滚，保留停写和证据；若迁移成功但启动/业务检查失败，不删除登记或逆改 schema。使用修复镜像处理，或在独立目标验证备份恢复方案后再获批准切换；不覆盖原生产卷。原 API 本已故障，因此保留旧镜像只是取证措施，不能称为健康回退目标。

SSH 密码曾出现在原指南、聊天及可能已提交的历史中；当前文件移除不代表已经撤回。管理员需安排轮换，本轮没有擅自更改凭据。

证据：[evidence/v2-m4-production-preflight](evidence/v2-m4-production-preflight/README.md)。本轮只读核对和本机构建不改变 M0–M5 总体完成状态。
