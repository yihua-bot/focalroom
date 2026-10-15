# ADR 0001 — LOCAL 与 SHARED 项目权威划分

状态：提议采纳（2026-10-15）

背景：v1 单机应用不能自动满足多人跨浏览器协作；若同步本地 SQLite 与云端 PG 并允许双主写入，会出现版本覆盖与授权不一致。

决策：LOCAL 的正式业务状态仅以本地 Rust+SQLite 为权威；SHARED 的正式业务状态仅以云端服务+Postgres 为权威。迁移 LOCAL→SHARED 显式操作并提供资源清单、完整性检查、版本确认和失败回滚；云共享项目的桌面端是缓存/只读或 proposal workspace，不是并行正式主库。

备选：全面云优先（损失离线和原图隐私）；双向全量离线同步（复杂且风险较大）；统一 Rust 服务端和客户端（平台开发成本更高）；以后可凭实际项目规模复盘。

约束：一切关键写入都经过共享 Schema 的命令语义和权限检查；正式序列不采用 LWW CRDT；WebSocket 只传播 committed event / ephemeral presence。

回滚：暂停云 promotion，保留 LOCAL 可用性；迁移前制作本地备份，不能静默合并已经出现的两套权威版本。
