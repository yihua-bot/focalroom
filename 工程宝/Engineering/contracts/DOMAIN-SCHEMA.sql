-- FocalRoom v3 / PostgreSQL TARGET DOMAIN BLUEPRINT
-- DESIGN EXAMPLE ONLY: do NOT execute against existing production databases.
-- Existing migrations and current schema must first be inventoried; use additive migrations.
-- Authoritative shared project schema. Local SQLite requires semantically equivalent adapters.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE fr3_projects (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), workspace_id uuid NOT NULL,
 title text NOT NULL, creative_brief text NOT NULL DEFAULT '',
 lifecycle text NOT NULL DEFAULT 'ACTIVE' CHECK(lifecycle IN ('ACTIVE','ARCHIVED','DELETED')),
 formal_revision bigint NOT NULL DEFAULT 0,
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE fr3_source_assets (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), project_id uuid NOT NULL REFERENCES fr3_projects(id),
 mime_type text NOT NULL, original_filename text NOT NULL,
 source_sha256 text NOT NULL CHECK(source_sha256 ~ '^[a-f0-9]{64}$'),
 width bigint, height bigint, bit_depth smallint, source_icc_sha256 text,
 original_is_mutable boolean NOT NULL DEFAULT false CHECK (original_is_mutable=false),
 created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(project_id,id)
);
CREATE INDEX fr3_assets_project_hash ON fr3_source_assets(project_id,source_sha256);
CREATE TABLE fr3_asset_locations (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), asset_id uuid NOT NULL REFERENCES fr3_source_assets(id),
 locator_kind text NOT NULL CHECK(locator_kind IN ('LOCAL_BOOKMARK','EXTERNAL_CATALOG','CLOUD_OBJECT')),
 locator_json jsonb NOT NULL, available boolean NOT NULL DEFAULT true,
 last_verified_at timestamptz, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE fr3_edit_variants (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), asset_id uuid NOT NULL REFERENCES fr3_source_assets(id),
 name text NOT NULL, edit_authority text NOT NULL CHECK(edit_authority IN ('LIGHTROOM_CLASSIC','CAPTURE_ONE','EXTERNAL_RENDER','FOCALROOM')),
 external_version text, created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(asset_id,id)
);
CREATE TABLE fr3_renditions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), variant_id uuid NOT NULL REFERENCES fr3_edit_variants(id),
 source_asset_id uuid NOT NULL REFERENCES fr3_source_assets(id),
 CONSTRAINT fk_variant_asset_consistency FOREIGN KEY(source_asset_id,variant_id) REFERENCES fr3_edit_variants(asset_id,id),
 media_format text NOT NULL, file_sha256 text NOT NULL CHECK(file_sha256 ~ '^[a-f0-9]{64}$'),
 width bigint NOT NULL CHECK(width > 0), height bigint NOT NULL CHECK(height > 0),
 bit_depth smallint NOT NULL, color_profile_sha256 text,
 render_intent text, created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(id,source_asset_id)
);
CREATE INDEX fr3_renditions_by_asset ON fr3_renditions(source_asset_id,created_at DESC);
CREATE TABLE fr3_representations (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), rendition_id uuid NOT NULL REFERENCES fr3_renditions(id),
 representation_type text NOT NULL CHECK(representation_type IN('THUMBNAIL','DISPLAY_PROXY','TILE_PYRAMID','AI_PREVIEW','PRINT_MASTER')),
 content_sha256 text NOT NULL, width bigint, height bigint,
 source_profile_sha256 text, target_profile_sha256 text, color_pipeline_version text NOT NULL,
 storage_key text NOT NULL, generated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(rendition_id,representation_type,color_pipeline_version,target_profile_sha256)
);
CREATE TABLE fr3_boards (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), project_id uuid NOT NULL REFERENCES fr3_projects(id),
 title text NOT NULL, status text NOT NULL DEFAULT 'DRAFT' CHECK(status IN('DRAFT','SUBMITTED','OFFICIAL','ARCHIVED')),
 base_formal_revision bigint NOT NULL DEFAULT 0, board_schema_version integer NOT NULL DEFAULT 1,
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE fr3_board_nodes (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), board_id uuid NOT NULL REFERENCES fr3_boards(id),
 kind text NOT NULL CHECK(kind IN('PHOTO','TEXT','STICKY','GROUP','FRAME','ARROW','INK','SHAPE')),
 asset_id uuid REFERENCES fr3_source_assets(id), rendition_id uuid REFERENCES fr3_renditions(id),
 CONSTRAINT fk_canvas_rendition_asset FOREIGN KEY(rendition_id,asset_id) REFERENCES fr3_renditions(id,source_asset_id),
 parent_id uuid REFERENCES fr3_board_nodes(id),
 x double precision NOT NULL DEFAULT 0,y double precision NOT NULL DEFAULT 0,
 w double precision NOT NULL DEFAULT 100 CHECK(w > 0),h double precision NOT NULL DEFAULT 100 CHECK(h > 0),
 rotation double precision NOT NULL DEFAULT 0,z_index bigint NOT NULL DEFAULT 0,
 locked boolean NOT NULL DEFAULT false, properties jsonb NOT NULL DEFAULT '{}'::jsonb,
 edited_at timestamptz NOT NULL DEFAULT now(),
 CHECK(kind <> 'PHOTO' OR asset_id IS NOT NULL)
);
CREATE INDEX fr3_board_node_order ON fr3_board_nodes(board_id,z_index);
CREATE TABLE fr3_sequences (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),project_id uuid NOT NULL REFERENCES fr3_projects(id),
 name text NOT NULL,revision bigint NOT NULL DEFAULT 0,created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE fr3_sequence_items (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),sequence_id uuid NOT NULL REFERENCES fr3_sequences(id),
 asset_id uuid REFERENCES fr3_source_assets(id),rendition_id uuid REFERENCES fr3_renditions(id),
 position integer NOT NULL, chapter_id uuid, note text NOT NULL DEFAULT '',
 UNIQUE(sequence_id,position)
);
CREATE TABLE fr3_comments (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), project_id uuid NOT NULL REFERENCES fr3_projects(id),
 author_id uuid NOT NULL,parent_comment_id uuid REFERENCES fr3_comments(id),
 anchor_type text NOT NULL CHECK(anchor_type IN('PHOTO_POINT','BOARD_POINT','BOARD_NODE','SEQUENCE_ITEM','PUBLICATION_PAGE','PROJECT_REVISION')),
 anchor_json jsonb NOT NULL, body text NOT NULL, state text NOT NULL DEFAULT 'OPEN' CHECK(state IN('OPEN','RESOLVED')),
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE fr3_publications (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),project_id uuid NOT NULL REFERENCES fr3_projects(id),
 kind text NOT NULL CHECK(kind IN('PHOTOBOOK','EXHIBITION','PORTFOLIO')),
 title text NOT NULL, spec_json jsonb NOT NULL, status text NOT NULL DEFAULT 'DRAFT',
 created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE fr3_publication_pages (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), publication_id uuid NOT NULL REFERENCES fr3_publications(id),
 page_index integer NOT NULL, page_spec jsonb NOT NULL,
 UNIQUE(publication_id,page_index)
);
CREATE TABLE fr3_publication_frames (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), page_id uuid NOT NULL REFERENCES fr3_publication_pages(id),
 rendition_id uuid REFERENCES fr3_renditions(id), frame_kind text NOT NULL,
 geometry_json jsonb NOT NULL, content_json jsonb NOT NULL DEFAULT '{}'::jsonb
);
CREATE TABLE fr3_external_edit_links (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),source_asset_id uuid NOT NULL REFERENCES fr3_source_assets(id),
 variant_id uuid REFERENCES fr3_edit_variants(id),provider text NOT NULL CHECK(provider IN('LIGHTROOM_CLASSIC','CAPTURE_ONE')),
 catalog_fingerprint text NOT NULL,external_photo_id text NOT NULL,external_variant_id text,
 last_seen_external_version text, created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(provider,catalog_fingerprint,external_photo_id,external_variant_id)
);
CREATE TABLE fr3_transfer_jobs (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),project_id uuid NOT NULL REFERENCES fr3_projects(id),
 connector text NOT NULL, external_batch_key text NOT NULL,status text NOT NULL,
 manifest_sha256 text, error_code text, result_json jsonb NOT NULL DEFAULT '{}'::jsonb,
 created_at timestamptz NOT NULL DEFAULT now(),updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(project_id,external_batch_key)
);
CREATE TABLE fr3_project_commands (
 id uuid PRIMARY KEY,project_id uuid NOT NULL REFERENCES fr3_projects(id),
 actor_id uuid NOT NULL,idempotency_key text NOT NULL, base_revision bigint NOT NULL,
 command_type text NOT NULL, payload jsonb NOT NULL, payload_sha256 text NOT NULL,
 result_json jsonb NOT NULL, resulting_revision bigint,
 created_at timestamptz NOT NULL DEFAULT now(),UNIQUE(project_id,actor_id,idempotency_key)
);
CREATE TABLE fr3_events (
 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,project_id uuid NOT NULL REFERENCES fr3_projects(id),
 revision bigint NOT NULL,command_id uuid REFERENCES fr3_project_commands(id),
 entity_type text NOT NULL,entity_id uuid NOT NULL,event_type text NOT NULL,
 payload jsonb NOT NULL,created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(project_id,revision,id)
);
CREATE INDEX fr3_event_stream ON fr3_events(project_id,id);
CREATE TABLE fr3_review_proposals (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),project_id uuid NOT NULL REFERENCES fr3_projects(id),
 author_id uuid NOT NULL,source text NOT NULL CHECK(source IN('HUMAN','AGENT')),
 base_revision bigint NOT NULL,change_set jsonb NOT NULL,
 state text NOT NULL DEFAULT 'DRAFT' CHECK(state IN('DRAFT','SUBMITTED','APPROVED','REJECTED','STALE')),
 created_at timestamptz NOT NULL DEFAULT now(),approved_by uuid
);
CREATE TABLE fr3_print_contracts (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),project_id uuid NOT NULL REFERENCES fr3_projects(id),
 provider_name text NOT NULL,format text NOT NULL,
 contract_spec jsonb NOT NULL,profile_sha256 text,approved_at timestamptz
);
CREATE TABLE fr3_output_jobs (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),publication_id uuid NOT NULL REFERENCES fr3_publications(id),
 contract_id uuid NOT NULL REFERENCES fr3_print_contracts(id),edition_snapshot_hash text NOT NULL,
 status text NOT NULL,preflight_json jsonb,output_sha256 text,output_storage_key text,
 created_at timestamptz NOT NULL DEFAULT now(),completed_at timestamptz
);
-- CRITICAL: add real tenant-aware RLS / ACL in actual migrations; this target blueprint
-- does NOT itself grant user access and is NOT production-ready SQL.
