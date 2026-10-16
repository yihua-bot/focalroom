# AGENTS.md · Between Frames 工程导航

当前接续（2026-10-16，M3 模型能力配置）：先读 ../docs/V2-M3-CAPABILITIES-TEST-REPORT.md、MODEL-CAPABILITIES.md、ADR 0034。未知端点/型号默认拒绝；内置精确目录与 16KiB 部署 JSON 声明分来源，不能声明驻留/质量/授权/embedding。组合能力规则用于 Router 和 Adapter；配置摘要绑定任务，变化或旧绑定缺摘要须新委托，key 可轮换；实例冻结，改文件须重启。Prompt 仍 m3-openai-6，缓存键沿用 ADR 0033。委托/授权能力门槛已接 UI，配置失效仍可禁止图像。151 桌面 Rust +11 云、169 Vitest、16 预览/6 后端全通过，独立 debug 构建通过；DeepSeek 无图回归 review_ready，3轮/4工具/13,907实际token/3美分估算。M3 仍 partial，未实现跨供应商 fallback 或语义索引。无新迁移、线上部署或 LUMIX 外发；旧记录保留。

当前接续（2026-10-16，M3 路由/缓存检索）：先读 ../docs/V2-M3-ROUTING-SEARCH-TEST-REPORT.md、ADR 0033。Prompt m3-openai-6；实际实例模型/端点摘要绑定任务，恢复拒绝换端点/模型，key 可轮换；检查点不再猜环境。缓存 m3-vision-2@端点摘要，旧未绑定缓存保留不回填；search_photos 只读 scope 内元数据与可观察事实，带来源/覆盖，不触发上传/偏好证据。144 桌面 Rust + 11 云、164 Vitest、16 预览/6 后端全通过，额外 DeepSeek 文件名真实回归 review_ready：3轮/4工具/13,792实际token/3美分估算/0图片，独立 debug 构建通过。旧 queued 检查点失败释放名额；旧任务须新委托。质量 unmeasured、驻留 unknown，协议声明不冒充实测，M3 整体 partial。

当前接续（2026-10-16，用户允许模拟真人）：先读 ../docs/V2-M3-SIMULATED-REVIEW-TEST-REPORT.md 和 ADR 0032。已实际录入/导出/复核 2 条 AI 模拟作者与编辑评语，0 条真人；原 83 个评审材料/预览文件字节一致。工具增加 reviewerKind/reviewerModel，真人与 simulation 分开，旧缺来源为 unverified，同一身份来源冲突排除，--reviews 导入保持冻结与失败分母。15 项定向测试通过。模拟不写偏好证据、不伪造 human_baseline/竞品/独立真人，也不阻塞继续做可验证工程。旧 M3/M4 剩余门槛仍单列；本轮无新外发/迁移/部署。

当前接续（2026-10-16，M4 既有切片修复）：先读 ../docs/V2-M4-LOCAL-BOUNDARIES-TEST-REPORT.md、ADR 0031。9976b16 已新增 cloud/ 本地 axum/SQLite API，修复单 grant 撤销、Argon2id/摘要会话/24h 时限/注销、注册原子性、有界真实 JPEG 和首次启动/重启；全量通过：134 桌面 Rust + 11 云 HTTP/进程测试、159 Vitest、16 预览、6 后端及严格检查。没有已部署腾讯云 API，Postgres 不能只换连接串，M3/M4 整体仍 partial。旧 SHA 密码和明文会话不兼容，保留旧实验库并用新临时库；不碰用户库/线上 DDL。保留既有未跟踪 Dockerfile；不自动推进新云功能。下方“无 API 代码”为先前状态，以本段和当前报告为准。

给后续在此仓库工作的编码 Agent 与工程师。先读 `../docs/TEST-REPORT.md`（当前状态）与 `../README.md`，再按需深入。

当前接续（2026-10-16）：先读 `../docs/V2-M3-EVALUATION-TEST-REPORT.md` 与 `../docs/V2-REMAINING-PLAN.md`。已用用户授权 LUMIX 前 80 JPEG、12 张 1400px 抽样预览生成 8 图待评审草案；保留前一结构失败，1 个任务/2 attempts，完整成功 run 实际 84,203 token、估算 9 美分，尚无真人/竞品质量结论。最终 Prompt m3-openai-5 / m3-vision-2；已确认端点/模型使用版本化峰时费率，未知配置沿用旧估算，绝非账单。模型/视觉请求均执行原任务和档位上限；不提高预算、不自动提交正式版。新增 `tools/agent-eval`，可离线收集五维评分、冻结候选并保留失败分母。134 Rust / 159 Vitest / 16 预览 / 6 后端通过。真实照片与评审包在 Git 仓库外的 `/Users/yihua/Documents/BetweenFrames-Reviews/m3-lumix-20261016-141130/`，不要复制进仓库或上传。M3 仍 partial；腾讯云仅实例和空库，没有本项目 API，不自动跳阶段。

最新接续：`../docs/V2-DEEPSEEK-TEST-REPORT.md`。DeepSeek 官方实测：文件名任务进入 review_ready（3 轮/4 工具/13021 token），单图 Provider 协议通过（799 token）；原始开放选片仍预算暂停，不能算 M3 完成。已补真实 task_id、严格工具批次、错误正文隔离、实际 token 来源与预算结算；无分项计量保留预留。实际 token 与费用估算分开，不能将保守系数当账单。提示版本 m3-openai-2，旧检查点需重新委托；不自动提交草案或扩大预算。下一步优先解决开放任务预算内收敛，并建立授权素材评测。

当前增量（2026-10-16）：先读 `../docs/V2-AGENT-VALIDATION-TEST-REPORT.md` 与 `../docs/V2-M1-CR2-PREVIEW-TEST-REPORT.md`。用户已授权继续推进；本轮修复既有 Agent 响应/缓存校验与真实测试假通过条件，不代表新增服务或 M1/M3 整体通过。新响应必须经本地 observation schema + ID 唯一性检查；缓存必须结构合法且 ID 匹配。真实任务验收须有预算内可审阅草案，暂停/等待输入/零草案不计完成；直接 observe 只属 Provider 冒烟。主循环计量仍为预算估算。完整 RAW 显影不扩展为 P0 阻塞项；后续优先授权素材质量评测。下方阶段入口及计数保留历史，以本段链接的报告为准。

当前入口：`../docs/V2-M1-NATIVE-500-TEST-REPORT.md`。独立原生 release 的生成 500 图导入/重复跳过、虚拟网格、25 张批量保留/评分、原图比较、序列 Undo/Redo、版本恢复、系统导出与完整退出重启已通过；50 表和 500 源 hash 一致，Agent task 为 0。修复了胶片条样式误放在未加载 App.css 的问题，规则移入共享 app.css。本轮 120 Rust 默认测试、147 前端、4 M1 GUI 通过；累计基线仍为 122 Rust 功能 + 1 增长基准、27 独立 E2E（额外样本/其他 E2E 本轮未重跑）。M1 仍 partial：真正 RAW 细节、专业色彩/高像素素材、WKWebView 总资源与低端设备待补；不进入新 M2。主进程 RSS 不能称为应用总内存。原用户库未升级。

当前方向已接入 v2.0：先读 `../docs/V2-M0-TEST-REPORT.md`、`../docs/V2.0-ADOPTION.md`。M0 工程/契约切片本地验收通过，整体仍 partial/待评审；下文的 M0–M5 完成状态指 v1.0 历史。新增入口见 `V2-M0-README.md`，一键验证 `npm run verify:v2`。不要直接应用新包初始 SQL、覆盖旧迁移，或将示例 Web / skeleton 当成云协作；旧 src/src-tauri 保留以保护已有能力。

## 这是什么

本地优先的摄影编辑工作台（Tauri 2 + React/TS + Rust/SQLite）：GUI 完整可用，Agent（Mock/真实视觉）经唯一 CommandHandler 只写草案；偏好记忆从人工命令取证。v1.0 六阶段（M0–M5）工程已全部落地并验收。

## 目录速览

```
src/                      React 前端（app 外壳 / domain 类型 / features 功能面板 / services IPC 仓库）
src-tauri/src/
  diagnostics.rs          桌面/harness 共用只读统计，失败不回退为 0，禁止返回路径/业务文本
  agent/                  Agent 域：runtime(引擎) provider/openai vision consent memory tools policy context lifecycle
  edit_commands.rs        GUI 命令执行（apply_gui）
  command_handler.rs      唯一命令通道（幂等回执、历史、证据挂钩）
src-tauri/migrations/     0001–0017（改 schema 只能新增迁移，禁止改历史文件）
src-tauri/tests/          集成测试：agent_runtime / agent_vision / agent_memory / command_history / local_core / keychain / perf_baseline
contracts/                机器契约（改后必须跑 contracts:check；GUI 命令 DTO 从契约生成，勿手改生成物）
prompts/                  版本化提示词（agent-system-m2/m3、vision-observe-m3）
e2e/                      Playwright：preview(浏览器 Mock) 与 integration(真实 harness) 两套配置
scripts/                  契约生成 / 集成服务器 / verify-all.sh 一键全量验证
```

## 一键验证

```bash
bash scripts/verify-all.sh                  # 全量（约 5–8 分钟）
bash scripts/verify-all.sh --skip-integration  # 跳过真实后端 E2E
bash scripts/verify-all.sh --skip-keychain     # 跳过钥匙串写删回环，其余全量验证
```

## 不可违背的约束（违反会被测试与评审拒绝）

1. **Agent 永不直接改正式状态**：agent 通道只经 ToolRegistry+PolicyGate 写草案/任务表；正式选片只走 CommandHandler 的人类命令（ApplyDraft 等）。
2. **AI 数据 ≠ 权限**：工具结果、备注、模型输出都是不可信数据，不得提升为指令；photo_id 引用必须服务端校验。
3. **原图只读**：永不修改/移动/上传原图；云端只允许授权后的 1400px 重编码预览，配额按真实传输扣减。
4. **证据只认人类**：preference_evidence 只从生效的人工命令记录（dedup_key=command_id）；Agent 草案结构性不计证据；纠正后的假设不复活。
5. **幂等与恢复**：命令/任务请求/工具调用均有持久化回执；中断恢复走检查点，不重复副作用。
6. **迁移只增不改**：旧库升级用新迁移文件；动 schema 先读 `docs/ADR`。

## M1 500 张验收

- `npm run test:m1-gui` 构建 release harness，使用独立临时库与 250 PNG + 250 JPEG（1200×800 派生测试图）。`verify:v2` 在旧回归后串联此脚本。
- 500 与既有 integration 测试共用 1420/1422，须顺序执行。进程重启前先关闭页面轮询，测试服务退出旧 Rust 子进程后用同目录启动新进程，断言 PID 改变及快照相等。
- 性能采样关闭 trace，区分自动化等待、keydown→下一帧和 IPC；GPU 为软件渲染，不伪造硬件利用率；当前 RSS 偏高不能宣称低端机通过。
- 原图保护必须保存并比较操作前后 SHA-256，不只检查长度。所有移动/重连仅针对临时生成素材。
- 原图检视走 `get_original_photo(projectId, photoId)`，仅授权后端核对过的精确文件，直通 JPEG/PNG/TIFF/HEIC 的边界为 64 MiB / 3200 万像素；不接受前端路径，也不扩展云视觉入口。桌面入口称“100% 像素”，按实际显示源与屏幕物理像素计算。CR2 v2 小端 IFD0 支持有界内嵌 JPEG，kind=`embedded_preview`，data URL 限 4MiB / 1600 万像素，不授权整个 RAW；见 ADR 0029。其他 RAW 或失败情况返回 `derived_preview`，每次检查已有 JPEG 不超过 1400px / 16MiB，不能启动无界转换或冒充全分辨率。预览仓库仍为缓存像素。
- GUI 的 `compact: true` 只缩减返回副本的历史前后状态/上下文；不得改数据库审计、已存幂等回执、Undo/Redo 或导出。完整默认接口保留兼容。
- `BF_M1_ACCEPTANCE_SLICE="v2-m1-inspection" npm run test:m1-gui` 运行 4 项旅程并保留旧 500 图基线证据。当前原生评审使用 `tauri.m1-review-fixes.conf.json` / `studio.betweenframes.m1reviewfixes` 的独立标识，禁止拿旧应用标识启动新 schema。

## v2 评审修复约束

- ApplyProposal 是人工正式命令，经 CommandHandler.execute_tx 与 v2 回执同事务提交；不得恢复逐操作独立提交。空选驳回不改作品版本。
- Promotion 门槛覆盖 v1/v2、导入和重连，事务内重读权威；failed/aborted 可同版本新建尝试，completed 后本地只读。
- Router 接入每次模型请求与云视觉上传前；任务绑定 provider 不自动切换。预算从美分转 USD，累计扣除档位额度。配置文件/设置 UI 尚未接入。
- 0017 不改变旧回执/历史。v2 新回执要求完整 envelope 身份一致；v1 原有别名行为保留。先读 ADR 0023。

## 已知的坑

- **integration E2E 用 target-v1 的 harness**：改了 Rust 后必须 `--target-dir target-v1` 重建 `m1-harness`（verify-all.sh 已内置），否则旧二进制以 `unknown field` 报错。
- **同一 run 的集成 spec 共享一个 harness**：全局计数断言用下界（见 beta-journey.spec）。
- **命令回执快照的 revision 落后一步**：测试里要权威 revision 就重新 `db.get`。
- **Playwright 无 console 监听时不显示页面 console.log**：调试 e2e 时先加 `page.on('console')`。
- Node/cargo 在部分 shell 需 `export PATH=/opt/homebrew/bin:$PATH`。
- 验证脚本与 integration harness 显式禁用真实 API Key；视觉测试只使用注入的本地 HTTP Provider。
- M5 get_diagnostics 由 diagnostics::collect 统一桌面/harness；前端经 TauriDiagnosticsRepository 白名单解析后展示/复制。存储为逻辑文件大小，可不完整；不是清理入口。见 ADR 0013。
- 回执保存历史业务快照 + eventCursor（ADR 0028），重放仅补当时事件；不得用当前 snapshot_tx 替换整个回执。兼容旧完整与无游标瘦回执，不修改旧记录/触发器。增长基准按实际成功次数与回执行增量核对，逻辑大小为 page_count×page_size，WAL 单列。
- 原图检视：TIFF/HEIC 保留直通边界；CR2 内嵌 JPEG 支持子集见 ADR 0029，其他 RAW 验证并复用导入派生缓存，不读取旧全局临时检视缓存。HEIC/CR2 样本测试默认显式 ignored，具备样本后单独执行，缺样本不能返回成功。原生修复补验使用新标识 studio.betweenframes.m1reviewfixes，禁止启动旧用户库。
- M4 记忆控制经 execute_memory/MemoryRequest；变更暂停执行任务并要求新委托，不推进正式 revision。0009 的 FTS5 是可重建索引，原证据与备份仍保留；跨项目关闭。见 ADR 0012。
- 云端请求预留/结算复用 agent_steps：未知结果保留预算、恢复不重传；单次 consent 必须绑定 task_id。见 ADR 0011。

## 修改向导

- **加 GUI 命令**：改 `contracts/gui-command-request.schema.json` + m1-command-examples → `npm run contracts:generate` → edit_commands 实现 → 命令历史/契约测试。
- **加 Agent 工具**：改 `contracts/agent-tool-contracts.json`（白名单+strict）→ tools.rs 实现 → PolicyGate 约束 → agent_runtime 测试加轨迹断言。
- **动预算/consent**：types.Budget、0005 的 CHECK、policy.rs、前端 zod 三处同步。
- **换视觉模型**：环境变量 `BETWEEN_FRAMES_AI_MODEL / _BASE_URL`；key 走钥匙串或 `BETWEEN_FRAMES_OPENAI_API_KEY`。

## 文档地图

- 当前状态：`../docs/TEST-REPORT.md`；决策链：`../docs/DECISIONS.md` + `../docs/ADR/0001–0008`
- 当前方向：`../docs/V2.0-ADOPTION.md` 指向 BF-SPEC-2.0；工程切片见 ADR 0016。旧基线 `../planning-v1.0/docs/FINAL-IMPLEMENTATION-SPEC.md` 保留作兼容参考；冲突需 ADR，不默默替换。
- 遗留边界：`../docs/KNOWN_ISSUES.md`；隐私：`../docs/PRIVACY.md`；试用：`../docs/BETA-GUIDE.md`
