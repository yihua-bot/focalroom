
-- v2.0 M7 支付事件账本（B01/B02）：服务端权威、幂等回执、HMAC 签名校验。
CREATE TABLE IF NOT EXISTS v2_billing_events (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 event_id TEXT NOT NULL UNIQUE,
 order_id TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN('payment.succeeded','payment.refunded','subscription.updated')),
 amount_minor INTEGER NOT NULL CHECK(amount_minor >= 0),
 currency TEXT NOT NULL DEFAULT 'USD',
 signature TEXT NOT NULL,
 received_at TEXT NOT NULL
);
CREATE INDEX billing_order ON v2_billing_events(order_id, id);
CREATE TABLE IF NOT EXISTS v2_orders (
 id TEXT PRIMARY KEY,
 workspace_id TEXT NOT NULL REFERENCES v2_workspaces(id),
 plan TEXT NOT NULL CHECK(plan IN('standard','pro','edu')),
 status TEXT NOT NULL DEFAULT('open') CHECK(status IN('open','paid','refunded','canceled')),
 amount_minor INTEGER NOT NULL CHECK(amount_minor >= 0),
 currency TEXT NOT NULL DEFAULT 'USD',
 updated_at TEXT NOT NULL
);
