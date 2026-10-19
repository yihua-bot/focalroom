# 生产发布与邮件证据 · 2026-10-19

对应 [报告](../../V2-HOSTED-AI-PRODUCTION-DEPLOYMENT-REPORT.md)。均为过滤凭据后的实际结果，没有原始.env、dump、OTP、会话、真实收件人或密钥。

- preflight.json / topology-before.json：发布前服务、磁盘、27表、九项迁移与角色。
- backup-upload.jsonl：PG custom归档检查与镜像字节数/SHA校验。
- install.jsonl：固定镜像、0014/0015、回环健康/鉴权、私有配置及最终镜像身份。
- mail-container-preflight.json：API容器内访问Resend/已验证From，未发邮件。
- public-https.json：公网200/401及TLS验证结果。
- api-restart.json：单独重启API、PG容器保留；浏览器另验会话保持。
- used-code-rejected.json：已消费验证码再提交400，没有重复发送。
- production-final.json：最终行数/权限、单封邮件delivered、7源文件SHA、真实模型请求0。
- validation.json：实际容器关键源码与本地匹配、敏感数据排除。

网页截图含真实账户信息，仅在本机私有评审目录保存，不进仓库。用户后续明确确认“已在收件箱收到”，见human-inbox-confirmation.json。production-final.json保留当时服务器输出的待确认快照，不改写原始结果；最新validation.json已记录人工收件通过。备份留在服务器，未执行生产覆盖恢复。
