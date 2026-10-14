# M0 测试与功能验收

测试日期：2026-10-13（Asia/Shanghai）。仅执行 M0，等待 UI/UX 评审后再开始 M1。

环境：macOS arm64、Node 25.8.1、npm 11.11.0、Rust/Cargo 1.94.0、Xcode 工具链可用。依赖锁定：React 19.3.0、Vite 8.3.4、TypeScript 6.0.3、Tauri CLI 2.12.2 / Rust 2.12.3、Vitest 4.1.11、Playwright 1.64.0 / Chromium 156。

完整阅读了 README、产品与工程规格、执行指令、Demo 源码、SQL schema 与 AI JSON Schema。Playwright 实际打开并运行 `demo-standalone.html`，核对 28 张图、独立规则建议、人工选择、序列与比较作为参考；没有将该 AI 逻辑迁入新工程。

## 验收矩阵

| 项目 | 完成内容 | 验证方式 | 结果 | 未解决问题 |
|---|---|---|---|---|
| 工程初始化 | Tauri/React/TS/Vite、测试与 lint/typecheck 脚本、锁文件 | TypeScript、ESLint、Vite build、Cargo fmt | passed | 无 M0 工程阻塞 |
| 桌面窗口 | macOS arm64 Debug `.app`、受限 capability/CSP | Tauri build、开发启动、原生 AX 树和实际截图 | passed | WKWebView 全交互未完成自动化验证 |
| 应用布局 | AppShell、SideNav、ProjectHeader、暗房 tokens、可收起检查器 | Chromium 1440×960、390×844 截图和溢出检查 | passed | UI/UX 人工评审待进行 |
| 项目与空状态 | ProjectList、ProjectDialog、Hash 路由、新建/打开内存项目、EmptyState | Playwright 项目导航与空项目旅程 | passed | M1 持久化尚未实现 |
| 选片台 | ContactSheetGrid、完整比例、密度、三状态筛选、入选底带 | 仓库/组件测试、K/X/箭头端到端 | passed | 真实导入与大图库性能待 M1 |
| 照片检查器 | PhotoInspector、状态按钮、备注保存、完整大图 | 备注切换恢复、输入框快捷键隔离、图片资源检查 | passed | 未保存草稿切换会丢弃；无 100% 缩放 |
| 两图比较 | CompareModal、可选理由、不确定、焦点陷阱、Esc | 状态不改写、Tab、Esc 自动化 | passed | 同步缩放尚未实现 |
| 作品序列 | SequenceBoard、保留集合、拖拽把手、上下移动、键盘拖放 | 集合不变性单元测试、鼠标与键盘端到端 | passed | 持久化、历史版本和 Undo 留在后续阶段 |
| 辅助面板 | AIPanel、PreferencePanel、ExportDialog | 占位状态、按钮禁用、无外部请求、人工事件展示 | passed | 服务按里程碑后置 |

## 命令与结果

在 `between-frames/` 中执行：

```text
npm run typecheck                           passed
npm run lint                                passed（0 warnings）
npm run test                                passed（2 files / 12 tests）
npm run test:e2e                            passed（9 tests）
npm run build                               passed（由 Tauri beforeBuildCommand 执行）
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check   passed
Tauri macOS Debug app build                 passed
Tauri dev                                   compiled and running
```

单元/组件覆盖：隔离仓库快照、28 张素材存在、状态/序列一致、重复 K 撤销、比较不改写状态、非法照片/排序拒绝、备注幂等、内存重置、快捷键输入隔离、占位 AI、空筛选恢复与加载失败。

E2E 覆盖：原始 Demo 运行、工作台和检查器、密度与面板收起、比较/焦点陷阱、鼠标拖拽与键盘上下移动、把手键盘拖放、空项目、备注与刷新重置、后续服务边界/无外部请求、窄屏溢出。

首次测试暴露 Vite 对父目录素材的访问限制，已将工程与 `assets/` 显式加入开发/测试允许目录。截图检查发现拖拽动画中的卡片偏移，测试增加落位断言后重新生成截图。键盘拖放测试在传感器完成布局测量前发送方向键，现绑定 activator ref，并等待渲染帧后验证结果。最终回归结果如上。

## Tauri 构建证据及网络恢复

初次 `npm run tauri build -- --debug` 在 `Updating crates.io index` 阶段停滞。对索引条目的独立请求返回：

```text
curl: (28) Operation timed out after 10006 milliseconds with 25505 out of 248833 bytes received
```

停止仅由本任务启动的停滞构建后，以命令级临时镜像完成构建，未写入全局 Cargo 配置：

```bash
npm run tauri build -- --debug -- \
  --config 'source.crates-io.replace-with="m0-mirror"' \
  --config 'source.m0-mirror.registry="sparse+https://rsproxy.cn/index/"' \
  --config "http.timeout=30" --config "net.retry=1"
```

缓存就绪后，本次最后一版使用相同临时源加 `--offline` 再次构建成功。产物：

```text
between-frames/src-tauri/target/debug/bundle/macos/Between Frames.app
Mach-O 64-bit executable arm64
```

开发模式复现：先启动 `npm run dev`，复用本次已下载的缓存：

```bash
npm run tauri dev -- --no-watch \
  --config '{"build":{"beforeDevCommand":""}}' -- \
  --config 'source.crates-io.replace-with="m0-mirror"' \
  --config 'source.m0-mirror.registry="sparse+https://rsproxy.cn/index/"' --offline
```

实际输出 `Finished dev profile` 与 `Running target/debug/between-frames`。原生窗口读取到 `tauri://localhost`、28 张网格和检查器，截图确认图片资源显示。启动日志有 macOS 输入法 `IMKCFRunLoopWakeUpReliable` 提示，进程持续运行；没有据此宣称完整原生交互回归。原生自动点击因工具“用户已改变窗口”保护未执行成功，交互结果使用 Chromium 证据。

## 截图

以下为实际 Playwright 验收截图（不是设计稿）：

- [工作台与检查器](screenshots/m0/workspace.png)
- [两图比较](screenshots/m0/compare.png)
- [作品序列](screenshots/m0/sequence.png)
- [新建空项目](screenshots/m0/empty.png)
- [390px 窄屏](screenshots/m0/mobile.png)

## 文件变更与架构权衡

原 `demo-standalone.html`、`index.html`、`assets/` 及三份需求正文保持原样；根 README 追加 M0 启动入口。新增工程真实文件清单见 [M0-FILES.md](M0-FILES.md)。本目录不是 Git 仓库，没有 commit 或 push。

组件按职责拆分（SRP），仓库依赖接口（DIP），共享 Button/Modal/EmptyState 和 tokens（DRY），M0 不建立数据库或 AI 抽象（KISS/YAGNI）。Hash 路由够用且适用于桌面静态资源；Zustand 不混入业务照片数据。细节见 [DECISIONS.md](../DECISIONS.md)。

## 下一里程碑（尚未执行）

先等待 UI/UX 评审。评审通过后 M1：SQLite migration 与业务 IPC、项目 CRUD、只读照片导入、缩略图/预览和失联重链、人工状态/备注/结构化事件、序列事务与导出；执行 200 张真实图库重启一致性和原图 SHA256 不变验收。AI 服务继续留在 M2 以后。
