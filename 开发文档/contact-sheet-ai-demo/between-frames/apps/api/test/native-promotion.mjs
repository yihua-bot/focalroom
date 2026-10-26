import assert from 'node:assert/strict';
import { mkdir, writeFile, readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { createServer } from 'node:http';
import { createHash, randomUUID } from 'node:crypto';
import { Pool } from 'pg';
import sharp from 'sharp';
import { chromium, expect } from '@playwright/test';
import { createServer as createVite } from 'vite';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';
const root = fileURLToPath(new URL('../../..', import.meta.url));
const evidence = resolve(root, '../docs/evidence/v2-m4-m5-postgres'); await mkdir(evidence, { recursive: true });
const pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL }); await migrate(pool);
const api = await createApi({ pool, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'native-previews')), origin: 'http://127.0.0.1:1424' }); await api.app.listen(0, '127.0.0.1'); const base = await api.app.getUrl();
const email = 'native-owner@fr.test', password = 'native-password-123';
const account = await (await fetch(`${base}/api/v2/auth/register`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email, password, display_name: 'Native Owner' }) })).json();
let dropFinalize = true;
const proxy = createServer(async (req, res) => {
  try { const chunks = []; for await (const chunk of req) chunks.push(chunk); const upstream = await fetch(base + req.url, { method: req.method, headers: { ...req.headers, host: new URL(base).host }, ...(!['GET', 'HEAD'].includes(req.method) ? { body: Buffer.concat(chunks) } : {}) }); const bytes = Buffer.from(await upstream.arrayBuffer()); if (req.url.endsWith('/finalize') && dropFinalize && upstream.ok) { dropFinalize = false; res.destroy(); return; } res.writeHead(upstream.status, { 'content-type': upstream.headers.get('content-type') ?? 'application/json' }); res.end(bytes); } catch { res.writeHead(502); res.end('{}'); }
}); await new Promise(resolve => proxy.listen(0, '127.0.0.1', resolve)); const proxyBase = `http://127.0.0.1:${proxy.address().port}`;
let vite, browser, assets; let harness, next = 0; const pending = new Map();
const directory = join(process.env.FOCALROOM_TEST_DIRECTORY, 'native-harness'); await mkdir(join(directory, 'fixtures'), { recursive: true });
for (let n = 1; n <= 2; n++) await sharp({ create: { width: 150, height: 100, channels: 3, background: n === 1 ? '#27658a' : '#ae8d55' } }).jpeg().toFile(join(directory, 'fixtures', `photo-${n}.jpg`));
const hash = async path => createHash('sha256').update(await readFile(path)).digest('hex');
const originals = await Promise.all([1,2].map(n => hash(join(directory, 'fixtures', `photo-${n}.jpg`))));
function start() { harness = spawn(join(root, 'src-tauri/target-v1/debug/m1-harness'), [directory], { stdio: ['pipe', 'pipe', 'inherit'], env: { ...process.env, BETWEEN_FRAMES_OPENAI_API_KEY: '' } }); createInterface({ input: harness.stdout }).on('line', line => { const reply = JSON.parse(line); const p = pending.get(reply.id); pending.delete(reply.id); if (reply.ok) p.resolve(reply.value); else p.reject(Object.assign(new Error(reply.error.message), { code: reply.error.code })); }); }
function rpc(command, args = {}) { return new Promise((resolve, reject) => { const id = ++next; pending.set(id, { resolve, reject }); harness.stdin.write(JSON.stringify({ id, command, args }) + '\n'); }); }
async function stop() { if (!harness) return; const current = harness; harness = null; current.stdin.end(); await new Promise(resolve => current.once('exit', resolve)); }
async function project(title) { let p = await rpc('create_project', { title, brief: 'Shared authority test' }); const token = await rpc('choose_import', { projectId: p.id }); const job = await rpc('import_photos', { projectId: p.id, selectionToken: token }); let state; do { await new Promise(r => setTimeout(r, 25)); state = await rpc('get_import_job', { jobId: job }); } while (state.status === 'running'); assert.equal(state.imported, 2); p = await rpc('get_project', { projectId: p.id }); return p; }
const share = async (p, target = base, action = 'resume') => { await rpc("set_cloud_test_session",{origin:target,token:account.token}); return rpc('share_project', { request: { projectId: p.id, expectedRevision: p.revision, baseUrl: target, webUrl: 'http://127.0.0.1:1424', action, previewUploadAuthorized: true } }); };
try {
  start(); await rpc("set_cloud_test_session",{origin:base,token:account.token}); const p = await project('正常桌面共享'); const pack = await rpc('export_project', { projectId: p.id, format: 'shared' }); const manifest = JSON.parse(await readFile(join(pack.path, 'manifest.json'), 'utf8')); assert.equal(JSON.stringify(manifest).includes(directory), false); assert.equal(manifest.media_policy.ai_upload_authorized, false);
  assets = createServer(async (req, res) => { try { const path = decodeURIComponent(req.url.slice(1)); if (!path.startsWith(directory + "/cache/") || !path.endsWith(".jpg")) throw new Error("outside cache"); res.writeHead(200, { "content-type": "image/jpeg" }); res.end(await readFile(path)); } catch { res.writeHead(404); res.end(); } }); await new Promise(resolve => assets.listen(0,"127.0.0.1",resolve));
  vite = await createVite({ root, server: { host:"127.0.0.1",port:0,strictPort:true } }); await vite.listen(); const desktopBase="http://127.0.0.1:"+vite.httpServer.address().port;
  browser = await chromium.launch(); const page = await browser.newPage({ locale:"zh-CN", viewport:{width:1440,height:960} }); const pageErrors=[]; page.on("pageerror", e=>pageErrors.push(e.message)); await page.exposeFunction("fixtureRpc",rpc);
  await page.addInitScript(({assetBase})=>Object.assign(globalThis,{isTauri:true,__TAURI_EVENT_PLUGIN_INTERNALS__:{unregisterListener:()=>{}},__TAURI_INTERNALS__:{invoke:(name,args)=>name.startsWith("plugin:event|")?Promise.resolve(0):globalThis.fixtureRpc(name,args),convertFileSrc:path=>assetBase+encodeURIComponent(path),transformCallback:()=>0}}),{assetBase:"http://127.0.0.1:"+assets.address().port+"/"});
  await page.goto(desktopBase+"/#/project/"+p.id); await page.getByRole("button",{name:/^导出/}).click(); await page.locator(".sharing-controls input[type=checkbox]").check();
  await page.locator(".sharing-controls").scrollIntoViewIfNeeded(); await page.screenshot({path:join(evidence,"desktop-sharing-1440.png")}); await page.getByRole("button",{name:"开始共享"}).click(); await expect(page.getByRole("status").filter({hasText:"已开启在线审稿"})).toBeVisible(); await page.keyboard.press("Escape");
  await expect(page.locator(".project-link")).toContainText("共享项目 · 只读"); await expect(page.getByRole("button", { name:"导入照片", exact:true })).toBeDisabled(); await page.getByRole("button", {name:"项目列表", exact:true}).click(); await expect(page.locator(".project-card")).toContainText("正常桌面共享"); await page.locator(".project-open").click(); await expect(page.locator(".project-link")).toContainText("共享项目 · 只读"); await page.setViewportSize({width:1280,height:800}); await page.screenshot({path:join(evidence,"desktop-readonly-1280.png")}); assert.deepEqual(pageErrors,[]); await browser.close(); browser=null; await vite.close(); vite=null; await new Promise(resolve=>assets.close(resolve)); assets=null;
  const result = await rpc("get_sharing_status",{projectId:p.id}); assert.equal(result.authority, "shared"); await assert.rejects(rpc('update_photo_decision', { projectId: p.id, expectedRevision: p.revision, photoId: p.photos[0].id, status: 'keep' }), { code: 'AUTHORITY_MISMATCH' });
  const uncertain = await project('云提交后响应丢失'); await assert.rejects(share(uncertain, proxyBase), { code: 'CLOUD_UNAVAILABLE' }); const frozen = await rpc('get_sharing_status', { projectId: uncertain.id }); assert.equal(frozen.authority, 'local-promoting'); assert.equal(frozen.state, 'verifying');
  await stop(); start(); assert.equal((await rpc('get_sharing_status', { projectId: uncertain.id })).authority, 'local-promoting');
  await assert.rejects(share(uncertain, base), { code: 'SHARE_BINDING_CHANGED' });
  await assert.rejects(share(uncertain, proxyBase, 'abort'), { code: 'AUTHORITY_MISMATCH' }); assert.equal((await rpc('get_sharing_status', { projectId: uncertain.id })).authority, 'local-promoting');
  // 云端已产生后续版本，旧迁移回执仍须绑定最初冻结版本。
  const cloud = frozen.remote.projectId; const snapshot = await (await fetch(`${base}/api/v2/projects/${cloud}/snapshot`, { headers: { authorization: `Bearer ${account.token}` } })).json(); const id = randomUUID(); const reply = await fetch(`${base}/api/v2/projects/${cloud}/commands`, { method: 'POST', headers: { authorization: `Bearer ${account.token}`, 'content-type': 'application/json' }, body: JSON.stringify({ version: '2.0', command_id: id, idempotency_key: id, workspace_id: snapshot.workspace_id, project_id: cloud, expected_revision: snapshot.revision, kind: 'photo.set_selection', payload: { photo_ids: [snapshot.snapshot.photos[0].id], selection_state: 'KEEP' } }) }); assert.equal(reply.status, 200);
  assert.equal((await share(uncertain, proxyBase)).authority, 'shared');
  const failed = await project('预览损坏可取消'); await writeFile(failed.photos[0].previewPath, 'broken preview'); await assert.rejects(share(failed), { code: 'MEDIA_POLICY' }); assert.equal((await rpc('get_sharing_status', { projectId: failed.id })).authority, 'local-promoting'); assert.equal((await share(failed, base, 'abort')).authority, 'local'); const updated = await rpc('update_photo_decision', { projectId: failed.id, expectedRevision: failed.revision, photoId: failed.photos[0].id, status: 'keep' }); assert.ok(updated.revision > failed.revision);
  assert.deepEqual(await Promise.all([1,2].map(n => hash(join(directory, 'fixtures', `photo-${n}.jpg`)))), originals);
  const text = (await readFile(join(directory, 'projects.sqlite'))).toString('latin1'); assert.equal(text.includes(password), false); assert.equal(text.includes(account.token), false);
  const resultEvidence = { passed: true, runtime: 'real Rust harness + NestJS + temporary PostgreSQL', source_photos: 2, source_hashes_unchanged: true, actual_ai_calls: 0, flows: ['preview-copy-export', 'desktop-sharing-form-1440', 'desktop-readonly-1280', 'single-authority-promotion', 'local-write-denial', 'commit-response-loss', 'native-process-restart', 'endpoint-rebind-denial', 'committed-cancel-denial', 'receipt-original-revision', 'safe-staging-abort'] }; await writeFile(join(evidence, 'native-promotion.json'), JSON.stringify(resultEvidence, null, 2)); console.log(JSON.stringify(resultEvidence));
} finally { await browser?.close(); await vite?.close(); if(assets) await new Promise(resolve=>assets.close(resolve)); await stop(); await new Promise(resolve => proxy.close(resolve)); await api.close(); await pool.end(); }
