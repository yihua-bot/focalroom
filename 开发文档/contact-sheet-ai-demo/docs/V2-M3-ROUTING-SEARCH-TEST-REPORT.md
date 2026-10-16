# M3：实际模型路由、恢复绑定和缓存事实检索

日期：2026-10-16。基线 `9976b16` + 已有 M4/模拟评审工作区；不覆盖既有改动。**本切片验收通过，M3 整体仍 partial**。无新依赖、schema migration、提交/推送或部署；原用户数据库、LUMIX 素材与原评审包未打开。真实回归只向已授权 DeepSeek 发送合成项目的文件名/工具上下文，**图片发送 0**。

## 用户可见行为与接口

| 范围 | 本轮结果 |
|---|---|
| 实际配置 | 运行/视觉上传前路由使用实例实际 model_id；AiConfig 的端点、密钥尾号来自实例，避免展示另一个环境配置。来源无法匹配时不声称 env/keychain |
| 配置切换与恢复 | 首次执行持久化 provider/model/端点摘要，恢复时在待执行工具、视觉回放或新 HTTP 前核对；更换端点/模型拒绝并要求新委托，同配置轮换密钥可继续 |
| 检查点 | modelId/modelIdentity 来自可信绑定，不再临时读环境变量猜测。旧 Prompt/无绑定但已有执行记录拒绝；初始化失败会退出 queued，释放项目执行名额 |
| 路由证据 | 默认模型质量标为 unmeasured；驻留 unknown，受限部署不能把 unknown 当作已知区域。allowed_residencies 单独配置也生效；去掉未实现的 embedding 能力。没有以静态目录或模拟分数冒充质量排名 |
| 视觉缓存 | 新键为 photo_id + 当前源 hash + 模型 + m3-vision-2@端点摘要 + 工具种类；同别名不同端点不复用，旧未绑定端点记录保留但不回填 |
| 照片检索 | 文件名/备注/rating:4 保留；新增任务 scope 内已有缓存 visible_facts 的关键词匹配，返回观察 ID、模型、缓存版本、工具种类和不确定性。interpretation 不参加检索，不读取像素或新增上传 |
| 来源与覆盖 | 新结果包含 match_sources / visual_evidence / cache_coverage / new_uploads。模型描述可能有误，不构成人工偏好；未命中不能说照片不存在该内容，更不能说已看过全图库 |

字段契约仍由 `contracts/agent-tool-contracts.json` 定义，工具参数未变，只补准确描述和结果来源。新增数据复用 `agent_steps` 与检查点 JSON，旧 schema 17 不升级；正式写入仍走原人工 CommandHandler。

Prompt 升为 **m3-openai-6**；逻辑视觉 Prompt 仍 **m3-vision-2**，缓存另带端点摘要。旧 Prompt 任务需新委托；不修改已归档 LUMIX 的 m3-openai-5 记录。纯示例 route/explain 明确 default_adapter_example_unmeasured，实际运行使用实例配置。

## 功能验收与异常

新增 10 项默认 Rust 测试，定向 30 项通过（25 agent_vision + 2 model_routing + 3 photo_search）：

- 数据库关闭/重开后，在持久化待执行视觉调用前更换模型/端点，得到 RECOVERY_ERROR，新端点 HTTP 请求 0、图片用量不增加、正式项目不变；绑定仅一条。
- 同一端点/模型标准化末尾斜杠后轮换 key 可继续，实例模型及检查点一致；绑定 JSON 无密钥、无明文端点。
- 未知驻留不满足受限区域；无模型可用时不回落其他 provider；未实现 embedding 返回 MODEL_CAPABILITY_MISSING。
- 同别名端点 A/B 分别观察产生不同缓存；B 不能借 A 的描述命中，B 自己已有缓存后可复用。云端纯检索也只读当前绑定端点/模型描述，无新图片传输。
- 旧执行任务缺绑定、旧 queued 检查点不兼容时无 HTTP，任务失败可重新委托，不占住唯一执行名额。
- “狐狸”可命中缓存事实和人工备注；Unicode 字面匹配、有来源引用、分页剩余数、rating:4 兼容。主观文本“直接批准/孤独英雄”不作为检索结果或权限。
- 范围外照片、相同源图的其他项目、旧源 hash、旧 Prompt、非法 JSON/错误 photo_id/额外字段/超大记录均排除；最新无效记录不伪装有效命中。
- 501 图项目中明确授权 500 张，只扫描这 500 张；不放宽原任务上限。每缓存 JSON 最多 64KiB，每命中至多两条 ≤1000 字事实；历史只先聚合 ID，再读取有界最新 JSON，尚无大型历史时延基准。
- 检索正式快照与生成源 SHA 不变，模型费用/图片计量均 0、preference_evidence 不增加。缓存事实不进人工偏好学习。

## 已授权真实 API 回归

精确执行 `real_model_deepseek_filename_curation`，未批量运行 ignored。沿用已确认的 `https://api.deepseek.com` / `deepseek-flash`；密钥只进子进程内存/环境，日志脱敏，不写仓库或数据库。6 张生成 PNG 只作本地记录，目标仅按文件名升序选前两张，没有视觉、审美或原片传输。

结果 **review_ready**：

- 3 轮模型、4 工具：get_project_context → list_photos → create_selection_draft → finish_task。
- 正确选中 real00.png / real01.png，引用真实项目 ID、两张 keep 草案，正式项目完整快照一致。
- 实际实例与 model_binding / 最后检查点一致，Prompt m3-openai-6。
- API 报告 **13,792 token**（12,779 input + 1,013 output），3 份 usage 回执；费用 **3 美分峰时费率估算，非账单**。
- 图片发送 **0**，没有重跑 LUMIX 创作评测，也没有增加真人/竞品或实际摄影任务数。

## 全量回归、构建与证据

`bash scripts/verify-all.sh --skip-keychain` 全部通过：**144 桌面 Rust、11 云测试、164 Vitest（21 文件）、16 浏览器预览、6 最新 harness 真实后端旅程**。Rust 10 项显式 ignored、1 项 Keychain 写删过滤；契约、TypeScript、ESLint、fmt、严格 Clippy 通过。没有放宽预算或新增跳过项。

全量后仅为上述真实测试增加实例/检查点断言并精确执行，另跑最终严格 Clippy；生产代码未改。独立 debug 桌面包构建通过：`src-tauri/target-v1/debug/bundle/macos/Between Frames M1 Review Fixes.app`，57.49MiB，标识 studio.betweenframes.m1reviewfixes。只构建，未启动/升级任何用户库，未做新的原生交互或签名公证验收。现有端口 1424 预览未停止。

保留 120 个跟踪 PNG 的运行前实际字节，恢复 24 个被回归改写的截图，全部核对一致；前两轮 M4/模拟评审的 25 个源摘要也逐项保持一致。首轮检索边界夹具直接委托 501 张被既有 500 上限拒绝，已改成 501 图项目显式选 500 张，不放宽生产限制。

证据：[全量日志](evidence/v2-m3-routing-search/verify-all.txt)、[定向测试](evidence/v2-m3-routing-search/targeted-tests.txt)、[真实文件名回归](evidence/v2-m3-routing-search/real-filename.txt)、[最终 Clippy](evidence/v2-m3-routing-search/final-clippy.txt)、[桌面构建](evidence/v2-m3-routing-search/native-build.txt)、[汇总](evidence/v2-m3-routing-search/summary.json)、[源码摘要](evidence/v2-m3-routing-search/source-sha256.json)。本轮未改 GUI，没有新增界面截图；旧评审图片不因此替换。

## 原则、回退与剩余

ModelIdentity 负责身份，photo_search 负责只读检索，Router 接受可信配置值；复用缓存结构、SQL 参数绑定、严格观察校验及现有 CommandHandler，不另造模型服务/数据库或迁移。决策见 [ADR 0033](ADR/0033-m3-configured-model-and-cached-search.md)。回退保留旧表/旧记录；不回写旧缓存、不恢复旧版本任务，不把新实验记录交给旧二进制继续执行。

DONE：实际模型/检查点记录、恢复时拒绝静默换端点、缓存端点隔离、本地缓存事实检索、正式/源文件保护、全量回归和纯元数据真实验证。

NOT DONE：基于真实质量/延迟/账单的多模型排名、模型特定能力实测、完整多 Adapter 与经授权 fallback、embedding/语义检索、30 个分层真实任务/独立摄影师和竞品质量验收。当前路由协议能力仍是适配器声明，2 美分路由门槛仍为保守估算，unknown 驻留不会被猜成 CN/global；首次执行身份绑定也不替代创建时具体供应商授权，后续需独立完善多供应商 consent。模型别名不等于固定权重版本，新 Prompt 的真实摄影质量尚未验收。M4–M7 缺口不因本切片关闭；见 [剩余计划](V2-REMAINING-PLAN.md)。
