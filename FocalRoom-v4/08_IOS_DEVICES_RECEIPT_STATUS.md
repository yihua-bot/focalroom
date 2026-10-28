# FocalRoom iPhone 设备登记与回执查询开发记录

日期：2026-10-28，Asia/Shanghai。接续 `07_IOS_MIGRATION_SYNC_STATUS.md`；“同步与离线”批次第二片：设备注册/撤销与回执正式查询契约（README 后续顺序第 1 项的剩余可开发部分）。目标“按计划依次进行”，本片完成后进入第 2 项。

本轮新增一次数据库迁移 `0030_device_registry.sql`（`fr_devices` + `fr_sessions.device_id` 级联绑定），已同步更新 `account-security` 与 `min_privilege` 的迁移清单断言；其余服务端改动仍限于 `apps/api`。iPhone 继续零新增第三方依赖。

## 已实现

### 服务端

- `fr_devices(id, user_id, device_key, name, platform, created_at, last_seen_at)`，`UNIQUE(user_id, device_key)` 保证同键重复注册幂等（只刷新名称/平台/活跃时间）。`fr_sessions.device_id` 外键 `ON DELETE CASCADE`：撤销设备在一条 DELETE 内级联失效其全部会话，无独立清理路径。
- `POST /devices` 注册（幂等）、`GET /devices` 列表（含“当前会话所在设备”标记）、`POST /devices/:id/session` 将当前会话绑定到本人设备、`DELETE /devices/:id` 撤销本人设备。绑定与撤销均校验设备归属，他人设备按 404 处理。会话绑定走独立端点，未改动既有 `createSession` 写入路径。
- `GET /projects/:id/commands/:commandId` 回执查询：读 `fr_commands` 既有回执，项目读权限内可见；未知编号 404。与架构契约“未知回执必须走查询，不得盲重发”对齐。

### iPhone

- 每次安装生成稳定设备键（UserDefaults，非机密），登录/恢复后自动幂等登记并绑定当前会话；登记失败不阻断主流程，回前台重试。
- 账户页新增“已登录设备”：列出设备与平台、标记当前设备、逐台撤销；撤销当前设备等同于在此设备退出（会话级联失效后按登录失效处理）。
- 选片 UNKNOWN 恢复改为“先查后发”：按原命令编号查询权威回执，命中并校验（新版本号、事件绑定）即确认并清除待提交记录；只有查询返回 404 才按原编号重发。联调验证回执确认路径不再次推进正式版本。

## 验证结果

| 检查 | 结果与范围 |
|---|---|
| Swift 核心测试 | 22 项（3 项 HEIC/无环境联调跳过），0 失败 |
| Swift → 真实隔离后端 | 3 条旅程通过；新增设备幂等注册/绑定/列表、丢回执后回执查询确认不重发（版本只推进一次）、撤销设备即时 401 且他设备不受影响 |
| 后端 PostgreSQL 集成 | 15 套件 126 项通过；`integration.mjs` 新增设备与回执用例（幂等注册、他人设备 404、撤销级联、同 key 重注册为新设备、回执 404/403/400、`/auth/me` 失效） |
| 最小权限/迁移清单 | `min_privilege`、`account-security` 断言更新至 0030 后全部通过；`focalroom_app` 仅获 `fr_devices` 全列与 `fr_sessions(device_id)` 列授权 |
| 原生界面旅程 | 独立评审模拟器 1 项通过 0 失败（120 秒），新增账户页设备列表与本机设备断言；迁云、事件游标回归保持通过 |
| TypeScript 类型/Lint/契约 | 全部通过 |
| iPhone Release | arm64、`generic/platform=iOS` 编译通过，未配置分发签名 |
| 工程检查 | 14 份变更源码无行尾空白；SHA256 清单归档 |

证据：`apps/mobile-ios/test-results/focalroom-pg-tests-ciHoJE/`（ios-verification.json、iPhone-UI.xcresult、attachments）；日志与源码摘要 `apps/mobile-ios/test-results/devices-receipt-verification-2026-10-28/`。

本轮开始时共享仓库 HEAD 已由外部会话推进至 `72a2c7ad9bf2c1ee4e48f24338be66ccb16fe0fa`（07 记录的 f378dc1 之后），本轮未执行 commit/push，改动为工作树增量。未调用真实模型、邮件、生产 API 或生产数据库；测试素材为程序生成色块。

## 设计取舍

PRIVATE_SYNCED 正式状态机（LOCAL_ONLY→SYNC_ENROLLING→PRIVATE_SYNCED→SHARED）本轮评估后暂缓：现库 `STAGING/SHARED/ABORTED` CHECK 约束承载核心权威语义，改名或拆分需在共享工作树改动核心状态列并回归全部既有路径；而“迁云后云端权威 + 不隐式授权 + 显式共享邀请”已在功能上覆盖私有同步语义。待设备/回执/游标三轮稳定后，再以独立迁移评审收口。

## 后续仍需完成

按 README 顺序：独立离线访问/导出、研究与语音入口、离线中心、真机媒体支持矩阵（第 2 项）；随后统一 AI 会话与画布（第 3 项）、推送/StoreKit/数据权利/真机验收（第 4 项）。本轮未提交、推送、部署、签名或进入 TestFlight；远端 CI 未执行。
