-- 支付资料只保存服务端引用，不保存银行卡或支付凭据。
CREATE TABLE fr_billing_customers (
  user_id TEXT PRIMARY KEY REFERENCES fr_users(id),
  customer_id TEXT NOT NULL UNIQUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE fr_billing_checkouts (
  user_id TEXT NOT NULL REFERENCES fr_users(id),
  request_id TEXT NOT NULL,
  price_id TEXT NOT NULL,
  session_id TEXT UNIQUE,
  checkout_url TEXT,
  expires_at TIMESTAMPTZ,
  state TEXT NOT NULL DEFAULT 'PENDING' CHECK(state IN ('PENDING','READY','CLOSED')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY(user_id,request_id)
);
CREATE TABLE fr_billing_subscriptions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES fr_users(id),
  customer_id TEXT NOT NULL,
  price_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('active','trialing','past_due','canceled','incomplete','unpaid','paused','incomplete_expired')),
  current_period_end TIMESTAMPTZ NOT NULL,
  cancel_at_period_end BOOLEAN NOT NULL DEFAULT false,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX fr_billing_subscription_user ON fr_billing_subscriptions(user_id,current_period_end DESC);
CREATE TABLE fr_billing_invoices (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES fr_users(id),
  subscription_id TEXT NOT NULL,
  amount BIGINT NOT NULL CHECK(amount >= 0),
  currency TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('paid','open','void')),
  issued_at TIMESTAMPTZ NOT NULL,
  tokens_granted INTEGER NOT NULL DEFAULT 0 CHECK(tokens_granted >= 0)
);
CREATE TABLE fr_billing_events (
  id TEXT PRIMARY KEY,
  digest TEXT NOT NULL,
  event_type TEXT NOT NULL,
  received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
DO $$ BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname='focalroom_app') THEN
    GRANT SELECT,INSERT,UPDATE ON fr_billing_customers,fr_billing_checkouts,fr_billing_subscriptions,fr_billing_invoices TO focalroom_app;
    GRANT SELECT,INSERT ON fr_billing_events TO focalroom_app;
  END IF;
END $$;
