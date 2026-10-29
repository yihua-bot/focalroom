-- K0 摄影知识本体（Focal Photography Intelligence）：
-- 八类知识对象（摄影师/作品/系列/摄影书/流派/展览/理论/技法）
-- + 事实断言（Claim，含出处与争议空间）+ 版权状态 + 审校流程。
-- 所有知识为公共层（区别于项目层与个人审美记忆层），只读服务所有终端。

-- 摄影师：生平、创作阶段、方法、影响。
CREATE TABLE fr_photographers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name_en TEXT NOT NULL,
    name_zh TEXT,
    also_known_as TEXT[] NOT NULL DEFAULT '{}',
    birth_year INTEGER CHECK(birth_year IS NULL OR (birth_year >= 1700 AND birth_year <= 2100)),
    death_year INTEGER CHECK(death_year IS NULL OR (death_year >= 1700 AND death_year <= 2100)),
    nationality TEXT,
    biography TEXT,
    method_notes TEXT,
    movements TEXT[] NOT NULL DEFAULT '{}',
    review_status TEXT NOT NULL DEFAULT 'imported' CHECK(review_status IN ('imported','verified','editor_reviewed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(name_en)
);

-- 摄影系列：创作意图、方法、关联作品集合。
CREATE TABLE fr_photo_series (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    photographer_id UUID NOT NULL REFERENCES fr_photographers(id) ON DELETE CASCADE,
    title_en TEXT NOT NULL,
    title_zh TEXT,
    intent TEXT,
    method_notes TEXT,
    start_year INTEGER,
    end_year INTEGER,
    capture_format TEXT,
    review_status TEXT NOT NULL DEFAULT 'imported',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(photographer_id, title_en)
);

-- 摄影作品：拍摄年份与印相年份分开；媒介、主题、视觉概念。
CREATE TABLE fr_artworks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    photographer_id UUID NOT NULL REFERENCES fr_photographers(id) ON DELETE CASCADE,
    series_id UUID REFERENCES fr_photo_series(id) ON DELETE SET NULL,
    title_en TEXT NOT NULL,
    title_zh TEXT,
    capture_date TEXT,
    print_date TEXT,
    medium TEXT,
    capture_format TEXT,
    subjects TEXT[] NOT NULL DEFAULT '{}',
    visual_concepts TEXT[] NOT NULL DEFAULT '{}',
    editorial_notes TEXT,
    review_status TEXT NOT NULL DEFAULT 'imported',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_fr_artworks_photographer ON fr_artworks(photographer_id);
CREATE INDEX idx_fr_artworks_series ON fr_artworks(series_id);

-- 摄影书：出版版本、编排、装帧。
CREATE TABLE fr_photobooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    photographer_id UUID REFERENCES fr_photographers(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    publisher TEXT,
    publication_year INTEGER CHECK(publication_year IS NULL OR (publication_year >= 1800 AND publication_year <= 2100)),
    edition_notes TEXT,
    structure_notes TEXT,
    review_status TEXT NOT NULL DEFAULT 'imported',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 事实断言：每条史实/观点独立可引用，标明出处、类型与争议空间。
CREATE TABLE fr_knowledge_claims (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type TEXT NOT NULL CHECK(entity_type IN ('photographer','series','artwork','photobook','movement','exhibition','theory','technique')),
    entity_id UUID NOT NULL,
    claim_type TEXT NOT NULL CHECK(claim_type IN ('historical_fact','curatorial_interpretation','focalroom_recommendation')),
    claim_text TEXT NOT NULL,
    sources JSONB NOT NULL DEFAULT '[]',
    disputed BOOLEAN NOT NULL DEFAULT false,
    dispute_notes TEXT,
    review_status TEXT NOT NULL DEFAULT 'imported',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_fr_knowledge_claims_entity ON fr_knowledge_claims(entity_type, entity_id);

-- 知识关系：实体之间的有向关系（影响、属于流派、包含作品等）。
CREATE TABLE fr_knowledge_relations (
    from_type TEXT NOT NULL CHECK(from_type IN ('photographer','series','artwork','photobook','movement','exhibition','theory','technique')),
    from_id UUID NOT NULL,
    relation TEXT NOT NULL,
    to_type TEXT NOT NULL CHECK(to_type IN ('photographer','series','artwork','photobook','movement','exhibition','theory','technique')),
    to_id UUID NOT NULL,
    source_claim_id UUID REFERENCES fr_knowledge_claims(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY(from_type, from_id, relation, to_type, to_id)
);

-- 知识来源：博物馆/官方 URL、登记号、验证时间。
CREATE TABLE fr_knowledge_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    url TEXT NOT NULL,
    institution TEXT NOT NULL,
    accession_number TEXT,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_fr_knowledge_sources_entity ON fr_knowledge_sources(entity_type, entity_id);

DO $$
BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname = 'focalroom_app') THEN
    GRANT SELECT ON fr_photographers, fr_photo_series, fr_artworks, fr_photobooks, fr_knowledge_claims, fr_knowledge_relations, fr_knowledge_sources TO focalroom_app;
  END IF;
END
$$;
