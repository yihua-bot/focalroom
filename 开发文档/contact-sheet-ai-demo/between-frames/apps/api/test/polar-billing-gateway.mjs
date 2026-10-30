import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createHmac,randomUUID} from 'node:crypto';
import {PolarBillingGateway,polarPaymentUrl,POLAR_API_VERSION} from '../src/polar-billing-gateway.ts';
import {BillingService} from '../src/billing.ts';
import {billingConfig} from '../src/billing-gateway.ts';

const org=randomUUID(),product=randomUUID(),customer=randomUUID(),subscription=randomUUID(),order=randomUUID(),sessionId=randomUUID();
const price={id:product,planId:'studio',interval:'month',amount:1999,currency:'USD',includedTokens:1000000,seats:3};
const config={provider:'polar',polarEnvironment:'sandbox',organizationId:org,secret:'polar_oat_local_fixture',webhookSecret:'whsec_'+Buffer.alloc(32,7).toString('base64'),prices:[price]};
const key=value=>'polar:sandbox:'+value;
const meta={application:'focalroom',focalroom_user_id:'local-user',focalroom_request_id:randomUUID()};
const session=()=>({id:sessionId,customer_id:customer,product_id:product,organization_id:org,status:'open',amount:1999,currency:'usd',url:'https://sandbox.polar.sh/checkout/local_fixture',expires_at:new Date(Date.now()+3600000).toISOString(),metadata:meta});
const productBody=()=>({id:product,organization_id:org,is_archived:false,is_recurring:true,recurring_interval:'month',recurring_interval_count:1,prices:[{id:randomUUID(),amount_type:'fixed',price_amount:1999,price_currency:'usd',is_archived:false}]});
function signed(type,data,{scheme='standard',timestamp=String(Math.floor(Date.now()/1000)),version=POLAR_API_VERSION}={}) {
 const body=Buffer.from(JSON.stringify({type,data,api_version:version})),webhookId='evt_'+randomUUID();
 const secret=scheme==='legacy'?Buffer.from(config.webhookSecret):Buffer.from(config.webhookSecret.slice(6),'base64');
 const signature=createHmac('sha256',secret).update(`${webhookId}.${timestamp}.`).update(body).digest('base64');
 return {body,headers:{'webhook-id':webhookId,'webhook-timestamp':timestamp,'webhook-signature':'v1,'+signature}};
}
test('Polar and Stripe configuration stay separate; missing credentials never enable subscriptions',()=>{
 const names=['FOCALROOM_BILLING_PROVIDER','FOCALROOM_BILLING_ENABLED','FOCALROOM_POLAR_ACCESS_TOKEN','FOCALROOM_POLAR_WEBHOOK_SECRET','FOCALROOM_BILLING_PRICES','FOCALROOM_POLAR_ORGANIZATION_ID','FOCALROOM_POLAR_ENVIRONMENT'];
 const old=Object.fromEntries(names.map(k=>[k,process.env[k]]));
 try {
  process.env.FOCALROOM_BILLING_PROVIDER='polar';process.env.FOCALROOM_BILLING_ENABLED='0';process.env.FOCALROOM_POLAR_ACCESS_TOKEN='';process.env.FOCALROOM_POLAR_WEBHOOK_SECRET='';
  assert.equal(billingConfig(),undefined);
  process.env.FOCALROOM_BILLING_ENABLED='1';assert.throws(()=>billingConfig());
  Object.assign(process.env,{FOCALROOM_POLAR_ACCESS_TOKEN:config.secret,FOCALROOM_POLAR_WEBHOOK_SECRET:config.webhookSecret,FOCALROOM_POLAR_ORGANIZATION_ID:org,FOCALROOM_POLAR_ENVIRONMENT:'sandbox',FOCALROOM_BILLING_PRICES:JSON.stringify([price])});
  assert.equal(billingConfig().provider,'polar');
  process.env.FOCALROOM_BILLING_PRICES=JSON.stringify([{...price,id:'openyc-product'}]);assert.throws(()=>billingConfig());
 }finally{for(const [k,v] of Object.entries(old)){if(v===undefined)delete process.env[k];else process.env[k]=v;}}
});
test('catalog and account plan use explicit price mapping; no invented prices or remote-id prefix guessing',async()=>{
 const pro={...price,id:randomUUID(),planId:'pro'},both={...config,prices:[pro,price]};
 const service=new BillingService({query:async sql=>({rows:sql.includes('FROM fr_billing_subscriptions')?[{price_id:product,status:'active',current_period_end:new Date(Date.now()+3600000),cancel_at_period_end:false}]:[],rowCount:0})},
  {authenticate:async()=>({id:'local-user'})},both,undefined,true);
 const catalog=service.catalog();
 assert.deepEqual(catalog.plans.map(p=>[p.id,p.prices.map(v=>v.id)]),[['free',[]],['pro',[pro.id]],['studio',[product]]]);
 assert.equal((await service.overview({})).planId,'studio');assert.equal((await service.overview({})).canCancelCheckout,false);
});
test('strict official payment URLs distinguish sandbox, production, checkout and portal',()=>{
 assert.equal(polarPaymentUrl(session().url,false,'sandbox'),session().url);
 assert.ok(polarPaymentUrl('https://polar.sh/focalroom/portal?customer_session_token=fixture',true,'production'));
 for(const url of ['http://polar.sh/checkout/x','https://polar.sh.evil.test/checkout/x','https://a:b@polar.sh/checkout/x','https://polar.sh:8443/checkout/x','https://polar.sh/checkout/x/redirect','https://polar.sh/other','https://sandbox.polar.sh/checkout/x'])assert.throws(()=>polarPaymentUrl(url,false,'production'));
});
test('checkout verifies actual product, merchant, amount and cadence, and binds identity before returning URL',async()=>{
 const writes=[];
 const gateway=new PolarBillingGateway(config,async(path,method,body)=>{
  if(path==='/products/'+product)return productBody();
  assert.equal(path,'/checkouts/');assert.equal(method,'POST');writes.push(body);return session();
 });
 const result=await gateway.checkout(key(customer),'local-user',price,meta.focalroom_request_id,'https://team.focroom.cc');
 assert.equal(result.id,key(sessionId));assert.equal(writes.length,1);assert.equal(writes[0].customer_id,customer);assert.equal(writes[0].metadata.application,'focalroom');assert.equal(writes[0].allow_trial,false);
 for(const mutate of [p=>p.organization_id=randomUUID(),p=>p.prices[0].price_amount=1,p=>p.recurring_interval='year']) {
  let posted=false;
  const bad=new PolarBillingGateway(config,async(_p,m)=>{if(m==='POST')posted=true;const p=productBody();mutate(p);return p;});
  await assert.rejects(bad.checkout(key(customer),'local-user',price,meta.focalroom_request_id,'https://team.focroom.cc'));assert.equal(posted,false);
 }
});
test('lost checkout response is recovered by read-only lookup; missing, duplicate and foreign sessions do not create another checkout',async()=>{
 let writes=0;
 for(const items of [[session()],[],[session(),session()],[{...session(),customer_id:randomUUID()}]]) {
  const gateway=new PolarBillingGateway(config,async(_path,method)=>{if(method==='POST')writes++;return {items,pagination:{max_page:1}};});
  const attempt=gateway.recoverCheckout(key(customer),'local-user',price,meta.focalroom_request_id);
  if(items.length===1&&items[0].customer_id===customer)assert.equal((await attempt).id,key(sessionId));else await assert.rejects(attempt);
 }
 assert.equal(writes,0);
});
test('CNY checkout selects the approved currency even when the merchant catalog also requires USD',async()=>{
 const cny={...price,planId:'pro',amount:9900,currency:'CNY'};
 const body=productBody();body.prices.push({id:randomUUID(),amount_type:'fixed',price_amount:9900,price_currency:'cny',is_archived:false});
 const writes=[];
 const gateway=new PolarBillingGateway({...config,prices:[cny]},async(path,method,value)=>{
  if(method==='GET'){assert.equal(path,'/products/'+product);return body;}
  writes.push(value);return {...session(),amount:9900,currency:'cny'};
 });
 await gateway.checkout(key(customer),'local-user',cny,meta.focalroom_request_id,'https://team.focroom.cc');
 assert.equal(writes.length,1);assert.equal(writes[0].currency,'cny');assert.deepEqual(writes[0].products,[product]);
 assert.equal('product_price_id' in writes[0],false);assert.equal('prices' in writes[0],false);
});
test('multi-currency catalog never falls back to USD or accepts ambiguous, changed or metered prices',async()=>{
 const cny={...price,amount:19900,currency:'CNY'};
 const entry=()=>({id:randomUUID(),amount_type:'fixed',price_amount:19900,price_currency:'cny',is_archived:false});
 for(const mutate of [p=>p,p=>p.prices.push({...entry(),price_amount:19901}),p=>p.prices.push(entry(),entry()),p=>p.prices.push({...entry(),amount_type:'metered_unit'}),p=>p.prices.push({...entry(),is_archived:true})]) {
  const p=productBody();mutate(p);let posted=false;
  const gateway=new PolarBillingGateway({...config,prices:[cny]},async(_path,method)=>{if(method==='POST')posted=true;return p;});
  await assert.rejects(gateway.checkout(key(customer),'local-user',cny,meta.focalroom_request_id,'https://team.focroom.cc'),e=>e.code==='BILLING_PRICE_CHANGED');
  assert.equal(posted,false);
 }
});
test('customer external IDs are namespaced and recovery does not create a duplicate customer',async()=>{
 const gateway=new PolarBillingGateway(config,async(path,method)=>{
  assert.equal(method,'GET');assert.ok(new URL('https://fixture.invalid'+path).searchParams.get('query').startsWith('focalroom:'));
  return {items:[{id:customer,external_id:'focalroom:local-user'}],pagination:{max_page:1}};
 });
 assert.equal(await gateway.createCustomer('local-user','fixture@fr.test'),key(customer));
});
test('one Polar product maps CNY and USD to separate local prices for checkout, subscription and invoice',async()=>{
 const usd={...price,id:'studio_month_usd',productId:product};
 const cny={...price,id:'studio_month_cny',productId:product,amount:19900,currency:'CNY'};
 for(const selected of [cny,usd]) {
  const p=productBody();p.prices.push({id:randomUUID(),amount_type:'fixed',price_amount:19900,price_currency:'cny',is_archived:false});
  const gateway=new PolarBillingGateway({...config,prices:[cny,usd]},async(path,method,body)=>{
   if(path==='/products/'+product)return p;
   if(method==='POST'){assert.deepEqual(body.products,[product]);assert.equal(body.currency,selected.currency.toLowerCase());return {...session(),amount:selected.amount,currency:selected.currency.toLowerCase()};}
   if(path.startsWith('/subscriptions/'))return {id:subscription,customer_id:customer,product_id:product,status:'active',amount:selected.amount,currency:selected.currency.toLowerCase(),recurring_interval:'month',recurring_interval_count:1,current_period_end:new Date(Date.now()+3600000).toISOString(),cancel_at_period_end:false,customer:{external_id:'focalroom:local-user'},metadata:meta};
   return {id:order,customer_id:customer,product_id:product,subscription_id:subscription,paid:true,status:'paid',total_amount:selected.amount,net_amount:selected.amount,currency:selected.currency.toLowerCase(),created_at:new Date().toISOString(),billing_reason:'subscription_cycle',metadata:meta};
  });
  await gateway.checkout(key(customer),'local-user',selected,meta.focalroom_request_id,'https://team.focroom.cc');
  assert.equal((await gateway.subscription(key(subscription))).priceId,selected.id);
  assert.equal((await gateway.invoice(key(order))).priceId,selected.id);
  const event=signed('order.paid',{id:order,subscription_id:subscription,product_id:product});
  assert.equal(gateway.event(event.body,'',event.headers).invoiceId,key(order));
 }
});
test('portal is bound to stored customer; active checkouts cannot falsely report canceled',async()=>{
 const gateway=new PolarBillingGateway(config,async(path,_method,body)=>{
  if(path==='/customer-sessions/'){assert.equal(body.customer_id,customer);return {customer_id:customer,customer_portal_url:'https://sandbox.polar.sh/focalroom/portal?customer_session_token=fixture'};}
  return session();
 });
 assert.ok((await gateway.manage(key(customer),'https://team.focroom.cc')).includes('/portal'));
 await assert.rejects(gateway.cancelCheckout(key(sessionId),key(customer)),e=>e.code==='BILLING_PAYMENT_PENDING');
 await assert.rejects(gateway.manage('cus_openyc','https://team.focroom.cc'),e=>e.code==='BILLING_PROVIDER_MISMATCH');
});
test('Standard Webhooks and explicit legacy keys validate raw bytes and reject tampering, stale signatures and wrong versions',()=>{
 const gateway=new PolarBillingGateway(config);
 const value=signed('order.paid',{id:order,subscription_id:subscription,product_id:product});
 const result=gateway.event(value.body,'',value.headers);assert.equal(result.subscriptionId,key(subscription));assert.equal(result.invoiceId,key(order));
 assert.throws(()=>gateway.event(Buffer.from(value.body+' '),'',value.headers));
 const stale=signed('order.paid',{}, {timestamp:'1'});assert.throws(()=>gateway.event(stale.body,'',stale.headers));
 const next=signed('order.paid',{}, {version:'2027-01'});assert.throws(()=>gateway.event(next.body,'',next.headers));
 const legacy=signed('order.paid',{id:order,subscription_id:subscription,product_id:product},{scheme:'legacy'});
 assert.throws(()=>gateway.event(legacy.body,'',legacy.headers));
 assert.equal(new PolarBillingGateway({...config,polarWebhookScheme:'legacy'}).event(legacy.body,'',legacy.headers).invoiceId,key(order));
});
test('subscription-created cannot issue invoice credits; foreign products are ignored; refunds keep stable provider identity',()=>{
 const gateway=new PolarBillingGateway(config);
 for(const [type,data] of [['subscription.created',{id:subscription,product_id:product}],['order.paid',{id:order,subscription_id:subscription,product_id:randomUUID()}]]) {
  const s=signed(type,data),value=gateway.event(s.body,'',s.headers);assert.equal(value.invoiceId,null);
 }
 const refundId=randomUUID(),refund=signed('refund.updated',{id:refundId,order_id:order,amount:500,currency:'usd',status:'succeeded'});
 const result=gateway.event(refund.body,'',refund.headers);assert.equal(result.refund.id,key(refundId));assert.equal(result.refund.invoiceId,key(order));
 const pending=signed('refund.updated',{id:refundId,order_id:order,amount:500,currency:'usd',status:'pending'});assert.equal(gateway.event(pending.body,'',pending.headers).refund,null);
 const taxed=signed('refund.updated',{id:refundId,order_id:order,amount:500,tax_amount:100,currency:'usd',status:'succeeded'});assert.equal(gateway.event(taxed.body,'',taxed.headers).refund.amount,600);
 const cumulative=signed('order.refunded',{id:order,product_id:product});assert.throws(()=>gateway.event(cumulative.body,'',cumulative.headers),e=>e.code==='BILLING_RECONCILIATION_REQUIRED');
});
test('subscription and order reads validate canonical billing identity and retain period boundaries',async()=>{
 const gateway=new PolarBillingGateway(config,async path=>path.startsWith('/subscriptions/')?
  {id:subscription,customer_id:customer,product_id:product,status:'active',amount:1999,currency:'usd',recurring_interval:'month',recurring_interval_count:1,current_period_end:new Date(Date.now()+3600000).toISOString(),cancel_at_period_end:true,customer:{external_id:'focalroom:local-user'},metadata:meta}:
  {id:order,customer_id:customer,product_id:product,subscription_id:subscription,paid:true,status:'paid',total_amount:1999,net_amount:1999,currency:'usd',created_at:new Date().toISOString(),billing_reason:'subscription_cycle',metadata:meta});
 assert.equal((await gateway.subscription(key(subscription))).cancelAtPeriodEnd,true);
 assert.equal((await gateway.invoice(key(order))).billingReason,'subscription_cycle');
});
