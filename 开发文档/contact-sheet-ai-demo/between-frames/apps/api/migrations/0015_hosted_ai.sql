-- 额度单位为供应商报告的 input_tokens + output_tokens，不代表货币或摄影质量。
CREATE TABLE fr_ai_accounts (
  user_id TEXT PRIMARY KEY REFERENCES fr_users(id),
  granted BIGINT NOT NULL DEFAULT 0 CHECK (granted >= 0 AND granted <= 1000000000),
  spent BIGINT NOT NULL DEFAULT 0 CHECK (spent >= 0),
  reserved BIGINT NOT NULL DEFAULT 0 CHECK (reserved >= 0),
  CHECK (spent + reserved <= granted)
);
CREATE TABLE fr_ai_grants (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES fr_ai_accounts(user_id),
  tokens INTEGER NOT NULL CHECK (tokens > 0),
  reason TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE fr_ai_requests (
  user_id TEXT NOT NULL REFERENCES fr_ai_accounts(user_id),
  request_id TEXT NOT NULL,
  body_hash TEXT NOT NULL,
  policy TEXT NOT NULL,
  reserved INTEGER NOT NULL CHECK (reserved > 0),
  charged INTEGER CHECK (charged >= 0 AND charged <= reserved),
  state TEXT NOT NULL CHECK (state IN ('PENDING','COMPLETED','REJECTED','UNKNOWN')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  finished_at TIMESTAMPTZ,
  PRIMARY KEY(user_id,request_id)
);
CREATE INDEX fr_ai_pending ON fr_ai_requests(user_id,state);
CREATE TABLE fr_ai_reconciliations (
  user_id TEXT NOT NULL,
  request_id TEXT NOT NULL,
  charged INTEGER NOT NULL CHECK(charged >= 0),
  billing_reference TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY(user_id,request_id),
  FOREIGN KEY(user_id,request_id) REFERENCES fr_ai_requests(user_id,request_id)
);
DO $$ BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname='focalroom_app') THEN
    GRANT SELECT,INSERT,UPDATE ON fr_ai_accounts,fr_ai_requests TO focalroom_app;
    GRANT SELECT,INSERT ON fr_ai_grants TO focalroom_app;
    GRANT SELECT,INSERT ON fr_ai_reconciliations TO focalroom_app;
  END IF;
END $$;
