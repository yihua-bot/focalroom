// 仅在 run.mjs 的全新临时数据库和未安装 App 的独立 iPhone 模拟器中验证。
import assert from 'node:assert/strict';
import { randomUUID, createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdir, writeFile, readFile, cp } from 'node:fs/promises';
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
const aiCalls = [];
const run = (args, env = {}) => new Promise((resolve, reject) => {
  const child = spawn('xcodebuild', args, { cwd: root, stdio: 'inherit', env: { ...process.env, ...env } });
  child.once('error', reject);
  child.once('exit', code => code === 0 ? resolve() : reject(new Error(`xcodebuild exited ${code}`)));
});
try {
  await migrate(pool);
  const aiTransport = async body => {
    const parts = body.input.flatMap(message => message.content ?? []);
    const images = parts.filter(part => part.type === 'input_image');
    const text = parts.filter(part => part.type === 'input_text').map(part => part.text).join('\n');
    const last = body.input.at(-1).content.find(part => part.type === 'input_text').text;
    assert.ok(!(text.includes('本机甲问题') && text.includes('本机乙问题')), '本机项目的历史不得混入另一项目');
    for (const image of images) {
      const metadata = await sharp(Buffer.from(image.image_url.split(',')[1], 'base64')).metadata();
      assert.equal(metadata.format, 'jpeg');
      assert.ok(metadata.width <= 1400 && metadata.height <= 1400);
      assert.equal(metadata.exif, undefined);
      assert.equal(image.detail, 'low');
    }
    aiCalls.push({ last, images: images.length });
    const textReply = `本机回复：${last.split('\n')[0]}` + (last.includes('本机甲问题')
      ? '\n\n我们可以先从画面的光线、节奏和视觉重心入手，再考虑它在整组作品中的位置。'
      : '');
    const reply = { id: randomUUID(), status: 'completed',
      output: [{ type: 'message', role: 'assistant', content: [{ type: 'output_text', text: textReply }] }],
      usage: { input_tokens: 12, output_tokens: 8, total_tokens: 20 } };
    if (body.stream) {
      const encoder = new TextEncoder();
      const event = value => encoder.encode(`data: ${JSON.stringify(value)}\n\n`);
      const stream = new ReadableStream({
        start(controller) {
          if (last.includes('模拟流中断')) {
            controller.enqueue(event({ type: 'response.output_text.delta', delta: '尚未完成的片段' }));
            controller.close();
            return;
          }
          controller.enqueue(event({ type: 'response.output_text.delta', delta: textReply }));
          // 首段立即显示，完成回执晚于项目切换，核对流式内容与结果均不串项目。
          const finish = () => {
            controller.enqueue(event({ type: 'response.completed', response: reply }));
            controller.close();
          };
          if (images.length) setTimeout(finish, 12000);
          else finish();
        },
      });
      return new Response(stream, { headers: { 'content-type': 'text/event-stream' } });
    }
    return new Response(JSON.stringify(reply), { headers: { 'content-type': 'application/json' } });
  };
  api = await createApi({ pool, store: new LocalPreviewStore(join(directory, 'picker-previews')), origin: 'http://127.0.0.1:1424',
    hostedAi: { key: 'local-fixture-key', initialTokens: 50000 }, hostedAiTransport: aiTransport });
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
  await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=$1', [owner.user.id]);
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
  const derivedDataPath = args[args.indexOf('-derivedDataPath') + 1];
  execFileSync('xcrun', ['simctl', 'install', device, join(derivedDataPath, 'Build/Products/Debug-iphonesimulator/FocalRoom.app')]);
  const container = execFileSync('xcrun', ['simctl', 'get_app_container', device, 'studio.focalroom.ios.development', 'data'], { encoding: 'utf8' }).trim();
  const scope = { server: base, userID: owner.user.id };
  const scopeKey = createHash('sha256').update(`${base}\n${owner.user.id}`).digest('hex');
  const libraryPath = join('Library/Application Support/FocalRoom-iPhone/LocalLibrary', scopeKey);
  const mediaPath = join(container, libraryPath, 'media');
  await mkdir(mediaPath, { recursive: true });
  const localProjects = [];
  for (const [index, label] of ['甲', '乙'].entries()) {
    const bytes = await sharp({ create: { width: 160, height: 100, channels: 3, background: index ? '#a58966' : '#7696a5' } }).jpeg().toBuffer();
    const hash = createHash('sha256').update(bytes).digest('hex');
    const now = Date.now() / 1000 - 978307200;
    localProjects.push({ id: index ? randomUUID() : projectIDs[0], title: `本机项目${label}`, createdAt: now, updatedAt: now,
      photos: [{ id: randomUUID(), filename: `本机${label}.jpg`, sourceSHA256: hash, previewSHA256: hash, byteCount: bytes.length,
        width: 160, height: 100, orientation: 1, format: 'JPEG', previewWidth: 160, previewHeight: 100, selection: 'UNDECIDED' }] });
    await writeFile(join(mediaPath, `${hash}.original`), bytes);
    await writeFile(join(mediaPath, `${hash}.jpg`), bytes);
  }
  await writeFile(join(container, libraryPath, 'library.json'), JSON.stringify({ version: 1, scope, projects: localProjects }));
  await run([...args, '-only-testing:FocalRoomUITests/FocalRoomUITests/testProjectSelectionInAITabCanvasAndMessages',
    '-resultBundlePath', join(evidenceDirectory, 'ProjectPicker.xcresult'), 'test-without-building'], {
    TEST_RUNNER_FOCALROOM_IOS_TEST_BASE_URL: base,
    TEST_RUNNER_FOCALROOM_IOS_PICKER_FIRST_ID: projectIDs[0],
    TEST_RUNNER_FOCALROOM_IOS_PICKER_SECOND_ID: projectIDs[1],
    TEST_RUNNER_FOCALROOM_IOS_PICKER_LOCAL_FIRST_ID: localProjects[0].id,
    TEST_RUNNER_FOCALROOM_IOS_PICKER_LOCAL_SECOND_ID: localProjects[1].id,
    TEST_RUNNER_FOCALROOM_IOS_PICKER_LOCAL_PHOTO_ID: localProjects[0].photos[0].id,
  });
  const rows = (await pool.query('SELECT id,revision,snapshot FROM fr_projects ORDER BY id')).rows;
  assert.deepEqual(rows, beforeProjects, '切换项目不得创建云端副本、修改作品版本或快照');
  assert.deepEqual((await pool.query('SELECT id,revision FROM fr_boards ORDER BY id')).rows, beforeBoards, '切换项目不得修改画布');
  assert.equal(aiCalls.length, 3, '取消授权不发图，流中断与恢复不得重复调用模型桩');
  assert.deepEqual(aiCalls.map(call => call.images), [1, 0, 0]);
  const jobStates = (await pool.query('SELECT state,count(*)::int AS count FROM fr_ai_requests GROUP BY state ORDER BY state')).rows;
  assert.deepEqual(jobStates, [{ state: 'COMPLETED', count: 2 }, { state: 'UNKNOWN', count: 1 }],
    '流中断必须保留未知结果，不得把片段标成完整回复');
  const finalContainer = execFileSync('xcrun', ['simctl', 'get_app_container', device, 'studio.focalroom.ios.development', 'data'], { encoding: 'utf8' }).trim();
  const localDocument = JSON.parse(await readFile(join(finalContainer, libraryPath, 'library.json'), 'utf8'));
  assert.equal(localDocument.projects.length, 3);
  assert.ok(localDocument.projects.every(project => !project.migration), '选择本机项目和观察不得隐式迁云');
  for (const project of localProjects) {
    assert.deepEqual(localDocument.projects.find(value => value.id === project.id), project, '本机项目与选片不得被观察操作改写');
    for (const photo of project.photos) for (const suffix of ['original', 'jpg']) {
      const bytes = await readFile(join(finalContainer, libraryPath, 'media', `${photo.sourceSHA256}.${suffix}`));
      assert.equal(createHash('sha256').update(bytes).digest('hex'), photo.sourceSHA256, '原片与本机预览逐字节不变');
    }
  }
  const evidence = { fixture_only: true, source_baseline: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    tabs: ['Focal AI', '画布', '消息'], cloud_projects: 2, cloud_mode: 'PRIVATE_SYNCED', local_project_visible: true,
    search_tested: true, cloud_revision_unchanged: true, actual_model_calls: 0, stub_model_calls: aiCalls.length,
    local_selection_persists_across_tabs: true, local_ai_without_promotion: true, local_chat_isolated_after_restart: true,
    late_reply_isolated: true, originals_unchanged: true, local_cloud_same_uuid_distinguished: true };
  await writeFile(join(evidenceDirectory, 'result.json'), JSON.stringify(evidence, null, 2) + '\n');
  console.log(JSON.stringify(evidence));
} finally {
  try { await cp(evidenceDirectory, join(root, 'test-results', `project-picker-${basename(directory)}`), { recursive: true }); }
  finally { await api?.close(); await pool.end(); }
}
