// 仅在既有 runner 创建的全新回环 PostgreSQL 中运行，无生产、真人照片、邮件或模型调用。
import assert from 'node:assert/strict';
import { randomUUID, createHash } from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { join, basename } from 'node:path';
import { writeFile, mkdir, readFile, open, unlink, cp, appendFile } from 'node:fs/promises';
import { Pool } from 'pg';
import sharp from 'sharp';
import { createApi } from '../../api/src/server.ts';
import { migrate } from '../../api/src/database.ts';
import { LocalPreviewStore } from '../../api/src/storage.ts';

assert.ok(process.env.FOCALROOM_TEST_DIRECTORY, '必须由 apps/api/test/run.mjs 启动');
assert.equal(new URL(process.env.FOCALROOM_TEST_DATABASE_URL).hostname, '127.0.0.1');
const root = fileURLToPath(new URL('..', import.meta.url));
const sourceBaseline = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
const directory = process.env.FOCALROOM_TEST_DIRECTORY;
const evidenceDirectory = join(directory, 'ios-evidence');
const savedEvidenceDirectory = join(root, 'test-results', basename(directory));
await mkdir(evidenceDirectory, { recursive: true });
// 放到本轮临时目录，避免产物变化触发本地 Swift package 的重复解析。
const verificationBuild = join(directory, 'ios-build');
const magicCodesPath = join(directory, 'magic-codes.jsonl');
const pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL });
let api;
let visionCalls = 0;
let deviceLease;
let deviceLeasePath;
const run = (command, args, env = {}) => new Promise((resolve, reject) => {
  const child = spawn(command, args, { cwd: root, stdio: 'inherit', env: { ...process.env, ...env } });
  child.once('error', reject);
  child.once('exit', code => code === 0 ? resolve() : reject(new Error(`${command} exited ${code}`)));
});

try {
  await migrate(pool);
  // 确定性模型桩：走真实 API 面、鉴权与计量校验，不调真实模型（沿仓库注入本地 Provider 的既有模式）。
  const aiTransport = async body => {
    const images = body.input.flatMap(item => item.content ?? []).filter(part => part.type === 'input_image');
    for (const image of images) {
      const bytes = Buffer.from(image.image_url.split(',')[1], 'base64');
      const metadata = await sharp(bytes).metadata();
      assert.ok(metadata.width <= 1400 && metadata.height <= 1400);
      assert.equal(metadata.format, 'jpeg');
      assert.equal(image.detail, 'low');
      assert.equal(metadata.exif, undefined, '模型桩不得收到源 EXIF');
    }
    if (images.length) visionCalls += 1;
    const last = body.input[body.input.length - 1];
    const prompt = typeof last?.content?.[0]?.text === 'string' ? last.content[0].text : '';
    const text = `桩回复：收到「${prompt.slice(0, 40)}」（${body.input.length} 条输入，观察 ${images.length} 张预览）`;
    // 流式请求按 SSE 形状应答（与服务端 stream 路由的上游契约一致），验证客户端流式消费。
    if (body.stream === true) {
      const frame = event => `data: ${JSON.stringify(event)}\n\n`;
      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(frame({ type: 'response.output_text.delta', delta: '桩回复：' })));
          controller.enqueue(new TextEncoder().encode(frame({ type: 'response.output_text.delta', delta: text.slice('桩回复：'.length) })));
          controller.enqueue(new TextEncoder().encode(frame({
            type: 'response.completed',
            response: {
              id: `resp-stub-${randomUUID()}`, status: 'completed',
              output: [{ type: 'message', role: 'assistant', content: [{ type: 'output_text', text }] }],
              usage: { input_tokens: 12, output_tokens: 8, total_tokens: 20 },
            },
          })));
          controller.close();
        },
      });
      return new Response(stream, { status: 200, headers: { 'content-type': 'text/event-stream' } });
    }
    return new Response(JSON.stringify({
      id: `resp-stub-${randomUUID()}`, status: 'completed',
      output: [{ type: 'message', role: 'assistant', content: [{ type: 'output_text', text }] }],
      usage: { input_tokens: 12, output_tokens: 8, total_tokens: 20 },
    }), { status: 200, headers: { 'content-type': 'application/json' } });
  };
  api = await createApi({ pool, store: new LocalPreviewStore(join(directory, 'ios-previews')), origin: 'http://127.0.0.1:1424',
    hostedAi: { key: 'stub-key', initialTokens: 50000 }, hostedAiTransport: aiTransport,
    magicEmail: async (email, code) => {
      await appendFile(magicCodesPath, JSON.stringify({ email, code }) + '\n');
      return true;
    } });
  await api.app.listen(0, '127.0.0.1');
  const base = await api.app.getUrl();
  const call = async (path, token, body) => {
    const response = await fetch(base + '/api/v2' + path, {
      method: body === undefined ? 'GET' : 'POST',
      headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    const result = await response.json();
    assert.equal(response.status, 200, JSON.stringify(result));
    return result;
  };
  const password = 'iphone-local-test-2026';
  const owner = await call('/auth/register', null, { email: 'iphone@fr.test', password, display_name: 'iPhone 本地测试' });
  // 夹具内直接置为已验证邮箱：AI 端点要求验证后使用，不引入真实邮件投递。
  await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=$1', [owner.user.id]);
  const viewer = await call('/auth/register', null, { email: 'viewer@fr.test', password, display_name: '只读测试账户' });
  const erasureUser = await call('/auth/register', null, { email: 'erase-ui@fr.test', password, display_name: '界面删除演练账户' });
  const workspace = (await call('/workspaces', owner.token))[0].id;
  const photos = await Promise.all(['#b7a58d', '#475f68', '#89746a'].map(async (background, index) => {
    const bytes = await sharp({ create: { width: 960, height: 640, channels: 3, background } }).jpeg().toBuffer();
    return { id: `ios-photo-${index + 1}`, filename: `iPhone_Test_${index + 1}.jpg`, bytes,
      source_sha256: createHash('sha256').update(bytes).digest('hex'), selection: 'UNDECIDED' };
  }));
  const manifest = {
    workspace_id: workspace, local_project_id: 'iphone-local-fixture', promotion_id: randomUUID(), title: 'iPhone 联调项目', from_revision: 0,
    snapshot: { photos: photos.map(({ bytes: _bytes, ...photo }) => { void _bytes; return photo; }),
      sequences: [{ id: 'ios-sequence', title: '主序列', items: photos.map(photo => ({ id: 'item-' + photo.id, photo_id: photo.id })) }] },
    media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false },
  };
  const promotion = await call('/promotions', owner.token, manifest);
  const projectID = promotion.project_id;
  for (const photo of photos) {
    const response = await fetch(`${base}/api/v2/projects/${projectID}/previews/${photo.id}`, {
      method: 'PUT', headers: { Authorization: `Bearer ${owner.token}`, 'Content-Type': 'image/jpeg' }, body: photo.bytes,
    });
    assert.equal(response.status, 200);
  }
  await call(`/projects/${projectID}/finalize`, owner.token, { manifest_digest: promotion.manifest_digest });
  const invitation = await call(`/workspaces/${workspace}/invitations`, owner.token,
    { email: viewer.user.email, role: 'VIEWER', project_id: projectID });
  await call('/invitations/accept', viewer.token, { token: new URL(invitation.invite_url).hash.split('=')[1] });
  const commentID = randomUUID();
  await call(`/projects/${projectID}/commands`, owner.token, {
    version: '2.0', command_id: commentID, idempotency_key: commentID, workspace_id: workspace, project_id: projectID,
    expected_revision: 0, kind: 'comment.add', payload: { body: 'iPhone 本地隔离测试批注', anchor: { type: 'photo', photo_id: 'ios-photo-1', x: .5, y: .5 } },
  });
  const board = await call('/boards', owner.token, { projectId: projectID, title: 'iPhone 测试画布' });
  const asset = (await pool.query('SELECT asset_id FROM fr_asset_aliases WHERE project_id=$1 AND photo_id=$2', [projectID, 'ios-photo-1'])).rows[0];
  assert.ok(asset, '实际上传链路应建立照片到资产的映射');
  const boardCommandID = randomUUID();
  await call(`/boards/${board.id}/commands`, owner.token, {
    schemaVersion: 3, commandId: boardCommandID, idempotencyKey: boardCommandID, projectId: projectID, baseRevision: 0,
    operations: [{ opId: randomUUID(), action: { type: 'createNode', nodeType: 'photo', x: 0, y: 0, width: 320, height: 240, assetId: asset.asset_id } }],
  });
  const env = { FOCALROOM_IOS_TEST_BASE_URL: base, FOCALROOM_IOS_TEST_PROJECT_ID: projectID,
    FOCALROOM_IOS_TEST_MAGIC_CODES: magicCodesPath };
  if (process.argv.includes('--serve')) {
    console.log(JSON.stringify({ base, projectID, fixture_only: true, email: 'iphone@fr.test', password, directory }));
    await writeFile(join(directory, 'ios-review.json'), JSON.stringify({ base, projectID, fixture_only: true }));
    await new Promise(resolve => { process.once('SIGTERM', resolve); process.once('SIGINT', resolve); });
  } else {
    await run('swift', ['test', '--scratch-path', join(verificationBuild, 'SwiftCore'), '--filter', 'LiveAPIIntegrationTests'], env);
    assert.equal((await pool.query("SELECT count(*)::int AS count FROM fr_commands WHERE project_id=$1 AND envelope->>'kind'='photo.set_selection'", [projectID])).rows[0].count, 1);
    const migration = (await pool.query("SELECT id,state,revision,snapshot->'photos' AS photos,(SELECT count(*)::int FROM fr_previews WHERE project_id=fr_projects.id) AS previews FROM fr_projects WHERE title='本机迁云联调'")).rows[0];
    assert.ok(migration, 'Swift 迁云旅程应创建云端项目');
    assert.equal(migration.state, 'SHARED');
    assert.equal(migration.revision, 2, '迁云提交后两条联调选片（另一设备 + 回执恢复）各推进一次版本');
    assert.equal(migration.photos.length, 2);
    assert.equal(migration.photos.filter(photo => photo.selection === 'KEEP').length, 0, '联调选片后无保留照片，本机保留状态只在迁云快照中体现');
    assert.equal(migration.previews, 2, '迁云只上传派生预览');
    const researchLeft = (await pool.query('SELECT count(*)::int AS n FROM fr_research_references WHERE project_id=$1', [projectID])).rows[0].n;
    assert.equal(researchLeft, 0, 'Swift research 旅程应清理自身资料，UI 旅程以空列表起测');
    const evidence = { fixture_only: true, actual_model_calls: 0, generated_photos: photos.length, lost_receipt_command_count: 1,
      source_baseline: sourceBaseline, ui_tested: false, migration_project: migration.id, migration_resumed_after_dropped_upload: true,
      research_tested: true };
    if (process.argv.includes('--ui')) {
      const device = process.env.FOCALROOM_IOS_SIMULATOR_ID;
      assert.ok(device, '请指定独立评审 iPhone 模拟器 FOCALROOM_IOS_SIMULATOR_ID');
      const devices = JSON.parse(execFileSync('xcrun', ['simctl', 'list', 'devices', '-j'], { encoding: 'utf8' }));
      assert.ok(Object.values(devices.devices).flat().some(item => item.udid === device && item.name.startsWith('FocalRoom ')), '仅使用独立 FocalRoom 评审模拟器');
      deviceLeasePath = join(root, 'test-results', `device-${device}.lock`);
      deviceLease = await open(deviceLeasePath, 'wx').catch(() => { throw new Error('该评审设备已被占用，请使用新建的独立模拟器；不卸载其他轮次的 App'); });
      await deviceLease.writeFile(JSON.stringify({ pid: process.pid, fixture_only: true, run: basename(directory) }));
      // 每次旅程独立构建目录，避免其他归档或验证进程锁住同一 build.db。
      const derivedDataPath = join(verificationBuild, 'Xcode');
      const buildArgs = ['-project', 'FocalRoom.xcodeproj', '-scheme', 'FocalRoom', '-configuration', 'Debug',
        '-destination', `id=${device}`, '-derivedDataPath', derivedDataPath, '-parallel-testing-enabled', 'NO', 'CODE_SIGN_IDENTITY=-'];
      await run('xcodebuild', [...buildArgs, 'build-for-testing']);
      let previousContainer;
      try { previousContainer = execFileSync('xcrun', ['simctl', 'get_app_container', device, 'studio.focalroom.ios.development', 'data'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim(); } catch { /* 首次安装 */ }
      if (previousContainer) {
        const marker = JSON.parse(await readFile(join(previousContainer, 'Documents/.focalroom-test-fixture.json'), 'utf8').catch(() => '{}'));
        assert.equal(marker.fixture_only, true, '旧 App 没有测试素材标记，拒绝卸载；请新建评审模拟器');
        execFileSync('xcrun', ['simctl', 'uninstall', device, 'studio.focalroom.ios.development']);
      }
      execFileSync('xcrun', ['simctl', 'install', device, join(derivedDataPath, 'Build/Products/Debug-iphonesimulator/FocalRoom.app')]);
      const container = execFileSync('xcrun', ['simctl', 'get_app_container', device, 'studio.focalroom.ios.development', 'data'], { encoding: 'utf8' }).trim();
      const documentDirectory = join(container, 'Documents');
      await mkdir(documentDirectory, { recursive: true });
      await writeFile(join(documentDirectory, '.focalroom-test-fixture.json'), JSON.stringify({ fixture_only: true, run: basename(directory) }));
      const fileName = `FocalRoomFile-${projectID.slice(0, 8)}.jpg`;
      const fileBytes = await sharp({ create: { width: 640, height: 960, channels: 3, background: '#6f486a' } }).jpeg().toBuffer();
      await writeFile(join(documentDirectory, fileName), fileBytes);
      const pickerFiles = [];
      for (const photo of photos) {
        const path = join(directory, photo.filename);
        await writeFile(path, photo.bytes);
        pickerFiles.push(path);
      }
      execFileSync('xcrun', ['simctl', 'addmedia', device, ...pickerFiles]);
      await run('xcodebuild', [...buildArgs, '-resultBundlePath', join(evidenceDirectory, 'iPhone-UI.xcresult'), 'test-without-building'], {
        ...env, TEST_RUNNER_FOCALROOM_IOS_TEST_BASE_URL: base, TEST_RUNNER_FOCALROOM_IOS_TEST_PROJECT_ID: projectID,
        TEST_RUNNER_FOCALROOM_IOS_TEST_FILE_NAME: fileName,
      });
      const state = (await pool.query('SELECT revision,snapshot FROM fr_projects WHERE id=$1', [projectID])).rows[0];
      assert.equal((await pool.query('SELECT count(*)::int AS n FROM fr_account_closures WHERE user_id=$1', [erasureUser.user.id])).rows[0].n, 1);
      assert.equal((await pool.query('SELECT count(*)::int AS n FROM fr_sessions WHERE user_id=$1', [erasureUser.user.id])).rows[0].n, 0);
      evidence.account_erasure_ui = true;
      assert.equal(state.revision, 4, 'Swift 选片/提案全量采纳与 UI 选片/提案部分采纳各只推进一次正式版本');
      assert.equal(state.snapshot.photos[0].selection, 'KEEP');
      assert.equal(state.snapshot.photos.length, 3, '本机导入不得隐式写入云端图库');
      assert.deepEqual(state.snapshot.sequences[0].items.map(item => item.photo_id),
        ['ios-photo-1', 'ios-photo-2', 'ios-photo-3'], 'UI 部分采纳只应用勾选的 add 操作');
      const proposalStates = (await pool.query('SELECT state FROM fr_proposals WHERE project_id=$1 ORDER BY id', [projectID])).rows;
      assert.deepEqual(proposalStates.map(row => row.state).sort(), ['APPLIED', 'APPLIED', 'REJECTED'],
        '提案一全量采纳、提案二驳回、提案三被 UI 部分采纳');
      const boardRevision = (await pool.query('SELECT min(revision) AS r FROM fr_boards WHERE project_id=$1', [projectID])).rows[0].r;
      assert.ok(Number(boardRevision) >= 2, '画布移动命令应推进 Board 修订');
      const uiMigration = (await pool.query("SELECT id,state,revision,snapshot->'photos' AS photos,(SELECT count(*)::int FROM fr_previews WHERE project_id=fr_projects.id) AS previews FROM fr_projects WHERE title='iPhone 本机导入'")).rows[0];
      assert.ok(uiMigration, 'UI 旅程迁云应创建云端项目');
      assert.equal(uiMigration.state, 'SHARED');
      assert.equal(uiMigration.revision, 0, '迁云提交本身不推进正式修订');
      assert.equal(uiMigration.photos.length, 3);
      assert.equal(uiMigration.photos.filter(photo => photo.selection === 'KEEP').length, 1, '本机选片随迁云进入云端');
      assert.equal(uiMigration.previews, 3, '只上传 2048 派生预览，不上传原片');
      const uiResearch = (await pool.query('SELECT count(*)::int AS n FROM fr_research_references WHERE project_id=$1', [projectID])).rows[0].n;
      assert.equal(uiResearch, 0, 'UI research 旅程自建自删，不残留');
      const scope = createHash('sha256').update(`${base}\n${owner.user.id}`).digest('hex');
      // Xcode 测试安装会迁移数据容器，结束后重新解析，不能沿用安装前的绝对路径。
      const finalContainer = execFileSync('xcrun', ['simctl', 'get_app_container', device, 'studio.focalroom.ios.development', 'data'], { encoding: 'utf8' }).trim();
      const localDirectory = join(finalContainer, 'Library/Application Support/FocalRoom-iPhone/LocalLibrary', scope);
      const local = JSON.parse(await readFile(join(localDirectory, 'library.json'), 'utf8'));
      assert.equal(local.projects.length, 1);
      assert.equal(local.projects[0].photos.length, 3, '相册去重后两张，加文件导入一张，重启与退出不能丢失');
      assert.equal(local.projects[0].photos.filter(photo => photo.selection === 'KEEP').length, 1, '本机选片状态必须持久保存');
      assert.equal(local.projects[0].migration?.phase, 'migrated', '迁云终态必须随本机记录持久化');
      assert.equal(local.projects[0].migration?.cloudProjectID, uiMigration.id, '本机记录绑定同一云端项目');
      const scopeIndex = JSON.parse(await readFile(
        join(finalContainer, 'Library/Application Support/FocalRoom-iPhone/LocalLibrary/index.json'), 'utf8'));
      // 两条原生旅程分别登录作品账户与删除演练账户；关闭账号不删除本机取回索引。
      assert.equal(scopeIndex.length, 2, '只保留两条原生旅程实际登录过的账户索引');
      assert.deepEqual(scopeIndex.map(entry => ({ server: entry.server, userID: entry.userID, email: entry.email }))
        .sort((a, b) => a.userID.localeCompare(b.userID)),
      [owner, erasureUser].map(value => ({ server: base, userID: value.user.id, email: value.user.email }))
        .sort((a, b) => a.userID.localeCompare(b.userID)), '索引必须绑定各自账号和服务，不可串号');
      for (const entry of scopeIndex) {
        assert.deepEqual(Object.keys(entry).sort(), ['email', 'name', 'server', 'updatedAt', 'userID'], '离线索引不得保存凭据');
      }
      for (const photo of local.projects[0].photos) {
        const bytes = await readFile(join(localDirectory, 'media', `${photo.sourceSHA256}.original`));
        assert.equal(createHash('sha256').update(bytes).digest('hex'), photo.sourceSHA256);
        const preview = await readFile(join(localDirectory, 'media', `${photo.previewSHA256}.jpg`));
        assert.equal(createHash('sha256').update(preview).digest('hex'), photo.previewSHA256);
      }
      const importedFile = local.projects[0].photos.find(photo => photo.filename === fileName);
      assert.ok(importedFile);
      assert.equal(importedFile.sourceSHA256, createHash('sha256').update(fileBytes).digest('hex'), '文件导入必须逐字节保留源文件');
      evidence.local_imported_photos = 3;
      evidence.local_import_did_not_change_cloud = true;
      evidence.ui_migration_project = uiMigration.id;
      evidence.ui_research_journey = true;
      evidence.canvas_edit_tested = true;
      evidence.proposals_partial_adoption = true;
      evidence.sync_center_tested = true;
      evidence.offline_scope_index = true;
      evidence.ui_offline_browse = true;
      evidence.ui_tested = true;
    }
    assert.equal(visionCalls, process.argv.includes('--ui') ? 2 : 1, '只发送明确授权的观察请求，恢复不重新发送图像');
    evidence.authorized_vision_calls = visionCalls;
    await writeFile(join(evidenceDirectory, 'ios-verification.json'), JSON.stringify(evidence, null, 2) + '\n');
    console.log(JSON.stringify({ evidence: join(savedEvidenceDirectory, 'ios-verification.json'), ...evidence }));
  }
} finally {
  try {
    // 测试进程退出后再归档，避免 xcresult 持续写入触发本地 package 的监听。
    await cp(evidenceDirectory, savedEvidenceDirectory, { recursive: true });
  } finally {
    await api?.close();
    await pool.end();
    if (deviceLease) { await deviceLease.close(); await unlink(deviceLeasePath); }
  }
}
