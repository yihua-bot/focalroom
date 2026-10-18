// M4 运维指标回归：/metrics 端点、请求计数/延迟桶、DB 派生指标与令牌保护。
import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { Pool } from 'pg';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';

let api, pool, base;
const json = { 'content-type': 'application/json' };

before(async () => {
  assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, '必须由隔离 PostgreSQL runner 启动');
  pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  await migrate(pool);
  api = await createApi({ pool, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'previews-metrics')), origin: 'http://127.0.0.1:1424' });
  await api.app.listen(0, '127.0.0.1');
  base = await api.app.getUrl();
});

after(async () => { if (api) await api.close(); if (pool) await pool.end(); });

test('M4 /metrics 暴露请求计数、延迟桶与 DB 派生指标', async () => {
  // 制造若干请求：健康检查 ×2、404 ×1。
  await fetch(`${base}/api/v2/health`);
  await fetch(`${base}/api/v2/health`);
  await fetch(`${base}/api/v2/no-such-endpoint`);
  const response = await fetch(`${base}/api/v2/metrics`);
  assert.equal(response.status, 200);
  const text = await response.text();
  // 请求计数按路由与状态聚合。
  assert.match(text, /focalroom_api_requests_total\{route="\/health",method="GET",status="200"\} 2/);
  assert.match(text, /focalroom_api_requests_total\{route="\/no-such-endpoint",method="GET",status="404"\} 1/);
  // 延迟直方图含桶边界与累计计数。
  assert.match(text, /focalroom_api_request_duration_ms_bucket\{route="\/health",method="GET",le="\+Inf"\} 2/);
  assert.match(text, /focalroom_api_request_duration_ms_count\{route="\/health",method="GET"\} 2/);
  // DB 派生指标：任务与费用（空库为零）与队列深度。
  assert.match(text, /focalroom_agent_queue_depth 0/);
  assert.match(text, /focalroom_agent_cost_minor_usd_total 0/);
  assert.match(text, /focalroom_metrics_uptime_seconds \d+/);
  // metrics 自身不计入业务路由计数（端点短路）。
  assert.doesNotMatch(text, /route="\/metrics".*status="200"/);
});

test('M4 UUID 路由模板归一，防止标签基数爆炸', async () => {
  const uuid = '1b6e28a4-0f9c-4d0f-9a01-0d21d6b0aa71';
  await fetch(`${base}/api/v2/projects/${uuid}/snapshot`, { headers: json });
  const text = await (await fetch(`${base}/api/v2/metrics`)).text();
  assert.match(text, /focalroom_api_requests_total\{route="\/projects\/:id\/snapshot",method="GET",status="401"\} 1/);
});

test('M4 FOCALROOM_METRICS_TOKEN 未设置时开放，设置后需令牌', async () => {
  // 默认开放（上一测试已验证）。设置后拒绝与放行。
  process.env.FOCALROOM_METRICS_TOKEN = 'secret-token';
  try {
    const denied = await fetch(`${base}/api/v2/metrics`);
    assert.equal(denied.status, 403);
    const wrongQuery = await fetch(`${base}/api/v2/metrics?token=wrong`);
    assert.equal(wrongQuery.status, 403);
    const allowedQuery = await fetch(`${base}/api/v2/metrics?token=secret-token`);
    assert.equal(allowedQuery.status, 200);
    const allowedBearer = await fetch(`${base}/api/v2/metrics`, { headers: { authorization: 'Bearer secret-token' } });
    assert.equal(allowedBearer.status, 200);
  } finally {
    delete process.env.FOCALROOM_METRICS_TOKEN;
  }
});
