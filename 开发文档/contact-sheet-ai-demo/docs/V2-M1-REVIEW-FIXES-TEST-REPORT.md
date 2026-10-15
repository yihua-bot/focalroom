# v2 M1：接续评审三项问题修复验收

日期：2026-10-15。范围：[评审 R1–R3](V2-M1-CONTINUATION-REVIEW.md)，以及相关测试/状态文档纠正。**三项问题已修复并复验，M1 整体继续 partial；未进入新 M2。** 架构决定见 [ADR 0028](ADR/0028-m1-review-fixes.md)。

## 功能修复与复现

| 问题 | 修复后的行为 | 实际验证 |
|---|---|---|
| R1：RAW 绕过像素边界 | 检视不再执行无约束的 sips 转换，复用并校验最长边 1400px 的已有 JPEG；类型为 `derived_preview`，不声明嵌入 JPEG 或全分辨率 | 同一 7000×5000 改名 TIFF：修复前返回 3500 万像素，修复后 **1400×1000**；替换成 1500px 缓存会报 INSPECTION_LIMIT；源 SHA-256 不变 |
| R2：重放返回后续项目状态 | 新瘦回执保存事件截止游标；仅补回当时事件，保留首次业务快照。兼容旧完整回执与 ADR 0026 无游标瘦回执 | A→B→重放 A，返回 **revision=2 / project.revision=2 / title=A**，完整回执与首次一致；别名、后续评分/版本、无操作、数据库重开均通过 |
| R3：增长基准计数错误 | 成功调用计数与回执行增量相等，按实际 1250 次计算；逻辑占用与 main/WAL/SHM 分别记录 | **1250 次 / 回执增量 1250**；基准守卫通过；原始 JSON/输出归档 |

两项 P1 的回归均先在修复前失败，再在修复后通过。[修复前失败日志](evidence/v2-m1-review-fixes/red-replay.txt)、[RAW 失败日志](evidence/v2-m1-review-fixes/red-raw.txt)、[修复后独立复现](evidence/v2-m1-review-fixes/probes.json)。复现脚本保存在同目录 `reproduce-probes.py`，需在应用根目录使用已重建的 debug harness 和本机 Pillow 运行；它只创建临时项目与生成图片，不访问用户库。

## 原图检视与 UI 边界

- 原图只读、所属项目/指纹校验与精确文件授权保持；源文件大小在计算指纹前检查。RAW 检视不再创建全局临时转换文件；已有旧临时文件未删除，也不再读取。
- RAW 返回的 JPEG 每次检查格式、大小（16MiB）与尺寸（两边均不超过 1400），不只判断文件存在。无法使用时明确报错。
- 桌面按钮使用“100% 像素”；画面区分“原图”与“RAW 派生预览 · 非全分辨率”。派生预览的页脚不再显示原图标识；Retina 比例针对实际显示源。
- JPEG/PNG/TIFF/HEIC 直通仍保留原有 64MiB / 3200 万像素边界。RAW 全分辨率检视或真正的相机嵌入 JPEG 提取仍未交付，不能因本轮修复关闭这些产品验收项。
- 复用既有缓存和共用事件查询，避免额外解码框架/重复 SQL；不新增依赖或数据库迁移，不扩大 AI 上传权限。业务写入仍经唯一 CommandHandler，审计 append-only 约束未停用。

## 校正后的存储基准

Apple M4 / 10 核 / 24GiB，release Rust；40 张 600×400 生成 JPEG，125 轮，每轮两组“1 条决策 + 两次 Undo/Redo”，合计 **1250 条 GUI 命令**。这是 CommandHandler 与准备查询的后端基准，不是 WebView 交互或真实摄影素材性能验收。

最终复测没有并行编译或 E2E；运行本次已构建的同一 release 测试二进制。编译阶段的首轮采样也保留，但以最终输出为准。

| 指标 | 最终实测 |
|---|---:|
| 实际成功命令 / 持久化回执增量 | 1250 / 1250 |
| 耗时 / 吞吐 | 15.63s / 79.97 命令每秒 |
| 导入后逻辑占用 | 606,208 字节 |
| 最终逻辑占用 | **64,733,184 字节（61.73 MiB）** |
| 每百命令逻辑增长 | 5,130,158.08 字节（约 4.89 MiB） |
| 关闭前 main / WAL / SHM | 63,594,496 / 4,247,752 / 32,768 字节 |

逻辑占用是 `PRAGMA page_count × page_size`，包含已提交但尚在 WAL 的页面；不能把主文件的 4KB 当作初始逻辑大小。事件数组仍不重复写入回执，新增加的游标不改变旧回执对外结构。旧“7.99GB→63.7MB / 500 命令”、2→28 命令每秒和每百命令增长数字撤回，不据无效计数做倍数对比。历史数据/审计记录没有为降低占用而删除。

[最终基准 JSON](evidence/v2-m1-review-fixes/growth-baseline.json)、[完整输出](evidence/v2-m1-review-fixes/growth-final.txt)。运行同一用例可复现方法，耗时与页面分配会随机器/运行状态变化。

## 测试和构建

- Rust 默认集 **120 项通过**；4 项 ignored（2 性能、2 真实格式），钥匙串写删回环 1 项过滤。随后显式运行本机 HEIC/CR2 样本 **2 项通过**、增长基准 **1 项通过**。合计 **122 项不同功能测试 + 1 项增长基准**；既有 200 图性能基准未重跑。
- 前端 **147 项 / 19 文件通过**，包含派生预览标识、延迟响应、Retina 和错误回退。
- E2E **27 条独立旅程通过**：16 预览 + 1 Web + 6 既有真实后端 + 4 M1 GUI。历史“29 条”不成立；本轮只新增 1 条 RAW 格式边界 GUI 旅程，重复执行不算新增。
- 500 图完整旅程、500 个源 hash、新 Rust 进程恢复、异常导入、导出、版本及 Undo/Redo 继续通过；RAW GUI 使用改名 PNG 验证来源标识，不冒充真实相机 RAW。
- 契约、TypeScript、ESLint、fmt、严格 Clippy（含 integration-harness）、Desktop/Web workspace 构建和独立原生 release 包构建通过。release 测试构建有既有 crate-type 输出名碰撞提示，前端有既有 bundle/dynamic-import 提示；没有更新依赖来掩盖提示。
- HEIC/CR2 缺素材时不再直接返回成功：默认显式 ignored，具备 `/tmp/between-frames-heic-fixture.heic` / `/tmp/between-frames-raw-fixture.CR2` 时单独运行；缺失则断言失败。ICC 与 16 位测试拆开，ICC 字节分支不能静默跳过；这只证明字节/缓存精度边界，不证明实际显示准确。

汇总、命令输出、关键源码摘要见 [证据目录](evidence/v2-m1-review-fixes/)。本轮开始时 75 份历史截图已逐字节保存，回归覆盖后恢复到本轮开始状态，保留了接续开发者原有修改；新截图存放于 `screenshots/v2-m1-review-fixes/`。

## 原生补验

新配置 `tauri.m1-review-fixes.conf.json` 构建 **Between Frames M1 Review Fixes.app**，应用标识 `studio.betweenframes.m1reviewfixes`，使用新库（schema 17）。没有启动新代码去升级原用户库或旧评审库。

通过真实系统选择器导入同一张 **7000×5000** 改名 TIFF，新增 1、失败 0；WKWebView 点击“100% 像素”后显示“RAW 派生预览 · 非全分辨率”，实际缓存为 **1400×1000**。源文件 SHA-256 与入库一致，检视后未变化，Agent task 为 0。截图通过 CUA 在会话内实际检查，未声称存在本地原生截图归档。[原生结果](evidence/v2-m1-review-fixes/native-results.json)。

该补验专门覆盖修复边界，不能替代原生 500 图、真实相机 RAW 全格式或低端设备验收。

## 复现与剩余

```bash
# 在 between-frames 根目录执行；1420/1422 的套件必须串行。
BF_M1_ACCEPTANCE_SLICE="v2-m1-review-fixes" npm run test:m1-gui
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" -- --skip store_load_delete_roundtrip
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --test photo_inspection -- --ignored
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --release --test perf_baseline long_edit_history_growth_baseline -- --ignored --nocapture
npm run contracts:check && npm run typecheck && npm run lint && npm test
npm run test:e2e && npm run test:web
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --locked --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
CARGO_TARGET_DIR="target-v1" npm run tauri -- build --config "src-tauri/tauri.m1-review-fixes.conf.json" --bundles app
```

M1 仍 partial，总账保持 17 pass / 7 partial；原生 500 图、真正 RAW 细节检视、真实高像素素材、专业色彩和低端硬件仍待验收。长期事件读取、更多照片/版本条件下的存储和内存需要单独评估。本轮不提交/推送，不新增 AI 服务。
