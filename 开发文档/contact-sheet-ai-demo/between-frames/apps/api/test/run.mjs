import { mkdtemp, mkdir, access } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { createServer } from 'node:net';
const exec = promisify(execFile);
const directory = await mkdtemp(join(tmpdir(), 'focalroom-pg-tests-'));
let bin = process.env.FOCALROOM_TEST_PG_BIN;
if (!bin) { try { await access('/opt/homebrew/opt/postgresql@15/bin/initdb'); bin = '/opt/homebrew/opt/postgresql@15/bin'; } catch { bin = (await exec('pg_config', ['--bindir'])).stdout.trim(); } }
const data = join(directory, 'data'), socket = join(directory, 'socket'); await mkdir(socket);
const port = await new Promise(resolve => { const socket = createServer(); socket.listen(0, '127.0.0.1', () => { const port = socket.address().port; socket.close(() => resolve(port)); }); });
let started = false;
try {
  await exec(join(bin, 'initdb'), ['-D', data, '-U', 'focalroom', '--auth=trust', '--no-locale', '-E', 'UTF8']);
  await exec(join(bin, 'pg_ctl'), ['-D', data, '-l', join(directory, 'postgres.log'), '-o', `-h 127.0.0.1 -p ${port} -k ${socket}`, '-w', 'start']); started = true;
  const custom = process.argv.slice(2);
  const suites = ['integration','storage','account-security','magic-login','desktop-login','account-avatar','hosted-ai','hosted-ai-async','billing','min_privilege','agent_worker','metrics','boards'];
  // 每套件独立数据库，避免触发器、角色初始化、清库与指标断言依赖其他文件的执行时序。
  const runs = custom.length ? [{args:custom,database:'postgres'}] : suites.map((name,index)=>({args:['--test',`apps/api/test/${name}.mjs`],database:`focalroom_suite_${index}`}));
  for (const run of runs) {
    if (run.database !== 'postgres') await exec(join(bin,'createdb'),['-h','127.0.0.1','-p',String(port),'-U','focalroom',run.database]);
    const child = spawn(process.execPath,run.args,{stdio:'inherit',env:{...process.env,FOCALROOM_TEST_DATABASE_URL:`postgresql://focalroom@127.0.0.1:${port}/${run.database}`,FOCALROOM_TEST_DIRECTORY:directory,BETWEEN_FRAMES_OPENAI_API_KEY:'',FOCALROOM_AGENT_MODEL_KEY:'',FOCALROOM_DEEPSEEK_API_KEY:'',FOCALROOM_RESEND_API_KEY:'',FOCALROOM_BILLING_ENABLED:'0',FOCALROOM_STRIPE_SECRET_KEY:'',FOCALROOM_STRIPE_WEBHOOK_SECRET:'',FOCALROOM_BILLING_PRICES:''}});
    const code = await new Promise(resolve=>{child.once('exit',code=>resolve(code??1));child.once('error',()=>resolve(1));});
    if (code) process.exitCode = code;
  }
} finally {
  if (started) await exec(join(bin, 'pg_ctl'), ['-D', data, '-w', 'stop', '-m', 'fast']);
  console.log(`Isolated PostgreSQL evidence: ${directory}`);
}
