# ADR 0018：v2.0 M3 ModelRouter

日期：2026-10-15。

- 按模板 config/model-router.yaml 的语义实现代码内默认路由目录（mock 本地零成本 / openai 云端视觉+工具），不引入 YAML 依赖：模板字段为策略语义而非运行数据，部署扩展用 JSON 覆盖文件即可（未实现，YAGNI）。成本为保守估算系数，真实计费由质量与账单数据校准，不冒充报价。
- 过滤顺序 region → privacy_consent → capability → availability；排序质量档 → 估算成本。三类可读错误：MODEL_CAPABILITY_MISSING（可用方均无该能力）、PRIVACY_REGION_REJECTED（区域驻留或图像授权缺失——路由永不自动扩大图像范围）、BUDGET_EXCEEDED（min(档位上限, 剩余预算)）。
- no_provider_selection_required_in_normal_ui：路由决策经 explain() 投影供诊断，不含密钥/端点；UI 不暴露模型选择。
- 接线：AgentEngine 在 openai 任务缺 key 时按 Router 语义报 MODEL_CAPABILITY_MISSING（替代笼统 MODEL_UNAVAILABLE）。
- 记忆授权：v1 scope=project_local + 跨项目 schema 层拒绝已满足"作者私有记忆不跨授权域"；路由侧图像范围不扩大由测试钉死。
- M3 其余项（≥30 任务真实质量评估、真实计费校准、竞品对比）需真模型/真人，不冒充完成。
