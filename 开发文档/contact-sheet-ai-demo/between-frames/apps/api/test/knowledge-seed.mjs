// K5 摄影知识生产种子验证：幂等（重复执行不重复插入）、实体计数、搜索关键数据。
// 种子文件 apps/api/seeds/photography-knowledge.sql 与 knowledge.mjs 测试夹具同源同步。
import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { Pool } from 'pg';
import { migrate } from '../src/database.ts';

let pool;

async function counts() {
  const tables = ['fr_photographers', 'fr_photo_series', 'fr_artworks', 'fr_photobooks', 'fr_knowledge_claims', 'fr_knowledge_sources'];
  const result = {};
  for (const table of tables) {
    result[table] = (await pool.query(`SELECT count(*)::int AS n FROM ${table}`)).rows[0].n;
  }
  return result;
}

before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, '必须使用隔离 PostgreSQL runner');
  pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  await migrate(pool);
});

after(async () => { await pool?.end(); });

test('K5 seed loads verified curatorial content', async () => {
  const seed = await readFile(new URL('../seeds/photography-knowledge.sql', import.meta.url), 'utf8');
  await pool.query(seed);

  const first = await counts();
  assert.equal(first.fr_photographers, 5, '五位摄影师');
  assert.equal(first.fr_photo_series, 4, '四个系列');
  assert.equal(first.fr_artworks, 1, '一件作品');
  assert.equal(first.fr_photobooks, 2, '两本摄影书');
  assert.equal(first.fr_knowledge_claims, 5, '五条事实断言');
  assert.equal(first.fr_knowledge_sources, 5, '五条来源');
});

test('K5 seed is idempotent', async () => {
  const seed = await readFile(new URL('../seeds/photography-knowledge.sql', import.meta.url), 'utf8');
  const beforeCounts = await counts();
  await pool.query(seed);
  const afterCounts = await counts();
  assert.deepEqual(afterCounts, beforeCounts, '重复执行不得重复插入');
});

test('K5 seed keeps the Shore 35mm vs 8x10 distinction searchable', async () => {
  const formats = (await pool.query(`
    SELECT s.title_en, s.capture_format
    FROM fr_photo_series s JOIN fr_photographers p ON p.id = s.photographer_id
    WHERE p.name_en = 'Stephen Shore'
  `)).rows;
  const surfaces = formats.find(f => f.title_en === 'American Surfaces');
  const uncommon = formats.find(f => f.title_en === 'Uncommon Places');
  assert.equal(surfaces.capture_format, '35mm Rollei');
  assert.equal(uncommon.capture_format, '8x10 large format');

  const topographics = (await pool.query(
    `SELECT name_en FROM fr_photographers WHERE 'New Topographics' = ANY(movements)`
  )).rows;
  assert.equal(topographics.length, 1);
  assert.equal(topographics[0].name_en, 'Stephen Shore');
});

test('K5 seed claims carry museum sources', async () => {
  const claims = (await pool.query(`
    SELECT c.claim_text, c.sources
    FROM fr_knowledge_claims c
    WHERE c.entity_type = 'photographer' AND c.entity_id = (SELECT id FROM fr_photographers WHERE name_en = 'Diane Arbus')
  `)).rows;
  assert.equal(claims.length, 1);
  assert.ok(claims[0].claim_text.includes('Rolleiflex'));
  const sources = claims[0].sources;
  assert.ok(Array.isArray(sources) && sources.length === 1, '断言内嵌来源 JSONB');
  assert.equal(sources[0].institution, 'The Met');
});
