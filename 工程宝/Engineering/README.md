# FocalRoom v3.0｜最终产品与工程实施包

版本：2026-10-20；性质：目标方案 + 约束与验收契约，不代表当前仓库已实现全部功能。

## 建议阅读顺序

1. `FOCALROOM-V3-FINAL-MASTER-PLAN.md`：完整战略、产品、UI/UX、技术架构、专业色彩、Agent、画布、往返集成、排版/印刷和交付路线。
2. `delivery/AGENT-START-HERE.md`：编码 Agent 必须先读的安全和开发指南。
3. `architecture/ADR-AND-GAP-MAP.md`：核心架构决策与现有仓库差距。
4. `ux/SCREEN-AND-INTERACTION-CONTRACT.md`：逐页面状态、手势、缩放、涂鸦、评论、工具与无障碍要求。
5. `contracts/DOMAIN-SCHEMA.sql`：PostgreSQL 目标领域示例，不可直接用于线上迁移。
6. `contracts/api-openapi.yaml`：OpenAPI 3.1 目标端点示例，非完整实现。
7. `contracts/canvas-command.schema.json`：Canvas 正式命令 schema，可机器验证。
8. `contracts/external-rendition-manifest.schema.json`：外部调色回流文件清单 schema。
9. `contracts/agent-tools.schema.json`：Agent 基本工具封装 schema，具体工具参数要按产品能力逐个扩展。
10. `quality/ACCEPTANCE-MATRIX.md`、`quality/COLOR-PRINT-VALIDATION.md`：自动化、实体设备、真人、供应商和生产验收规范。
11. `delivery/IMPLEMENTATION-BACKLOG.csv`：53 个有验收目标的实施任务，不是工期承诺。

## 现有代码基线

仓库为 `https://github.com/yihua-bot/focalroom`，2026-10-19 审查观察到本地 GUI/序列编辑/Agent Runtime/云端共享评审基础；真正的自由叙事 Canvas、高精度 ICC 统一管线、Lightroom/Capture One 双向连接、摄影书印前引擎仍属 v3 目标；最新 CI 失败需要 G0 修复。实际开发前再核对 Git HEAD 与未提交改动。

## 使用与安全

此包没有生产密钥或私密照片。不要将 `DOMAIN-SCHEMA.sql` 直接执行到用户/生产数据库。不要在未授权的情况下访问真实 Lightroom/Capture One Catalog、外发摄影作品、修改真实对象存储或开启收费模型。任何实测和预期指标需要明确区分。

推荐将整个工程包放进仓库新目录（例如 `docs/v3-north-star/`），在单独 PR 经过架构评审后合并。不要直接覆盖原开发文档，也不应删除现有产品代码。
