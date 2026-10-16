# ADR 0031：接续已有 M4 本地 HTTP 切片，先修复权限与认证边界

日期：2026-10-16。基线：`9976b16`。状态：本地工程切片；不是 M3/M4 阶段通过或生产部署决定。

## 背景

接续时发现仓库已新增 `between-frames/cloud` axum/SQLite API，已有两项真实 HTTP 测试，但入口文档仍称完全没有 API 代码。它不使用 `PostgresPool`，schema/时间表达式也依赖 SQLite，不能以“sqlx 双方言、只换连接串”宣布可接生产库。腾讯云没有部署本项目服务的事实不变。M3 质量门槛仍未关闭；本轮只修复既有切片，不追加新协作功能或把该切片升级为共享项目权威。

## 决策

- 授权撤销按 project_id + grant_id 精确更新，不再忽略 gid 批量撤销。授权 upsert 保留稳定 ID、清空旧 revoked_at；不删除并重建 grant。已有五表 schema 不改，不迁移桌面用户库。
- 密码采用 RustCrypto Argon2id PHC（随机盐和参数同存），复用 password_hash，保留 salt 旧列。阻塞计算至多四个并发；未知账户执行同类计算后统一拒绝。令牌使用两个随机 UUID 拼接，仅摘要存库，24h 时限由当前可信 SQLite 时间执行；只注销当前令牌。
- 注册用户与会话在一个事务中提交。SQL/文件系统细节不回传。规范化邮箱，限制请求体、密码、名称和项目标题。
- 不接受旧快速密码哈希或明文会话，不做自动弱算法回退。当前只能另建独立实验库；正式密码重置/身份迁移需单独设计和验收，不删除旧账户或重写旧库。
- 预览请求严格匹配项目登记、64 位小写 SHA 标识、精确 `.jpg`。实际文件为 ≤16MiB、两边 ≤1400px 可解码 JPEG，字节数和尺寸与登记一致。Unix 防最终文件符号链接替换；根/父目录须可信。SHA 为源指纹，不误称派生内容完整性证明。没有上传端点，不能声称清 GPS/上传授权已闭环。
- 列表/文件/认证/错误均 no-store。文件发送前再查 grant；已传输的像素无法收回。不给 URL 公开匿名访问，不用此 HTTP ACL 冒充对象存储短签名/CDN 撤销。
- 首次启动显式 create_if_missing + 外键；默认回环监听。以真实二进制、新临时目录和完整进程重启验收。
- 按职责拆成 auth / previews / error / 项目 handlers，复用同一 ACL 与现有表。云测试纳入 verify-all，不引入第二套业务命令系统。前端和正式桌面状态不变。

采用 [RustCrypto Argon2 0.5.3](https://docs.rs/argon2/0.5.3/argon2/) 的 PHC 接口及 [sqlx SQLite 连接选项](https://docs.rs/sqlx/0.8.6/sqlx/sqlite/struct.SqliteConnectOptions.html)，实际依赖固定于 cloud/Cargo.lock。新增依赖仅在独立云 crate，不升级桌面核心依赖。

## 回滚与限制

仅源代码和测试更改，没有线上 DDL/部署。保留旧实验库原件，切换不同 BF_CLOUD_DB 即可隔离；新版 PHC/摘要记录不能交给旧弱实现使用。未改既有未跟踪 Dockerfile，未构建/部署容器。生产 TLS、邮箱验证/重置、防爆破、身份审查、Postgres 权威、Workspace/Invite、Promotion 和私有对象存储仍需实现。Axum 的实验实现不替代规格的生产服务选型评审。
