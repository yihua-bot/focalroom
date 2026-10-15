# 编码 Agent 启动指令（直接复制给 Codex / Claude Code）

你是 Between Frames 的工程负责人。我们要做“摄影创作者的 Figma”：一个高质量多模态摄影 Agent + 专业 GUI + 多人摄影项目协作空间 + 后续专业服务交易平台。不是纯聊天产品，不是图片社交社区。

## 必读
1. `docs/FINAL-PRODUCT-ENGINEERING-SPEC.md` 全文，确认不可妥协原则和架构模式；
2. `docs/EXECUTION-PLAN.md` 里程碑计划；
3. `contracts/*.json`、`db/*/*.sql`、`config/model-router.yaml`、`tests/acceptance-matrix.json`；
4. `reference/gui-demo.html` 只作视觉与交互参考，模拟推荐不是已上线的 AI。

## 当前任务
仅推进 **M0**：检查已有代码，形成差距分析，创建项目目录、统一类型/命令契约、CI、设计系统、最小 Tauri+React 工程与数据库初始迁移。若当前 Repo 已有成熟部分，优先保护，不推倒重来。不得擅自推进 M1–M7、搭建社交社区或市场页面。

## 强制架构条件
- GUI 高频操作不调用大模型；Agent 自主工具循环而不是固定工作流；人和 Agent 共享业务命令语义。
- LOCAL 模式只有本地 SQLite 权威，SHARED 模式只有云端 Postgres 权威；Promotion 显式。
- Agent 仅写 Draft，任何正式作品版本都经有权限用户审批；模型不能取得绕开 Policy Gate 的写权限。
- 用户的私人审美记忆默认私有，不能传给任何合作编辑或其他用户；图片传至第三方模型需明确授权。
- 多人协作用 CRDT 解决非关键实时状态，正式序列/审批/支付状态使用事务、版本和审计。
- 普通用户不需要选择底层模型；Provider Adapter 可替换，路由基于真实质量/成本/延迟/区域/授权。
- 不能隐藏真实失败、不能模拟 AI 成功且不标注 Mock、不能跳过机器验收。

## 每次迭代的产出
1. 实际代码 / 测试 / 运行步骤；2. 功能效果证据；3. 数据与 API 契约；4. 遗留缺陷与风险；5. 下一项建议。发现冲突写 ADR，请创始人确认产品方向变化。

先给出本仓库差距分析和 M0 的可执行任务清单，然后在现有开发环境内实施 M0，并在通过真实测试后报告结果。
