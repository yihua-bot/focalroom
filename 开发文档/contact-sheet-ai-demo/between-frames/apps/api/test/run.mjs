import { mkdtemp, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { createServer } from 'node:net';
const exec = promisify(execFile);
const directory = await mkdtemp(join(tmpdir(), 'focalroom-pg-tests-'));
const bin = process.env.FOCALROOM_TEST_PG_BIN ?? '/opt/homebrew/opt/postgresql@15/bin';
const data = join(directory, 'data'), socket = join(directory, 'socket'); await mkdir(socket);
const port = await new Promise(resolve => { const socket = createServer(); socket.listen(0, '127.0.0.1', () => { const port = socket.address().port; socket.close(() => resolve(port)); }); });
let started = false;
try {
  await exec(join(bin, 'initdb'), ['-D', data, '-U', 'focalroom', '--auth=trust', '--no-locale', '-E', 'UTF8']);
  await exec(join(bin, 'pg_ctl'), ['-D', data, '-l', join(directory, 'postgres.log'), '-o', `-h 127.0.0.1 -p ${port} -k ${socket}`, '-w', 'start']); started = true;
  const child = spawn(process.execPath, process.argv.slice(2).length ? process.argv.slice(2) : ['--test', 'apps/api/test/integration.mjs'], { stdio: 'inherit', env: { ...process.env, FOCALROOM_TEST_DATABASE_URL: `postgresql://focalroom@127.0.0.1:${port}/postgres`, FOCALROOM_TEST_DIRECTORY: directory, BETWEEN_FRAMES_OPENAI_API_KEY: '' } });
  process.exitCode = await new Promise(resolve => { child.once('exit', code => resolve(code ?? 1)); child.once('error', () => resolve(1)); });
} finally {
  if (started) await exec(join(bin, 'pg_ctl'), ['-D', data, '-w', 'stop', '-m', 'fast']);
  console.log(`Isolated PostgreSQL evidence: ${directory}`);
}
