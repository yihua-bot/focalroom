//! 登录事务绑定来源、PKCE verifier 和 state；深链不携带会话凭据。
use crate::domain::{AppError, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CLOUD_ORIGIN: &str = "https://team.focroom.cc";
pub fn hex64(value: &str) -> bool { value.len()==64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }
pub fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() }
fn random() -> String { format!("{}{}",uuid::Uuid::new_v4().simple(),uuid::Uuid::new_v4().simple()) }
pub fn trusted_origin(value: &str) -> Result<String> {
    let u=reqwest::Url::parse(value).map_err(|_| AppError::new("CONFIG_INVALID","登录服务地址不合法"))?;
    let local=matches!(u.host_str(),Some("localhost"|"127.0.0.1"|"[::1]"));
    if !(u.origin().ascii_serialization()==CLOUD_ORIGIN || (local && u.scheme()=="http" && u.port()!=Some(0)))
        || !u.username().is_empty() || u.password().is_some() || u.query().is_some() || u.fragment().is_some() || u.path()!="/" {
        return Err(AppError::new("CONFIG_INVALID","登录只允许官方服务或精确的本机开发来源"));
    }
    Ok(u.origin().ascii_serialization())
}
#[derive(Serialize,Deserialize)]
pub struct LoginGuard {
    pub origin: String,
    pub request_id: String,
    state_hash: String,
    pub verifier: String,
    expires: u64,
}
impl LoginGuard {
    pub fn issue(origin: &str, time: u64) -> Result<(Self,String,String)> {
        let state=random(); let verifier=random();
        let challenge=URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Ok((Self { origin:trusted_origin(origin)?,request_id:String::new(),state_hash:format!("{:x}",Sha256::digest(state.as_bytes())),verifier,expires:time+900 },state,challenge))
    }
    pub fn validate(&self,url: &str,time: u64) -> Result<(String,String)> {
        if time>=self.expires { return Err(AppError::new("AUTH_EXPIRED","登录已过期，请重新发起")); }
        let u=reqwest::Url::parse(url).map_err(|_| AppError::new("AUTH_CALLBACK_INVALID","登录回调不合法"))?;
        if u.scheme()!="focalroom" || u.host_str()!=Some("callback") || !matches!(u.path(),""|"/") || u.port().is_some() || !u.username().is_empty() || u.password().is_some() || u.fragment().is_some() {
            return Err(AppError::new("AUTH_CALLBACK_INVALID","登录回调不合法"));
        }
        let pairs:Vec<_>=u.query_pairs().collect();
        let code=pairs.iter().find(|(k,_)|k=="code").map(|(_,v)|v.as_ref()).unwrap_or("");
        let state=pairs.iter().find(|(k,_)|k=="state").map(|(_,v)|v.as_ref()).unwrap_or("");
        if pairs.len()!=2 || !hex64(code) || !hex64(state) || format!("{:x}",Sha256::digest(state.as_bytes()))!=self.state_hash {
            return Err(AppError::new("AUTH_STATE_MISMATCH","登录回调与本次请求不匹配"));
        }
        Ok((code.to_owned(),state.to_owned()))
    }
}
pub async fn open_browser(url: &str) -> Result<()> {
    let u=reqwest::Url::parse(url).map_err(|_| AppError::new("BROWSER_OPEN_FAILED","登录地址不合法"))?;
    trusted_origin(&u.origin().ascii_serialization())?;
    if u.path()!="/shared" { return Err(AppError::new("BROWSER_OPEN_FAILED","登录地址不合法")); }
    #[cfg(target_os="macos")]
    {
        let value=url.to_owned();
        let ok=tokio::task::spawn_blocking(move || std::process::Command::new("/usr/bin/open").arg("--").arg(value).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok_and(|s|s.success())).await.unwrap_or(false);
        if ok { Ok(()) } else { Err(AppError::new("BROWSER_OPEN_FAILED","无法打开系统浏览器，请重试")) }
    }
    #[cfg(not(target_os="macos"))]
    { Err(AppError::new("UNSUPPORTED_PLATFORM","当前平台尚未接入系统浏览器登录")) }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn callback_binds_state_path_fields_and_expiry() {
        let (guard,state,_)=LoginGuard::issue(CLOUD_ORIGIN,100).unwrap();
        let good=format!("focalroom://callback?code={}&state={state}","a".repeat(64));
        assert!(guard.validate(&good,101).is_ok());
        for bad in [good.replace("callback","other"),good.clone()+"&code=duplicate",good.clone()+"#fragment",good.replace(&state,&"b".repeat(64)),good.replace("code=","token=")] { assert!(guard.validate(&bad,101).is_err()); }
        assert_eq!(guard.validate(&good,1000).unwrap_err().code,"AUTH_EXPIRED");
    }
    #[test] fn auth_origins_are_exact_and_pending_survives_serialization() {
        for bad in ["http://localhost.evil.test","http://127.0.0.1.evil.test","https://evil.test","https://team.focroom.cc:444","https://user@team.focroom.cc","https://team.focroom.cc/path"] { assert!(trusted_origin(bad).is_err(),"{bad}"); }
        assert!(trusted_origin("http://127.0.0.1:1424").is_ok());
        let (guard,state,challenge)=LoginGuard::issue(CLOUD_ORIGIN,100).unwrap();
        assert_eq!(URL_SAFE_NO_PAD.encode(Sha256::digest(guard.verifier.as_bytes())),challenge);
        let restored:LoginGuard=serde_json::from_str(&serde_json::to_string(&guard).unwrap()).unwrap();
        assert!(restored.validate(&format!("focalroom://callback?code={}&state={state}","c".repeat(64)),101).is_ok());
    }
}
