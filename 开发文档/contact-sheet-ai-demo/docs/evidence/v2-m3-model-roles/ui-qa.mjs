// 独立临时库 + 合成照片；真实运行时配置投影，不调用任何模型或用户数据库。
import { createRequire } from 'node:module';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { createServer } from 'node:http';
import { mkdtemp, readFile, realpath, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { createHash } from 'node:crypto';
const require = createRequire(resolve('package.json'));
const { chromium, expect } = require('@playwright/test');
const { createServer: createVite } = await import(require.resolve('vite'));
const directory = resolve('../docs/evidence/v2-m3-model-roles');
const root = await realpath(await mkdtemp(join(tmpdir(), 'focalroom-role-ui-')));
const endpoint = 'http://127.0.0.1:9';
const hash = value => createHash('sha256').update(value).digest('hex');
for (const [role, capabilities] of [
  ['curator', ['tool_calling', 'instruction_following']],
  ['observer', ['vision', 'structured_output', 'instruction_following']],
]) await writeFile(join(root, role + '.json'), JSON.stringify({ version: 1, endpointSha256: hash(endpoint), modelId: role + '-ui', capabilities }));
const child = spawn(resolve('src-tauri/target-v1/debug/m1-harness'), [root, 'standard'], {
  stdio: ['pipe', 'pipe', 'inherit'],
  env: { ...process.env, BETWEEN_FRAMES_OPENAI_API_KEY: 'test-local-ui-only',
    BETWEEN_FRAMES_AI_BASE_URL: endpoint, BETWEEN_FRAMES_AI_MODEL: 'curator-ui',
    BETWEEN_FRAMES_AI_OBSERVATION_MODEL: 'observer-ui',
    BETWEEN_FRAMES_AI_CAPABILITY_PROFILE: join(root, 'curator.json'),
    BETWEEN_FRAMES_AI_OBSERVATION_CAPABILITY_PROFILE: join(root, 'observer.json') },
});
let sequence = 0;
const pending = new Map();
createInterface({ input: child.stdout }).on('line', line => {
  const reply = JSON.parse(line);
  pending.get(reply.id)?.(reply);
  pending.delete(reply.id);
});
const rpc = (command, args = {}) => new Promise((accept, reject) => {
  const id = ++sequence;
  const timeout = setTimeout(() => reject(new Error('harness timeout: ' + command)), 20000);
  pending.set(id, reply => { clearTimeout(timeout); reply.ok ? accept(reply.value) : reject(new Error(JSON.stringify(reply.error))); });
  child.stdin.write(JSON.stringify({ id, command, args }) + '\n');
});
const assets = createServer(async (request, response) => {
  try {
    const path = decodeURIComponent(request.url.slice(1));
    if (!path.startsWith(root + '/cache/') || !path.endsWith('.jpg')) throw new Error('outside fixture cache');
    response.setHeader('Content-Type', 'image/jpeg');
    response.end(await readFile(path));
  } catch { response.writeHead(404); response.end(); }
});
let vite, browser;
try {
  await new Promise(accept => assets.listen(0, '127.0.0.1', accept));
  const assetBase = 'http://127.0.0.1:' + assets.address().port + '/';
  vite = await createVite({ server: { host: '127.0.0.1', port: 0, strictPort: true } });
  await vite.listen();
  const base = 'http://127.0.0.1:' + vite.httpServer.address().port;
  const project = await rpc('create_project', { title: '模型分工 · 合成验收', brief: '本地合成界面验证，不调用真实模型。' });
  const token = await rpc('choose_import', { projectId: project.id });
  const job = await rpc('import_photos', { projectId: project.id, selectionToken: token });
  await expect.poll(async () => (await rpc('get_import_job', { jobId: job })).status, { timeout: 60000 }).toBe('completed');
  const before = await rpc('get_project', { projectId: project.id });
  const checksums = await rpc('test_fixture_checksums');
  const config = await rpc('get_ai_config', { projectId: project.id });
  expect(config.modelId).toBe('curator-ui');
  expect(config.observation.modelId).toBe('observer-ui');
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });
  const remote = [], errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    if (['127.0.0.1', 'localhost', '[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
    remote.push(route.request().url()); return route.abort();
  });
  await page.exposeFunction('fixtureRpc', rpc);
  await page.addInitScript(({ assetBase }) => {
    Object.assign(globalThis, { isTauri: true, __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
      __TAURI_INTERNALS__: { invoke: (command, args) => command.startsWith('plugin:event|') ? Promise.resolve(0) : globalThis.fixtureRpc(command, args),
        convertFileSrc: path => assetBase + encodeURIComponent(path), transformCallback: () => 0 } });
  }, { assetBase });
  await page.goto(base + '/#/project/' + project.id);
  await page.getByRole('button', { name: /摄影编辑 Agent/ }).click();
  await expect(page.getByText(/curator-ui 负责策展，observer-ui 负责图像观察/)).toBeVisible();
  await page.locator('.agent-form summary').click();
  await page.getByLabel('云端图像上限（张）').fill('2');
  await page.getByText('模型与密钥 · 诊断').click();
  await page.getByRole('button', { name: '管理图像授权' }).click();
  const dialog = page.getByRole('dialog', { name: '云端图像授权' });
  await expect(dialog).toContainText('型号：observer-ui');
  await expect(dialog).toContainText('同一服务的策展模型 curator-ui');
  await dialog.scrollIntoViewIfNeeded();
  await page.screenshot({ path: join(directory, 'roles-consent-1440.png') });
  await page.setViewportSize({ width: 1280, height: 800 });
  await dialog.scrollIntoViewIfNeeded();
  await page.screenshot({ path: join(directory, 'roles-consent-1280.png') });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.getByRole('button', { name: '本项目（可撤回，100 张）' }).click();
  await expect(dialog).toHaveCount(0);
  const granted = (await rpc('get_ai_config', { projectId: project.id })).consent;
  expect(granted.provider).toBe(config.consentProvider);
  expect(granted.mode).toBe('project');
  expect((await rpc('get_agent_projection', { projectId: project.id })).tasks).toHaveLength(0);
  await page.getByRole('button', { name: '管理图像授权' }).click();
  await page.getByRole('button', { name: '禁止云端图像' }).click();
  await expect(dialog).toHaveCount(0);
  expect((await rpc('get_ai_config', { projectId: project.id })).consent.mode).toBe('deny');
  expect(await rpc('get_project', { projectId: project.id })).toEqual(before);
  expect(await rpc('test_fixture_checksums')).toEqual(checksums);
  expect(remote).toEqual([]); expect(errors).toEqual([]);
  await writeFile(join(directory, 'ui-qa.json'), JSON.stringify({ synthetic: true, root, config, photos: before.photos.length,
    screenshots: ['roles-consent-1440.png', 'roles-consent-1280.png'], tasks: 0, modelRequests: 0,
    grantAndDeny: true, formalProjectUnchanged: true, fixtureHashesUnchanged: true, remote, errors }, null, 2));
  process.stdout.write('Model-role browser acceptance passed.\n');
} finally {
  await browser?.close();
  await vite?.close();
  await new Promise(accept => assets.close(accept));
  child.stdin.end();
}
