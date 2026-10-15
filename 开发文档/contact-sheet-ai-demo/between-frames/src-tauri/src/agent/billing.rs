// v2.0 M7 支付事件账本（B01/B02）：
// B01 webhook 重放幂等——同 event_id 返回首次回执，不重复结算；
// B02 客户端不可写账本——事件必须带服务端 HMAC 签名，伪造/篡改拒绝。
// HMAC 密钥来自环境/钥匙串（部署配置），客户端无路径调用写入口。
use crate::{
    db::Store,
    domain::{self, AppError, Result},
};
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::Row;

pub const BILLING_KEY_ENV: &str = "BETWEEN_FRAMES_BILLING_HMAC_KEY";

fn expected_signature(payload: &str, secret: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC 密钥");
    mac.update(payload.as_bytes());
    format!("{:x}", mac.finalize().into_bytes())
}

fn billing_secret() -> Result<String> {
    std::env::var(BILLING_KEY_ENV)
        .ok()
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| {
            AppError::new(
                "PERMISSION_DENIED",
                "账本 HMAC 密钥未配置；支付事件处理不可用（这是安全默认，不是故障）",
            )
        })
}

/// 处理 webhook（B01+B02）：验签→幂等→结算。payload 为原始事件 JSON 字符串。
pub async fn handle_webhook(store: &Store, payload: &str, signature: &str) -> Result<Value> {
    let secret = billing_secret()?;
    let expected = expected_signature(payload, &secret);
    // B02：签名不符（伪造或篡改）拒绝且不落任何行。
    let _ = expected;
    if signature.len() != expected.len() || signature != expected {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "签名校验失败：事件不是由可信支付服务签发",
        ));
    }
    let event: Value = serde_json::from_str(payload)
        .map_err(|_| AppError::new("VALIDATION_ERROR", "事件载荷不是合法 JSON"))?;
    let event_id = event["event_id"]
        .as_str()
        .ok_or_else(|| AppError::new("VALIDATION_ERROR", "缺少 event_id"))?
        .to_owned();
    let order_id = event["order_id"]
        .as_str()
        .ok_or_else(|| AppError::new("VALIDATION_ERROR", "缺少 order_id"))?
        .to_owned();
    let kind = event["kind"].as_str().unwrap_or_default().to_owned();
    if !matches!(
        kind.as_str(),
        "payment.succeeded" | "payment.refunded" | "subscription.updated"
    ) {
        return Err(AppError::new("VALIDATION_ERROR", "事件类型不合法"));
    }
    let amount = event["amount_minor"].as_i64().unwrap_or(-1);
    if amount < 0 {
        return Err(AppError::new("VALIDATION_ERROR", "金额不合法"));
    }
    let mut tx = store.pool.begin_with("BEGIN IMMEDIATE").await?;
    // B01：同 event_id 幂等——返回首次回执，不重复改单。
    if let Some(first) =
        sqlx::query("SELECT id, order_id, kind FROM v2_billing_events WHERE event_id=?")
            .bind(&event_id)
            .fetch_optional(&mut *tx)
            .await?
    {
        let (_, first_order, first_kind): (i64, String, String) =
            (first.get("id"), first.get("order_id"), first.get("kind"));
        tx.commit().await?;
        return Ok(
            json!({"replayed": true, "event_id": event_id, "order_id": first_order, "kind": first_kind, "settled": false}),
        );
    }
    let now = domain::timestamp();
    sqlx::query("INSERT INTO v2_billing_events(event_id, order_id, kind, amount_minor, signature, received_at) VALUES(?,?,?,?,?,?)")
        .bind(&event_id).bind(&order_id).bind(&kind).bind(amount).bind(signature).bind(&now)
        .execute(&mut *tx).await?;
    // 结算：仅真实存在的订单；状态机 open→paid / paid→refunded。
    let order: Option<(String, i64)> =
        sqlx::query_as("SELECT status, amount_minor FROM v2_orders WHERE id=?")
            .bind(&order_id)
            .fetch_optional(&mut *tx)
            .await?;
    let mut settled = false;
    if let Some((status, order_amount)) = order {
        let next = match (status.as_str(), kind.as_str()) {
            ("open", "payment.succeeded") if amount == order_amount => Some("paid"),
            ("paid", "payment.refunded") => Some("refunded"),
            _ => None,
        };
        if let Some(next) = next {
            sqlx::query("UPDATE v2_orders SET status=?, updated_at=? WHERE id=?")
                .bind(next)
                .bind(&now)
                .bind(&order_id)
                .execute(&mut *tx)
                .await?;
            settled = true;
        }
    }
    tx.commit().await?;
    Ok(
        json!({"replayed": false, "event_id": event_id, "order_id": order_id, "kind": kind, "settled": settled}),
    )
}

pub async fn create_order(
    store: &Store,
    workspace: &str,
    order_id: &str,
    plan: &str,
    amount_minor: i64,
) -> Result<()> {
    if !matches!(plan, "standard" | "pro" | "edu") || amount_minor < 0 {
        return Err(AppError::new("VALIDATION_ERROR", "订单参数不合法"));
    }
    sqlx::query("INSERT OR IGNORE INTO v2_orders(id, workspace_id, plan, amount_minor, updated_at) VALUES(?,?,?,?,?)")
        .bind(order_id).bind(workspace).bind(plan).bind(amount_minor).bind(domain::timestamp())
        .execute(&store.pool).await?;
    Ok(())
}

pub async fn order_status(store: &Store, order_id: &str) -> Result<Value> {
    let row = sqlx::query("SELECT status, plan, amount_minor FROM v2_orders WHERE id=?")
        .bind(order_id)
        .fetch_optional(&store.pool)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "订单不存在"))?;
    Ok(
        json!({"status": row.get::<String, _>("status"), "plan": row.get::<String, _>("plan"), "amount_minor": row.get::<i64, _>("amount_minor")}),
    )
}
