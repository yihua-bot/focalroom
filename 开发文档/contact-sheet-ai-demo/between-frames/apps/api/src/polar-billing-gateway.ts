import { createHmac, timingSafeEqual } from 'node:crypto';
import { z } from 'zod';
import { assert, ApiError } from './errors.ts';
import type { Headers } from './auth.ts';
import type { BillingConfig, BillingGateway, BillingPrice, BillingRemoteEvent } from './billing-gateway.ts';

// 与 OpenYC 共用商户时，只接受本产品目录和 FocalRoom 的客户命名空间。
export const POLAR_API_VERSION='2026-10';
const id=z.uuid(), time=z.iso.datetime({offset:true});
const metadata=z.record(z.string(),z.unknown());
export type PolarTransport=(path:string,method:'GET'|'POST',body?:unknown)=>Promise<unknown>;
const checkoutSchema=z.object({id,customer_id:id,product_id:id,organization_id:id,status:z.enum(['open','expired','confirmed','succeeded','failed']),
  amount:z.number().int().positive(),currency:z.string(),url:z.string(),expires_at:time,metadata});

export function polarPaymentUrl(value:unknown,manage:boolean,environment:'sandbox'|'production') {
  const url=new URL(z.string().url().parse(value));
  const host=environment==='sandbox'?'sandbox.polar.sh':'polar.sh';
  assert(url.protocol==='https:' && url.hostname===host && !url.port && !url.username && !url.password &&
    (manage? /^\/[^/]+\/portal(?:\/|$)/.test(url.pathname):/^\/checkout\/[A-Za-z0-9_-]+\/?$/.test(url.pathname)),
    502,'BILLING_RESPONSE_INVALID','暂时无法打开付款页面');
  return url.href;
}

export class PolarBillingGateway implements BillingGateway {
  readonly canCancelCheckout=false;
  readonly environment:'sandbox'|'production';
  readonly transport:PolarTransport;
  readonly config:BillingConfig;
  constructor(config:BillingConfig,transport?:PolarTransport) {
    assert(config.provider==='polar' && id.safeParse(config.organizationId).success,500,'BILLING_CONFIG_INVALID','订阅服务暂时不可用');
    this.environment=config.polarEnvironment??'sandbox';
    this.config=config;
    this.transport=transport??(async(path,method,body)=>{
      assert(/^\/[a-z-]+\/(?:[A-Za-z0-9_:-]+\/?(?:\?[^#]*)?|\?[^#]*)?$/.test(path),500,'BILLING_REQUEST_INVALID','付款未完成');
      const base=this.environment==='sandbox'?'https://sandbox-api.polar.sh/v1':'https://api.polar.sh/v1';
      // 不自动重试写操作；Polar 的未知付款结果只能查询核对，不能再创建一单。
      const response=await fetch(base+path,{method,redirect:'error',signal:AbortSignal.timeout(15000),
        headers:{Authorization:'Bearer '+config.secret,'Polar-Version':POLAR_API_VERSION,'Content-Type':'application/json'},
        ...(body===undefined?{}:{body:JSON.stringify(body)})});
      assert(response.ok,502,'BILLING_UNAVAILABLE','付款服务暂时不可用，请稍后核对账户状态');
      const reader=response.body?.getReader();assert(reader,502,'BILLING_RESPONSE_INVALID','付款未完成');
      const chunks:Uint8Array[]=[];let length=0;
      try {for(;;){const {done,value}=await reader.read();if(done)break;length+=value.length;assert(length<=1024*1024,502,'BILLING_RESPONSE_INVALID','付款结果无法确认');chunks.push(value);}}
      finally {await reader.cancel().catch(()=>{});}
      return JSON.parse(Buffer.concat(chunks).toString('utf8'));
    });
  }
  private key(value:string) {return `polar:${this.environment}:${id.parse(value)}`;}
  private raw(value:string) {
    const prefix=`polar:${this.environment}:`;
    assert(value.startsWith(prefix),409,'BILLING_PROVIDER_MISMATCH','此订阅需要在原付款渠道中管理');
    return id.parse(value.slice(prefix.length));
  }
  private product(price:BillingPrice) {return price.productId??price.id;}
  private price(product:string,currency:string) {
    const price=this.config.prices.find(value=>this.product(value)===product && value.currency===currency.toUpperCase());
    assert(price,409,'BILLING_PRICE_CHANGED','订阅方案已更新，请刷新后重试');return price;
  }
  async createCustomer(userId:string,email:string) {
    const external='focalroom:'+userId;
    // 使用平台唯一 external_id；上次创建回执丢失时先查回，避免重复客户。
    const query=new URLSearchParams({query:external,organization_id:this.config.organizationId!,limit:'100'});
    const list=z.object({items:z.array(z.object({id,external_id:z.string().nullable()})),pagination:z.object({max_page:z.number().int()})}).parse(await this.transport('/customers/?'+query,'GET'));
    const matches=list.items.filter(customer=>customer.external_id===external);
    assert(matches.length<=1 && list.pagination.max_page<=1,409,'BILLING_RECONCILIATION_REQUIRED','付款资料需要核对');
    if(matches.length) return this.key(matches[0].id);
    const value=z.object({id,external_id:z.literal(external)}).parse(await this.transport('/customers/','POST',{external_id:external,email,metadata:{application:'focalroom',focalroom_user_id:userId}}));
    return this.key(value.id);
  }
  private async validateProduct(price:BillingPrice) {
    const product=z.object({id,organization_id:id,is_archived:z.boolean(),is_recurring:z.boolean(),recurring_interval:z.string(),recurring_interval_count:z.number(),
      prices:z.array(z.object({id,amount_type:z.string(),price_amount:z.number().optional(),price_currency:z.string(),is_archived:z.boolean()}))}).parse(await this.transport('/products/'+id.parse(this.product(price)),'GET'));
    const active=product.prices.filter(p=>!p.is_archived);
    const matching=active.filter(p=>p.price_currency.toUpperCase()===price.currency);
    // 多币种目录必须按已批准币种选价；不使用商户默认币种或隐式换汇。
    assert(product.id===this.product(price) && product.organization_id===this.config.organizationId && !product.is_archived && product.is_recurring &&
      product.recurring_interval===price.interval && product.recurring_interval_count===1 && active.every(p=>p.amount_type==='fixed') &&
      new Set(active.map(p=>p.price_currency.toUpperCase())).size===active.length && matching.length===1 && matching[0].price_amount===price.amount,
      409,'BILLING_PRICE_CHANGED','订阅方案已更新，请刷新后重试');
  }
  private session(value:unknown,customer:string,user:string,price:BillingPrice,request:string) {
    const session=checkoutSchema.parse(value);
    assert(session.customer_id===this.raw(customer) && session.product_id===this.product(price) && session.organization_id===this.config.organizationId &&
      session.amount===price.amount && session.currency.toUpperCase()===price.currency && session.metadata.application==='focalroom' &&
      session.metadata.focalroom_user_id===user && session.metadata.focalroom_request_id===request,
      409,'BILLING_SESSION_FORBIDDEN','付款资料无法核对');
    assert(session.status==='open' && Date.parse(session.expires_at)>Date.now(),409,'BILLING_PAYMENT_PENDING','付款正在确认或已经结束，请刷新账户状态');
    return {id:this.key(session.id),url:polarPaymentUrl(session.url,false,this.environment),expiresAt:Date.parse(session.expires_at)/1000};
  }
  async checkout(customer:string,user:string,price:BillingPrice,request:string,origin:string) {
    await this.validateProduct(price);
    const value=await this.transport('/checkouts/','POST',{products:[this.product(price)],currency:price.currency.toLowerCase(),customer_id:this.raw(customer),
      success_url:origin+'/shared#billing=return',return_url:origin+'/shared#billing=canceled',allow_discount_codes:false,allow_trial:false,
      metadata:{application:'focalroom',focalroom_user_id:user,focalroom_request_id:request}});
    return this.session(value,customer,user,price,request);
  }
  async recoverCheckout(customer:string,user:string,price:BillingPrice,request:string) {
    const matches:unknown[]=[];
    for(let page=1;page<=5;page++) {
      const query=new URLSearchParams({customer_id:this.raw(customer),product_id:this.product(price),organization_id:this.config.organizationId!,limit:'100',page:String(page)});
      const list=z.object({items:z.array(z.object({metadata}).passthrough()),pagination:z.object({max_page:z.number().int()})}).parse(await this.transport('/checkouts/?'+query,'GET'));
      matches.push(...list.items.filter(s=>s.metadata.application==='focalroom' && s.metadata.focalroom_user_id===user && s.metadata.focalroom_request_id===request));
      if(page>=list.pagination.max_page)break;
    }
    assert(matches.length===1,409,'BILLING_RECONCILIATION_REQUIRED','付款结果尚未确认，请稍后刷新或联系支持；不会重复创建付款');
    return this.session(matches[0],customer,user,price,request);
  }
  async manage(customer:string,origin:string) {
    const value=z.object({customer_id:id,customer_portal_url:z.string()}).parse(await this.transport('/customer-sessions/','POST',{customer_id:this.raw(customer),return_url:origin+'/shared#billing=return'}));
    assert(value.customer_id===this.raw(customer),409,'BILLING_CUSTOMER_MISMATCH','无法打开此订阅');
    return polarPaymentUrl(value.customer_portal_url,true,this.environment);
  }
  async cancelCheckout(session:string,customer:string) {
    const value=checkoutSchema.parse(await this.transport('/checkouts/'+this.raw(session),'GET'));
    assert(value.customer_id===this.raw(customer),403,'BILLING_SESSION_FORBIDDEN','无法取消此付款');
    // 当前公开 API 不提供主动过期。只关闭已明确过期/失败的本地记录。
    assert(value.status==='expired'||value.status==='failed',409,'BILLING_PAYMENT_PENDING','请关闭付款页面，等待付款链接到期；已订阅的方案请在管理订阅中取消');
  }
  async subscription(value:string) {
    const sub=z.object({id,customer_id:id,product_id:id,status:z.enum(['active','trialing','past_due','canceled','unpaid','incomplete','incomplete_expired','paused']),
      current_period_end:time,cancel_at_period_end:z.boolean(),amount:z.number().int(),currency:z.string(),recurring_interval:z.string(),recurring_interval_count:z.number(),
      customer:z.object({external_id:z.string().nullable()}),metadata}).parse(await this.transport('/subscriptions/'+this.raw(value),'GET'));
    const price=this.price(sub.product_id,sub.currency);
    assert(this.key(sub.id)===value && sub.metadata.application==='focalroom' && typeof sub.metadata.focalroom_user_id==='string' &&
      sub.customer.external_id==='focalroom:'+sub.metadata.focalroom_user_id && sub.amount===price.amount && sub.currency.toUpperCase()===price.currency &&
      sub.recurring_interval===price.interval && sub.recurring_interval_count===1,409,'BILLING_SUBSCRIPTION_INVALID','订阅需要进一步核对');
    return {id:value,customer:this.key(sub.customer_id),priceId:price.id,status:sub.status,currentPeriodEnd:Date.parse(sub.current_period_end)/1000,cancelAtPeriodEnd:sub.cancel_at_period_end};
  }
  async invoice(value:string) {
    const order=z.object({id,customer_id:id,product_id:id,subscription_id:id,paid:z.boolean(),status:z.enum(['draft','pending','paid','refunded','partially_refunded','void']),
      total_amount:z.number().int().nonnegative(),net_amount:z.number().int().nonnegative(),currency:z.string(),created_at:time,billing_reason:z.string(),metadata}).parse(await this.transport('/orders/'+this.raw(value),'GET'));
    const price=this.price(order.product_id,order.currency);
    assert(this.key(order.id)===value && order.metadata.application==='focalroom' && order.net_amount===price.amount && order.total_amount>=order.net_amount,
      409,'BILLING_INVOICE_INVALID','账单需要进一步核对');
    return {id:value,customer:this.key(order.customer_id),subscriptionId:this.key(order.subscription_id),priceId:price.id,
      status:order.paid?'paid' as const:order.status==='void'?'void' as const:'open' as const,amount:order.total_amount,currency:order.currency.toUpperCase(),
      issuedAt:Date.parse(order.created_at)/1000,billingReason:order.billing_reason};
  }
  async charge():Promise<never> {throw new ApiError(409,'BILLING_RECONCILIATION_REQUIRED','此付款争议需要人工核对');}
  event(body:Buffer,_signature:string,headers:Headers={}) : BillingRemoteEvent {
    const webhookId=headers['webhook-id'],timestamp=headers['webhook-timestamp'],signature=headers['webhook-signature'];
    assert(body.length>0 && body.length<=1024*1024 && typeof webhookId==='string' && /^[A-Za-z0-9_-]{1,200}$/.test(webhookId) && typeof timestamp==='string' && /^\d+$/.test(timestamp) &&
      Math.abs(Date.now()/1000-Number(timestamp))<=300 && typeof signature==='string' && signature.length<=2048,
      400,'BILLING_SIGNATURE_INVALID','付款通知无效');
    const secret=this.config.webhookSecret;
    const key=this.config.polarWebhookScheme==='legacy'?Buffer.from(secret):Buffer.from(secret.replace(/^whsec_/,''),'base64');
    assert(key.length>=16,500,'BILLING_CONFIG_INVALID','订阅服务暂时不可用');
    const expected=createHmac('sha256',key).update(`${webhookId}.${timestamp}.`).update(body).digest();
    assert(signature.split(' ').some(part=>{const [v,s]=part.split(',');if(v!=='v1'||!s||!/^[A-Za-z0-9+/]+={0,2}$/.test(s))return false;const actual=Buffer.from(s,'base64');return actual.length===expected.length&&timingSafeEqual(actual,expected);}),400,'BILLING_SIGNATURE_INVALID','付款通知无效');
    const event=z.object({type:z.string().max(100),api_version:z.literal(POLAR_API_VERSION),data:z.record(z.string(),z.unknown())}).parse(JSON.parse(body.toString('utf8')));
    const data=event.data;
    const result:BillingRemoteEvent={id:`polar:${this.environment}:${webhookId}`,type:event.type,subscriptionId:null,invoiceId:null,refund:null};
    if(typeof data.product_id==='string'&&!this.config.prices.some(p=>this.product(p)===data.product_id))return result;
    // 累计退款/争议不能冒充单次退款，未接通的账务通知明确留待核账。
    assert(!['order.refunded','order.disputed','dispute.created'].includes(event.type),409,'BILLING_RECONCILIATION_REQUIRED','此付款通知需要人工核对');
    if(event.type.startsWith('subscription.')) result.subscriptionId=this.key(id.parse(data.id));
    if(event.type==='order.paid' && data.subscription_id) {result.subscriptionId=this.key(id.parse(data.subscription_id));result.invoiceId=this.key(id.parse(data.id));}
    if(event.type==='refund.updated' && data.status==='succeeded') {
      const refund=z.object({id,order_id:id,amount:z.number().int().positive(),tax_amount:z.number().int().nonnegative().default(0),currency:z.string()}).parse(data);
      result.refund={id:this.key(refund.id),kind:'refund',amount:refund.amount+refund.tax_amount,currency:refund.currency.toUpperCase(),invoiceId:this.key(refund.order_id),chargeId:null};
    }
    return result;
  }
}
