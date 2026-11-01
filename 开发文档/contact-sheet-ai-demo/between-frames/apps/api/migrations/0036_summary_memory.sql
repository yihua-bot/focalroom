-- 长期记忆三层：任务摘要 + 向量嵌入（应用层余弦相似度）
-- 参考 OpenYC RAG 设计：raw memory + summary memory + vector retrieval

-- 任务摘要：任务完成后自动生成，下次任务注入上下文
CREATE TABLE IF NOT EXISTS fr_task_summaries (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    task_id TEXT,
    summary TEXT NOT NULL,
    -- 摘要类型：selection（选片）/ narrative（叙事）/ color（调色）/ general
    kind TEXT NOT NULL DEFAULT 'general',
    -- 用户偏好提取（如"偏好暖色调""常用横构图"）
    preferences_extracted JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_task_summaries_user ON fr_task_summaries(user_id, project_id, created_at DESC);

-- 知识库向量嵌入（应用层余弦相似度——后续可升级 pgvector）
CREATE TABLE IF NOT EXISTS fr_knowledge_embeddings (
    id BIGSERIAL PRIMARY KEY,
    entity_kind TEXT NOT NULL, -- photographer/series/photobook/claim
    entity_id TEXT NOT NULL,
    content TEXT NOT NULL,     -- 被嵌入的文本快照
    embedding JSONB NOT NULL,  -- float 数组（应用层计算）
    model TEXT NOT NULL DEFAULT 'deepseek-embed',
    dim INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(entity_kind, entity_id)
);
CREATE INDEX IF NOT EXISTS idx_knowledge_embeddings_kind ON fr_knowledge_embeddings(entity_kind);
