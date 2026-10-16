//! M4 垂直切片集成测试：真实 HTTP 验证 S02（撤销即拒）。
use reqwest::{Client, Response};
use serde_json::{json, Value};

fn val(r: Value, k: &str) -> String {
    r[k].as_str()
        .unwrap_or_else(|| panic!("字段 {k} 缺失: {r}"))
        .to_owned()
}

async fn spawn_server() -> (
    String,
    std::path::PathBuf,
    sqlx::SqlitePool,
    tempfile::TempDir,
) {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("test.sqlite");
    let preview_dir = temp.path().join("previews");
    std::fs::create_dir_all(&preview_dir).unwrap();
    let _ = &db_path;
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let dir = preview_dir.clone();
    let router = bf_cloud::build_router(pool.clone(), dir).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (format!("http://{addr}"), preview_dir, pool, temp)
}

async fn register(client: &Client, base: &str, email: &str, name: &str) -> String {
    let resp: Response = client
        .post(format!("{base}/auth/register"))
        .json(&json!({"email": email, "display_name": name, "password": "password123"}))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    assert_eq!(status, 200, "注册 {email} 应成功: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    format!("Bearer {}", val(v, "token"))
}

#[tokio::test]
async fn s02_vertical_slice_full_flow() {
    let (base, preview_dir, pool, _guard) = spawn_server().await;
    let client = Client::new();

    let owner_token = register(&client, &base, "owner@test.dev", "Owner").await;
    let reviewer_token = register(&client, &base, "reviewer@test.dev", "Reviewer").await;

    // 创建项目。
    let resp = client
        .post(format!("{base}/projects"))
        .header("authorization", &owner_token)
        .json(&json!({"title": "测试项目"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let project: Value = resp.json().await.unwrap();
    let pid = val(project, "id");

    // 注册预览文件。
    let sha = "a".repeat(64);
    // 使用 spawn_server 返回的共享 pool。
    sqlx::query("INSERT INTO preview_files(id,project_id,sha256,width,height,file_size) VALUES(?,?,?,?,?,?)")
        .bind(uuid::Uuid::new_v4().to_string()).bind(&pid).bind(&sha).bind(1400).bind(933).bind(45000)
        .execute(&pool).await.unwrap();
    tokio::fs::write(
        preview_dir.join(format!("{sha}-1400.jpg")),
        vec![0xFF, 0xD8, 0xFF, 0xE0],
    )
    .await
    .unwrap();

    // 未授权拒绝。
    let resp = client
        .get(format!("{base}/projects/{pid}/previews"))
        .header("authorization", &reviewer_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "未授权必须拒绝");

    // Owner 授权。
    let resp = client
        .post(format!("{base}/projects/{pid}/grants"))
        .header("authorization", &owner_token)
        .json(&json!({"grantee_email": "reviewer@test.dev", "role": "reviewer"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "授权应成功");
    let grant: Value = resp.json().await.unwrap();
    let gid = val(grant, "grant_id");

    // 授权后能读。
    let resp = client
        .get(format!("{base}/projects/{pid}/previews"))
        .header("authorization", &reviewer_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "授权后应能列预览");
    let resp = client
        .get(format!("{base}/projects/{pid}/previews/{sha}.jpg"))
        .header("authorization", &reviewer_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "授权后能读文件");

    // Owner 撤销。
    let resp = client
        .delete(format!("{base}/projects/{pid}/grants/{gid}"))
        .header("authorization", &owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // S02 核心：撤销后立即拒绝。
    let resp = client
        .get(format!("{base}/projects/{pid}/previews"))
        .header("authorization", &reviewer_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "S02：撤销后列表必须拒绝");
    let resp = client
        .get(format!("{base}/projects/{pid}/previews/{sha}.jpg"))
        .header("authorization", &reviewer_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "S02：撤销后文件 URL 必须拒绝");

    // Owner 不受影响。
    let resp = client
        .get(format!("{base}/projects/{pid}/previews"))
        .header("authorization", &owner_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // 非 owner 不能授权。
    let resp = client
        .post(format!("{base}/projects/{pid}/grants"))
        .header("authorization", &reviewer_token)
        .json(&json!({"grantee_email": "owner@test.dev", "role": "viewer"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "非 owner 不能授权");

    // 无认证被拒。
    let resp = client
        .get(format!("{base}/projects/{pid}/previews"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401, "无认证被拒");
}

#[tokio::test]
async fn auth_rejects_bad_credentials() {
    let (base, _, _, _guard) = spawn_server().await;
    let client = Client::new();
    let resp = client
        .post(format!("{base}/auth/register"))
        .json(&json!({"email": "a@b.c", "display_name": "A", "password": "123"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "弱密码应被拒");
}
