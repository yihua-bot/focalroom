# v2 M3 模型能力配置验收（2026-10-16）

本切片完成模型特定能力配置与请求门槛，M3 整体仍为 **partial**。接续实际模型身份/缓存事实检索，不扩展云共享、支付或新的 AI 服务。既有云协议和模拟评审源码核对 19 文件 SHA-256 一致；用户未提交修改、未跟踪 Dockerfile 均保留。

## 交付与行为

1. Responses 协议不再默认赋予任意模型全部能力。内置目录精确匹配已核对的端点/型号；未知模型或代理须有可信部署声明。相同模型名、近似主机名、HTTP 降级均不能继承官方端点能力。
2. 新增 `BETWEEN_FRAMES_AI_CAPABILITY_PROFILE` 本地 JSON 配置：16 KiB 上限、版本/字段/能力白名单、端点摘要和模型精确绑定。损坏、不匹配或不可读的显式配置直接拒绝，不回退到内置目录；错误不记录文件路径、正文或密钥。来源分为官方文档、本项目协议冒烟、部署声明，不冒充质量、区域、费用或上传授权。
3. 工具编排、视觉、结构化任务按全部所需能力过滤，含图像输入额外要求视觉能力；Router 与 Adapter 共用要求。直接 generate/observe 也不能绕过。缺能力的图像请求不会发送，也不扣其上传配额；此前已经发生的合法文本调用仍照常计量。
4. 首次执行绑定有效能力集合摘要；能力配置变化后，恢复在待执行工具/视觉调用前拒绝。凭证不参与摘要，可轮换密钥。缺少能力摘要的旧绑定仍能读取，但须新建委托，保留旧记录，不回填猜测。
5. 委托面板展示配置错误，错误配置、读取失败或不支持的图像预算不能创建委托；纯文本模型可运行无图编辑。真实无图任务不再误标 Mock。授权卡打开后配置失效，停止继续授权，但“禁止云端图像”仍可用。IPC 拒绝缺少能力字段的旧格式，避免前后端版本混用时默认为允许。

能力解析独立于 Router/网络适配，复用组合能力要求；只支持当前四项协议能力，未新增配置框架、数据库迁移、依赖或自动模型探测。设计见 [ADR 0034](ADR/0034-m3-model-capability-profiles.md)，部署步骤见 [MODEL-CAPABILITIES.md](MODEL-CAPABILITIES.md)。

## 自动化结果

| 验证 | 结果 | 范围 |
|---|---:|---|
| 桌面 Rust | 151 passed | 原 144 + 7 新测试；10 ignored、1 Keychain 写删测试显式过滤 |
| 本地云 API | 11 passed | HTTP/真实进程，独立 SQLite；不访问腾讯云 |
| Vitest | 169 passed / 21 files | 原 164 + 5 新测试；包括授权卡变更后仍可拒绝 |
| 浏览器预览 E2E | 16 passed | GUI 布局、检视、序列与历史 |
| 真实后端 E2E | 6 passed | 最新 harness，临时库，草案/正式隔离、记忆、审批及重启 |
| 契约、TS、ESLint、Rust fmt、严格 Clippy | passed | Clippy all-targets + integration-harness、`-D warnings` |
| 独立桌面 debug 应用 | build passed | `studio.betweenframes.m1reviewfixes`，57.73 MiB；未启动、未进行本轮原生 UI/签名/公证验收 |

主脚本 `bash scripts/verify-all.sh --skip-keychain` 一次完整通过，含 168 前端测试；授权卡最后复核新增一项后，重新执行 typecheck、lint、全部 169 Vitest，并补验最终前端的预览/后端 GUI 旅程。最终桌面包包含该授权修复。Rust 行为在最后的前端调整中未改动。

定向 Rust：28 个 agent_vision + 6 个 model_routing 全通过。新增覆盖配置文件边界与脱敏、有效集合规范化/冻结、全部能力要求、内置与显式配置优先级、未知模型零 HTTP/零预算预留、文本或缺结构化模型有 consent 也零图像传输、旧绑定无能力摘要拒绝。原恢复测试再增加“端点/模型相同但能力变化”的重启场景：拒绝 pending inspect；原 key 轮换成功路径保留。正式项目快照一致。

前端五个新测试覆盖：IPC 能力格式严格解析、未知能力禁止提交、纯文本预算归零后可继续、配置查询失败不发委托、授权卡已打开后能力失效仍能禁止图像。回归测试等待 React Query 更新后的真实提示，避免把异步通知前的旧按钮状态当结果。

## 授权真实服务回归

按用户既有 DeepSeek 授权，仅精确执行 `real_model_deepseek_filename_curation`。输入为临时生成的 6 张图片的文件名/元数据，**图片发送 0**，本轮没有新增 LUMIX 图片外发，也没有新的真实摄影质量任务。

| 项目 | 实测 |
|---|---|
| 端点/模型 | `https://api.deepseek.com` / `deepseek-flash` |
| 配置来源 | 内置 `local_protocol_smoke`；执行绑定含能力摘要 |
| Prompt / 视觉缓存 | `m3-openai-6` / `m3-vision-2@端点摘要`；本轮未改 Prompt/缓存键 |
| 结果 | `review_ready`，1 份 2 图草案，准确保留 real00/real01，次序一致 |
| 工具/轮数 | 3 轮 / 4 工具：list_photos → get_project_context → create_selection_draft → finish_task |
| 真实 token 回执 | 3 条，输入 12,853 + 输出 1,054 = 13,907 |
| 费用 | 3 美分版本化估算，非账单或发票 |
| 权威数据 | 正式项目完整快照不变，模型账本和最新 checkpoint 身份一致 |

单次协议回归不计专业质量通过，不增加摄影师、竞品或盲评样本。密钥仅经子进程环境传递，未写入报告/数据库/日志；保存的日志先做脱敏。

## 证据和复现

证据目录：[evidence/v2-m3-capabilities](evidence/v2-m3-capabilities/summary.json)。保存主脚本日志、最终前端/GUI 验证、定向测试、真实无图回归、桌面构建、源码摘要与产物摘要。GUI 测试前保护 104 个已有 PNG，测试后恢复本轮起始字节；不以 HEAD 覆盖先前修改。

```bash
bash "scripts/verify-all.sh" --skip-keychain
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --test model_routing --test agent_vision
npm run typecheck
npm run lint
npm test
npm run test:e2e
CARGO_TARGET_DIR="target-v1" npm run test:integration
BETWEEN_FRAMES_OPENAI_API_KEY="" CARGO_TARGET_DIR="target-v1" npm run tauri -- build --debug --config "src-tauri/tauri.m1-review-fixes.conf.json" --bundles app
```

真实模型测试默认 ignored，不能整组开启；本轮仅精确执行已授权的纯元数据任务。Keychain 写删、额外专业格式样本与性能基准没有在此切片新增运行，不将 ignored 计为通过。

## 剩余工作

- 这仍是单个 Responses Provider；独立 Anthropic/Gemini Adapter、故障健康状态、供应商专属授权与同能力 fallback 尚未实现。不因本配置自动转交第三方。
- 质量排名、延迟和模型能力的持续实测、账单校准尚未完成；官方文档、一次协议冒烟、部署声明分别保留来源，不等同专业摄影评测。模型别名仍不能冻结远端权重。
- 实例冻结能力配置，修改文件须重启应用；当前没有普通设置编辑器或热更新撤销。旧任务缺摘要/配置改变须新建委托，不能原地续跑。
- 当前 consent 仍按适配器/任务授权；首次执行身份绑定不能证明创建时已对具体供应商授权，后续多供应商方案需补齐。
- 缓存事实检索仍为字面关键词；embedding/语义索引与低价索引/策展分工待做。缓存键未因能力声明变化而改变，既有同源同模型同端点的本地观察可以复用。
- 原真实质量评测门槛保持：现有 1 个授权 LUMIX 任务/2 attempts、2 条 AI 模拟角色、0 真人，不能冒充 30 项真实创作任务、10 位摄影师或同题竞品基线。M4–M7 按 [剩余计划](V2-REMAINING-PLAN.md) 顺序推进。
