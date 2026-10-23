-- Focal AI 异步任务：长耗时视觉请求绕过 Cloudflare 100 秒超时。
-- 复用 fr_ai_requests 表（request_id 主键），加结果存储列。
ALTER TABLE fr_ai_requests ADD COLUMN IF NOT EXISTS result_json TEXT;
ALTER TABLE fr_ai_requests ADD COLUMN IF NOT EXISTS error_code TEXT;
CREATE INDEX IF NOT EXISTS idx_ai_requests_poll
    ON fr_ai_requests(user_id, request_id) WHERE state IN ('PENDING','RUNNING');
