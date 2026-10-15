// 仅用于 Playwright：把测试 stdio 接到浏览器，生产应用不包含此服务。
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, realpath } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { createInterface } from 'node:readline';

const root = await realpath(await mkdtemp(join(tmpdir(), 'between-frames-m1-')));
const child = spawn(resolve('src-tauri', process.env.CARGO_TARGET_DIR || 'target', 'debug/m1-harness'), [root], { stdio: ['pipe', 'pipe', 'inherit'], env: { ...process.env, BETWEEN_FRAMES_OPENAI_API_KEY: '' } });
let sequence = 0;
const pending = new Map();
createInterface({ input: child.stdout }).on('line', line => {
  const result = JSON.parse(line);
  pending.get(result.id)?.(result); pending.delete(result.id);
});
function rpc(command, args) {
  return new Promise(resolve => {
    const id = ++sequence;
    pending.set(id, resolve);
    child.stdin.write(JSON.stringify({ id, command, args }) + '\n');
  });
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
    if (url.pathname === '/health') { response.end('ready'); return; }
    if (url.pathname === '/asset') {
      const path = resolve(url.searchParams.get('path') || '');
      if (!path.startsWith(root + '/cache/') || !path.endsWith('.jpg')) { response.writeHead(403); response.end(); return; }
      response.setHeader('Content-Type', 'image/jpeg'); response.end(await readFile(path)); return;
    }
    if (url.pathname !== '/rpc' || request.method !== 'POST' || request.headers['content-type'] !== 'application/json') { response.writeHead(404); response.end(); return; }
    let body = ''; for await (const chunk of request) { body += chunk; if (body.length > 100_000) throw new Error('request too large'); }
    const { command, args } = JSON.parse(body);
    response.setHeader('Content-Type', 'application/json'); response.end(JSON.stringify(await rpc(command, args)));
  } catch (error) { response.writeHead(500); response.end(JSON.stringify({ error: String(error) })); }
});
server.listen(1422, '127.0.0.1', () => process.stdout.write(`测试服务 http://localhost:1422（${root}）\n`));
child.on('exit', () => { server.close(); process.exit(); });
process.on('SIGTERM', () => { child.kill(); server.close(); });
