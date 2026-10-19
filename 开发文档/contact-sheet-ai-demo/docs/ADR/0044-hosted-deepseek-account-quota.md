# ADR 0044 · FocalRoom 提供 AI 与账户额度

日期：2026-10-19。状态：本地实现与隔离验证；未生产部署。

用户选择首发由 FocalRoom 提供 AI，并指定 DeepSeek。正式桌面入口改为 FocalRoom 账户服务，不再依赖开发机的模型环境变量或要求用户输入供应商密钥。旧的 OpenAI 兼容、原生供应商适配及离线测试入口保留，正式入口不自动回退到这些服务。

## 调用与身份

原生 Keychain 会话 → FocalRoom `/api/v2/ai/deepseek-flash-v1/responses` → 固定 `https://api.deepseek.com/responses`、`deepseek-flash`。只支持同步 Responses 输入、客户端函数工具及 JSON 输出；不转发 Cookie、用户 Bearer、任意端点、远端文件、服务端搜索工具或跨请求会话 ID。供应商 Key 仅由 API 进程环境注入，不进入镜像、SQLite、浏览器或 GUI。

策略名称 v1 固定收件人/型号；服务变更须新策略和客户端授权。任务 ModelIdentity 增加可选 `accountId`，正式账户服务绑定当前云用户，并加入图像授权收件人摘要、检查点与缓存身份。正式 IPC 经 AgentEngine.request 将身份在创建任务的 SQLite 事务内冻结，不以进程是否有个人模型密钥推断供应商；缺账户/服务/额度时拒绝创建，不降级 Mock。切换账户/退出不允许续跑旧任务；HTTP 发送前重新验证会话和冻结的账户。非账户服务省略字段，旧身份字节和旧授权计算保留。

图像仍经本地 PolicyGate、任务 scope 与项目/单次授权后发送。授权卡明确列出 FocalRoom 与 DeepSeek。API 仅接受有界 JPEG data URL（最多 20 张、最长边 1400px），校验真实像素并再次重编码去 EXIF/GPS，固定 detail=low。此设置影响模型看到的细节，不能当作全分辨率审片。拒绝远端 URL、file_id、重定向和未知服务端工具。

## 额度账本

新增 0015，旧迁移不改。`fr_ai_accounts` 保存 granted/spent/reserved，单位为 input_tokens+output_tokens，非人民币、美元或已支付余额；`fr_ai_grants` 留分配流水；`fr_ai_requests` 保存用户、请求 UUID、正文摘要、策略、预留及状态；不保存提示词、照片或响应正文。`fr_ai_reconciliations` 保存人工对账依据。

必须有有效会话、已验证邮箱、相符策略和足额余额；Cookie 请求仍校验 Origin/CSRF。服务未配置明确 503，额度不足 402。默认新用户赠额 0；运营显式配置可更改新建 AI 账户的首次额度（0–1,000,000），已建立账户不随环境变化补发。未验证邮箱/未启用服务时查询不创建 AI 账户。管理员通过离线幂等工具分配，未开放任何公开赠额/充值接口，不实现支付订阅。

跨实例行锁串行化预留：完整文本 JSON（含属性、转义和工具 schema）UTF-8 字节上界 + 每图 8192 + 输出上限 + 协议余量 8192。每账户最多 2 个活跃请求、每分钟最多 30 次尝试。成功校验 usage 三项一致、输出未超限且总用量不超过预留后结算。确定的供应商拒绝（400/401/403/404/422）释放预留；超时、5xx、缺计量、超限或截断标 UNKNOWN 并保留。进程崩溃留下 PENDING，不能自动重传或自动退款。相同用户和请求编号只发送一次，重放返回 409，不缓存结果供重放。

管理工具仅在供应商计量确认后处理 UNKNOWN 或超过 10 分钟的 PENDING；拒绝活跃/已结算/超预留用量，按外部依据幂等记录并在同一事务释放/扣费。应用没有公开的人工对账接口。原生托管请求的所有不确定错误均不可自动重试；本地任务预算与原有不确定结果检查点继续生效。

## 验证与边界

官方能力依据：[DeepSeek Responses](https://api-docs.deepseek.com/guides/responses_api/)、[Vision](https://api-docs.deepseek.com/guides/vision/)，2026-10-19。文档能力不等于实际密钥可用性、专业质量或正式服务验收。

本轮只实现桌面策展/观察的账户服务。独立的云评论 Worker 仍使用原任务授权和部署配置，没有接入此 token 账本；本轮不启用其生产服务，不把它计作统一额度已完成。真实模型请求、真实邮件、生产迁移/发布、公证和干净机器验收另行执行。真人质量评测与协作验收门槛保持未关闭。
