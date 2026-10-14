# v1.0 M3 实施计划

范围：OpenAIProvider（Responses API tool calling）、图像 consent、预览上传与视觉观察缓存、inspect/compare 真实视觉路径、图像/费用预算放开、provider 分流。完成后停止评审；记忆假设/FTS5 属 M4。依据 FINAL-IMPLEMENTATION-SPEC §6/§7/§9.4/§12.2/§13-M3。

1. 契约与提示词：inspect/compare 描述升级为「有授权时真实视觉观察，无授权时元数据降级」；新增 consent GUI 命令（grant/revoke/deny）与 AI 配置查询；`prompts/agent-system-m3.md` 与视觉观察子请求提示词分版本管理。原始需求包保留不修改。
2. migration 0005：`privacy_consents`（项目级授权：mode=task/project/deny、provider、配额与已用量、可撤回）；`photo_observations`（photo_hash+model+prompt_version 观察缓存）；重建 `model_usage` 去除 cloud_images/cost=0 硬约束；`agent_tasks` 增加 provider_id。旧库升级走新增兼容 migration 并保留 0001–0004。
3. OpenAIProvider：reqwest(rustls) 调 Responses API；tools 用 ToolRegistry 生成的 strict schema；完整本地消息序列（system/task/tool 历史），store:false 自行保存；错误分类可重试/不可重试。API key 只从环境变量读取，不进 SQLite/日志/前端；base_url 可配置（测试与兼容网关）。无 key 时任务回落 Mock，GUI 明示。
4. 视觉观察为工具执行器内的独立子请求（不入主循环上下文）：授权检查 → 观察缓存命中则零上传 → 否则读 1400px 预览 base64 上传（剥离 EXIF/GPS，JPEG 重编码）→ 严格 JSON 观察（客观标签/主观解读/不确定性）落 photo_observations → usage 计量 cloud_images/费用。AG-04：无授权不发任何图像，返回元数据降级结果并明示。
5. 预算放开：Budget 允许 max_cloud_images 0–100、费用上限保持；PolicyGate/execution 在每次上传前检查图像与费用余量，触顶转 paused。provider 由后端按配置决定（mock/openai），checkpoint 校验 providerId/promptVersion 一致；恢复不得跨 provider。
6. GUI：AI 设置区（provider 状态、密钥尾号脱敏、默认授权管理）；首次视觉任务前 consent 对话框（数量/尺寸/费用范围/目的/保存策略，粒度：本次任务/本项目可撤回/禁止云端）；AIPanel 预算表单增图像数与费用，任务计量显示图像张数与费用。
7. 测试：Provider 解析单测（tool_calls/无工具文本/错误样本）；本地 HTTP 模拟 Responses API 的闭环集成（consent 拒绝断言 0 图像外发、consent 允许断言 data URL 与缓存二次零上传、幻觉 photo_id 拒绝、图像/费用触顶暂停）；M2 全量回归；AG-01/02/03 增真实 Provider 路径样本。真实模型 E2E 需用户提供 key，无 key 不宣称通过。

KISS/YAGNI：不引入模型 SDK、向量库或流式传输；观察子请求用非流式 JSON；keychain 完整实现留 M5，本轮环境变量 + 脱敏边界，ADR 记录理由。
