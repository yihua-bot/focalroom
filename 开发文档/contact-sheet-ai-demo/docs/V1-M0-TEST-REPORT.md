# v1.0 M0 GUI 测试与验收

日期：2026-10-13（Asia/Shanghai）。范围为当前 v1.0 M0；复用旧版本地核心，完成多选、人工评分、搜索、Loupe、同步比较、面板控制与 200→25 离线旅程。本轮停止，等待 UI/UX 评审；没有进入 M1 统一命令/Undo/Redo，也没有接入 AI Runtime 或记忆引擎。

旧版 M1 报告完整保留在 [V0.1-M1-TEST-REPORT.md](V0.1-M1-TEST-REPORT.md)。需求版本不等同于 npm/Tauri 工程包版本，工程仍为 0.1.0。

## 功能验收

| 项目 | 完成内容 | 验证方式 | 结果 | 未解决问题 |
|---|---|---|---|---|
| 工程与布局 | 保留 Tauri/React，三栏、网格、检查器、Filmstrip，可折叠 | typecheck/lint、双尺寸 E2E、最终 .app | passed | Debug 未签名/公证；JS 538.31kB |
| 多选与状态 | Cmd/Ctrl 切换、Shift 范围、当前视图全选、明确 K/X/U | 单测、GUI E2E、原生 Shift 方向键选 25 张 | passed | 统一命令、Undo/Redo 属于 M1 |
| 批量事务 | 先验证完整选区归属；状态/序列/事件单事务；重复设置无操作 | Rust 非法/重复/跨项目/rollback 测试 | passed | 不是 commandId 幂等实现 |
| 搜索与人工评分 | 文件名/备注搜索；0–5 星单张/批量保存 | GUI E2E、真实 Store、原生输入隔离与 25 张 4 星 | passed | 无 AI 评分推断 |
| Loupe | fit/100% 缓存像素、缩放、受限平移、翻片、Esc/Space | GUI E2E 与原生截图/导航 | passed | 100% 缓存预览，非原图逐像素 |
| 比较 | 选中两图、同步/独立缩放平移、理由与不确定 | GUI E2E、真实 Store、原生两个模式 | passed | 比较不自动改写选片状态 |
| 快捷键与焦点 | 输入/IME/对话框/把手隔离，关闭恢复 DOM 焦点 | GUI E2E、原生搜索与备注输入 | passed | 原生仅验证关闭回工作台；精确 DOM 焦点在 Chromium 验证 |
| 序列 | 保留拖拽、键盘把手和按钮排序 | 既有 GUI 回归、原生键盘及鼠标、重启 | passed（已测操作） | 原生前两次自动拖动无变化，首拖偶发性待人工复核 |
| 导入与只读 | 系统文件夹选择器导入 200；缓存与虚拟网格 | 原生 200/200、新增 200、失败 0；源 hash 校验 | passed | 500 张与高分辨率定量性能待验 |
| 200→25 本地旅程 | 人工选片/评分/备注/比较/排序/重启 | 真实 Rust/SQLite 浏览器旅程、完整原生进程重启 | passed | 小尺寸测试图库，不外推摄影师 Beta |
| 旧库评分升级 | 新增 0002，原 0001 保持不变；旧数据保留 | 临时旧库 migration、二次打开、外键验证 | passed | 用户原应用 DB 未升级；24 表整体兼容留在 M1 |
| 记忆/上下文方案 | 约束已归档到后续阶段 | 阅读用户补充并记录 M2/M4 范围 | recorded | 算法、FTS5、预算与 checkpoint 尚未实现 |

## 自动化结果

所有命令由 Agent 实际执行。GUI 预览使用 MockRepository；真实 Store 旅程使用 feature 限制的 stdio harness 和临时 SQLite，二者不混为同一种证据。

| 命令 | 结果 |
|---|---|
| npm run typecheck | passed |
| npm run lint | passed，0 warnings |
| npm run test | passed，3 files / 15 tests |
| npm run test:e2e | passed，14 tests / 12.7s |
| 响应式网格测量修正后的针对性 GUI 回归 | passed，3 tests / 4.5s |
| CARGO_TARGET_DIR=target-v1 npm run test:integration | passed，1 完整旅程 / 54.2s（用例 53.2s） |
| cargo test --test local_core | passed，10 tests / 3.66s，无 HEIC/CR2 SKIP |
| cargo fmt -- --check | passed |
| cargo clippy --all-targets --features integration-harness -- -D warnings | passed |
| Tauri Debug Review build | passed，arm64 .app / 41.89MiB |

真实 Store 旅程阻断全部非 loopback 浏览器请求，断言外部请求为 0；本地桥与 Vite 仍可访问。系统网络未人为断开或修改，原生最终产物不依赖 Vite；生产 CSP 的 connect-src 仅允许 Tauri IPC，本轮没有网络模型/上传调用。

Rust 重点测试：200 JPEG → 25 keep → 评分 → 备注 → 反序 → 关闭/重开 DB → 快照一致；全部 200 源 SHA256 不变。新增评分迁移测试从实际旧 0001/sqlx migration 记录升级，验证 ID/title/note/revision=7/order/event/hash 不变、评分默认 0、重复启动无重复迁移、foreign_key_check 清洁。批量非法选区/重复 ID/评分 6 拒绝，投影/事件/revision 一致回滚；重复 keep 不改变已调整的顺序。

原有容错/取消/格式、EXIF、原图失联重链、归档恢复、导出转义与只读测试继续通过。原生 HEIC 与 Canon CR2 样本本次确实执行；其准备方式和授权边界见历史报告。

## 真实 macOS 重启证据

使用独立 Review 标识 studio.betweenframes.m0review，数据库位于 ~/Library/Application Support/studio.betweenframes.m0review/BetweenFrames/projects.sqlite。用户原桌面应用与其数据库未在本轮升级或退出。

原生系统选择器导入 200 张示例派生 480px JPEG；Shift+Right 选中前 25 张后保留，全部人工评分 4 星。备注框输入 k/x 不触发全局选片；原生 Cmd 多选第 1/3 张进行比较，225% 同步平移后切换独立模式，仅保存“不确定”比较记录。Loupe 支持 100% 缓存像素，翻片返回 fit，Esc 回工作台。搜索 frame-199 返回一张，继续输入 k 只改变搜索文本，keep 总数仍为 25。

前两次原生自动拖动没有改变顺序；随后键盘把手排序与鼠标跨卡片拖动均成功。最终前三张为 frame-000.jpg → frame-002.jpg → frame-001.jpg；所有 25 张序列保持。该现象保留在已知问题，等待人工首拖复核，未追加未证实的修复。

Cmd+Q 后以进程列表确认 Review 已退出，原用户应用 PID 20633 保持运行。重新构建最终 .app，再启动打开项目与序列。重启前后只读快照完全相同：

- 1 project，revision 207；
- 200 photos / 200 photo_decisions；
- 25 keep、175 undecided、0 reject；25 张 rating=4；
- 1 条已保存人工备注，25 sequence_items；
- 207 decision_events、1 comparison_event；
- SQLite integrity_check=ok，foreign_key_check 无错误；
- 200 个源 SHA256 与导入前清单完全一致。

证据：[重启前快照](V1-M0-NATIVE-SNAPSHOT-BEFORE.json)、[重启后快照](V1-M0-NATIVE-SNAPSHOT-AFTER.json)、[源 SHA256 清单](V1-M0-NATIVE-SOURCE-CHECKSUMS.json)。证据文件含本机测试夹具路径，不是面向用户的私密导出格式。

## 修正与架构权衡

- 明确 K/X/U 替代旧重复按键切换语义；测试更新到当前需求。
- macOS E2E 使用 ControlOrMeta 进行多选，避免 Control+click 打开上下文菜单。
- Radix 无 Trigger 的调用补充焦点返回逻辑，关闭查看器后可继续键盘选片。
- 虚拟网格窗口变宽后旧行高估计造成留白；布局变化主动 measure，并验证两行间隔小于 40px。
- 星级保留固定底部空间，避免动态评分改变网格行高。
- Store 批量更改沿用现有事务/Repository；共享 PhotoViewport 统一 Loupe/Compare 行为（DRY/SRP/DIP），未增加模型、向量库或自建拖动状态机（KISS/YAGNI）。
- 工程移动后旧 target 的插件元数据残留绝对路径；使用独立 target-v1 重建，未删除旧产物或修改全局配置。
- 严格 Clippy 首轮发现导入授权/任务嵌套类型过于复杂，提取两个类型别名后通过，行为不变。

## 复现与产物

在 between-frames 目录执行：

```bash
npm run typecheck && npm run lint && npm run test
npm run test:e2e
CARGO_TARGET_DIR="target-v1" npm run test:integration
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --test local_core -- --nocapture
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
```

本机 Cargo 依赖使用命令级已缓存镜像；在 Cargo 参数中、测试程序的 -- 之前追加：

```text
--config 'source.crates-io.replace-with="m0-mirror"'
--config 'source.m0-mirror.registry="sparse+https://rsproxy.cn/index/"' --offline
```

最终构建实际命令：

```bash
CARGO_TARGET_DIR="target-v1" npm run tauri build -- --debug --config "tauri.review.conf.json" --   --config 'source.crates-io.replace-with="m0-mirror"'   --config 'source.m0-mirror.registry="sparse+https://rsproxy.cn/index/"' --offline
```

产物：between-frames/src-tauri/target-v1/debug/bundle/macos/Between Frames Review.app。Debug 未签名/公证；Vite 单包 538.31kB 提示不是构建失败。

## 实际截图

浏览器截图连接真实 Rust/SQLite（不是占位渲染）：

- [工作台 1920×1080](screenshots/v1-m0/workspace-1920.png)、[1280×800](screenshots/v1-m0/workspace-1280.png)；
- [比较](screenshots/v1-m0/compare.png)、[序列](screenshots/v1-m0/sequence.png)、[空状态](screenshots/v1-m0/empty.png)。

真实 macOS/WKWebView：

- [工作台](screenshots/v1-m0-native/workspace.jpg)、[比较](screenshots/v1-m0-native/compare.jpg)、[缩放比较](screenshots/v1-m0-native/compare-zoomed.jpg)；
- [Loupe](screenshots/v1-m0-native/lightbox.jpg)、[序列](screenshots/v1-m0-native/sequence.jpg)、[空状态](screenshots/v1-m0-native/empty.jpg)、[最终重启](screenshots/v1-m0-native/restarted.jpg)。

## 下一里程碑

等待本轮 M0 UI/UX 评审后进入 M1：唯一 CommandHandler、commandId/idempotencyKey、版本/Snapshot/Undo/Redo、兼容升级、导出隐私、GUI/Agent 共享 DomainService。M2 才做动态 Mock Runtime 和 ContextBuilder；M4 才做证据驱动记忆。详见 [已知问题](KNOWN_ISSUES.md)、[文件清单](V1-M0-FILES.md)、[ADR 0003](ADR/0003-v1-m0-gui.md)、[记忆/上下文补充](MEMORY-CONTEXT-ADDENDUM.md)。
