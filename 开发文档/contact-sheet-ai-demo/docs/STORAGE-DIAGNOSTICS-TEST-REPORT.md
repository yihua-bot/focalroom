# M5 本地存储诊断增量验收

日期：2026-10-15（Asia/Shanghai）。在 M4 本地记忆补验之后，补齐既有 M5 的存储诊断缺口。本轮实现和原生验收完成，停止等待 UI/UX 评审。

## 功能验收

| 能力 | 实现与验收 | 结果 |
|---|---|---|
| 共享诊断接口 | 桌面与 harness 共用 diagnostics::collect，统一原来不同的计数结构；数据库读取失败返回明确错误，不再回退为 0 | Rust/IPC/真实旅程通过 |
| 存储统计 | 固定应用数据目录内统计主库、WAL、SHM、平面缓存、迁移备份；文件数、逻辑大小及缓存配置额度 | 已知大小夹具和原生实际统计通过 |
| 备份完整性巡检（评审后增补） | 最近 5 份备份只读 quick_check 三态（完整性通过/异常/无法打开）；符号链接不打开直接标记；措辞为完整性而非可恢复（ADR 0015 修订） | 三态 + 符号链接回归通过 |
| 不完整统计 | 目录/文件不可读、符号链接、嵌套目录、扫描上限标为不完整；只显示下界或暂不可用；缺失可选 WAL/SHM 和缓存目录按空处理 | Rust 边界与 React 状态测试通过 |
| 只读性 | 无新增迁移、清理入口、模型调用或正式命令；诊断不会修改项目/任务/记忆 | Rust 投影不变、5 条集成回归、原生 35 表一致 |
| 脱敏 | 只含版本、计数、文件大小与提供方就绪状态；不含项目名称、备注、文件名、路径、密钥尾号、端点、模型/提示词文本 | Rust 假密钥/私密文本测试与前端白名单过滤通过 |
| 预览和复制 | 先看摘要和 JSON，用户主动复制已显示的快照；复制失败提供手动复制入口；查询失败时禁止复制 | React 成功/失败测试，真实浏览器剪贴板内容等于显示 JSON；原生返回复制成功提示 |
| 刷新与关闭 | 手动刷新采集时间/统计；显示读取中状态；Escape 关闭并返回原操作界面 | 单测、浏览器与原生通过；浏览器另验焦点返回 |

## 自动化与构建

执行 `bash scripts/verify-all.sh --skip-keychain`，完整通过：

- **72 Rust tests**（新增 1 个扫描上限单测、4 个 diagnostics 集成测试）；文档测试 0 项通过；钥匙串写删 1 项跳过，性能基线 1 项 ignored。
- **14 files / 56 前端单测**，新增 6 项诊断界面/IPC 测试。
- **16 预览 E2E**，全部通过。
- **5 条真实 Rust/SQLite 集成旅程**：Agent 52.7s、Beta 11.2s、M5 诊断 1.3s、M1 11.3s、M4 6.5s；完整运行 1.4 分钟。
- 契约、typecheck、Lint、fmt、严格 Clippy（all-targets + integration-harness）通过。
- `BETWEEN_FRAMES_OPENAI_API_KEY="" CARGO_TARGET_DIR="target-v1" npm run tauri -- build --debug --config "src-tauri/tauri.memory-review.conf.json"` 成功；独立包 **Between Frames Memory Review.app / 55.34MiB**，本轮沿用独立 identifier `studio.betweenframes.memoryreview`。

诊断 E2E 验证了白名单 JSON、复制内容与界面快照相同、刷新、关闭与焦点返回，以及正式项目与源 hash 不变；没有界面脚本异常或非本机请求。Beta 旅程同步改为断言与桌面一致的 database 计数字段。原始日志见 [m5-storage-diagnostics](m5-storage-diagnostics/)。

## 原生验收

完整退出旧评审进程并启动最终包，在既有独立演示项目中通过「Agent → 模型与密钥 · 诊断 → 查看本地存储与诊断」验收。

- 显示 1 个项目、28 个照片引用、2 个既有暂停任务、6 条人工证据、9 个迁移。
- 实际主库 335,872 B、WAL 3,802,792 B、SHM 32,768 B、56 个缓存文件合计 1,784,605 B、迁移备份 0 B，已统计合计 **5,956,037 B（界面 5.7MiB）**；与文件元数据核对一致。此数值为本次观测，不作为后续启动的固定值。
- 检查 JSON、点击复制后返回成功提示；刷新时间从 10:42:30 更新为 10:43:46；Escape 正常关闭。原生复制验收记录为 API 成功提示，完整剪贴板内容一致性由真实浏览器旅程验证。
- 操作前后 **35 张表逐行完全一致**，包括任务/检查点、人工历史、记忆及 FTS5 表；正式 revision 35，原用户库未升级。
- **28 张演示源图 SHA256 全部一致**；没有启动新任务或调用真实模型。本轮无实现修改发生在完整测试之后。

证据：[原生存储界面](m5-storage-diagnostics/native-storage.jpg)、[原生 JSON](m5-storage-diagnostics/native-json.jpg)、[刷新](m5-storage-diagnostics/native-refreshed.jpg)、[结构化结果](m5-storage-diagnostics/native-verification.json)、[浏览器复制旅程](screenshots/m5-storage-diagnostics/view-and-copy.png)。验收快照含演示项目原始审计数据，保留在本机；产品诊断由白名单字段生成。

## 变更与工程原则

- 新增 diagnostics.rs、domain/diagnostics.ts、TauriDiagnosticsRepository 和 DiagnosticsDialog，分别负责后端采集、类型、IPC 校验和界面。
- commands.rs 与 harness 删除重复的诊断统计逻辑；缓存额度解析由启动清理和诊断共用。AIPanel 只负责打开诊断入口。
- 新增 Rust、React、IPC 与真实后端测试，更新 Beta/隐私/当前状态文档；无新增依赖或数据库迁移。

沿用 KISS/DRY/SOLID：复用 SQLite、现有对话框与仓库接口；统一后端逻辑；按职责拆分，诊断界面依赖只读接口。决策见 [ADR 0013](ADR/0013-m5-local-storage-diagnostics.md)。

## 剩余问题与下一步

- 统计为文件逻辑大小，不是物理磁盘占用；文件观测与数据库计数不是同一原子快照；扫描每个目录最多 100,000 条，不完整时明确标记。
- 备份计数不代表备份完整性或可恢复性，此次没有新增备份操作或清空分析缓存入口。
- 签名/公证、真人 Beta、真实模型与计价、12–24MP 代表性图库性能、钥匙串写删本轮未补验。
- 跨项目记忆与物理擦除仍未实现。后续优先级由评审决定，本轮不继续其他功能。
