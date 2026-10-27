-- 退款与争议额度补偿：退款/拒付后撤回对应发票已授予的 token 配额。
-- id 采用支付方的退款/争议对象 ID（re_*/dp_*），同一退款以不同事件 ID 重投时保持幂等。
-- tokens_due 为按退款金额比例应撤回数；tokens_revoked 为账户可用额度内实际撤回数，
-- 两者之差即已消费导致的补偿缺口，留待人工对账。

CREATE TABLE IF NOT EXISTS fr_billing_refunds (
    id TEXT PRIMARY KEY,
    invoice_id TEXT NOT NULL REFERENCES fr_billing_invoices(id),
    user_id TEXT NOT NULL REFERENCES fr_users(id),
    kind TEXT NOT NULL CHECK(kind IN ('refund','dispute')),
    amount BIGINT NOT NULL CHECK(amount > 0),
    tokens_due INTEGER NOT NULL CHECK(tokens_due >= 0),
    tokens_revoked INTEGER NOT NULL CHECK(tokens_revoked >= 0),
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS fr_billing_refunds_invoice
    ON fr_billing_refunds(invoice_id);

DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT SELECT, INSERT ON fr_billing_refunds TO focalroom_app;
  END IF;
END
$$;
