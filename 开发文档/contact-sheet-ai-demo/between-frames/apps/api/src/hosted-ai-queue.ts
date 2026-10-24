import { createCipheriv, createDecipheriv, randomBytes, randomUUID } from 'node:crypto';
import type { Pool, PoolClient } from 'pg';
import { z } from 'zod';
import { digest, transaction } from './database.ts';
import { assert, ApiError } from './errors.ts';

export const QUEUE_CONSENT = 'encrypted-queue-v1';
export const QUEUE_RETENTION_SECONDS = 600;
export const QUEUE_MAX_JOBS = 256;
export const QUEUE_MAX_BYTES = 128 * 1024 * 1024;
export type QueueConfig = { activeKeyId: string; keys: Record<string, string> };
export type QueueInput = {userId:string;requestId:string;sessionDigest:string;hash:string;images:number;body:unknown};
type QueueMetadata = {user_id:string;request_id:string;session_digest:string;body_hash:string;policy:string;input_hash:string;key_id:string;images:number};
export type QueuedRequest = {userId:string;requestId:string;body:unknown;images:number;reservation:number;dispatchId:string};

export function queueConfigFromEnv(): QueueConfig | undefined {
  if (process.env.FOCALROOM_AI_QUEUE_ENABLED !== '1') return undefined;
  try {
    return validateQueueConfig({activeKeyId:process.env.FOCALROOM_AI_QUEUE_ACTIVE_KEY_ID,keys:JSON.parse(process.env.FOCALROOM_AI_QUEUE_KEYS ?? '{}')});
  } catch {throw new ApiError(500,'AI_QUEUE_CONFIG_INVALID','分析恢复服务配置不完整');}
}
export function validateQueueConfig(value:unknown):QueueConfig {
  const config=z.strictObject({activeKeyId:z.string().regex(/^[a-zA-Z0-9_-]{1,64}$/),keys:z.record(z.string().regex(/^[a-zA-Z0-9_-]{1,64}$/),z.string().regex(/^[A-Za-z0-9+/]{43}=$/))}).parse(value);
  assert(Object.keys(config.keys).length<=8 && Object.prototype.hasOwnProperty.call(config.keys,config.activeKeyId),500,'AI_QUEUE_CONFIG_INVALID','分析恢复服务配置不完整');
  for(const key of Object.values(config.keys)) assert(Buffer.from(key,'base64').length===32 && Buffer.from(key,'base64').toString('base64')===key,500,'AI_QUEUE_CONFIG_INVALID','分析恢复服务配置不完整');
  return {activeKeyId:config.activeKeyId,keys:{...config.keys}};
}
function aad(row:QueueMetadata):Buffer {
  return Buffer.from(JSON.stringify(['focalroom-ai-queue-v1',row.user_id,row.request_id,row.session_digest,row.body_hash,row.policy,row.input_hash,row.key_id,row.images]));
}
export function sealQueueInput(config:QueueConfig,input:QueueInput,policy:string) {
  const plaintext=Buffer.from(JSON.stringify(input.body));
  assert(plaintext.length<=16*1024*1024,413,'AI_INPUT_TOO_LARGE','分析内容超过大小上限');
  const inputHash=digest(plaintext),nonce=randomBytes(12),keyId=config.activeKeyId;
  const metadata:QueueMetadata={user_id:input.userId,request_id:input.requestId,session_digest:input.sessionDigest,body_hash:input.hash,policy,input_hash:inputHash,key_id:keyId,images:input.images};
  const cipher=createCipheriv('aes-256-gcm',Buffer.from(config.keys[keyId],'base64'),nonce,{authTagLength:16});
  cipher.setAAD(aad(metadata));
  const encrypted=Buffer.concat([cipher.update(plaintext),cipher.final()]);
  return {keyId,inputHash,payload:Buffer.concat([Buffer.from([1]),nonce,cipher.getAuthTag(),encrypted])};
}
function openQueueInput(config:QueueConfig,row:QueueMetadata & {payload:Buffer}):unknown {
  const bytes=row.payload,key=Object.prototype.hasOwnProperty.call(config.keys,row.key_id)?config.keys[row.key_id]:undefined;
  assert(key && bytes.length>=29 && bytes.length<=16*1024*1024+29 && bytes[0]===1,500,'AI_QUEUE_INPUT_UNAVAILABLE','无法恢复这次分析');
  const cipher=createDecipheriv('aes-256-gcm',Buffer.from(key,'base64'),bytes.subarray(1,13),{authTagLength:16});
  cipher.setAAD(aad(row));cipher.setAuthTag(bytes.subarray(13,29));
  const text=Buffer.concat([cipher.update(bytes.subarray(29)),cipher.final()]);
  assert(digest(text)===row.input_hash,500,'AI_QUEUE_INPUT_UNAVAILABLE','无法恢复这次分析');
  return JSON.parse(text.toString('utf8'));
}
export async function enqueue(tx:PoolClient,config:QueueConfig,request:QueueInput,policy:string) {
  const sealed=sealQueueInput(config,request,policy);
  // 跨实例串行化新增容量；领取/过期只会减少正文占用，不需要持此锁等待模型。
  await tx.query('SELECT pg_advisory_xact_lock(759362122)');
  const size=(await tx.query("SELECT count(*)::int AS jobs,coalesce(sum(octet_length(payload)),0)::bigint AS bytes FROM fr_ai_jobs WHERE phase='QUEUED'")).rows[0];
  assert(size.jobs<QUEUE_MAX_JOBS && Number(size.bytes)+sealed.payload.length<=QUEUE_MAX_BYTES,429,'AI_QUEUE_BUSY','正在处理较多照片，请稍后再试');
  await tx.query("INSERT INTO fr_ai_jobs(user_id,request_id,session_digest,key_id,input_hash,images,payload,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,now()+interval '10 minutes')",
    [request.userId,request.requestId,request.sessionDigest,sealed.keyId,sealed.inputHash,request.images,sealed.payload]);
}

// 所有路径按账户→请求→队列加锁，不持数据库事务等待模型网络。
async function cancelUnsent(tx:PoolClient,user:string,id:string,code:string) {
  const updated=await tx.query("UPDATE fr_ai_requests SET state='REJECTED',charged=0,error_code=$3,finished_at=now() WHERE user_id=$1 AND request_id=$2 AND state='PENDING' AND charged IS NULL RETURNING reserved",[user,id,code]);
  if(updated.rowCount) await tx.query('UPDATE fr_ai_accounts SET reserved=reserved-$2 WHERE user_id=$1',[user,updated.rows[0].reserved]);
  await tx.query("UPDATE fr_ai_jobs SET phase='DONE',payload=NULL WHERE user_id=$1 AND request_id=$2 AND phase='QUEUED'",[user,id]);
}
export async function claimQueued(pool:Pool,config:QueueConfig,policy:string,validateBody:(body:unknown)=>unknown):Promise<QueuedRequest|undefined> {
  return transaction(pool,async tx=>{
    const candidate=(await tx.query("SELECT q.user_id,q.request_id FROM fr_ai_jobs q JOIN fr_ai_accounts a ON a.user_id=q.user_id WHERE q.phase='QUEUED' ORDER BY q.created_at,q.user_id,q.request_id FOR UPDATE OF a SKIP LOCKED LIMIT 1")).rows[0];
    if(!candidate)return undefined;
    const user=candidate.user_id,id=candidate.request_id;
    const request=(await tx.query('SELECT * FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2 FOR UPDATE',[user,id])).rows[0];
    const row=(await tx.query("SELECT *,expires_at>now() AS unexpired FROM fr_ai_jobs WHERE user_id=$1 AND request_id=$2 AND phase='QUEUED' FOR UPDATE",[user,id])).rows[0];
    if(!row)return undefined;
    if(request.state!=='PENDING' || request.charged!==null) {
      await tx.query("UPDATE fr_ai_jobs SET phase='DONE',payload=NULL WHERE user_id=$1 AND request_id=$2",[user,id]);return undefined;
    }
    const session=(await tx.query('SELECT s.user_id,u.email_verified_at FROM fr_sessions s JOIN fr_users u ON u.id=s.user_id WHERE s.digest=$1 AND s.user_id=$2 AND s.expires_at>now() FOR SHARE OF s',[row.session_digest,user])).rows[0];
    if(!row.unexpired || !session?.email_verified_at || request.policy!==policy) {
      await cancelUnsent(tx,user,id,!row.unexpired?'AI_QUEUE_EXPIRED':'AI_QUEUE_PERMISSION_REVOKED');return undefined;
    }
    let body:unknown;
    try {body=validateBody(openQueueInput(config,{...row,body_hash:request.body_hash,policy:request.policy}));}
    catch {await cancelUnsent(tx,user,id,'AI_QUEUE_INPUT_UNAVAILABLE');return undefined;}
    const dispatchId=randomUUID();
    // 一旦提交发送标记，即使进程随后退出，也不能重新领取；正文同时清除。
    await tx.query("UPDATE fr_ai_jobs SET phase='DISPATCHING',attempts=attempts+1,dispatched_at=now(),dispatch_id=$3,payload=NULL WHERE user_id=$1 AND request_id=$2 AND phase='QUEUED' AND attempts=0",[user,id,dispatchId]);
    return {userId:user,requestId:id,body,images:row.images,reservation:request.reserved,dispatchId};
  });
}
export async function expireQueued(pool:Pool) {
  // 分批处理，避免清理占住其他账户。排队过期能证明尚未发送，才释放预留。
  const expired=(await pool.query("SELECT user_id,request_id FROM fr_ai_jobs WHERE phase='QUEUED' AND expires_at<=now() ORDER BY expires_at LIMIT 100")).rows;
  for(const row of expired) await transaction(pool,async tx=>{
    await tx.query('SELECT user_id FROM fr_ai_accounts WHERE user_id=$1 FOR UPDATE',[row.user_id]);
    await tx.query('SELECT request_id FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2 FOR UPDATE',[row.user_id,row.request_id]);
    const current=(await tx.query("SELECT phase,expires_at<=now() AS expired FROM fr_ai_jobs WHERE user_id=$1 AND request_id=$2 FOR UPDATE",[row.user_id,row.request_id])).rows[0];
    if(current?.phase==='QUEUED' && current.expired)await cancelUnsent(tx,row.user_id,row.request_id,'AI_QUEUE_EXPIRED');
  });
  // 发送中断只能标记未知，不能退款或重发。迟到的可信结果仍可原子结算。
  await pool.query("UPDATE fr_ai_jobs SET phase='UNKNOWN' WHERE phase='DISPATCHING' AND dispatched_at<now()-interval '6 minutes'");
}
export async function cancelQueued(pool:Pool,user:string,id:string) {
  return transaction(pool,async tx=>{
    await tx.query('SELECT user_id FROM fr_ai_accounts WHERE user_id=$1 FOR UPDATE',[user]);
    const request=(await tx.query('SELECT request_id,error_code FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2 FOR UPDATE',[user,id])).rows[0];
    assert(request,404,'NOT_FOUND','分析不存在');
    const job=(await tx.query('SELECT phase FROM fr_ai_jobs WHERE user_id=$1 AND request_id=$2 FOR UPDATE',[user,id])).rows[0];
    assert(job,409,'AI_ALREADY_PROCESSING','这次分析已开始，无法撤回已发送的照片');
    if(job.phase==='QUEUED'){await cancelUnsent(tx,user,id,'AI_QUEUE_CANCELLED');return {cancelled:true};}
    return {cancelled:job.phase==='DONE' && request.error_code==='AI_QUEUE_CANCELLED'};
  });
}
