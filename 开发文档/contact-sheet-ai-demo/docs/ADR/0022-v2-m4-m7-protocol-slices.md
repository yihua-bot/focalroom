# ADR 0022：v2.0 M4 Promotion 与 M7 支付协议切片

日期：2026-10-15。

S01（Promotion 单权威，migration 0014）：
- v2_promotions 状态机 pending→uploading→verifying→completed（completed 必须带云侧确认回执）；任何未终止态可 failed/aborted。
- 不变量：同项目并发 promotion 唯一拒绝；迁移中 authority=local-promoting（本地写入暂停由业务层门槛执行）；failed/aborted 回落 local 后可重新发起；completed 为终止态。
- 协议层本地可验证；真实上传/云回执在 M4 云服务部署后接入同一状态机。

B01/B02（支付事件账本，migration 0015）：
- v2_billing_events（event_id UNIQUE）+ v2_orders（open/paid/refunded/canceled）。
- B01 幂等：同 event_id 重放返回首次回执（replayed=true, settled=false），零重复行。
- B02 防伪造：事件必须携带服务端 HMAC-SHA256 签名（密钥经环境/部署配置注入，客户端无写入口调用路径）；签名不符或载荷篡改（改金额后原签名失效）即拒且零落行。
- 结算状态机：open→paid（金额须匹配订单）、paid→refunded；金额不符的事件记账但不结算（如实返回 settled=false）。
- hmac crate 为新依赖（RustCrypto 系列，与既有 sha2 同族）。

真实支付服务商 webhook 端点与云部署属外部条件；本切片让 B01/B02 的安全语义现在即可被自动化验证。
