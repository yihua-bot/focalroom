import { before, after, test } from 'node:test';
import assert from 'node:assert/strict';
import { createHmac, randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { Pool } from 'pg';
import { createApi } from '../src/server.ts';
import { migrate } from '../src/database.ts';
import { LocalPreviewStore } from '../src/storage.ts';
import { StripeBillingGateway } from '../src/billing-gateway.ts';

const config={secret:'sk_test_fixture',webhookSecret:'whsec_fixture',prices:[{id:'price_monthly',interval:'month',amount:1900,currency:'USD',includedTokens:50000},{id:'price_yearly',interval:'year',amount:19000,currency:'USD',includedTokens:600000}]};
let pool,api,base,owner,other,disabledApi,noAiApi,salesOffApi;
const customers=new Map(),sessions=new Map(),subscriptions=new Map(),invoices=new Map(),charges=new Map();
let failAfterCreating=false, remoteCheckouts=0, priceChanged=false;
const gateway=new StripeBillingGateway(config,async(path,method,body,key)=>{
  if(path==='/customers') { const id='cus_'+body.get('metadata[focalroom_user_id]').replaceAll('-','');customers.set(body.get('metadata[focalroom_user_id]'),id);return {id}; }
  if(path.startsWith('/prices/')) { const price=config.prices.find(p=>path.endsWith(p.id));return {id:price.id,unit_amount:priceChanged?9999:price.amount,currency:price.currency.toLowerCase(),active:true,recurring:{interval:price.interval,interval_count:1}}; }
  if(path==='/checkout/sessions') {
    assert.equal(body.get('mode'),'subscription');
    if(!sessions.has(key)) { const id='cs_'+randomUUID().replaceAll('-','');sessions.set(key,{id,url:'https://checkout.stripe.com/c/pay/'+id,expires_at:Math.floor(Date.now()/1000)+3600,customer:body.get('customer'),status:'open',payment_status:'unpaid'});remoteCheckouts++; }
    if(failAfterCreating) { failAfterCreating=false;throw new Error('模拟远端已创建、响应丢失'); }
    return sessions.get(key);
  }
  if(path==='/billing_portal/sessions') { assert.equal(body.get('customer'),customers.get(owner.user.id));return {url:'https://billing.stripe.com/p/session/local'}; }
  if(path.startsWith('/checkout/sessions/')) {
    const id=path.split('/')[3],session=[...sessions.values()].find(value=>value.id===id);assert.ok(session);
    if(method==='POST') { assert.ok(path.endsWith('/expire'));session.status='expired'; }
    return structuredClone(session);
  }
  if(path.startsWith('/subscriptions/')) { const value=subscriptions.get(path.split('/').pop());assert.ok(value,'仅允许本地订阅夹具');return structuredClone(value); }
  if(path.startsWith('/invoices/')) { const value=invoices.get(path.split('/').pop());assert.ok(value,'仅允许本地账单夹具');return structuredClone(value); }
  if(path.startsWith('/charges/')) { const value=charges.get(path.split('/').pop());assert.ok(value,'仅允许本地付款夹具');return structuredClone(value); }
  throw new Error('没有实现的本地支付调用：'+path+' '+method);
});
async function call(path,{method='GET',account=owner,body,status=200,headers={}}={}) {
  const response=await fetch(base+'/api/v2'+path,{method,headers:{'content-type':'application/json',...(account?{authorization:'Bearer '+account.token}:{}),...headers},...(body===undefined?{}:{body:JSON.stringify(body)})});
  const value=await response.json();assert.equal(response.status,status,JSON.stringify(value));return value;
}
function event(id,type,object) { return {id,type,livemode:false,data:{object}}; }
async function webhook(value,{timestamp=Math.floor(Date.now()/1000),signature,expected=200}={}) {
  const body=JSON.stringify(value),signed=signature??'t='+timestamp+',v1='+createHmac('sha256',config.webhookSecret).update(timestamp+'.'+body).digest('hex');
  const response=await fetch(base+'/api/v2/billing/webhook',{method:'POST',headers:{'content-type':'application/json','stripe-signature':signed},body});
  const result=await response.json();assert.equal(response.status,expected,JSON.stringify(result));return result;
}
before(async()=>{
  assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL);
  pool=new Pool({connectionString:process.env.FOCALROOM_TEST_DATABASE_URL});await migrate(pool);
  api=await createApi({pool,origin:'http://127.0.0.1:1424',store:new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY,'billing-previews')),billing:config,billingGateway:gateway,hostedAi:{key:'fixture-not-a-real-key',initialTokens:0},hostedAiTransport:async()=>{throw new Error('订阅测试不调用模型');}});
  await api.app.listen(0,'127.0.0.1');base=await api.app.getUrl();
  owner=await call('/auth/register',{method:'POST',account:null,body:{email:'billing-owner@fr.test',password:'billing-fixture-password',display_name:'摄影师'}});
  other=await call('/auth/register',{method:'POST',account:null,body:{email:'billing-other@fr.test',password:'billing-fixture-password',display_name:'另一用户'}});
});
after(async()=>{await salesOffApi?.close();await noAiApi?.close();await disabledApi?.close();await api?.close();await pool?.end();});

test('catalog is public; private billing and checkout enforce authentication, verification, CSRF and configured prices',async()=>{
  const catalog=await call('/billing/catalog',{account:null});assert.equal(catalog.salesAvailable,true);assert.equal(catalog.plans[1].prices[0].amount,1900);assert.ok(!JSON.stringify(catalog).includes(config.secret));
  await call('/billing/overview',{account:null,status:401});
  await call('/billing/checkout',{method:'POST',body:{priceId:'price_monthly',requestId:randomUUID()},status:403});
  await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=ANY($1)',[[owner.user.id,other.user.id]]);
  assert.equal((await call('/auth/me')).emailVerified,true);
  await call('/billing/checkout',{method:'POST',account:null,headers:{cookie:'fr_session='+owner.token,origin:'http://127.0.0.1:1424'},body:{priceId:'price_monthly',requestId:randomUUID()},status:403});
  await call('/billing/checkout',{method:'POST',body:{priceId:'price_invented',requestId:randomUUID()},status:503});
  assert.equal(remoteCheckouts,0);
});

test('lost remote checkout response retains intent; retries and concurrent requests create one checkout and no entitlement',async()=>{
  const id=randomUUID();failAfterCreating=true;
  await call('/billing/checkout',{method:'POST',body:{priceId:'price_monthly',requestId:id},status:500});
  assert.equal((await pool.query("SELECT state FROM fr_billing_checkouts WHERE user_id=$1",[owner.user.id])).rows[0].state,'PENDING');
  const retry=await call('/billing/checkout',{method:'POST',body:{priceId:'price_monthly',requestId:randomUUID()}});
  const replay=await call('/billing/checkout',{method:'POST',body:{priceId:'price_monthly',requestId:id}});
  assert.equal(retry.url,replay.url);assert.equal(remoteCheckouts,1);
  await Promise.all([0,1].map(()=>call('/billing/checkout',{method:'POST',body:{priceId:'price_monthly',requestId:id}})));
  assert.equal(remoteCheckouts,1);
  const overview=await call('/billing/overview');assert.equal(overview.planId,'free');assert.equal(overview.usage.hasAllowance,false);assert.equal(overview.pendingCheckout,true);
});

test('signature failures and stale notifications never grant access; paid invoice grants once even under different event IDs',async()=>{
  const customer=customers.get(owner.user.id);
  subscriptions.set('sub_fixture',{id:'sub_fixture',customer,status:'active',current_period_end:Math.floor(Date.now()/1000)+2592000,cancel_at_period_end:false,items:{data:[{price:{id:'price_monthly'}}]}});
  invoices.set('in_fixture',{id:'in_fixture',customer,subscription:'sub_fixture',status:'paid',amount_paid:1900,currency:'usd',created:Math.floor(Date.now()/1000),billing_reason:'subscription_create',lines:{data:[{price:{id:'price_monthly'},proration:false}]}});
  const paid=event('evt_paid','invoice.paid',{id:'in_fixture',subscription:'sub_fixture'});
  await webhook(paid,{signature:'t=1,v1='+'a'.repeat(64),expected:400});
  await webhook(paid,{timestamp:Math.floor(Date.now()/1000)-3600,expected:400});
  assert.equal((await call('/billing/overview')).planId,'free');
  await webhook(paid);await webhook(paid);await webhook({...paid,id:'evt_second_delivery'});
  const balance=(await pool.query('SELECT granted FROM fr_ai_accounts WHERE user_id=$1',[owner.user.id])).rows[0];assert.equal(Number(balance.granted),50000);
  const overview=await call('/billing/overview');assert.equal(overview.planId,'pro');assert.equal(overview.usage.remainingPercent,100);assert.equal(overview.invoices.length,1);
  assert.equal(overview.pendingCheckout,false);
  await call('/billing/cancel-checkout',{method:'POST',body:{},status:409});
  const foreign=await call('/billing/overview',{account:other});assert.equal(foreign.planId,'free');assert.equal(foreign.invoices.length,0);
  await webhook({...paid,data:{object:{id:'in_other',subscription:'sub_fixture'}}},{expected:409});
});

test('refunds and disputes revoke quota idempotently with spent shortfall recorded',async()=>{
  const customer=customers.get(owner.user.id);
  // 全额退款：in_fixture 授予的 50000 全部撤回。
  const refunded=event('evt_refund_1','charge.refunded',{id:'ch_fixture',refunded:true,invoice:'in_fixture',currency:'usd',refunds:{data:[{id:'re_fixture',amount:1900}]}});
  await webhook(refunded);
  let overview=await call('/billing/overview');assert.equal(overview.usage.totalCredits,0);assert.equal(overview.usage.hasAllowance,false);
  // 幂等：同事件重投 + 换事件 ID 重投同一退款对象，均不重复扣减。
  await webhook(refunded);await webhook({...refunded,id:'evt_refund_replayed'});
  const refunds=(await pool.query('SELECT kind,amount,tokens_due,tokens_revoked FROM fr_billing_refunds')).rows;
  assert.equal(refunds.length,1);assert.equal(refunds[0].kind,'refund');assert.equal(refunds[0].amount,'1900');
  assert.equal(refunds[0].tokens_due,50000);assert.equal(refunds[0].tokens_revoked,50000);
  overview=await call('/billing/overview');assert.equal(overview.usage.totalCredits,0);
  // 新周期发票再次授予 50000；消费 40000 后半额退款：应撤 25000，仅可撤 10000，缺口留档。
  invoices.set('in_second',{id:'in_second',customer,subscription:'sub_fixture',status:'paid',amount_paid:1900,currency:'usd',created:Math.floor(Date.now()/1000),billing_reason:'subscription_cycle',lines:{data:[{price:{id:'price_monthly'},proration:false}]}});
  await webhook(event('evt_paid_2','invoice.paid',{id:'in_second',subscription:'sub_fixture'}));
  overview=await call('/billing/overview');assert.equal(overview.usage.totalCredits,50);
  await pool.query('UPDATE fr_ai_accounts SET spent=40000 WHERE user_id=$1',[owner.user.id]);
  await webhook(event('evt_refund_2','charge.refunded',{id:'ch_second',refunded:true,invoice:'in_second',currency:'usd',refunds:{data:[{id:'re_second',amount:950}]}}));
  const second=(await pool.query("SELECT tokens_due,tokens_revoked FROM fr_billing_refunds WHERE id='re_second'")).rows[0];
  assert.equal(second.tokens_due,25000);assert.equal(second.tokens_revoked,10000);
  overview=await call('/billing/overview');assert.equal(overview.usage.totalCredits,40);
  // 争议只带 charge 引用：经 charge 回查定位发票并撤回剩余可用。
  charges.set('ch_second_full',{id:'ch_second_full',customer,invoice:'in_second',amount:1900,currency:'usd'});
  await pool.query('UPDATE fr_ai_accounts SET spent=0 WHERE user_id=$1',[owner.user.id]);
  await webhook(event('evt_dispute','charge.dispute.created',{id:'dp_fixture',charge:'ch_second_full',amount:1900,currency:'usd'}));
  const dispute=(await pool.query("SELECT kind,tokens_due,tokens_revoked FROM fr_billing_refunds WHERE id='dp_fixture'")).rows[0];
  assert.equal(dispute.kind,'dispute');assert.equal(dispute.tokens_due,50000);assert.equal(dispute.tokens_revoked,40000);
  overview=await call('/billing/overview');assert.equal(overview.usage.totalCredits,0);
  // 未知发票的退款拒绝核对。
  await webhook(event('evt_refund_unknown','charge.refunded',{id:'ch_unknown',refunded:true,invoice:'in_missing',currency:'usd',refunds:{data:[{id:'re_unknown',amount:100}]}}),{expected:409});
  // 恢复共享夹具（granted=50000/spent=0），维持后续取消测试的原有前置。
  await pool.query('UPDATE fr_ai_accounts SET granted=50000, spent=0, reserved=0 WHERE user_id=$1',[owner.user.id]);
});

test('cancellation and old event delivery use current remote state; portal is account-scoped and active subscriptions reject another purchase',async()=>{
  await call('/billing/checkout',{method:'POST',body:{priceId:'price_yearly',requestId:randomUUID()},status:409});
  assert.match((await call('/billing/manage',{method:'POST',body:{}})).url,/^https:\/\/billing.stripe.com\//);
  await call('/billing/manage',{method:'POST',account:other,body:{},status:404});
  subscriptions.get('sub_fixture').status='canceled';
  await webhook(event('evt_cancel','customer.subscription.deleted',{id:'sub_fixture'}));
  await webhook(event('evt_delayed_update','customer.subscription.updated',{id:'sub_fixture'}));
  assert.equal((await call('/billing/overview')).status,'canceled');
  assert.equal(Number((await pool.query('SELECT granted FROM fr_ai_accounts WHERE user_id=$1',[owner.user.id])).rows[0].granted),50000);
});

test('unconfigured sales stay closed and public configuration cannot expose secrets',async()=>{
  disabledApi=await createApi({pool,origin:'http://127.0.0.1:1424',store:new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY,'billing-disabled'))});
  await disabledApi.app.listen(0,'127.0.0.1');
  const address=await disabledApi.app.getUrl();
  const catalog=await fetch(address+'/api/v2/billing/catalog').then(r=>r.json());assert.equal(catalog.salesAvailable,false);assert.deepEqual(catalog.plans[1].prices,[]);
  const result=await fetch(address+'/api/v2/billing/checkout',{method:'POST',headers:{'content-type':'application/json',authorization:'Bearer '+owner.token},body:JSON.stringify({priceId:'price_monthly',requestId:randomUUID()})});assert.equal(result.status,503);
});

test('payment configuration without an available FocalRoom AI service cannot sell an AI plan',async()=>{
  noAiApi=await createApi({pool,origin:'http://127.0.0.1:1424',store:new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY,'billing-no-ai')),billing:config,billingGateway:gateway});
  await noAiApi.app.listen(0,'127.0.0.1');const address=await noAiApi.app.getUrl();
  const catalog=await fetch(address+'/api/v2/billing/catalog').then(r=>r.json());assert.equal(catalog.salesAvailable,false);
  const result=await fetch(address+'/api/v2/billing/checkout',{method:'POST',headers:{'content-type':'application/json',authorization:'Bearer '+owner.token},body:JSON.stringify({priceId:'price_monthly',requestId:randomUUID()})});assert.equal(result.status,503);
});

test('price mismatch and unexpected payment destinations are rejected before redirect',async()=>{
  priceChanged=true;
  await assert.rejects(gateway.checkout(customers.get(owner.user.id),owner.user.id,config.prices[0],randomUUID(),'http://127.0.0.1:1424'),e=>e.code==='BILLING_PRICE_CHANGED');
  const malicious=new StripeBillingGateway(config,async path=>path.startsWith('/prices/')?{id:'price_monthly',unit_amount:1900,currency:'usd',active:true,recurring:{interval:'month',interval_count:1}}:{id:'cs_bad',url:'https://evil.test/pay',expires_at:9999999999});
  await assert.rejects(malicious.checkout('cus_x','u',config.prices[0],randomUUID(),'http://127.0.0.1:1424'),e=>e.code==='BILLING_RESPONSE_INVALID');
});

test('canceling an unpaid checkout expires only this account session and permits another plan',async()=>{
  priceChanged=false;
  await call('/billing/checkout',{method:'POST',account:other,body:{priceId:'price_monthly',requestId:randomUUID()}});
  const otherSession=[...sessions.values()].find(value=>value.customer===customers.get(other.user.id));
  await call('/billing/cancel-checkout',{method:'POST',body:{}});
  assert.equal(otherSession.status,'open','不能取消别人的付款');
  await call('/billing/cancel-checkout',{method:'POST',account:other,body:{}});
  assert.equal(otherSession.status,'expired');
  await call('/billing/checkout',{method:'POST',account:other,body:{priceId:'price_yearly',requestId:randomUUID()}});
  assert.equal((await call('/billing/overview',{account:other})).planId,'free','创建付款页面不授予订阅');
});

test('paused or incomplete subscriptions never display a renewal promise or receive AI allowance',async()=>{
  const customer=customers.get(other.user.id);
  for(const status of ['paused','incomplete']) {
    subscriptions.set('sub_paused',{id:'sub_paused',customer,status,current_period_end:Math.floor(Date.now()/1000)+2592000,cancel_at_period_end:false,items:{data:[{price:{id:'price_yearly'}}]}});
    await webhook(event('evt_'+status,'customer.subscription.updated',{id:'sub_paused'}));
    const overview=await call('/billing/overview',{account:other});
    assert.equal(overview.planId,'free');assert.equal(overview.currentPeriodEnd,null);assert.equal(overview.usage.availableCredits,0);
  }
});

test('closing new sales retains existing customer management and signed notification handling',async()=>{
  salesOffApi=await createApi({pool,origin:'http://127.0.0.1:1424',store:new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY,'billing-sales-off')),billing:{...config,salesEnabled:false},billingGateway:gateway,hostedAi:{key:'fixture-only',initialTokens:0}});
  await salesOffApi.app.listen(0,'127.0.0.1');const address=await salesOffApi.app.getUrl();
  const headers={'content-type':'application/json',authorization:'Bearer '+owner.token};
  const catalog=await fetch(address+'/api/v2/billing/catalog').then(r=>r.json());assert.equal(catalog.salesAvailable,false);
  const checkout=await fetch(address+'/api/v2/billing/checkout',{method:'POST',headers,body:JSON.stringify({priceId:'price_monthly',requestId:randomUUID()})});assert.equal(checkout.status,503);
  const portal=await fetch(address+'/api/v2/billing/manage',{method:'POST',headers,body:'{}'});assert.equal(portal.status,200);
  const body=JSON.stringify(event('evt_sales_off','customer.subscription.updated',{id:'sub_fixture'})),timestamp=Math.floor(Date.now()/1000);
  const signature='t='+timestamp+',v1='+createHmac('sha256',config.webhookSecret).update(timestamp+'.'+body).digest('hex');
  const notification=await fetch(address+'/api/v2/billing/webhook',{method:'POST',headers:{'content-type':'application/json','stripe-signature':signature},body});assert.equal(notification.status,200);
});
