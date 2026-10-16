# M3：模拟作者/编辑评审与来源隔离

日期：2026-10-16。用户明确允许“模拟真人”继续开发。本轮已完成模拟评审录入→导出→冻结映射复核→分来源汇总的工程旅程，**M3 真实摄影质量与全计划仍 partial**。模拟角色支持持续开发和流程验收，真人/竞品证据另外收集，不把缺真人评测当成停止所有工程的理由。

## 实际交付

- 评语增加 reviewerKind / reviewerModel；页面要求选择真人或 AI，AI 模式明确显示用途。模型/会话名称需据实填写，未暴露精确版本不虚构版本。
- 同一 reviewerId 的来源/模型冲突会拒绝页面改写；CLI 排除矛盾身份全部分数并报告冲突。未标来源的旧评语按 unverified，不自动升级为真人。
- 真人五维、逐运行分数和事实错误率保持独立；AI 记录进入 simulation。多个模拟角色、即使使用作者 ID，都不能满足作者与独立真人门槛。
- 评语绑定由冻结摘要及原种子派生的 packetId，跨批次匿名编号相同也拒绝错配；旧缺批次记录不计分。旧无评分 input/key 可重新生成新页面。
- CLI 新增可重复 `--reviews`，直接导入页面导出文件，要求原 private-key、复核冻结材料，保留原输入和全部失败分母；重复评分不静默覆盖。源文件/既有材料不修改。
- 纯 AI 导出为 simulated-ai-reviews.json，混合来源明确命名；工具不调用模型、不自动打分、不修改应用数据库或审美记忆。

对应规格 M3 / AGENT-QUALITY-EVAL；决策见 [ADR 0032](ADR/0032-m3-simulated-review-provenance.md)，字段与运行方法见 [评测工具说明](../between-frames/tools/agent-eval/README.md)。没有新增依赖或数据库 migration，不改变正式 CommandHandler/ACL，也没有伪造真人审批。

## LUMIX 模拟演练

使用原 80 图、12 张模型观察→8 图候选的已授权批次，保留原 1 任务/2 attempts 和前次失败。当前 Codex 会话查看 8 图总览，并打开第 1、4、5、7 张的 1400px 派生预览，再分别模拟作者与编辑角色：**2 条 simulated_ai、0 条真人、0 位独立真人**。不是独立盲评、竞品实验或两个模型的一致性实验。

五维模拟分数只在独立栏保存。作者模拟提出对照试排“狐狸—道路—马”；编辑模拟建议检查尾段连续远景节奏、回看第 5 张主马在预览中偏软的原因。仅为主观工作建议，不断言原片失焦，不生成正式取舍或作者偏好。每角色核对相同四个宽泛主体/环境事实，合计 8 次检查但只有 4 个重复核对的事实，不是 8 个独立样本；其余 68 张不计本会话已检查。

本地新材料：

`/Users/yihua/Documents/BetweenFrames-Reviews/m3-simulated-2026-10-16T12-02-56-437Z/`

- SIMULATED-REVIEW.md：可读模拟意见和明确身份。
- simulated-ai-reviews.json：通过实际 Chromium 页面录入并下载的两条来源完整评语。
- evaluated/report.json：human=0 / simulatedAi=2，顶层真人均值 null，simulation 单列，状态 partial。
- simulated-review.png：实际新页面、来源提示、两条记录与 8 图截图；真实照片不进 Git。
- simulation-validation.json：原始批次 input/key/评审页及 80 张预览共 83 文件摘要前后一致，零 HTTP 请求。没有读取/修改原用户库、写正式作品或生成 preference_evidence。

原评审包和私人映射保留原位，不覆盖已打开页面。本轮没有新 DeepSeek 请求或照片上传，开发模拟意见不回写第三次人工反馈/记忆增益任务。

## 测试和功能验收

定向 **15 项评测测试通过**（含实际 Chromium 旅程）：

1. 模拟作者/编辑即使使用真实作者 ID 也不能满足真人门槛；真人分数/事实错误率为 null，模拟分数独立。
2. 真人与模拟同时存在时真人分母/均值不变；旧缺来源评语进入 unverified，矛盾身份全体排除。
3. 模拟缺模型拒绝、真人附模拟模型拒绝；页面必须选来源，同一 ID 不能换来源，纯模拟与混合下载名称正确。
4. CLI 导入必须带原 key；原输入保持字节一致，篡改候选顺序拒绝，重复导入显式 partial，无静默覆写。
5. 不同批次即使都叫 case-1/candidate-1，packetId 不同就拒绝；缺批次标识也不计分。
6. 原有缺图拒绝、五维/理由/事实抽查校验、放大/Esc、输入不执行 HTML、离线零 HTTP 和严格 JPEG 打包保持通过。

全量 `bash scripts/verify-all.sh --skip-keychain` 退出 0：**134 桌面 Rust + 11 云 HTTP/进程测试、163 Vitest、16 浏览器预览、6 最新 harness 后端旅程通过**，契约/类型/Lint/fmt/严格 Clippy 通过。补齐批次绑定后再次执行类型/Lint/全部 **164 Vitest（21 文件，含 15 评测测试和真实 Chromium 离线旅程）**，全部通过；未改生产 Rust，不重复未变化的后端旅程。没有新忽略项，原 10 项 Rust ignored 和 1 项 Keychain 写删过滤保留；未执行远端 CI、原生交互或发布构建。
 
120 个跟踪 PNG 按运行前实际字节备份，恢复 24 个被回归改写的截图，全部核对一致。最终真实素材演练具有 packetId，2 AI/0 human、原 83 文件一致、零 HTTP；第一次原型中间材料保留，不作为额外评委或新任务。

证据：[全量日志](evidence/v2-m3-simulated-review/verify-all.txt)、[最终前端检查](evidence/v2-m3-simulated-review/final-frontend.txt)、[演练汇总](evidence/v2-m3-simulated-review/summary.json)、[源码摘要](evidence/v2-m3-simulated-review/source-sha256.json)。真实照片、截图及完整意见只在仓库外，不复制进 Git。

## 工程原则和剩余

复用同一 schema、冻结摘要和 CLI；分数/事实汇总提取共用函数，在既有 UI 增加来源字段，不创建另一套评测或模拟用户身份系统。原应用“人工命令才构成偏好证据”的约束继续保留。回退为原无评分材料即可隔离演练；旧无来源评语不能直接回填为真人。

仍需完成真实分层创作任务、作者/独立编辑与竞品质量证据，以及 M3 测量后路由、图像索引/检索、多 Adapter/经授权 fallback 和费用账单校准。模拟允许工程继续，但不能证明上述摄影质量或实际经营门槛。M4 仍是本地 HTTP 切片，线上 Postgres 权威/Invite/Promotion/对象存储/Web 审稿未完成；本轮不进入新的云功能或部署。阶段缺口见 [剩余计划](V2-REMAINING-PLAN.md)。
