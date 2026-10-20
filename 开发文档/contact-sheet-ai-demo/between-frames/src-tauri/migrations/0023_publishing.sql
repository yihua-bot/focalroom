-- G7 Publishing Studio：摄影书/展览/作品集数据层。
-- expand 阶段：纯新增表。

-- 出版物（书籍/展览/作品集的顶层实体）。
CREATE TABLE IF NOT EXISTS publications (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    -- photobook / exhibition / portfolio
    pub_type TEXT NOT NULL DEFAULT 'photobook' CHECK(pub_type IN ('photobook','exhibition','portfolio')),
    title TEXT NOT NULL DEFAULT '未命名出版物',
    -- 出版规格（JSON：开本/纸张/装订/出血/色彩等）
    spec_json TEXT NOT NULL DEFAULT '{}',
    -- draft / in_review / finalized / delivered
    state TEXT NOT NULL DEFAULT 'draft' CHECK(state IN ('draft','in_review','finalized','delivered')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 出版页（每页有物理尺寸和布局参数）。
CREATE TABLE IF NOT EXISTS publication_pages (
    id TEXT PRIMARY KEY,
    publication_id TEXT NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    -- 页码（从 1 开始；0 = 封面；-1 = 封底）
    page_number INTEGER NOT NULL,
    -- 左页或右页（跨页 = spread）
    side TEXT NOT NULL DEFAULT 'right' CHECK(side IN ('left','right','spread','cover','back')),
    -- 物理尺寸（mm；从 publication spec 继承但可覆盖）
    width_mm REAL NOT NULL DEFAULT 210,
    height_mm REAL NOT NULL DEFAULT 297,
    -- 边距/出血（mm）
    margin_mm REAL NOT NULL DEFAULT 15,
    bleed_mm REAL NOT NULL DEFAULT 3,
    -- 母版/网格参数（JSON）
    layout_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(publication_id, page_number)
);

-- 页面图片框（引用 Rendition，不复制像素）。
CREATE TABLE IF NOT EXISTS publication_frames (
    id TEXT PRIMARY KEY,
    page_id TEXT NOT NULL REFERENCES publication_pages(id) ON DELETE CASCADE,
    -- 框位置/尺寸（mm，相对于页面左上角）
    x_mm REAL NOT NULL DEFAULT 10,
    y_mm REAL NOT NULL DEFAULT 10,
    width_mm REAL NOT NULL DEFAULT 100,
    height_mm REAL NOT NULL DEFAULT 100,
    -- 填充方式：fit（完整显示）/ fill（裁切填满）/ stretch
    fit_mode TEXT NOT NULL DEFAULT 'fit' CHECK(fit_mode IN ('fit','fill','stretch')),
    -- 引用的资产+渲染版本
    asset_id TEXT REFERENCES source_assets(id),
    rendition_id TEXT REFERENCES renditions(id),
    -- 裁切偏移（归一化 0-1；相对于源图）
    crop_x REAL NOT NULL DEFAULT 0.5,
    crop_y REAL NOT NULL DEFAULT 0.5,
    crop_zoom REAL NOT NULL DEFAULT 1.0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pub_frames_page ON publication_frames(page_id);

-- 文字框（标题/正文/页码等）。
CREATE TABLE IF NOT EXISTS publication_texts (
    id TEXT PRIMARY KEY,
    page_id TEXT NOT NULL REFERENCES publication_pages(id) ON DELETE CASCADE,
    x_mm REAL NOT NULL DEFAULT 10,
    y_mm REAL NOT NULL DEFAULT 10,
    width_mm REAL NOT NULL DEFAULT 100,
    height_mm REAL NOT NULL DEFAULT 20,
    -- 文字内容（JSON：富文本范围受控）
    content_json TEXT NOT NULL DEFAULT '{}',
    -- 样式（字体/大小/颜色/对齐）
    style_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 输出合同（印厂规格）。
CREATE TABLE IF NOT EXISTS print_contracts (
    id TEXT PRIMARY KEY,
    publication_id TEXT NOT NULL REFERENCES publications(id) ON DELETE CASCADE,
    -- 目标印厂/输出设备
    target TEXT NOT NULL DEFAULT 'unknown',
    -- PDF/X 版本 / 微喷 TIFF 等
    output_format TEXT NOT NULL DEFAULT 'pdf_x_4' CHECK(output_format IN ('pdf_x_1a','pdf_x_4','tiff_rgb_16','tiff_cmyk_8','jpeg_proof')),
    -- ICC profile 要求
    icc_requirement TEXT,
    -- 分辨率要求（PPI）
    min_ppi INTEGER NOT NULL DEFAULT 300,
    -- 出血/裁切标记
    bleed_mm REAL NOT NULL DEFAULT 3,
    crop_marks BOOLEAN NOT NULL DEFAULT true,
    -- 预检结果（JSON：最后一次检查）
    preflight_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_print_contracts ON print_contracts(publication_id);
