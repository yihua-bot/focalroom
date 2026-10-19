// 默认只读；--apply 仅供管理员在确认独立数据库后手动执行，不提供公开充值接口。
import {Pool} from "pg";
import {grantAiTokens} from "../src/hosted-ai.ts";
import {assertSchema} from "../src/database.ts";
const [email,grantId,amount,reason,apply] = process.argv.slice(2);
if (!process.env.FOCALROOM_DATABASE_URL) throw new Error("必须显式指定独立数据库连接");
if (!email || (grantId && (!amount || !reason))) throw new Error("用法：node apps/api/tools/ai-credit.ts <email> [grant-id tokens reason --apply]");
const pool = new Pool({connectionString:process.env.FOCALROOM_DATABASE_URL,max:1});
try {
  await assertSchema(pool);
  const user = (await pool.query("SELECT id FROM fr_users WHERE email=$1",[email.toLowerCase()])).rows[0];
  if (!user) throw new Error("账户不存在");
  if (grantId) {
    if (apply !== "--apply") throw new Error("未写入；请核对分配参数后添加 --apply");
    await grantAiTokens(pool,user.id,grantId,Number(amount),reason);
  }
  const account=(await pool.query("SELECT granted,spent,reserved FROM fr_ai_accounts WHERE user_id=$1",[user.id])).rows[0] ?? null;
  const pending=(await pool.query("SELECT request_id,state,reserved,charged,created_at FROM fr_ai_requests WHERE user_id=$1 AND state IN ('PENDING','UNKNOWN') ORDER BY created_at",[user.id])).rows;
  console.log(JSON.stringify({account,pending}));
} finally {await pool.end();}
