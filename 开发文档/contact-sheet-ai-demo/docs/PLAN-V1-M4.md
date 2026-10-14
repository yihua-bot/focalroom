# v1.0 M4 实施计划

范围：L2 人工偏好证据、L3 可纠正假设、Memory Studio 与检索注入。完成后停止评审；Keychain/签名/性能属 M5。依据 FINAL-IMPLEMENTATION-SPEC §8/§13-M4 与 MEMORY-CONTEXT-ADDENDUM。

1. migration 0006：preference_evidence（append-only、dedup_key=command_id 因果去重）与 preference_hypotheses（scope/status/confidence/纠正/停用）。
2. 证据只从生效的人工 GUI 命令记录（keep/reject/比较/备注/草案采纳与丢弃）；撤销/重做/幂等重放不增计数；Agent 草案通道不写证据。
3. 假设为确定性规则物化（证据文本 2-gram 聚合，≥2 条支持），默认 project_local；纠正后 disabled 且同表述不复活（AG-11）；FTS5 留待假设量增长后（v1 以过滤+排序满足检索正确性）。
4. retrieve_preferences 升级为证据支持检索（排序：确认>未确认、证据数、时间）；跨项目 scope 在 schema 白名单层拒绝；ContextBuilder 限量注入假设摘要。
5. Memory Studio GUI：证据时间线、假设卡（证据链接/反例/纠正/停用恢复）、「Agent 选择不算证据」明示。
6. 测试：去重/确定性物化、AG-11 不复活、Agent 草案零证据、工具级检索与白名单拒绝、撤销不影响证据；全量回归与文档。

KISS/YAGNI：不引入向量库、NLP 分词或独立记忆服务；关键词统计为可解释的确定性规则。
