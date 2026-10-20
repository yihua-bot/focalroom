# FocalRoom v3.0 · Coding Agent 接手入口

**执行次序：** 读主规格 → 当前仓库核实 → 冻结基线 → 小步迁移 → 单 Gate 实施 → 自动验证 → 真实环境验收 → 变更记录。不要同时把 G0–G9 代码一次性重写。

## 项目仓库

- GitHub：`https://github.com/yihua-bot/focalroom`
- 历史工程（当前观察）：`开发文档/contact-sheet-ai-demo/between-frames/`
- 本包的代码/API/DDL 示例是**目标契约**，不直接覆盖当前迁移。真正开发前必须 `git status`、`git rev-parse HEAD`、查看 migration registry 和最近 CI 状态。
- 保留当前本地项目、用户照片及现有数据库，禁止尝试打开、升级、清理用户真实资料目录；默认使用全新隔离 fixture 和数据库副本。

## 第一轮仅执行 G0（不能跳过）

1. `git status --short`；确认当前 HEAD、未提交改动、分支和正在运行的服务。保留其他人的未提交修改。
2. 查当前 `README.md`、`AGENTS.md`、`docs/DEVELOPER-HANDOFF-2026-10-19.md`、`V2-COMPLIANCE.md`、最近 `github/actions`，执行当前提供的回归命令（按实际仓库说明，不推测可执行入口）。
3. 修复既有 CI：Web E2E 标题断言与 SharedApp 当前标题不匹配；测试时未运行 API 的代理错误；所有失败均做根因分类，不能单纯删测试让它变绿。
4. 生成 `current-state-v3.json`：模块、实测通过/失败、mock/生产路径、现存数据库表、旧项目兼容性、业务权限、色彩与导出限制。
5. 生成 `migrations-plan-v3.md` 与旧库只读快照哈希；保持旧 migration 不变。
6. 由技术负责人评审 G1/G2 的 API、数据及色彩设计后，开始第一批实现。

## 开发硬规则

- 未经确认不能操作生产部署、第三方真实账户、购买服务、触发带费用的 API 或发送真实摄影作品；安全的隔离 mock / 本地生成样例可自行测试。
- 开发过程中始终保护 SourceAsset byte hash；任何导入/缓存/渲染/回流失败不允许损坏源文件。
- 每个正式写命令必须满足 schema、身份/ACL、project scope、幂等、版本/CRDT 冲突策略和审计；Agent 只创建 Draft。
- 新 API 要有类型、契约测试、错误码和可追溯 ID；原生与 Web 必须使用同一语义协议。
- `MockProvider` 仅用于接口测试；真实摄影质量须使用明确获得许可的真实素材及独立评分，不允许编造“质量达到 Photoreka”。
- 色彩质量必须有参考 ICC 文件和相对/绝对变换验证；不能仅以“肉眼看起来好”验收；Web 上不冒充专业色彩级别。
- Lightroom/Capture One 不允许直接改私有目录数据库；外部同步依赖官方可用 SDK、受控文件导出、manifest 和显式校验。
- 发布物输出走高分辨率母版和 PrintContract，不可使用 AI 压缩图生成正式印刷稿。
- 每个 Gate 交付最少：实际可运行代码、数据库迁移与回滚说明、单测/集成/端到端测试、真实或说明不足的验收结果、截图/录屏、已知风险、提交 ID。

## 推荐首三个 PR 序列

- **PR-01 G0 CI Baseline**：修复 E2E 偏差、建立测试环境启动、状态盘点与基线跑通。
- **PR-02 G1 Identity Model**：新增 SourceAsset、EditVariant、Rendition、ExternalEditLink schema + alias 映射与兼容读取，只针对隔离测试库；绝不立即切换全部生产写流。
- **PR-03 G2 Color Proof of Concept**：原生 ColorSync/ImageIO 的 ICC/16-bit TIFF/100MP 局部分块 Viewer 验证程序，并同 WebView 方案定量比较后确定实现方式。

## 交付结果模板

```
Gate: G1 · Asset Domain
Commit: <sha>
Changes: <list>
Data Migration: old schema → new schema; backup restore result; alias invariants
Domain Tests: pass/fail + logs
UI E2E: pass/fail + screenshot
Manual/External Acceptance: covered/not covered (explicit reasons)
Safety: source sha256 equality & zero unintended upload
Rollback: safe steps
Remaining: <specific gaps>
```

**请先实现符合约束的最小可检验切片，不要为了尽快宣布“v3 全完成”而模拟关键专业流程。**
