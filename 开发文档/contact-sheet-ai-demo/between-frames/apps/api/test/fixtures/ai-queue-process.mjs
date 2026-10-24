// 只由隔离队列测试启动，模型传输只接受本机地址。
import {Pool} from 'pg';
import {AuthService} from '../../src/auth.ts';
import {HostedAiService} from '../../src/hosted-ai.ts';
import {assertSchema} from '../../src/database.ts';
const url=new URL(process.env.FOCALROOM_TEST_QUEUE_MODEL);
if(!['127.0.0.1','localhost'].includes(url.hostname))throw new Error('fixture transport must stay local');
const pool=new Pool({connectionString:process.env.FOCALROOM_TEST_DATABASE_URL});await assertSchema(pool);
const service=new HostedAiService(pool,new AuthService(pool,'http://127.0.0.1:1424'),{
 key:'fixture-provider-secret',initialTokens:0,queue:{activeKeyId:'first',keys:{first:process.env.FOCALROOM_TEST_QUEUE_KEY}},
},body=>fetch(url,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)}));
process.send?.('ready');
process.once('SIGTERM',()=>{void service.drain().then(()=>pool.end()).then(()=>process.exit(0));});
