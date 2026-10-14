# v1.0 M1 测试与验收

日期：2026-10-14（Asia/Shanghai）。M1 已完成实现、自动化与最终 macOS 原生验收，停止等待评审。范围为统一命令、幂等/冲突、持久化 Undo/Redo、命名版本、兼容升级与默认私密导出。M2 Runtime、Provider、Draft、ContextBuilder 和记忆引擎均未启动。

M0 报告保留在 [V1-M0-TEST-REPORT.md](V1-M0-TEST-REPORT.md)，旧版报告见 [V0.1-M1-TEST-REPORT.md](V0.1-M1-TEST-REPORT.md)。产品基线为 v1.0，工程包版本仍为 0.1.0。

## 功能验收

| 项目 | 完成内容 | 验证方式 | 结果 | 未解决问题 |
|---|---|---|---|---|
| 契约与正式写入 | 14 类严格 GUI 命令、后端注入身份、唯一 Handler | Rust/TS 漂移检查、额外字段/必填校验、原生 215 条完整 envelope 校验 | passed | Agent 工具属 M2 |
| 事务与幂等 | 状态/revision/事件/历史/回执同事务；ID/键重放、别名绑定 | 回执写入故障回滚、重启重放、两个 Store 并发冲突 | passed | 未实现 Agent task receipt |
| Undo/Redo | 多选单组、栈顶补偿、重启可恢复、新分支保留审计 | Rust、GUI、原生 Cmd+Z/Shift+Z、完整退出后重做 | passed | 旧事件不伪造追溯撤销；导入不删除照片 |
| 命名版本 | 保存选片/评分/已保存备注/顺序；影响预览；可撤销恢复 | 跨项目/版本恢复测试、原生位置变化 2 张预览 | passed | 未保存草稿不入版本；快照增长待量化 |
| 比较历史 | 逻辑撤回与重激活同一行，不改保留状态 | Rust/原生重启后重做，比较行数仍为 1 | passed | 未做审美推断 |
| 兼容升级 | 保留 0001/0002，新增六表；已提交 WAL 的一致性备份 | 临时 v2 旧库升级/二次启动/备份恢复、FK/归属预检 | passed | 用户原应用和 M0 Review DB 未升级 |
| 默认私密导出 | CSV 无路径列，JSON 移除已知资源路径字段，显式 opt-in | Rust 默认/opt-in、原生保存对话框、实际导出 SHA256 | passed | 自填备注路径原文保留；HTML 含缓存图 |
| 本地编辑与只读 | 200→25、4 星、备注/序列/版本/历史恢复 | 真实 Store、原生十二表快照、200 源 hash | passed | 原生首拖偶发性未在 M1 宣称修复 |
| 工程与布局 | 紧凑历史栏、版本对话框、现有暗房视觉 | 1920/1280 截图、最终 arm64 Debug .app | passed | unsigned；JS 548.35kB 提示 |
| 高分辨率/性能 | 保留格式、失联、取消、EXIF 回归 | Rust 本地核心测试 | passed（已测范围） | 12–24MP/500 张、p95/FPS/内存/Beta 未验 |

## 自动化结果

所有检查均实际执行。GUI E2E 使用内存 MockRepository；真实 Store 旅程使用重新构建的 feature 限制 stdio harness、临时 SQLite 和 loopback 桥。系统/事件传输替身不能证明原生行为，已另用最终原生包验收。

| 检查 | 结果 |
|---|---|
| npm run contracts:check | passed，生成产物一致 |
| npm run typecheck / npm run lint | passed，0 warnings |
| npm run test | passed，4 files / 18 tests |
| npm run test:e2e | passed，16 tests / 15.6s |
| CARGO_TARGET_DIR=target-v1 npm run test:integration | passed，1 综合旅程 / 56.3s（用例 55.4s） |
| cargo test --test command_history | passed，10 tests / 0.18s |
| cargo test --test local_core | passed，10 tests / 3.39s，无 HEIC/CR2 SKIP |
| cargo fmt -- --check | passed |
| cargo clippy --all-targets --features integration-harness -- -D warnings | passed |
| JSON Schema 校验 | 三份 schema 合法，14 示例通过，伪造 actor 拒绝，215 条原生 envelope 通过 |
| 最终 Tauri Debug build | passed，arm64 / 42.77MiB |

Rust 覆盖：14 类严格请求与必填 revision/causationId、授权先于回放、跨项目拒绝、幂等别名/指纹、并发 revision、no-op、单组撤销、重启与 redo 分支失效/旧组拒绝、版本范围/可撤销恢复、比较逻辑复用、回执 INSERT 故障全部回滚、归属触发器、包含 WAL 的旧库备份/恢复、意图/归档/恢复补偿。既有格式、EXIF、取消、失联重链、导出转义/拒绝覆盖、原图只读回归通过。

真实 Store 浏览器旅程阻断非 loopback 请求，断言外部请求 0；本地桥/Vite 仍可访问。未改系统网络；最终原生包不依赖 Vite。没有模型或上传调用，不将本次结果称为 AI 外发拦截验收。

## 最终原生重启证据

应用 Between Frames M1 Review，独立 identifier studio.betweenframes.m1review；新项目 f525a522-7894-4d87-841b-f267acd87c95。系统文件夹选择器导入 200 张 480px JPEG，新增 200、失败 0。用户原应用和 M0 Review 未退出或升级。

Shift+Right 连选 25 张保留、Cmd+Z 全组撤销、Cmd+Shift+Z 重做；批量 4 星并保存 frame-000 备注。命名版本「初选 · 25 张」保存后，调整前两张并验证排序撤销/重做。版本预览位置变化 2 张，恢复后再次撤销/重做。比较 frame-000/frame-002，保存「不确定」后撤销，原行保留、active=0。

重启前 revision=215：projects 1、photos 200、photo_decisions 200、sequence_items 25、decision_events 215、comparison_events 1、command_events 215、command_history 6、history_heads 1、sequence_versions 1、comparison_state 1、command_receipt_aliases 30。integrity_check=ok，foreign_key_check 清洁；215 条 canonical command_json 通过 domain-envelope schema，200 源 SHA256 不变。

Cmd+Q 后以进程检查确认退出，重启最终 .app 并打开同一项目。十二表只读一致性快照完全相同，评分、备注、顺序、版本和待重做操作恢复。随后 Cmd+Shift+Z 追加一次补偿：revision/decision_events/command_events 增至 216，比较仍为同一条原始行、active=1。此操作与刚重启时相等快照区分记录。再次验证 200 源 hash 无变化。

证据：[重启前](V1-M1-NATIVE-SNAPSHOT-BEFORE.json)、[刚重启后](V1-M1-NATIVE-SNAPSHOT-AFTER.json)、[重启后重做](V1-M1-NATIVE-REDO-RESULT.json)、[源 hash](V1-M1-NATIVE-SOURCE-CHECKSUMS.json)。完整快照约 12MB/份，含测试路径及嵌套回执，属于本地验收材料，不是用户私密导出。

## 实际导出

系统保存对话框中默认未勾选包含路径，回执 hash 与实际文件一致：

| 文件 | SHA256 |
|---|---|
| [JSON](native-m1-exports/m1-private.json) | 7d43659892f4c164ee99d0c4f39f739a12d666f08dd1c1c682af531a0590d6ca |
| [CSV](native-m1-exports/m1-private.csv) | 64accbb0ecbf282c1ecf67526c686aa007cf7cb5fa4a20cc95241e6aa3f21d6f |
| [接触印相 HTML](native-m1-exports/m1-contact-sheet.html) | 9adf399f70fee2ffca5186a4e9a1102165bcf1bc035a9d67eab10f8998db9b99 |

JSON schemaVersion=2/includesLocalPaths=false、200 photos/25 sequence、未含源绝对路径；CSV 无路径列；HTML 含 25 个缓存图 figure。[校验和](V1-M1-NATIVE-EXPORT-CHECKSUMS.json)。显式 opt-in 由 Rust 回归验证。人工输入文本原文保留，此机制不承诺自动匿名化备注。

## 修正与工程原则

serde unit variant 即使 deny_unknown_fields 仍接受额外字段，生成器改用空结构体，14 类拒绝额外字段测试通过。Tauri 点号事件名导致原生监听错误，逻辑 project.changed/photos.imported 映射为 project:changed/photos:imported，从契约生成双方常量；修正后重建并补验最终原生包。

历史回执早于当前 revision 时重查投影；传输失败仅以同一 envelope 重试一次，业务冲突不换 ID 静默覆盖。归档状态纳入投影，补偿后不会错误留在活动工作台。

SRP 分离事务/编辑/状态/资源/审计；DIP 保持 Repository 边界；DRY 从机器契约生成 Rust/TS；KISS/YAGNI 仅新增 M1 六表，不复制整套 24 表或预建 AI 空模块。没有新增依赖。继续使用 target-v1 避免工程移动后旧 target 的绝对路径残留，未删除旧产物或改全局配置。

## 复现命令

从 between-frames 目录执行，依赖已安装：

~~~bash
npm run contracts:check
npm run typecheck
npm run lint
npm run test
npm run test:e2e
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --test command_history --test local_core -- --nocapture
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
~~~

本机 Cargo 对命令追加以下参数，放在测试程序/Clippy 的 -- 前，不修改全局配置：

~~~text
--config 'source.crates-io.replace-with="m0-mirror"'
--config 'source.m0-mirror.registry="sparse+https://rsproxy.cn/index/"' --offline
~~~

最终构建实际命令：

~~~bash
CARGO_TARGET_DIR="target-v1" npm run tauri build -- --debug --config "tauri.m1-review.conf.json" -- --config 'source.crates-io.replace-with="m0-mirror"' --config 'source.m0-mirror.registry="sparse+https://rsproxy.cn/index/"' --offline
~~~

产物：between-frames/src-tauri/target-v1/debug/bundle/macos/Between Frames M1 Review.app。Debug 未签名/公证。使用独立 Review config 避免升级用户原库；省略该 config 使用旧 desktop identifier，启动会应用新 migration，旧操作不生成追溯撤销。

## 实际截图

真实 Rust/SQLite 浏览器：[1920 工作台](screenshots/v1-m1/workspace-1920.png)、[1280 工作台](screenshots/v1-m1/workspace-1280.png)、[比较](screenshots/v1-m1/compare.png)、[序列](screenshots/v1-m1/sequence.png)、[版本](screenshots/v1-m1/versions.png)、[空状态](screenshots/v1-m1/empty.png)。

最终 macOS/WKWebView：[工作台](screenshots/v1-m1-native/workspace.jpg)、[比较](screenshots/v1-m1-native/compare.jpg)、[序列](screenshots/v1-m1-native/sequence.jpg)、[空状态](screenshots/v1-m1-native/empty.jpg)、[版本影响](screenshots/v1-m1-native/version-diff.jpg)、[导出](screenshots/v1-m1-native/export.jpg)、[重启](screenshots/v1-m1-native/restarted.jpg)、[重启后重做](screenshots/v1-m1-native/redone-after-restart.jpg)。1280 工作台及原生版本影响截图已实际打开检查。

## 剩余问题与停止点

原生首拖偶发性需人工复核，M1 未宣称修复。100% 为最长边 1400px 缓存预览，未保存备注草稿不跨重启。高分辨率/500 张图库、目标 Mac 性能、ICC/完整 RAW、缓存配额、快照存储增长仍待后续验证。unsigned Debug 与 JS 大包提示保留。旧日志保留但无追溯 Undo，用户原 DB 未升级。

本轮停止 M1，等待 UI/UX 和恢复语义评审。M2 才做动态 Mock Runtime、Policy、任务/检查点、Draft 审批与 ContextBuilder；M3 真实视觉/consent，M4 证据记忆。原始包 AG-01–15 尚未运行，不宣称通过。见 [已知问题](KNOWN_ISSUES.md)、[文件清单](V1-M1-FILES.md)、[ADR 0004](ADR/0004-v1-m1-command-history.md)、[记忆约束](MEMORY-CONTEXT-ADDENDUM.md)。
