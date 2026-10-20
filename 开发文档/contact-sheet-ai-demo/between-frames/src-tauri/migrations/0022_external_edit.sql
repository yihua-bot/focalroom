-- G5 Round-trip：外部编辑器（Lightroom/Capture One）往返数据层。
-- expand 阶段：纯新增表，可随时 DROP 回滚。

-- 外部编辑连接：FocalRoom 资产 ↔ Lightroom/C1 的版本绑定。
CREATE TABLE IF NOT EXISTS external_edit_links (
    id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL REFERENCES source_assets(id),
    -- lightroom_classic / capture_one
    provider TEXT NOT NULL CHECK(provider IN ('lightroom_classic','capture_one')),
    -- 外部软件的稳定照片 ID（LR catalogPhotoId / C1 variantId）
    external_photo_id TEXT NOT NULL,
    external_variant_id TEXT,
    -- 外部目录指纹（LR catalog UUID / C1 session ID）
    catalog_fingerprint TEXT,
    -- 绑定状态
    state TEXT NOT NULL DEFAULT 'active' CHECK(state IN ('active','broken','unmatched')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(asset_id, provider, external_photo_id)
);
CREATE INDEX IF NOT EXISTS idx_ext_links_asset ON external_edit_links(asset_id);
CREATE INDEX IF NOT EXISTS idx_ext_links_provider ON external_edit_links(provider, catalog_fingerprint);

-- 外发/回流批次管理。
CREATE TABLE IF NOT EXISTS transfer_jobs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    -- outgoing（FocalRoom→外部）/ incoming（外部→FocalRoom）
    direction TEXT NOT NULL CHECK(direction IN ('outgoing','incoming')),
    provider TEXT NOT NULL CHECK(provider IN ('lightroom_classic','capture_one')),
    -- PREPARED→DISPATCHED→EDITING_EXTERNAL→RENDER_DETECTED→VERIFYING→MATCHED→REVIEW_PENDING→ACCEPTED/REJECTED
    state TEXT NOT NULL DEFAULT 'prepared' CHECK(state IN (
        'prepared','dispatched','editing_external','render_detected',
        'verifying','matched','review_pending','accepted','rejected',
        'ambiguous_match','source_changed','wrong_profile','broken_link','export_incomplete'
    )),
    -- 批次内资产列表（JSON: [{assetId, externalPhotoId}])
    items_json TEXT NOT NULL DEFAULT '[]',
    -- manifest 引用（incoming 时由外部生成）
    manifest_path TEXT,
    manifest_sha256 TEXT,
    -- 渲染文件目录
    render_directory TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_transfer_jobs_project ON transfer_jobs(project_id, state);
