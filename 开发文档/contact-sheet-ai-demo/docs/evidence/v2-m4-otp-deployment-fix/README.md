# 验证码与镜像修复证据

本地验收日期：2026-10-18。结论与边界见 [报告](../../V2-M4-OTP-DEPLOYMENT-FIX-TEST-REPORT.md)，生产操作见 [部署方案](../../V2-M4-OTP-DEPLOYMENT-PLAN.md)。

| 文件 | 内容 |
| --- | --- |
| pg-all-final.log | 52 项 PostgreSQL/API 全套，0 失败/跳过 |
| pg-all-first-failure.log | 首轮旧迁移清单断言失败，保留以对照修复 |
| pg-cookie-get-final.log | GET 认证修复后 4 项 Worker/鉴权测试；与全套重叠，不相加 |
| frontend-final.log | 34 文件、237 项前端测试 |
| lint-final.log / build-final.log | Lint、类型与 Desktop/Web 构建 |
| docker-smoke-first-eacces.log | 祖先目录权限遗漏导致的首轮实际容器失败 |
| docker-smoke-final.log | 最终真实 API/Web 容器链路通过 |
| docker-api-build-final.log / docker-web-build.log | 两个真实业务镜像构建日志 |
| image-summary.json | UID1000、关键文件摘要、PG 实际版本、HTML/代理/邮件失败状态 |
| source-sha256.json | 本轮 21 个相关源码文件最终 SHA256；八个关键文件与已验镜像再次核对一致 |
| docker-smoke-used.py / local-build-context.json | 当时执行的 Docker 测试脚本及本机临时上下文定位；不包含真实密钥。依赖保留的本机路径，不是独立部署脚本 |
| browser-checks.json / browser-project-comment.png | 模拟邮件登录、刷新、八图项目、批注、退出的 UI 验收记录/截图 |
| ssh-preflight.txt | 只读 SSH 在认证阶段被拒绝；没有远端执行结果 |

复跑业务测试（应用根目录，使用已有 Node/npm 和本机 PostgreSQL 工具）：

```bash
npm run test:cloud:pg
npm test
npm run lint
npm run build:workspaces
```

`test:cloud:pg` 创建临时数据库，不指向用户库；后续源码新增测试可能使数量变化。日志反映记录时的执行，不预先保证未来修改仍通过。测试脚本/镜像/卷保留供审阅；不要把测试脚本改为生产地址，也不要自动清理 Docker 资源。

真实邮件、腾讯云部署、公网 HTTPS 会话和原生签名包不在本证据范围。截图是本地夹具，不是真人评审或线上作品。
