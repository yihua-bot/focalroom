-- M4 云 Agent Worker：评论汇总任务与模型调用记录。
-- 任务只读评论并调用授权模型做汇总分析；不写正式状态、不上传图片、不访问作者记忆。
CREATE TABLE IF NOT EXISTS fr_agent_task_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES fr_projects(id) ON DELETE CASCADE,
    task_type TEXT NOT NULL DEFAULT 'summarize_comments',
    created_by UUID NOT NULL REFERENCES fr_users(id),
    model_id TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    comment_count INTEGER NOT NULL DEFAULT 0,
    tokens_in INTEGER NOT NULL DEFAULT 0,
    tokens_out INTEGER NOT NULL DEFAULT 0,
    cost_estimate_minor_usd INTEGER NOT NULL DEFAULT 0,
    provenance JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_agent_task_results_project
    ON fr_agent_task_results(project_id, created_at DESC);
