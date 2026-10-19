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
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn account_id(&self) -> Result<String> {
        self.read()?
            .session
            .map(|s| s.user.id)
            .ok_or_else(|| AppError::new("AUTH_REQUIRED", "请先登录 FocalRoom"))
    }
    pub async fn ai_account(&self) -> Result<Value> {
        let token = self.bearer(&self.origin).await?;
        self.request("/ai/account", None, Some(&token)).await
    }
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
    fn avatar_path(&self, user: &CloudUser) -> Option<std::path::PathBuf> {
        use sha2::Digest;
        let url = user.avatar.as_deref()?;
        let parsed = reqwest::Url::parse(url).ok()?;
        if url.len() > 2048
            || parsed.scheme() != "https"
            || !matches!(
                parsed.host_str(),
                Some(
                    "lh3.googleusercontent.com"
                        | "lh4.googleusercontent.com"
                        | "lh5.googleusercontent.com"
                        | "lh6.googleusercontent.com"
                )
            )
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.port().is_some()
            || parsed.fragment().is_some()
        {
            return None;
        }
        let key = format!(
            "{:x}",
            sha2::Sha256::digest(format!("{}\0{url}", user.id).as_bytes())
        );
        self.avatar_cache
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .map(|dir| dir.join(key))
    }
    fn avatar_mime(bytes: &[u8]) -> Option<&'static str> {
        if bytes.len() > 1_048_576 {
            return None;
        }
        let format = image::guess_format(bytes).ok()?;
        let mime = match format {
            image::ImageFormat::Png => "image/png",
            image::ImageFormat::Jpeg => "image/jpeg",
            image::ImageFormat::WebP => "image/webp",
            _ => return None,
        };
        let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(1024);
        limits.max_image_height = Some(1024);
        limits.max_alloc = Some(16 * 1024 * 1024);
        reader.limits(limits);
        reader.decode().ok()?;
        Some(mime)
    }
    fn cached_avatar(&self, user: &CloudUser) -> Option<String> {
        use std::io::Read;
        let file = std::fs::File::open(self.avatar_path(user)?).ok()?;
        let mut bytes = Vec::new();
        file.take(1_048_577).read_to_end(&mut bytes).ok()?;
        let mime = Self::avatar_mime(&bytes)?;
        use base64::Engine;
        Some(format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    }
    async fn refresh_avatar(&self, user: &CloudUser) {
        let Some(path) = self.avatar_path(user) else {
            return;
        };
        if self.cached_avatar(user).is_some() {
            return;
        }
        let Ok(mut response) = self
            .client
            .get(user.avatar.as_deref().unwrap_or(""))
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .and_then(|r| r.error_for_status())
        else {
            return;
        };
        let mut bytes = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    if bytes.len() + chunk.len() > 1_048_576 {
                        return;
                    }
                    bytes.extend_from_slice(&chunk);
                }
                Ok(None) => break,
                Err(_) => return,
            }
        }
        if Self::avatar_mime(&bytes).is_none() {
            return;
        }
        let Some(parent) = path.parent() else {
            return;
        };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let temporary = path.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        if let Ok(mut file) = options.open(&temporary) {
            use std::io::Write;
            if file.write_all(&bytes).is_ok() {
                let _ = std::fs::rename(&temporary, &path);
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
                    self.refresh_avatar(&value).await;
                    let mut value = value;
                    value.avatar = self.cached_avatar(&value);
                    user = Some(value);
                    authenticated = true;
                }
                Err(e) if e.code == "AUTH_REVOKED" => {
                    r.session = None;
                    self.save(&r)?;
                }
                Err(e) => {
                    // 离线/网关故障不视为登出：保留本地身份供 UI 展示并记录原因。
                    connection_error = Some(e.message);
                    let mut cached = s.user.clone();
                    cached.avatar = self.cached_avatar(&cached);
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
    #[test]
    fn cache_requires_trusted_url_valid_bounded_pixels_and_url_bound_path() {
        struct Vault;
        impl CredentialStore for Vault {
            fn read(&self, _: &str) -> Result<Option<String>> {
                Ok(None)
            }
            fn write(&self, _: &str, _: &str) -> Result<()> {
                Ok(())
            }
            fn delete(&self, _: &str) -> Result<()> {
                Ok(())
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let auth = CloudAuth::new("http://127.0.0.1:1441", "avatar-test", Arc::new(Vault)).unwrap();
        auth.set_avatar_cache(dir.path().to_owned());
        let mut user = CloudUser {
            id: "../../outside".into(),
            email: "fixture@test.test".into(),
            name: "Fixture".into(),
            avatar: Some("https://lh3.googleusercontent.com/a/one".into()),
        };
        let path = auth.avatar_path(&user).unwrap();
        assert_eq!(path.parent(), Some(dir.path()));
        std::fs::write(&path, [0x89, 0x50, 0x4e, 0x47]).unwrap();
        assert!(auth.cached_avatar(&user).is_none());
        let image = image::DynamicImage::new_rgb8(2, 2);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        std::fs::write(&path, bytes.into_inner()).unwrap();
        assert!(
            auth.cached_avatar(&user)
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        user.avatar = Some("https://lh3.googleusercontent.com/a/two".into());
        assert_ne!(auth.avatar_path(&user).unwrap(), path);
        assert!(auth.cached_avatar(&user).is_none());
        for value in [
            "http://lh3.googleusercontent.com/a",
            "https://evil.test/a",
            "https://lh3.googleusercontent.com.evil.test/a",
        ] {
            user.avatar = Some(value.into());
            assert!(auth.avatar_path(&user).is_none());
        }
        assert!(CloudAuth::avatar_mime(&vec![0; 1_048_577]).is_none());
    }
}
