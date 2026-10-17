-- 保留旧回执原样，NULL 表示旧版本未保存命令原文，绝不伪造补写历史。
ALTER TABLE fr_commands ADD COLUMN envelope jsonb;
CREATE FUNCTION fr_require_envelope() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.envelope IS NULL THEN RAISE EXCEPTION 'new command requires envelope'; END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER require_command_envelope BEFORE INSERT ON fr_commands FOR EACH ROW EXECUTE FUNCTION fr_require_envelope();
CREATE TRIGGER immutable_acl_audit BEFORE UPDATE OR DELETE ON fr_acl_audit FOR EACH ROW EXECUTE FUNCTION fr_immutable();
