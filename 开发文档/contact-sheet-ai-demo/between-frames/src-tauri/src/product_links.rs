//! 产品设置只允许打开固定的公开链接，不接收任意外部地址。
use crate::domain::{AppError, Result};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductLink {
    Website,
    Instagram,
}
impl ProductLink {
    fn url(&self) -> &'static str {
        match self {
            Self::Website => "https://focroom.cc/",
            Self::Instagram => "https://www.instagram.com/YIHUAPHOTO/",
        }
    }
}
#[tauri::command]
pub async fn open_product_link(link: ProductLink) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let ok = tokio::task::spawn_blocking(move || {
            std::process::Command::new("/usr/bin/open")
                .arg("--")
                .arg(link.url())
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
        let _ = link;
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
        assert!(serde_json::from_str::<ProductLink>("\"https://evil.test\"").is_err());
    }
}
