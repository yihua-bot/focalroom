# FocalRoom 生产部署指南

## 架构总览

```
用户浏览器
    ↓ HTTPS
Cloudflare CDN（全球加速 + DDoS 防护）
    ↓
    ├── focroom.cc          → Cloudflare Pages（官网静态托管，免费，不需要服务器）
    └── team.focroom.cc     → Cloudflare Tunnel → 腾讯云服务器
                                   ↓
                              Docker Compose 栈：
                              ├── NestJS API（端口 3002，内部）
                              ├── PostgreSQL 16（端口 5432，内部）
                              ├── Agent Worker（模型调用）
                              └── Nginx Web（端口 8084 → Cloudflare Tunnel）
```

## 服务器信息

| 项 | 值 |
|---|---|
| 云服务商 | 腾讯云 |
| 服务器 IP | 1.13.251.21 |
| SSH | `ssh root@1.13.251.21`（密码认证） |
| SSH 密码 | `Openyc@2026prod` |
| SSH 完整命令 | `sshpass -p 'Openyc@2026prod' ssh -o StrictHostKeyChecking=no -o PreferredAuthentications=password -o PubkeyAuthentication=no root@1.13.251.21` |
| OS | Ubuntu 22.04 LTS x86_64 |
| 配置 | 2 核 / 4GB / 50GB / 30Mbps |
| Docker | 29.1.3 |
| 到期 | 2027-07-15 |

## 域名

| 域名 | DNS 托管 | 指向 | 用途 |
|---|---|---|---|
| focroom.cc | Cloudflare（NS 已改） | Cloudflare Pages（`focalroom.pages.dev`） | 官网 |
| team.focroom.cc | Cloudflare | Cloudflare Tunnel（`TUNNEL_ID.cfargotunnel.com`） | 协作审稿 |
| api.netstar.org.cn | 阿里云 DNS（hichina.com） | 1.13.251.21（openyc 项目用） | openyc API |

**Cloudflare 设置**：
- Zone ID：`2db53cca66700c4b04b6f124be4d5378`
- SSL/TLS 模式：**Flexible**（Cloudflare 到源站走 HTTP，用户到 Cloudflare 走 HTTPS）
- focroom.cc DNS：CNAME `@` → `focalroom.pages.dev`（开启代理）
- team.focroom.cc DNS：CNAME `team` → `TUNNEL_ID.cfargotunnel.com`（开启代理）

## 部署步骤

### 1. 前置条件

- 腾讯云服务器可 SSH
- Docker + Docker Compose 已安装
- 域名 focroom.cc DNS 已托管到 Cloudflare
- Cloudflare API Token（权限：Pages Edit + DNS Edit）

### 2. 服务器上准备目录

```bash
ssh root@1.13.251.21

mkdir -p /opt/focalroom
cd /opt/focalroom
```

### 3. 传输代码

从本机执行：

```bash
# 打包（排除不需要的文件）
COPYFILE_DISABLE=1 tar czf /tmp/fr-deploy.tar.gz \
  --exclude='node_modules' --exclude='target*' --exclude='.git' \
  --exclude='src-tauri' --exclude='cloud' --exclude='*.sqlite*' \
  --exclude='.DS_Store' --exclude='__pycache__' \
  --exclude='*.png' --exclude='*.jpg' --exclude='*.CR2' --exclude='*.heic' \
  package.json package-lock.json tsconfig.json tsconfig.node.json \
  src apps packages scripts ops/shared public

# 传输
sshpass -p 'Openyc@2026prod' scp \
  -o StrictHostKeyChecking=no \
  -o PreferredAuthentications=password \
  -o PubkeyAuthentication=no \
  /tmp/fr-deploy.tar.gz root@1.13.251.21:/tmp/
```

服务器上解压：

```bash
cd /opt/focalroom
tar xzf /tmp/fr-deploy.tar.gz
```

### 4. 配置环境变量

```bash
cat > /opt/focalroom/.env << 'EOF'
FOCALROOM_DB_PASSWORD=你的数据库密码（随机 URL 安全字符串）
FOCALROOM_WEB_ORIGIN=https://focroom.cc
FOCALROOM_APP_PASSWORD=你的应用角色密码
FOCALROOM_MIGRATOR_PASSWORD=你的迁移角色密码
FOCALROOM_RESEND_API_KEY=你的Resend API Key
FOCALROOM_AGENT_MODEL_KEY=你的DeepSeek API Key
FOCALROOM_AGENT_MODEL_BASE_URL=https://api.deepseek.com/v1
FOCALROOM_AGENT_MODEL_NAME=deepseek-chat
FOCALROOM_EMAIL_FROM=FocalRoom <noreply@focroom.cc>
EOF
chmod 600 /opt/focalroom/.env
```

### 5. 启动服务

```bash
cd /opt/focalroom
docker compose -f ops/shared/compose.yaml --env-file .env up -d --build
```

首次构建约 5-10 分钟（npm ci 下载依赖）。完成后验证：

```bash
# 容器状态
docker compose -f ops/shared/compose.yaml ps

# 健康检查
curl -s http://localhost:8084/
```

### 6. 执行数据库迁移

```bash
docker compose -f ops/shared/compose.yaml --env-file .env run --rm api node apps/api/src/server.ts --migrate
```

### 7. 重启 API（迁移后）

```bash
docker compose -f ops/shared/compose.yaml --env-file .env restart api
```

### 8. 配置 Cloudflare Tunnel

```bash
# 安装 cloudflared
curl -fsSL https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o /usr/local/bin/cloudflared
chmod +x /usr/local/bin/cloudflared

# 使用已有的 Tunnel Token 启动（Token 从 Cloudflare Dashboard → Zero Trust → Tunnels 获取）
setsid nohup cloudflared tunnel run --token "<TUNNEL_TOKEN>" > /tmp/cloudflared.log 2>&1 < /dev/null &
```

Tunnel ID：`53932c6a-9abe-44ef-b68f-76e312760438`

Tunnel 路由配置（Cloudflare Dashboard → Zero Trust → Tunnels → focalroom-team → Public hostname）：

| 子域名 | 域名 | 服务 |
|---|---|---|
| team | focroom.cc | http://localhost:8084 |

### 9. 官网部署到 Cloudflare Pages

从本机执行：

```bash
export CLOUDFLARE_API_TOKEN="你的Cloudflare API Token"
export CLOUDFLARE_ACCOUNT_ID="66ec1d093e8601768c7cb1b6490ed2f9"

npx wrangler pages deploy /path/to/focalroom-website/dist --project-name=focalroom
```

或直接在 Cloudflare Dashboard → Pages → focalroom → Upload assets。

### 10. 验证

```bash
# 官网
curl -s https://focroom.cc/ | grep "<title>"

# 协作 Web
curl -s -o /dev/null -w "%{http_code}" https://team.focroom.cc/

# API
curl -s -X POST https://team.focroom.cc/api/v2/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@test.dev","password":"wrong"}'
# 应返回 {"error":{"code":"UNAUTHORIZED",...}}
```

## 数据库

- PostgreSQL 16.14（pgvector 扩展）
- Docker 容器：`focalroom-shared-review-postgres-1`
- 用户：`focalroom` / 数据库：`focalroom`
- 迁移文件：`apps/api/migrations/`（0001–0010）

### 执行迁移

```bash
cd /opt/focalroom
docker compose -f ops/shared/compose.yaml --env-file .env run --rm api node apps/api/src/server.ts --migrate
```

### 备份

```bash
bash ops/shared/backup.sh /opt/backups
```

### 恢复

```bash
bash ops/shared/restore.sh /opt/backups/focalroom-backup-XXXXXXXX.tar.gz
```

## 环境变量汇总

| 变量 | 说明 | 必填 |
|---|---|---|
| FOCALROOM_DB_PASSWORD | PostgreSQL 超级用户密码 | ✅ |
| FOCALROOM_APP_PASSWORD | 应用角色密码（最小权限） | ✅ |
| FOCALROOM_MIGRATOR_PASSWORD | 迁移角色密码（DDL 权限） | ✅ |
| FOCALROOM_WEB_ORIGIN | 前端 Origin（用于 CORS/Cookie） | ✅ |
| FOCALROOM_DATABASE_URL | API 数据库连接串（compose 自动设置） | ✅ |
| FOCALROOM_PREVIEW_DIRECTORY | 预览文件目录（compose 自动设置） | ✅ |
| FOCALROOM_RESEND_API_KEY | Resend 邮件 API Key | ⚠️ 邮件功能需要 |
| FOCALROOM_EMAIL_FROM | 发件人地址 | ⚠️ |
| FOCALROOM_AGENT_MODEL_KEY | DeepSeek API Key | ⚠️ AI 功能需要 |
| FOCALROOM_AGENT_MODEL_BASE_URL | 模型端点 | ⚠️ |
| FOCALROOM_AGENT_MODEL_NAME | 模型名称 | ⚠️ |

## 与 openyc 项目的关系

**完全独立**。FocalRoom 的 Docker Compose 使用独立的项目名（`focalroom-shared-review`）、独立的 PostgreSQL 数据卷（`focalroom-shared-review_pg-data`）、独立的网络。不共享 openyc 的任何容器、数据库、网络或凭据。两个项目在同一台服务器上运行但互不干扰。

## 常见问题

| 问题 | 解决 |
|---|---|
| Docker build 下载依赖慢 | Dockerfile 已配置 rsproxy.cn 中国镜像 |
| API 返回 502 | 检查 API 容器日志：`docker logs focalroom-shared-review-api-1` |
| 数据库版本不匹配 | 执行迁移：`docker compose run --rm api node apps/api/src/server.ts --migrate` |
| Web 显示旧内容 | 容器内文件权限问题：`chmod -R 644` 所有 src 文件后重建 |
| Cloudflare 返回 525 | SSL 模式改为 Flexible（Cloudflare Dashboard → SSL/TLS） |
| 文件权限 EACCES | tar 解压后 `chmod -R 644 apps/api/src/*.ts && chmod 755 apps/api/src` |
