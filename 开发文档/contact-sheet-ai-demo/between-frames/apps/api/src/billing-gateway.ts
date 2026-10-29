import { createHmac, timingSafeEqual } from 'node:crypto';
import { z } from 'zod';
import { assert } from './errors.ts';

export const priceSchema = z.strictObject({
  id: z.string().regex(/^price_[A-Za-z0-9]+$/).max(120), interval: z.enum(['month','year']),
  amount: z.number().int().positive().max(100000000), currency: z.enum(['USD','CNY','EUR','JPY','KRW']),
  includedTokens: z.number().int().positive().max(100000000),
  seats: z.number().int().min(1).max(50).default(3),
});
export type BillingPrice = z.infer<typeof priceSchema>;
export type BillingConfig = { secret: string; webhookSecret: string; prices: BillingPrice[]; salesEnabled?: boolean; freeTierMonthlyTokens?: number };
export function billingConfig(): BillingConfig | undefined {
  const salesEnabled = process.env.FOCALROOM_BILLING_ENABLED === '1';
  const secret = process.env.FOCALROOM_STRIPE_SECRET_KEY ?? '';
  const webhookSecret = process.env.FOCALROOM_STRIPE_WEBHOOK_SECRET ?? '';
  // 关闭新销售仍处理已有订阅的管理和通知；完全未配置时不创建支付网关。
  if (!salesEnabled && !secret && !webhookSecret) return undefined;
  const prices = z.array(priceSchema).min(1).max(2).parse(JSON.parse(process.env.FOCALROOM_BILLING_PRICES ?? '[]'));
  assert(secret.startsWith('sk_') && webhookSecret.startsWith('whsec_') && new Set(prices.map(p => p.id)).size === prices.length && new Set(prices.map(p => p.interval)).size === prices.length,
    500,'BILLING_CONFIG_INVALID','订阅服务暂时不可用');
  return { secret, webhookSecret, prices, salesEnabled };
}
export type BillingRefundEvent = { id: string; kind: 'refund' | 'dispute'; amount: number; currency: string; invoiceId: string | null; chargeId: string | null };
export type BillingCharge = { id: string; customer: string; invoiceId: string | null; amount: number; currency: string };
export type BillingRemoteEvent = { id: string; type: string; subscriptionId: string | null; invoiceId: string | null; refund: BillingRefundEvent | null };
export type BillingSubscription = { id: string; customer: string; priceId: string; status: string; currentPeriodEnd: number; cancelAtPeriodEnd: boolean };
export type BillingInvoice = { id: string; customer: string; subscriptionId: string; priceId: string; status: 'paid'|'open'|'void'; amount: number; currency: string; issuedAt: number; billingReason: string };
export interface BillingGateway {
  createCustomer(userId: string, email: string): Promise<string>;
  checkout(customer: string, userId: string, price: BillingPrice, requestId: string, origin: string): Promise<{ id: string; url: string; expiresAt: number }>;
  manage(customer: string, origin: string): Promise<string>;
  cancelCheckout(session: string, customer: string): Promise<void>;
  subscription(id: string): Promise<BillingSubscription>;
  invoice(id: string): Promise<BillingInvoice>;
  charge(id: string): Promise<BillingCharge>;
  event(body: Buffer, signature: string): BillingRemoteEvent;
}
export type BillingTransport = (path: string, method: 'GET'|'POST', body?: URLSearchParams, idempotency?: string) => Promise<unknown>;
const reference = z.string().regex(/^[A-Za-z0-9_]+$/).max(200);
const ref = (value: unknown) => reference.parse(typeof value === 'object' && value ? (value as {id?:unknown}).id : value);
// 退款通知取 refunds 列表最后一条（本次退款）；累计 amount_refunded 在多次退款下会重复扣减。
const refundFromCharge = (object: Record<string, unknown>): BillingRefundEvent => {
  const data=(object.refunds as {data?: unknown[]} | undefined)?.data;
  const latest=Array.isArray(data)?data[data.length-1]:undefined;
  const refund=z.object({id:reference,amount:z.number().int().positive()}).parse(latest);
  assert(object.refunded===true && typeof object.invoice==='string' && typeof object.currency==='string',400,'BILLING_REFUND_INVALID','退款通知无法核对');
  return {id:refund.id,kind:'refund',amount:refund.amount,currency:object.currency.toUpperCase(),invoiceId:ref(object.invoice),chargeId:null};
};
const refundFromDispute = (object: Record<string, unknown>): BillingRefundEvent => {
  const dispute=z.object({id:reference,charge:z.unknown(),amount:z.number().int().positive(),currency:z.string()}).parse(object);
  return {id:dispute.id,kind:'dispute',amount:dispute.amount,currency:dispute.currency.toUpperCase(),invoiceId:null,chargeId:ref(dispute.charge)};
};
export function paymentUrl(value: unknown, host: 'checkout.stripe.com'|'billing.stripe.com') {
  const url = new URL(z.string().url().parse(value));
  assert(url.protocol === 'https:' && url.hostname === host && !url.port && !url.username && !url.password,502,'BILLING_RESPONSE_INVALID','暂时无法打开付款页面');
  return url.href;
}
export class StripeBillingGateway implements BillingGateway {
  readonly config: BillingConfig;
  readonly transport: BillingTransport;
  constructor(config: BillingConfig, transport?: BillingTransport) {
    this.config = config;
    this.transport = transport ?? (async (path, method, body, idempotency) => {
      assert(/^\/[a-z_]+(?:\/[A-Za-z0-9_]+)*$/.test(path),500,'BILLING_REQUEST_INVALID','付款未完成');
      const response = await fetch('https://api.stripe.com/v1'+path,{ method, redirect:'error',signal:AbortSignal.timeout(15000),
        headers:{Authorization:'Bearer '+config.secret,'Stripe-Version':'2025-02-24.acacia',...(body?{'Content-Type':'application/x-www-form-urlencoded'}:{}),...(idempotency?{'Idempotency-Key':idempotency}:{})},body });
      assert(response.ok,502,'BILLING_UNAVAILABLE','付款服务暂时不可用，请稍后重试');
      const reader = response.body?.getReader(); assert(reader,502,'BILLING_RESPONSE_INVALID','付款未完成');
      const chunks:Uint8Array[]=[];let size=0;
      try { for(;;) { const {done,value}=await reader.read();if(done)break;size+=value.byteLength;assert(size<=1024*1024,502,'BILLING_RESPONSE_INVALID','付款未完成');chunks.push(value); } }
      finally { await reader.cancel().catch(()=>{}); }
      return JSON.parse(Buffer.concat(chunks).toString('utf8'));
    });
  }
  async createCustomer(userId: string, email: string) {
    const value = await this.transport('/customers','POST',new URLSearchParams({email,'metadata[focalroom_user_id]':userId}),'focalroom-customer:'+userId);
    return ref(value);
  }
  async checkout(customer: string, userId: string, price: BillingPrice, requestId: string, origin: string) {
    // 付款前核对真实 Price，防止展示金额与实际扣款不一致。
    const actual = z.object({id:z.string(),unit_amount:z.number(),currency:z.string(),active:z.boolean(),recurring:z.object({interval:z.string(),interval_count:z.number()})}).parse(await this.transport('/prices/'+reference.parse(price.id),'GET'));
    assert(actual.id===price.id && actual.active && actual.unit_amount===price.amount && actual.currency.toUpperCase()===price.currency && actual.recurring.interval===price.interval && actual.recurring.interval_count===1,
      409,'BILLING_PRICE_CHANGED','订阅方案已更新，请刷新后重试');
    const body = new URLSearchParams({mode:'subscription',customer,client_reference_id:userId,'line_items[0][price]':price.id,'line_items[0][quantity]':'1',
      'subscription_data[metadata][focalroom_user_id]':userId,success_url:origin+'/shared#billing=return',cancel_url:origin+'/shared#billing=canceled'});
    const value = z.object({id:z.string(),url:z.string(),expires_at:z.number().int()}).parse(await this.transport('/checkout/sessions','POST',body,'focalroom-checkout:'+userId+':'+requestId));
    return { id:reference.parse(value.id),url:paymentUrl(value.url,'checkout.stripe.com'),expiresAt:value.expires_at };
  }
  async manage(customer: string, origin: string) {
    const value = z.object({url:z.string()}).parse(await this.transport('/billing_portal/sessions','POST',new URLSearchParams({customer,return_url:origin+'/shared#billing=return'})));
    return paymentUrl(value.url,'billing.stripe.com');
  }
  async cancelCheckout(session: string, customer: string) {
    const value=z.object({id:reference,customer:z.unknown(),status:z.enum(['open','complete','expired']),payment_status:z.string()}).parse(await this.transport('/checkout/sessions/'+reference.parse(session),'GET'));
    assert(value.id===session && ref(value.customer)===customer,403,'BILLING_SESSION_FORBIDDEN','无法取消此付款');
    if(value.status==='expired') return;
    assert(value.status==='open' && value.payment_status==='unpaid',409,'BILLING_PAYMENT_PENDING','付款已处理或正在确认，请刷新账户状态');
    const expired=z.object({id:reference,status:z.literal('expired')}).parse(await this.transport('/checkout/sessions/'+reference.parse(session)+'/expire','POST',new URLSearchParams()));
    assert(expired.id===session,502,'BILLING_RESPONSE_INVALID','付款状态暂时无法确认');
  }
  async subscription(id: string): Promise<BillingSubscription> {
    const value = z.object({id:reference,customer:z.unknown(),status:z.enum(['active','trialing','past_due','canceled','incomplete','unpaid','paused','incomplete_expired']),current_period_end:z.number().int().positive(),cancel_at_period_end:z.boolean(),items:z.object({data:z.array(z.object({price:z.object({id:reference})})).length(1)})}).parse(await this.transport('/subscriptions/'+reference.parse(id),'GET'));
    return {id:value.id,customer:ref(value.customer),priceId:value.items.data[0].price.id,status:value.status,currentPeriodEnd:value.current_period_end,cancelAtPeriodEnd:value.cancel_at_period_end};
  }
  async invoice(id: string): Promise<BillingInvoice> {
    const value = z.object({id:reference,customer:z.unknown(),subscription:z.unknown(),status:z.enum(['paid','open','void']),amount_paid:z.number().int().nonnegative(),currency:z.string(),created:z.number().int().positive(),billing_reason:z.string(),lines:z.object({data:z.array(z.object({price:z.object({id:reference}).nullable(),proration:z.boolean()}))})}).parse(await this.transport('/invoices/'+reference.parse(id),'GET'));
    const recurring=value.lines.data.filter(line=>!line.proration && line.price);
    assert(recurring.length===1,409,'BILLING_INVOICE_REVIEW','账单需要进一步核对');
    return {id:value.id,customer:ref(value.customer),subscriptionId:ref(value.subscription),priceId:recurring[0].price!.id,status:value.status,amount:value.amount_paid,currency:value.currency.toUpperCase(),issuedAt:value.created,billingReason:value.billing_reason};
  }
  async charge(id: string): Promise<BillingCharge> {
    const value = z.object({id:reference,customer:z.unknown(),invoice:z.unknown().nullable(),amount:z.number().int().positive(),currency:z.string()}).parse(await this.transport('/charges/'+reference.parse(id),'GET'));
    return {id:value.id,customer:ref(value.customer),invoiceId:value.invoice?ref(value.invoice):null,amount:value.amount,currency:value.currency.toUpperCase()};
  }
  event(body: Buffer, signature: string): BillingRemoteEvent {
    assert(body.length>0 && body.length<=1024*1024 && signature.length<=2048,400,'BILLING_SIGNATURE_INVALID','付款通知无效');
    const parts=signature.split(',').map(part=>part.trim().split('='));
    const timestamps=parts.filter(([key])=>key==='t');
    const timestamp=timestamps[0]?.[1];
    assert(timestamps.length===1 && /^\d+$/.test(timestamp??'') && Math.abs(Date.now()/1000-Number(timestamp))<=300,400,'BILLING_SIGNATURE_INVALID','付款通知无效');
    const expected=createHmac('sha256',this.config.webhookSecret).update(timestamp+'.').update(body).digest();
    assert(parts.some(([key,value])=>key==='v1' && /^[a-f0-9]{64}$/.test(value??'') && timingSafeEqual(expected,Buffer.from(value,'hex'))),400,'BILLING_SIGNATURE_INVALID','付款通知无效');
    const event=z.object({id:reference,type:z.string().max(100),livemode:z.boolean(),data:z.object({object:z.record(z.string(),z.unknown())})}).parse(JSON.parse(body.toString('utf8')));
    assert(event.livemode===this.config.secret.startsWith('sk_live_'),400,'BILLING_MODE_MISMATCH','付款通知无效');
    const object=event.data.object;
    const subscriptionId=event.type.startsWith('customer.subscription.')?ref(object.id):event.type==='invoice.paid' && object.subscription?ref(object.subscription):null;
    const refund=event.type==='charge.refunded'?refundFromCharge(object):event.type==='charge.dispute.created'?refundFromDispute(object):null;
    return {id:event.id,type:event.type,subscriptionId,invoiceId:event.type==='invoice.paid' && subscriptionId?ref(object.id):null,refund};
  }
}
