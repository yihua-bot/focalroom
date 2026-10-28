# FocalRoom v4.0｜实施 Backlog 与 Gate 验收矩阵

**日期**：2026-10-27  
**与主方案关系**：本文件将 G0–G11 拆解为独立可验收任务，不能代替 01_MASTER_PLAN.md 对业务边界的说明。  
**完成判定**：提交代码、自动化测试、真实设备或实际后端测试证据、权限验证、迁移/回滚方案、可追踪变更记录，全部满足才标记 DONE。

## 0. 工程问题管理规则

每条 Issue 使用 `FRM-###` 稳定编号，字段：owner、scope、interfaces、files、data_migration、security_review、acceptance_tests、rollback_plan、evidence、dependencies、state。状态：`SPEC → READY → IMPLEMENTING → CODE_REVIEW → TESTING → ACCEPTED / BLOCKED`。高风险数据/安全功能禁止跳过测试直接合并。

## G0：可重现的基线与风险收敛

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-001 | 锁定当前 HEAD 与现有功能/未完成功能清单 | 无 | SHA、功能矩阵、更新的 ADR | 逐项关联源文件与测试，不凭提交标题宣称完成 |
| FRM-002 | 现有 CI 全门禁修复 | 001 | 测试运行记录与失败分类 | 核心 Rust/TS/云端/Playwright 门禁通过 |
| FRM-003 | 旧 SQLite 项目备份/恢复/迁移冒烟 | 001 | 恢复脚本与演练报告 | 旧数据、评论、序列、AI 历史可恢复 |
| FRM-004 | 写入端、权限、媒体上传威胁模型 | 001 | 安全清单与负面用例 | 证实双主/越权/误传图被阻止 |

## G1：共享领域核心

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-005 | 项目/资产/变体/渲染/节点/批注统一 ID 契约 | 001 | Schema v3 + 映射规则 | Mac/云端/移动 ID 不冲突、不靠文件名匹配 |
| FRM-006 | Rust 领域纯函数抽取 | 005 | focal-domain crate | 同样输入得到一致的领域输出 |
| FRM-007 | Canvas Command / Merge / Undo 引擎提取 | 006 | focal-commands crate | 现有桌面功能回归 100% 通过 |
| FRM-008 | OpenAPI 3 + JSON Schema + Swift/Kotlin/TS 客户端生成 | 005 | CI 自动生成产物 | 破坏性协议变更不经过版本升级不能合并 |
| FRM-009 | 旧本地数据库迁移/影像映射 | 005,006 | SQLite migrations | 历史记录不被伪造，幂等回迁演练 |
| FRM-010 | 客户端能力协商与旧版保护 | 008 | capability endpoint | 旧版本不能覆盖新版本写入 |

## G2：云端权威与同步

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-011 | LOCAL_ONLY / PRIVATE_SYNCED / SHARED 状态机 | 005,009 | migration adapter | 单主、失败可恢复、旧 SHARED_CACHE 不丢 |
| FRM-012 | 首次 bootstrap 与对象映射 | 011 | bootstrap endpoint | 迁移中断不会生成部分可编辑云项目 |
| FRM-013 | 持久事件序列、游标和快照 | 011,008 | event store | WS 丢 100 条后可全量正确追平 |
| FRM-014 | 本地 outbox、重试、UNKNOWN 回执查询 | 013 | Rust sync crate / native adapter | 杀进程恢复、幂等不双写 |
| FRM-015 | 严格 ACL 的正式命令、评论和 AI 任务 scope | 011,008 | authority service | 所有角色和撤权负面测试通过 |
| FRM-016 | 冲突仲裁、三方 merge、版本检测 | 007,013 | merge API | 无 last-write-wins 误覆盖 |
| FRM-017 | 云端设备注册、登录、退出和撤销 | 015 | device endpoints | 被撤设备无法再取得新访问票据 |
| FRM-018 | 各端事件追平和冲突可视状态 | 014,016 | SyncCenter SDK | 断网编辑可识别未提交与冲突 |

## G3：专业媒体与色彩

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-019 | Mac Color Engine 主 Viewer / Canvas 接入 | 001 | native render adapter | 16-bit TIFF/ICC 基准验证，不再只靠 1400px JPEG |
| FRM-020 | 云端受控派生物多策略、上传/下载协议 | 005,015 | derivative worker / manifest | AI/检视/印前管线隔离；哈希与 ICC 明确 |
| FRM-021 | 高像素分块、缓存键与生命周期 | 019,020 | tile service | 100MP 源按需读取、OOM 防护 |
| FRM-022 | iOS/iPadOS 原生高保真 Viewer | 020 | color-managed viewer | P3/sRGB/ICC 真机颜色对照一致 |
| FRM-023 | Android 广色域/色彩显示与兼容 | 020 | native image adapter | 不支持色彩空间时正确降级并提示 |
| FRM-024 | 参考色彩数据集、机器/目视测试 | 019,022,023 | benchmark+报告 | 每种格式至少有可核对参考渲染 |

## G4：iPhone/Android 核心应用

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-025 | SwiftUI iPhone shell + auth | 008,017 | TestFlight internal | 账号恢复/邀请/退出/重登通过 |
| FRM-026 | Kotlin Compose shell + auth | 008,017 | Android internal | Play 测试设备登录闭环 |
| FRM-027 | 系统 Photos Picker/Android Picker 与本地入库 | 005,025,026 | import pipeline | 不申请全相册权限也可完成导入 |
| FRM-028 | 项目首页/Library/Contact Sheet | 012,018,027 | 项目与选片主界面 | 1000 张项目可浏览、批量选片一致 |
| FRM-029 | 单张/双张专业检视及缓存状态 | 022,023,028 | Mobile Viewer | 原图/预览状态清晰，不错色 |
| FRM-030 | 研究与灵感、系统语音入口 | 025,026,018 | Research UI | 离线笔记多端无丢失 |
| FRM-031 | 同步中心和项目离线配置 | 018,028 | Sync settings | 离线/受限流量/缓存清理路径清楚 |

## G5：统一 Focal AI

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-032 | Unified Conversation/TaskRun schema 和读取 | 005,008 | Agent contract | 桌面与移动同会话连续并幂等 |
| FRM-033 | FocalChat 进入已有 Agent Runtime，不双轨 | 032 | unified orchestrator | 自然语言无需切模式即可调用工具 |
| FRM-034 | 视觉索引、实际照片观察与来源链 | 020,033 | Vision tools | 引用的图像确实被模型查看过 |
| FRM-035 | 多模态授权/费用/失败恢复 | 015,033,034 | consent flow | 缺授权不上传、失败不重扣 |
| FRM-036 | 端到端流式工具步骤与草案卡 | 033,025,026 | Agent UI | 用户可见任务状态与执行证据 |
| FRM-037 | 项目/作者审美记忆建议与撤回 | 033 | memory service | 长期偏好必须经明确采纳 |
| FRM-038 | 专业摄影师盲评系统 | 034,036 | benchmark | 证据准确性与编辑质量有复现记录 |

## G6：手机画布与协作

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-039 | iPhone/Android Canvas Viewer + 分级画质 | 028,029,007 | viewport UI | 大画布平移/缩放顺畅 |
| FRM-040 | 移动触控命令与评论、画笔 | 039,016 | Canvas interaction | 坐标、节点身份、触控手势可追溯 |
| FRM-041 | AI 草案比较、部分采纳、审稿卡 | 036,039,016 | Editorial Review Mobile | 选 3/20 只采纳对应合法操作 |
| FRM-042 | 离线草案/在线正式采纳/冲突恢复 | 041,018 | merge UX | 未获回执不能显示正式完成 |
| FRM-043 | 多人批注/@、权限、推送通知 | 015,039 | collaboration mobile | Reviewer 永远不能越权合并 |

## G7：iPad 专业创作

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-044 | iPad 三栏多窗口与外设操作 | 025,039 | iPad shell | 不将 iPhone 页面机械放大 |
| FRM-045 | PencilKit 转跨端矢量笔迹 | 044,007 | stroke adapter | Mac/Web/Android 同样可解释笔迹 |
| FRM-046 | 无限画布性能与多节点编辑 | 044,045,021 | Canvas Pro | 大规模真实作品不卡死、不丢点 |
| FRM-047 | iPad 专业审稿和摄影书跨页查看 | 041,044 | editorial/preview UI | 分屏批注和部分采纳完整 |

## G8：调色往返与版本回流

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-048 | Lightroom Classic collection/publish 深度衔接 | 005,020 | LR bridge v2 | 原片身份/虚拟副本和输出一致 |
| FRM-049 | Capture One Session/Catalog 扩展与可靠回流 | 005,020 | C1 bridge v2 | 手动匹配错误可检测和纠正 |
| FRM-050 | 新 Rendition 与画布更新策略 | 048,049,016 | version notification | 不自动覆盖锁定成片，移动端获通知 |

## G9：出版与交付

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-051 | 摄影书 Layout Engine / 跨页组件 | 005,020 | layout workspace | 从 Sequence 到自由跨页再到输出一致 |
| FRM-052 | 展览墙面和艺术纸输出模板 | 051 | exhibition studio | 展览尺寸与裁切版本可追溯 |
| FRM-053 | ICC 软打样、预检、PDF/X/外部专业渲染 | 019,051 | print pipeline | 指定印厂真实样书和印刷规范通过 |
| FRM-054 | 手机/网页出版审阅状态 | 051,043 | publishing review | 跨页评论定位准确；无误称正式签样 |

## G10：发布与安全运营

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-055 | iOS/Android 订阅和跨端权益 | 017,025,026 | billing adapter | 不重复购买、恢复购买、退款回收 |
| FRM-056 | 数据导出/删除、儿童肖像/保密设置 | 015,020 | privacy controls | 删除与撤回经过真实账本测试 |
| FRM-057 | 端到端负载/恢复/混沌/安全评审 | 所有 G | CI + device farm | 所有阻断项清零 |
| FRM-058 | Beta 反馈闭环与多专业用户真人测试 | 028,038,041 | 研究和验收报告 | 问题记录、修复验证、可复现 |
| FRM-059 | App Store / Google Play / Mac 发行与运营 | 055–058 | 可发布版本 | 真实审核、隐私、稳定性、支付门禁 |

## G11：可选专业生态

| ID | 任务 | 依赖 | 交付物 | 验收 |
|---|---|---|---|---|
| FRM-060 | 专业编辑服务/导师角色及委托 | G10 | 服务对象与授权 | 人类服务不绕过照片保密规则 |
| FRM-061 | 服务交付/报价/版权/投诉机制 | 060 | 服务协议和流程 | 可实际完成交易争议与履约 |

---

## 关键端到端测试清单

| 编号 | 真实测试用例 | 预期结果 |
|---|---|---|
| E2E-01 | Mac 旧版项目迁移到私人多端项目，中途网络断开 | 不出现双主、无丢失，恢复后可以继续 |
| E2E-02 | Mac 1000 张图库，手机首次打开 | 先展示已授权缩略预览，按需获取大图 |
| E2E-03 | 手机用系统 PhotoPicker 选择 20 张，没有全相册授权 | 可以完成导入/上传用户选定文件 |
| E2E-04 | 同一照片从手机导入，再在 Mac 导入相同内容 | 身份映射正确，不制造隐蔽重复 |
| E2E-05 | 移动端离线评分 30 张后恢复网络 | 按业务规则合并或报冲突，不静默丢失 |
| E2E-06 | Mac 与 iPad 同时移动同一节点 | 明确报告冲突，允许人工选择，不最后写入胜出 |
| E2E-07 | Mac 删除节点，手机离线对该节点批注 | 评论保留/标记失效，不能绑定到错误节点 |
| E2E-08 | 三方草案 20 条 AI 改动，只采纳 3 条 | 事务只应用对应合法改动和依赖项 |
| E2E-09 | AI 合并后人工又编辑另一张照片，再补偿撤销 AI | 保留后续不相关人工修改 |
| E2E-10 | 手机提交改动成功，HTTP 回执丢失，自动重试 | 恢复同一回执，不重复执行 |
| E2E-11 | 项目成员被撤权，但已缓存高质量图像 | 新媒体访问被拒，已下载像素风险有明确策略 |
| E2E-12 | Focal AI 在手机分析未授权上云的 Mac 原片 | 拒绝/请求显式授权，不私自读取磁盘 |
| E2E-13 | 手机聊天→Mac 继续同会话→iPad 查看草案 | Conversation/Task/Board 关联一致 |
| E2E-14 | Agent 模型超时，手机进入后台 | 后端任务可恢复，额度不重复扣费 |
| E2E-15 | 无权限 Reviewer 对 AI 草案点击采纳 | UI 没有正式按钮，服务端也拒绝 |
| E2E-16 | 16-bit Adobe RGB TIFF 查看于 Mac 与 iPad | 与参考 CMM 变换在定义容差内一致 |
| E2E-17 | ICC 缺失/损坏或屏幕色彩空间切换 | 显示状态告警，不静默当 sRGB/P3 真值 |
| E2E-18 | 100MP TIFF 缩放至 200% | 按需瓦片读取，性能无资源失控 |
| E2E-19 | LR 同一原片两种调色版本来回导入 | SourceAsset 相同，Variant/Rendition 分开 |
| E2E-20 | 输出摄影书时某个页面仍引用 1400px AI 缓存 | 印前拒绝并定位页面/图框 |
| E2E-21 | iPad Pencil 手绘笔迹，在 Android 和 Mac 打开 | 笔划位置与样式不漂移 |
| E2E-22 | 手机在蜂窝网络选择“只 Wi-Fi 下载”后关闭 App | 系统后台策略尊重用户偏好 |
| E2E-23 | 账号远程注销某设备，设备离线重开 | 本地明确受限状态，联网后无法继续正式写入 |
| E2E-24 | 客户端版本过旧，收到新 Canvas 操作类型 | 只读/提示升级，不能忽略未知操作然后提交 |
| E2E-25 | 非法文件、解压炸弹、畸形 ICC、超大像素图 | 不越界、不资源失控、不会绕过限制 |
| E2E-26 | 共享画布同时 3 名成员评论并查看 AI 方案 | 评论与权限一致、对象锚点稳定 |
| E2E-27 | 摄影师长期偏好建议被否定 | 不进入全局记忆，历史保留可查 |
| E2E-28 | 项目删除/撤销同步后再次登录 | 按保留策略清理缓存并且无孤儿授权票据 |
| E2E-29 | App Store 购买、恢复、退款、跨端权益 | 数字权益一致，退款不可被旧客户端绕过 |
| E2E-30 | 从手机念头到 Mac 印刷完成完整真实项目 | 资产身份、版本、批注、审批和输出回执全链闭环 |

## 发布验收决策

任何以下情况均属于 `BLOCKER`：正式作品数据无法恢复、权限越权、未经授权上传原片、参考 ICC 错色且 UI 不提示、云端双主、AI 未授权正式写入、同一计费请求重复扣费、旧版客户端覆盖新版作品、印前使用低清预览。其余性能/视觉缺陷按设备等级及对真实创作的影响定级；不能仅用单元测试覆盖率证明产品可发布。
