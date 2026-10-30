//! 订阅会话在后端创建；桌面只打开严格限定的支付页面。
//! 支持 Stripe 和 Polar 两个支付网关。
use crate::domain::{AppError, Result};

pub(crate) async fn open_payment(value: &serde_json::Value, manage: bool) -> Result<()> {
    let url = payment_url(value["url"].as_str().unwrap_or(""), manage)?;
    crate::product_links::open_system_url(&url).await
}
fn payment_url(value: &str, manage: bool) -> Result<String> {
    let url = reqwest::Url::parse(value)
        .map_err(|_| AppError::new("BILLING_URL_INVALID", "无法打开付款页面"))?;

    // 安全检查（所有网关共用）
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppError::new("BILLING_URL_INVALID", "无法打开付款页面"));
    }

    let host = url.host_str().unwrap_or("");
    let path = url.path();

    // Stripe：checkout.stripe.com/c/... 或 billing.stripe.com/p/...
    let stripe_ok = if manage {
        host == "billing.stripe.com" && path.starts_with("/p/")
    } else {
        host == "checkout.stripe.com" && path.starts_with("/c/")
    };

    // Polar：polar.sh/checkout/...（购买）或 polar.sh/{org}/portal...（管理）
    let polar_ok = if manage {
        host == "polar.sh" && path.split('/').nth(2) == Some("portal")
    } else {
        host == "polar.sh" && path.starts_with("/checkout/")
    };

    if !stripe_ok && !polar_ok {
        return Err(AppError::new("BILLING_URL_INVALID", "无法打开付款页面"));
    }
    Ok(url.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_expected_https_payment_pages_can_open() {
        // Stripe
        assert!(payment_url("https://checkout.stripe.com/c/pay/test", false).is_ok());
        assert!(payment_url("https://billing.stripe.com/p/session/test", true).is_ok());
        // Polar
        assert!(payment_url("https://polar.sh/checkout/polar_c_abc123", false).is_ok());
        assert!(payment_url("https://polar.sh/my-org/portal", true).is_ok());
        assert!(payment_url("https://polar.sh/my-org/portal/sub", true).is_ok());
        // 拒绝
        for value in [
            "http://checkout.stripe.com/c/pay/x",
            "http://polar.sh/checkout/x",
            "https://checkout.stripe.com.evil.test/c/pay/x",
            "https://a:secret@checkout.stripe.com/c/pay/x",
            "https://checkout.stripe.com:8443/c/pay/x",
            "https://checkout.stripe.com/redirect",
            "https://polar.sh/evil/path",
            "https://evil-polar.sh/checkout/x",
            "focalroom://callback?token=fake",
        ] {
            assert!(payment_url(value, false).is_err(), "不应打开 {value}");
        }
        // 管理页不能当购买页
        assert!(payment_url("https://billing.stripe.com/p/session/test", false).is_err());
        assert!(payment_url("https://polar.sh/org/portal", false).is_err());
    }
}
