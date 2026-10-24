-- 仅显式允许短期加密暂存的请求进入队列；供应商调用开始后绝不再次领取。
CREATE TABLE fr_ai_jobs (
  user_id TEXT NOT NULL,
  request_id TEXT NOT NULL,
  session_digest TEXT NOT NULL CHECK (session_digest ~ '^[a-f0-9]{64}$'),
  key_id TEXT NOT NULL CHECK (length(key_id) BETWEEN 1 AND 64),
  input_hash TEXT NOT NULL CHECK (input_hash ~ '^[a-f0-9]{64}$'),
  images INTEGER NOT NULL CHECK (images BETWEEN 0 AND 20),
  phase TEXT NOT NULL DEFAULT 'QUEUED' CHECK (phase IN ('QUEUED','DISPATCHING','DONE','UNKNOWN')),
  attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 1),
  payload BYTEA,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ NOT NULL,
  dispatched_at TIMESTAMPTZ,
  dispatch_id TEXT,
  PRIMARY KEY (user_id, request_id),
  FOREIGN KEY (user_id, request_id) REFERENCES fr_ai_requests(user_id, request_id),
  CHECK (expires_at > created_at AND expires_at <= created_at + interval '10 minutes'),
  CHECK ((phase='QUEUED' AND payload IS NOT NULL AND octet_length(payload) BETWEEN 29 AND 16777245 AND attempts=0 AND dispatch_id IS NULL)
    OR (phase<>'QUEUED' AND payload IS NULL)),
  CHECK (phase<>'DISPATCHING' OR (attempts=1 AND dispatched_at IS NOT NULL AND dispatch_id IS NOT NULL))
);
CREATE INDEX fr_ai_jobs_queue ON fr_ai_jobs(created_at,user_id,request_id) WHERE phase='QUEUED';
CREATE INDEX fr_ai_jobs_inflight ON fr_ai_jobs(dispatched_at) WHERE phase='DISPATCHING';
DO $$ BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname='focalroom_app') THEN
    GRANT SELECT,INSERT,UPDATE ON fr_ai_jobs TO focalroom_app;
    REVOKE DELETE,TRUNCATE ON fr_ai_jobs FROM focalroom_app;
  END IF;
END $$;
