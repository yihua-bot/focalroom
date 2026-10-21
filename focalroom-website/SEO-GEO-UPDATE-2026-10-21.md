# FocalRoom 官网 · 内容、多语言 SEO / GEO 更新

日期：2026-10-21。范围：独立官网源码和静态发布文件。当前为本地待发布版本，不代表 focroom.cc 已更新。

## 内容依据与产品边界

- 用户本轮的“影像创作工作流”说明：以项目承载反复往返的构思、选片、视觉编辑、叙事、评审与呈现。
- 工程宝中的 FOCALROOM-V3-FINAL-MASTER-PLAN.md，以及产品仓库 docs/v3/CREATIVE-WORKSPACE-ADDENDUM-2026-10-21.md。
- 产品状态以代码核对结论为限：现有选片/序列/私密评片可介绍；完整 Research & Moodboard、叙事画布、LR/C1 往返、共享画布、专业出版与色彩输出不得宣传为当前安装包全部可用。

保留原品牌、纸感配色、海岸样片、预设交互演示、原有四语词表改动和现有下载包。新增工作流页及首页创作循环；FAQ 直接回答适合谁、是否替代调色软件、哪些能力尚在开发。联系信息使用用户确认的 sliqiang85@gmail.com 和 Instagram YIHUAPHOTO。不增加加入内测或表单。

## 已实现的 SEO

1. 以静态 HTML 生成四语各四页：主页、workflow/、guide/、privacy/，共 16 个独立地址。中文根路径 /，其他语言 /en/、/ja/、/ko/。正文不依赖 JavaScript 才能翻译。
2. 每页独立 title、description、自指 canonical；完整双向 hreflang 含自身与 x-default，全部使用正式域名绝对地址。
3. 语言选择提供普通链接，关闭 JavaScript 仍可访问；不按 IP 或浏览器偏好改写明确语言地址。保留旧 ?lang= 参数的交互兼容。
4. Open Graph、Twitter 分享元信息与语言页一致；语言动态切换时 URL、canonical、标题/描述、JSON-LD 同步，演示状态不丢失。
5. sitemap.xml 只含 16 个 canonical 地址，不把查询参数、旧手册或下载文件重复列为内容页。robots.txt 声明 sitemap 并允许抓取公开内容。
6. 提供 404.html，避免 Cloudflare 将未知路径作为 SPA 首页返回；历史中文手册通过 _headers 的 X-Robots-Tag: noindex 排除。旧 .html 入口的 _redirects 已备妥，真实响应须上线后验证。
7. 修正首页 download/download/ 重复路径。保留现有 1.0.4 ZIP 的字节与 SHA，不重新构建客户端。

## 已实现的 GEO

- 用可见正文回答产品定义、核心工作、目标用户、与像素处理软件的关系、现有与规划能力、资料来源与创作者决定权。
- 四语分别表达本地语言中的照片选片、创作编辑、叙事、评审与摄影书方向；不堆砌关键词或生成重复入口页。
- 生成 Organization / WebSite / WebPage / SoftwareApplication JSON-LD；主页 FAQPage 的问题和答案逐项来自可见 FAQ，不编造价格、评分或用户证明。FAQ 标记并不保证获得 FAQ 富媒体展示。
- 四语 llms.txt 提供可选的简明产品索引，列明当前、开发中、规划中和各语言正文链接。未把它宣称为生成式搜索的标准或必要文件。
- 正文与结构化数据共同生成，避免“用户看到规划，机器读到已上线”的矛盾。

Google 的 AI 搜索仍依赖可抓取、可索引、有用且可信的内容，不要求专用 AI 文件或特殊结构化数据；收录、排名和生成式引用均无保证。本轮不修改 Cloudflare 账户的训练爬虫策略，不提交未经授权的站长平台操作。

## 加载与分发修复

样片保留原 PNG，并生成 1536×1024 WebP 供现有 CSS 使用：3,038,640 → 375,370 bytes，单资源减少 87.6%。构图与显示尺寸保持；原始素材来源未改变。不引入第三方字体、跟踪脚本或翻译 API。

下载 ZIP 为 7,263,662 bytes，SHA-256：9c9f51a9f85430c7173640f5a2af369ac10531d11c6a356f0316b9a5d50fb6c6。Info.plist 写 10.15，但实际 Mach-O LC_BUILD_VERSION 最低系统为 11.0，因此页面改为 macOS 11+。本轮没有对旧包重新执行签名/公证验收，没有给官网新增“本轮已公证”声明。

## 验证结果

- npm run build：369 个集中词条，16 个静态页面生成。
- npm run check：四语正文、唯一 H1、重复 ID、canonical/hreflang、schema 与正文、sitemap 和站内资源检查通过；464 个 HTML 本地引用。
- npm test：7 项语言测试通过，含显式路径优先、旧参数兼容、存储受限、保留文档与 URL hash。
- Chromium headless：40 项浏览器检查通过。四语首页 1440/768/375，工作流页 1440/375 无横向溢出；全部 16 页关闭 JavaScript 可读；草案→采纳→四语切换→撤销保持状态；元信息随语言切换；移动菜单和真实 HTTPS 工作区链接通过；无页面异常、控制台错误或站外自动请求。
- 浏览器结果与 20 张全页截图在 qa/2026-10-21/，该目录按既有规则忽略；另保留首屏截图作视觉检查。
- 原生 CUA 预览管道不可用，本轮使用真实 Chromium 自动化；不称为 Safari、物理手机、屏幕阅读器或完整 WCAG 验收。
- 本地 Pages/workerd 另通过 15 项 HTTP 检查：语言页面、5 条旧入口重定向、自定义 404、历史手册 noindex、robots/sitemap MIME 与下载入口。首次端口占用已换到独立 24581；本地运行时兼容日期为 2026-02-19，不冒充真实 CDN 验证。
- 原生应用、产品生产 API、数据库和 DNS 未修改；没有真实模型或产品登录请求。

## 发布阻塞与接手步骤

只读运行 wrangler pages project list 时，现有 OAuth token 刷新返回 400，随后提示需要有效授权。因此未获取到正式 Pages 项目清单，未发起部署、未更换域名或另建项目。历史 Sites 私有预览身份保留，不作为正式 Cloudflare 发布的替代。

用户已在本轮明确确认发布。再次只读核验仍返回旧凭据刷新 400，因此已启动最小权限（account:read、user:read、pages:write）官方 OAuth 流程，自动打开未生效并超时，手动授权链接用户仍报告打不开或已超时。当前没有有效发布凭据；没有重复请求发布批准。

发布产物：artifacts/focalroom-website-20261021.zip；完整 dist/ 文件 SHA 清单：artifacts/release-manifest-20261021.json。该 ZIP 为官网静态目录，包含既有客户端下载 ZIP；不含源码、.git、.openai、qa、凭据或产品数据库。解压到独立空目录或直接使用本项目 dist/。

1. 在用户自己的本机终端执行 wrangler login 并完成 Cloudflare 官方浏览器授权；这与本会话自动打开未生效的流程分开。若 CLI 登录持续失败，可在 https://dash.cloudflare.com/ 打开原 Pages 项目，核对其自定义域名为 focroom.cc，再上传现成静态 ZIP；无需另建项目或改 DNS。不要把 token 或密码发进聊天。
2. 只读执行 wrangler pages project list，确认 focroom.cc 对应的既有 Pages 项目及其生产分支。不要新建项目来绕过授权问题。
3. 本批源码没有提交或推送；用户已明确确认本次生产官网发布。若另需提交官网源码，单独说明实际范围；历史私有 Sites 发布确认不作为本次公开发布批准。
4. 获批后从当前官网目录执行 wrangler pages deploy dist，显式指定已核对的 project-name 与原生产分支；不修改 DNS、团队服务或客户端文件。也可在原 Pages 项目中直接上传该静态目录。
5. 核对真实响应：四语各页 200、错误路径 404、旧入口跳转、中文手册 noindex header、robots/sitemap MIME 与正文、正式下载文件 SHA 和全部 canonical。若 Cloudflare 注入 robots 规则或拦截搜索爬虫，作为单独配置问题核对，不擅自关闭安全设置。
6. 在站点管理员自己的 Search Console / Bing Webmaster Tools 中验证所有权并提交 sitemap；面向韩国的站长工具是否接入由管理员决定。没有所有权凭据时保留待办，不写为已提交或已收录。
7. 留存部署 ID 和上一个生产版本以便回滚；回滚官网不操作腾讯云数据库或客户端签名包。

## 依据

- [Google：AI Features and Your Website](https://developers.google.com/search/docs/appearance/ai-features)
- [Google：Localized Versions / hreflang](https://developers.google.com/search/docs/specialty/international/localized-versions)
- [Google：Structured Data Guidelines](https://developers.google.com/search/docs/appearance/structured-data/sd-policies)

以上官方资料于本轮查阅。没有采用隐藏内容、伪造评价、关键词堆砌或保证推荐排名的做法。

## 本轮最终状态

官网代码、静态构建、7 项语言测试、40 项浏览器检查及 15 项本地 Pages HTTP 规则检查均已完成。用户已明确授权公开发布；实际上传未开始，阻塞为 Cloudflare 有效登录凭据缺失。没有声称公开站已更新，也未执行 git commit/push。静态 ZIP 和逐文件 SHA 清单已生成并做解压校验。

## Google 验证文件增量

用户随后提供 google409d83b744653349 (2).html。按文件正文确定正式文件名为 google409d83b744653349.html，原始 53 字节保存在 public/，构建复制到 dist/，不翻译、不包装 HTML、不加入内容 sitemap。新增静态一致性检查，检查源文件、构建文件与指定验证正文一致。

Cloudflare Pages 专用规则将精确 .html URL 以 200 代理到本域静态资源，避免默认扩展名跳转。实际本地 Pages GET 返回 200、无重定向、正文与附件逐字节一致；build/check 和 7 项语言测试通过。未因本次静态验证文件增量重跑原有 40 项浏览器旅程。

公网 https://focroom.cc/google409d83b744653349.html 实测仍为 404；Cloudflare 项目列表认证仍失败，未部署或完成 Google 账户中的验证。发布包与 SHA 清单已刷新，包含该文件。部署成功后先核对公网正文，再回 Search Console 点击验证。
