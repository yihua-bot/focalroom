//! 订阅会话在后端创建；桌面只打开严格限定的支付页面。
use crate::domain::{AppError, Result};

pub(crate) async fn open_payment(value: &serde_json::Value, manage: bool) -> Result<()> {
    let url = payment_url(value["url"].as_str().unwrap_or(""), manage)?;
    crate::product_links::open_system_url(&url).await
}
fn payment_url(value: &str, manage: bool) -> Result<String> {
    let url = reqwest::Url::parse(value)
        .map_err(|_| AppError::new("BILLING_URL_INVALID", "无法打开付款页面"))?;
    let host = if manage {
        "billing.stripe.com"
    } else {
        "checkout.stripe.com"
    };
    if url.scheme() != "https"
        || url.host_str() != Some(host)
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || !url.path().starts_with(if manage { "/p/" } else { "/c/" })
    {
        return Err(AppError::new("BILLING_URL_INVALID", "无法打开付款页面"));
    }
    Ok(url.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_expected_https_payment_pages_can_open() {
        assert!(payment_url("https://checkout.stripe.com/c/pay/test", false).is_ok());
        assert!(payment_url("https://billing.stripe.com/p/session/test", true).is_ok());
        for value in [
            "http://checkout.stripe.com/c/pay/x",
            "https://checkout.stripe.com.evil.test/c/pay/x",
            "https://a:secret@checkout.stripe.com/c/pay/x",
            "https://checkout.stripe.com:8443/c/pay/x",
            "https://checkout.stripe.com/redirect",
            "focalroom://callback?token=fake",
        ] {
            assert!(payment_url(value, false).is_err(), "不应打开 {value}");
        }
        assert!(payment_url("https://billing.stripe.com/p/session/test", false).is_err());
    }
}
