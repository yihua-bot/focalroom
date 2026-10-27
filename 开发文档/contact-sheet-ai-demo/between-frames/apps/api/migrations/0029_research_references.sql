-- I1 研究参考（剩余清单）：项目级笔记/链接与作者/来源/许可元数据。
-- 参考是辅助资料：独立身份域，不进入 fr_source_assets/photos，不计入入选数量或出版；
-- 不推进正式修订、不写版本事件。图片参考与 Moodboard UI 待产品形态定向另做。

CREATE TABLE IF NOT EXISTS fr_research_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id TEXT NOT NULL REFERENCES fr_projects(id),
    kind TEXT NOT NULL CHECK(kind IN ('note','link')),
    title TEXT NOT NULL,
    body TEXT,
    url TEXT,
    author TEXT,
    source TEXT,
    license TEXT,
    created_by TEXT NOT NULL REFERENCES fr_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK((kind = 'note' AND body IS NOT NULL) OR (kind = 'link' AND url IS NOT NULL))
);
CREATE INDEX IF NOT EXISTS idx_fr_research_references_project
    ON fr_research_references(project_id, created_at);

DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT SELECT, INSERT, UPDATE, DELETE ON fr_research_references TO focalroom_app;
  END IF;
END
$$;
