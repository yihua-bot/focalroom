//! 浏览器授权、PKCE兑换和安全会话；只向GUI返回账户状态。
use crate::{
    cloud_credentials::CredentialStore,
    domain::{AppError, Result},
    login_guard::{self, LoginGuard},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
#[derive(Clone, Serialize, Deserialize)]
pub struct CloudUser {
    pub id: String,
    pub email: String,
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudStatus {
    pub authenticated: bool,
    pub waiting: bool,
    pub origin: String,
    pub user: Option<CloudUser>,
    pub error: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Session {
    token: String,
    user: CloudUser,
}
#[derive(Default, Serialize, Deserialize)]
struct Record {
    pending: Option<LoginGuard>,
    session: Option<Session>,
}
pub struct CloudAuth {
    origin: String,
    service: String,
    vault: Arc<dyn CredentialStore>,
    client: reqwest::Client,
    lock: tokio::sync::Mutex<()>,
    error: std::sync::Mutex<Option<String>>,
    avatar_cache: std::sync::Mutex<Option<std::path::PathBuf>>,
}
impl CloudAuth {
    pub fn new(origin: &str, namespace: &str, vault: Arc<dyn CredentialStore>) -> Result<Self> {
        let origin = login_guard::trusted_origin(origin)?;
        Ok(Self {
            service: format!("{namespace}.cloud-session"),
            origin,
            vault,
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|_| AppError::new("CLOUD_CONFIG", "登录网络初始化失败"))?,
            lock: Default::default(),
            error: Default::default(),
            avatar_cache: Default::default(),
        })
    }
    fn read(&self) -> Result<Record> {
        self.vault
            .read(&self.service)?
            .map(|v| {
                serde_json::from_str(&v)
                    .map_err(|_| AppError::new("KEYCHAIN_ERROR", "登录存储格式错误"))
            })
            .unwrap_or_else(|| Ok(Record::default()))
    }
    fn save(&self, r: &Record) -> Result<()> {
        if r.pending.is_none() && r.session.is_none() {
            self.vault.delete(&self.service)
        } else {
            self.vault.write(
                &self.service,
                &serde_json::to_string(r)
                    .map_err(|_| AppError::new("KEYCHAIN_ERROR", "无法保存登录状态"))?,
            )
        }
    }
    async fn request(&self, path: &str, body: Option<Value>, token: Option<&str>) -> Result<Value> {
        let url = format!("{}/api/v2{path}", self.origin);
        let mut req = if let Some(body) = body {
            self.client.post(url).json(&body)
        } else {
            self.client.get(url)
        };
        if let Some(token) = token {
            req = req.bearer_auth(token);
        }
        let mut response = req
            .send()
            .await
            .map_err(|_| AppError::new("CLOUD_UNAVAILABLE", "无法连接云端，请检查网络后重试"))?;
        let status = response.status();
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| AppError::new("CLOUD_UNAVAILABLE", "登录响应中断，请重新发起"))?
        {
            if bytes.len() + chunk.len() > 128 * 1024 {
                return Err(AppError::new("CLOUD_PROTOCOL", "登录响应超过上限"));
            }
            bytes.extend_from_slice(&chunk);
        }
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(AppError::new("AUTH_REVOKED", "登录已失效，请重新登录"));
        }
        if !status.is_success() {
            return Err(AppError::new(
                "CLOUD_REJECTED",
                "登录请求未完成，请重新发起",
            ));
        }
        serde_json::from_slice(&bytes)
            .map_err(|_| AppError::new("CLOUD_PROTOCOL", "登录响应格式错误"))
    }
    fn user(value: Value) -> Result<CloudUser> {
        let user: CloudUser = serde_json::from_value(value)
            .map_err(|_| AppError::new("CLOUD_PROTOCOL", "账户响应格式错误"))?;
        if user.id.is_empty()
            || user.id.len() > 160
            || user.email.is_empty()
            || user.email.len() > 254
            || user.name.len() > 200
        {
            return Err(AppError::new("CLOUD_PROTOCOL", "账户响应格式错误"));
        }
        Ok(user)
    }
    pub async fn begin(&self) -> Result<String> {
        let _lock = self.lock.lock().await;
        let (mut pending, state, challenge) = LoginGuard::issue(&self.origin, login_guard::now())?;
        let value = self
            .request(
                "/auth/desktop/start",
                Some(json!({"state":state,"code_challenge":challenge})),
                None,
            )
            .await?;
        let id = value["request_id"]
            .as_str()
            .filter(|v| login_guard::hex64(v))
            .ok_or_else(|| AppError::new("CLOUD_PROTOCOL", "登录请求格式错误"))?;
        let mut url = reqwest::Url::parse(&format!("{}/shared", self.origin))
            .map_err(|_| AppError::new("CLOUD_CONFIG", "登录地址错误"))?;
        url.query_pairs_mut()
            .append_pair("desktop", "1")
            .append_pair("request", id)
            .append_pair("state", &state);
        if value["login_url"].as_str() != Some(url.as_str()) {
            return Err(AppError::new("CLOUD_PROTOCOL", "登录来源不匹配"));
        }
        pending.request_id = id.to_owned();
        let mut r = self.read()?;
        r.pending = Some(pending);
        self.save(&r)?;
        *self.error.lock().unwrap_or_else(|p| p.into_inner()) = None;
        Ok(url.into())
    }
    pub async fn cancel(&self) -> Result<()> {
        let _lock = self.lock.lock().await;
        let mut r = self.read()?;
        r.pending = None;
        self.save(&r)?;
        *self.error.lock().unwrap_or_else(|p| p.into_inner()) = None;
        Ok(())
    }
    pub async fn complete(&self, url: &str) -> Result<()> {
        let _lock = self.lock.lock().await;
        let mut r = self.read()?;
        let pending = r
            .pending
            .as_ref()
            .ok_or_else(|| AppError::new("AUTH_NO_SESSION", "没有等待中的登录，请重新发起"))?;
        let (code, state) = pending.validate(url, login_guard::now())?;
        if pending.origin != self.origin {
            return Err(AppError::new("AUTH_STATE_MISMATCH", "登录来源已变化"));
        }
        let exchange=self.request("/auth/desktop/exchange",Some(json!({"request_id":pending.request_id,"code":code,"state":state,"code_verifier":pending.verifier})),None).await;
        // 一旦尝试兑换，不重放网络结果不明的授权码；重新发起新的事务。
        r.pending = None;
        self.save(&r)?;
        let value = exchange?;
        let token = value["token"]
            .as_str()
            .filter(|v| login_guard::hex64(v))
            .ok_or_else(|| AppError::new("CLOUD_PROTOCOL", "登录凭据格式错误"))?
            .to_owned();
        let user = Self::user(value["user"].clone())?;
        let verified = Self::user(self.request("/auth/me", None, Some(&token)).await?)?;
        if verified.id != user.id {
            return Err(AppError::new("CLOUD_PROTOCOL", "账户校验不匹配"));
        }
        r.session = Some(Session {
            token,
            user: verified,
        });
        self.save(&r)?;
        *self.error.lock().unwrap_or_else(|p| p.into_inner()) = None;
        Ok(())
    }
    /// 注入头像缓存目录（app cache 目录下的 avatars/）。
    pub fn set_avatar_cache(&self, dir: std::path::PathBuf) {
        *self.avatar_cache.lock().unwrap_or_else(|p| p.into_inner()) = Some(dir);
    }
    fn avatar_paths(&self, id: &str) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        let dir = self
            .avatar_cache
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()?;
        Some((dir.join(id), dir.join(format!("{id}.url"))))
    }
    /// 读本地缓存头像为 data URL；URL 变化或未缓存返回 None。
    fn cached_avatar(&self, user: &CloudUser) -> Option<String> {
        let url = user.avatar.as_deref()?;
        let (data, marker) = self.avatar_paths(&user.id)?;
        let cached_url = std::fs::read_to_string(&marker).ok()?;
        if cached_url != url {
            return None;
        }
        let bytes = std::fs::read(&data).ok()?;
        if bytes.is_empty() {
            return None;
        }
        let mime = match bytes.first() {
            Some(0x89) => "image/png",
            Some(b'R') => "image/webp",
            _ => "image/jpeg",
        };
        use base64::Engine;
        Some(format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    }
    /// 在线时后台下载头像到缓存；失败静默（占位符兜底）。
    async fn refresh_avatar(&self, user: &CloudUser) {
        let Some(url) = user.avatar.clone() else {
            return;
        };
        let Some((data, marker)) = self.avatar_paths(&user.id) else {
            return;
        };
        if let (Ok(cached), Ok(bytes)) = (std::fs::read_to_string(&marker), std::fs::read(&data)) {
            if !bytes.is_empty() && cached == url {
                return;
            }
        }
        let fetched = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await
            .and_then(|r| r.error_for_status());
        if let Ok(response) = fetched {
            if let Ok(bytes) = response.bytes().await {
                if !bytes.is_empty() && bytes.len() <= 1_048_576 {
                    let _ = std::fs::write(&data, &bytes);
                    let _ = std::fs::write(&marker, &url);
                }
            }
        }
    }
    pub fn remember_error(&self, error: &AppError) {
        *self.error.lock().unwrap_or_else(|p| p.into_inner()) = Some(error.message.clone());
    }
    pub async fn status(&self) -> Result<CloudStatus> {
        let _lock = self.lock.lock().await;
        let mut r = self.read()?;
        if r.pending
            .as_ref()
            .is_some_and(|p| p.expired(login_guard::now()))
        {
            r.pending = None;
            self.save(&r)?;
        }

        let mut user = None;
        let mut authenticated = false;
        let mut connection_error = None;
        if let Some(s) = &r.session {
            match self.request("/auth/me", None, Some(&s.token)).await {
                Ok(v) => {
                    let value = Self::user(v)?;
                    if value.id != s.user.id {
                        return Err(AppError::new("CLOUD_PROTOCOL", "账户校验不匹配"));
                    }
                    let mut value = value;
                    if value.avatar.is_some() && !self.cached_avatar(&value).is_some() {
                        let mut probe = value.clone();
                        probe.avatar = None;
                        let _ = self.refresh_avatar(&probe).await;
                    }
                    if let Some(url) = self.cached_avatar(&value) {
                        value.avatar = Some(url);
                    }
                    user = Some(value);
                    authenticated = true;
                }
                Err(e) if e.code == "AUTH_REVOKED" => {
                    r.session = None;
                    self.save(&r)?;
                }
                Err(e) => {
                    // 离线/网关故障不视为登出：保留本地身份供 UI 展示并记录原因。
                    self.remember_error(&e);
                    let mut cached = s.user.clone();
                    cached.avatar = self.cached_avatar(&cached).or(if cached.avatar.is_some() {
                        None
                    } else {
                        None
                    });
                    user = Some(cached);
                }
            }
        }
        Ok(CloudStatus {
            authenticated,
            waiting: r.pending.is_some(),
            origin: self.origin.clone(),
            user,
            error: connection_error
                .or_else(|| self.error.lock().unwrap_or_else(|p| p.into_inner()).clone()),
        })
    }
    pub async fn bearer(&self, origin: &str) -> Result<String> {
        if origin != self.origin {
            return Err(AppError::new(
                "AUTH_ORIGIN_MISMATCH",
                "请使用当前登录账户绑定的共享服务",
            ));
        }
        let _lock = self.lock.lock().await;
        let r = self.read()?;
        let s = r
            .session
            .as_ref()
            .ok_or_else(|| AppError::new("AUTH_REQUIRED", "请先登录云端"))?;
        let user = Self::user(self.request("/auth/me", None, Some(&s.token)).await?)?;
        if user.id != s.user.id {
            return Err(AppError::new("AUTH_REVOKED", "账户校验已失效"));
        }
        Ok(s.token.clone())
    }
    pub async fn logout(&self) -> Result<bool> {
        let _lock = self.lock.lock().await;
        let r = self.read()?;
        let revoked = if let Some(s) = r.session {
            match self
                .request("/auth/logout", Some(json!({})), Some(&s.token))
                .await
            {
                Ok(_) => true,
                Err(e) => e.code == "AUTH_REVOKED",
            }
        } else {
            true
        };
        self.vault.delete(&self.service)?;
        *self.error.lock().unwrap_or_else(|p| p.into_inner()) = None;
        Ok(revoked)
    }
}

#[cfg(test)]
mod avatar_cache_tests {
    use super::*;
    fn user(avatar:Option<&str>)->CloudUser {CloudUser{id:"u1".into(),email:"a@b.c".into(),name:"摄影师".into(),avatar:avatar.map(str::to_string)}}
    #[test]
    fn cached_avatar_reads_file_and_detects_url_change() {
        let dir=std::env::temp_dir().join(format!("fr-avatar-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        struct MemVault(std::sync::Mutex<std::collections::HashMap<String,String>>);
        impl CredentialStore for MemVault {
            fn read(&self,service:&str)->Result<Option<String>>{Ok(self.0.lock().unwrap().get(service).cloned())}
            fn write(&self,service:&str,secret:&str)->Result<()>{self.0.lock().unwrap().insert(service.into(),secret.into());Ok(())}
            fn delete(&self,service:&str)->Result<()>{self.0.lock().unwrap().remove(service);Ok(())}
        }
        let auth=CloudAuth::new("http://127.0.0.1:1441","avatar-test",std::sync::Arc::new(MemVault(Default::default()))).unwrap();
        auth.set_avatar_cache(dir.clone());
        // 无 URL → None
        assert!(auth.cached_avatar(&user(None)).is_none());
        // 未缓存 → None
        let u=user(Some("https://lh3.example/a.jpg"));
        assert!(auth.cached_avatar(&u).is_none());
        // 写入 1x1 PNG + url 标记
        let png:[u8;8]=[0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A];
        std::fs::write(dir.join("u1"),png).unwrap();
        std::fs::write(dir.join("u1.url"),"https://lh3.example/a.jpg").unwrap();
        let url=auth.cached_avatar(&u).expect("应返回缓存头像");
        assert!(url.starts_with("data:image/png;base64,"),"mime 由魔数推断: {url}");
        // URL 变化 → None（旧缓存不冒充新头像）
        let changed=user(Some("https://lh3.example/b.jpg"));
        assert!(auth.cached_avatar(&changed).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
