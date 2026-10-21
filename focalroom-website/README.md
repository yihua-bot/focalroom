# FocalRoom 官网

中、英、日、韩四语摄影创作工作空间官网，正式域名为 https://focroom.cc。网站与桌面应用、团队 API 和数据库独立。无加入内测、申请表、邮箱收集或虚构评价。

本次内容根据用户提供的创作工作流、v3 最终蓝图和 2026-10-21 创作工作空间补充文档更新。Research & Moodboard 为规划；完整画布、外部调色往返和出版制作仍在开发，不能用愿景冒充现有下载版能力。

## 本地预览与验证

```sh
npm run build
npm run check
npm test
npm run dev
```

本地地址：http://127.0.0.1:1441/。构建只需 Node.js 和 Python 3，无依赖安装。

可复用本机已安装的 Playwright 做浏览器回归：

```sh
node "scripts/test-browser.mjs" "/absolute/path/to/playwright/index.mjs"
```

运行前保持本地服务器在 1441；测试只访问本机，不调用生产登录或 AI。结果与截图保存在忽略目录 qa/2026-10-21/。

## 源文件与生成文件

- templates/：中文首页、创作工作流、快速指南、隐私说明的 HTML 源模板。
- public/：按原字节复制到网站根目录的静态文件；Google 验证文件固定为 google409d83b744653349.html，不保留下载时附加的“(2)”。
- locales/messages.tsv：原有四语词表，保留已有修改。
- locales/workspace.json：本轮创作工作空间及产品状态的四语补充词表。
- scripts/site-content.mjs：共享词表、路由、翻译与结构化数据生成规则。
- scripts/build-locales.mjs：生成 16 个可独立抓取的静态页面、元信息、语言索引与托管规则。
- dist/：直接交付静态托管；其中 HTML、locales.js、page-data.js、sitemap.xml、robots.txt、llms.txt、_headers、_redirects、404.html 由构建生成，不单独编辑。
- dist/app.js、i18n.js、styles.css、languages.css、workspace.css、config.js：当前手写脚本、样式及真实入口配置。
- dist/user-guide.html：历史中文长篇手册，未声称提供四语翻译；托管规则标记 noindex，避免作为当前功能说明索引。
- .openai/hosting.json：历史 Sites 预览身份，保留供溯源；本次不创建或发布另一套 Sites，不修改现有域名路由。

中文首页为 /，其他语言为 /en/、/ja/、/ko/。每种语言均有 workflow/、guide/、privacy/。显式语言地址优先于浏览器和已保存偏好，不按 IP 自动重定向。旧 ?lang= 链接仍可在启用 JavaScript 时切换，但不进入 sitemap。语言链接在关闭 JavaScript 时仍可访问各静态版本；启用 JavaScript 后切换不清空本页演示序列和撤销栈。

## 搜索与生成式检索

各静态页面均生成独立 title、description、自指 canonical、相互对应的 hreflang（含 x-default）、社交分享元信息和 JSON-LD。软件功能描述仅列现有能力，FAQ 与正文一致，不伪造价格、评分、用户数或推荐。

四语 llms.txt 是可选的简明阅读索引，明确区分现有、开发中和规划中；它不是搜索排名或 AI 引用的必要条件。robots.txt 允许公开页面抓取，并列出 sitemap。Cloudflare 的托管爬虫策略仍需在上线后单独核对，不能仅凭源码保证抓取成功。

详细设计依据、验证证据、发布步骤和未完成事项见 SEO-GEO-UPDATE-2026-10-21.md。

## 下载与产品边界

现有下载包 FocalRoom-1.0.4-macos-arm64.zip 保持原字节：7,263,662 bytes，SHA-256 为 9c9f51a9f85430c7173640f5a2af369ac10531d11c6a356f0316b9a5d50fb6c6。已修正原首页重复 download/ 路径。

Info.plist 虽写 10.15，包内 Mach-O LC_BUILD_VERSION 的最低系统为 11.0，因此官网按 macOS 11+ 显示。这次不修改、重新签名或发布客户端，也不以包中配置代替签名/公证验证。

团队入口保持 https://team.focroom.cc/shared。官网不创建账户、不收集服务凭据；进入团队入口后适用产品服务规则。

## 演示与素材

logo 沿用用户现有资产；海岸组照为 AI 生成样片。演示使用预设选片建议，不访问真实 AI 或生产 API、不接收个人照片。选片仅在内存中，刷新重置；仅语言偏好以 focalroom.locale 保存，存储被禁用时仍可切换。素材来源见 ASSETS.md。

## 发布状态

Google Search Console 验证文件已接入构建与内容一致性检查，公开目标为 https://focroom.cc/google409d83b744653349.html。本地 Pages 验证该精确地址直接返回 200、53 字节原始内容；专用 200 代理规则避免默认去除 .html 后缀的外部跳转。加入验证文件后的官网 ZIP 和 SHA 清单已重新生成。

本轮公网检查该地址仍为 404，Cloudflare 凭据刷新仍返回 400，未上传。必须先部署，使公开地址返回原始验证内容，再回 Search Console 点击验证；不能把本地文件存在当成站点已验证。

本次本地修改未提交、未推送、未发布到 Cloudflare。现有 Wrangler OAuth 凭据失效（只读项目列表请求返回认证错误）。用户已明确确认本次生产发布。仍需完成本地 Cloudflare 授权并确认映射 focroom.cc 的原 Pages 项目；官方 OAuth 自动打开与手动链接均未完成授权，用户报告链接打不开或超时，当前停在发布前。禁止为了绕开授权而切换到旧 Sites、另建项目或修改 DNS。
