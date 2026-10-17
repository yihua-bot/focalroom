//! 本地夹具直连回环服务，不经过系统代理转发测试凭据。
//! 旧 SQLite 适配不具备 v2 权威；真实桌面→PostgreSQL 流程由 apps/api/test/native-promotion.mjs 验证。
use reqwest::Client;
use serde_json::{json, Value};

async fn spawn() -> (
    String,
    sqlx::SqlitePool,
    tempfile::TempDir,
    tokio::task::JoinHandle<()>,
) {
    let temp = tempfile::tempdir().unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let dir = temp.path().join("previews");
    std::fs::create_dir(&dir).unwrap();
    let app = bf_cloud::build_router(pool.clone(), dir).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (base, pool, temp, task)
}
#[tokio::test]
async fn sqlite_v2_login_never_authenticates_by_email_alone() {
    let (base, pool, _temp, task) = spawn().await;
    let client = Client::builder().no_proxy().build().unwrap();
    client
        .post(format!("{base}/auth/register"))
        .json(&json!({"email":"owner@v2.test","display_name":"owner","password":"password123"}))
        .send()
        .await
        .unwrap();
    for password in ["password123", "wrong-password", ""] {
        let response = client
            .post(format!("{base}/api/v2/auth/login"))
            .json(&json!({"email":"owner@v2.test","password":password}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 501);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["error"]["code"], "UNSUPPORTED_AUTHORITY");
        assert!(body.get("token").is_none());
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    task.abort();
}
#[tokio::test]
async fn sqlite_v2_never_registers_uploads_or_finalizes_an_unverified_manifest() {
    let (base, pool, temp, task) = spawn().await;
    let client = Client::builder().no_proxy().build().unwrap();
    let account: Value = client
        .post(format!("{base}/auth/register"))
        .json(&json!({"email":"owner@reject.test","display_name":"owner","password":"password123"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token = account["token"].as_str().unwrap();
    for path in [
        "/api/v2/promotions",
        "/api/v2/projects/foreign/finalize",
        "/api/v2/projects/foreign/abort",
    ] {
        let response = client
            .post(format!("{base}{path}"))
            .bearer_auth(token)
            .json(&json!({"manifest_digest":"wrong","snapshot":{"photos":[]}}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 501);
    }
    let upload = client
        .put(format!(
            "{base}/api/v2/projects/foreign/previews/../../original"
        ))
        .bearer_auth(token)
        .header("content-type", "image/jpeg")
        .body(vec![0xff, 0xd8])
        .send()
        .await
        .unwrap();
    assert!(!upload.status().is_success());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cloud_projects")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        std::fs::read_dir(temp.path().join("previews"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        client
            .get(format!("{base}/api/v2/workspaces"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    task.abort();
}
