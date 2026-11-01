import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { Pool } from 'pg';
import { migrate } from '../src/database.ts';
import { SummaryMemoryService } from '../src/summary-memory.ts';
import { KnowledgeVectorService } from '../src/knowledge-vector.ts';

let pool, summary, vector;

before(async () => {
  const url = process.env.FOCALROOM_TEST_DATABASE_URL ?? 'postgresql://focalroom_migrator:focalroom@localhost:5432/focalroom_test';
  pool = new Pool({ connectionString: url });
  await migrate(pool);
  summary = new SummaryMemoryService(pool);
  vector = new KnowledgeVectorService(pool);
});

after(async () => { await pool?.end(); });

test('摘要保存与检索（同项目优先）', async () => {
  await summary.save({ userId: 'u1', projectId: 'p1', summary: '选了20张风景照', kind: 'selection', preferences: ['暖色调'] });
  await summary.save({ userId: 'u1', projectId: 'p2', summary: '编辑了叙事画布', kind: 'narrative' });
  await summary.save({ userId: 'u2', projectId: 'p1', summary: '另一个用户', kind: 'general' });
  const results = await summary.retrieve('u1', 'p1', 5);
  assert.ok(results.length >= 2);
  assert.strictEqual(results[0].summary, '选了20张风景照'); // 同项目排前
});

test('摘要上下文块包含偏好', async () => {
  const block = await summary.contextBlock('u1', 'p1');
  assert.ok(block.includes('选了20张风景照'));
  assert.ok(block.includes('暖色调'));
});

test('向量索引构建与语义搜索', async () => {
  // 插入测试数据（知识表可能为空）
  await pool.query("INSERT INTO fr_photographers(name_en, name_zh, biography) VALUES('Test Photographer', '测试摄影师', 'American color photography pioneer') ON CONFLICT (name_en) DO NOTHING");
  await pool.query("INSERT INTO fr_knowledge_claims(entity_type, entity_id, claim_type, claim_text) SELECT 'photographer', id, 'historical_fact', 'Used 35mm Rolleiflex camera for street photography' FROM fr_photographers WHERE name_en = 'Test Photographer' LIMIT 1");
  const count = await vector.buildIndex();
  assert.ok(count >= 2, `至少 2 个实体（实际 ${count}）`);
  // 搜摄影师名（英文）
  const results = await vector.search('Test Photographer American color', 3);
  assert.ok(results.length > 0, '应有搜索结果');
  // 搜概念（中文）
  const results2 = await vector.search('摄影 色彩', 3);
  assert.ok(results2 !== null);
});
