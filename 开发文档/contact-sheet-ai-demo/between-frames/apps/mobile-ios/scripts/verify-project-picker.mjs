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
// 本机照片是纯色块，出网字节按平均色认归属：App 会重压缩预览（哈希必变），颜色不会变。
const localTint = [[0x76, 0x96, 0xa5], [0xa5, 0x89, 0x66]];
const isTint = (tint, expected) => Boolean(tint) && tint.every((value, index) => Math.abs(value - expected[index]) <= 12);
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
    // 共享核心把「编辑目标」放在首条、项目与记忆数据放在末条，纯文字聊天仍以末条为用户输入。
    const brief = body.input[0].content?.find(part => part.type === 'input_text')?.text ?? '';
    const questions = parts.filter(part => part.type === 'input_text')
      .map(part => part.text).filter(item => !item.startsWith('以下为项目与人工记忆数据'));
    const last = brief.startsWith('编辑目标：') ? brief.split('\n')[0].slice('编辑目标：'.length) : questions.at(-1) ?? '';
    assert.ok(!(text.includes('本机甲问题') && text.includes('本机乙问题')), '本机项目的历史不得混入另一项目');
    const tints = [];
    for (const image of images) {
      const buffer = Buffer.from(image.image_url.split(',')[1], 'base64');
      const metadata = await sharp(buffer).metadata();
      assert.equal(metadata.format, 'jpeg');
      assert.ok(metadata.width <= 1400 && metadata.height <= 1400);
      assert.equal(metadata.exif, undefined);
      assert.equal(image.detail, 'low');
      const stats = await sharp(buffer).stats();
      tints.push(stats.channels.slice(0, 3).map(channel => Math.round(channel.mean)));
    }
    const observationIDs = parts.filter(part => part.type === 'input_text' && part.text.startsWith('photo_id: ')).map(part => part.text.slice(10));
    aiCalls.push({ last, images: images.length, tools: body.tools?.length ?? 0, photoIDs: observationIDs, tints,
      甲: text.includes('本机甲问题'), 乙: text.includes('本机乙问题') });
    const textReply = `本机回复：${last.split('\n')[0]}` + (last.includes('本机甲问题')
      ? '\n\n我们可以先从画面的光线、节奏和视觉重心入手，再考虑它在整组作品中的位置。'
      : '');
    const envelope = output => ({ id: randomUUID(), status: 'completed', model: 'deepseek-flash', output,
      usage: { input_tokens: 12, output_tokens: 8, total_tokens: 20 } });
    const asJSON = output => new Response(JSON.stringify(envelope(output)), { headers: { 'content-type': 'application/json' } });
    const message = text => [{ type: 'message', role: 'assistant', content: [{ type: 'output_text', text }] }];
    const tool = (name, args) => [{ type: 'function_call', call_id: `fixture-${name}`, status: 'completed', name, arguments: JSON.stringify(args) }];
    const delayed = async (ms, build) => { await new Promise(resolve => setTimeout(resolve, ms)); return build(); };
    if (images.length && observationIDs.length) {
      // 带图请求来自共享核心的照片观察子调用：延后返回，留出切换项目的时间核对结果不串项目。
      const observations = observationIDs.map(id => ({ photo_id: id, visible_facts: ['画面为测试生成的均匀色块'],
        interpretation: '仅用于流程验证', uncertainty: '本地模型替身，不能代表摄影质量' }));
      return delayed(9000, () => asJSON(message(JSON.stringify({ observations }))));
    }
    if (body.tools?.length) {
      // 任务循环按桌面替身的方式走完：观察照片 → 生成草案 → 收尾总结，避免只回文字而留下重复结论。
      const project = brief.match(/项目 ID：([^\n]+)/)[1];
      const task = brief.match(/任务 ID：([^\n]+)/)[1];
      const raw = body.input[1].content[0].text;
      const context = JSON.parse(raw.slice(raw.indexOf('{')));
      const photo = context.gui.selected_photo_ids[0];
      const receipts = body.input.filter(item => item.type === 'function_call_output').map(item => JSON.parse(item.output));
      if (!receipts.some(item => item.ok && item.result?.mode === 'vision_observed')) {
        return asJSON(tool('inspect_photos', { project_id: project, photo_ids: [photo] }));
      }
      if (!receipts.some(item => item.result?.draft_id)) {
        return asJSON(tool('create_selection_draft', { project_id: project, base_project_revision: context.project.base_revision,
          title: '共享核心选片草案', summary: '本地替身创建的真实待审草案',
          items: [{ photo_id: photo, proposed_action: 'keep', position: 0, rationale: '流程验证建议，需作者确认' }] }));
      }
      const draft = receipts.find(item => item.result?.draft_id).result.draft_id;
      return asJSON(tool('finish_task', { project_id: project, task_id: task, draft_ids: [draft],
        summary: textReply }));
    }
    const reply = envelope(message(textReply));
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
          // 完成回执晚于项目切换，核对结果不串项目。
          const finish = () => {
            controller.enqueue(event({ type: 'response.completed', response: reply }));
            controller.close();
          };
          if (last.includes('本机乙问题')) setTimeout(finish, 9000);
          else finish();
        },
      });
      return new Response(stream, { headers: { 'content-type': 'text/event-stream' } });
    }
    if (last.includes('模拟流中断')) {
      // 手机聊天走一次性请求，没有半截正文：用「回执正文残缺」复现结果未知，界面不得把它当完整回复。
      return new Response(new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode('{"id":"truncated","status":"compl'));
          controller.close();
        },
      }), { headers: { 'content-type': 'application/json' } });
    }
    if (last.includes('本机乙问题')) {
      return delayed(9000, () => new Response(JSON.stringify(reply), { headers: { 'content-type': 'application/json' } }));
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
  const photoCalls = aiCalls.filter(call => call.images);
  await writeFile('/tmp/picker-aicalls.json', JSON.stringify(aiCalls, null, 1));
  assert.equal(photoCalls.length, 1, '取消授权的那次不得发送照片');
  // 共享核心把看图拆成独立子调用，那一发没有用户原话，且 App 重压缩后哈希也变，只能按像素字节认归属。
  assert.ok(photoCalls[0]?.tints?.length === 1 && isTint(photoCalls[0].tints[0], localTint[0]),
    `出网照片必须属于已确认授权的甲项目：${JSON.stringify(aiCalls.map(call => [call.images, call.tints, call.photoIDs]))}`);
  assert.ok(!aiCalls.some(call => call.tints.some(tint => isTint(tint, localTint[1]))), '未提问的乙照片不得出网');
  assert.equal(aiCalls.filter(call => call.last === '模拟流中断').length, 1, '流中断后的核对不得重复调用模型桩');
  assert.ok(aiCalls.length <= 8, `任务循环不应失控：${JSON.stringify(aiCalls.map(call => [call.last, call.images, call.tools]))}`);
  const jobStates = (await pool.query('SELECT state,count(*)::int AS count FROM fr_ai_requests GROUP BY state ORDER BY state')).rows;
  const stateCount = Object.fromEntries(jobStates.map(row => [row.state, row.count]));
  assert.ok((stateCount.UNKNOWN ?? 0) >= 1 || (stateCount.FAILED ?? 0) >= 1,
    '流中断的请求必须留下待核对状态，不得记成完整回复');
  assert.ok(stateCount.COMPLETED >= 1, '已闭环的请求应保留完成状态');
  const finalContainer = execFileSync('xcrun', ['simctl', 'get_app_container', device, 'studio.focalroom.ios.development', 'data'], { encoding: 'utf8' }).trim();
  const localDocument = JSON.parse(await readFile(join(finalContainer, libraryPath, 'library.json'), 'utf8'));
  assert.equal(localDocument.projects.length, 3);
  assert.ok(localDocument.projects.every(project => !project.migration), '选择本机项目和观察不得隐式迁云');
  for (const project of localProjects) {
    const current = localDocument.projects.find(value => value.id === project.id);
    assert.ok(current, '本机项目不得在观察与画布编辑后消失');
    // 画布编辑是本轮测试要验的写入，会改 canvas 与 updatedAt；照片与选片必须逐字节不变。
    const stable = value => ({ id: value.id, title: value.title, createdAt: value.createdAt, photos: value.photos });
    assert.deepEqual(stable(current), stable(project), '本机项目与选片不得被观察操作改写');
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
