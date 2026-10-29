-- I1 研究素材复用画布（用户决策 2026-10-28）：新增 reference 节点类型。
-- reference 节点承载研究参考（链接/笔记 + 作者/来源/许可元数据），
-- node_type 独立于 photo，不计入选片与照片资产来源。SQLite 的 CHECK
-- 约束不能原地修改，按仓库既有模式重建 canvas_nodes。
CREATE TABLE canvas_nodes_v34 (
    id TEXT PRIMARY KEY,
    board_id TEXT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    node_type TEXT NOT NULL CHECK(node_type IN ('photo','text','sticky','shape','arrow','ink','group','chapter','reference')),
    x REAL NOT NULL DEFAULT 0,
    y REAL NOT NULL DEFAULT 0,
    width REAL NOT NULL DEFAULT 200,
    height REAL NOT NULL DEFAULT 200,
    rotation REAL NOT NULL DEFAULT 0 CHECK(rotation >= -360 AND rotation <= 360),
    z_order INTEGER NOT NULL DEFAULT 0,
    asset_id TEXT REFERENCES source_assets(id),
    rendition_ref TEXT,
    content_json TEXT,
    style_json TEXT,
    children_json TEXT,
    locked INTEGER NOT NULL DEFAULT 0,
    visible INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
INSERT INTO canvas_nodes_v34(id,board_id,node_type,x,y,width,height,rotation,z_order,asset_id,rendition_ref,content_json,style_json,children_json,locked,visible,created_at,updated_at)
    SELECT id,board_id,node_type,x,y,width,height,rotation,z_order,asset_id,rendition_ref,content_json,style_json,children_json,locked,visible,created_at,updated_at
    FROM canvas_nodes ORDER BY rowid;
DROP TABLE canvas_nodes;
ALTER TABLE canvas_nodes_v34 RENAME TO canvas_nodes;
CREATE INDEX idx_canvas_nodes_board ON canvas_nodes(board_id, z_order);
CREATE INDEX idx_canvas_nodes_asset ON canvas_nodes(asset_id);
-- 0027 的画布修订触发器随旧表删除，原样重建。
CREATE TRIGGER board_revision_bump
AFTER INSERT ON canvas_nodes
BEGIN
  UPDATE boards SET revision = revision + 1, updated_at = datetime('now') WHERE id = NEW.board_id;
END;
CREATE TRIGGER board_revision_bump_update
AFTER UPDATE OF x, y, width, height, visible, locked ON canvas_nodes
BEGIN
  UPDATE boards SET revision = revision + 1, updated_at = datetime('now') WHERE id = NEW.board_id;
END;
