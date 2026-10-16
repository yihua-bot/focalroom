# M4 已有本地 HTTP 切片：认证、撤销隔离和预览边界修复

日期：2026-10-16。基线：`9976b16`。本轮为已有实现修复，**M3 质量门槛、M4 整体和 v2 全计划仍 partial**。没有部署、SSH、线上 DDL、桌面数据库迁移或真实 AI 外发；未提交/推送。保留既有未跟踪 `cloud/Dockerfile` 原样。

## 背景与实际范围

接续发现最新提交已新增 `between-frames/cloud` axum/SQLite 服务及两项 HTTP 测试；此前入口文档称“没有 API 代码”已过时。正确状态是：**已有独立本地实验 API，腾讯云仍无本项目已部署 API；Postgres 权威尚未实现**。本服务没有 Web 登录/审稿 UI，原提交中的“Web 读取”仅指 HTTP 可读，不是已完成产品界面。当前 `SqlitePool`、SQLite schema/时间表达式不支持只换 PostgreSQL 连接串。

对应 v2 规格 §3.1（项目单权威）、§3.5（受控媒体预览）、执行计划 M4、验收 S02。本轮修复已有切片的错误，不以 M4 切片绕过 M3 的任务规模和独立评测。架构边界见 [ADR 0031](ADR/0031-m4-local-cloud-boundaries.md)，运行/API/兼容性见 [cloud README](../between-frames/cloud/README.md)。

## 功能与异常验收

| 范围 | 修复与实测行为 |
|---|---|
| 单人撤销 | 旧实现忽略 URL 的 gid，按项目撤销全部 active grant；现按 project_id + grant_id。A 被撤销立即 403，B 和 owner 仍可读；其他项目 gid/不存在 gid/重复撤销均 0 条，不误伤 |
| 恢复授权 | 改为 upsert，保留 grant_id、清空 revoked_at；不再 INSERT OR REPLACE 删除旧行。非 owner 撤销返回 403 |
| 密码 | 快速 SHA-256 改为 Argon2id PHC，随机盐；同密码存不同串。密码计算移到阻塞线程，最多四个并发；未知账户同类计算，错误凭证统一拒绝 |
| 会话 | 响应返回随机令牌，库只保存摘要；24h 时限，旧/未来/非法 created_at 均拒绝。新增 logout 仅注销当前会话；另一登录会话继续有效 |
| 注册事务 | 用户与会话同事务；临时库触发器令会话 INSERT 失败，HTTP 500 且没有孤立用户。重复邮箱 409，规范化邮箱可登录/授权；无 SQL 细节回传 |
| 输入和缓存 | 非法邮箱/弱或超长密码/空名称 400；>16KiB 请求 413。认证、列表、文件和拒绝响应均 private,no-store / Vary:Authorization / nosniff |
| 预览 | 项目所属、精确 SHA64.jpg、小写十六进制、≤16MiB、真实可解码 JPEG 与登记尺寸/字节数；假 JPEG、额外后缀、跨项目、超尺寸、符号链接均拒绝。错误数据库短 SHA 返回 500，服务 health 仍正常，无切片 panic |
| 新目录和重启 | 真实 bf-cloud 二进制在不存在的 SQLite 文件/预览目录上启动；默认回环配置。进程退出后同临时目录启动新进程，原令牌/项目仍可访问 |

旧 SHA-256 实验账户和明文会话拒绝，未做弱算法回退或自动数据重写。旧实验库须保留原件，使用新独立库；密码重置/真实身份迁移尚未实现。这是兼容性变化，不能用新服务直接接旧真实账户。

预览只有读取接口；测试直接登记独立生成 JPEG。源照片 SHA 标识不等于派生 JPEG 内容 hash；上传完整性、去 GPS、素材授权仍须受控上传管道验收。Unix 最终文件打开有 O_NOFOLLOW/O_NONBLOCK；存储根和父目录须可信。撤销阻止后续请求，不撤回已下载像素；文件处理后再次检查 ACL，列表、并发/已发送响应仍遵循正常 HTTP 边界。没有对象存储签名/CDN 验收。

## 验证

`npm run verify:cloud`：**11 项真实 HTTP/进程测试通过（原 2 + 新 9），fmt、严格 Clippy 通过**。未跳过本模块任何测试，无外部 API、Keychain 或原照片。已将该检查纳入 `scripts/verify-all.sh`；远端 CI 尚未执行。

全量执行 `bash scripts/verify-all.sh --skip-keychain`，退出码 0：
- 桌面 Rust **134 通过、10 显式 ignored、1 Keychain 写删过滤**；云 crate **11 通过、0 ignored**。
- **159 Vitest / 21 文件，16 浏览器预览 E2E，6 最新 harness 真实后端旅程**全部通过。
- GUI/Agent/记忆/v2 契约、TypeScript、ESLint、两个 crate 的 fmt/严格 Clippy、harness 重建通过。新增云检查包含在现有 CI 调用链中；没有执行远端 CI。
- 120 个跟踪 PNG 在测试前按实际字节备份，测试改写的 22 张恢复至运行前字节，避免覆盖既有评审证据。
- 没有 UI 改动，不另做原生交互、Desktop/Web 发布构建或 Docker 构建；不把编译测试二进制算作部署验收。

证据：[全量日志](evidence/v2-m4-local-boundaries/verify-all.txt)、[云模块段](evidence/v2-m4-local-boundaries/cloud-check.txt)、[汇总](evidence/v2-m4-local-boundaries/summary.json)、[源文件摘要](evidence/v2-m4-local-boundaries/source-sha256.json)。初次编译发现 OsRng feature 未启用、严格 Clippy 发现测试中的多余借用，均已修复，最终全量通过。

## 工程原则与剩余

认证、预览读取、错误响应按职责拆分；共用一个 ACL 判断和现有表，集中口令/令牌规则，复用 SQLite 事务，避免新造权限/存储框架。新增依赖只在独立 cloud crate，锁定 lockfile；移除直接声明但未使用的 CORS 依赖。没有改历史 schema、契约生成物、桌面正式状态或 UI。

DONE：已有切片的单 grant 撤销、会话/密码保护、注册原子性、有界预览、新目录启动和重启回归；本地一键验证入口。

NOT DONE：M3 30 个分层任务/真人与竞品盲评/测量后路由和图像索引；M4 的 Postgres 权威、正式命令与审计、Workspace/Invite、显式 Promotion、私有对象存储/受控上传、真实 Web 登录审稿与部署。生产身份需邮箱验证、密码重置、防爆破/TLS/运维与人工安全评审；四并发限制不等于生产限流。Docker 草案本轮不构建、不修改，不宣称可部署。无 UI 改动，本轮不产出页面截图，也不把 HTTP 自动化称为原生 UI 验收。

回滚仅涉及本地源代码；未修改用户或线上数据。新版凭证格式不兼容旧弱实现，不把新实验库直接交给旧二进制。
