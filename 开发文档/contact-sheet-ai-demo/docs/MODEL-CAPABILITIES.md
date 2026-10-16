# 模型能力配置

适用于 M3 Responses Adapter。能力决定允许请求的协议类型，不能授予图像上传、数据驻留、服务档位或专业质量资格。配置只来自可信部署端；任务目标、备注、工具参数和模型响应不能修改配置。

## 内置范围

仅精确匹配以下端点和模型，去除端点首尾空白和末尾 `/`，不按主机后缀、代理转发或相同模型名继承：

| 端点 | 模型 | 能力来源 |
|---|---|---|
| `https://api.openai.com/v1` | `gpt-4o-mini`、`gpt-4o-mini-2024-07-18` | [官方文档](https://developers.openai.com/api/docs/models/gpt-4o-mini)，2026-10-16 核对 |
| `https://api.deepseek.com` | `deepseek-flash` | 本项目既有真实工具和视觉协议冒烟，见 [M3 评测报告](V2-M3-EVALUATION-TEST-REPORT.md) |

这两类配置允许 `vision`、`tool_calling`、`structured_output`、`instruction_following`。来源分别为 `official_documentation` 和 `local_protocol_smoke`。质量仍为 `unmeasured`，云驻留仍为 `unknown`；不代表已通过专业摄影评测、长期可用性、冻结模型权重或账单校准。

## 自定义部署声明

其他模型/端点必须提供本地 JSON 文件，通过 `BETWEEN_FRAMES_AI_CAPABILITY_PROFILE` 指向其路径。配置由部署人员核对后提供，不在普通创作流程中要求摄影师选型号。设置界面编辑器和自动模型探测尚未实现。

文件形状：

```json
{
  "version": 1,
  "endpointSha256": "填写标准化后的实际 Base URL 的 SHA-256 小写十六进制",
  "modelId": "填写 BETWEEN_FRAMES_AI_MODEL 的精确值",
  "capabilities": ["tool_calling", "instruction_following"]
}
```

示例是纯文本工具编排配置。端点摘要计算规则：`sha256(base_url.trim().trim_end_matches('/').as_bytes())`；模型 ID 精确匹配，不归一化大小写或猜测别名。不要在文件中填写密钥、原图路径或授权凭据。文件有 16 KiB 上限，只读取普通文件；未知字段、未知能力、版本错误、损坏 JSON、端点/模型不符或读取失败均拒绝。显式配置无效时，内置配置也不能代替它。

能力集合可为空以禁用请求；数组顺序和重复项不改变有效集合。允许能力只有上述四项，当前没有 embedding Adapter，不能声明 `embedding_or_multimodal` 来绕过。`operator_declaration` 只表示部署声明，不冒充实测。

## 执行与恢复

- 工具编排要求 `tool_calling` + `instruction_following`；观察/策展要求 `vision` + `structured_output` + `instruction_following`；记忆结构化请求要求 `structured_output` + `instruction_following`。有图像输入时还要求 `vision`。
- Router、模型请求与视觉 Adapter 直接入口均检查能力。未满足时不发该请求；视觉预算不会赋予视觉能力，Pro 档也不会。尚未发出的图片不扣上传配额。
- 配置错误会显示在委托面板；纯文本配置仍可发起不上传照片的编辑委托。开启图像预算时提示归零或联系部署方，不能先授权再失败。授权卡打开后配置失效，会阻止继续授权；“禁止云端图像”始终保留。没有 API Key 的本地 Mock 沿用原流程。
- 实例创建时冻结配置，运行中不重读文件；修改后需重启应用。同样的端点、模型、能力集合允许轮换密钥；有效能力改变后旧任务必须新建委托。旧绑定缺少能力摘要也拒绝续跑，不补猜、不重写旧记录。
- 任务绑定追加 `capabilityProfileSha256`，不存密钥、配置文件路径或配置正文。源照片 hash/模型/端点/视觉 Prompt 的既有缓存键保持不变；能力变化要求新委托，但不会清除同源同端点已经取得的本地观察。缓存复用不是新的上传授权。
- `AiConfig.ready` 仍表示存在已配置的真实 Provider。调用方还必须检查 `configurationError` 和 `capabilities`；配置失败不能当成本地 Mock，旧前端/后端组合缺字段会拒绝解析。

当前只有一个已配置的 Responses Provider，错误重试仍在原提供方内进行。未实现跨供应商自动 fallback；本配置也不能代替新的供应商条款/图片传输授权。项目级 consent 目前仍绑定适配器，首次执行身份绑定不能补足创建时的供应商证明，后续多 Adapter 必须另行实现。
