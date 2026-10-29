// 仅在 run.mjs 的全新临时数据库和未安装 App 的独立 iPhone 模拟器中验证。
import assert from 'node:assert/strict';
import { randomUUID, createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, writeFile, cp } from 'node:fs/promises';
import { join, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Pool } from 'pg';
import sharp from 'sharp';
import { createApi } from '../../api/src/server.ts';
import { migrate } from '../../api/src/database.ts';
import { LocalPreviewStore } from '../../api/src/storage.ts';

const directory = process.env.FOCALROOM_TEST_DIRECTORY;
assert.ok(directory, '必须通过 apps/api/test/run.mjs 运行');
assert.equal(new URL(process.env.FOCALROOM_TEST_DATABASE_URL).hostname, '127.0.0.1');
const device = process.env.FOCALROOM_IOS_SIMULATOR_ID;
assert.ok(device, '请指定本轮新建的独立 FocalRoom 模拟器');
const devices = JSON.parse(execFileSync('xcrun', ['simctl', 'list', 'devices', '-j'], { encoding: 'utf8' }));
assert.ok(Object.values(devices.devices).flat().some(item => item.udid === device && item.name.startsWith('FocalRoom Project Picker ')));
let installed = false;
try {
  execFileSync('xcrun', ['simctl', 'get_app_container', device, 'studio.focalroom.ios.development', 'data'], { stdio: 'ignore' });
  installed = true;
} catch { /* 首次安装 */ }
assert.equal(installed, false, '拒绝覆盖已有应用；请使用新的独立模拟器');
const root = fileURLToPath(new URL('..', import.meta.url));
const evidenceDirectory = join(directory, 'picker-evidence');
await mkdir(evidenceDirectory, { recursive: true });
const pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
let api;
const run = (args, env = {}) => new Promise((resolve, reject) => {
  const child = spawn('xcodebuild', args, { cwd: root, stdio: 'inherit', env: { ...process.env, ...env } });
  child.once('error', reject);
  child.once('exit', code => code === 0 ? resolve() : reject(new Error(`xcodebuild exited ${code}`)));
});
try {
  await migrate(pool);
  api = await createApi({ pool, store: new LocalPreviewStore(join(directory, 'picker-previews')), origin: 'http://127.0.0.1:1424', hostedAi: { key: '', initialTokens: 0 } });
  await api.app.listen(0, '127.0.0.1');
  const base = await api.app.getUrl();
  const call = async (path, token, body) => {
    const response = await fetch(`${base}/api/v2${path}`, {
      method: body === undefined ? 'GET' : 'POST',
      headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    const result = await response.json();
    assert.equal(response.status, 200, JSON.stringify(result));
    return result;
  };
  const owner = await call('/auth/register', null, { email: 'picker@fr.test', password: 'iphone-local-test-2026', display_name: '项目切换测试' });
  const workspaceID = (await call('/workspaces', owner.token))[0].id;
  const projectIDs = [];
  for (const label of ['甲', '乙']) {
    const bytes = await sharp({ create: { width: 64, height: 64, channels: 3, background: label === '甲' ? '#475f68' : '#89746a' } }).jpeg().toBuffer();
    const photo = { id: randomUUID(), filename: `${label}.jpg`, source_sha256: createHash('sha256').update(bytes).digest('hex'), selection: 'UNDECIDED' };
    const manifest = {
      workspace_id: workspaceID, local_project_id: randomUUID(), promotion_id: randomUUID(), title: `切换项目${label}`, from_revision: 0,
      snapshot: { photos: [photo], sequences: [{ id: randomUUID(), title: '主序列', items: [{ id: randomUUID(), photo_id: photo.id }] }] },
      media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false },
    };
    const promotion = await call('/private-promotions', owner.token, manifest);
    const id = promotion.project_id;
    const upload = await fetch(`${base}/api/v2/projects/${id}/previews/${photo.id}`, { method: 'PUT', headers: { Authorization: `Bearer ${owner.token}`, 'Content-Type': 'image/jpeg' }, body: bytes });
    assert.equal(upload.status, 200);
    await call(`/projects/${id}/finalize`, owner.token, { manifest_digest: promotion.manifest_digest });
    await call('/boards', owner.token, { projectId: id, title: `${label}项目内容` });
    const commandID = randomUUID();
    await call(`/projects/${id}/commands`, owner.token, {
      version: '2.0', command_id: commandID, idempotency_key: commandID, workspace_id: workspaceID, project_id: id,
      expected_revision: 0, kind: 'comment.add', payload: { body: `${label}项目内容`, anchor: { type: 'photo', photo_id: photo.id, x: .5, y: .5 } },
    });
    projectIDs.push(id);
  }
  const beforeProjects = (await pool.query('SELECT id,revision,snapshot FROM fr_projects ORDER BY id')).rows;
  const beforeBoards = (await pool.query('SELECT id,revision FROM fr_boards ORDER BY id')).rows;
  const args = ['-project', 'FocalRoom.xcodeproj', '-scheme', 'FocalRoom', '-configuration', 'Debug', '-destination', `id=${device}`,
    '-derivedDataPath', process.env.FOCALROOM_IOS_PICKER_DERIVED_DATA ?? join(directory, 'Xcode'), '-parallel-testing-enabled', 'NO', 'CODE_SIGN_IDENTITY=-'];
  await run([...args, 'build-for-testing']);
  await run([...args, '-only-testing:FocalRoomUITests/FocalRoomUITests/testProjectSelectionInAITabCanvasAndMessages',
    '-resultBundlePath', join(evidenceDirectory, 'ProjectPicker.xcresult'), 'test-without-building'], {
    TEST_RUNNER_FOCALROOM_IOS_TEST_BASE_URL: base,
    TEST_RUNNER_FOCALROOM_IOS_PICKER_FIRST_ID: projectIDs[0],
    TEST_RUNNER_FOCALROOM_IOS_PICKER_SECOND_ID: projectIDs[1],
  });
  const rows = (await pool.query('SELECT id,revision,snapshot FROM fr_projects ORDER BY id')).rows;
  assert.deepEqual(rows, beforeProjects, '切换项目不得创建云端副本、修改作品版本或快照');
  assert.deepEqual((await pool.query('SELECT id,revision FROM fr_boards ORDER BY id')).rows, beforeBoards, '切换项目不得修改画布');
  const evidence = { fixture_only: true, source_baseline: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    tabs: ['Focal AI', '画布', '消息'], cloud_projects: 2, cloud_mode: 'PRIVATE_SYNCED', local_project_visible: true,
    search_tested: true, cloud_revision_unchanged: true, actual_model_calls: 0 };
  await writeFile(join(evidenceDirectory, 'result.json'), JSON.stringify(evidence, null, 2) + '\n');
  console.log(JSON.stringify(evidence));
} finally {
  try { await cp(evidenceDirectory, join(root, 'test-results', `project-picker-${basename(directory)}`), { recursive: true }); }
  finally { await api?.close(); await pool.end(); }
}
