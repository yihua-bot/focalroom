# 当前验收：M1 原生 release 500 图与灯箱修复（2026-10-15）

生成图库的真实系统选择器、500 图浏览/编辑、25 图序列与版本、系统保存导出和完整退出重启通过；50 张表与 500 个源 hash 一致。修复未加载胶片条 CSS 导致大尺寸缩略图溢出的问题。本轮 120 Rust 默认、147 前端、4 M1 GUI 通过；无新迁移/依赖。原生资源采样只覆盖宿主主进程，不能据此证明总内存或低端机达标。M1 整体仍 partial，继续补 RAW 细节/色彩/资源证据，不进入新 M2。见 [V2-M1-NATIVE-500-TEST-REPORT](V2-M1-NATIVE-500-TEST-REPORT.md)。以下为历史结果。

# 历史验收：M1 接续评审三项修复（2026-10-15）

RAW 检视改为校验后的 1400px 派生预览；幂等重放恢复首次完整业务/历史回执；存储基准按实际 1250 命令计算并核对回执行增量。122 Rust 功能测试、1 增长基准、147 前端单测、27 条独立 E2E、严格检查及两端/原生构建通过。独立原生包实测 7000×5000 边界图仅检视 1400×1000，源 hash 不变。报告、证据与最终基准见 [本轮报告](V2-M1-REVIEW-FIXES-TEST-REPORT.md) 和 ADR 0028。旧 29 条 E2E、每 500 命令增长数据及“RAW 嵌入预览”实现声明撤回。M1 仍 partial，总账 17 pass / 7 partial，原用户库未升级，不进入新 M2。以下为历史切片记录。

# 历史验收：M1 原图检视、GUI 内存与原生恢复（2026-10-15）

JPEG/PNG 按需原图、Retina 1:1、同步对比及明确缓存回退通过；精简历史投影减少 91.1% 载荷，40 次编辑后 JS Heap 220.8→74.4MiB。独立原生文件/文件夹选择、保存、完整退出重启通过，49 表与源 hash 前后一致。113 Rust / 146 单测 / 26 E2E、严格检查与两端/原生构建通过。见 [本轮报告](V2-M1-INSPECTION-TEST-REPORT.md) 与 ADR 0025。M1 仍 partial，总账 17 pass / 7 partial；其他原图格式、色彩、原生 500 图和低端性能待补；未升级原用户库，不进入新的 M2。以下保留历史切片结果。

# 历史验收：M1 的 500 张 GUI 切片（2026-10-15）

250 PNG + 250 JPEG 的真实文件通过 GUI 导入、滚动、人工编辑、检查、序列、撤销/重做、版本恢复、导出和新 Rust 进程重启一致性；500 个源 SHA-256 前后一致，异常与取消恢复通过。110 Rust / 142 单测 / 25 E2E 通过，见 [M1 报告](V2-M1-500-GUI-TEST-REPORT.md)。总账为 17 pass / 7 partial；M1 整体仍 partial，原生选择器、原图 1:1/色彩、内存优化和低端设备验收未关闭，不进入新的 M2 开发。以下为历史切片结果。

# 当前验收：v2 评审修复（2026-10-15）

评审八项问题已修复：提案 GUI 契约/错误处理、审批整体事务与单组撤销、位置语义、持久化幂等、Promotion 写入门槛/失败重试、Router 实际执行接线。110 Rust、142 前端单测、16 预览、1 Web、6 条真实后端旅程通过；钥匙串写删跳过，性能测试 ignored。新增 0017 仅在临时库验证，原用户库未升级。见 [本轮报告](V2-REVIEW-FIXES-TEST-REPORT.md)。

**撤回旧版“22 pass / v2 工程侧完成”结论**。当前 [总账](V2-COMPLIANCE.md) 为 16 pass / 8 partial；云服务与 Creator 权限等仍有工程缺口，不能仅归为外部账号条件。以下阶段记录和文末的早期进度保留历史，不作为当前完成声明。

# 当前验收：v2.0 M0 工程与契约切片（2026-10-15）

83 Rust、134 前端单测、17 浏览器 E2E、5 后端旅程通过，类型/契约/Lint/fmt/严格 Clippy、两端前端与独立桌面构建通过。原生 28 图导入/选片/完整退出重启通过，37 表逐行一致，28 源 hash 不变。M0 整体 partial/待架构与 UI/UX 评审；新模式持久化迁移、远端 CI 未实施/未执行，不自动进入 M1。详见 [V2-M0-TEST-REPORT](V2-M0-TEST-REPORT.md)。以下保留历史验收。

# 当前验收：M5 本地存储诊断（2026-10-15）

已完成桌面/harness 共享诊断投影、固定目录存储统计、失败/不完整状态、脱敏 JSON 预览/复制和刷新。全量 **72 Rust、56 前端单测、16 预览、5 真实后端旅程** 通过，契约/typecheck/Lint/fmt/严格 Clippy 通过；原生补验前后 35 表逐行一致、28 源 hash 不变。无新增迁移、依赖或模型任务，原用户库未升级。见 [STORAGE-DIAGNOSTICS-TEST-REPORT](STORAGE-DIAGNOSTICS-TEST-REPORT.md) 与 [ADR 0013](ADR/0013-m5-local-storage-diagnostics.md)。本轮停止等待评审；签名公证、真实模型/计价、真人 Beta、真实大图库性能等边界保留。

# 当前验收：M4 本地记忆控制（2026-10-15）

已完成假设与证据控制、原文/反例、FTS5 项目内检索、幂等回执和任务记忆版本保护。全量 67 Rust、50 前端单测、16 预览及 4 真实后端旅程通过；新增边界与最终构建复验详见 [MEMORY-CONTROLS-TEST-REPORT](MEMORY-CONTROLS-TEST-REPORT.md) 和 [ADR 0012](ADR/0012-m4-local-memory-controls.md)。钥匙串写删与性能基线本轮未执行；解锁后的原生 UI、旧任务恢复拒绝、新任务委托与完整退出重启已补验通过，35 表快照及 28 源 hash 一致。原用户库未升级，跨项目仍关闭。下文保留之前的验收历史。

# 当前评审修复验收（2026-10-14）

评审发现的六项链路缺口已修复并补回归。前一轮 **59 Rust、44 前端单测、16 预览 E2E、3 真实后端旅程通过**，契约/类型/Lint/fmt/Clippy 与独立 Debug 构建通过；钥匙串写删测试未执行。**原生系统选择器、实际 PDF 和退出重启已补验；补验发现的 Safari 页脚分页问题已修复并跑相关回归。真实模型与计价待补验。**详见 [评审修复报告](REVIEW-FIXES-TEST-REPORT.md)、[原生补验报告](NATIVE-REVIEW-FIXES-REPORT.md) 与 [ADR 0011](ADR/0011-review-fixes.md)。以下为原 M5 及版本增量历史报告。

# v1.0 M5 测试与验收（Beta 工程准备）

日期：2026-10-14（Asia/Shanghai）。M5 工程部分已完成并验收：Keychain 密钥存储、诊断命令、性能基线（Release）、Beta 完整旅程 E2E、隐私文档。**发布流程项未完成**：签名/公证（无开发者证书）、DMG 打包、20 位真实用户 Beta 与真实摄影师试用——这些需要证书与真人，工程侧无法替代，留待发布执行。

M4 报告归档于 [V1-M4-TEST-REPORT.md](V1-M4-TEST-REPORT.md)；M3 真实模型 E2E 仍待 API Key 补验。

## 功能验收

| 项目 | 完成内容 | 验证方式 | 结果 | 未解决问题 |
|---|---|---|---|---|
| Keychain 密钥 | store/load/delete + env→钥匙串解析链 + GUI 存入/移除 | keychain 2 项回环测试（独立 service）+ 旅程 | passed | 写入瞬间进程参数可见（PRIVACY 声明）；非 macOS 未集成 |
| 诊断工具 | get_diagnostics（版本/计数/配置，脱敏声明）+ GUI 复制 | Beta 旅程断言诊断内容 | passed | 缓存体积统计未含 |
| 性能基线 | Release：导入 2.9s/200 张（70 张/秒）、决策 p95 23.0ms、投影 3.4ms | perf_baseline（--ignored 手动运行，退化守卫） | passed | 数字为本机（Apple Silicon），非跨硬件承诺 |
| Beta 旅程 | 意图→导入 200→选片+备注→Mock 委托→双草案→假设纠正（AG-11 旅程级）→局部采纳→诊断→重启一致 | beta-journey E2E（真实后端 52.2s） | passed | 真实模型版本待 key |
| 隐私文档 | PRIVACY.md（照片/上传/密钥/导出/诊断/边界） | 文档评审 | — | 证据删除语义待产品决定 |
| 缓存配额清理（M1-09 补） | 孤儿缓存（无 photo 引用）按最旧回收，默认 2GB；引用缓存/原图不动 | cache_quota 2 项测试 + verify-all 全量 | passed | 引用缓存淘汰待懒重建 |
| M0–M4 回归 | 全部能力保留 | 45 Rust + 28 单测 + 16 预览 + 3 集成 | passed | — |

## 自动化结果（2026-10-14 18:30 全量复验）

| 检查 | 结果 |
|---|---|
| cargo test（target-v1，debug） | passed，45 tests（agent_runtime 13、agent_vision 4、agent_memory 4、command_history 10、local_core 10、keychain 2、cache_quota 2） |
| cargo test --release --test perf_baseline | passed（数字见上，--ignored 手动运行） |
| cargo fmt / 严格 Clippy（含 integration-harness） | passed，0 warnings |
| cargo build --release --bin between-frames | passed（unsigned） |
| npm run typecheck / npm run lint | passed，0 warnings（eslint 忽略构建产物目录） |
| npm run test | passed，7 files / 28 tests |
| npm run test:e2e | passed，16 tests |
| CARGO_TARGET_DIR="target-v1" npm run test:integration | passed，3 tests（agent-core、local-core、beta-journey 52.2s） |

## 复现命令

~~~bash
npm run typecheck && npm run lint && npm test && npm run test:e2e
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1"
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --release --test perf_baseline -- --nocapture --ignored
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --release --bin between-frames
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
~~~

## 剩余问题与停止点

签名/公证/DMG 需 Apple 开发者证书；20 位用户 Beta 与 3 位摄影师无指导试用需真人执行（验收脚本以 beta-journey 自动化打底）；真实模型 E2E 仍待 API Key；M4 遗留（跨项目假设、FTS5、证据删除入口）继续有效。

## v1.1 增量（2026-10-14，两轮）

第一轮：版本对比（A vs B / 当前序列的新增/移除/移位/人工判断变化，纯前端复用快照）与项目模板（五个静态模板预填）。第二轮：项目标签——migration 0007（tags_json）、独立通道 set_project_tags（组织元数据不占人工撤销栈、不推动 revision/不使草案 stale）、编辑对话框标签输入（、分隔）、列表标签 chips 与筛选；Rust project_tags 1 项（往返/校验/幂等/revision 不变/归档拒绝）+ 组件 9 项。第三轮：沉浸审片（版式增强）——灯箱内 K/X/U 决策与 0–5 评分（与网格同一正式命令通道）、状态与星级实时反馈、底部胶片条（状态标/星级/当前高亮/点击跳转）；组件测试 4 项。**v1.1 三项全部交付**，verify-all 全量通过（46 Rust、41 前端单测）。见 [PLAN-V1.1](PLAN-V1.1.md)、[ADR 0009](ADR/0009-v1-1-templates-diff.md)。

## v1.2 第一轮增量（2026-10-14）

印刷参考导出（format=print：A4 横版分页 HTML，封面+每页双图+裁切参考，浏览器打印存为 PDF，零新依赖）与元数据侧车（format=sidecar：每照片一 JSON + manifest，目录级只新建不覆盖，路径默认私密可 opt-in）已交付：export_v12 两项测试（3 张作品 2 页 3 plate 结构断言；侧车内容/manifest/重复导出拒绝/路径 opt-in）；导出对话框新增两个按钮。verify-all 全量通过（48 Rust、41 单测）。页跨图编辑已随后交付：册页标记（spread/full/无）经 SetPhotoBookMark 可撤销命令修改并进版本快照；印刷分页尊重标记（独占页/提前收页），book_marks 两项测试（命令往返/撤销/快照恢复 + 分页矩阵）通过。**v1.2 三项全部交付**，verify-all 全量通过（50 Rust、41 前端单测）。见 [PLAN-V1.2](PLAN-V1.2.md)、[ADR 0010](ADR/0010-v1-2-print-sidecar.md)。

## 备份可恢复性巡检增量（2026-10-15，ADR 0015）

诊断升级：最近 5 份升级前备份逐份只读 quick_check，三态报告（ok/corrupt/unreadable，SQLITE_NOTADB 区分）；界面在存储统计下列出巡检结果并明示「应用内暂无自动恢复入口」与人工评估指引。新增 backup_health 测试（三态 + 名称脱敏）；verify-all --skip-keychain exit=0 全绿（73 Rust）。自动恢复替换不做的理由见 ADR 0015。

收尾评审修复（2026-10-15）：墓碑按关键词匹配（证据数漂移不复活，回归钉死）、巡检不跟随符号链接（非普通文件不打开目标）、已纠正假设补删除入口、巡检措辞降级为「完整性通过」。修复后全量 exit=0（75 Rust、57 前端单测、5 集成）。

## v2.0 进展（2026-10-15）

M0 基座切片已提交（257ee6a）并独立复验（verify:v2 exit=0，77 项 v2 契约测试）；M1 Rust 侧复验 3 项（500 张混合集/Undo-Redo 位置唯一/超大拒绝）见 [V2-M1-GUI-REVERIFY](V2-M1-GUI-REVERIFY.md)。M2 执行桥已交付（ADR 0017）：set_selection/sequence 命令经鉴权后接入 v1 唯一通道真实执行（幂等/历史/Undo/证据继承），Agent principal 正式写入纯函数层即拒，proposal/comment 诚实 NOT_IMPLEMENTED 留 M5；v2_bridge 4 项测试。M2 工具别名完成：create_edit_proposal/revise_edit_proposal/evaluate_sequence/retrieve_memory 解析为规范工具，get_comments/summarize_review_disagreement 显式 NOT_IMPLEMENTED（M5）；v2 名字完整任务循环测试（list→proposal→finish→review_ready，正式状态零触碰）。**M2 工程侧关闭**。M3 Router 已交付（ADR 0018）：能力/区域/隐私/预算过滤 + 质量成本排序，三类可读错误（MODEL_CAPABILITY_MISSING/PRIVACY_REGION_REJECTED/BUDGET_EXCEEDED）、图像范围永不自动扩大、explain() 可解释投影不含密钥；6 场景路由测试 + engine 接线。M3 真实质量评估/计费/竞品对比需真模型与真人，不冒充完成。M5 评论切片已交付（ADR 0019）：0011 迁移 + 桥的人类通道写入（同锚点聚合/resolve 重开/跨项目锚点拒绝）+ get_comments 工具解锁（Agent 只读、不构成偏好证据）；summarize 留后续；proposal.create/submit/apply 已交付（ADR 0020）：0012 迁移、nonce 防越权、部分采纳/空集驳回、base 漂移拒绝重建、Agent 仅可创建而人工审批执行、逐操作经 v1 通道可整体撤销。v2_proposals 2 项测试（全流程+陈旧基线）。提案 GUI 已交付：get_proposals/execute_v2_command IPC（principal 为受信入口标识，客户端不可自报角色）、AI 面板旁 ProposalsPanel（列表/展开操作/nonce 表单/部分采纳/空选驳回）、组件测试 3 项。M6 组织骨架已交付（ADR 0021）：0013 迁移 + 席位上限/撤回不可自愈/OWNER 保护/审计留痕 + v2_org 测试。**V2-COMPLIANCE 总账**：24 场景全映射——22 pass（S01/S02/B01/B02 经协议层切片升级，ADR 0022）/ 2 partial（C03/E01 需真实网络两端）/ 0 missing——v2.0 工程侧完成基线。Promotion（0014）、支付账本（0015）、签名媒体链接（0016）落地；hmac 为新依赖。M1 真机性能与人工走查、M2 轨迹映射全量回归、M3+ 未启动。

六阶段工程实现至此全部落地，停止等待整体评审；规格符合性终审见 [V1.0-COMPLIANCE](V1.0-COMPLIANCE.md)。另见 [已知问题](KNOWN_ISSUES.md)、[文件清单](V1-M5-FILES.md)、[ADR 0008](ADR/0008-v1-m5-beta-readiness.md)、[隐私说明](PRIVACY.md)。
