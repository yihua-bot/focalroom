// K0 真实链路验收：隔离 PostgreSQL + 已播种知识数据 + 真实共享服务 + 桌面/原生 Rust 客户端。
// 必须由 apps/api/test/run.mjs 启动（临时 cluster、回环端口、私有预览目录），不进离线 CI 默认批次。
// 这一轮覆盖的是「已登录账户的知识调用能否区分无结果与异常」在真实服务上的行为；不调用模型、不上传照片。
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { createServer } from 'node:net';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';
import { before, after, test } from 'node:test';
import { Pool } from 'pg';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';

const root = fileURLToPath(new URL('../../..', import.meta.url));
assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL, '必须使用隔离 PostgreSQL runner');
assert.equal(new URL(process.env.FOCALROOM_TEST_DATABASE_URL).hostname, '127.0.0.1');

let pool, api, base;
let stages = 0;
const summary = { passed: false, runtime: 'real shared API + temporary PostgreSQL + shipped Rust client', flows: [] };

/** 显式运行 src-tauri 中被 @ignore 的真实会话用例；任何断言失败都让本套件失败。 */
async function cargo(rest) {
  const args = ['test', '--manifest-path', 'src-tauri/Cargo.toml', '--target-dir', 'src-tauri/target-v1', ...rest];
  const code = await new Promise((resolve, reject) => {
    const child = spawn('cargo', args, {
      cwd: root,
      stdio: 'inherit',
      env: { ...process.env, FOCALROOM_KNOWLEDGE_ACCEPTANCE_ORIGIN: base, FOCALROOM_DESKTOP_TEST_ORIGIN: base },
    });
    child.once('error', reject);
    child.once('exit', code => resolve(code ?? 1));
  });
  assert.equal(code, 0, `cargo ${args.join(' ')} 未通过`);
}

before(async () => {
  pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
  await migrate(pool);
  // 验收夹具：与 knowledge.mjs 同一批公开摄影史条目，只写入本轮临时库。
  await pool.query(`INSERT INTO fr_photographers(id, name_en, name_zh, birth_year, nationality, biography, movements, review_status)
    VALUES('b1000000-0000-4000-8000-000000000001', 'Stephen Shore', '斯蒂芬·肖尔', 1947, 'American',
      'American photographer known for images of banal scenes and objects in the United States.',
      ARRAY['New Color Photography','New Topographics'], 'verified')`);
  await pool.query(`INSERT INTO fr_photographers(id, name_en, name_zh, birth_year, nationality, biography, movements, review_status)
    VALUES('b1000000-0000-4000-8000-000000000002', 'Rinko Kawauchi', '川内伦子', 1972, 'Japanese',
      'Japanese photographer known for intimate images of everyday details in pale, luminous color.',
      ARRAY['Contemporary Japanese Photography'], 'verified')`);
  await pool.query(`INSERT INTO fr_photo_series(id, photographer_id, title_en, title_zh, intent, start_year, end_year, capture_format, review_status)
    VALUES('b1000000-0000-4000-8000-000000000003', 'b1000000-0000-4000-8000-000000000001', 'American Surfaces', '美国表面',
      'Daily snapshots of meals, motel rooms, and people encountered during road trips.', 1972, 1973, '35mm Rollei', 'verified'),
    ('b1000000-0000-4000-8000-000000000004', 'b1000000-0000-4000-8000-000000000002', 'Utatane', '假寐',
      'Quiet observations of daily life capturing fleeting moments between waking and sleeping.', 2001, null, '6x6 medium format', 'verified')`);
  await pool.query(`INSERT INTO fr_photobooks(id, photographer_id, title, publisher, publication_year, structure_notes, review_status)
    VALUES('b1000000-0000-4000-8000-000000000005', 'b1000000-0000-4000-8000-000000000001', 'Uncommon Places', 'Aperture', 1982,
      'Sequenced by road trip chronology; large-format plates with captions at the back.', 'verified')`);
  await mkdir(join(process.env.FOCALROOM_TEST_DIRECTORY, 'knowledge-desktop-previews'), { recursive: true });
  // 桌面客户端会逐字节比对服务返回的登录地址，因此服务必须绑定在自己的真实端口上启动。
  const port = await new Promise(resolve => {
    const probe = createServer();
    probe.listen(0, '127.0.0.1', () => {
      const free = probe.address().port;
      probe.close(() => resolve(free));
    });
  });
  base = `http://127.0.0.1:${port}`;
  api = await createApi({
    pool,
    origin: base,
    store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'knowledge-desktop-previews')),
  });
  await api.app.listen(port, '127.0.0.1');
  assert.equal(await api.app.getUrl(), base);
});

after(async () => {
  await api?.close();
  await pool?.end();
  // 三条真实链路各成功一次才算本轮验收通过，否则记录会固定印成未通过。
  summary.passed = stages === 3;
  console.log(JSON.stringify(summary));
});

test('K0 桌面真实会话经系统钥匙串读取已播种知识，并把无结果与异常分开', async () => {
  await cargo(['--lib', '--', '--ignored', '--nocapture', 'agent::knowledge::http_tests::real_pg_knowledge_session']);
  summary.flows.push('macos-keychain-session', 'seeded-knowledge-search', 'empty-versus-revoked');
  stages += 1;
});

test('K0 手机侧同一份核心经原生入口读取已播种知识，撤权报异常而不是空列表', async () => {
  // 与 apps/mobile-ios/scripts/build-agent-core.sh 相同的编译开关：不含桌面依赖的共享核心。
  await cargo(['--lib', '--no-default-features', '--features', 'native-agent', '--', '--ignored', '--nocapture',
    'native_agent::journey_tests::real_pg_knowledge_session']);
  summary.flows.push('native-ffi-session', 'native-tool-receipts', 'empty-versus-revoked');
  stages += 1;
});

test('K0 原生登录协议与钥匙串条目在真实服务上闭环', async () => {
  await cargo(['--test', 'cloud_login', '--', '--ignored', '--nocapture', 'native_cloud_keychain_roundtrip']);
  await cargo(['--test', 'cloud_login', '--', '--ignored', '--nocapture',
    'real_pg_api_and_keychain_complete_one_desktop_session']);
  summary.flows.push('native-desktop-login-protocol', 'system-keychain-roundtrip', 'single-use-authorization-code');
  stages += 1;
});
