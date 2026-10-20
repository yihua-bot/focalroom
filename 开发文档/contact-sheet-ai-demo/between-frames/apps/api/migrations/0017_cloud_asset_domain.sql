-- G1 Asset Domain 3.0（云端）：fr_source_assets / fr_asset_aliases / fr_edit_variants / fr_renditions。
-- expand 阶段：纯新增表，不改 fr_previews / fr_projects 任何列；可随时 DROP 回滚。
-- 回填在 API 启动后由代码执行（从 fr_previews 提取）。

CREATE TABLE IF NOT EXISTS fr_source_assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id TEXT NOT NULL REFERENCES fr_projects(id) ON DELETE CASCADE,
    photo_id TEXT NOT NULL,                    -- 桌面端 source_asset id（与本地同值）
    content_sha256 TEXT NOT NULL,
    original_locator JSONB NOT NULL DEFAULT '[]',
    file_size BIGINT NOT NULL DEFAULT 0,
    width INTEGER NOT NULL DEFAULT 0,
    height INTEGER NOT NULL DEFAULT 0,
    format TEXT NOT NULL DEFAULT 'unknown',
    source_profile_digest TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(project_id, photo_id)
);
CREATE INDEX IF NOT EXISTS idx_fr_source_assets_project ON fr_source_assets(project_id);

CREATE TABLE IF NOT EXISTS fr_asset_aliases (
    photo_id TEXT PRIMARY KEY,
    asset_id UUID NOT NULL REFERENCES fr_source_assets(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES fr_projects(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_fr_asset_aliases_asset ON fr_asset_aliases(asset_id);

CREATE TABLE IF NOT EXISTS fr_edit_variants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id UUID NOT NULL REFERENCES fr_source_assets(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    external_software TEXT,
    external_variant_id TEXT,
    is_current BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(asset_id, name)
);

CREATE TABLE IF NOT EXISTS fr_renditions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    variant_id UUID NOT NULL REFERENCES fr_edit_variants(id) ON DELETE CASCADE,
    sha256 TEXT NOT NULL,
    object_key TEXT NOT NULL,                  -- 对象存储键
    width INTEGER NOT NULL,
    height INTEGER NOT NULL DEFAULT 0,
    bit_depth SMALLINT NOT NULL DEFAULT 8,
    icc_digest TEXT,
    source_type TEXT NOT NULL CHECK(source_type IN ('self_render','external_lightroom','external_capture_one','external_unknown')),
    source_ref TEXT,
    file_size BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(variant_id, sha256)
);
CREATE INDEX IF NOT EXISTS idx_fr_renditions_variant ON fr_renditions(variant_id);

DO $$ BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname='focalroom_app') THEN
    GRANT SELECT,INSERT,UPDATE,DELETE ON fr_source_assets, fr_asset_aliases, fr_edit_variants, fr_renditions TO focalroom_app;
  END IF;
END $$;
