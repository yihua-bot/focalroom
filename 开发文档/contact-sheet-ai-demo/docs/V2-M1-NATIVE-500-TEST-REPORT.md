# v2 M1：原生 release 500 图验收与灯箱修复

日期：2026-10-15。**生成图库的原生 500 图功能切片通过，修复一项实际灯箱布局缺陷；M1 整体仍 partial，未进入新 M2。** 本轮直接操作 Tauri/WKWebView 和 macOS 系统选择器，未用浏览器 shim 代替原生验收。此前 R1–R3 的修复与格式边界仍以 [ADR 0028](ADR/0028-m1-review-fixes.md) 为准。

## 实现与缺陷

原生打开 500 图灯箱时，底部胶片条显示成大尺寸图片并向下溢出。原因是 `.review-filmstrip`、决策布局和评分标识的 CSS 留在未被应用导入的旧 `src/App.css` 中。将这些规则移入共享入口实际加载的 `src/styles/app.css`，保留 52×40 缩略图、状态边框与横向滚动；不加载含旧脚手架全局重置的整个 App.css。

完整 M1 GUI 旅程新增几何断言：胶片条高度不超过 65px、按钮尺寸 52×40、内容宽度超过可视宽度而横向滚动。修复后重建独立 release 包，再通过原生截图实际确认单行胶片条。原生截图在 CUA 会话内检查，**没有本地原生 PNG 归档**；[本轮浏览器回归截图](screenshots/v2-m1-native-500-regression/loupe.png) 另行保存，不混称原生截图。

实现仅迁移必要样式，减少重复定义，复用现有灯箱和共享入口；无新增依赖、业务命令或数据库迁移。这是局部布局修复，无新增架构决策。

## 环境和素材

- 应用：`Between Frames M1 Review Fixes.app`，`studio.betweenframes.m1reviewfixes`；schema 17 的独立评审库。未启动新代码升级原用户库或旧评审库。
- 构建：`CARGO_TARGET_DIR="target-v1" npm run tauri -- build --config "src-tauri/tauri.m1-review-fixes.conf.json" --bundles app`。修复后的原生可执行文件 SHA-256 见 [源码摘要](evidence/v2-m1-native-500/source-sha256.json)。
- Apple M4 / 10 核 / 24GiB；硬件原始信息见 [host.txt](evidence/v2-m1-native-500/host.txt)。
- 500 张：250 PNG + 250 JPEG，全部 1200×800，总计 **305,575,284 字节（291.42 MiB）**；基于仓库 28 张参考图缩放并加入唯一内容标识。500 个 SHA-256 各异，**不属于 500 张独立摄影师原片或高像素 RAW 集**。
- 素材由 release harness 在全新临时目录准备后退出；正式验收的导入、编辑、版本、导出全部由原生 UI 完成。只读 SQLite/哈希脚本用于核对结果，不注入业务状态。

## 原生功能结果

| 项目 | 实际结果 |
|---|---|
| 系统文件夹选择器导入 | 新增 500、重复 0、失败 0；再次选择同目录时新增 0、跳过 500、失败 0 |
| 虚拟网格 | 首段 1–20、中段 225–260、末段 481–500 的 AX 与可见画面核对通过，完整构图显示正常 |
| 批量选片 | Shift 连选 25，批量保留；键盘评四星后只读库核对 25 张 keep/4、475 张 undecided/0 |
| 人工备注 | 第 25 张保存“原生 500 张验收：保留此图作为序列停顿。”，重启前后数据一致 |
| 快捷决策 | 第 500 张执行 X/U 三轮，每次 UI 状态核对，最终恢复待定，保留序列仍 25 张 |
| 灯箱与比较 | 修复后胶片条为单行缩略图；翻片通过；JPEG/PNG 按需原图、双图同步 100%→150% 显示来源和比例正确 |
| 序列与 Undo/Redo | 首张下移、撤销恢复、重做恢复；25 个序列位置互不重复 |
| 版本 | 保存“原生500选25 · 人工版本”，再次移动后恢复预览准确显示位置变化 2 张，确认后恢复；最终前 3 张为 frame-001.jpg / frame-000.png / frame-002.png |
| 系统保存面板 | 项目 JSON、选片 CSV、接触印相均实际保存并核对文件 SHA-256 与 UI 回执相等 |
| 完整退出重启 | 确认旧 PID 29204 退出，新 PID **29978**；项目 500 张 / 入选 25 / 可撤销编辑 13 / 已保存版本 1 恢复，全部 **50 张表**内容摘要完全一致 |
| 原文件和 AI | 操作前、重建后、最终重启后 500 个文件 SHA-256 均与初始清单和入库指纹一致；Agent task 为 0 |

原生系统评分弹出菜单曾使 CUA 输入出现状态冲突；通过菜单提供的 Cancel 辅助动作恢复，再使用收起状态下的键盘评分继续。用户稍后也确认关闭菜单，刷新后继续验收；没有据工具报错判定应用评分功能失败。

导出 JSON 含 500 照片、25 项序列；CSV 为 500 行数据加表头；接触印相为 25 个 figure。三者不含该图库的本机原图目录；JSON `includesLocalPaths=false`，序列与最终恢复的 SQLite 顺序一致。导出在最后六次 X/U 检查之前，后者增加审计历史，不能声称导出 revision 等于最终 revision。[导出摘要](evidence/v2-m1-native-500/exports.json)。

50 表比较包含 `_sqlx_migrations`、`sqlite_sequence` 和 FTS 辅助表；比较行内容排序后的 SHA-256，而非可能受 WAL/checkpoint 影响的数据库文件字节。重建期间也独立比较过一轮，结果一致。[重启前](evidence/v2-m1-native-500/before-restart.json)、[重启后](evidence/v2-m1-native-500/after-restart.json)、[只读复验脚本](evidence/v2-m1-native-500/verify-native-state.py)。脚本绑定本轮独立库与临时目录；素材失效后应重新准备，不应改指向用户库。

## 性能采样及限制

首次原生导入：选择 Open 到首次观察到全部完成为 **17.686s 上界**，其中含工具观察间隔；首末照片入库时间跨度 9.808s，不把它当作完整导入耗时。批量保留的自动化命令加 AX 观察往返 567ms；六次快捷决策往返 451–767ms，包含工具等待，**不是按键到绘制或 IPC 时延**。

导入至菜单阶段对主进程 PID 24230 按约 1Hz 采样 341 次，覆盖 343.60s：RSS 起始 112,448 KiB、采样最大 **126,240 KiB（123.28 MiB）**、末次 78,160 KiB；累计 CPU 增加 11.97s。该阶段在 CSS 修复前；采样在回归/编译启动前停止，包含大量空闲和菜单工具等待。

这是 **Rust/原生宿主主进程**，未可靠识别各 WebKit XPC 进程归属，不能称为应用总内存或 WebView 峰值，不能据此排除泄漏或证明低端机达标。未取得原生 GPU 利用率、真实 key-to-paint 数据；不引用 Chromium CDP/SwiftShader 数值替代。[逐次样本](evidence/v2-m1-native-500/main-process-samples.jsonl)、[方法与汇总](evidence/v2-m1-native-500/main-process-summary.json)。

## 本轮测试

- Rust 默认集 **120 passed**，4 ignored（2 性能、2 真实格式样本），钥匙串写删回环 1 filtered。无 Rust 业务源码改动；本轮未重跑真实 HEIC/CR2 或增长基准，不能将上一轮的额外 2+1 重复计入本轮执行数。
- 前端 **147 passed / 19 文件**；契约、TypeScript、ESLint、`git diff --check` 通过。
- **M1 GUI 4 passed**，含完整 500 图编辑/重启/导出、异常导入/取消/重连、Retina 原图与 RAW 派生边界，并新增胶片条几何断言。没有新增独立旅程数；项目累计基线仍为上一轮 27 条，本轮仅重跑其中 4 条。
- 独立原生 release 构建通过；保留既有 bundle 大小与动态导入提示。原生 500 图操作与上表的核对通过。
- 本轮回归截图写入新目录 `screenshots/v2-m1-native-500-regression/`，原有 75 份历史截图 SHA-256 保持一致；未提交/推送或清理其他开发者的工作区。

原始输出见 [证据目录](evidence/v2-m1-native-500/)。本轮原生旅程与浏览器回归的素材、进程和数据分别记录，不能相互冒充。

## 剩余与下一步

生成 PNG/JPEG 图库的原生功能缺口已补验，G01 增补原生证据；24 场景总账仍 17 pass / 7 partial，**M1 整体保持 partial**。下一步优先补真实相机 RAW 的受控细节检视和 EXIF/ICC/高位深样本矩阵，并取得 WKWebView 总资源与低端设备证据。当前 RAW 仍只是最长边 1400px 的校验派生预览；真实高像素摄影素材、专业色彩显示与长时间/低端硬件验收未关闭，不进入新 M2。
