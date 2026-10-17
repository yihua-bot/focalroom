# ADR 0040：PostgreSQL 共享权威与 M5 协作闭环

日期：2026-10-17。范围：按用户“把能推进的全部推进完成，一直推进到 M5”实施可本地验收工程；真人/生产外部门槛继续独立记录。该明确指令允许在 M3 真人质量门槛未关闭时推进 M4/M5 工程，不将 M3 自动记作通过。

## 决策

- 正式共享服务为 `apps/api`：NestJS transport、独立 PostgreSQL、共享 v2 Zod 契约、Auth/Authority/PreviewStore/Realtime 分责；旧 `cloud/` SQLite 只保留实验兼容。不能把 SQLite 连接串换成 PostgreSQL，也不能让旧实验入口颁发假云权威。
- Node 25 原生 TypeScript；Nest 标准 Controller/DI/route 元数据显式应用，无额外运行时编译器。正式写入经过 AuthorityService，前端、Yjs 和模型输出均没有写库旁路。
- 普通 Membership 不隐式开放所有项目；Owner/Admin 管理工作区项目，其他人须 ProjectGrant。邀请 7 天、摘要存储、邮箱绑定、一次性接受，可撤销，当前手动分享链接。Editor 正式修改/审批分别授权，Reviewer 只评论与提案，Viewer 只读。
- 密码 scrypt-v1，24h session 摘要；Cookie HttpOnly/SameSite Strict、写请求 Origin + CSRF，桌面进程内 Bearer。短期 WS/媒体票据与 5 分钟只读评论 Task 身份均保持当前 session/ACL 约束。
- 项目行锁、权限/版本检查、真实操作执行、正式版本、事件、幂等回执同一 PostgreSQL 事务。相同键的不同载荷拒绝；重放也重查当前角色。独立提交提案不推进正式版本；批准选中操作才更新作品，空选驳回不推进，过期提案保留原文。
- 草案仅作者可见，作者可按提案 revision 修订并保留原始请求审计；提交后通过逐项 Diff 审阅。不可变历史版本可单独只读查看，不自动恢复当前版本。Web 图片按 48 张分页获取，避免一次下载完整图库。
- 数据库启动只核验迁移，不自动 DDL。保留已提交 `0001` 的 SHA，追加 `0002`：新回执必须保存完整 command envelope，旧回执保留 NULL，不编造历史；审计不可变。迁移在同一受锁事务中执行，版本/SQL SHA 不符拒绝继续。
- 桌面真实 Promotion 先冻结本地，清单/预览完整后云端显式提交，本地记录确认再只读。服务/账户/工作区绑定持久化，无密码/token；网络不确定保持冻结并可重试，已提交不能取消。云端初始 revision 单独保存，后续云编辑不改变迁移回执。桌面当前保留迁移时只读快照，后续编辑在 Web。
- 原生续验修正了把桌面后端误当成 LOCAL 权威的侧栏/列表文案；未取权威状态的列表用本机缓存标签，避免新增逐卡查询。两个共享链接复用一个组件；macOS 通过只接受 projectId 的资源命令打开浏览器，Rust 从已完成迁移绑定重建 HTTPS/回环来源和合法云项目 UUID，复核完整URL，不接受前端任意URL或Shell文本。其他原生平台明确提示复制地址，不声称已实现浏览器启动。无新依赖、迁移或正式写入通道。
- 共享包导入是独立副本，明确不切换本地权威。仅缓存 JPEG 重编码；服务再校验 <=2048px/8MiB 并去 EXIF/XMP/GPS，不授予 AI 上传、作者记忆或原图权限。原源指纹与派生内容校验分开。私有本地存储/R2 可替换，通过短期一次性网关读取并重查撤权；不暴露可绕过 ACL 的裸桶 URL。
- WebSocket 首帧消费一次性票据，当前 session/ACL 持续复核，持久事件仅做刷新信号；重连重新拉权威 snapshot。Presence 30s 失活；Yjs 仅每用户每项目私人 note（4000 字/64KiB），同用户窗口合并。正式序列、审批、权限不使用 CRDT。
- Agent 评论 Task 身份只允许本项目原评论读取，保留作者和引用、到期/撤销/父会话失效即拒绝。不能执行人类命令、模型上传或形成作者偏好；云模型 Worker/队列仍未实现。

## 验证与限度

真实临时 PostgreSQL、实际 Rust harness/Nest HTTP、3 个独立浏览器会话、S3 本地协议服务器；覆盖审批竞态、丢失确认、进程重启、错误配置恢复拒绝、显式安全取消、预览完整性、撤权与任务隔离。生成素材不计专业摄影评测，不联系真实 AI。

本机 Docker daemon 不可用，未构建容器、未接真实 R2、未执行生产 DDL/部署。开发认证不代表邮箱验证、账户恢复、集中限流和生产运维已完成。M3 30 项/10 位摄影师与 M5 5 个真人双人项目仍未关闭。完整记录见 `../V2-M4-M5-SHARED-AUTHORITY-TEST-REPORT.md`。

## 并发工作区说明

本轮起始 HEAD 为 aa64110；工作期间发现其他进程推进到 24462e0，包含删除旧 Provider fallback helper 与新增 SQLite `/api/v2` 适配。该适配的密码/提交边界与本决策不兼容。保留外部提交历史，不回退或覆盖他人工作；运行入口必须安全封闭，正式 v2 权威仍按上述 PostgreSQL 实现验证。本聊天没有执行 git commit/push/reset。
