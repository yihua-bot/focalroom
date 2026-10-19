# 账户 AI 验收证据

此目录只保存工程测试日志、源码摘要、生成账户的账本与原生截图，不包含真实凭据、邮件验证码、用户原照片或模型请求/响应正文。

- `native-protocol-invariants.json`：修复后的任务 provider/account/model、报告用量 15、正式 revision 与照片决定/源哈希未变。
- `native-pg-ledger.json`：真实隔离 PostgreSQL 的 1 次 COMPLETED、charged=15、reserved=0、available=249985。
- `native-final-account-debit.png`、`native-final-logout-disabled.png`：原生调用后的余额和退出禁用。
- `native-final-diagnostics.png`：最后包的诊断来源显示为 FocalRoom/DeepSeek。
- `sources.sha256.json`、`validation.json`：最终来源摘要和检查边界。早期单独 AI Review app 已被后续注册包验收取代，不作为最终发布候选。
- `final-image-code-match.json`、`server-archive.json`：本地 API 镜像与关键源码 SHA 一致、无供应商 Key，amd64 归档未上传生产。
- 失败日志保留共享数据库夹具时序、TCP 非阻塞夹具和入口修复前的检查记录；最终日志独立命名，不删除失败分母。

所有模型行为为显式本地协议夹具，不是 DeepSeek 服务质量证明。真实邮件/模型/生产部署、正式签名公证与干净机器验收未计通过。
