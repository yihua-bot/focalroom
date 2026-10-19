# FocalRoom API/Web 发布与真实邮件验收

日期：2026-10-19。用户明确回复“确认部署并测试邮件”后执行。范围仅腾讯云独立 FocalRoom API/Web、备份、0014/0015 和一次真实验证码邮件；未修改 openyc 服务、官网、DNS 或公开客户端，没有 git commit/push。

## 已完成

生产地址：[团队协作](https://team.focroom.cc/shared)。API/Web 已替换为匹配当前实现的固定 amd64 镜像。数据库仅新增0014头像字段和0015账户AI账本；原有九项登记迁移摘要一致，没有盲跑SQL目录。生产PostgreSQL容器及两个数据卷保留。

| 项目 | 实测结果 |
|---|---|
| 备份 | 停API取得一致窗口，PG custom dump 70,807 bytes、27张表、预览0；pg_restore --list、文件SHA、预览清单通过；恢复旧API后上传镜像 |
| 镜像归档 | 119,451,314 bytes；两端SHA256为 `46341efc2076f98c105de88382463faea03ffac3f5e2465f4fc4415ee162eb19` |
| API | `focalroom-hosted-ai-api:20261019` / `sha256:7585d079143e4832e679793b2211ad556e1f8b3f964a72334819a4f12ef28fa1` |
| Web | `focalroom-hosted-ai-web:20261019` / `sha256:701567b2c00da025762352d31ede04cbc14e67365f726eb891ef2b960b8bcda3` |
| 镜像/源码 | UID1000无网络导入、11项登记迁移的镜像字节摘要、7个关键运行源文件与本地SHA、Nginx配置检查均通过 |
| 回环/公网 | /shared标题正确、/api/v2/health为200；匿名/auth/me和/ai/account为401；系统curl公网TLS验证结果0，无跳过证书校验 |
| 角色/凭据 | 应用角色无schema CREATE/表所有权；新账本DML有效，分配流水无DELETE；API环境移除迁移URL，仅一次性任务读取独立私有env |
| 邮件 | Resend仅openyc.vip已验证且允许发送；From为 `FocalRoom <noreply@openyc.vip>`，未擅自改DNS或声称focroom.cc发件域已验证 |
| 真验证码 | 生产页面仅发送一次；精确匹配测试邮件，Resend last_event=delivered；取该已发邮件的验证码完成真实Cookie登录，没有重复发信 |
| 会话 | 账户/工作区显示；页面刷新/API单独重启后保持登录；退出后/ai/account为UNAUTHORIZED，数据库活动会话0 |
| 单次消费 | 已用验证码再提交为400 / VALIDATION_ERROR；没有再发邮件 |
| AI状态 | 已登录生产/ai/account返回verified=true、enabled=false；granted/spent/reserved/available均0；未配置DeepSeek服务端密钥 |

收件验收已补齐：Resend发件记录为delivered，用户于2026-10-19明确回复“已在收件箱收到”，人工确认邮件进入Gmail收件箱。该人工证据独立于供应商回执；未读取私人收件内容，也未追加发送邮件。验证码、会话凭据、邮件Key、真实收件地址和数据库备份不进入仓库证据。

## 最终状态与操作边界

由0用户/0项目变为1个已验证测试账户、1个工作区/OWNER成员关系；摄影项目0。现有31张表、11项登记迁移；验证码1条且已消费、活动会话0。AI账户/分配/请求均0，真实模型调用0，云评论Worker未启用。

未执行compose down、删库/删卷或全局image/builder prune，旧镜像保留。发布及重启只操作FocalRoom API/Web。首次SSH查询成功、第二次认证拒绝，稍后重连成功；后续使用已校验主机密钥的复用连接，没有关闭HostKey校验或反复试密码。

最终剩余1,782,812,672 bytes，约1.66GiB；不宣称公共用户规模、配额或灾备门槛通过。

## 后续运维配置

服务器目录：`/opt/focalroom/releases/hosted-ai-20261019`。

- 当前运行配置为 `runtime.compose.private.json`，含密钥、0600。后续仅API/Web重建须使用该文件和 `--project-name focalroom-shared-review`，执行 `up -d --no-deps --no-build --pull never api web`。不要只用旧 `/opt/focalroom/ops/shared/compose.yaml`，否则会带回旧配置或迁移凭据、遗漏邮件设置。
- 迁移读取 `migrator.private.env`，不挂到常驻API；运行配置和.env为0600，release目录0700，文件不进仓库。
- `backup/`保存custom dump、预览归档、私有有效Compose/容器配置/.env和摘要。本轮检查了归档目录/校验和，**没有执行独立环境全量恢复演练**，不能称生产灾备完成。
- 已有0015，旧API的严格schema断言不兼容，不能直接起旧镜像或删迁移登记回退；需兼容镜像停用功能，覆盖恢复另行明确批准。

## 剩余首发项

1. DeepSeek有效服务端凭据、明确额度政策及有限真实调用未落实；不从聊天历史供应商Key自动启用生产。真实文本/图像/工具、预算和账本仍需验证。本轮没有发送照片。
2. Developer ID证书存在，Apple公证profile/ASC Key路径仍未提供；正式release安装包、签名/公证/staple/Gatekeeper、正式deep-link和干净机器生产全流程未验，官网没有新增下载。
3. 容量/存储配额/满盘保护、备份保留与独立恢复/告警仍开；Google真实登录未验，当前Web按关闭开关构建。
4. UI观察：验证码登录已验证账户仍显示“发送邮箱验证邮件”入口。没有点击发送第二封邮件；后续应按验证状态修正该冗余入口。
5. M3专业真人评测、M5真人双人项目、远端CI仍需证据，本次不把M0–M5全部标完成。

## 证据

过滤后的实际命令结果及源码摘要见 [evidence](evidence/v2-hosted-ai-production-20261019/README.md)。登录/重启截图含真实账户资料，仅保存在本机私有目录 `/Users/yihua/Documents/BetweenFrames-Reviews/hosted-ai-production-20261019/`，未加入仓库。

功能回归沿用 [本地报告](V2-HOSTED-AI-TEST-REPORT.md) 的257前端、71PG、224Rust和严格检查；本轮无产品源码变更，新增的是上述生产验收，不能把旧单测计数说成本轮重跑。
