# FocalRoom iPhone iOS 本期实施包

版本：North Star v4.0 / iPhone 本期范围修订；开发记录更新至 2026-10-28。

## 当前执行范围

本期只开发 **iPhone iOS 客户端**，iPad/iPadOS 与 Android 延后，不列入本期开发、测试或发行门禁。现有 Mac、Web 与云端只承担 iPhone 所需的兼容和配套工作，不另行启动新客户端或全面改版。

**先读 `04_IOS_PHASE1_SCOPE.md`**：它定义本期平台范围、原任务依赖调整及验收口径。下列 v4.0 原始多平台文档保留为背景资料；涉及平台范围、任务排期和发布门禁时，以本期范围说明为准。原 DOCX/PDF 尚未改写为 iPhone 专版，不作为本期独立执行入口。

## 包内文件

- `IOS_1_0_0_RELEASE_STATUS.md`：正式 Bundle/Team/版本、归档签名、IPA 导出与 Xcode 账号条件的接续记录。
- `IOS_1_0_0_PRODUCTION_UPGRADE_PLAN.md`：公网路由实测、生产备份/迁移/回退方案；待批准执行，不代表已部署。
- `16_IOS_COLLAB_OFFLINE_STORE_STATUS.md`：本机画布、AI 草案闭环、评论协作/邀请/授权、离线队列、StoreKit+APNs 的合并记录与用户缺陷修复。
- `15_IOS_COMPLETION_AUDIT.md`：本期最终完成审计——已交付（07–14）/ 剩余可开发 / 外部阻塞三类清单与验证累计。
- `14_IOS_READING_SURFACES_STATUS.md`：版本历史、双版本差异、提案投影与账单目录阅读面的实现、验证证据及边界。
- `13_IOS_FOCAL_AI_STATUS.md`：Focal AI 文本对话（异步提交/回执恢复/不重复计费）与项目分析任务阅读的实现、验证证据及边界。
- `12_IOS_CANVAS_PROPOSALS_STATUS.md`：手机画布编辑（moveNodes 权威命令）与 AI 草案逐项部分采纳的实现、验证证据、调试记录及边界。
- `11_IOS_SYNC_CENTER_STATUS.md`：同步中心、云端预览离线缓存与媒体矩阵自动化的实现、验证证据及边界。
- `10_IOS_RESEARCH_VOICE_STATUS.md`：研究资料（笔记/链接）消费端与 iOS 26 设备端语音转写的实现、验证证据及边界。
- `09_IOS_OFFLINE_EXPORT_STATUS.md`：本机资料离线只读访问、按账户资料索引与项目导出的实现、验证证据及边界。
- `08_IOS_DEVICES_RECEIPT_STATUS.md`：设备登记/撤销、会话级联失效与回执查询契约的实现、验证证据及取舍。
- `07_IOS_MIGRATION_SYNC_STATUS.md`：本机项目迁云、持久事件游标与同步恢复的实现、验证证据及边界。
- `06_IOS_IMPORT_COMPARISON_STATUS.md`：本机项目、系统相册/文件导入、双图比较的后续实现、验证证据及边界。
- `05_IOS_DEVELOPMENT_STATUS.md`：首批原生 iPhone 代码、验证结果、工程位置及尚未完成范围。
- `04_IOS_PHASE1_SCOPE.md`：本期执行入口，只交付 iPhone iOS；明确延期任务、依赖调整和 iPhone 验收。
- `00_正式实施方案.docx`：14 页正式产品与工程基线，可编辑。含 22 节、五端职责、14 个手机/iPad 主要页面、关键流程、UI/UX、云端/本地架构、同步、Agent、色彩、发行、实施 Gate 与验收目标。
- `00_正式实施方案.pdf`：同一主文档阅读版。
- `01_MASTER_PLAN.md`：与正式主文档同内容的 Markdown 源文件。
- `02_BACKLOG_ACCEPTANCE.md`：61 项研发任务与 30 个真实端到端验收场景。
- `03_ARCHITECTURE_CONTRACTS.md`：同步模式、事件游标、API/对象示例、离线 outbox、媒体派生、统一 AI、权限及客户端接口草案。

## 执行约束

1. 目标架构，不是已部署实现；API v3、Swift/Kotlin 应用及迁云协议等为规划项。
2. 以 `https://github.com/yihua-bot/focalroom` 固定提交 SHA 开始。2026-10-27 静态审查时最近提交为 `086d8f6536e3`；主干可能已继续变化，实施前必须再确认。
3. 按 `04_IOS_PHASE1_SCOPE.md` 收敛后的 G0–G2（基线、共享领域、同步权威）推进，再交付 iPhone；延期平台不阻塞本期。
4. Color Engine、Agent 工具及 Editorial Review 在当前仓库中已有工程进展，应复用和验证而不是重做。
5. 严格区分“已经完成代码实现”“真实设备/真人验收”和“未来最终目标”。
