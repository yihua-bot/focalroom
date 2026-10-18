// Worker 进程入口：node workers/agent/src/main.ts
// 环境变量：FOCALROOM_DATABASE_URL（应用角色）、FOCALROOM_AGENT_MODEL_KEY 等。
import { Pool } from 'pg';
import { startWorkerLoop } from './index.ts';

const url = process.env.FOCALROOM_DATABASE_URL;
if (!url) {
  console.error('[agent-worker] 缺少 FOCALROOM_DATABASE_URL');
  process.exit(1);
}
const pool = new Pool({ connectionString: url, max: 2 });
console.log('[agent-worker] 启动；轮询间隔', process.env.FOCALROOM_WORKER_INTERVAL_MS ?? 2000, 'ms');
await startWorkerLoop(pool, Number(process.env.FOCALROOM_WORKER_INTERVAL_MS ?? 2000));
await pool.end();
