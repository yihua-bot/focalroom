import type { Pool } from 'pg';
import { z } from 'zod';
import { UNAVAILABLE_SUBSCRIPTION_CATALOG } from '@betweenframes/contracts';
import type { AuthService, Headers } from './auth.ts';
import { transaction, digest } from './database.ts';
import { assert } from './errors.ts';
import { checkRateLimit } from './rate-limit.ts';
import { grantAiTokensTx } from './hosted-ai.ts';
import { StripeBillingGateway, type BillingConfig, type BillingGateway } from './billing-gateway.ts';

export class BillingService {
  readonly pool: Pool;
  readonly auth: AuthService;
  readonly config?: BillingConfig;
  readonly gateway?: BillingGateway;
  readonly salesEnabled: boolean;
  constructor(pool: Pool, auth: AuthService, config?: BillingConfig, gateway?: BillingGateway, salesEnabled = false) {
    this.pool=pool;this.auth=auth;this.config=config;this.gateway=config ? gateway ?? new StripeBillingGateway(config) : undefined;
    this.salesEnabled=salesEnabled && config?.salesEnabled!==false;
  }
  catalog() {
    return {salesAvailable:!!this.gateway && this.salesEnabled,plans:UNAVAILABLE_SUBSCRIPTION_CATALOG.plans.map(plan=>({
      ...plan,prices:plan.id==='pro'?(this.config?.prices??[]).map(({includedTokens,...price})=>({...price,includedCredits:includedTokens/1000})):[],
    }))};
  }
  async overview(headers: Headers) {
    const user=await this.auth.authenticate(headers,false);
    const sub=(await this.pool.query("SELECT * FROM fr_billing_subscriptions WHERE user_id=$1 ORDER BY current_period_end DESC,updated_at DESC LIMIT 1",[user.id])).rows[0];
    const paid=!!sub && ['active','trialing','past_due'].includes(sub.status) && new Date(sub.current_period_end).getTime()>Date.now();
    const balance=(await this.pool.query('SELECT granted,spent,reserved FROM fr_ai_accounts WHERE user_id=$1',[user.id])).rows[0];
    const granted=Number(balance?.granted??0),available=Math.max(0,granted-Number(balance?.spent??0)-Number(balance?.reserved??0));
    const invoices=(await this.pool.query('SELECT id,amount,currency,status,issued_at FROM fr_billing_invoices WHERE user_id=$1 ORDER BY issued_at DESC LIMIT 30',[user.id])).rows;
    const customer=(await this.pool.query('SELECT customer_id FROM fr_billing_customers WHERE user_id=$1',[user.id])).rows[0];
    const pending=await this.pool.query("SELECT 1 FROM fr_billing_checkouts WHERE user_id=$1 AND state IN ('PENDING','READY') AND (expires_at IS NULL OR expires_at>now()) LIMIT 1",[user.id]);
    return {planId:paid?'pro':'free',planName:paid?'FocalRoom Pro':'基础版',status:paid?sub.status:sub?.status==='canceled'?'canceled':'free',
      currentPeriodEnd:paid || sub?.status==='canceled' ? sub.current_period_end : null,cancelAtPeriodEnd:paid ? sub.cancel_at_period_end : sub?.status==='canceled',
      usage:{remainingPercent:granted?Math.min(100,Math.floor(available/granted*100)):null,hasAllowance:available>0,availableCredits:available/1000,totalCredits:granted/1000},
      invoices:invoices.map(row=>({id:row.id,date:row.issued_at,amount:Number(row.amount),currency:row.currency,status:row.status,url:null})),manageAvailable:!!customer && !!this.gateway,pendingCheckout:!!pending.rowCount};
  }
  async checkout(headers: Headers, input: unknown) {
    const user=await this.auth.authenticate(headers,true);
    assert(user.email_verified_at,403,'EMAIL_UNVERIFIED','请先完成邮箱验证');
    const value=z.strictObject({priceId:z.string().max(200),requestId:z.string().uuid()}).parse(input);
    const price=this.config?.prices.find(p=>p.id===value.priceId);
    assert(this.gateway && price && this.salesEnabled,503,'BILLING_NOT_AVAILABLE','订阅暂未开放');
    const gateway=this.gateway;
    const rate=await checkRateLimit(this.pool,'billing:'+user.id,10);
    assert(rate.allowed,429,'RATE_LIMITED','操作过于频繁，请稍后再试');
    const prepared = await transaction<{url:string}|{customer:string;requestId:string}>(this.pool,async tx=>{
      await tx.query('SELECT pg_advisory_xact_lock(hashtextextended($1,701))',[user.id]);
      const existing=(await tx.query('SELECT * FROM fr_billing_checkouts WHERE user_id=$1 AND request_id=$2',[user.id,value.requestId])).rows[0];
      if(existing) {
        assert(existing.state!=='CLOSED',410,'BILLING_SESSION_EXPIRED','付款页面已过期，请重新选择订阅');
        assert(Date.now()-new Date(existing.created_at).getTime()<23*3600000,409,'BILLING_RECONCILIATION_REQUIRED','付款结果需要核对，请联系支持');
        assert(existing.price_id===price.id,409,'IDEMPOTENCY_CONFLICT','此付款请求已用于其他方案');
        if(existing.state==='READY') {
          assert(new Date(existing.expires_at).getTime()>Date.now(),410,'BILLING_SESSION_EXPIRED','付款页面已过期，请重新选择订阅');
          return {url:existing.checkout_url};
        }
      }
      const active=await tx.query("SELECT id FROM fr_billing_subscriptions WHERE user_id=$1 AND status IN ('active','trialing','past_due') AND current_period_end>now()",[user.id]);
      assert(!active.rowCount,409,'ALREADY_SUBSCRIBED','你已有订阅，请在管理订阅中调整方案');
      // 其他尚有效的付款页面不能同时创建第二份订阅。
      const pending=(await tx.query("SELECT * FROM fr_billing_checkouts WHERE user_id=$1 AND request_id<>$2 AND state IN ('PENDING','READY') AND (expires_at IS NULL OR expires_at>now()) ORDER BY created_at DESC LIMIT 1",[user.id,value.requestId])).rows[0];
      if(pending) {
        assert(pending.price_id===price.id,409,'BILLING_CHECKOUT_PENDING','请先完成当前付款，再选择其他方案');
        if(pending.state==='READY') return {url:pending.checkout_url};
        assert(Date.now()-new Date(pending.created_at).getTime()<23*3600000,409,'BILLING_RECONCILIATION_REQUIRED','付款结果需要核对，请联系支持');
        const customer=(await tx.query('SELECT customer_id FROM fr_billing_customers WHERE user_id=$1',[user.id])).rows[0];
        assert(customer,409,'BILLING_RECONCILIATION_REQUIRED','付款结果需要核对，请联系支持');
        return {customer:customer.customer_id,requestId:pending.request_id};
      }
      let customer=(await tx.query('SELECT customer_id FROM fr_billing_customers WHERE user_id=$1',[user.id])).rows[0]?.customer_id;
      if(!customer) {
        customer=await gateway.createCustomer(user.id,user.email);
        await tx.query('INSERT INTO fr_billing_customers(user_id,customer_id) VALUES($1,$2)',[user.id,customer]);
      }
      await tx.query('INSERT INTO fr_billing_checkouts(user_id,request_id,price_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING',[user.id,value.requestId,price.id]);
      return {customer,requestId:value.requestId};
    });
    if('url' in prepared) return prepared;
    // 先持久化付款意图再调用远端；丢失响应仍用同一支付幂等键恢复。
    const session=await gateway.checkout(prepared.customer,user.id,price,prepared.requestId,this.auth.origin);
    await this.pool.query("UPDATE fr_billing_checkouts SET session_id=$3,checkout_url=$4,expires_at=to_timestamp($5),state='READY' WHERE user_id=$1 AND request_id=$2 AND state='PENDING'",[user.id,prepared.requestId,session.id,session.url,session.expiresAt]);
    return {url:session.url};
  }
  async manage(headers: Headers) {
    const user=await this.auth.authenticate(headers,true);
    assert(this.gateway,503,'BILLING_NOT_AVAILABLE','订阅暂未开放');
    const customer=(await this.pool.query('SELECT customer_id FROM fr_billing_customers WHERE user_id=$1',[user.id])).rows[0];
    assert(customer,404,'NOT_FOUND','没有可管理的订阅');
    const rate=await checkRateLimit(this.pool,'billing-portal:'+user.id,10);
    assert(rate.allowed,429,'RATE_LIMITED','操作过于频繁，请稍后再试');
    return {url:await this.gateway.manage(customer.customer_id,this.auth.origin)};
  }
  async cancelCheckout(headers: Headers) {
    const user=await this.auth.authenticate(headers,true);
    assert(this.gateway,503,'BILLING_NOT_AVAILABLE','订阅暂未开放');
    const gateway=this.gateway;
    const rate=await checkRateLimit(this.pool,'billing-cancel:'+user.id,10);
    assert(rate.allowed,429,'RATE_LIMITED','操作过于频繁，请稍后再试');
    return transaction(this.pool,async tx=>{
      await tx.query('SELECT pg_advisory_xact_lock(hashtextextended($1,701))',[user.id]);
      const pending=(await tx.query("SELECT * FROM fr_billing_checkouts WHERE user_id=$1 AND state IN ('PENDING','READY') ORDER BY created_at DESC LIMIT 1",[user.id])).rows[0];
      if(!pending) {
        const paid=await tx.query("SELECT id FROM fr_billing_subscriptions WHERE user_id=$1 AND status IN ('active','trialing') AND current_period_end>now()",[user.id]);
        assert(!paid.rowCount,409,'BILLING_PAYMENT_PENDING','付款已确认，请在管理订阅中调整方案');
        return {canceled:true};
      }
      const customer=(await tx.query('SELECT customer_id FROM fr_billing_customers WHERE user_id=$1',[user.id])).rows[0];
      assert(customer,409,'BILLING_RECONCILIATION_REQUIRED','付款结果需要核对，请联系支持');
      let session=pending.session_id;
      if(!session) {
        const price=this.config?.prices.find(p=>p.id===pending.price_id);
        assert(price && Date.now()-new Date(pending.created_at).getTime()<23*3600000,409,'BILLING_RECONCILIATION_REQUIRED','付款结果需要核对，请联系支持');
        session=(await gateway.checkout(customer.customer_id,user.id,price,pending.request_id,this.auth.origin)).id;
      }
      await gateway.cancelCheckout(session,customer.customer_id);
      await tx.query("UPDATE fr_billing_checkouts SET state='CLOSED' WHERE user_id=$1 AND request_id=$2",[user.id,pending.request_id]);
      return {canceled:true};
    });
  }
  async webhook(body: Buffer, signature: string) {
    assert(this.gateway,503,'BILLING_NOT_AVAILABLE','订阅暂未开放');
    const gateway=this.gateway,event=gateway.event(body,signature),fingerprint=digest(body);
    return transaction(this.pool,async tx=>{
      await tx.query('SELECT pg_advisory_xact_lock(hashtextextended($1,702))',[event.id]);
      const old=(await tx.query('SELECT digest FROM fr_billing_events WHERE id=$1',[event.id])).rows[0];
      if(old) { assert(old.digest===fingerprint,409,'BILLING_EVENT_CONFLICT','付款通知无法核对');return {received:true}; }
      if(event.subscriptionId) {
        const initial=await gateway.subscription(event.subscriptionId);
        const customer=(await tx.query('SELECT user_id FROM fr_billing_customers WHERE customer_id=$1',[initial.customer])).rows[0];
        if(customer) {
          await tx.query('SELECT pg_advisory_xact_lock(hashtextextended($1,701))',[customer.user_id]);
          // 获得用户锁后重新读取权威状态，旧事件不能覆盖新状态。
          const sub=await gateway.subscription(event.subscriptionId);
          const price=this.config?.prices.find(p=>p.id===sub.priceId);
          assert(sub.id===event.subscriptionId && sub.customer===initial.customer && price,409,'BILLING_SUBSCRIPTION_INVALID','订阅需要进一步核对');
          const existing=(await tx.query('SELECT user_id,customer_id FROM fr_billing_subscriptions WHERE id=$1',[sub.id])).rows[0];
          assert(!existing || (existing.user_id===customer.user_id && existing.customer_id===sub.customer),409,'BILLING_SUBSCRIPTION_CONFLICT','订阅需要进一步核对');
          await tx.query(`INSERT INTO fr_billing_subscriptions(id,user_id,customer_id,price_id,status,current_period_end,cancel_at_period_end)
            VALUES($1,$2,$3,$4,$5,to_timestamp($6),$7) ON CONFLICT(id) DO UPDATE SET status=EXCLUDED.status,price_id=EXCLUDED.price_id,current_period_end=EXCLUDED.current_period_end,cancel_at_period_end=EXCLUDED.cancel_at_period_end,updated_at=now()`,
            [sub.id,customer.user_id,sub.customer,sub.priceId,sub.status,sub.currentPeriodEnd,sub.cancelAtPeriodEnd]);
          if(event.invoiceId) {
            const invoice=await gateway.invoice(event.invoiceId);
            assert(invoice.id===event.invoiceId && invoice.customer===sub.customer && invoice.subscriptionId===sub.id && invoice.priceId===price.id && invoice.currency===price.currency,
              409,'BILLING_INVOICE_INVALID','账单需要进一步核对');
            assert(invoice.status==='paid',409,'BILLING_PAYMENT_PENDING','付款尚未确认，请稍后重试');
            const qualifying=invoice.status==='paid' && ['subscription_create','subscription_cycle'].includes(invoice.billingReason);
            const tokens=qualifying?price.includedTokens:0;
            const saved=await tx.query('INSERT INTO fr_billing_invoices(id,user_id,subscription_id,amount,currency,status,issued_at,tokens_granted) VALUES($1,$2,$3,$4,$5,$6,to_timestamp($7),$8) ON CONFLICT DO NOTHING RETURNING id',
              [invoice.id,customer.user_id,sub.id,invoice.amount,invoice.currency,invoice.status,invoice.issuedAt,tokens]);
            if(saved.rowCount && tokens) await grantAiTokensTx(tx,customer.user_id,'billing:'+digest(invoice.id),tokens,'subscription:'+price.id);
          }
          await tx.query("UPDATE fr_billing_checkouts SET state='CLOSED' WHERE user_id=$1 AND state='READY' AND $2 IN ('active','trialing')",[customer.user_id,sub.status]);
        }
      }
      await tx.query('INSERT INTO fr_billing_events(id,digest,event_type) VALUES($1,$2,$3)',[event.id,fingerprint,event.type]);
      return {received:true};
    });
  }
}
