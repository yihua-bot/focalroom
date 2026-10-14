# v1.0 M1：统一命令、撤销与版本

用户已在 M0 交付后要求继续。本轮仅完成 M1，结束后停止评审，不实现 AI Runtime。

状态：以下七步已完成，2026-10-14 停止等待 M1 评审。实际结果见 [TEST-REPORT.md](TEST-REPORT.md)，文件清单见 [V1-M1-FILES.md](V1-M1-FILES.md)。

1. 契约先行：工程 `contracts/` 定义 GUI request、可信后端 envelope、响应和 M1 命令；从机器契约生成 Rust/TS 命令 DTO，检查生成文件一致。actor/source 由后端注入，GUI 不传路径/SQL/身份。
2. 兼容迁移：新增 0003，保留 0001/0002；增加命令回执/事件、undo/redo 游标与序列版本、比较记录的逻辑撤回。旧库迁移前使用 SQLite 一致性备份；先临时旧库迁移、回滚与恢复测试。原用户 DB 不自动升级，原生验收使用新的 M1 Review 标识。
3. 后端：唯一 CommandHandler 负责正式写入；照片导入/重链仍先通过系统授权与后台解码，然后通过同一事务边界提交。请求重放按命令 ID/幂等键与指纹验证；冲突确定失败；状态、revision、事件、补偿历史、回执同事务。
4. Undo/Redo：项目级持久化线性栈，一次多选为一组，补偿追加新事件；只能撤销/重做当前栈顶，旧组/并发 revision 不静默覆盖。新有效编辑使 redo 分支失效但保留审计。原图导入/重链不删除照片或改写原图；命名版本保持不可变，保存版本不占用编辑撤销栈。
5. GUI：撤销/重做按钮与 Cmd/Ctrl+Z、Shift+Z；版本保存/预览影响范围/明确恢复，可撤销恢复。浏览器示例实现同样交互但标注内存；真实权威状态保持 Rust/SQLite。
6. 导出：默认 CSV/JSON 不含原图/缓存绝对路径，迁移用途需显式勾选；保持源 fingerprint、失联处理与禁止覆盖。
7. 验证：契约一致、Rust 幂等/冲突/事务故障注入/跨项目/权限/持久化 Undo/Redo/版本/备份恢复；typecheck、lint、Vitest、GUI 与真实 Store E2E、fmt、Clippy、Tauri build；原生操作与完整重启、源 hash、截图。

本轮不照搬 24 表初始 schema，不预建 Agent/Draft/Memory 空模块。新增迁移仅覆盖 M1 实际能力，当前六表和旧事件保留。
