-- M3#11 云端作者记忆：Creator 隔离的跨项目偏好假设存储。
CREATE TABLE IF NOT EXISTS fr_author_hypotheses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES cloud_projects(id) ON DELETE CASCADE,
    statement TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'unconfirmed' CHECK(status IN('unconfirmed','confirmed','corrected','retired')),
    confidence TEXT NOT NULL DEFAULT 'tentative' CHECK(confidence IN('tentative','moderate')),
    disabled_for_retrieval BOOLEAN NOT NULL DEFAULT false,
    evidence_ids_json TEXT NOT NULL DEFAULT '[]',
    counterexample_ids_json TEXT NOT NULL DEFAULT '[]',
    user_correction TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(project_id, statement)
);
CREATE INDEX IF NOT EXISTS idx_author_hypo_project
    ON fr_author_hypotheses(project_id, status);
