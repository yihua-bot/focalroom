-- G3 Narrative Canvas：Board / CanvasNode 数据层。
-- expand 阶段：纯新增，不改现有表。Board 是项目内的自由画布工作空间。
-- CanvasNode 引用 source_assets（G1），不复制图片内容。

CREATE TABLE IF NOT EXISTS boards (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    title TEXT NOT NULL DEFAULT '未命名画布',
    -- Draft（探索状态）/ Official（正式修订）/ Frozen（出版定稿）
    state TEXT NOT NULL DEFAULT 'draft' CHECK(state IN ('draft','official','frozen')),
    -- 画布视口（非持久化，但保存最后位置方便恢复）
    last_view_x REAL NOT NULL DEFAULT 0,
    last_view_y REAL NOT NULL DEFAULT 0,
    last_view_zoom REAL NOT NULL DEFAULT 1.0 CHECK(last_view_zoom > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(project_id, title)
);

CREATE TABLE IF NOT EXISTS canvas_nodes (
    id TEXT PRIMARY KEY,
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    -- 节点类型：photo / text / sticky / shape / arrow / ink / group / chapter
    node_type TEXT NOT NULL CHECK(node_type IN ('photo','text','sticky','shape','arrow','ink','group','chapter')),
    -- 逻辑坐标（独立于设备像素；v3 蓝图 §06.1）
    x REAL NOT NULL DEFAULT 0,
    y REAL NOT NULL DEFAULT 0,
    width REAL NOT NULL DEFAULT 200,
    height REAL NOT NULL DEFAULT 200,
    rotation REAL NOT NULL DEFAULT 0 CHECK(rotation >= -360 AND rotation <= 360),
    z_order INTEGER NOT NULL DEFAULT 0,
    -- 照片节点引用源资产；其他类型为 NULL
    asset_id TEXT REFERENCES source_assets(id),
    -- 照片节点引用的渲染版本
    rendition_ref TEXT,
    -- 文本/便签内容（JSON：富文本范围受控）
    content_json TEXT,
    -- 形状/箭头参数（JSON：颜色、线宽、端点等）
    style_json TEXT,
    -- 分组节点的子节点 ID 列表（JSON array）
    children_json TEXT,
    -- 锁定/可见
    locked INTEGER NOT NULL DEFAULT 0,
    visible INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_canvas_nodes_board ON canvas_nodes(board_id, z_order);
CREATE INDEX IF NOT EXISTS idx_canvas_nodes_asset ON canvas_nodes(asset_id);

-- 画布评论锚点（扩展 photo_decisions 体系，支持 Board 坐标锚定）
CREATE TABLE IF NOT EXISTS board_annotations (
    id TEXT PRIMARY KEY,
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    -- 锚定方式：board（画布绝对坐标）/ node（节点引用）/ normalized（节点内归一化坐标）
    anchor_type TEXT NOT NULL DEFAULT 'board' CHECK(anchor_type IN ('board','node','normalized')),
    -- board 锚定时使用 x,y；node 锚定时使用 node_id；normalized 锚定时 node_id + nx,ny
    x REAL,
    y REAL,
    node_id TEXT REFERENCES canvas_nodes(id) ON DELETE SET NULL,
    nx REAL,
    ny REAL,
    author TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL,
    resolved INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_board_annotations ON board_annotations(board_id, resolved);
