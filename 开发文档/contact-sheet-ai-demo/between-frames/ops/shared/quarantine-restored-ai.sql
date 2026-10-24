-- 仅用于已验证、保持停写的独立备份恢复目标。
-- 快照里的 QUEUED 不证明上游从未收到：源库可能在备份后已经发送。
-- 因此恢复后统一隔离未结束的请求，不重新执行、不释放预留、不修改已结算计量。
DO $$ BEGIN
  IF to_regclass('public.fr_ai_jobs') IS NOT NULL THEN
    UPDATE fr_ai_requests SET state='UNKNOWN',error_code='AI_RESULT_UNKNOWN',finished_at=now()
      WHERE state='PENDING' AND charged IS NULL;
    UPDATE fr_ai_jobs SET phase='UNKNOWN',payload=NULL
      WHERE phase IN ('QUEUED','DISPATCHING');
  END IF;
END $$;
