# FocalRoom 官网客户端首发检查

同日审计复核更正：公开仓库[yihua-bot/focalroom](https://github.com/yihua-bot/focalroom)及根workflow实际存在，本机没有remote不等于没有远端；已读run39224541729日志，确因Web预览/Shared入口断言不符失败，桌面构建跳过。最新交接与修复顺序见[第11节](DEVELOPER-HANDOFF-2026-10-19.md#11-外部审计逐项核对与第一批修复单)，不再沿用下方“缺少根workflow”的旧判断。

最新增量：账户AI本地实现已补（[本地报告](V2-HOSTED-AI-TEST-REPORT.md)、ADR0044）；随后用户批准生产发布，API/Web与0014/0015已部署，真实验证码Resend delivered、网页登录/刷新/API重启保持/退出通过，API迁移凭据已隔离。From暂为已验证openyc.vip，用户已确认邮件进入收件箱。DeepSeek真实调用未执行、服务关闭/额度0；正式客户端/公证/官网分发和容量灾备门槛仍开。以 [生产报告](V2-HOSTED-AI-PRODUCTION-DEPLOYMENT-REPORT.md) 为最新事实。下方保留先前只读快照，其中“尚未部署/无邮件”不是最新状态。

日期：2026-10-19。范围：现有代码、签名身份、官网静态文件与腾讯云FocalRoom独立栈只读核对。没有构建新的正式客户端、修改生产配置/数据库、发送验证码或执行公证/官网发布。

## 当前结论

可以进入首发收尾，当前评审包不能直接作为官网下载的生产客户端。首发不需要先实现所有后续平台/付费功能，但下面7项发布门槛仍需完成。以下区分代码事实、本次生产只读结果与尚待验收，不能继续沿用旧“只剩公证”或“15/18”计数。

### 1. 制作并验证正式分发产物

2026-10-19补查：本机Developer ID身份再次确认有效；分别运行 `notarytool history --keychain-profile FOCALROOM_NOTARY` 与 `BETWEENFRAMES_NOTARY` 均退出69，提示当前默认钥匙串找不到对应条目。这不排除另存于自定义钥匙串，但目前没有收到其路径。当前package.json、scripts/、ops/和主tauri.conf.json也未找到这两个预设名，不能声称项目已配置可用profile。证据见 `evidence/production-release-readiness-20261019/notary-profile-check.json`。可由用户在本机终端运行store-credentials，省略--password并在安全提示中输入专用密码；本轮没有创建/改写钥匙串或上传应用。

- 本机Keychain实测存在有效的Developer ID Application签名身份，Team KXSFCZF4S3。无需重新申请证书。
- 最新产物仍是studio.focalroom.loginreview的debug包；严格codesign资源检查失败，属于linker ad-hoc，不能拿历史已签名包说明它已签名。
- 默认tauri.conf.json为0.1.0、studio.betweenframes.desktop，bundle.targets仅app。需要冻结最终源码、使用正式标识和生产服务、关闭integration-harness及回环评审覆盖，构建release并制作DMG/ZIP。
- 需使用Developer ID完成运行时加固/时间戳签名、公证、staple和Gatekeeper检查，并记录产物SHA256。没有在本项目找到可复用的公证配置，凭据可用性未核实，需profile名或ASC API Key路径，不能直接认定用户没有公证凭据。
- Apple参考：[Developer ID](https://developer.apple.com/developer-id/)、[分发包装与独立Mac测试](https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution)。Tauri参考：[分发](https://v2.tauri.app/distribute/)。

### 2. 将匹配的API/Web版本升级至生产

本次SSH只读结果：API/Web容器running、PostgreSQL16.14 healthy，WebOrigin正确，当前登记迁移0001/0002/0006/0008/0009/0010/0011/0012/0013，**缺0014**。此前报告中的“生产仅到0012”已不是当前事实；但也未达到当前源码所需的0014头像结构。

需对最终API/Web镜像和迁移做一致发布，先备份再由迁移角色执行0014；核对桌面PKCE握手、头像、退出和实际共享业务。不能只发布客户端，不能直接对SQL目录全部执行。此轮没有执行这些生产变更。

### 3. 打通真实用户登录

生产API的FOCALROOM_RESEND_API_KEY当前为空；用户数0、项目数0。页面HTTPS200仅证明页面可达，不证明用户能收到验证码。Google Client ID已配置，但真实授权、网页开关与生产CSP未通过验收，不能当作可用的替代入口。

首发可暂关闭Google，完成发件域/邮件服务配置，并用指定真实邮箱验证发送、收取、错误码、重发、网页会话、返回正式桌面、重启、退出，以及实际共享。当前本地模拟邮件验收不代替这一项。

### 4. 确定并实现普通用户的AI配置方式

源码src-tauri/src/agent/openai.rs默认https://api.openai.com/v1与gpt-4o-mini；服务地址/型号读环境变量，API Key可从Keychain读取。AIPanel当前只有Key输入，没有端点/模型选择。用户从Finder打开的客户端不会继承开发机终端的DeepSeek环境；未配置时仍是Mock流程，不等于自动视觉选片。

二选一：

- 自带Key首发：补齐供应商/端点/模型与能力配置的用户入口、安全保存和重启行为，在干净用户环境完成真实视觉任务。
- FocalRoom提供AI：需实现受账户认证、用户额度和预算控制的服务端调用链路及撤权/失败处理；已有云Worker评论汇总不等于桌面策展官方代理。不能把公司共用供应商密钥放进安装包。

该选择直接影响首发工程范围，尚未由用户决定。

### 5. 收口公开使用前的生产运维底线

本次读取根分区剩余2,042,032,128字节，约1.90GiB；当前私有预览仍在服务器磁盘。发布前应完成扩容或对象存储、上传/存储配额和满盘保护，并确定可执行的备份保留、独立恢复与基本告警。

API运行环境仍有FOCALROOM_MIGRATOR_DATABASE_URL。应将DDL凭据隔离到一次性迁移任务，不长期放在对外API容器中。历史暴露的服务器密码需要管理员安排轮换；本轮未读取输出或改变该密码。

### 6. 完成官网分发入口

本次系统curl验证focroom.cc与team.focroom.cc均HTTPS200；官网在线config.js的workspaceUrl仍为空，首页未找到DMG/ZIP等下载链接。不能把“网站已上线”当成“软件下载链路已上线”。

需要接入真实HTTPS工作区，四语下载按钮、系统/架构要求、版本/发布日期/更新记录/校验值，并配置实际下载对象和HTTPS链接。安装包与用户私有预览应使用独立访问策略。

Cloudflare Pages单文件上限25MiB；最终安装包若超过限制，应改走R2或其他下载存储，官网只保留链接，不能盲目放进Pages静态目录。参考：[Cloudflare官方限制](https://developers.cloudflare.com/pages/platform/limits/)。最终release包大小尚未测定，不能从100.54MiB debug包推断release压缩大小。

### 7. 从官网下载的全新安装验收

需用最终签名/公证产物完成“HTTPS下载→Gatekeeper默认设置打开→安装→全新项目/导入→真实AI→浏览器登录→正式scheme回跳→共享→重启/退出”，并用旧数据副本验升级/备份；不能拿同机独立review标识和临时PG证明正式标识、用户环境与生产已验。

建议首包明确限定macOS Apple Silicon。Intel、Windows/Linux在完成对应构建/原生能力验收之前不提供下载；不能以Tauri跨平台框架替代具体实现。

## 不必挤进首个受限预览版的事项

- Google登录：邮件登录真正可用时可保持关闭。
- Intel/Windows/Linux和自动更新：可后补，首版明确平台与手动更新方式。
- M3专业质量与M5真人协作门槛：仍未完成，但可通过明确标记的预览发布招募真实试用，不能宣称所有M0–M5或专业摄影质量验收通过。
- 远端CI已存在但当前已核对run失败：根workflow为`.github/workflows/betweenframes-v2.yml`、macos-14与版本文件，Web预览/Shared测试目标不符导致后续跳过。产品子目录另一份旧ci.yml的Node22/健康路径/吞错问题另计，不混称根workflow。修复并取得新远端绿灯前，不称release gate可用。

## 用户需提供/决定

1. AI交付方式：自带Key或FocalRoom提供。
2. 公证Keychain profile或ASC API Key本地路径；不要在聊天粘贴私钥/密码。
3. 真实验证码测试接收邮箱及现有邮件服务配置位置；若没有，需要配置服务与发件域。
4. 最终公网发布前审阅版本、产物摘要、具体生产变更与下载链接；本次只读检查不执行上线。

只读证据：[生产状态](evidence/production-release-readiness-20261019/production-readonly.json)、[线上官网配置](evidence/production-release-readiness-20261019/live-website-config.js)。本地功能测试与分阶段原生证据见[登录与账户报告](V2-M4-DESKTOP-LOGIN-FIX-TEST-REPORT.md)。
