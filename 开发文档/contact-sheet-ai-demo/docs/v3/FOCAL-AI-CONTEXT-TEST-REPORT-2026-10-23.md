# Focal AI 创作上下文增量：验收报告

日期：2026-10-23，Asia/Shanghai。实现范围见 [实施文档](FOCAL-AI-CONTEXT-ITERATION-2026-10-23.md)。

> 后续增量：用户指定生产托管网关后，Keychain 账户预检通过，但两次真实任务分别在身份 GET 与图片 POST 超时阶段失败。详见 [真实验收报告](FOCAL-AI-HOSTED-REAL-ACCEPTANCE-2026-10-23.md)。以下“未调用模型/未外发”描述本报告原始离线验证阶段，不作为后续真实验收的结论。

## 验证对象

- 共享目录 `开发文档/contact-sheet-ai-demo/between-frames/`，HEAD `b108065` 加未提交修改。
- 保留已有 S1 统一会话和 v3.1 审稿工作；本轮没有 commit / push / PR / 生产发布。
- 下列计数来自本轮实际执行，而非用户此前汇报。源码指纹与原始日志见 `evidence/focal-ai-context-2026-10-23/`。

## 最终结果

| 检查 | 结果 | 原始记录 |
|---|---|---|
| Rust 默认全量 | 313 通过，17 跳过，0 失败；39 个结果段 | `rust-full.log` |
| 新增创作上下文定向 | 8 通过，包含在上述总数内 | `creative-targeted.log` |
| 前端全量 | 50 文件、333 项通过 | `frontend.log` |
| 类型 / Lint / 契约 | 通过 | `typecheck.log` / `lint.log` / `contracts.log` |
| Rust fmt / 严格 Clippy | 通过；Clippy 含全部 targets 及 integration-harness | `fmt.log` / `clippy.log` |
| 后端浏览器全套（含账户） | 13 条通过；最终配置整套重跑 | `integration.log` |
| 统一会话、修订与记忆 | 3 条通过 | `focal-journeys.log` |
| v3.1 审稿集成 | 2 条通过 | `editorial.log` |
| 登录、订阅、账单定向 | 3 条通过，已包含在上述 13 条中；独立后端及 Web 服务 | `account.log` |
| 浏览器内存预览 | 18 条通过 | `preview.log` |
| 托管 AI 的隔离 PostgreSQL 测试 | 7 项通过；回环假上游 | `hosted-ai.log` |
| 桌面前端生产构建 | 通过，包含在 Tauri 构建前置步骤中 | `native-build.log` |
| 隔离原生 debug 检查包 | 构建通过，74.53 MiB；未签名、未启动 | `native-build.log` / `native-bundle.json` |
| diff 空白检查 / 验证脚本语法 | 通过 | `hygiene.log` |
| 真实付费模型 / 原生 GUI / 新候选签名公证 / 生产部署 / 远端 CI | 本轮未执行，不计通过 | — |

浏览器共 36 条不同流程；不是 36 次真人验收，也没有真实支付或邮件发送。新真实模型验收为新增的 1 项 ignored，另外 16 项沿用原有需要外部资源的跳过测试；跳过不表示功能通过。

## 新增 8 项后端回归覆盖

1. 授权范围内画布、版本完整决定/序列、外部版本和出版布局读取；输出不含文件路径，未观察数据不冒充视觉证据。
2. 跨任务分支及三轮父子关联；不能通过新任务直接编辑上一任务的草案。
3. 正式基线上的删除意图在后续草案中保留；工具重放只有一个子方案；独立采纳后以新的正式作品为根。
4. 父版本过期、换账户、换端点、缩窄照片范围均拒绝且不残留半个任务；选片草案也复制真实条目。
5. 上下文绑定被篡改时拒绝恢复；旧任务不被自动追加新工具权限。
6. 偏好建议不自动成为证据；人工编辑确认、重复确认、范围/账户限制、排除/恢复及修正不改变正式作品。
7. 本地方法来源可核对，未知查询不产生虚构条目。
8. 300 张生成照片 → 真实 AgentEngine → 三份各 20 张画布，默认预算内完成；完整回执、剩余协议配对、重启、正式状态及源图哈希保持。

第 8 项使用受控 Provider，而非真实摄影模型。它证明结构、权限、预算与持久化，不证明三条叙事的审美差异或摄影专业质量。

## 浏览器人工可查看的证据

- `browser/revision-history.png`：两份真实草案与修订来源，重启后父方案未变化。
- `browser/knowledge-and-memory.png`：原创方法原文与编号，偏好保存后显示用户编辑的准确文本。

截图中的 SIMULATED 方案来自仅编译进测试驱动的固定 Provider，不是原生生产模型结果。原生包不包含测试驱动。

## 失败、修复与重跑

- 300 图用例暴露按工具名压缩会覆盖不同分页或方案、重复布局参数超上下文预算。改为完整工具批次归档、保留成对协议及持久完整回执，没有增大预算。
- 修订分支曾未正确保留删除基线；另存独立作品后又沿用旧根。增加 tombstone/根语义回归并修复。
- 偏好在重启后曾显示模型建议原文，而非人工编辑后的实际保存文本。投影改读人工命令；更正/停用显示已更新状态。
- 原有导出用例要求界面显示 SHA，与此前产品文案清理冲突。改为核验实际导出回执和文件字节，未把技术用语加回正式 UI。
- `account-experience` 混在无后端预览配置中；其 Web 用例还依赖未启动的 1424 服务。移至集成配置并启动 Web，三条通过。
- 旧预览用例假设访客能直接打开 AI；更新为验证登录原因、取消、选区保持与焦点返回。已登录的对话能力由真实后端旅程覆盖。
- 项目列表测试仍使用“示例项目”旧名；项目卡片增加以项目名为可访问名称，更新导航断言。同步单元测试时一度误用了 Playwright 的 `exact` 参数，类型检查阻止了构建，移除后全部重跑通过。

保存初次失败日志，最终结果不覆盖或隐去失败过程。Playwright trace 留在工作区 `test-results/`，之后运行会替换对应目录；耐久证据以本目录日志和截图为准。

## 隔离原生构建

检查包：`between-frames/src-tauri/target/debug/bundle/macos/FocalRoom Context Review.app`。

- identifier：`studio.focalroom.contextreview`。
- scheme：`focalroom-context-review`，不替换正式登录协议。
- updater：关闭；不生成更新产物。
- 版本沿用当前代码 1.1.5；这不代表 1.1.5 已发行。
- 使用 `--no-sign`，不提交 Apple 公证，不上传，不可当作正式安装包。
- 未启动，不访问正式用户数据库或钥匙串，不声称原生 UI 验收通过。

## 尚未完成与执行条件

真实模型 300 图及后续修改入口已编译，默认跳过。本轮环境未设置端点、模型或 API 凭据，未访问供应商、未上传 LUMIX。运行所需安全配置路径已向用户询问，密钥不应再次发到聊天或写入仓库。外发上限和批准机制见实施文档第 5 节。

当前还不能宣称所有非真人任务完成：真实模型端到端、托管账户额度联调、新候选正式签名公证与生产部署仍未验；完整 v3.1 三方合并、历史恢复、云端及专业色彩也是独立待办。8 篇原创札记不等于完整摄影知识库。真人摄影师盲评继续单列。

新测试已接入 `scripts/verify-all.sh`，并显式清空三家供应商环境密钥；尚未提交和触发远端 CI，本地通过不冒充 CI 绿。

## 复现入口

在 `between-frames/` 执行，下列浏览器命令顺序运行；原生构建必须在浏览器套件退出后执行。使用隔离临时库，测试驱动清空模型密钥。

```sh
npm run typecheck
npm run lint
npm run contracts:check
npm test -- --maxWorkers=2
cargo test --manifest-path "src-tauri/Cargo.toml"
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --all-targets --features integration-harness -- -D warnings
cargo build --manifest-path "src-tauri/Cargo.toml" --features integration-harness --bin m1-harness
CI=true npx playwright test --config "playwright.integration.config.ts"
CI=true npx playwright test --config "playwright.focal-agent.config.ts" "e2e/focal-agent.spec.ts" "e2e/focal-creative.spec.ts"
CI=true npx playwright test --config "playwright.editorial.config.ts"
CI=true npx playwright test --config "playwright.config.ts"
node "apps/api/test/run.mjs" --test "apps/api/test/hosted-ai.mjs"
```

集成驱动默认取 `src-tauri/target/debug/m1-harness`；若用 target-v1 构建，浏览器执行环境对应设置 `CARGO_TARGET_DIR=target-v1`。不能运行旧驱动后将 unknown-field 错误归到新产品代码。
