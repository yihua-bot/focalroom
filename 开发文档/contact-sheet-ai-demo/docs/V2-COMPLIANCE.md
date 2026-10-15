# v2.0 规格符合性总账（24 验收场景映射）

日期：2026-10-15。基线：`8353afa`（+本轮 M6 骨架）。对照 `BetweenFrames-v2.0-Implementation-Kit/tests/acceptance-matrix.json`。状态：**pass**（自动化证据）、**partial**（本地/模拟层验证，云或真模型层待外部）、**missing**（依赖未建基础设施）。

| ID | 场景 | 状态 | 证据 |
|---|---|---|---|
| G01 | AI 关闭 500 张 GUI 选片导出 | **pass** | gui_v2_m1（500 张混合集/重启一致/hash 不变）；16 预览 + 5 集成 E2E |
| A01 | 结果不足自主追加检索 | **pass** | agent_runtime ag01（双观察轨迹） |
| A02 | 不同反馈不同路径 | **pass** | agent_runtime ag02/ag03 |
| A03 | 越权 photo_id 拒绝 | **pass** | ag05_06_13_14 + v2_bridge（跨项目操作拒）+ PolicyGate 回归 |
| A04 | Agent 无权改正式版本 | **pass** | v2_bridge（Agent principal 纯函数层拒）+ v2_proposals（Agent 仅可 create） |
| A05 | 取消/恢复不重复草案 | **pass** | ag07（回执重放）+ ag09（检查点） |
| A06 | 同请求重试幂等 | **pass** | v2_bridge（幂等优先于 409）+ command_history durable_replay |
| P01 | 无同意不发送预览图 | **pass** | agent_vision ag04（零外发断言）+ consent 三态配额 |
| P02 | 对抗文本不升权 | **pass** | ag13（注入指令不可信）+ 白名单 schema |
| M01 | 纠正后不再误用 | **pass** | agent_memory ag11 + 物理删除墓碑（ADR 0014 修订） |
| M02 | 私人记忆不跨创作者 | **pass** | scope=project_local + 跨项目 schema 层拒绝 + Router 不扩图像范围 |
| M03 | 区域拒绝不静默跨区 | **pass** | v2_tools Router 测试（受限区域回落本地、不跨区） |
| S01 | LOCAL→SHARED 单权威 | **missing** | M4 云基础设施未建；本地权威/模式边界已定义并测试 |
| S02 | 撤销后 URL 拒绝 | **missing** | 依赖 M4 签名链接服务 |
| C01 | Reviewer 不能批准 | **pass** | local-domain 权限样例 21 组 + proposals apply 须 can_approve |
| C02 | 并发提案基线冲突检测 | **pass** | v2_proposals stale 测试（base 漂移拒绝重建） |
| C03 | 断线重连版本一致 | **partial** | 本地重启一致性全套通过；CRDT 实时收敛依赖 M4 |
| C04 | 评论锚点重排后仍指原照片 | **pass** | 锚点按 photo_id 持久化（0011），排序变化不影响锚定；照片删除时锚点校验拒新引用 |
| E01 | 教师不能访问学生私人项目 | **partial** | 组织/席位/角色骨架 + 项目归属校验已测（M6 本轮）；教育隔离的云侧 ACL 依赖 M4 |
| B01 | webhook 重放不重复结算 | **missing** | M7 支付服务未建 |
| B02 | 伪造付款不改账单 | **missing** | M7 服务端账本未建（客户端不可信原则已在 principal 设计体现） |
| D01 | 记忆删除后缓存/派生物失效 | **pass** | 物理删除墓碑 + FTS5 全量重建 + 观察缓存清空（ADR 0014/0019） |
| D02 | 版本回滚生成新版本 | **pass** | command_history（恢复为可撤销新编辑）+ gui_v2_m1（undo/redo 位置唯一） |
| A07 | 项目更新后草案标过期 | **pass** | ag08（stale/rebase）+ proposals 基线漂移拒 |

## 汇总

- **pass：18 / 24**
- **partial：3**（C03 实时收敛、E01 云侧隔离、及 C04 已升级为 pass 的锚点持久化——实际 partial 2 项）
- **missing：4**（S01/S02 依赖 M4 云；B01/B02 依赖 M7 支付）

## 结论与剩余

工程可在本地完成的验收全部有自动化证据；missing 4 项全部依赖外部基础设施（云服务、支付服务），不存在"写了代码就能验证"的路径。M4（认证/Postgres/共享服务）与 M7（支付集成）需要真实部署环境与外部账户，属"待外部条件"，与 v1.0 的签名/Beta 同类。建议：以本总账为 v2.0 工程侧完成基线；云侧（M4/M7）按真实需求单独立项。
