-- 云端作者记忆（M3#11 / 剩余清单 #9）：Creator 隔离的跨项目偏好假设。
-- 0010_author_memory.sql 是未登记设计稿（UUID 外键指向不存在的 cloud_projects），
-- 历史文件字节不变；本迁移以 fr_projects 的 text 主键落实兼容结构。

CREATE TABLE IF NOT EXISTS fr_author_hypotheses (
    id text PRIMARY KEY,
    project_id text NOT NULL REFERENCES fr_projects(id) ON DELETE CASCADE,
    statement text NOT NULL CHECK(length(statement) BETWEEN 1 AND 1000),
    status text NOT NULL DEFAULT 'unconfirmed' CHECK(status IN('unconfirmed','confirmed','corrected','retired')),
    confidence text NOT NULL DEFAULT 'tentative' CHECK(confidence IN('tentative','moderate')),
    disabled_for_retrieval boolean NOT NULL DEFAULT false,
    evidence_ids_json text NOT NULL DEFAULT '[]',
    counterexample_ids_json text NOT NULL DEFAULT '[]',
    user_correction text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(project_id, statement)
);
CREATE INDEX IF NOT EXISTS idx_author_hypo_project
    ON fr_author_hypotheses(project_id, status);
CREATE INDEX IF NOT EXISTS idx_author_hypo_updated
    ON fr_author_hypotheses(updated_at DESC);

DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT SELECT, INSERT, UPDATE, DELETE ON fr_author_hypotheses TO focalroom_app;
  END IF;
END
$$;
