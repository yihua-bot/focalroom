// 仅在供应商账单确认后操作；无 HTTP 入口，必须显式 --apply。
import {Pool} from "pg";
import {assertSchema} from "../src/database.ts";
import {reconcileAiRequest} from "../src/hosted-ai.ts";
const [email,requestId,tokens,reference,apply] = process.argv.slice(2);
if (!process.env.FOCALROOM_DATABASE_URL || !email || !requestId || !tokens || !reference) throw new Error("用法：node apps/api/tools/ai-reconcile.ts email request-id actual-tokens billing-reference [--apply]；必须显式配置独立数据库");
const pool=new Pool({connectionString:process.env.FOCALROOM_DATABASE_URL,max:1});
try {
  await assertSchema(pool);
  const user=(await pool.query("SELECT id FROM fr_users WHERE email=$1",[email.toLowerCase()])).rows[0];
  if (!user) throw new Error("账户不存在");
  const request=(await pool.query("SELECT request_id,state,reserved,charged,created_at FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2",[user.id,requestId])).rows[0];
  if (!request) throw new Error("请求不存在");
  if (apply === '--apply') console.log(JSON.stringify(await reconcileAiRequest(pool,user.id,requestId,Number(tokens),reference)));
  else console.log(JSON.stringify({applied:false,request,proposed_charge:Number(tokens),billing_reference:reference}));
} finally {await pool.end();}
