# ADR 0024：M1 的 500 张 GUI 验收与性能证据

日期：2026-10-15。范围为继续补齐 M1 的 G01；不提前进入 M2，也不改变上一轮 ADR 0023 的审批/权威边界。

## 实施

复用桌面 React UI 和同一 Rust/SQLite CommandHandler，通过仅测试编译的 stdio harness 连接浏览器。原有 200 张 integration 配置保持不变；新增独立 `playwright.m1.config.ts` 和 `npm run test:m1-gui`，使用 release harness、临时数据库及临时素材。`verify:v2` 在原回归后顺序执行 500 张验收，避免 1420/1422 端口竞争。

素材生成从 harness main 分离为单一职责模块，提供 standard 与 m1-500 两个固定配置。后者生成 250 PNG + 250 JPEG、1200×800、500 个不同 SHA-256；JPEG 用唯一 COM 段，PNG 用不循环的标记。画面来自仓库 28 张参考素材的派生，明确不称作 500 张独立摄影师原片。文件选择器由测试 shim 返回项目绑定、一次性的目录授权令牌；不接受任意客户端路径。health 必须收到初始化完成的真实 harness 回执才就绪。500 张旅程使用测试服务专用的 process restart：关闭页面轮询后让旧子进程正常退出，使用同一临时目录启动新进程并断言 PID 变化；保留既有 test_restart 数据库重开语义供历史旅程回归。

异常素材独立于主 500 张：损坏 JPEG、无效 PNG 编码、600MiB 稀疏超大文件、同内容重复 PNG。通过页面触发失败与重试，取消导入仅保留已提交记录；移动/重连仅作用于临时素材，联合原目录与 relocated 目录核对完整源 hash。

旧 Rust 500 张测试原先只验证 hash 长度，不能证明未修改；现保存操作前 SHA-256，并逐张比较操作后内容及入库指纹。macOS 临时路径使用 canonicalize 对齐 /var 与 /private/var，不按字符串前缀误判。

灯箱胶片条图片加 loading=lazy，减少打开 500 张项目时的无用缩略图解码。保持原来的全部照片按钮、选择和翻片语义，不引入新的虚拟化系统或依赖。

## 性能边界

主 GUI 旅程以追踪关闭的 Chromium 和 release Rust 采样。将自动化等待耗时、keydown→DOM 状态变化后下一帧、IPC 往返分别记录，不能把 Playwright 的断言轮询时间称为应用延迟。滚动采样记录挂载卡片数、两帧时长与 CDP Script/Layout/Task/JS Heap；浏览器进程的 CPU 累计时间和 RSS 来自 CDP/ps，Rust 进程用 ps 记录。

软件 GPU 渲染器、浏览器端到端测试、1200×800 派生图，不等于原生 WKWebView、低端硬件、高分辨率真实 RAW/TIFF 或实际 GPU 利用率。GPU utilization 无法取得时记录 null 与原因，不记录为零或推定达标。本次 renderer RSS 偏高，作为后续原生/大图库内存优化与复测入口。

G01 可据实际 GUI→Rust→SQLite→导出→重启的可执行断言标 pass（测试层）。M1 整体仍 partial：原图 1:1、色彩边界、原生系统文件选择器与低端机 CPU/GPU/内存验收未关闭。

## 原则与回滚

测试配置和素材模块各负单一职责，复用原有 IPC/领域命令（SRP、DRY）；只增加当前验收所需配置与采样，不创建新业务写入通道、迁移或依赖（KISS、YAGNI）。原图与用户数据库不参与测试。

本轮新增测试文件、脚本和灯箱 lazy 属性可单独回退；不会回退或覆盖上一轮尚未提交的修复。没有应用数据迁移、提交或推送。
