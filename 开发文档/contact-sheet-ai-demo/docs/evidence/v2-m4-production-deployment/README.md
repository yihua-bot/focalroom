# 已批准的生产部署证据

日期：2026-10-18。结论与剩余边界见 [部署报告](../../V2-M4-PRODUCTION-DEPLOYMENT-REPORT.md)。

- `deployment-used.py`：实际部署控制程序，兼容Python3.10修复后版本。包含明确旧状态/容量/范围/归档与镜像校验、备份、限定角色变更、迁移及本机验收；不包含实际密码。程序为本次基线的一次性审阅记录，不应盲目重跑。
- `attempt-1-failure.json`：首轮兼容失败及处理说明；服务器实际首轮journal另包含于final-verification.json。
- `verification-used.py` / `final-verification.json`：部署完成journal、私有备份的有限元数据、八项迁移SHA和角色权限。保留服务器Python回访公网403/非JSON结果，不当作公网通过。
- `public-system-curl.json`：独立系统curl、保持TLS校验，公网200新标题和401/JSON。
- `browser-checks.json`：IAB登录页和API匿名响应复验；未执行真实账户登录。
- `public-before-502.png` / `public-login-ready.png`：公网502到新登录页的实际截图。

没有把数据库dump、环境文件、SSH密码或邮件/模型密钥保存到本目录。此前amd64构建/归档和隔离旧库升级证据保留于 `../v2-m4-production-preflight/`；其“尚未上传”反映批准前的历史时点，当前已部署事实以本目录为准。
