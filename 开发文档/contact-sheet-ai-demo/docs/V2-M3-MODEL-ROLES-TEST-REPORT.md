# M3 同端点模型分工验收（2026-10-17）

结论：FocalRoom 已可分别冻结策展主循环与图像观察型号，沿用同一 Responses 服务端点、授权和累计预算。只证明本地工程链路，不证明两个实际商业型号的成本优势、专业摄影质量或 M3 整体完成。决策见 [ADR0039](ADR/0039-m3-same-endpoint-model-roles.md)。

## 行为与功能验收

| 场景 | 结果 |
|---|---|
| 两型号执行 | 本地 HTTP 返回真实协议响应，4 次策展请求不含 input_image；1 次观察请求处理 2 张生成图，形成 2 项 review_ready 草案，正式项目不变 |
| 独立能力 | 策展主循环可只具备工具调用/遵循指令能力；观察角色独立要求视觉/结构化/遵循指令。不借用主模型能力，也不自动代传 |
| 可信配置 | 两角色能力文件分别匹配端点/型号，有界、冻结；缺失、损坏、错配、只配置观察文件和默认复用均已验证 |
| 图像授权 | 卡片显示实际观察型号，并说明观察事实交给同一服务的策展型号；旧策展型号同意不能为另一观察型号授权，显式错配在任何 HTTP 前拒绝，隐式错配零图像外发 |
| 恢复 | 检查点保存两个型号和有效能力摘要；重开后任一角色/能力变化拒绝续跑，密钥轮换允许。记录不含密钥或明文端点 |
| 缓存与检索 | 使用观察型号和既有端点/Prompt 缓存键；新任务换策展型号可复用观察，换观察型号不误命中；事实检索跟随观察来源且不上传 |
| 预算与审计 | 两角色使用各自费率，合计到原任务预算；图片预算不足在观察 HTTP 前暂停。账本 modelRole/modelId 及累计费用一致，未知传输不免费重传 |
| 端点限制 | 不同端点注入以 MODEL_ROLE_CONFIG_INVALID 拒绝；本切片没有跨服务 fallback |
| 界面模式 | Agent 入口使用中性名称，实际 Mock/真实配置在面板说明；本地侧栏明确图像上传需授权 |

新变量为 BETWEEN_FRAMES_AI_OBSERVATION_MODEL / BETWEEN_FRAMES_AI_OBSERVATION_CAPABILITY_PROFILE；部署方式见 [能力配置](MODEL-CAPABILITIES.md)。主 Prompt m3-openai-9，Mock m2-mock-2，视觉 Prompt/缓存仍 m3-vision-2。旧 Prompt 已执行任务需新委托，不改写旧任务。

复用既有 Adapter、Router、预算/同意、缓存和检查点职责；能力解析共用严格入口，只新增角色配置与绑定，没有新表、迁移、生产依赖或通用供应商框架。

## 自动化验证

- verify-all.sh --skip-keychain 最终 exit=0：190 桌面 Rust（含新增 9 项）、13 本地云 HTTP/进程测试、183 Vitest（含新增 3 项）、16 预览 E2E、6 后端 E2E；契约、类型、Lint、fmt 与严格 Clippy 均通过。
- 13 项 ignored 和钥匙串写删 1 项 filtered 不计通过，真实模型/专业素材/性能 ignored 不冒充已运行。
- 最终 UI 文字修订后，再次通过类型/Lint、183 单测、16 预览与 6 后端旅程。Rust 核心此后未修改，不重复计数。
- 根/桌面/Web 构建、Web E2E、独立角色界面和最终桌面包的结果见下节与 evidence 日志。

原始记录在 evidence/v2-m3-model-roles/：verify-all.log 为整套通过日志；verify-before-final.log 保留首轮严格检查失败；post-ui-checks.log 保存最终前端/两个 E2E 套件复验输出。

失败已如实保留：新增前端用例最初按不存在的设置文案定位，23 pass/1 fail；首轮严格 Clippy 指出测试 MutexGuard 跨 await，改为词法作用域；入口改名后 App.test 仍使用旧名称，182 pass/1 fail，已同步定位并完整复验。相关摘要见 initial-frontend-failure.txt、post-ui-first-failure.txt，未放宽业务断言或增加重试掩盖错误。

## 独立角色界面验收

ui-qa.mjs 使用最新编译的真实 harness、独立临时 SQLite 和 200 张派生测试素材，实际读取运行时两份能力/型号投影；不是拦截替换 aiConfig。端点固定回环，两个型号为测试名称，不调用模型，不读取 LUMIX 或用户项目。

1440×960 / 1280×800 下检查实际观察收件人、事实转交策展型号、卡片可操作与无横向溢出。主动项目授权和全局禁止落入真实临时库，授权后任务数为 0，正式项目和 200 素材 hash 前后一致。页面无外部请求/脚本错误；记录为 ui-qa.json、roles-consent-1440.png、roles-consent-1280.png。

## 构建与源记录

最终根/桌面/Web 构建和 1 项 Web E2E 通过；两种尺寸的独立角色界面在入口文案修订后再次通过。最终 FocalRoom Review.app debug 包构建成功，59.19 MiB（62,066,584 字节），Info.plist 名称与 studio.betweenframes.m1reviewfixes 标识正确，包内图标摘要与源 ICNS 一致；NaturalLanguage 仍弱链接。日志与 final-verification.json 保存结果。原生窗口未启动，用户数据库未打开，钥匙串写删、真实 Provider、线上部署、支付和 Git 提交推送均未执行。

本轮开始时 90 张验收 PNG 有独立字节备份，测试结束将 29 张被改写的截图按备份恢复，90 项摘要全部核对一致；新截图单独落在本证据目录。source-sha256.json 记录本轮最终 252 项源码、配置与品牌资源摘要，结束时全部核对一致。

## 剩余问题

M3 仍 partial：尚缺两角色在实际商业模型上的成本/延迟/质量对比、质量驱动自动选模、全图库后台观察、多 Adapter 与跨服务文本/图像授权 fallback、云作者记忆/真实 Creator 隔离。>=30 个创作任务、专家/竞品同题和 >=10 位摄影师的真实质量门槛仍未关闭。本轮没有增加真人、专业质量或生产云证据，不提升既有 17 pass / 7 partial 总账层级。
