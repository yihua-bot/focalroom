# Focal AI 语言改动：测试、缓存与发布影响复核

后续状态（2026-11-04）：固定中文观察问题已完成本地接线修复，缓存采用「保留原始观察、回复按当前语言转述」策略，见[修复与验证记录](./AI-OBSERVATION-LANGUAGE-FIX-2026-11-04.md)。下文保留 11 月 3 日复核时的发现与结论。

日期：2026-11-03（Asia/Shanghai）。范围：核对交接中的语言规则、版本更新、缓存及测试影响。本轮未修改产品代码／提示词，未提交、部署、打包、访问正式用户库或调用真实模型；只新增本报告。

工作树包含多方改动与截图／构建产物。核对期间读取到 HEAD `8be1e435`，测试针对当时本地工作树，不将其描述为冻结发行版本的验证。

## 1. 当前结论

语言规则的主要入口已补，但还不能宣称「逐图观察跟随摄影师语言」已经接通：宿主生成的观察问题仍固定中文，缓存复用也没有语言维度。现有自动测试通过，真实模型的多语言行为尚未验证。

交接中的「知识模块三个类型推断错误导致 lib 测试不能编译」已不符合本次运行状态；本轮 `cargo test --lib` 成功完成。没有为证明某组改动而临时移除或覆盖他人代码。

## 2. 产品源文件影响范围

交接所述语言改动共 12 个文件：

- 五份提示词：`agent-system-m3.md`、`agent-canvas-v1.md`、`vision-observe-m3.md`、`focal-agent-v2.md`、`focal-chat-v2.md`。
- iOS 文字聊天副本：`apps/mobile-ios/FocalRoom/ChatPrompt.swift`。
- 服务端评论提示词：`apps/api/src/agent-tasks.ts`。
- 版本与缓存：`agent/mod.rs`、`agent/context.rs`、`agent/vision.rs`。
- 版本断言：`agent/task_scope_tests.rs`、`focal_chat_tests.rs`。

以上 Rust 路径均相对 `between-frames/src-tauri/src/`，提示词相对 `between-frames/prompts/`。当前工作树另外存在知识检索、UI、iOS 工程及产物修改；本报告不把它们算进这 12 个文件，也不认定它们已经适合一起发布。

## 3. 两处尚未闭合的行为

### 3.1 观察问题仍固定中文

位置：[vision.rs](../../between-frames/src-tauri/src/agent/vision.rs:172)。

- `inspect_photos` 固定发送「描述每张照片的可观察事实与它在项目中可能的编辑作用。」。
- `compare_photos` 在工具的 brief 前添加中文比较指令，可能形成混合语言。
- 新视觉提示词却要求以「观察问题的语言」决定输出语言。
- 当前这段路径只读取项目 brief，没有把摄影师最近消息直接接入观察语言选择。

因此，仅修改视觉提示词不能保证英文、日文、韩文任务取得相应语言的逐图观察。自动测试的固定模型响应无法证明这一行为已修复。

### 3.2 按调用语言生成内容，却跨语言共用缓存

位置：[model_binding.rs](../../between-frames/src-tauri/src/agent/model_binding.rs:71)、[vision.rs](../../between-frames/src-tauri/src/agent/vision.rs:103)。

缓存查询依据照片 ID、照片内容哈希、观察模型、prompt_version 与工具种类；其中 prompt_version 为视觉版本加端点摘要，不含输出语言、观察问题或项目 brief。

对于同一照片／模型／端点／工具，英文任务产生的观察可直接被后续日文任务复用。若产品保证逐图观察本身跟随本次语言，仅升级一次视觉版本无法满足这个保证。

实现前需统一设计：观察事实保持一种稳定的内部表达、面向用户再按语言呈现；或者明确把观察语言纳入任务及缓存身份。前者有利于复用，后者会增加分语言观察成本。不能只改提示词后宣称已实现任一方案，也不能未经来源与兼容性核对把旧观察改标成新版本。

## 4. 缓存与旧任务的实际影响

| 变化 | 当前机制 | 实际影响 |
|---|---|---|
| `m3-openai-9 → 10` | 检查点保存基础 promptVersion，恢复时比较 | 不兼容的旧任务会被要求重新委托，可能显示「执行版本已变」 |
| `focal-agent-v2.md` 正文改变 | 对话保存提示词 SHA，读取时核对 | 旧对话任务也可能触发「对话能力已更新」 |
| `canvas-1 → 2` 与正文改变 | 授权记录包含画布提示词摘要 | 旧画布任务可能在授权核对阶段被拒绝；不能把所有恢复错误统称同一句提示 |
| `+focal-2.2`／`+canvas-2` | 请求版本标签更新 | 标签与正文一致；不能单独替代其余绑定检查 |
| `m3-vision-4-upload512-q75 → 5` | 当前观察查询要求版本相等 | v4 不命中 v5 的直接观察复用 |
| 视觉版本升档 | `photo_search.rs` 也过滤当前视觉版本 | 旧观察不再进入这条当前关键词／语义候选路径；检索覆盖会减少 |
| `creative-context-v2.md` 并发改变 | 创作上下文另有绑定校验 | 属于额外恢复影响，应与语言改动分开记录 |

「旧逐图观察全部作废、全部重新扣费」不够准确：旧记录没有因此被删除；当前版本的部分读取路径不再使用它们。只有后续确实请求补观察、通过授权和预算校验且发出新请求，才涉及新的上传与费用；不会因为版本变化就自动重跑全图库。历史草案与冻结依据也不能简单等同于当前检索缓存。

「跟随目标语言等价于跟随 app 语言」只对采用当前本地化快捷模板的路径成立。[AIPanel.tsx](../../between-frames/src/features/ai-editor/AIPanel.tsx:283) 的选片建议优先使用 `project.brief`；用户输入、项目简介和已保存任务目标都可以与当前界面语言不同。

另有一处版本标识遗漏：`agent-system-m3.md` 首行仍为 `m3-openai-9`，运行时已返回 `m3-openai-10`。运行时检查使用后者，不据此判定缓存失效错误；但提示词正文与审阅记录会混淆版本，后续应一并同步。若再次改写正文，应在最终内容确定后统一核对版本与恢复绑定。

## 5. 受影响测试与本轮结果

| 检查 | 本轮结果 | 覆盖与限制 |
|---|---|---|
| `cargo test --lib` | 183 通过，0 失败，3 跳过 | 包含 focal_chat、task_scope、context 和当前知识模块；不是全部 Rust integration tests |
| `cargo test --test agent_vision` | 47 通过，0 失败 | 本地模拟模型；含缓存、端点隔离、恢复、预算和统一会话链路 |
| `node apps/api/test/run.mjs --test apps/api/test/agent_worker.mjs` | 5 通过，0 失败 | 临时隔离 PostgreSQL 与本地模拟模型，无正式外发 |
| `npm run typecheck` | 通过 | 前端与相关 TypeScript 类型检查，不等于 UI／原生构建通过 |
| iOS 副本逐字比对 | 与 `prompts/focal-chat-v2.md` 一致 | 未运行 iOS 编译或真机聊天 |
| 语言改动相关文件 `git diff --check` | 通过 | 不代表整个多方工作树的格式检查已通过 |

视觉集成测试中特别相关的已有案例：

- `consented_vision_uploads_once_then_cache_hits_without_reupload`
- `pixel_pipeline_change_does_not_reuse_legacy_observations_or_reupload_current_cache`
- `observation_cache_is_scoped_to_endpoint_even_when_model_alias_matches`
- `role_cache_and_search_follow_observer_across_new_curators`
- `obsolete_queued_checkpoint_fails_once_and_releases_the_project_for_redelegation`
- `paused_upload_settles_and_replays_after_restart_without_retransmission`
- `unified_conversation_answers_then_inspects_and_creates_a_reviewable_draft`

本轮没有新增仅检查提示词字面的测试。后续真正需要的新增验证是：明确语言依据能到达实际观察请求；相同照片跨语言调用时缓存行为符合选定设计；用户在 app 语言与项目语言不同时得到约定结果。真实模型还需验证英文／日文／韩文的回答、追问、草案摘要与便签；不能用本地固定回复代替。

## 6. 评论汇总的发布边界

评论提示词由独立 [Worker](../../between-frames/workers/agent/src/index.ts:3) 导入并执行。本地 [compose.yaml](../../between-frames/ops/shared/compose.yaml:47) 将 agent-worker 定义为单独服务，与 API 共用构建入口。

因此「只更新 API 就会生效」不成立；实际需要更新执行评论汇总的进程或镜像。生产当前是否运行 Worker、使用哪份镜像与配置、待执行任务情况，本轮均未访问核验，不能称为「零风险上线」。本地测试证明两种评论任务的现有链路可运行，不证明模型会遵循新语言规则。

若后续发布，应先形成只含评论提示词改动的可审查产物，核对现有 Worker 状态、回退版本和生效路径，再按用户的生产操作确认规则执行。不能把共享工作树整体构建物当作只包含两段提示词的发布包。

## 7. 建议顺序

1. 先确定观察事实与用户呈现是否共用语言，闭合固定中文问题和缓存身份的冲突。
2. 根据所选方案补请求链路与跨语言缓存行为验证；不通过自动重跑旧图库完成验收。
3. 再做桌面／移动端小范围真实语言验证，明确实际样片与费用后执行。
4. 评论 Worker 改动可独立准备发布，但需核对真正执行进程；生产发布不与桌面重新打包捆绑。

方法原则：保留稳定事实与按需呈现的职责分工；复用现有缓存、授权与恢复机制；不为了语言路由添加第二套任务系统。作品集策展方法模块也应沿用最终确定的语言约定，不复制一套不同规则。
