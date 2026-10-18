# 生产只读核对与本地待部署镜像证据

日期：2026-10-18。解释和批准范围见 [生产核对记录](../../V2-M4-PRODUCTION-PREFLIGHT.md)。

- `remote-containers-and-migrations.json`：指定项目容器状态、旧镜像、源码 SHA、迁移元数据、错误类别和路径。未保存原始环境变量/日志正文。
- `remote-ownership-and-config.json`：数据库 owner、角色、扩展、用户/项目总数；配置仅记录是否存在，连接串仅保留用户名/主机/库名。无业务正文或凭据。
- `amd64-image-validation.json`、`amd64-*-build.log`：目标架构镜像构建、ID、UID1000 导入及源码 SHA、新 Web 标题。
- `release-archive.json`：离线镜像归档路径/大小/SHA；尚未上传。
- `existing-upgrade-used.mjs`、`existing-upgrade-final.log`：隔离临时 PG 的管理员旧表升级模拟，含失败回滚、限定所有权转移、幂等、数据保留和 DML/DDL 边界。测试生成密码不落输出，不连接生产。
- `deployment-override.yaml`：待批准的 API/Web 镜像和 Origin/邮件变量覆盖配置，不含密钥。
- `override-validation.json`：本机 Compose 合并验证，固定镜像/精确 Origin、空邮件 key、Web 回环和 PG 不暴露；没有启动生产服务。

未做生产 DDL、文件写入、权限变更、服务重启、真实邮件/模型调用、提交/推送或 Cloudflare 更改。SSH 前一轮失败记录保留在验证码修复证据目录；本轮通过用户提供的指定凭据完成只读核对，二者时间点不同。
