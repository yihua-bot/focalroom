import {before,after,test} from 'node:test';
import assert from 'node:assert/strict';
import {randomUUID,createHmac} from 'node:crypto';
import {join} from 'node:path';
import {Pool} from 'pg';
import {createApi} from '../src/server.ts';
import {migrate} from '../src/database.ts';
import {LocalPreviewStore} from '../src/storage.ts';
import {PolarBillingGateway,POLAR_API_VERSION} from '../src/polar-billing-gateway.ts';

const org=randomUUID(),product=randomUUID(),subId=randomUUID(),orderId=randomUUID();
const config={provider:'polar',polarEnvironment:'sandbox',organizationId:org,secret:'polar_oat_local_fixture',webhookSecret:'whsec_'+Buffer.alloc(32,9).toString('base64'),
 prices:[{id:product,planId:'studio',interval:'month',amount:1999,currency:'USD',includedTokens:90000,seats:3}]};
const customers=new Map(),sessions=[],orders=new Map();
let pool,api,base,owner,other,lost=true,postCount=0,periodCancel=false;
const key=id=>'polar:sandbox:'+id;
const transport=async(path,method,body)=>{
 const url=new URL('https://fixture.invalid'+path);
 if(url.pathname==='/customers/'&&method==='GET')return {items:[...customers].filter(([u])=>url.searchParams.get('query')==='focalroom:'+u).map(([u,id])=>({id,external_id:'focalroom:'+u})),pagination:{max_page:1}};
 if(path==='/customers/'&&method==='POST'){const id=randomUUID();customers.set(body.metadata.focalroom_user_id,id);return {id,external_id:body.external_id};}
 if(path==='/products/'+product)return {id:product,organization_id:org,is_archived:false,is_recurring:true,recurring_interval:'month',recurring_interval_count:1,prices:[{id:randomUUID(),amount_type:'fixed',price_amount:1999,price_currency:'usd',is_archived:false}]};
 if(path==='/checkouts/'&&method==='POST'){
  postCount++;const id=randomUUID();const value={...body,id,customer_id:body.customer_id,product_id:product,organization_id:org,status:'open',amount:1999,currency:'usd',url:'https://sandbox.polar.sh/checkout/'+id,expires_at:new Date(Date.now()+3600000).toISOString()};sessions.push(value);
  if(lost){lost=false;throw new Error('local fixture: response lost after remote creation');}return value;
 }
 if(url.pathname==='/checkouts/'&&method==='GET')return {items:sessions.filter(s=>s.customer_id===url.searchParams.get('customer_id')),pagination:{max_page:1}};
 if(url.pathname.startsWith('/checkouts/')&&method==='GET'){const session=sessions.find(s=>s.id===url.pathname.split('/').pop());assert.ok(session);return session;}
 if(path==='/subscriptions/'+subId)return {id:subId,customer_id:customers.get(owner.user.id),product_id:product,status:'active',current_period_end:new Date(Date.now()+2592000000).toISOString(),cancel_at_period_end:periodCancel,amount:1999,currency:'usd',recurring_interval:'month',recurring_interval_count:1,customer:{external_id:'focalroom:'+owner.user.id},metadata:{application:'focalroom',focalroom_user_id:owner.user.id}};
 if(path.startsWith('/orders/')){const o=orders.get(path.split('/').pop());assert.ok(o);return o;}
 if(path==='/customer-sessions/')return {customer_id:body.customer_id,customer_portal_url:'https://sandbox.polar.sh/focalroom/portal?customer_session_token=fixture'};
 throw new Error('Unexpected local Polar fixture path '+path);
};
async function call(path,{method='GET',account=owner,body,status=200}={}){
 const response=await fetch(base+'/api/v2'+path,{method,headers:{'content-type':'application/json',...(account?{authorization:'Bearer '+account.token}:{})},...(body===undefined?{}:{body:JSON.stringify(body)})});
 const value=await response.json();assert.equal(response.status,status,JSON.stringify(value));return value;
}
async function webhook(type,data,{webhookId='evt_'+randomUUID(),status=200,tamper=false}={}){
 const body=JSON.stringify({api_version:POLAR_API_VERSION,type,data}),timestamp=String(Math.floor(Date.now()/1000));
 const signature=createHmac('sha256',Buffer.from(config.webhookSecret.slice(6),'base64')).update(`${webhookId}.${timestamp}.${body}`).digest('base64');
 const response=await fetch(base+'/api/v2/billing/webhook',{method:'POST',headers:{'content-type':'application/json','webhook-id':webhookId,'webhook-timestamp':timestamp,'webhook-signature':'v1,'+signature},body:body+(tamper?' ':'')});
 const value=await response.json();assert.equal(response.status,status,JSON.stringify(value));return value;
}
function paidOrder(id,reason='subscription_create'){
 return {id,customer_id:customers.get(owner.user.id),product_id:product,subscription_id:subId,paid:true,status:'paid',total_amount:1999,net_amount:1999,currency:'usd',created_at:new Date().toISOString(),billing_reason:reason,metadata:{application:'focalroom',focalroom_user_id:owner.user.id}};
}
before(async()=>{
 assert.ok(process.env.FOCALROOM_TEST_DATABASE_URL);pool=new Pool({connectionString:process.env.FOCALROOM_TEST_DATABASE_URL});await migrate(pool);
 api=await createApi({pool,origin:'http://127.0.0.1:1424',store:new LocalPreviewStore(join(process.env.FOCALROOM_TEST_DIRECTORY,'polar-previews')),billing:config,billingGateway:new PolarBillingGateway(config,transport),hostedAi:{key:'fixture-not-a-real-key',initialTokens:0},hostedAiTransport:async()=>{throw new Error('No external AI calls');}});
 await api.app.listen(0,'127.0.0.1');base=await api.app.getUrl();
 owner=await call('/auth/register',{method:'POST',account:null,body:{email:'polar-owner@fr.test',password:'polar-fixture-password',display_name:'摄影师'}});
 other=await call('/auth/register',{method:'POST',account:null,body:{email:'polar-other@fr.test',password:'polar-fixture-password',display_name:'另一用户'}});
 await pool.query('UPDATE fr_users SET email_verified_at=now() WHERE id=ANY($1)',[[owner.user.id,other.user.id]]);
});
after(async()=>{await api?.close();await pool?.end();});
test('Polar checkout requires login and own verified account; catalog exposes only mapped product',async()=>{
 const catalog=await call('/billing/catalog',{account:null});assert.deepEqual(catalog.plans.map(p=>p.prices.length),[0,0,1]);
 await call('/billing/checkout',{method:'POST',account:null,body:{priceId:product,requestId:randomUUID()},status:401});
 assert.equal(postCount,0);
});
test('response loss and concurrent retries recover one remote checkout without granting credits',async()=>{
 const requestId=randomUUID();
 await call('/billing/checkout',{method:'POST',body:{priceId:product,requestId},status:500});
 const recovered=await Promise.all([0,1].map(()=>call('/billing/checkout',{method:'POST',body:{priceId:product,requestId:randomUUID()}})));
 assert.equal(recovered[0].url,recovered[1].url);assert.equal(postCount,1);
 const overview=await call('/billing/overview');assert.equal(overview.planId,'free');assert.equal(overview.usage.availableCredits,0);assert.equal(overview.canCancelCheckout,false);
 await call('/billing/cancel-checkout',{method:'POST',body:{},status:409});
});
test('only authenticated paid orders grant credits, exactly once across webhook replay and changed delivery ID',async()=>{
 const data={id:orderId,product_id:product,subscription_id:subId};orders.set(orderId,paidOrder(orderId));
 await webhook('order.paid',data,{tamper:true,status:400});assert.equal((await call('/billing/overview')).usage.availableCredits,0);
 const delivery='evt_'+randomUUID();await webhook('order.paid',data,{webhookId:delivery});await webhook('order.paid',data,{webhookId:delivery});await webhook('order.paid',data);
 const current=await call('/billing/overview');assert.equal(current.planId,'studio');assert.equal(current.usage.availableCredits,90);assert.equal(current.invoices.length,1);assert.equal(current.pendingCheckout,false);
 assert.equal((await call('/billing/overview',{account:other})).usage.availableCredits,0);
 await webhook('order.paid',{...data,product_id:randomUUID()});assert.equal((await call('/billing/overview')).usage.availableCredits,90);
 const portal=await call('/billing/manage',{method:'POST',body:{}});assert.match(portal.url,/sandbox\.polar\.sh\/focalroom\/portal/);
});
test('renewal grants one new period; scheduled cancellation retains already paid period and does not grant again',async()=>{
 const renewal=randomUUID();orders.set(renewal,paidOrder(renewal,'subscription_cycle'));
 await webhook('order.paid',{id:renewal,subscription_id:subId,product_id:product});assert.equal((await call('/billing/overview')).usage.availableCredits,180);
 periodCancel=true;await webhook('subscription.updated',{id:subId,product_id:product});
 const current=await call('/billing/overview');assert.equal(current.cancelAtPeriodEnd,true);assert.equal(current.usage.availableCredits,180);
});
test('partial refund and notification redelivery revoke quota once; forged cheap invoice grants nothing',async()=>{
 const refund=randomUUID(),data={id:refund,order_id:orderId,amount:1000,currency:'usd',status:'succeeded'};
 await webhook('refund.updated',data);const first=(await call('/billing/overview')).usage.availableCredits;
 assert.ok(first<180&&first>90);await webhook('refund.updated',data);assert.equal((await call('/billing/overview')).usage.availableCredits,first);
 const cheap=randomUUID();orders.set(cheap,{...paidOrder(cheap),net_amount:1,total_amount:1});
 await webhook('order.paid',{id:cheap,subscription_id:subId,product_id:product},{status:409});assert.equal((await call('/billing/overview')).usage.availableCredits,first);
 const rows=await pool.query('SELECT id FROM fr_billing_refunds');assert.equal(rows.rowCount,1);assert.equal(rows.rows[0].id,key(refund));
});
