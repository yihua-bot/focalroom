# BETWEEN FRAMES v2.0 — Agent + Workspace + Professional Services

**这是规划与契约工程包，不是已完成的软件，也不是可一键部署的产品。**

## 从这里开始
1. `docs/FINAL-PRODUCT-ENGINEERING-SPEC.md`：完整产品、技术、交互、权限、商业、交付和评测要求。
2. `AGENT-START-HERE.md`：交给编码 Agent 的第一条执行指令。
3. `docs/EXECUTION-PLAN.md`：M0–M7 可验收里程碑任务。
4. `contracts/`：受控业务命令/Agent 工具/审批的 JSON Schema 草案。
5. `db/`：LOCAL 与 SHARED 两种权威数据域的数据库迁移起点。
6. `config/model-router.yaml`：自动模型路由、质量/费用/授权规则模板。
7. `tests/acceptance-matrix.json`、`tests/verify_contracts.py`：验收矩阵及静态检查。
8. `reference/gui-demo.html`：旧版 GUI Demo（AI 是模拟，不作为生产验收依据）。

## 核心原则
- 有竞争力的摄影 Agent 是产品核心，不是一个装饰性按钮。
- GUI 必须专业且独立可操作，Agent 是同一工作区内的强能力。
- 不做关注、粉丝、动态流、点赞等社交网络；关系来自实际协作。
- 个人记忆默认私有；用户授权共享项目内容与模型处理图像。
- Agent 只能产生草案，由有权限的人审批正式版本。
- 为支持真正的跨设备协作，SHARED 项目以云端服务为唯一权威，LOCAL 项目保持本地权威。

## 基线限制
数据库/Schema 为**初始设计样例**，并未替代生产迁移审计、鉴权、数据加密、支付合规及完整性能测试。执行前必须通过 ADR 明确各模块的稳定版本与合同升级策略。
