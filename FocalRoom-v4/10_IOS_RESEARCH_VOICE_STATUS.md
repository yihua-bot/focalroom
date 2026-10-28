# FocalRoom iPhone 研究资料与语音输入开发记录

日期：2026-10-28，Asia/Shanghai。接续 `09_IOS_OFFLINE_EXPORT_STATUS.md`；目标「按计划依次进行」，本片为 README 后续顺序第 2 项中的研究与语音入口。服务端零改动——复用既有 `/api/v2` research 权威路由（权限、校验、审计与“辅助资料不进正式修订”语义均已在后端集成测试覆盖），本轮只做 iPhone 消费端与语音能力。

## 已实现

### 研究资料（笔记/链接）

- `ResearchEntry/ResearchRecord/ResearchDraft/ResearchPatch` DTO 与服务端 `researchReferenceSchema`/`researchEditSchema` 逐字段对齐：判别联合（笔记必有正文、链接必有 http(s) URL）、标题/正文/URL 与可选元数据的长度上限全部在客户端先行校验；可选字段空串一律按“不发送”处理（服务端 `min(1)` 拒绝空串，且该 API 无清除字段路径——测试断言编码后的 JSON 键集合）。
- 图库右上角“研究资料”入口：卡片式列表（类型图标、标题、正文/URL 摘要、创建者与来源元数据）、新建/编辑表单（类型在编辑时锁定）、删除带确认；观察者只读入口隐藏，权限由服务端最终裁决。
- 旅程接线：创建/更新/删除后回读列表，`projectRead` 会话边界与既有评论/画布读取一致。

### 语音输入

- iOS 26 `SpeechAnalyzer` + `SpeechTranscriber`（`.progressiveTranscription`）设备端实时转写：麦克风输入经 `AVAudioEngine` 喂入，结果按音频时间段聚合为渐进文本，先独立显示，用户确认后并入笔记正文；停止时收尾输入并清理音频管线。
- 边界诚实化：低于 iOS 26、无麦克风权限、缺中文语音资源或无音频输入时，分别给出明确原因并回退键盘，不伪装可用。音频不上传、不保存；`NSMicrophoneUsageDescription` 已加入 Info.plist，隐私清单无需新增采集类目（转写文本即用户主动保存的笔记内容，已由 `OtherUserContent` 声明覆盖）。
- 实现说明：`SpeechTranscriber` 为 iOS 26 类型，存储属性无法标注 available，会话状态经 `LiveSession` 装箱持有；`results`/`installedLocales` 均为 async（throws）属性，按 SDK swiftinterface 实测签名编码。

## 验证结果

| 检查 | 结果与范围 |
|---|---|
| Swift 核心测试 | 31 项（4 项跳过：HEIC/无环境联调）0 失败；含 6 项新 research 单测：草稿裁剪与 nil 编码、笔记禁带 url 键、链接 URL 校验（含非 http(s) 拒绝）、长度上限、补丁空串语义与键集合、列表 DTO 解码 |
| Swift → 真实隔离后端 | research 旅程通过：空列表起测 → 建笔记/链接 → 排序与创建者名 → 部分更新（标题+许可）→ Viewer 403 → 删链接 → 清理后归零（UI 以空列表起测） |
| 原生界面旅程 | 独立评审模拟器 1 项 0 失败（181 秒）：新增研究段——进入研究资料 → 空状态 → 新建笔记（含语音入口存在性断言与截图）→ 保存出列表 → 编辑标题 → 删除回空状态；迁云/设备/回执/离线/导出回归保持通过 |
| 工程检查 | 类型/Lint/契约通过（服务端与前端零改动）；Release arm64 通过；11 份变更源码无行尾空白、SHA256 归档；Info.plist lint 通过 |

证据：`apps/mobile-ios/test-results/focalroom-pg-tests-MlyClq/`（ios-verification.json、iPhone-UI.xcresult、attachments 含研究编辑与语音入口截图）；日志与源码摘要 `apps/mobile-ios/test-results/research-voice-verification-2026-10-28/`。

## 调试记录（保留为后续参照）

- `updateResearch` 初版漏传 `method: "PATCH"`，默认 POST 命中无路由 → 404「接口不存在」；经 `--serve` 夹具 curl 对照定位。
- 编辑保存把空作者/来源/许可序列化为 `""`，服务端 `min(1)` 拒绝 → 表单停留；属契约级空串语义遗漏，已修并加键集合断言。
- SwiftUI List 内 `.plain` 按钮在 iOS 26 模拟器上报 `isHittable == false`（命中被内部文本截获），改用 ScrollView+卡片（与全 App 一致）并对卡片行使用坐标触摸（相册网格同款既定解法）。
- 研究段改变了导航栈深度，原旅程的双返回会误触账户按钮；已按新深度调整并注释原因。

## 边界

语音转写的实际识别质量需真实麦克风与语音输入，模拟器 CI 无法产生真实语音——自动化覆盖到入口、状态与回退路径，识别效果列入真机人工验收。研究资料为在线读取，暂无离线缓存（随离线中心一并考虑）。图片参考与 Moodboard 维持后端既定边界（独立媒体/权限契约后做）。

## 后续仍需完成

README 第 2 项收尾：离线中心（云端资料离线缓存与待提交中心化）、真机媒体支持矩阵；随后第 3 项统一 AI 会话与画布、第 4 项推送/StoreKit/数据权利/真机验收。本轮未提交、推送、部署、签名或进入 TestFlight；远端 CI 未执行。开发期间共享 HEAD 由外部会话持续推进（本轮验证记录的 source_baseline 为 000dc03），本轮改动保持工作树增量。
