// 真实 Swift → C ABI → Rust AgentEngine → 本机 API/临时 PG → 模型替身。
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
assert.ok(directory, '仅通过临时数据库 runner 执行');
assert.equal(new URL(process.env.FOCALROOM_TEST_DATABASE_URL).hostname, '127.0.0.1');
const device = process.env.FOCALROOM_IOS_SIMULATOR_ID;
assert.ok(device);
const devices = JSON.parse(execFileSync('xcrun', ['simctl', 'list', 'devices', '-j'], { encoding: 'utf8' }));
assert.ok(Object.values(devices.devices).flat().some(item => item.udid === device && item.name.startsWith('FocalRoom Shared AI ')));
let installed = false;
try { execFileSync('xcrun', ['simctl', 'get_app_container', device, 'studio.focalroom.ios.development', 'data'], { stdio: 'ignore' }); installed = true; } catch {}
assert.equal(installed, false, '仅安装到本轮新建的独立模拟器');
const root = fileURLToPath(new URL('..', import.meta.url));
const evidence = join(directory, 'ai-unification-evidence');
await mkdir(evidence, { recursive: true });
const pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
const calls = [];
let api, libraryFile, libraryRelative;
const activeLibrary = () => join(execFileSync('xcrun', ['simctl', 'get_app_container', device, 'studio.focalroom.ios.development', 'data'], { encoding: 'utf8' }).trim(), libraryRelative, 'library.json');
const run = (args, environment = {}) => new Promise((resolve, reject) => {
  const child = spawn('xcodebuild', args, { cwd: root, stdio: 'inherit', env: { ...process.env, ...environment } });
  child.once('error', reject); child.once('exit', code => code === 0 ? resolve() : reject(new Error(`Xcode ${code}`)));
});
const tool = (name, args) => [{ type: 'function_call', call_id: `fixture-${name}`, status: 'completed', name, arguments: JSON.stringify(args) }];
try {
  await migrate(pool);
  const transport = async body => {
    try {
    const parts = body.input.flatMap(item => item.content ?? []);
    const images = parts.filter(part => part.type === 'input_image');
    calls.push({ instructions: body.instructions, input: body.input, images: images.length, tools: body.tools?.map(item => item.name) ?? [] });
    let output;
    if (images.length) {
      for (const image of images) {
        const imageInfo = await sharp(Buffer.from(image.image_url.split(',')[1], 'base64')).metadata();
        assert.equal(imageInfo.format, 'jpeg'); assert.ok(imageInfo.width <= 512 && imageInfo.height <= 512);
        assert.equal(imageInfo.exif, undefined);
      }
      const ids = parts.filter(part => part.type === 'input_text' && part.text.startsWith('photo_id: ')).map(part => part.text.slice(10));
      const observations = ids.map(id => ({ photo_id: id, visible_facts: ['画面为测试生成的均匀色块'], interpretation: '仅用于流程验证', uncertainty: '本地模型替身，不能代表摄影质量' }));
      output = [{ type: 'message', content: [{ type: 'output_text', text: JSON.stringify({ observations }) }] }];
    } else if (!body.tools?.length) {
      assert.ok(body.instructions.includes('中文交流以自然、完整的中文为主'));
      output = [{ type: 'message', content: [{ type: 'output_text', text: '## 从光线开始\n\n先观察光线的方向和柔硬，再看它怎样影响主体。\n\n- 留意明暗关系\n- 比较画面重心' }] }];
    } else {
      const first = body.input[0].content[0].text;
      const project = first.match(/项目 ID：([^\n]+)/)[1], task = first.match(/任务 ID：([^\n]+)/)[1];
      const raw = body.input[1].content[0].text;
      const context = JSON.parse(raw.slice(raw.indexOf('{')));
      const photo = context.gui.selected_photo_ids[0];
      const receipts = body.input.filter(item => item.type === 'function_call_output').map(item => JSON.parse(item.output));
      const observed = receipts.some(item => item.ok && item.result?.mode === 'vision_observed');
      const draft = receipts.find(item => item.result?.draft_id)?.result.draft_id;
      if (!observed) output = tool('inspect_photos', { project_id: project, photo_ids: [photo] });
      else if (!draft) output = tool('create_selection_draft', { project_id: project, base_project_revision: context.project.base_revision,
        title: '共享核心选片草案', summary: '本地替身创建的真实待审草案', items: [{ photo_id: photo, proposed_action: 'keep', position: 0, rationale: '流程验证建议，需作者确认' }] });
      else {
        const before = JSON.parse(await readFile(activeLibrary(), 'utf8'));
        assert.ok(before.projects[0].photos.every(photo => photo.selection === 'UNDECIDED'), '确认前不能改写源项目');
        output = tool('finish_task', { project_id: project, task_id: task, draft_ids: [draft], summary: '已生成一份可审阅的选片草案。' });
      }
    }
    return new Response(JSON.stringify({ id: randomUUID(), status: 'completed', model: 'deepseek-flash', output,
      usage: { input_tokens: 12, output_tokens: 8, total_tokens: 20 } }), { headers: { 'content-type': 'application/json' } });
    } catch (error) { await writeFile(join(evidence, 'transport-error.txt'), error.stack + '\n'); throw error; }
  };
  api = await createApi({ pool, store: new LocalPreviewStore(join(directory, 'previews')), origin: 'http://127.0.0.1:1424',
    hostedAi: { key: 'local-fixture-key', initialTokens: 1000000 }, hostedAiTransport: transport });
  await api.app.listen(0, '127.0.0.1');
  const base = await api.app.getUrl();
  const register = await fetch(`${base}/api/v2/auth/register`, { method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: 'shared-ai@fr.test', password: 'iphone-local-test-2026', display_name: '共享 AI 测试' }) });
  assert.equal(register.status, 200); const owner = await register.json();
  await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=$1', [owner.user.id]);
  const derived = process.env.FOCALROOM_IOS_SHARED_DERIVED_DATA ?? join(directory, 'Xcode');
  const args = ['-quiet', '-project', 'FocalRoom.xcodeproj', '-scheme', 'FocalRoom', '-configuration', 'Debug', '-destination', `id=${device}`,
    '-derivedDataPath', derived, '-parallel-testing-enabled', 'NO', 'CODE_SIGN_IDENTITY=-'];
  await run([...args, 'build-for-testing']);
  execFileSync('xcrun', ['simctl', 'install', device, join(derived, 'Build/Products/Debug-iphonesimulator/FocalRoom.app')]);
  const container = execFileSync('xcrun', ['simctl', 'get_app_container', device, 'studio.focalroom.ios.development', 'data'], { encoding: 'utf8' }).trim();
  const scope = { server: base, userID: owner.user.id };
  const scopeKey = createHash('sha256').update(`${base}\n${owner.user.id}`).digest('hex');
  libraryRelative = join('Library/Application Support/FocalRoom-iPhone/LocalLibrary', scopeKey);
  const library = join(container, libraryRelative);
  const media = join(library, 'media'); await mkdir(media, { recursive: true });
  const photos = [];
  for (const [index, color] of ['#7696a5', '#a58966'].entries()) {
    const bytes = await sharp({ create: { width: 640, height: 400, channels: 3, background: color } }).jpeg().toBuffer();
    const hash = createHash('sha256').update(bytes).digest('hex');
    photos.push({ id: randomUUID(), filename: `测试照片${index + 1}.jpg`, sourceSHA256: hash, previewSHA256: hash, byteCount: bytes.length,
      width: 640, height: 400, orientation: 1, format: 'JPEG', previewWidth: 640, previewHeight: 400, selection: 'UNDECIDED' });
    await writeFile(join(media, `${hash}.original`), bytes); await writeFile(join(media, `${hash}.jpg`), bytes);
  }
  const now = Date.now() / 1000 - 978307200;
  const project = { id: randomUUID(), title: '共享 AI 本机项目', createdAt: now, updatedAt: now, photos };
  libraryFile = join(library, 'library.json');
  await writeFile(libraryFile, JSON.stringify({ version: 1, scope, projects: [project] }));
  await run([...args, '-only-testing:FocalRoomUITests/FocalRoomUITests/testSharedAIEngineJourney', '-resultBundlePath', join(evidence, 'SharedAI.xcresult'), 'test-without-building'], {
    TEST_RUNNER_FOCALROOM_IOS_TEST_BASE_URL: base, TEST_RUNNER_FOCALROOM_IOS_SHARED_PROJECT_ID: project.id,
    TEST_RUNNER_FOCALROOM_IOS_SHARED_PHOTO_ID: photos[0].id,
  });
  libraryFile = activeLibrary();
  const final = JSON.parse(await readFile(libraryFile, 'utf8')).projects[0];
  assert.equal(final.photos[0].selection, 'KEEP'); assert.equal(final.photos[1].selection, 'UNDECIDED');
  assert.deepEqual(final.sequencePhotoIDs, [photos[0].id]);
  assert.equal(final.appliedAIDraftIDs.length, 1);
  for (const photo of photos) for (const suffix of ['original', 'jpg']) {
    const activeMedia = join(libraryFile, '..', 'media');
    assert.equal(createHash('sha256').update(await readFile(join(activeMedia, `${photo.sourceSHA256}.${suffix}`))).digest('hex'), photo.sourceSHA256);
  }
  assert.equal((await pool.query('SELECT count(*)::int AS count FROM fr_projects')).rows[0].count, 0, 'AI 不得隐式迁云');
  assert.equal(calls.filter(item => item.images).length, 1);
  assert.equal(calls.length, 5, '聊天一次、任务三轮文字与一次视觉观察；取消授权和重启不新增模型调用');
  const report = { fixture_only: true, actual_model_calls: 0, stub_model_calls: calls.length, shared_chat_prompt: true,
    real_agent_tool_loop: true, vision_previews_max_edge: 512, source_unchanged_before_review: true,
    original_hashes_unchanged: true, reviewed_selection_persisted: true, restart_did_not_resubmit: true };
  await writeFile(join(evidence, 'result.json'), JSON.stringify(report, null, 2) + '\n');
  await writeFile(join(evidence, 'requests.json'), JSON.stringify(calls, null, 2) + '\n');
  console.log(JSON.stringify(report));
} finally {
  await writeFile(join(evidence, 'requests.json'), JSON.stringify(calls, null, 2) + '\n');
  try { await cp(evidence, join(root, 'test-results', `ai-unification-${basename(directory)}`), { recursive: true }); }
  finally { await api?.close(); await pool.end(); }
}
