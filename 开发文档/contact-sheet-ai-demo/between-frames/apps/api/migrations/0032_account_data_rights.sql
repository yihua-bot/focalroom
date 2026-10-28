-- 关闭标记阻止旧登录方式/会话恢复；保留最小匿名主体供共享作品和计量审计引用。
CREATE TABLE fr_account_closures (
  user_id TEXT PRIMARY KEY REFERENCES fr_users(id),
  receipt_id UUID NOT NULL UNIQUE,
  closed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- 删除对象存储失败时持久保留重试项，不将“账号关闭”混同为介质清理完成。
CREATE TABLE fr_erasure_objects (
  object_key TEXT PRIMARY KEY CHECK(object_key ~ '^[a-f0-9]{64}\.jpg$'),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
DO $$ BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname='focalroom_app') THEN
    GRANT SELECT,INSERT ON fr_account_closures TO focalroom_app;
    GRANT SELECT,INSERT,DELETE ON fr_erasure_objects TO focalroom_app;
    GRANT UPDATE(email) ON fr_users TO focalroom_app;
  END IF;
END $$;
