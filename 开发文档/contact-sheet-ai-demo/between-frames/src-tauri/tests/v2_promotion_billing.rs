// v2.0 S01/B01/B02：Promotion 单权威、webhook 重放幂等、伪造签名拒绝。
use between_frames_lib::{
    agent::{billing, promotion},
    db::Store,
    image_pipeline,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::path::Path;

fn sign(payload: &str, key: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).unwrap();
    mac.update(payload.as_bytes());
    format!("{:x}", mac.finalize().into_bytes())
}

async fn project_with_photo(path: &Path) -> (Store, String) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let project = db.create("迁移", "").await.unwrap();
    let source = path.join("one.png");
    image::RgbImage::from_pixel(32, 24, image::Rgb([200, 100, 50]))
        .save(&source)
        .unwrap();
    db.insert_photo(
        &project.id,
        &image_pipeline::prepare(&source, &path.join("cache")).unwrap(),
    )
    .await
    .unwrap();
    (db, project.id)
}

#[tokio::test]
async fn s01_promotion_single_authority_no_dual_master() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = project_with_photo(temp.path()).await;
    // 初始 local。
    assert_eq!(
        promotion::authority_of(&db, &project).await.unwrap()["authority"],
        "local"
    );
    // 并发第二个 promotion 被拒。
    let first = promotion::begin(&db, &project, "local-owner")
        .await
        .unwrap();
    let second = promotion::begin(&db, &project, "local-owner")
        .await
        .unwrap_err();
    assert_eq!(second.code, "INVALID_TRANSITION");
    // 迁移中：local-promoting（本地写入暂停语义）。
    assert_eq!(
        promotion::authority_of(&db, &project).await.unwrap()["authority"],
        "local-promoting"
    );
    let id = first["promotion_id"].as_str().unwrap();
    // 跳级/无回执 completed 拒绝。
    let jump = promotion::advance(&db, id, "completed", Some("cloud-ok"))
        .await
        .unwrap_err();
    assert_eq!(jump.code, "INVALID_TRANSITION");
    // 正常推进：uploading → verifying（无回执 completed 拒）→ 带回执 completed。
    promotion::advance(&db, id, "uploading", None)
        .await
        .unwrap();
    promotion::advance(&db, id, "verifying", None)
        .await
        .unwrap();
    let no_receipt = promotion::advance(&db, id, "completed", None)
        .await
        .unwrap_err();
    assert_eq!(no_receipt.code, "INVALID_TRANSITION");
    promotion::advance(&db, id, "completed", Some("cloud-confirmation-1"))
        .await
        .unwrap();
    assert_eq!(
        promotion::authority_of(&db, &project).await.unwrap()["authority"],
        "shared"
    );
    // completed 后不能再动（终止态）。
    let done = promotion::advance(&db, id, "aborted", None)
        .await
        .unwrap_err();
    assert_eq!(done.code, "INVALID_TRANSITION");
    // failed 后可重新 begin：authority 回落 local。
    let db2 = project_with_photo(temp.path()).await.0; // 不同项目验证失败路径
    let p2 = db2.list(false).await.unwrap();
    let other = p2.iter().find(|p| p.id != project).unwrap();
    let f = promotion::begin(&db2, &other.id, "local-owner")
        .await
        .unwrap();
    promotion::advance(&db2, f["promotion_id"].as_str().unwrap(), "failed", None)
        .await
        .unwrap();
    assert_eq!(
        promotion::authority_of(&db2, &other.id).await.unwrap()["authority"],
        "local"
    );
}

#[tokio::test]
async fn b01_b02_webhook_replay_idempotent_and_forgery_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    unsafe { std::env::set_var(billing::BILLING_KEY_ENV, "test-hmac-secret") };
    billing::create_order(&db, "local-workspace", "order-1", "pro", 9900)
        .await
        .unwrap();
    let payload = r#"{"event_id":"evt-1","order_id":"order-1","kind":"payment.succeeded","amount_minor":9900}"#;
    let signature = sign(payload, "test-hmac-secret");
    // B02：伪造签名拒绝，账本零行。
    let forged = billing::handle_webhook(&db, payload, "deadbeef")
        .await
        .unwrap_err();
    assert_eq!(forged.code, "PERMISSION_DENIED");
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v2_billing_events")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    // 篡改金额后原签名失效。
    let tampered =
        r#"{"event_id":"evt-1","order_id":"order-1","kind":"payment.succeeded","amount_minor":1}"#;
    let tampered_result = billing::handle_webhook(&db, tampered, &signature)
        .await
        .unwrap_err();
    assert_eq!(tampered_result.code, "PERMISSION_DENIED");
    // 正常结算：open→paid。
    let paid = billing::handle_webhook(&db, payload, &signature)
        .await
        .unwrap();
    assert_eq!(paid["settled"], true);
    assert_eq!(
        billing::order_status(&db, "order-1").await.unwrap()["status"],
        "paid"
    );
    // B01：重放同 event 幂等——不重复结算、不新增行。
    let replay = billing::handle_webhook(&db, payload, &signature)
        .await
        .unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["settled"], false);
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v2_billing_events")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);
    // 退款事件签名链：paid→refunded；金额不匹配的 succeeded 不结算。
    let refund = r#"{"event_id":"evt-2","order_id":"order-1","kind":"payment.refunded","amount_minor":9900}"#;
    let refund_result = billing::handle_webhook(&db, refund, &sign(refund, "test-hmac-secret"))
        .await
        .unwrap();
    assert_eq!(refund_result["settled"], true);
    assert_eq!(
        billing::order_status(&db, "order-1").await.unwrap()["status"],
        "refunded"
    );
    unsafe { std::env::remove_var(billing::BILLING_KEY_ENV) };
}
