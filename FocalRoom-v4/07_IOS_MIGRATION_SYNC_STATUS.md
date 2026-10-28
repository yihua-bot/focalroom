# FocalRoom iPhone 本机迁云与事件游标开发记录

日期：2026-10-28，Asia/Shanghai。接续 `06_IOS_IMPORT_COMPARISON_STATUS.md`；本轮按既定优先级推进“同步与离线”批次的第一片：本机项目迁云与持久事件游标同步恢复。其余同步与离线事项仍未完成。

本轮继续只开发 iPhone 及其必需的服务端配套。iPhone 源码位于 `开发文档/contact-sheet-ai-demo/between-frames/apps/mobile-ios`，服务端改动限于 `apps/api`，无新增第三方依赖、无新增数据库迁移。

## 已实现

### 服务端（最小增量）

- 新增 `GET /api/v2/projects/:id/events?after=<cursor>&limit=<n>`：读取既有 `fr_events` 追加日志，按项目权限校验，返回增量事件与 `nextCursor`；满页返回续读游标，空页代表已追平。事件表、快照 cursor 字段与命令事件写入均为既有实现，本轮只补读取入口。
- 复用既有 promotions 权威通道（`POST /promotions` → `PUT /projects/:id/previews/:photo` → `finalize`/`abort`）承载 iPhone 迁云，未改动其事务语义。

### iPhone

- 迁云清单与 `promotionSchema` 逐字段一致：照片身份即本机照片 UUID，选片状态随快照进入云端；序列只承载本机已“保留”照片。media_policy 固定为仅预览、去 GPS、长边 2048、不授权 AI 上传。
- 迁云记录持久化于本机项目记录：迁移键、阶段（enrolling/uploading/migrated）、原始清单字节、服务端摘要与云端项目绑定、已上传清单。重试与续跑原样重发记录中的清单字节，服务端幂等键识别为同一迁移；不创建第二个云端项目。
- 中断与恢复：断网、丢回执或进程重启后凭记录续跑；每张预览按内容寻址重传安全。显式放弃先幂等核对服务端状态再 `abort` 未提交副本；云端已提交（SHARED）后拒绝放弃。
- 冻结边界：迁云期间与迁云后，本机项目导入与选片全部冻结（`LocalLibrary` 层强制），损坏或与清单不匹配的迁移记录拒绝继续写入，不静默重建。迁云后云端项目为唯一权威，本机保留原片与预览作离线存档，提供“打开云端项目”入口。
- 上传载荷：本机 2048 像素预览经 SHA 校验后上传；极端情况下超出服务端 8 MiB 上限时仅对派生预览降低质量重编码，原片永不上传、永不改写。
- 持久事件游标：App 回到前台时按缓存快照 cursor 拉取项目增量事件，分页追平后仅在出现新事件时重新获取权威快照；离线或失败回退到既有快照/离线路径。

## 数据边界

迁云只上传派生预览与本机选片快照，不上传原片，也不向任何云模型授权图像。迁云不隐式授权其他账户（联调已验证未授权账户 403）。Mac/Web 续接使用既有共享登录、工作区与项目权限，未新增客户端功能。

本机目录仍排除系统自动备份；迁云不等于备份，README 与界面继续提示保留来源副本。账号不可用时的独立离线解锁/导出仍未实现。

## 验证结果

| 检查 | 结果与范围 |
|---|---|
| Swift 核心测试 | 21 项（2 项 HEIC 编码器相关在本机跳过），0 失败；含 4 项新增迁移记录/冻结/损坏拒绝/载荷上限测试 |
| Swift → 真实隔离后端 | 2 条旅程通过：既有回执恢复 + 新增迁云续跑（模拟上传丢回执→重启→同清单字节续跑→单一云端项目）与事件游标同步（另一设备提交→按 cursor 拉增量→刷新快照→追平） |
| 后端 PostgreSQL 集成 | 15 套件 125 项通过，0 失败；`integration.mjs` 新增项目事件游标用例（增量、分页续读、snapshot.cursor 一致、越权 403、非法游标 400、未知项目 404） |
| 原生界面旅程 | iPhone 独立评审模拟器（iOS 26.3.1），1 项通过 0 失败，116 秒；覆盖登录、云端选片、本机导入去重、双图比较、真实迁云（菜单→确认→上传→完成）、打开云端项目 3 张照片、重启后“已迁云”状态持久、退出与再重启 |
| 退出后磁盘核验 | 本机 3 份原片与预览 SHA256 与记录一致；`library.json` 迁移记录 `phase=migrated` 且绑定同一云端项目 ID |
| 云端边界（UI 旅程） | 迁云项目 state=SHARED、revision=0（finalize 不推进正式修订）、3 张照片含 1 张保留、预览 3 份；未上传原片 |
| TypeScript 类型/Lint/契约 | 全部通过 |
| iPhone Release | arm64、`generic/platform=iOS` 编译通过，未配置分发签名 |
| 工程检查 | 变更源码（后端 3 份 + iPhone 11 份）无行尾空白；源文件 SHA256 清单归档 |

证据：`apps/mobile-ios/test-results/focalroom-pg-tests-p3nfXt/`（ios-verification.json、iPhone-UI.xcresult）；完整日志与源码摘要 `apps/mobile-ios/test-results/migration-verification-2026-10-28/`。

开发期间共享仓库 HEAD 仍为 `f378dc1a3072fc39dd320fadbdfb4167672aaab2`（与 06 记录一致）。本轮未执行 commit/push；新增代码限于上述 iPhone 工程文件、`apps/api/src/server.ts`、`apps/api/src/service.ts`、`apps/api/test/integration.mjs` 与本说明文件。未调用真实模型、邮件、生产 API 或生产数据库；测试照片均为程序生成色块。

## 本轮明确不做

设备注册/撤销、PRIVATE_SYNCED 正式状态机、离线 outbox 通用化、独立离线解锁/导出、研究/语音、统一 AI 会话与画布编辑、推送、StoreKit、数据权利入口及真机/专业色彩验收。迁云后的本机离线存档只是只读副本，不等于完整离线访问能力。

## 后续仍需完成

同步与离线批次剩余：设备注册/撤销、回执查询正式契约、PRIVATE_SYNCED 状态机、完整离线中心与导出；随后按 README 后续顺序推进 AI 与画布、产品配套和发行验收。本轮未提交、推送、部署、签名或进入 TestFlight；远端 CI 未执行。iPad/Android 继续延期。
