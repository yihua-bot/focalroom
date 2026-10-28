import assert from 'node:assert/strict';
import { mkdir, writeFile, readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { randomUUID } from 'node:crypto';
import { createServer as netServer } from 'node:net';
import { Pool } from 'pg';
import sharp from 'sharp';
import { chromium, expect } from '@playwright/test';
import { createServer, mergeConfig } from 'vite';
import { workspaceConfig } from '../../../scripts/workspace-vite.ts';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';
const root = fileURLToPath(new URL('../../..', import.meta.url));
const evidence = resolve(root, '../docs/evidence/v2-m4-m5-postgres'); await mkdir(evidence, { recursive: true });
const port = await new Promise(resolve => { const s = netServer(); s.listen(0, '127.0.0.1', () => { const p = s.address().port; s.close(() => resolve(p)); }); });
const origin = `http://127.0.0.1:${port}`;
const pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL }); await migrate(pool);
const deliveredCodes = new Map();
const api = await createApi({ magicEmail: async (email, code) => { deliveredCodes.set(email, code); return true; }, pool, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'web-previews')), origin }); await api.app.listen(0, '127.0.0.1');
const base = await api.app.getUrl();
const vite = await createServer(mergeConfig(workspaceConfig(resolve(root, 'apps/web'), port), { configFile: false, server: { proxy: { '/api/v2': { target: base, ws: true } } } })); await vite.listen();
const browser = await chromium.launch(); const contexts = []; const errors = [], remotes = [];
try {
  const register = async email => { const res = await fetch(`${base}/api/v2/auth/register`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email, display_name: email.split('@')[0], password: 'web-password-123' }) }); assert.equal(res.status, 200); return res.json(); };
  const owner = await register('photographer@web.test'); await register('editor@web.test'); const viewer = await register('viewer@web.test');
  const workspace = (await (await fetch(`${base}/api/v2/workspaces`, { headers: { authorization: `Bearer ${owner.token}` } })).json())[0].id;
  const jpegPaths = [];
  for (const [i, color] of ['#286179', '#b48453', '#596b51'].entries()) { const path = join(process.env.FOCALROOM_TEST_DIRECTORY, `photo-${i + 1}.preview.jpg`); await sharp({ create: { width: 1000, height: i === 1 ? 1400 : 700, channels: 3, background: color } }).composite([{ input: Buffer.from(`<svg width="1000" height="700"><text x="60" y="180" fill="white" font-size="88">FOCALROOM 0${i + 1}</text><circle cx="640" cy="420" r="160" fill="white" opacity=".16"/></svg>`), top: 0, left: 0 }]).jpeg().toFile(path); jpegPaths.push(path); }
  const manifest = { workspace_id: workspace, local_project_id: 'local-journey', promotion_id: randomUUID(), title: '海岸的间隙', from_revision: 0, snapshot: { photos: [1,2,3].map(n => ({ id: `photo-${n}`, filename: `Coast_${n}.jpg`, source_sha256: 'a'.repeat(64), selection: 'UNDECIDED' })), sequences: [{ id: 'sequence', title: '作品序列', items: [{ id: 'item-1', photo_id: 'photo-1' }] }] }, media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false } };
  const manifestPath = join(process.env.FOCALROOM_TEST_DIRECTORY, 'manifest.json'); await writeFile(manifestPath, JSON.stringify(manifest));
  const newPage = async () => { const context = await browser.newContext({ locale: 'zh-CN', viewport: { width: 1440, height: 960 } }); contexts.push(context); const page = await context.newPage(); page.on('pageerror', error => errors.push(error.message)); page.on('request', req => { const url = new URL(req.url()); if (['http:', 'https:'].includes(url.protocol) && url.hostname !== '127.0.0.1') remotes.push(url.origin); }); return page; };
  const login = async (page, email, url = '/shared') => {
    await page.goto(origin + url);
    await page.getByLabel('邮箱', { exact: true }).fill(email);
    await page.getByRole('button', { name: '发送验证码', exact: true }).click();
    await expect(page.getByLabel('验证码', { exact: true })).toBeVisible();
    assert.match(deliveredCodes.get(email) ?? '', /^\d{6}$/);
    await page.getByLabel('验证码', { exact: true }).fill(deliveredCodes.get(email));
    await page.getByRole('button', { name: '验证并登录', exact: true }).click();
  };
  const a = await newPage(); await a.goto(origin + '/shared'); await expect(a.getByRole('heading', { name: '登录 FocalRoom' })).toBeVisible(); await a.screenshot({ path: join(evidence, 'login-1440.png'), fullPage: true });
  await login(a, 'photographer@web.test'); await a.getByRole('button', { name: '导入共享预览包' }).click(); await a.getByLabel('项目清单文件').setInputFiles(manifestPath); await a.getByLabel('照片预览文件').setInputFiles(jpegPaths); await a.getByRole('checkbox').check(); await a.getByRole('button', { name: '建立共享副本' }).click(); await expect(a.getByRole('heading', { name: '海岸的间隙' })).toBeVisible(); await expect(a.getByText('实时连接', { exact: true })).toBeVisible();
  await expect.poll(() => a.getByRole('img', { name: '检查 Coast_1.jpg' }).evaluate(img => img.naturalWidth)).toBe(1000);
  await a.getByRole('button', { name: '权限', exact: true }).click(); await a.getByLabel('受邀邮箱').fill('editor@web.test'); await a.getByRole('button', { name: '生成私密邀请' }).click(); const inviteUrl = await a.getByLabel('邀请链接').inputValue();
  const b = await newPage(); await login(b, 'editor@web.test', new URL(inviteUrl).pathname + new URL(inviteUrl).hash); await b.getByRole('button', { name: '接受邀请' }).click(); await expect(b.getByText('实时连接', { exact: true })).toBeVisible(); await expect.poll(() => b.getByRole('img', { name: '检查 Coast_1.jpg' }).evaluate(img => img.naturalWidth)).toBe(1000);
  await b.getByRole('img', { name: '检查 Coast_1.jpg' }).click({ position: { x: 160, y: 110 } }); await b.getByLabel('添加批注', { exact: true }).fill('这张适合作为开场，建议与暖色照片形成间隔。', { timeout: 5000 }); await b.getByRole('button', { name: '发布批注' }).click();
  await a.getByRole('button', { name: '批注', exact: true }).click(); await expect(a.getByText('这张适合作为开场，建议与暖色照片形成间隔。', { exact: true })).toBeVisible();
  await b.getByRole('button', { name: '提案', exact: true }).click(); await b.getByRole('button', { name: '查看 Coast_2.jpg' }).click(); await b.getByLabel('提案标题').fill('以冷暖间隔连接海岸'); await b.getByRole('button', { name: '追加到序列' }).click(); await b.getByRole('button', { name: '保存提案草案' }).click(); await expect(b.getByRole('heading', { name: '以冷暖间隔连接海岸' })).toBeVisible(); await a.getByRole('button', { name: '提案', exact: true }).click(); await expect(a.getByRole('heading', { name: '以冷暖间隔连接海岸' })).toHaveCount(0); await b.getByRole('button', { name: '继续编辑草案' }).click(); await b.getByRole('button', { name: '查看 Coast_3.jpg' }).click(); await b.getByRole('button', { name: '追加到序列' }).click(); await b.getByRole('button', { name: '提交编辑提案' }).click(); await expect(b.getByRole('heading', { name: '以冷暖间隔连接海岸' })).toBeVisible();
  await a.getByRole('button', { name: '提案', exact: true }).click(); await expect(a.getByRole('heading', { name: '以冷暖间隔连接海岸' })).toBeVisible(); await a.getByRole('button', { name: '预览差异', exact: true }).click(); await expect(a.locator('.fr-diff')).toContainText('全量采纳后序列：Coast_1.jpg、Coast_2.jpg、Coast_3.jpg'); await expect(a.getByText('v0 · OWNER', { exact: true })).toBeVisible();
  for (const element of [a.getByRole('heading', { name: '编辑提案', exact: true }), a.getByRole('button', { name: '刷新差异', exact: true })]) {
    const contrast = await element.evaluate(node => {
      const rgb = value => (value.match(/[\d.]+/g) || []).map(Number);
      const style = element => element.ownerDocument.defaultView.getComputedStyle(element);
      const luminance = color => rgb(color).slice(0, 3).map(value => { const n = value / 255; return n <= .04045 ? n / 12.92 : ((n + .055) / 1.055) ** 2.4; }).reduce((sum, n, index) => sum + n * [.2126, .7152, .0722][index], 0);
      let background = node;
      while (background.parentElement && rgb(style(background).backgroundColor)[3] === 0) background = background.parentElement;
      const light = luminance(style(node).color), dark = luminance(style(background).backgroundColor);
      return (Math.max(light, dark) + .05) / (Math.min(light, dark) + .05);
    });
    assert.ok(contrast >= 4.5, 'Review heading and active action text must remain readable on their actual backgrounds');
  }
 await a.screenshot({ path: join(evidence, 'proposal-diff-1440.png'), fullPage: true }); await a.locator('.fr-proposal input[type=checkbox]').nth(1).uncheck(); await a.getByRole('button', { name: '批准选中改动' }).click(); await expect(a.getByText('v1 · OWNER', { exact: true })).toBeVisible(); await expect(b.getByText('v1 · REVIEWER', { exact: true })).toBeVisible(); await expect(a.locator('.fr-diff')).toContainText('作品已有更新，请以当前作品和审稿结果为准。'); await expect(a.locator('.fr-diff')).not.toContainText('全量采纳后序列');
  await a.getByRole('button', { name: '批注', exact: true }).click(); await a.getByLabel('私人笔记', { exact: true }).fill('作者自己的节奏笔记'); await expect(a.getByLabel('私人笔记', { exact: true })).toHaveValue('作者自己的节奏笔记'); await expect(b.getByRole('button', { name: '批准选中改动' })).toHaveCount(0);
  await a.screenshot({ path: join(evidence, 'review-1440.png'), fullPage: true }); await a.setViewportSize({ width: 1280, height: 800 }); await a.screenshot({ path: join(evidence, 'review-1280.png'), fullPage: true }); assert.equal(await a.evaluate(() => globalThis.document.documentElement.scrollWidth > globalThis.innerWidth), false);
  await a.reload(); await expect(a.getByRole('heading', { name: '海岸的间隙', exact: true })).toBeVisible(); await expect(a.getByText('v1 · OWNER', { exact: true })).toBeVisible(); await expect(a.getByLabel('私人笔记', { exact: true })).toHaveValue('作者自己的节奏笔记');
  await a.getByRole('button', { name: '版本', exact: true }).click(); await expect(a.locator('.fr-version strong').filter({ hasText: 'v1' })).toBeVisible(); await a.locator('.fr-version').filter({ hasText: 'v0' }).getByRole('button', { name: '查看版本' }).click(); await expect(a.getByText('正在查看 v0 · 只读', { exact: true })).toBeVisible(); await a.screenshot({ path: join(evidence, 'history-1280.png'), fullPage: true });
  const projects = await (await fetch(`${base}/api/v2/projects`, { headers: { authorization: `Bearer ${owner.token}` } })).json(); const id = projects[0].id;
  // 第三个独立浏览器会话，验证 Viewer 的产品入口为只读。
  const inviteViewer = await (await fetch(`${base}/api/v2/workspaces/${workspace}/invitations`, { ...{ method: 'POST', headers: { authorization: `Bearer ${owner.token}`, 'content-type': 'application/json' } }, body: JSON.stringify({ email: viewer.user.email, role: 'VIEWER', project_id: id }) })).json();
  const c = await newPage(); const viewerUrl = new URL(inviteViewer.invite_url); await login(c, viewer.user.email, viewerUrl.pathname + viewerUrl.hash); await c.getByRole('button', { name: '接受邀请' }).click(); await expect(c.getByText('v1 · VIEWER', { exact: true })).toBeVisible(); await expect(c.getByRole('button', { name: '发布批注' })).toBeDisabled();
  await a.getByRole('button', { name: '权限', exact: true }).click(); const grant = a.locator('.fr-grant').filter({ hasText: 'editor@web.test' }); await grant.getByRole('button', { name: '撤销访问' }).click(); await expect(b.getByRole('alert')).toContainText('访问已撤销'); await expect(b.locator('.fr-review')).toHaveCount(0); await b.screenshot({ path: join(evidence, 'revoked-1440.png'), fullPage: true });
  // 51 张生成预览验证按页获取与渲染，不一次加载完整图库。
  const largeManifest = { ...manifest, promotion_id: randomUUID(), local_project_id: "paging", title: "分页边界演练", snapshot: { photos: Array.from({length:51},(_,i)=>({id:"large-"+(i+1),filename:"Grid_"+(i+1)+".jpg",source_sha256:"b".repeat(64),selection:"UNDECIDED"})),sequences:[{id:"primary",title:"主序列",items:[{id:"large-first",photo_id:"large-1"}]}] } };
  const bearerHeaders={authorization:"Bearer "+owner.token,"content-type":"application/json"};
  const large=await (await fetch(base+"/api/v2/promotions",{method:"POST",headers:bearerHeaders,body:JSON.stringify(largeManifest)})).json();
  for(const photo of largeManifest.snapshot.photos) { const response=await fetch(base+"/api/v2/projects/"+large.project_id+"/previews/"+photo.id,{method:"PUT",headers:{authorization:"Bearer "+owner.token,"content-type":"image/jpeg"},body:await readFile(jpegPaths[0])});assert.equal(response.status,200); }
  assert.equal((await fetch(base+"/api/v2/projects/"+large.project_id+"/finalize",{method:"POST",headers:bearerHeaders,body:JSON.stringify({manifest_digest:large.manifest_digest})})).status,200);
  await a.goto(origin+"/shared#project="+large.project_id);await expect(a.getByRole("heading",{name:"分页边界演练",exact:true})).toBeVisible();await expect(a.locator(".fr-photo-grid button")).toHaveCount(48);await a.getByRole("button",{name:"下一页照片",exact:true}).click();await expect(a.locator(".fr-photo-grid button")).toHaveCount(3);await expect(a.getByRole("button",{name:"查看 Grid_51.jpg",exact:true})).toBeVisible();await expect.poll(() => a.getByRole("img", { name: "检查 Grid_1.jpg", exact: true }).evaluate(img => img.naturalWidth)).toBe(1000);await expect.poll(() => a.getByRole("img", { name: "Grid_51.jpg", exact: true }).evaluate(img => img.naturalWidth)).toBe(1000);await a.screenshot({path:join(evidence,"grid-page-1280.png"),fullPage:true});
  await expect(a).toHaveURL(origin + '/shared#project=' + large.project_id);
  await a.getByRole('button', { name: '← 项目', exact: true }).click();
  await expect(a.getByRole('heading', { name: '共享项目', exact: true })).toBeVisible();
  await expect(a).toHaveURL(origin + '/shared');
  await a.goBack(); await expect(a.getByRole('heading', { name: '分页边界演练', exact: true })).toBeVisible();
  await a.reload(); await expect(a.getByRole('heading', { name: '分页边界演练', exact: true })).toBeVisible();
  await expect(a.locator('.fr-photo-grid button')).toHaveCount(48);
  assert.deepEqual(errors, []); assert.deepEqual(remotes, []);
  const stats = { passed: true, users: 3, browser_contexts: 3, real_postgres: true, images: 54, paging_project: 51, maximum_grid_nodes: 48, actual_ai_calls: 0, original_uploads: 0, page_errors: errors, remote_origins: remotes, flows: ['login', 'preview-copy-import', 'invitation', 'photo-anchor', 'private-draft-save', 'owned-draft-revision', 'reviewer-proposal', 'proposal-diff-preview-without-write', 'review-text-contrast', 'stale-preview-hidden-on-update', 'partial-owner-approval', 'immutable-history', 'Yjs-private-note-reload', 'Viewer-readonly', 'live-revocation', '51-photo-pagination'] };
  await writeFile(join(evidence, 'web-journey.json'), JSON.stringify(stats, null, 2)); console.log(JSON.stringify(stats));
  assert.ok((await readFile(join(evidence, 'review-1440.png'))).length > 1000);
} finally { for (const context of contexts) await context.close(); await browser.close(); await vite.close(); await api.close(); await pool.end(); }
