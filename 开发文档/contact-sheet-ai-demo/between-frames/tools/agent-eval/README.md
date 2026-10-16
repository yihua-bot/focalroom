# M3 授权任务与盲评工具

对应 v2 `AGENT-QUALITY-EVAL.md`。本工具整理可审计记录，生成离线评审材料，分别汇总真人和 AI 模拟评分；不会调用模型、上传照片、自动生成评分或宣布 M3 通过。模板是空集，运行应返回 `partial`，不能把单测夹具记为摄影评测。

```bash
npm run agent:eval -- "tools/agent-eval/template.json" "/tmp/bf-evaluation-empty"
# exit 2 = 证据不齐（报告仍生成）；0 = 材料齐备、等待人工发布判断；1 = 格式/写入错误
```

输入字段的机器权威是 `evaluation.ts` 的 `inputSchema`，类型为 `EvaluationInput`，不修改数据库。所有对象严格拒绝未知字段。

- `tasks`：真实任务 ID、作者、目标、六类项目分层、首次/第三次阶段、目标张数、80–300 个照片 ID 与原始 SHA-256、作者授权凭据与精确范围。`synthetic` 不计摄影质量证据。第三次策展必须有反馈证据引用。
- `runs`：运行前登记的所有 attempt，包括失败/暂停；标明系统、模型、版本和 Prompt，记录有序 `keep` 照片 ID、说明、实际 token、耗时、工具调用及证据位置。`estimatedCostMinorUsd` 和 `billedCostMinorUsd` 分开；没有账单填 `null`。不得把 `undecided/reject` 条目当候选序列。
- `audit`：Agent 每次运行前后的正式状态、原图 hash 清单摘要、越权写入数及实际证据引用。缺审计不能当零违规通过；人类基线的编辑不计 Agent 越权。
- `followUp`：作者实际后续修改数、评审/采纳的变更数和命令证据；尚未产生填 `null`。采纳率仅诊断，不是最大化目标。
- `reviews`：保留本批次的 `packetId`，明确 `reviewerKind`（`human` / `simulated_ai`），`reviewerModel`（真人为 `null`，模拟填实际模型/AI 会话名）、评审者 ID、五维 1–5 分、可核验理由与事实抽查数。每个可审阅候选的真人门槛仍须一名实际作者和一名实际独立评审者。

首次运行生成一个新目录，拒绝覆盖。`input.snapshot.json`、`private-key.json`、`report.json` 只给组织者；其中私人映射和随机种子不发给评委。报告保留所有登记失败的分母，并按运行、版本和首次/第三次阶段列分项分数。没有样本的比例为 `null`。无来源凭据、未知 ID、非法写入等问题不能由高审美分抵消。

## AI 模拟评审和来源兼容

用户可以授权 AI 模拟作者、编辑等角色来演练交互和归纳改进建议。每条记录必须标记 `simulated_ai` 并填实际模型/会话名；模型精确版本未知时如实注明。多个角色出自同一模型，不因此变成独立真人，也不创建假的 `human_baseline` / `competitor` 运行。

`report.dimensions` / `scoreByRun` / `visualFactError` 只统计明确来源的真人记录；AI 评分、分运行结果和事实抽查另列 `report.simulation`。`reviewSources` 显示 human/simulatedAi/unverified/conflicted 数量，模拟角色即使使用作者 ID 也不能满足作者/独立真人门槛。模拟意见不会写入应用数据库或 `preference_evidence`，不能当成摄影师已采纳。

未标来源的旧评语按 `unverified` 读取并保持 partial，不自动升级为真人。需由组织者核对实际来源后重导出；不能仅为过门槛批量改标。相同 reviewerId 的来源/模型冲突会排除该身份的全部分数并报错。当前来源仍为组织者/评委声明，工具不能验证真人身份或防止故意伪造；这些检查也不证明统计独立性。

页面导出后可直接汇总，无须编辑冻结的输入：

```bash
npm run agent:eval -- "/原批次/input.snapshot.json" "/新的结果目录" "/原批次/private-key.json" --reviews "/评委导出/blind-reviews.json"
# 多名评委可重复 --reviews；纯 AI 导出名为 simulated-ai-reviews.json。
```

导入保留原输入、原映射和所有失败记录；输入变化拒绝，缺原 key 拒绝，重复评分返回 partial，不静默覆盖。每份 publicPack 带由原 seed 和冻结摘要生成的 HMAC `packetId`，每条导出评语保留同一标识；缺失或异批次标识拒绝，不能仅因 case-1/candidate-1 别名相同就混入评分。它用于批次归属校验，不验证真人身份或评分签名。

维持 `bf-m3-eval-1`；冻结摘要排除 reviews，因此旧无评分批次的 key 可继续使用。旧 publicPack 缺 packetId 时须用原 input/key 重新导出新页面；旧评语缺批次标识先拒绝，缺来源则按 unverified，不再贡献真人均值。这是明确的证据兼容变化，原文件保留不覆盖。

可加 `--previews "/绝对路径/cache"` 生成 `review-ready.html`：把已派生 JPEG 内嵌为一个离线文件，直接打开即可评分；不包含私人映射。只读取命名匹配的 JPEG，拒绝文件链接、单图超过 16MiB、总预览超过 128MiB。此文件包含真实照片，只分发给获授权的评审者。目录方式和单文件方式均由浏览器 CSP 禁止网络。

评委仅接收 `blind-pack.json`、`review.html` 和任务对应的已授权预览目录。预览命名沿用生产缓存 `{原图SHA256}-1400.jpg`，无需交付原图。材料中的目标和说明可能人工包含产品名，组织者须在发出前检查并冻结批次；程序不会修改原始解释以隐瞒来源。页面直接本地打开，选取 JSON 和预览目录后逐候选评分，导出 `blind-reviews.json`。页面不联网、不自动保存；离开前导出。此文件只有匿名任务/候选别名、评审者 ID、评分和理由。

将各真人导出的 `reviews` 数组合并到 `input.snapshot.json`（避免重复同一评审者/候选），再使用原私人映射汇总：

```bash
npm run agent:eval -- "/tmp/batch/input.snapshot.json" "/tmp/batch-reviewed" "/tmp/batch/private-key.json"
```

任务、照片、目标、run 注册表或运行内容变化会使冻结摘要失配，必须新建批次重评。不能在看过分数后替换失败 attempt 或沿用旧候选评分。工具无法证明运行前登记的时间、照片权属、评审者真人身份或外部证据真实性；这些需组织者独立核对。也不自动计算竞品偏好显著性或作美学发布判断，须保留同题对照和样本分析。

当前不足：尚无 30 项真实任务和专家评分；缺少实际竞品试用、完整照片级事实核查、套餐账单校准。离线页面只是评测辅助，不是云端认证审稿或 M4/M5 服务。

复现：`npx vitest run "tools/agent-eval"`（规则、CLI 冻结与覆盖保护、真实 Chromium 本地盲评/导出/零网络旅程）。测试中的评分和来源声明均为单测数据，不能纳入质量报告。
