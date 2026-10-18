//! M4 桌面端云登录：深链回调 + 登录门卫（state nonce 防伪造）。
//! 流程与 openyc 桌面端一致：
//! 1. issue_login_url() 生成 nonce → 打开浏览器
//! 2. 用户在 Web 登录 → Web 重定向 focalroom://callback?token=xxx&state=nonce
//! 3. handle_callback() 校验 state 匹配 → 存储 token
use crate::domain::{AppError, Result};
use sha2::{Digest, Sha256};
use std::time::Instant;

const STATE_TTL_SECS: u64 = 900; // 15 分钟

pub struct LoginGuard {
    state_hash: Option<String>,
    initiated: Option<Instant>,
    consumed: bool,
}

impl LoginGuard {
    pub fn new() -> Self {
        Self { state_hash: None, initiated: None, consumed: false }
    }

    /// 发起登录：生成 nonce 并返回带 state 参数的登录 URL。
    pub fn issue(&mut self, login_url: &str) -> String {
        let nonce = uuid::Uuid::new_v4().simple().to_string()
            + &uuid::Uuid::new_v4().simple().to_string();
        self.state_hash = Some(Sha256::digest(nonce.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>());
        self.initiated = Some(Instant::now());
        self.consumed = false;
        let sep = if login_url.contains('?') { "&" } else { "?" };
        format!("{login_url}{sep}state={nonce}")
    }

    /// 校验回调：state 匹配 + 未过期 + 未消费。
    pub fn validate(&mut self, token: &str, state: &str) -> Result<String> {
        if self.consumed {
            return Err(AppError::new("AUTH_REPLAYED", "登录回调已被使用"));
        }
        let initiated = self.initiated
            .ok_or_else(|| AppError::new("AUTH_NO_SESSION", "没有发起中的登录会话"))?;
        if initiated.elapsed().as_secs() > STATE_TTL_SECS {
            return Err(AppError::new("AUTH_EXPIRED", "登录会话已过期，请重新发起"));
        }
        let expected = self.state_hash.clone()
            .ok_or_else(|| AppError::new("AUTH_NO_SESSION", "没有登录状态"))?;
        let actual = Sha256::digest(state.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>();
        if expected != actual {
            return Err(AppError::new("AUTH_STATE_MISMATCH", "登录回调 state 不匹配（可能的伪造攻击）"));
        }
        if token.is_empty() {
            return Err(AppError::new("AUTH_NO_TOKEN", "回调未包含 token"));
        }
        self.consumed = true; // 一次性，防重放
        Ok(token.to_owned())
    }
}

/// 安全打开系统浏览器（macOS 用 /usr/bin/open，防 Shell 注入）。
pub async fn open_browser(url: &str) -> Result<()> {
    let url = url.to_owned();
    // 只允许 https:// 或 http://localhost
    let parsed = reqwest::Url::parse(&url)
        .map_err(|_| AppError::new("BROWSER_OPEN_FAILED", "URL 不合法"))?;
    let is_safe = parsed.scheme() == "https"
        || (parsed.scheme() == "http" && matches!(parsed.host_str(), Some(h) if h.starts_with("localhost") || h.starts_with("127.0.0.1")));
    if !is_safe {
        return Err(AppError::new("BROWSER_OPEN_FAILED", "仅允许 HTTPS 或 localhost URL"));
    }
    #[cfg(target_os = "macos")]
    {
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("/usr/bin/open")
                .arg("--")
                .arg(url)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
        .await
        .map_err(|_| AppError::new("BROWSER_OPEN_FAILED", "无法启动浏览器"))?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(AppError::new("UNSUPPORTED_PLATFORM", "请复制登录地址到系统浏览器"))
    }
}
