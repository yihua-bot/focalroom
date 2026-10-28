-- 普通命令/事件/版本仍不可修改；仅允许已关闭账号、已终止私有项目的受审计正文擦除。
CREATE TABLE fr_erasure_audit (
  receipt_id UUID NOT NULL REFERENCES fr_account_closures(receipt_id),
  project_id TEXT NOT NULL REFERENCES fr_projects(id),
  detail JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY(receipt_id,project_id)
);
CREATE TRIGGER immutable_erasure_audit BEFORE UPDATE OR DELETE ON fr_erasure_audit
  FOR EACH ROW EXECUTE FUNCTION fr_immutable();

CREATE FUNCTION fr_guard_private_erasure() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
  field TEXT := TG_ARGV[0];
  replacement JSONB;
BEGIN
  IF TG_OP <> 'UPDATE' THEN RAISE EXCEPTION 'immutable audit record'; END IF;
  replacement := CASE WHEN field='snapshot' THEN '{"photos":[],"sequences":[]}'::jsonb ELSE '{"erased":true}'::jsonb END;
  IF (to_jsonb(NEW) - field) = (to_jsonb(OLD) - field)
     AND to_jsonb(NEW)->field = replacement
     AND EXISTS (
       SELECT 1 FROM fr_projects p JOIN fr_account_closures c ON c.user_id=p.owner_id
       JOIN fr_erasure_audit a ON a.project_id=p.id AND a.receipt_id=c.receipt_id
       WHERE p.id=NEW.project_id AND p.state='ABORTED'
     ) THEN RETURN NEW;
  END IF;
  RAISE EXCEPTION 'immutable audit record';
END $$;

DROP TRIGGER immutable_commands ON fr_commands;
DROP TRIGGER immutable_versions ON fr_versions;
DROP TRIGGER immutable_events ON fr_events;
CREATE TRIGGER immutable_commands BEFORE UPDATE OR DELETE ON fr_commands
  FOR EACH ROW EXECUTE FUNCTION fr_guard_private_erasure('envelope');
CREATE TRIGGER immutable_versions BEFORE UPDATE OR DELETE ON fr_versions
  FOR EACH ROW EXECUTE FUNCTION fr_guard_private_erasure('snapshot');
CREATE TRIGGER immutable_events BEFORE UPDATE OR DELETE ON fr_events
  FOR EACH ROW EXECUTE FUNCTION fr_guard_private_erasure('detail');
DO $$ BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname='focalroom_app') THEN
    GRANT SELECT,INSERT ON fr_erasure_audit TO focalroom_app;
  END IF;
END $$;
