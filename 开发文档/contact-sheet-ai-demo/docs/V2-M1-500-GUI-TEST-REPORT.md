# v2 M1：500 张完整 GUI 验收与性能记录

后续进展见 [原图检视与原生验收报告](V2-M1-INSPECTION-TEST-REPORT.md)（ADR 0025）；以下保留本切片的历史基线。

日期：2026-10-15。基线：33e5ff9 + 上轮未提交评审修复；本轮仅补齐 M1 的 G01 自动化界面层。架构决策见 [ADR 0024](ADR/0024-v2-m1-500-gui-acceptance.md)。**G01 pass；M1 整体仍 partial，不进入新的 M2 开发。**

## 数据与运行环境

- 真实文件共 **500 张**：250 PNG + 250 JPEG，均为 1200×800，内容 SHA-256 各不相同；总计 291.4MiB。
- 素材从仓库 28 张参考图派生、加入唯一内容标记；不是 500 张独立摄影师原片，不用于证明真人创作质量或高像素图库性能。
- Apple M4 / 10 核 / 24GiB / arm64；Chromium 156.0.8078.4；React 界面由 Vite dev server 提供，Rust/SQLite harness 使用 release 构建。
- 浏览器通过 Tauri API shim 调用真实 Rust 后端和临时 SQLite；系统文件选择器仍模拟。只允许 loopback 请求，API Key 显式为空，Agent task 数为 0，外部请求与页面错误均为 0。
- 测试全程不打开现有原生应用数据库；没有迁移或业务 API 变更。上轮的 0017 只在新临时库中执行，原用户库未升级。

## 功能验收

| 场景 | 实际操作与断言 | 结果 |
|---|---|---|
| 新建与导入 | 页面新建项目、选择文件夹、500/500 完成；格式/尺寸/指纹与数据库逐一核对 | pass |
| 虚拟网格 | 七个首/中/尾滚动采样点，第 500 张可见，最多挂载 40 张卡片而非全部 500 张 | pass |
| 快捷键与批量编辑 | 第 500 张执行 40 次 X/U 决策；Shift 选 25 张、批量保留/4 星、状态筛选和人工备注 | pass |
| Loupe / Compare | 灯箱翻片、1200px 缓存预览的 100% 检视、双图同步缩放；胶片条不立即加载全部 500 个缩略图 | pass；原图 1:1 仍未完成 |
| 序列 / Undo / Redo | 25 张序列移动，三次撤销/重做循环；每次顺序正确且 ID 唯一 | pass |
| 作品版本 | 保存版本、再移动、预览位置差异、恢复为新命令；重启后版本与历史保留 | pass |
| 真进程重启 | 关闭页面轮询，Rust 进程 89412 退出，新进程 89581 读取同一库，整个作品投影与重启前相等 | pass |
| 导出 | 页面保存 JSON（500 张元数据/25 张序列）、CSV（500 行数据）、接触印相（25 图）；回执 SHA-256 与实际文件一致，默认不含原图路径 | pass |
| 原图保护 | 导入前保存 500 个 SHA-256，编辑/导出/进程重启后逐张对比；Rust 测试同时核对入库 hash | pass |
| 取消导入 | 500 张任务已提交 14 张时取消，新进程恢复相同部分投影，其余照片不冒充已提交 | pass |
| 重复与异常 | 重复 PNG 跳过 1 张；损坏 JPEG、无效 PNG、600MiB 稀疏大文件各自报错；页面重试仍为 0 新增/3 失败，不改已入库照片 | pass |
| 失联与重连 | 仅移动临时原图，页面显示失联但缓存可用；重新定位后 hash 一致；原目录与 relocated 联合指纹仍为 500 个且逐项相等 | pass |

## 性能实测

以下是本机测试采样，不是低端设备或原生 WKWebView 达标声明。关闭 Playwright trace，避免响应体追踪影响采样；未强制 GC。

| 指标 | 结果 | 定义 |
|---|---|---|
| 500 张导入 | 9.41s | 点击系统选择器的测试按钮至页面显示导入完成，包含约 300ms 轮询粒度，不含测试素材生成 |
| 按键到下一帧 p50 / p95 | 73.6 / 133.3ms | 40 次原生键盘事件，目标卡片状态变化后第一个 requestAnimationFrame；不是 GPU 显示器呈现时间 |
| IPC p50 / p95 | 86.3 / 127.6ms | 55 次真实 GUI 命令的 fetch→JSON 回执，不含后续 React 绘制 |
| 自动化完整步骤 p95 | 207.9ms | 包括 Playwright 往返与断言等待；不能替代应用交互延迟 |
| 滚动阶段主线程 TaskDuration 增量 | 0.1123s | 七次滚动采样的 CDP 累计差值 |
| Renderer RSS 采样 | 757.1MiB | 行为结束时进程 RSS，包含解码图片及页面状态；不是峰值 |
| JS Heap 采样 | 32.4 → 220.8 → 414.2MiB | 分别为滚动后、40 次编辑后、重启并导出后；跨页面导航不做累计时长相减 |
| Renderer CPU 累计 | 7.092s | Chromium 进程累计 CPU 时间，不是百分比或任务专属 CPU |
| Rust 进程采样 | 重启前 `324112   0:14.58`；重启后 `113520   0:00.77` | ps 原始输出：RSS KiB + 累计 CPU 时间；两者是不同进程 |
| GPU | SwiftShader 软件渲染，硬件利用率未取得 | 仅归档设备与进程信息，utilization 为 null，不宣称 GPU 性能通过 |

Renderer RSS 仍偏高；本轮只完成基线采样，后续需定位决策历史投影、查询缓存和图片解码的占用，再在原生与低端设备复测。上述值不证明内存泄漏，也不等同于生产构建前端的占用。

完整机器数据：[gui-metrics.json](evidence/v2-m1-500/gui-metrics.json)、[异常结果](evidence/v2-m1-500/exception-results.json)。

## 回归结果

- Rust **110 项通过**，性能基线 1 项 ignored，钥匙串写删回环 1 项 skip；本轮 3 项 gui_v2_m1 均通过，源 hash 断言已实质修正。
- 前端 **142 项通过**；契约检查、TypeScript、ESLint、Rust fmt、严格 Clippy（含 integration-harness）、Desktop/Web workspace 构建通过。
- E2E 共 **25 条独立旅程通过**：16 预览、1 Web、6 条既有真实后端、2 条新增 500 张 GUI。新 Node 进程驱动另补 200 张默认配置复验。
- `npm run test:m1-gui` 独立复现 2 项通过；脚本构建 release harness，测试只使用临时文件。`verify:v2` 已串联该验收，远端 CI 本轮未执行。
- 首轮验收测试修正了三处等待/路径问题：/var 与 /private/var 需要规范化；保存期间按钮 disabled 不能代替业务完成；导入 token/Job ID 返回前不能开始读取任务。修正后重跑，不把旧失败输出当作应用通过。

原始输出及计数见 [results.json](evidence/v2-m1-500/results.json)。

## 截图

[500 张网格末尾](screenshots/v2-m1-500/grid-end.png)、[灯箱](screenshots/v2-m1-500/loupe.png)、[同步对比](screenshots/v2-m1-500/compare.png)、[重启后序列](screenshots/v2-m1-500/sequence-restarted.png)、[异常导入](screenshots/v2-m1-500/import-failures.png)。历史里程碑截图保持原样。

## 剩余与停止点

**M1 仍 partial**：原生系统文件选择器/真正桌面应用退出再启动、原图 1:1 与色彩边界、独立摄影师高分辨率素材、低端机器 CPU/GPU/内存性能未验收；当前 100% 仍指编辑缓存预览。Renderer RSS 偏高需要专项优化。当前 [总账](V2-COMPLIANCE.md) 调整为 17 pass / 7 partial，仅 G01 自动化场景升级，不据此宣布 M1 或 v2 整体完成。

没有新增依赖、业务迁移或 AI 服务；不提交或推送。本轮停在 M1 验收切片，下一步继续补齐 M1，而不是自动进入 M2。

## 复现命令

在 between-frames 目录执行，相关测试共用 1420/1422，必须顺序运行；不会启动或升级现有原生评审应用。

```bash
npm run test:m1-gui
npm run typecheck
npm run lint
npm test
npm run test:e2e
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" -- --skip store_load_delete_roundtrip
CARGO_TARGET_DIR="target-v1" npm run test:integration
```

只复验异常：`npm run test:m1-gui -- --grep "异常"`。素材和数据库由脚本在系统临时目录生成，API Key 显式禁用。Web 基座 1424 可独立运行。
