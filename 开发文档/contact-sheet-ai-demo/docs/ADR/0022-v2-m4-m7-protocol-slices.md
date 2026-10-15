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

S02（签名媒体链接，migration 0016）：
- v2_media_grants：token = HMAC(项目|照片|过期时间)，签发时照片归属校验（跨项目拒绝）；
- 校验三重门：签名匹配 + 未过期 + 未撤销；撤销拒绝与过期拒绝文案明确区分（审计可分辨）；
- revoke_project 支持项目级批量撤销（降级/移除成员场景），幂等；
- 真实 CDN/边缘网关在 M4 部署后复用同一校验逻辑。至此 24 验收场景中 22 项 pass（协议层证据），仅 C03 实时收敛与 E01 云侧隔离因需要真实网络两端保持 partial。
