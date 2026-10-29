import { randomUUID } from "node:crypto";
import type { Pool, PoolClient } from "pg";
import sharp from "sharp";
import { z } from "zod";
import { AuthService, type Headers } from "./auth.ts";
import { digest, transaction } from "./database.ts";
import { assert, ApiError } from "./errors.ts";
import { checkRateLimit } from "./rate-limit.ts";
import { claimQueued, cancelQueued, enqueue, expireQueued, queueConfigFromEnv, validateQueueConfig, QUEUE_CONSENT, QUEUE_RETENTION_SECONDS, type QueueConfig } from './hosted-ai-queue.ts';

// v1 固定收件人和型号；切换供应商/型号必须新增策略与客户端授权身份。
export const HOSTED_AI_POLICY = "deepseek-flash-v1";
export const HOSTED_AI_MODEL = "deepseek-flash";
const MAX_BODY = 16 * 1024 * 1024, MAX_REPLY = 2 * 1024 * 1024;
const REQUEST_TIMEOUT_MS = 300000;
// 超出上游截止时间再留结算余量；进程中断不重放，也不凭超时退款。
const STALE_REQUEST_MS = REQUEST_TIMEOUT_MS + 60000;
const inputSchema = z.strictObject({
  model: z.literal(HOSTED_AI_MODEL),
  input: z.array(z.record(z.string(), z.unknown())).min(1).max(400),
  instructions: z.string().max(100000).optional(),
  tools: z.array(z.record(z.string(), z.unknown())).max(80).optional(),
  text: z.record(z.string(), z.unknown()).optional(),
  max_output_tokens: z.number().int().min(128).max(16000),
  store: z.literal(false),
});
export type HostedAiConfig = { key: string; initialTokens: number; queue?: QueueConfig };
export type HostedAiTransport = (body: unknown, key: string) => Promise<Response>;
type PreparedRequest = {
  userId: string; requestId: string; headers: Headers; sessionDigest: string;
  body: z.infer<typeof inputSchema>; images: number; reservation: number; hash: string;
};
const productionTransport: HostedAiTransport = (body, key) => fetch("https://api.deepseek.com/responses", {
  method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${key}` },
  body: JSON.stringify(body), redirect: "error", signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
});
export function hostedAiConfig(): HostedAiConfig | undefined {
  const key = process.env.FOCALROOM_DEEPSEEK_API_KEY?.trim();
  if (!key) return undefined;
  const initialTokens = Number(process.env.FOCALROOM_AI_INITIAL_TOKENS ?? 1000000);
  assert(Number.isSafeInteger(initialTokens) && initialTokens >= 0 && initialTokens <= 1000000,
    500, "AI_CONFIG_INVALID", "初始 AI 额度配置不合法");
  return { key, initialTokens, queue: queueConfigFromEnv() };
}
async function readReply(response: Response) {
  const reader = response.body?.getReader();
  assert(reader, 502, "AI_RESULT_UNKNOWN", "模型结果无法确认，请勿重复提交");
  const chunks: Uint8Array[] = []; let size = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read(); if (done) break;
      size += value.byteLength;
      assert(size <= MAX_REPLY, 502, "AI_RESULT_UNKNOWN", "模型响应超过上限，请勿重复提交");
      chunks.push(value);
    }
    return JSON.parse(Buffer.concat(chunks).toString("utf8"));
  } finally { await reader.cancel().catch(() => {}); }
}
// 管理端离线分配入口；没有公开充值/赠送接口。grantId 由操作者保存用于幂等重试。
export async function grantAiTokens(pool: Pool, userId: string, grantId: string, tokens: number, reason: string) {
  return transaction(pool, tx => grantAiTokensTx(tx, userId, grantId, tokens, reason));
}

// 支付回执与额度发放共用调用者事务；避免支付已确认但额度漏发。
export async function grantAiTokensTx(tx: PoolClient, userId: string, grantId: string, tokens: number, reason: string) {
  assert(Number.isSafeInteger(tokens) && tokens > 0 && tokens <= 1000000000 && reason.trim().length > 0 && reason.length <= 200 && grantId.length > 0 && grantId.length <= 160,
    400, "VALIDATION_ERROR", "额度分配参数不合法");
    await tx.query("INSERT INTO fr_ai_accounts(user_id) VALUES($1) ON CONFLICT DO NOTHING", [userId]);
    await tx.query("SELECT user_id FROM fr_ai_accounts WHERE user_id=$1 FOR UPDATE", [userId]);
    const old = (await tx.query("SELECT * FROM fr_ai_grants WHERE id=$1", [grantId])).rows[0];
    if (old) {
      assert(old.user_id === userId && old.tokens === tokens && old.reason === reason, 409, "AI_GRANT_CONFLICT", "分配编号已用于其他操作");
      return { applied: false };
    }
    await tx.query("INSERT INTO fr_ai_grants(id,user_id,tokens,reason) VALUES($1,$2,$3,$4)", [grantId,userId,tokens,reason]);
    await tx.query("UPDATE fr_ai_accounts SET granted=granted+$2 WHERE user_id=$1", [userId,tokens]);
    return { applied: true };

}

/// J5 决策（2026-10-28）：免费层每月 100 万 tokens。
/// 按 (userId, 年月) 确定性编号幂等发放——同月重复调用不重复发放，
/// 跨月自动进入新月额度。付费订阅期间不调用（订阅发票发放）。
export async function ensureFreeTierMonthlyGrant(pool: Pool, userId: string, monthlyTokens: number) {
  const monthKey = new Date().toISOString().slice(0, 7);
  return transaction(pool, tx => grantAiTokensTx(tx, userId, `free_tier:${userId}:${monthKey}`, monthlyTokens, `free_tier_monthly:${monthKey}`));
}

// 只供线下核账后的管理操作；活跃请求不能提前退款，流水和账本在同一事务结算。
export async function reconcileAiRequest(pool: Pool, userId: string, requestId: string, charged: number, billingReference: string) {
  assert(Number.isSafeInteger(charged) && charged >= 0 && billingReference.trim().length > 0 && billingReference.length <= 200,
    400,"VALIDATION_ERROR","对账参数不合法");
  return transaction(pool,async tx => {
    await tx.query("SELECT user_id FROM fr_ai_accounts WHERE user_id=$1 FOR UPDATE",[userId]);
    const old = (await tx.query("SELECT * FROM fr_ai_reconciliations WHERE user_id=$1 AND request_id=$2",[userId,requestId])).rows[0];
    if (old) {
      assert(old.charged === charged && old.billing_reference === billingReference,409,"AI_RECONCILIATION_CONFLICT","此请求已按其他记录结算");
      return {applied:false};
    }
    const request = (await tx.query("SELECT *,created_at < now()-interval '10 minutes' AS stale FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2 FOR UPDATE",[userId,requestId])).rows[0];
    const queued=(await tx.query("SELECT phase,dispatched_at<now()-interval '6 minutes' AS stale FROM fr_ai_jobs WHERE user_id=$1 AND request_id=$2 FOR UPDATE",[userId,requestId])).rows[0];
    assert(!queued || (queued.phase!=='QUEUED' && (queued.phase!=='DISPATCHING' || queued.stale)),409,'AI_REQUEST_NOT_RECONCILABLE','分析尚在排队或处理中，不能提前结算');
    assert(request && (request.state === 'UNKNOWN' || (request.state === 'PENDING' && request.stale)),409,"AI_REQUEST_NOT_RECONCILABLE","请求尚在执行或已经结算");
    assert(charged <= request.reserved,409,"AI_RECONCILIATION_OVERFLOW","实耗超过预留，请先人工核对计量异常，不能自动结算");
    await tx.query("INSERT INTO fr_ai_reconciliations(user_id,request_id,charged,billing_reference) VALUES($1,$2,$3,$4)",[userId,requestId,charged,billingReference]);
    await tx.query("UPDATE fr_ai_accounts SET reserved=reserved-$2,spent=spent+$3 WHERE user_id=$1",[userId,request.reserved,charged]);
    await tx.query("UPDATE fr_ai_requests SET state=$3,charged=$4,finished_at=now() WHERE user_id=$1 AND request_id=$2",[userId,requestId,charged ? 'COMPLETED':'REJECTED',charged]);
    await tx.query("UPDATE fr_ai_jobs SET phase='DONE',payload=NULL WHERE user_id=$1 AND request_id=$2",[userId,requestId]);
    return {applied:true};
  });
}

export class HostedAiService {
  private readonly pool: Pool;
  private readonly auth: AuthService;
  private readonly config?: HostedAiConfig;
  private readonly transport: HostedAiTransport;
  private readonly pending = new Set<Promise<void>>();
  private readonly resultCleanup: ReturnType<typeof setInterval>;
  private readonly queueTimer?: ReturnType<typeof setInterval>;
  private queueStopped = false;
  private claiming?: Promise<void>;
  private maintenance?: Promise<void>;
  constructor(pool: Pool, auth: AuthService, config?: HostedAiConfig, transport: HostedAiTransport = productionTransport) {
    this.pool = pool; this.auth = auth; this.config = config ? {...config,queue:config.queue ? validateQueueConfig(config.queue) : undefined} : undefined; this.transport = transport;
    // 异步回执是短期恢复缓存，不是无限期云端对话历史。失败时下一周期重试。
    const cleanup = () => {
      if(this.maintenance)return;
      const work=this.expireResults().catch(()=>{});this.maintenance=work;
      void work.then(()=>{if(this.maintenance===work)this.maintenance=undefined;});
    };
    this.resultCleanup = setInterval(cleanup, 30000);
    this.resultCleanup.unref();
    cleanup();
    if(this.config?.queue){
      this.queueTimer=setInterval(()=>{void this.runQueueOnce().catch(()=>{});},250);
      this.queueTimer.unref();
    }
  }
  async account(headers: Headers) {
    return transaction(this.pool, async tx => {
      const user = await this.auth.authenticate(headers, false, tx);
      const verified = !!(await tx.query("SELECT email_verified_at FROM fr_users WHERE id=$1", [user.id])).rows[0]?.email_verified_at;
      const amount = verified && this.config ? this.config.initialTokens : 0;
      // 未验证邮箱不创建账户，避免先查询额度丢失首次赠额；未启用服务也不消耗赠额机会。
      if (verified && this.config) {
        const created = await tx.query("INSERT INTO fr_ai_accounts(user_id,granted) VALUES($1,$2) ON CONFLICT DO NOTHING RETURNING user_id", [user.id, amount]);
        if (created.rowCount && amount) await tx.query("INSERT INTO fr_ai_grants(id,user_id,tokens,reason) VALUES($1,$2,$3,'initial')", [`initial:${user.id}`,user.id,amount]);
      }
      const row = (await tx.query("SELECT granted,spent,reserved FROM fr_ai_accounts WHERE user_id=$1", [user.id])).rows[0];
      const granted = Number(row?.granted ?? 0), spent = Number(row?.spent ?? 0), reserved = Number(row?.reserved ?? 0);
      return { enabled: !!this.config, verified, policy: HOSTED_AI_POLICY, provider: "DeepSeek", model: HOSTED_AI_MODEL,
        unit: "tokens", granted, spent, reserved, available: granted - spent - reserved,
        recovery: this.config?.queue ? {enabled:true,policy:QUEUE_CONSENT,maxQueueSeconds:QUEUE_RETENTION_SECONDS} : {enabled:false} };
    });
  }
  private async prepare(headers: Headers, input: unknown): Promise<PreparedRequest> {
    const user = await this.auth.authenticate(headers, true);
    assert(this.config, 503, "AI_NOT_CONFIGURED", "FocalRoom AI 暂未开放");
    const quota = await this.account(headers);
    assert(quota.verified,403,"AI_EMAIL_UNVERIFIED","请先验证邮箱");
    const rate = await checkRateLimit(this.pool,`hosted-ai:${user.id}`,30);
    assert(rate.allowed,429,"AI_RATE_LIMITED","AI 请求过于频繁，请稍后再试");
    assert(headers["x-focalroom-ai-policy"] === HOSTED_AI_POLICY, 409, "AI_POLICY_CHANGED", "AI 服务配置已改变，请更新客户端后重新授权");
    const requestId = z.uuid().parse(headers["x-focalroom-request-id"]);
    const body = inputSchema.parse(input);
    const original = JSON.stringify(body);
    assert(Buffer.byteLength(original) <= MAX_BODY, 413, "AI_INPUT_TOO_LARGE", "AI 请求超过大小上限");
    // 只接收本地派生预览，拒绝 URL/file_id/其他输入类型与服务端工具，避免 SSRF 或绕过授权。
    let images = 0;
    const walk = async (value: unknown, depth = 0): Promise<void> => {
      assert(depth <= 30, 400, "VALIDATION_ERROR", "请求嵌套过深");
      if (!value || typeof value !== "object") return;
      if (Array.isArray(value)) { for (const item of value) await walk(item,depth+1); return; }
      const item = value as Record<string, unknown>;
      if (item.type === "input_image") {
        assert(headers["x-focalroom-image-consent"] === "granted", 403, "AI_IMAGE_CONSENT_REQUIRED", "请先授权照片预览发送给 FocalRoom 与 DeepSeek");
        assert(++images <= 20 && Object.keys(item).every(key => ["type","image_url","detail"].includes(key)), 400, "AI_IMAGE_INVALID", "图像输入不合法");
        const match = typeof item.image_url === "string" && /^data:image\/jpeg;base64,([A-Za-z0-9+/]+={0,2})$/.exec(item.image_url);
        assert(match && match[1].length <= 3 * 1024 * 1024, 400, "AI_IMAGE_INVALID", "只接受有界 JPEG 派生预览");
        const bytes = Buffer.from(match[1], "base64");
        try {
          const image = sharp(bytes, {limitInputPixels: 1400 * 1400, animated: false});
          const meta = await image.metadata();
          assert(meta.format === "jpeg" && !!meta.width && !!meta.height && meta.width <= 1400 && meta.height <= 1400, 400, "AI_IMAGE_INVALID", "照片预览不得超过 1400px");
          const sanitized = await image.rotate().jpeg({quality:80}).toBuffer();
          item.image_url = `data:image/jpeg;base64,${sanitized.toString("base64")}`;
          item.detail = "low";
        } catch { throw new ApiError(400,"AI_IMAGE_INVALID","照片预览格式不合法"); }
        return;
      }
      assert(!("file_id" in item) && !("image_url" in item) && !("url" in item),400,"AI_INPUT_INVALID","不支持远端文件或图片链接");
      for (const value of Object.values(item)) await walk(value,depth+1);
    };
    for (const item of body.input) assert([undefined,"message","function_call","function_call_output"].includes(item.type as string | undefined),400,"AI_INPUT_INVALID","不支持该输入类型");
    for (const tool of body.tools ?? []) assert(tool.type === "function",400,"AI_INPUT_INVALID","只支持客户端函数工具");
    await walk(body);
    // 连同属性名、JSON 转义与工具 schema 一起计入文本上界；仅排除已单独预留的预览像素。
    const textBytes = Buffer.byteLength(JSON.stringify(body,(key,value) => key === 'image_url' ? '[JPEG preview]' : value));
    assert(textBytes <= 200000,413,"AI_INPUT_TOO_LARGE","文本上下文超过上限");
    const reservation = textBytes + images * 8192 + body.max_output_tokens + 8192;
    const hash = digest(original);
    return { userId: user.id, requestId, headers, sessionDigest:user.session, body, images, reservation, hash };
  }
  private async reserve(request: PreparedRequest, allowReplay: boolean, durable = false): Promise<boolean> {
    const { userId, requestId, headers, reservation, hash } = request;
    return transaction(this.pool, async tx => {
      // 会话校验与预留同事务；额度行锁串行化跨进程请求，不能超额透支。
      await this.auth.authenticate(headers,true,tx);
      const verified = (await tx.query("SELECT email_verified_at FROM fr_users WHERE id=$1",[userId])).rows[0]?.email_verified_at;
      assert(verified,403,"AI_EMAIL_UNVERIFIED","请先验证邮箱");
      const account = (await tx.query("SELECT * FROM fr_ai_accounts WHERE user_id=$1 FOR UPDATE",[userId])).rows[0];
      const old = (await tx.query("SELECT body_hash,state,policy FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2",[userId,requestId])).rows[0];
      if (old && allowReplay) {
        assert(old.body_hash === hash && old.policy === HOSTED_AI_POLICY,409,"AI_REQUEST_CONFLICT","请求编号已用于其他内容，未重复调用模型");
        return false;
      }
      assert(!old,409,old?.body_hash === hash ? "AI_REQUEST_REPLAY" : "AI_REQUEST_CONFLICT","此请求已处理或结果待确认，未重复调用模型");
      assert(account && Number(account.granted)-Number(account.spent)-Number(account.reserved) >= reservation,402,"AI_QUOTA_EXHAUSTED","AI 额度不足，请联系 FocalRoom");
      const active = Number((await tx.query("SELECT count(*) FROM fr_ai_requests WHERE user_id=$1 AND state='PENDING'",[userId])).rows[0].count);
      assert(active < 2,429,"AI_BUSY","正在处理其他 AI 请求，请稍后再试");
      await tx.query("INSERT INTO fr_ai_requests(user_id,request_id,body_hash,policy,reserved,state) VALUES($1,$2,$3,$4,$5,'PENDING')",[userId,requestId,hash,HOSTED_AI_POLICY,reservation]);
      await tx.query("UPDATE fr_ai_accounts SET reserved=reserved+$2 WHERE user_id=$1",[userId,reservation]);
      if(durable)await enqueue(tx,this.config!.queue!,request,HOSTED_AI_POLICY);
      return true;
    });
  }
  async respond(headers: Headers, input: unknown) {
    const request = await this.prepare(headers, input);
    await this.reserve(request, false);
    return this.execute(request, false);
  }
  async submit(headers: Headers, input: unknown) {
    const durable=headers['x-focalroom-recovery-consent']===QUEUE_CONSENT;
    assert(headers['x-focalroom-recovery-consent']===undefined || durable,400,'AI_QUEUE_CONSENT_INVALID','分析恢复选项已改变，请刷新后重试');
    assert(!durable || this.config?.queue,503,'AI_QUEUE_NOT_AVAILABLE','分析恢复暂不可用，请刷新后重试');
    const request = await this.prepare(headers, input);
    if (await this.reserve(request, true, durable) && !durable) {
      // 仅本次成功预留者执行。跨实例/断线重提只读同一回执，不再发图。
      const work = this.execute(request, true).then(() => {}, () => {});
      this.pending.add(work);
      void work.then(() => this.pending.delete(work));
    }
    return this.job(headers, request.requestId);
  }
  async drain() {
    this.queueStopped=true;
    if(this.queueTimer)clearInterval(this.queueTimer);
    clearInterval(this.resultCleanup);
    await this.claiming?.catch(()=>{});
    await Promise.all([...this.pending]);
    await this.maintenance;
  }
  private async expireResults() {
    await expireQueued(this.pool);
    await this.pool.query("UPDATE fr_ai_requests SET result_json=NULL,error_code='AI_RESULT_EXPIRED' WHERE result_json IS NOT NULL AND finished_at < now()-interval '24 hours'");
  }
  async job(headers: Headers, value: unknown) {
    const user = await this.auth.authenticate(headers, false);
    const requestId = z.uuid().parse(value);
    const rate = await checkRateLimit(this.pool,`hosted-ai-poll:${user.id}`,120);
    assert(rate.allowed,429,"AI_RATE_LIMITED","查询过于频繁，请稍后再试");
    // 包括重启遗留的 PENDING。仅转换结果不明状态；预留仍待有凭据的结算。
    await this.pool.query("UPDATE fr_ai_requests r SET state='UNKNOWN',error_code='AI_RESULT_UNKNOWN',finished_at=now() WHERE r.user_id=$1 AND r.request_id=$2 AND r.state='PENDING' AND ((NOT EXISTS(SELECT 1 FROM fr_ai_jobs j WHERE j.user_id=r.user_id AND j.request_id=r.request_id) AND r.created_at<now()-($3*interval '1 millisecond')) OR EXISTS(SELECT 1 FROM fr_ai_jobs j WHERE j.user_id=r.user_id AND j.request_id=r.request_id AND j.phase IN ('DISPATCHING','UNKNOWN') AND j.dispatched_at<now()-($3*interval '1 millisecond')))",[user.id,requestId,STALE_REQUEST_MS]);
    await this.pool.query("UPDATE fr_ai_requests SET result_json=NULL,error_code='AI_RESULT_EXPIRED' WHERE user_id=$1 AND request_id=$2 AND result_json IS NOT NULL AND finished_at < now()-interval '24 hours'",[user.id,requestId]);
    const row = (await this.pool.query("SELECT state,result_json,error_code FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2 AND policy=$3",[user.id,requestId,HOSTED_AI_POLICY])).rows[0];
    assert(row,404,"NOT_FOUND","任务不存在");
    return { jobId: requestId, state: row.state as string,
      pollUrl: `/api/v2/ai/deepseek-flash-v1/responses/async/${requestId}`,
      result: row.result_json ? JSON.parse(row.result_json) : null,
      error: row.error_code ?? (row.state === 'UNKNOWN' ? 'AI_RESULT_UNKNOWN' : row.state === 'REJECTED' ? 'AI_PROVIDER_REJECTED' : row.state === 'COMPLETED' && !row.result_json ? 'AI_RESULT_UNAVAILABLE' : null),
    };
  }
  async cancel(headers:Headers,value:unknown){
    const user=await this.auth.authenticate(headers,true);
    return cancelQueued(this.pool,user.id,z.uuid().parse(value));
  }
  async runQueueOnce() {
    if(this.queueStopped || this.claiming || !this.config?.queue || this.pending.size>=2)return;
    const claiming=this.claimNext();this.claiming=claiming;
    try{await claiming;}finally{if(this.claiming===claiming)this.claiming=undefined;}
  }
  private async claimNext(){
      const job=await claimQueued(this.pool,this.config!.queue!,HOSTED_AI_POLICY,value=>inputSchema.parse(value));
      if(!job)return;
      // 解密后的正文仅在本次执行内存中存在；不恢复原账户 token 或 Cookie。
      const request:PreparedRequest={...job,body:inputSchema.parse(job.body),headers:{},sessionDigest:'',hash:''};
      const work=this.execute(request,true,job.dispatchId).then(()=>{},()=>{});
      this.pending.add(work);void work.then(()=>this.pending.delete(work));
  }
  private async execute(request: PreparedRequest, persistResult: boolean, dispatchId?:string) {
    const { userId, requestId, body, images, reservation } = request;
    const settle = async (state: string, charged: number | null, result: unknown = null, error: string | null = null) => transaction(this.pool,async tx => {
      await tx.query("SELECT user_id FROM fr_ai_accounts WHERE user_id=$1 FOR UPDATE",[userId]);
      if(dispatchId){
        await tx.query('SELECT request_id FROM fr_ai_requests WHERE user_id=$1 AND request_id=$2 FOR UPDATE',[userId,requestId]);
        const job=(await tx.query('SELECT phase,dispatch_id FROM fr_ai_jobs WHERE user_id=$1 AND request_id=$2 FOR UPDATE',[userId,requestId])).rows[0];
        assert(job?.dispatch_id===dispatchId && ['DISPATCHING','UNKNOWN'].includes(job.phase),409,'AI_RESULT_UNKNOWN','分析结果需要进一步核对');
      }
      const changed = await tx.query("UPDATE fr_ai_requests SET state=$3,charged=$4,result_json=$5,error_code=$6,finished_at=now() WHERE user_id=$1 AND request_id=$2 AND state IN ('PENDING','UNKNOWN') AND charged IS NULL RETURNING reserved",[userId,requestId,state,charged,persistResult && result !== null ? JSON.stringify(result) : null,error]);
      if (changed.rowCount && charged !== null) await tx.query("UPDATE fr_ai_accounts SET reserved=reserved-$2,spent=spent+$3 WHERE user_id=$1",[userId,reservation,charged]);
      assert(changed.rowCount,409,"AI_RESULT_UNKNOWN","请求状态已变更，请先核对结果");
      if(dispatchId)await tx.query("UPDATE fr_ai_jobs SET phase=$3,payload=NULL WHERE user_id=$1 AND request_id=$2",[userId,requestId,state==='UNKNOWN'?'UNKNOWN':'DONE']);
    });
    try {
      // 图像观察只需简短事实；DeepSeek 默认为 high thinking，会挤占同步请求等待时间。
      // 编排请求保持原样；不允许客户端借此切换模型、服务或扩大权限。
      const upstream = images ? {...body, reasoning: {effort: "none"}} : body;
      const response = await this.transport(upstream,this.config!.key);
      if ([400,401,403,404,422].includes(response.status)) {
        await response.body?.cancel(); await settle("REJECTED",0,null,"AI_PROVIDER_REJECTED");
        throw new ApiError(502,"AI_PROVIDER_REJECTED","模型服务拒绝了请求，额度已释放");
      }
      assert(response.ok,502,"AI_RESULT_UNKNOWN","模型结果无法确认，请勿重复提交");
      const result = await readReply(response);
      const usage = z.object({input_tokens:z.number().int().nonnegative(),output_tokens:z.number().int().nonnegative(),total_tokens:z.number().int().nonnegative()}).parse(result.usage);
      assert(usage.total_tokens === usage.input_tokens+usage.output_tokens && usage.total_tokens <= reservation && usage.output_tokens <= body.max_output_tokens && result.status === "completed" && !result.error && Array.isArray(result.output),502,"AI_RESULT_UNKNOWN","模型结果或计量无法确认，请勿重复提交");
      const reply = {id: typeof result.id === "string" ? result.id : randomUUID(),object:"response",status:"completed",model:HOSTED_AI_MODEL,output:result.output,usage};
      await settle("COMPLETED",usage.total_tokens,reply);
      return reply;
    } catch(error) {
      // 网络中断、无计量或进程退出不退款/重发；PENDING/UNKNOWN 额度留待人工对账。
      if (!(error instanceof ApiError && error.code === 'AI_PROVIDER_REJECTED')) await settle("UNKNOWN",null,null,"AI_RESULT_UNKNOWN");
      if (error instanceof ApiError) throw error;
      throw new ApiError(502,"AI_RESULT_UNKNOWN","模型结果无法确认，预留额度暂未释放，请联系 FocalRoom");
    }
  }
}
