-- G1 Asset Domain 3.0：SourceAsset / Representation / Variant / Rendition 身份体系。
-- expand 阶段：纯新增表+回填，不改 photos 任何列；alias 映射表保护历史 photo_id 引用。
-- 回滚：DROP 这五张表即可恢复原状。

-- 1) 源资产（母版）：只读身份，从 photos 提取。
--    id 复用 photo id（alias 双向稳定），content_sha256 不变。
CREATE TABLE IF NOT EXISTS source_assets (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    content_sha256 TEXT NOT NULL,
    original_locator TEXT NOT NULL,          -- JSON: [{path, bookmarkData?, kind:"file"}]
    file_size INTEGER NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    format TEXT NOT NULL DEFAULT 'jpeg',    -- jpeg/png/tiff/heic/cr2/unknown
    source_profile_digest TEXT,              -- 源 ICC digest，缺失为 NULL
    source_hash_verified_at TEXT,            -- 最近一次哈希校验时间
    created_at TEXT NOT NULL,
    UNIQUE(project_id, content_sha256)
);

-- 2) photo_id → asset_id 映射（兼容旧引用：agent_steps/drafts/evidence/comments）。
CREATE TABLE IF NOT EXISTS asset_aliases (
    photo_id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES source_assets(id)
);
CREATE INDEX IF NOT EXISTS idx_asset_aliases_asset ON asset_aliases(asset_id);

-- 3) 资产表达（可显示版本）：缩略图/代理/AI 预览/印刷母版各有身份。
CREATE TABLE IF NOT EXISTS asset_representations (
    id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES source_assets(id),
    kind TEXT NOT NULL CHECK(kind IN ('thumbnail','display_proxy','ai_preview','print_master','tile_pyramid')),
    algorithm TEXT NOT NULL DEFAULT 'image_pipeline_v1',
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    profile_digest TEXT,                     -- 输出色彩 profile digest
    created_at TEXT NOT NULL,
    UNIQUE(asset_id, kind, width, height)
);
CREATE INDEX IF NOT EXISTS idx_repr_asset_kind ON asset_representations(asset_id, kind);

-- 4) 编辑变体：同一母版的不同调色/审美方向。
--    外部软件（Lightroom/C1）的变体通过 external_variant_id 绑定。
CREATE TABLE IF NOT EXISTS edit_variants (
    id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES source_assets(id),
    name TEXT NOT NULL,
    external_software TEXT,                  -- lightroom_classic/capture_one/null
    external_variant_id TEXT,               -- 外部软件的稳定变体 ID
    is_current INTEGER NOT NULL DEFAULT 0,  -- 当前显示变体（每 asset 最多 1）
    created_at TEXT NOT NULL,
    UNIQUE(asset_id, name)
);

-- 5) 渲染结果：不可变文件（SHA-256 锚定），外部回流/自身渲染产物。
CREATE TABLE IF NOT EXISTS renditions (
    id TEXT PRIMARY KEY,
    variant_id TEXT NOT NULL REFERENCES edit_variants(id),
    sha256 TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    bit_depth INTEGER NOT NULL DEFAULT 8,
    icc_digest TEXT,
    source_type TEXT NOT NULL CHECK(source_type IN ('self_render','external_lightroom','external_capture_one','external_unknown')),
    source_ref TEXT,                         -- manifest ID / 插件批次 ID
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(variant_id, sha256)
);
CREATE INDEX IF NOT EXISTS idx_renditions_variant ON renditions(variant_id);

-- 回填：photos → source_assets + alias + 2 条 representation
-- 在迁移执行器中用代码完成（需要读 photos 行生成 UUID+复制路径），SQL 仅建表。
