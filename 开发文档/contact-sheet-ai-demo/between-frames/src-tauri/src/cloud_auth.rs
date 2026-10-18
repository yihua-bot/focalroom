//! 浏览器授权、PKCE兑换和安全会话；只向GUI返回账户状态。
use crate::{cloud_credentials::CredentialStore,domain::{AppError,Result},login_guard::{self,LoginGuard}};
use serde::{Serialize,Deserialize};
use serde_json::{Value,json};
use std::sync::Arc;
#[derive(Clone,Serialize,Deserialize)]
pub struct CloudUser {pub id:String,pub email:String,pub name:String}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct CloudStatus {pub authenticated:bool,pub waiting:bool,pub origin:String,pub user:Option<CloudUser>,pub error:Option<String>}
#[derive(Serialize,Deserialize)]
struct Session {token:String,user:CloudUser}
#[derive(Default,Serialize,Deserialize)]
struct Record {pending:Option<LoginGuard>,session:Option<Session>}
pub struct CloudAuth {origin:String,service:String,vault:Arc<dyn CredentialStore>,client:reqwest::Client,lock:tokio::sync::Mutex<()>,error:std::sync::Mutex<Option<String>>}
impl CloudAuth {
    pub fn new(origin:&str,namespace:&str,vault:Arc<dyn CredentialStore>)->Result<Self> {
        let origin=login_guard::trusted_origin(origin)?;
        Ok(Self {service:format!("{namespace}.cloud-session"),origin,vault,client:reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(std::time::Duration::from_secs(10)).build().map_err(|_|AppError::new("CLOUD_CONFIG","登录网络初始化失败"))?,lock:Default::default(),error:Default::default()})
    }
    fn read(&self)->Result<Record> { self.vault.read(&self.service)?.map(|v|serde_json::from_str(&v).map_err(|_|AppError::new("KEYCHAIN_ERROR","登录存储格式错误"))).unwrap_or_else(||Ok(Record::default())) }
    fn save(&self,r:&Record)->Result<()> { if r.pending.is_none()&&r.session.is_none(){self.vault.delete(&self.service)}else{self.vault.write(&self.service,&serde_json::to_string(r).map_err(|_|AppError::new("KEYCHAIN_ERROR","无法保存登录状态"))?)} }
    async fn request(&self,path:&str,body:Option<Value>,token:Option<&str>)->Result<Value> {
        let url=format!("{}/api/v2{path}",self.origin);
        let mut req=if let Some(body)=body {self.client.post(url).json(&body)}else{self.client.get(url)};
        if let Some(token)=token {req=req.bearer_auth(token);}
        let mut response=req.send().await.map_err(|_|AppError::new("CLOUD_UNAVAILABLE","无法连接云端，请检查网络后重试"))?;
        let status=response.status(); let mut bytes=Vec::new();
        while let Some(chunk)=response.chunk().await.map_err(|_|AppError::new("CLOUD_UNAVAILABLE","登录响应中断，请重新发起"))? {if bytes.len()+chunk.len()>128*1024{return Err(AppError::new("CLOUD_PROTOCOL","登录响应超过上限"));}bytes.extend_from_slice(&chunk);}
        if status==reqwest::StatusCode::UNAUTHORIZED {return Err(AppError::new("AUTH_REVOKED","登录已失效，请重新登录"));}
        if !status.is_success() {return Err(AppError::new("CLOUD_REJECTED","登录请求未完成，请重新发起"));}
        serde_json::from_slice(&bytes).map_err(|_|AppError::new("CLOUD_PROTOCOL","登录响应格式错误"))
    }
    fn user(value:Value)->Result<CloudUser> {
        let user:CloudUser=serde_json::from_value(value).map_err(|_|AppError::new("CLOUD_PROTOCOL","账户响应格式错误"))?;
        if user.id.is_empty()||user.id.len()>160||user.email.is_empty()||user.email.len()>254||user.name.len()>200 {return Err(AppError::new("CLOUD_PROTOCOL","账户响应格式错误"));}Ok(user)
    }
    pub async fn begin(&self)->Result<String> {
        let _lock=self.lock.lock().await;
        let (mut pending,state,challenge)=LoginGuard::issue(&self.origin,login_guard::now())?;
        let value=self.request("/auth/desktop/start",Some(json!({"state":state,"code_challenge":challenge})),None).await?;
        let id=value["request_id"].as_str().filter(|v|login_guard::hex64(v)).ok_or_else(||AppError::new("CLOUD_PROTOCOL","登录请求格式错误"))?;
        let mut url=reqwest::Url::parse(&format!("{}/shared",self.origin)).map_err(|_|AppError::new("CLOUD_CONFIG","登录地址错误"))?;
        url.query_pairs_mut().append_pair("desktop","1").append_pair("request",id).append_pair("state",&state);
        if value["login_url"].as_str()!=Some(url.as_str()){return Err(AppError::new("CLOUD_PROTOCOL","登录来源不匹配"));}
        pending.request_id=id.to_owned(); let mut r=self.read()?;r.pending=Some(pending);self.save(&r)?;
        *self.error.lock().unwrap_or_else(|p|p.into_inner())=None;Ok(url.into())
    }
    pub async fn cancel(&self)->Result<()> {let _lock=self.lock.lock().await;let mut r=self.read()?;r.pending=None;self.save(&r)}
    pub async fn complete(&self,url:&str)->Result<()> {
        let _lock=self.lock.lock().await;let mut r=self.read()?;
        let pending=r.pending.as_ref().ok_or_else(||AppError::new("AUTH_NO_SESSION","没有等待中的登录，请重新发起"))?;
        let (code,state)=pending.validate(url,login_guard::now())?;
        if pending.origin!=self.origin{return Err(AppError::new("AUTH_STATE_MISMATCH","登录来源已变化"));}
        let exchange=self.request("/auth/desktop/exchange",Some(json!({"request_id":pending.request_id,"code":code,"state":state,"code_verifier":pending.verifier})),None).await;
        // 一旦尝试兑换，不重放网络结果不明的授权码；重新发起新的事务。
        r.pending=None;self.save(&r)?;
        let value=exchange?;
        let token=value["token"].as_str().filter(|v|login_guard::hex64(v)).ok_or_else(||AppError::new("CLOUD_PROTOCOL","登录凭据格式错误"))?.to_owned();
        let user=Self::user(value["user"].clone())?;
        let verified=Self::user(self.request("/auth/me",None,Some(&token)).await?)?;
        if verified.id!=user.id{return Err(AppError::new("CLOUD_PROTOCOL","账户校验不匹配"));}
        r.session=Some(Session{token,user:verified});self.save(&r)?;
        *self.error.lock().unwrap_or_else(|p|p.into_inner())=None;Ok(())
    }
    pub fn remember_error(&self,error:&AppError) {*self.error.lock().unwrap_or_else(|p|p.into_inner())=Some(error.message.clone());}
    pub async fn status(&self)->Result<CloudStatus> {
        let _lock=self.lock.lock().await;let mut r=self.read()?;
        if r.pending.as_ref().is_some_and(|p|p.validate(&format!("focalroom://callback?code={}&state={}","0".repeat(64),"0".repeat(64)),login_guard::now()).is_err_and(|e|e.code=="AUTH_EXPIRED")) {r.pending=None;self.save(&r)?;}
        let mut user=None;
        if let Some(s)=&r.session {match self.request("/auth/me",None,Some(&s.token)).await {Ok(v)=>{let value=Self::user(v)?;if value.id!=s.user.id{return Err(AppError::new("CLOUD_PROTOCOL","账户校验不匹配"));}user=Some(value);},Err(e) if e.code=="AUTH_REVOKED"=>{r.session=None;self.save(&r)?;},Err(e)=>return Err(e)}}
        Ok(CloudStatus{authenticated:user.is_some(),waiting:r.pending.is_some(),origin:self.origin.clone(),user,error:self.error.lock().unwrap_or_else(|p|p.into_inner()).clone()})
    }
    pub async fn bearer(&self,origin:&str)->Result<String> {
        if origin!=self.origin{return Err(AppError::new("AUTH_ORIGIN_MISMATCH","请使用当前登录账户绑定的共享服务"));}
        let _lock=self.lock.lock().await;let r=self.read()?;let s=r.session.as_ref().ok_or_else(||AppError::new("AUTH_REQUIRED","请先登录云端"))?;
        let user=Self::user(self.request("/auth/me",None,Some(&s.token)).await?)?;
        if user.id!=s.user.id{return Err(AppError::new("AUTH_REVOKED","账户校验已失效"));}Ok(s.token.clone())
    }
    pub async fn logout(&self)->Result<bool> {let _lock=self.lock.lock().await;let r=self.read()?;let revoked=if let Some(s)=r.session{self.request("/auth/logout",Some(json!({})),Some(&s.token)).await.is_ok()}else{true};self.vault.delete(&self.service)?;Ok(revoked)}
}
