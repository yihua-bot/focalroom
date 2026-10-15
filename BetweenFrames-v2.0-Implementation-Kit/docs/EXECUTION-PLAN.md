# Between Frames v2.0 · 开发 Agent 执行计划

此文件是编程任务执行契约；主权威文档为 `FINAL-PRODUCT-ENGINEERING-SPEC.md`。当前工程包**仅包含实施规范、接口样例、数据库原型迁移、验收配置和旧 GUI 参考，不包含一个完成可运行的生产应用**。

## 执行首轮：先审查当前代码

1. 若 Repo 已存在，展示目录、主要功能、测试结果，标记 `implemented / partial / missing`；不得删除已有正常功能。
2. 读取 `docs/`、`contracts/`、`db/`、`tests/`，创建或更新 `docs/adr/0001-architecture.md`。
3. 选择真实生产依赖的当前稳定版本，固定 lockfile；验证 Tauri 2、Node、Rust 构建。把版本锁定在 Repo 的机器可读依赖文件中。
4. 只实施 M0；完成并提交证据后才可处理下一里程碑。

## M0：合同优先、可运行底座

- 建 monorepo、`apps/desktop`、`apps/web`、`apps/api`、`workers/agent`、`packages/contracts`、`crates/local-domain`；只有桌面和 Web 初期需要完整 build，后端可为模块 skeleton。
- 做一致的设计 Tokens、可访问组件、三栏 Workspace 壳、虚拟 Grid / Inspect 面板占位。
- 生成 TypeScript / Rust DTO 或明确编译时的手动映射，定义 id 命名和错误码。
- 用假数据完成 UI 状态与业务状态分离；新增 Domain Command dispatcher stub，绝不从 React 组件直接写本地 SQLite。
- 实施数据库 `db/local/0001_local.sql`，在事务里建立测试 project / version / photo；运行 `tests/verify_contracts.py`。
- 交付：启动说明、桌面截图、测试报告、已知风险、架构 ADR、下个阶段拆单。

## M1：真实摄影 GUI（无 Agent 完整运行）

建议依序：真实文件选择/路径范围授权 → 读取 EXIF 和生成缩略图 → 网格虚拟化 → 筛选状态与快捷键 → Loupe/Compare → 作品序列排序 → Undo/Redo 与版本 → 本地导出 → 断网复验。

测试必须验证：500 张 PNG/JPEG 混合集的导入、滚动、离线重启恢复；同一动作 Undo/Redo 不产生序列位置重复；原始图片 hash 不变。失败场景：被移走文件、损坏图、编码异常、超大图、重复导入。

## M2：真正 Agent Loop（Mock）

- 严格实现 `AgentTask` lifecycle、`Tool Registry`、`Policy Gate`、`BudgetTracker`、`ModelProvider`、`CheckpointStore`。
- 首个工作工具：`get_project_context`, `list_photos`, `search_photos`, `get_sequence`, `create_edit_proposal`, `finish_task`；第二批可加入 inspect/compare。
- 用 MockProvider 构造**分支轨迹**：候选不足追加查找；越权 ID 拒绝；409 停止、rebase 询问；Agent 停止后无写入。
- 分离草案和正式版本；完整观察日志需引用 task_id、tool_call_id、真实照片 ID。

## M3：模型路由、图像理解、审美记忆

- 先部署每一任务的评测数据集（>=30 项），记录模型版本、Prompt 版本、合法授权照片范围、成本。
- `ModelRouter` 必须对 `MODEL_CAPABILITY_MISSING`, `PRIVACY_REGION_REJECTED`, `BUDGET_EXCEEDED` 给出可读错误。
- 分层索引照片，先检索候选再深度视觉观察，不把 300 张原图全部送入强视觉模型。
- 实现 Memory evidence, hypothesis, correction, scope; 被用户纠正的偏好不得在后续任务重新出现为确认事实。
- 与 Photoreka 等竞品真实用户同题任务对比（若无试用权限如实注明），禁止口头声称“更聪明”。

## M4：云协作基础

- 云认证、Workspace、ProjectGrant、Invite，Postgres 权威命令、R2 等对象存储私有预览、签名读取、项目迁移向导。
- 不允许将 LOCAL 直接改成 SHARED 而继续双端各自可写；明确 promotion 完成/失败/回滚方案。
- 收件人从 Web 登录即可发表评论（需项目角色授权），无需下载 Mac 应用。
- 对上传照片的云模型另行授权，创建共享项目不默认等于同意所有模型供应商上传。

## M5：协作版本与审稿

- 评论坐标、线程、编辑提案、Diff、审批、权限撤销、实时 presence。
- 两名编辑并发提案；作者审批其中之一后，另一位按 base_revision 冲突重审，不丢失数据。
- Websocket 断线重连以权威 snapshot/revision 修复状态。
- AI 获取项目评论必须通过项目 ACL，来源引用保留 reviewer 身份。

## M6：付费团队和学校

- 团队组织与席位、教育角色、工作区额度、计费事件与权益；先用 sandbox 支付。
- 建立操作审计界面、组权限、学生私人项目隔离、服务者只访问授权项目。

## M7：私密专业服务订单

- 编写服务范围与成果验收结构，分离平台协作审批与资金完成状态。
- 支付 provider 必须服务业务主体所在地区，webhook 验签和幂等；不自行持有应向合规支付机构管理的托管款。
- 先 5–10 单真实手工协作试点，再评估是否值得建设开放服务目录。

## 每一个功能任务的输出模板

- 背景 / 对应 spec 章节 / PR 涉及文件
- 用户故事和异常路径
- schema/api/migration 变化
- 新增或修改的测试；测试实测输出
- 真实操作截图或录屏
- 安全/权限/隐私检查
- 失败与回滚方式
- 本轮 DONE/NOT DONE 清单

**若缺少任何一项，只能标记 partial，禁止自动进入下一个里程碑。**
