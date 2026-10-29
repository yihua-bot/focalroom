-- K5 Focal Photography Intelligence 生产种子：已验证的策展内容。
-- 镜像 apps/api/test/knowledge.mjs 的测试夹具（同一批事实，两处同步维护）。
-- 覆盖：肖尔（American Surfaces 35mm vs Uncommon Places 8x10 区分）、Frank（The Americans/Guggenheim/Leica）、
--       川内伦子（Utatane 6x6）、Arbus（Rolleiflex 双反）、Eggleston（Guide/MoMA 首次彩色个展）。
-- 幂等：摄影师/系列按唯一键 UPSERT；作品/摄影书/断言/来源按 NOT EXISTS 去重，可重复执行。
-- 用法：迁移 0034 之后由管理员显式执行——
--   psql "$FOCALROOM_DATABASE_URL" -f "apps/api/seeds/photography-knowledge.sql"
-- 验证：node "apps/api/test/run.mjs" "apps/api/test/knowledge-seed.mjs"

BEGIN;

-- 摄影师（UNIQUE(name_en) → UPSERT 刷新内容）
INSERT INTO fr_photographers(name_en, name_zh, birth_year, nationality, biography, method_notes, movements, review_status)
VALUES('Stephen Shore', '斯蒂芬·肖尔', 1947, 'American',
  'American photographer known for images of banal scenes and objects in the United States.',
  'Pioneer of color photography and large-format urban landscape.',
  ARRAY['New Color Photography','New Topographics'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET
  name_zh = EXCLUDED.name_zh, birth_year = EXCLUDED.birth_year, nationality = EXCLUDED.nationality,
  biography = EXCLUDED.biography, method_notes = EXCLUDED.method_notes, movements = EXCLUDED.movements,
  review_status = EXCLUDED.review_status, updated_at = now();

INSERT INTO fr_photographers(name_en, name_zh, birth_year, nationality, movements, review_status)
VALUES('William Eggleston', '威廉·埃格尔斯顿', 1939, 'American', ARRAY['New Color Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET
  name_zh = EXCLUDED.name_zh, birth_year = EXCLUDED.birth_year, nationality = EXCLUDED.nationality,
  movements = EXCLUDED.movements, review_status = EXCLUDED.review_status, updated_at = now();

INSERT INTO fr_photographers(name_en, name_zh, birth_year, nationality, biography, movements, review_status)
VALUES('Robert Frank', '罗伯特·弗兰克', 1924, 'Swiss-American',
  'Swiss-American photographer and filmmaker; his book The Americans changed the direction of documentary photography.',
  ARRAY['Documentary','Snapshot Aesthetic'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET
  name_zh = EXCLUDED.name_zh, birth_year = EXCLUDED.birth_year, nationality = EXCLUDED.nationality,
  biography = EXCLUDED.biography, movements = EXCLUDED.movements, review_status = EXCLUDED.review_status, updated_at = now();

INSERT INTO fr_photographers(name_en, name_zh, birth_year, nationality, biography, movements, review_status)
VALUES('Rinko Kawauchi', '川内伦子', 1972, 'Japanese',
  'Japanese photographer known for intimate images of everyday details in pale, luminous color.',
  ARRAY['Contemporary Japanese Photography'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET
  name_zh = EXCLUDED.name_zh, birth_year = EXCLUDED.birth_year, nationality = EXCLUDED.nationality,
  biography = EXCLUDED.biography, movements = EXCLUDED.movements, review_status = EXCLUDED.review_status, updated_at = now();

INSERT INTO fr_photographers(name_en, name_zh, birth_year, death_year, nationality, movements, review_status)
VALUES('Diane Arbus', '黛安·阿勃丝', 1923, 1971, 'American', ARRAY['New Documents'], 'verified')
ON CONFLICT (name_en) DO UPDATE SET
  name_zh = EXCLUDED.name_zh, birth_year = EXCLUDED.birth_year, death_year = EXCLUDED.death_year,
  nationality = EXCLUDED.nationality, movements = EXCLUDED.movements, review_status = EXCLUDED.review_status, updated_at = now();

-- 系列（UNIQUE(photographer_id, title_en) → UPSERT）
INSERT INTO fr_photo_series(photographer_id, title_en, title_zh, intent, start_year, end_year, capture_format, review_status)
SELECT p.id, 'American Surfaces', '美国表面',
  'Daily snapshots of meals, motel rooms, and people encountered during road trips.', 1972, 1973, '35mm Rollei', 'verified'
FROM fr_photographers p WHERE p.name_en = 'Stephen Shore'
ON CONFLICT (photographer_id, title_en) DO UPDATE SET
  title_zh = EXCLUDED.title_zh, intent = EXCLUDED.intent, start_year = EXCLUDED.start_year,
  end_year = EXCLUDED.end_year, capture_format = EXCLUDED.capture_format, review_status = EXCLUDED.review_status;

INSERT INTO fr_photo_series(photographer_id, title_en, title_zh, intent, start_year, end_year, capture_format, review_status)
SELECT p.id, 'Uncommon Places', '不寻常之地',
  'Large-format photographs of American urban and suburban landscapes.', 1974, 1983, '8x10 large format', 'verified'
FROM fr_photographers p WHERE p.name_en = 'Stephen Shore'
ON CONFLICT (photographer_id, title_en) DO UPDATE SET
  title_zh = EXCLUDED.title_zh, intent = EXCLUDED.intent, start_year = EXCLUDED.start_year,
  end_year = EXCLUDED.end_year, capture_format = EXCLUDED.capture_format, review_status = EXCLUDED.review_status;

INSERT INTO fr_photo_series(photographer_id, title_en, title_zh, intent, start_year, end_year, capture_format, review_status)
SELECT p.id, 'The Americans', '美国人',
  'Cross-country road trip photographing everyday American life with raw, off-kilter framing.', 1955, 1956, '35mm Leica', 'verified'
FROM fr_photographers p WHERE p.name_en = 'Robert Frank'
ON CONFLICT (photographer_id, title_en) DO UPDATE SET
  title_zh = EXCLUDED.title_zh, intent = EXCLUDED.intent, start_year = EXCLUDED.start_year,
  end_year = EXCLUDED.end_year, capture_format = EXCLUDED.capture_format, review_status = EXCLUDED.review_status;

INSERT INTO fr_photo_series(photographer_id, title_en, title_zh, intent, start_year, capture_format, review_status)
SELECT p.id, 'Utatane', '假寐',
  'Quiet observations of daily life capturing fleeting moments between waking and sleeping.', 2001, '6x6 medium format', 'verified'
FROM fr_photographers p WHERE p.name_en = 'Rinko Kawauchi'
ON CONFLICT (photographer_id, title_en) DO UPDATE SET
  title_zh = EXCLUDED.title_zh, intent = EXCLUDED.intent, start_year = EXCLUDED.start_year,
  capture_format = EXCLUDED.capture_format, review_status = EXCLUDED.review_status;

-- 作品（无唯一约束 → NOT EXISTS 去重）
INSERT INTO fr_artworks(photographer_id, series_id, title_en, capture_date, capture_format, subjects, visual_concepts, review_status)
SELECT p.id, s.id, 'U.S. 97, South of Klamath Falls, Oregon', '1973-07', '35mm Rollei',
  ARRAY['road','landscape','American West'], ARRAY['snapshot aesthetic','banal scene'], 'verified'
FROM fr_photographers p
JOIN fr_photo_series s ON s.photographer_id = p.id AND s.title_en = 'American Surfaces'
WHERE p.name_en = 'Stephen Shore'
  AND NOT EXISTS (
    SELECT 1 FROM fr_artworks a
    WHERE a.photographer_id = p.id AND a.title_en = 'U.S. 97, South of Klamath Falls, Oregon'
  );

-- 摄影书（无唯一约束 → NOT EXISTS 去重）
INSERT INTO fr_photobooks(photographer_id, title, publisher, publication_year, structure_notes, review_status)
SELECT p.id, 'Uncommon Places', 'Aperture', 1982,
  'Sequenced by road trip chronology; large-format plates with captions at the back.', 'verified'
FROM fr_photographers p WHERE p.name_en = 'Stephen Shore'
  AND NOT EXISTS (
    SELECT 1 FROM fr_photobooks b
    WHERE b.photographer_id = p.id AND b.title = 'Uncommon Places'
  );

INSERT INTO fr_photobooks(photographer_id, title, publisher, publication_year, structure_notes, review_status)
SELECT p.id, 'William Eggleston''s Guide', 'MoMA', 1976,
  'First solo exhibition of color photographs at MoMA; dye-transfer prints. Plates sequ without captions, paired with a separate text section.', 'verified'
FROM fr_photographers p WHERE p.name_en = 'William Eggleston'
  AND NOT EXISTS (
    SELECT 1 FROM fr_photobooks b
    WHERE b.photographer_id = p.id AND b.title = 'William Eggleston''s Guide'
  );

-- 事实断言（NOT EXISTS on entity_type + entity_id + claim_text）
INSERT INTO fr_knowledge_claims(entity_type, entity_id, claim_type, claim_text, sources, review_status)
SELECT 'series', s.id, 'historical_fact',
  'American Surfaces was photographed with a Rollei 35mm camera during 1972-1973 road trips.',
  '[{"url":"https://www.moma.org/collection/","institution":"MoMA","verified_at":"2026-10-29"}]', 'verified'
FROM fr_photo_series s JOIN fr_photographers p ON p.id = s.photographer_id
WHERE p.name_en = 'Stephen Shore' AND s.title_en = 'American Surfaces'
  AND NOT EXISTS (
    SELECT 1 FROM fr_knowledge_claims c
    WHERE c.entity_type = 'series' AND c.entity_id = s.id
      AND c.claim_text = 'American Surfaces was photographed with a Rollei 35mm camera during 1972-1973 road trips.'
  );

INSERT INTO fr_knowledge_claims(entity_type, entity_id, claim_type, claim_text, sources, review_status)
SELECT 'series', s.id, 'historical_fact',
  'Uncommon Places was photographed primarily with an 8x10 large format camera.',
  '[{"url":"https://www.moma.org/collection/","institution":"MoMA","verified_at":"2026-10-29"}]', 'verified'
FROM fr_photo_series s JOIN fr_photographers p ON p.id = s.photographer_id
WHERE p.name_en = 'Stephen Shore' AND s.title_en = 'Uncommon Places'
  AND NOT EXISTS (
    SELECT 1 FROM fr_knowledge_claims c
    WHERE c.entity_type = 'series' AND c.entity_id = s.id
      AND c.claim_text = 'Uncommon Places was photographed primarily with an 8x10 large format camera.'
  );

INSERT INTO fr_knowledge_claims(entity_type, entity_id, claim_type, claim_text, sources, review_status)
SELECT 'series', s.id, 'historical_fact',
  'The Americans was photographed during 1955-1956 road trips funded by a Guggenheim Fellowship, primarily using a 35mm Leica camera.',
  '[{"url":"https://www.npg.si.edu","institution":"Smithsonian NPG","verified_at":"2026-10-29"}]', 'verified'
FROM fr_photo_series s JOIN fr_photographers p ON p.id = s.photographer_id
WHERE p.name_en = 'Robert Frank' AND s.title_en = 'The Americans'
  AND NOT EXISTS (
    SELECT 1 FROM fr_knowledge_claims c
    WHERE c.entity_type = 'series' AND c.entity_id = s.id
      AND c.claim_text = 'The Americans was photographed during 1955-1956 road trips funded by a Guggenheim Fellowship, primarily using a 35mm Leica camera.'
  );

INSERT INTO fr_knowledge_claims(entity_type, entity_id, claim_type, claim_text, sources, review_status)
SELECT 'photographer', p.id, 'historical_fact',
  'Diane Arbus used a square-format Rolleiflex twin-lens reflex camera for her iconic portraits.',
  '[{"url":"https://www.metmuseum.org","institution":"The Met","verified_at":"2026-10-29"}]', 'verified'
FROM fr_photographers p WHERE p.name_en = 'Diane Arbus'
  AND NOT EXISTS (
    SELECT 1 FROM fr_knowledge_claims c
    WHERE c.entity_type = 'photographer' AND c.entity_id = p.id
      AND c.claim_text = 'Diane Arbus used a square-format Rolleiflex twin-lens reflex camera for her iconic portraits.'
  );

INSERT INTO fr_knowledge_claims(entity_type, entity_id, claim_type, claim_text, sources, review_status)
SELECT 'photobook', b.id, 'historical_fact',
  'William Eggleston''s Guide (1976) was the first one-person exhibition of color photography at the Museum of Modern Art.',
  '[{"url":"https://www.moma.org","institution":"MoMA","verified_at":"2026-10-29"}]', 'verified'
FROM fr_photobooks b JOIN fr_photographers p ON p.id = b.photographer_id
WHERE p.name_en = 'William Eggleston' AND b.title = 'William Eggleston''s Guide'
  AND NOT EXISTS (
    SELECT 1 FROM fr_knowledge_claims c
    WHERE c.entity_type = 'photobook' AND c.entity_id = b.id
      AND c.claim_text = 'William Eggleston''s Guide (1976) was the first one-person exhibition of color photography at the Museum of Modern Art.'
  );

-- 知识来源（NOT EXISTS on entity_type + entity_id + url + institution）
INSERT INTO fr_knowledge_sources(entity_type, entity_id, url, institution, verified_at)
SELECT 'series', s.id, 'https://www.moma.org/collection/', 'MoMA', '2026-10-29'
FROM fr_photo_series s JOIN fr_photographers p ON p.id = s.photographer_id
WHERE p.name_en = 'Stephen Shore' AND s.title_en IN ('American Surfaces', 'Uncommon Places')
  AND NOT EXISTS (
    SELECT 1 FROM fr_knowledge_sources src
    WHERE src.entity_type = 'series' AND src.entity_id = s.id
      AND src.url = 'https://www.moma.org/collection/' AND src.institution = 'MoMA'
  );

INSERT INTO fr_knowledge_sources(entity_type, entity_id, url, institution, verified_at)
SELECT 'series', s.id, 'https://www.npg.si.edu', 'Smithsonian NPG', '2026-10-29'
FROM fr_photo_series s JOIN fr_photographers p ON p.id = s.photographer_id
WHERE p.name_en = 'Robert Frank' AND s.title_en = 'The Americans'
  AND NOT EXISTS (
    SELECT 1 FROM fr_knowledge_sources src
    WHERE src.entity_type = 'series' AND src.entity_id = s.id
      AND src.url = 'https://www.npg.si.edu' AND src.institution = 'Smithsonian NPG'
  );

INSERT INTO fr_knowledge_sources(entity_type, entity_id, url, institution, verified_at)
SELECT 'photographer', p.id, 'https://www.metmuseum.org', 'The Met', '2026-10-29'
FROM fr_photographers p WHERE p.name_en = 'Diane Arbus'
  AND NOT EXISTS (
    SELECT 1 FROM fr_knowledge_sources src
    WHERE src.entity_type = 'photographer' AND src.entity_id = p.id
      AND src.url = 'https://www.metmuseum.org' AND src.institution = 'The Met'
  );

INSERT INTO fr_knowledge_sources(entity_type, entity_id, url, institution, verified_at)
SELECT 'photobook', b.id, 'https://www.moma.org', 'MoMA', '2026-10-29'
FROM fr_photobooks b JOIN fr_photographers p ON p.id = b.photographer_id
WHERE p.name_en = 'William Eggleston' AND b.title = 'William Eggleston''s Guide'
  AND NOT EXISTS (
    SELECT 1 FROM fr_knowledge_sources src
    WHERE src.entity_type = 'photobook' AND src.entity_id = b.id
      AND src.url = 'https://www.moma.org' AND src.institution = 'MoMA'
  );

COMMIT;
