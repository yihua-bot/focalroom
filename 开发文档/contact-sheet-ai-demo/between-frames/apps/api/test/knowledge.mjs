// K0/K1 摄影知识公共检索测试：种子数据、跨实体搜索、实体详情、审校状态。
// 知识库为只读公共层——与项目研究参考（fr_research_references）和审美记忆权限隔离。
import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { Pool } from 'pg';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';

let pool, api, base, owner, viewer;

async function call(path, method = 'GET', account = owner, body, status = 200) {
  const response = await fetch(base + '/api/v2' + path, {
    method,
    headers: { 'content-type': 'application/json', ...(account ? { authorization: 'Bearer ' + account.token } : {}) },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  const value = await response.json();
  assert.equal(response.status, status, JSON.stringify(value));
  return value;
}

before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, '必须使用隔离 PostgreSQL runner');
  pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  await migrate(pool);
  api = await createApi({ pool, origin: 'http://127.0.0.1:1424',
    store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'knowledge-previews')) });
  await api.app.listen(0, '127.0.0.1');
  base = await api.app.getUrl();
  const register = email => call('/auth/register', 'POST', null, { email, password: 'knowledge-test-password-123', display_name: email });
  owner = await register('knowledge-owner@fr.test');
  viewer = await register('knowledge-viewer@fr.test');

  // K1 种子数据：斯蒂芬·肖尔——验证 35mm Rollei 与大画幅的正确区分。
  // 来源：MoMA 资料对 American Surfaces（35mm Rollei）与 Uncommon Places（大画幅）有明确区分。
  await pool.query(`INSERT INTO fr_photographers(id, name_en, name_zh, birth_year, nationality, biography, method_notes, movements, review_status)
    VALUES('a1000000-0000-4000-8000-000000000001', 'Stephen Shore', '斯蒂芬·肖尔', 1947, 'American',
      'American photographer known for images of banal scenes and objects in the United States.',
      'Pioneer of color photography and large-format urban landscape.', ARRAY['New Color Photography','New Topographics'], 'verified')`);
  await pool.query(`INSERT INTO fr_photo_series(id, photographer_id, title_en, title_zh, intent, start_year, end_year, capture_format, review_status)
    VALUES('a1000000-0000-4000-8000-000000000002', 'a1000000-0000-4000-8000-000000000001', 'American Surfaces', '美国表面',
      'Daily snapshots of meals, motel rooms, and people encountered during road trips.', 1972, 1973, '35mm Rollei', 'verified')`);
  await pool.query(`INSERT INTO fr_photo_series(id, photographer_id, title_en, title_zh, intent, start_year, end_year, capture_format, review_status)
    VALUES('a1000000-0000-4000-8000-000000000003', 'a1000000-0000-4000-8000-000000000001', 'Uncommon Places', '不寻常之地',
      'Large-format photographs of American urban and suburban landscapes.', 1974, 1983, '8x10 large format', 'verified')`);
  await pool.query(`INSERT INTO fr_artworks(id, photographer_id, series_id, title_en, capture_date, capture_format, subjects, visual_concepts, review_status)
    VALUES('a1000000-0000-4000-8000-000000000004', 'a1000000-0000-4000-8000-000000000001', 'a1000000-0000-4000-8000-000000000002',
      'U.S. 97, South of Klamath Falls, Oregon', '1973-07', '35mm Rollei',
      ARRAY['road','landscape','American West'], ARRAY['snapshot aesthetic','banal scene'], 'verified')`);
  await pool.query(`INSERT INTO fr_photobooks(id, photographer_id, title, publisher, publication_year, structure_notes, review_status)
    VALUES('a1000000-0000-4000-8000-000000000005', 'a1000000-0000-4000-8000-000000000001', 'Uncommon Places', 'Aperture', 1982,
      'Sequenced by road trip chronology; large-format plates with captions at the back.', 'verified')`);
  // 事实断言：American Surfaces 的拍摄格式（historical_fact，有来源）。
  await pool.query(`INSERT INTO fr_knowledge_claims(entity_type, entity_id, claim_type, claim_text, sources, review_status)
    VALUES('series', 'a1000000-0000-4000-8000-000000000002', 'historical_fact',
      'American Surfaces was photographed with a Rollei 35mm camera during 1972-1973 road trips.',
      '[{"url":"https://www.moma.org/collection/","institution":"MoMA","verified_at":"2026-10-29"}]', 'verified')`);
  await pool.query(`INSERT INTO fr_knowledge_claims(entity_type, entity_id, claim_type, claim_text, sources, review_status)
    VALUES('series', 'a1000000-0000-4000-8000-000000000003', 'historical_fact',
      'Uncommon Places was photographed primarily with an 8x10 large format camera.',
      '[{"url":"https://www.moma.org/collection/","institution":"MoMA","verified_at":"2026-10-29"}]', 'verified')`);

  // 第二位摄影师：威廉·埃格尔斯顿（movement 过滤测试）。
  await pool.query(`INSERT INTO fr_photographers(id, name_en, name_zh, birth_year, nationality, movements, review_status)
    VALUES('a1000000-0000-4000-8000-000000000006', 'William Eggleston', '威廉·埃格尔斯顿', 1939, 'American',
      ARRAY['New Color Photography'], 'verified')`);
});

after(async () => { await api?.close(); await pool?.end(); });

test('K0 knowledge search finds photographers across entity types', async () => {
  const results = await call('/knowledge/search?q=Stephen Shore');
  assert.ok(results.length >= 1, '应找到斯蒂芬·肖尔');
  const photographer = results.find(r => r.kind === 'photographer');
  assert.ok(photographer, '结果中应有摄影师');
  assert.equal(photographer.title, 'Stephen Shore');
  assert.equal(photographer.reviewStatus, 'verified');
});

test('K0 knowledge search finds artworks by visual concepts', async () => {
  const results = await call('/knowledge/search?q=snapshot aesthetic');
  const artwork = results.find(r => r.kind === 'artwork');
  assert.ok(artwork, '应通过视觉概念找到作品');
  assert.equal(artwork.title, 'U.S. 97, South of Klamath Falls, Oregon');
  assert.equal(artwork.photographer, 'Stephen Shore');
});

test('K0 knowledge search finds photobooks', async () => {
  const results = await call('/knowledge/search?q=Uncommon Places');
  const book = results.find(r => r.kind === 'photobook');
  assert.ok(book, '应找到摄影书');
  assert.equal(book.title, 'Uncommon Places');
  const series = results.find(r => r.kind === 'series');
  assert.ok(series, '也应找到同名系列');
});

test('K0 knowledge search filters by movement', async () => {
  const results = await call(`/knowledge/search?q=New Topographics&movement=${encodeURIComponent('New Topographics')}`);
  const photographer = results.find(r => r.kind === 'photographer');
  assert.ok(photographer, 'New Topographics 流派应找到肖尔');
  assert.equal(photographer.title, 'Stephen Shore');
});

test('K0 knowledge search filters by entity kind', async () => {
  const onlyBooks = await call(`/knowledge/search?q=${encodeURIComponent('Stephen Shore')}&kind=photobook`);
  assert.ok(onlyBooks.every(r => r.kind === 'photobook'), 'kind 过滤后只应有摄影书');
  const onlyPhotographers = await call(`/knowledge/search?q=${encodeURIComponent('color')}&kind=photographer`);
  assert.ok(onlyPhotographers.every(r => r.kind === 'photographer'), 'kind 过滤后只应有摄影师');
});

test('K0 photographer detail includes series, claims, and sources', async () => {
  const detail = await call('/knowledge/photographers/a1000000-0000-4000-8000-000000000001');
  assert.equal(detail.name_en, 'Stephen Shore');
  assert.equal(detail.series.length, 2, '应有两个系列');
  const surfaces = detail.series.find(s => s.title_en === 'American Surfaces');
  const uncommon = detail.series.find(s => s.title_en === 'Uncommon Places');
  assert.equal(surfaces.capture_format, '35mm Rollei');
  assert.equal(uncommon.capture_format, '8x10 large format');
  assert.ok(detail.claims.length >= 2, '应有事实断言');
  const formats = detail.claims.map(c => c.claim_text).join(' ');
  assert.ok(formats.includes('35mm') && formats.includes('8x10'), '断言应包含两种格式的区分');
});

test('K0 knowledge is read-only for all users', async () => {
  // 任何用户（含未认证）都只能 GET，没有 POST/PATCH/DELETE 端点。
  const anonymous = await fetch(base + '/api/v2/knowledge/search?q=Shore');
  assert.equal(anonymous.status, 401, '未认证不可访问');
  const response = await fetch(base + '/api/v2/knowledge/search?q=Shore', { method: 'POST' });
  assert.equal(response.status, 404, '没有写端点');
});

test('K0 knowledge search returns empty for irrelevant queries', async () => {
  const results = await call(`/knowledge/search?q=${encodeURIComponent('量子力学')}`);
  assert.equal(results.length, 0);
});
