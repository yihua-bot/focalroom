// 仅供本地评审：由临时 PostgreSQL runner 启动，生成账号与仓库示例照片，不读取用户图库。
import { Pool } from 'pg';
import sharp from 'sharp';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash, randomUUID } from 'node:crypto';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer, mergeConfig } from 'vite';
import { workspaceConfig } from '../../../scripts/workspace-vite.ts';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';
if (!process.env.FOCALROOM_TEST_DATABASE_URL || !process.env.FOCALROOM_TEST_DIRECTORY) throw new Error('请使用 npm run review:shared，禁止传入生产数据库');
const root = fileURLToPath(new URL('../../..', import.meta.url));
const reviewPort = Number(process.env.FOCALROOM_REVIEW_PORT ?? 1430);
if (!Number.isInteger(reviewPort) || reviewPort < 1024 || reviewPort > 65535 ||
    new URL(process.env.FOCALROOM_TEST_DATABASE_URL).hostname !== '127.0.0.1') throw new Error('仅允许隔离回环评审环境');
const origin = `http://127.0.0.1:${reviewPort}`, password = 'review-local-2026';
const mailbox = join(process.env.FOCALROOM_TEST_DIRECTORY, 'review-magic-mail.json');
const pool = new Pool({ connectionString: process.env.FOCALROOM_TEST_DATABASE_URL }); await migrate(pool);
const api = await createApi({ pool, store: new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY, 'review-previews')), origin,
  magicEmail: async (email, code) => {
    if (!['author@focalroom.local', 'editor@focalroom.local'].includes(email)) return false;
    await writeFile(mailbox, JSON.stringify({ email, code, fixture_only: true }), { mode: 0o600 });
    return true;
  },
}); await api.app.listen(0, '127.0.0.1'); const base = await api.app.getUrl();
let vite;
try {
  const call = async (path, token, body) => { const res = await fetch(base + '/api/v2' + path, { method: body ? 'POST' : 'GET', headers: { 'content-type': 'application/json', ...(token ? { authorization: 'Bearer ' + token } : {}) }, ...(body ? { body: JSON.stringify(body) } : {}) }); if (!res.ok) throw new Error('Review fixture setup failed'); return res.json(); };
  const owner = await call('/auth/register', null, { email: 'author@focalroom.local', display_name: '作者 · 本地演练', password });
  const editor = await call('/auth/register', null, { email: 'editor@focalroom.local', display_name: '编辑 · 本地演练', password });
  const workspace = (await call('/workspaces', owner.token))[0].id;
  const images = [];
  for (let i = 1; i <= 8; i++) { const bytes = await readFile(resolve(root, '../assets', `frame_${String(i).padStart(2, '0')}.jpg`)); images.push({ id: 'demo-frame-' + i, filename: `Frame_${String(i).padStart(2, '0')}.jpg`, source_sha256: createHash('sha256').update(bytes).digest('hex'), selection: i <= 3 ? 'KEEP' : 'UNDECIDED', bytes }); }
  const manifest = { workspace_id: workspace, local_project_id: 'local-review-demo', promotion_id: randomUUID(), title: '在街角，重新看见日常', from_revision: 0, snapshot: { photos: images.map(({ bytes: _bytes, ...p }) => { void _bytes; return p; }), sequences: [{ id: 'primary', title: '作品序列', items: images.slice(0,3).map(p => ({ id: 'item-' + p.id, photo_id: p.id })) }] }, media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false } };
  const start = await call('/promotions', owner.token, manifest);
  for (const image of images) { const bytes = await sharp(image.bytes).resize({ width: 1400, height: 1400, fit: 'inside', withoutEnlargement: true }).jpeg({ quality: 88 }).toBuffer(); const response = await fetch(`${base}/api/v2/projects/${start.project_id}/previews/${image.id}`, { method: 'PUT', headers: { authorization: 'Bearer ' + owner.token, 'content-type': 'image/jpeg' }, body: bytes }); if (!response.ok) throw new Error('Preview fixture failed'); }
  await call(`/projects/${start.project_id}/finalize`, owner.token, { manifest_digest: start.manifest_digest });
  const invitation = await call(`/workspaces/${workspace}/invitations`, owner.token, { email: editor.user.email, role: 'REVIEWER', project_id: start.project_id }); await call('/invitations/accept', editor.token, { token: new URL(invitation.invite_url).hash.split('=')[1] });
  const command = (kind, payload) => { const id = randomUUID(); return { version: '2.0', command_id: id, idempotency_key: id, workspace_id: workspace, project_id: start.project_id, expected_revision: 0, kind, payload }; };
  await call(`/projects/${start.project_id}/commands`, editor.token, command('comment.add', { body: '这是生成账户的协作演练批注，不是真人专业评审。可以试试回复、标记解决，或切换到提案逐项审阅。', anchor: { type: 'photo', photo_id: images[0].id, x: .58, y: .44 } }));
  const proposal = command('proposal.create', { title: '演练提案：给序列加入一次停顿', base_revision: 0, operations: [{ op: 'add', operation_id: 'demo-add', photo_id: images[4].id, to_index: 3, reason: '流程演练，不计审美偏好证据' }] }); await call(`/projects/${start.project_id}/commands`, editor.token, proposal); await call(`/projects/${start.project_id}/commands`, editor.token, command('proposal.submit', { proposal_id: proposal.command_id, expected_proposal_revision: 0 }));
  vite = await createServer(mergeConfig(workspaceConfig(resolve(root, 'apps/web'), reviewPort), { configFile: false, server: { proxy: { '/api/v2': { target: base, ws: true } } } })); await vite.listen();
  console.log(JSON.stringify({ review_url: origin + '/shared#project=' + start.project_id, api_url: base, accounts: [owner.user.email, editor.user.email], password, generated_fixture_only: true, actual_ai_calls: 0, review_magic_mailbox: mailbox, database_directory: process.env.FOCALROOM_TEST_DIRECTORY }));
  await new Promise(resolve => { process.once('SIGTERM', resolve); process.once('SIGINT', resolve); });
} finally { await vite?.close(); await api.close(); await pool.end(); }
