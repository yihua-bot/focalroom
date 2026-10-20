-- G6 Shared Workspace：云端画布（Board/CanvasNode/BoardAnnotation）。
-- expand 阶段：纯新增，不改 fr_projects/fr_previews。

CREATE TABLE IF NOT EXISTS fr_boards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id TEXT NOT NULL REFERENCES fr_projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL DEFAULT '未命名画布',
    state TEXT NOT NULL DEFAULT 'draft' CHECK(state IN ('draft','official','frozen')),
    created_by TEXT NOT NULL REFERENCES fr_users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(project_id, title)
);

CREATE TABLE IF NOT EXISTS fr_canvas_nodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    board_id UUID NOT NULL REFERENCES fr_boards(id) ON DELETE CASCADE,
    node_type TEXT NOT NULL CHECK(node_type IN ('photo','text','sticky','shape','arrow','ink','group','chapter')),
    x DOUBLE PRECISION NOT NULL DEFAULT 0,
    y DOUBLE PRECISION NOT NULL DEFAULT 0,
    width DOUBLE PRECISION NOT NULL DEFAULT 200,
    height DOUBLE PRECISION NOT NULL DEFAULT 200,
    rotation DOUBLE PRECISION NOT NULL DEFAULT 0,
    z_order INTEGER NOT NULL DEFAULT 0,
    asset_id TEXT,
    rendition_ref TEXT,
    content_json JSONB,
    style_json JSONB,
    children_json JSONB,
    locked BOOLEAN NOT NULL DEFAULT false,
    visible BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_fr_canvas_nodes_board ON fr_canvas_nodes(board_id, z_order);

CREATE TABLE IF NOT EXISTS fr_board_annotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    board_id UUID NOT NULL REFERENCES fr_boards(id) ON DELETE CASCADE,
    anchor_type TEXT NOT NULL DEFAULT 'board' CHECK(anchor_type IN ('board','node','normalized')),
    x DOUBLE PRECISION,
    y DOUBLE PRECISION,
    node_id UUID REFERENCES fr_canvas_nodes(id) ON DELETE SET NULL,
    nx DOUBLE PRECISION,
    ny DOUBLE PRECISION,
    author_id TEXT NOT NULL REFERENCES fr_users(id),
    body TEXT NOT NULL,
    resolved BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_fr_board_annotations ON fr_board_annotations(board_id, resolved);

DO $$ BEGIN
  IF EXISTS (SELECT FROM pg_roles WHERE rolname='focalroom_app') THEN
    GRANT SELECT,INSERT,UPDATE,DELETE ON fr_boards, fr_canvas_nodes, fr_board_annotations TO focalroom_app;
  END IF;
END $$;
