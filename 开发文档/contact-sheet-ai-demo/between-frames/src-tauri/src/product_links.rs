//! 产品设置只允许打开固定的公开链接，不接收任意外部地址。
use crate::domain::{AppError, Result};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductLink {
    Website,
    Instagram,
    Plugins,
}
impl ProductLink {
    fn url(&self) -> &'static str {
        match self {
            Self::Website => "https://focroom.cc/",
            Self::Instagram => "https://www.instagram.com/YIHUAPHOTO/",
            Self::Plugins => "https://focroom.cc/plugins/",
        }
    }
}
#[tauri::command]
pub async fn open_product_link(link: ProductLink) -> Result<()> {
    open_system_url(link.url()).await
}

/// 调用者须先完成固定目的地验证；不作为 IPC 暴露任意 URL。
pub(crate) async fn open_system_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let value = url.to_owned();
        let ok = tokio::task::spawn_blocking(move || {
            std::process::Command::new("/usr/bin/open")
                .arg("--")
                .arg(value)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
        })
        .await
        .unwrap_or(false);
        if ok {
            Ok(())
        } else {
            Err(AppError::new(
                "BROWSER_OPEN_FAILED",
                "无法打开系统浏览器，请重试",
            ))
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = url;
        Err(AppError::new(
            "UNSUPPORTED_PLATFORM",
            "请在浏览器打开此链接",
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_known_public_destinations() {
        assert_eq!(ProductLink::Website.url(), "https://focroom.cc/");
        assert_eq!(
            ProductLink::Instagram.url(),
            "https://www.instagram.com/YIHUAPHOTO/"
        );
        assert_eq!(ProductLink::Plugins.url(), "https://focroom.cc/plugins/");
        assert!(serde_json::from_str::<ProductLink>("\"https://evil.test\"").is_err());
    }
}
