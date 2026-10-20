// 仅用于 Playwright：把测试 stdio 接到浏览器，生产应用不包含此服务。
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, realpath } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { createInterface } from 'node:readline';

const root = await realpath(await mkdtemp(join(tmpdir(), 'between-frames-m1-')));
const fixtureProfile = process.argv.includes('--fixtures=m1-500') ? 'm1-500' : 'standard';
const buildProfile = process.argv.includes('--release') ? 'release' : 'debug';
let sequence = 0;
const pending = new Map();
const originalGrants = new Set();
let restarting = false;
function startHarness() {
  const worker = spawn(resolve('src-tauri', process.env.CARGO_TARGET_DIR || 'target', buildProfile, 'm1-harness'), [root, fixtureProfile], { stdio: ['pipe', 'pipe', 'inherit'], env: { ...process.env, BETWEEN_FRAMES_OPENAI_API_KEY: '' } });
  createInterface({ input: worker.stdout }).on('line', line => {
    const result = JSON.parse(line);
    const request = pending.get(result.id);
    pending.delete(result.id);
    request?.resolve(result);
  });
  worker.on('exit', () => { if (!restarting) { server.close(); process.exit(); } });
  return worker;
}
let child = startHarness();
async function restartHarness() {
  if (restarting) throw new Error('test harness is restarting');
  restarting = true;
  const previousPid = child.pid;
  const pendingAtRestart = pending.size;
  originalGrants.clear();
  try {
    // 页面关闭会取消浏览器请求，但已进入 stdio 的 RPC 仍需完成再关闭旧进程。
    await Promise.all([...pending.values()].map(request => request.promise));
    const stopped = new Promise(resolve => child.once('exit', resolve));
    child.stdin.end();
    await stopped;
    child = startHarness();
    const ready = await rpc('test_fixture_info', {});
    if (!ready.ok) throw new Error('restarted fixtures not ready');
    return { previousPid, pid: child.pid, pendingAtRestart };
  } finally { restarting = false; }
}
function rpc(command, args) {
  const id = ++sequence;
  let complete;
  const promise = new Promise(resolve => { complete = resolve; });
  pending.set(id, { promise, resolve: complete });
  child.stdin.write(JSON.stringify({ id, command, args }) + '\n');
  return promise;
}
const server = createServer(async (request, response) => {
  const origin = request.headers.origin;
  if (origin && origin !== 'http://localhost:1420') { response.writeHead(403); response.end(); return; }
  response.setHeader('Access-Control-Allow-Origin', 'http://localhost:1420');
  response.setHeader('Access-Control-Allow-Headers', 'Content-Type');
  response.setHeader('Access-Control-Allow-Methods', 'GET,POST,OPTIONS');
  if (request.method === 'OPTIONS') { response.writeHead(204); response.end(); return; }
  try {
    const url = new URL(request.url, 'http://localhost:1422');
    if (url.pathname === '/health') {
      if (restarting) { response.writeHead(503); response.end('test harness restarting'); return; }
      const ready = await rpc('test_fixture_info', {}); if (!ready.ok) throw new Error('test fixtures not ready'); response.end('ready'); return;
    }
    if (url.pathname === '/asset') {
      const path = resolve(url.searchParams.get('path') || '');
      if (!(path.startsWith(root + '/cache/') && path.endsWith('.jpg')) && !originalGrants.has(path)) { response.writeHead(403); response.end(); return; }
      response.setHeader('Content-Type', path.toLowerCase().endsWith('.png') ? 'image/png' : 'image/jpeg'); response.end(await readFile(path)); return;
    }
    if (url.pathname !== '/rpc' || request.method !== 'POST' || request.headers['content-type'] !== 'application/json') { response.writeHead(404); response.end(); return; }
    let body = ''; for await (const chunk of request) { body += chunk; if (body.length > 100_000) throw new Error('request too large'); }
    const { command, args } = JSON.parse(body);
    if (restarting) { response.writeHead(503, { 'Content-Type': 'application/json' }); response.end(JSON.stringify({ ok: false, error: { message: 'test harness restarting' } })); return; }
    const result = command === 'test_process_restart'
      ? { ok: true, value: await restartHarness() }
      : await rpc(command, args);
    if (command === 'get_original_photo' && result.ok) originalGrants.add(result.value.path ?? result.value);
    response.setHeader('Content-Type', 'application/json'); response.end(JSON.stringify(result));
  } catch (error) { response.writeHead(500, { 'Content-Type': 'application/json' }); response.end(JSON.stringify({ ok: false, error: { message: String(error) } })); }
});
server.listen(1422, '127.0.0.1', () => process.stdout.write(`测试服务 http://localhost:1422（${root}）\n`));
process.on('SIGTERM', () => { child.kill(); server.close(); });
