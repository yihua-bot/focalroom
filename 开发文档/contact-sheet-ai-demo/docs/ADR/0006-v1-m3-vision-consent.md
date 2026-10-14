# ADR 0006：M3 真实视觉 Provider 与授权链路

状态：已实现并通过自动化验收（2026-10-14）；真实模型 E2E 待用户提供 API Key 后补验。范围见 [PLAN-V1-M3](../PLAN-V1-M3.md)，测试证据见 [TEST-REPORT](../TEST-REPORT.md)。

OpenAIProvider 经 reqwest(rustls) 调 Responses API：tools 使用 ToolRegistry 生成的 strict schema，`store:false` 下多轮 function_call/output 由本地 checkpoint 完整保存与重发；错误按 5xx/429 可重试分类。API key 仅从环境变量进入进程，不落 SQLite/日志/前端（界面仅脱敏尾号）；macOS Keychain 完整存储留 M5，理由是本轮验收核心为闭环与授权，不引入新安全存储依赖。

视觉观察是工具执行器内的独立子请求，不进入主循环上下文：预取阶段持读事务完成 scope 校验、缓存命中与授权检查，网络与写事务分离，避免长事务持锁。观察缓存键为 photo_hash+model_id+prompt_version+kind，命中零上传零扣费；输入为最长边 1400px 预览重编码 JPEG（不含 EXIF/GPS），每张图前以文本标注 photo_id，输出严格 JSON 且服务端校验 photo_id 均属请求集合，幻觉引用整体丢弃。

consent 模型：privacy_consents 按 mode=task/project/deny 三态，task 模式经任务 consent_id 引用绑定；set 幂等回执，新授权自动撤旧，deny 撤回一切。配额按**真实传输张数**扣减（幻觉结果被丢弃时配额仍扣，隐私语义优先），费用为保守估算（每图 1 美分 + 每千 token 1 美分），非账单。预算 max_cloud_images 放开至 0–100，model_usage 重建去除 =0 硬约束，policy 在每次上传前检查图像/费用余量，触顶转 paused。

provider 由后端按环境决定（有 key→openai，否则 mock），前端只显示状态；checkpoint 记录 providerId/promptVersion，恢复时校验一致，不跨 provider 续跑。AG-04 保持确定性：无授权时视觉工具降级为人工元数据对照并在结果与步骤摘要中明示，零图像外发。
