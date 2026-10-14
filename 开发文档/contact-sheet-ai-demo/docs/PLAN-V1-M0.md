# v1.0 M0 补齐实施计划

日期：2026-10-13。用户要求继续开发，保留原 GUI 和本地核心；完成 M0 后停止评审，不提前接入模型或记忆推断。

1. 多选、范围选择与明确 K/X/U：Zustand 仅保存 UI 选区；Repository 新增批量能力，Rust Store 在一个事务中校验所有项目归属、更新投影并追加一个人工事件。重复设置不隐式切换待定。
2. 搜索、人工星级与快捷键：按文件名/备注过滤，不触发模型；评分 0–5，0 表示未评分。新增兼容 rating migration，保留已有 0001 校验；测试升级旧库，原项目 ID/备注/顺序保留。
3. Loupe 与 Compare：共用可平移/缩放的缓存预览组件，支持适合屏幕与 100% 预览、照片前后导航、同步/独立比较；明确 100% 针对缓存预览，不伪称原图逐像素检查。
4. GUI 控制：序列带可收起，Agent 占位可切换；快捷键遇输入框、对话框、拖动把手时正确隔离，保留焦点与键盘拖动。
5. 测试：npm typecheck/lint/Vitest/Playwright；Rust 事务、批量回滚、无操作、评分、旧库迁移；真实 Rust/SQLite 200→25、重开数据库与源 SHA256；桌面真实退出/重启和双尺寸截图。原生验收使用独立 review app identifier，避免本轮自动升级用户现有应用数据库。
6. 交付：Tauri build、真实截图、更新测试报告/已知问题/ADR/文件清单，停止于 M0。

记忆补充见 MEMORY-CONTEXT-ADDENDUM.md。当前只记录其约束；ContextBuilder/预算/检查点在 M2，Evidence/Hypothesis/FTS5 与纠正/遗忘在 M4。没有云请求或 Agent Runtime。
