# ADR 0043：桌面浏览器登录使用一次性授权码与 PKCE

日期：2026-10-18；2026-10-19 补齐账户资料。范围：M4 桌面账户和现有共享上传入口；不改变项目权威、模型授权或 M3/M5 验收门槛。

## 问题

20a90ec 把网页 CSRF 当作桌面 Bearer，UI、深链插件、持久化和共享调用尚未接通。长效会话不应通过自定义协议 URL 返回。浏览器已有会话也需要明确确认，而不是只在验证码组件内回跳。

## 决策

- 桌面 Rust 用系统 CSPRNG 各生成 32 字节 state 和 verifier。API 保存 state 摘要和 S256 challenge；事务最多 15 分钟。
- 现有网页 Cookie/CSRF 认证之后，统一显示账户确认页。明确确认才产生 60 秒、单次使用的授权码；只回传 code/state，固定 focalroom://callback，不接受自定义 redirect_uri。
- 桌面保留 verifier 并通过 HTTPS 兑换；API 同一事务锁定源会话和授权记录、校验 PKCE、消费授权码及创建独立桌面会话。源浏览器会话撤销或过期后不能再兑换。桌面会话沿用现有 24 小时有效期，浏览器登出不自动撤销已颁发的独立桌面会话。
- 登录待办与会话写入 macOS Keychain，服务名隔离到 app identifier。GUI 只接收账户、来源、等待和错误状态，不接收 verifier/token，不写 SQLite/localStorage，不通过 shell 或命令参数传递秘密。
- 监听插件的 URL 列表事件，并读取冷启动 get_current；重复投递去重。严格验证协议、host、path、字段数、state、过期和来源。打开浏览器失败撤销本机待办；不确定的兑换结果不重放，重新发起。
- 共享操作从原生账户服务读取凭据，来源必须匹配；移除 email/password 参数。离线状态不允许共享，但允许清除本机凭据并说明服务器撤销结果未知。
- 正式构建固定 https://team.focroom.cc。仅 integration-harness 特性允许 FOCALROOM_LOGIN_REVIEW_ORIGIN 指定精确 HTTP 回环；独立 review app identifier/数据库/钥匙串用于验收，不能分发为正式包。
- 49fcc65 中的 Google 服务端验证读取提供商 tokeninfo 并核对 audience/已验证邮箱，资料由 userinfo 获取；本轮补严格输入、受限头像 URL、资料立即返回及本地协议回归。前端入口默认关闭，只提交 access_token，不提交自报邮箱。真实 OAuth 配置、生产 CSP 和真实 Google 登录仍须单独验收。所有登录方式统一走桌面账户确认。
- 双端复用 AccountAvatar，缺失或加载失败显示昵称首字。桌面缓存使用账户/URL 摘要文件名、允许的 Google HTTPS 来源、无跳转且不带会话凭据的请求、有界读取、实际 PNG/JPEG/WebP 解码与像素限制；原子写入私有文件。只将已验证本地位图以 data URL 返回桌面。Web 不接收 data URL，远程图片限制在相同来源。离线显示已缓存身份和头像，保持 authenticated=false。

## 数据迁移与限制

新增 PostgreSQL 0013_desktop_login.sql；0014_user_avatar.sql 在已有提交中创建，本轮补登记及读写链路。旧 SQL 不改写，部署时使用迁移角色；应用角色仅 DML。现有生产部署必须另行审批和发布，桌面新版本需要同时匹配新 API/Web。

macOS 钥匙串有实测；Windows/Linux 当前明确不支持这条原生登录链路。退出断网只删除本机凭据，远端会话继续有效到撤销或自然过期。没有新增自动刷新 token 或跨服务凭据迁移。
