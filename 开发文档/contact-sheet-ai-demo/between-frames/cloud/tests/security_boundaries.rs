//! 本地夹具直连回环服务，不经过系统代理转发测试凭据。
//! 真实 HTTP、独立临时库和生成 JPEG；不使用用户库、原片或生产凭证。
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

struct Server {
    base: String,
    client: Client,
    pool: sqlx::SqlitePool,
    temp: tempfile::TempDir,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join("previews")).unwrap();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let app = bf_cloud::build_router(pool.clone(), temp.path().join("previews")).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            base,
            client: Client::builder().no_proxy().build().unwrap(),
            pool,
            temp,
            task,
        }
    }
    async fn register(&self, email: &str) -> Value {
        let response = self
            .client
            .post(format!("{}/auth/register", self.base))
            .json(&json!({"email":email,"display_name":"测试用户","password":"test-password-123"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        response.json().await.unwrap()
    }
    async fn project(&self, token: &str) -> String {
        let response = self
            .client
            .post(format!("{}/projects", self.base))
            .bearer_auth(token)
            .json(&json!({"title":"私密项目"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        response.json::<Value>().await.unwrap()["id"]
            .as_str()
            .unwrap()
            .into()
    }
    async fn grant(&self, token: &str, project: &str, email: &str, role: &str) -> String {
        let response = self
            .client
            .post(format!("{}/projects/{project}/grants", self.base))
            .bearer_auth(token)
            .json(&json!({"grantee_email":email,"role":role}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        response.json::<Value>().await.unwrap()["grant_id"]
            .as_str()
            .unwrap()
            .into()
    }
    async fn list_status(&self, token: &str, project: &str) -> StatusCode {
        let response = self
            .client
            .get(format!("{}/projects/{project}/previews", self.base))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        response.status()
    }
    async fn revoke(&self, token: &str, project: &str, grant: &str) -> Value {
        let response = self
            .client
            .delete(format!("{}/projects/{project}/grants/{grant}", self.base))
            .bearer_auth(token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        response.json().await.unwrap()
    }
    async fn preview(&self, project: &str, sha: &str, bytes: &[u8], width: i64, height: i64) {
        sqlx::query("INSERT INTO preview_files(id,project_id,sha256,width,height,file_size) VALUES(?,?,?,?,?,?)")
            .bind(uuid::Uuid::new_v4().to_string()).bind(project).bind(sha).bind(width).bind(height).bind(bytes.len() as i64)
            .execute(&self.pool).await.unwrap();
        if sha.len() == 64 {
            std::fs::write(
                self.temp
                    .path()
                    .join("previews")
                    .join(format!("{sha}-1400.jpg")),
                bytes,
            )
            .unwrap();
        }
    }
    async fn read(&self, token: &str, project: &str, file: &str) -> reqwest::Response {
        self.client
            .get(format!("{}/projects/{project}/previews/{file}", self.base))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
    }
}
fn token(value: &Value) -> &str {
    value["token"].as_str().unwrap()
}
fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbImage::new(width, height)
        .write_to(&mut bytes, image::ImageFormat::Jpeg)
        .unwrap();
    bytes.into_inner()
}

#[tokio::test]
async fn invalid_and_oversized_registration_is_rejected_before_persistence() {
    let server = Server::new().await;
    for value in [
        json!({"email":"bad","display_name":"Test","password":"test-password-123"}),
        json!({"email":"a@b.c","display_name":"  ","password":"test-password-123"}),
        json!({"email":"a@b.c","display_name":"Test","password":"short"}),
        json!({"email":"a@b.c","display_name":"Test","password":"x".repeat(1025)}),
    ] {
        assert_eq!(
            server
                .client
                .post(format!("{}/auth/register", server.base))
                .json(&value)
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
    }
    assert_eq!(
        server
            .client
            .post(format!("{}/auth/register", server.base))
            .header("content-type", "application/json")
            .body("x".repeat(17 * 1024))
            .send()
            .await
            .unwrap()
            .status(),
        413
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

async fn start_binary(temp: &tempfile::TempDir) -> (tokio::process::Child, String) {
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_bf-cloud"))
        .env("BF_CLOUD_DB", temp.path().join("new.sqlite"))
        .env("BF_PREVIEW_DIR", temp.path().join("new-previews"))
        .env("BF_CLOUD_PORT", "0")
        .env("BF_CLOUD_BIND", "127.0.0.1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let line = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
        .await
        .unwrap()
        .unwrap()
        .expect("二进制必须成功启动");
    let addr: std::net::SocketAddr = line.split_whitespace().last().unwrap().parse().unwrap();
    assert!(addr.ip().is_loopback());
    (child, format!("http://{addr}"))
}

#[tokio::test]
async fn first_start_creates_database_and_process_restart_preserves_login_and_project() {
    let temp = tempfile::tempdir().unwrap();
    let client = Client::builder().no_proxy().build().unwrap();
    let (mut child, base) = start_binary(&temp).await;
    assert!(temp.path().join("new.sqlite").is_file());
    assert!(temp.path().join("new-previews").is_dir());
    let response = client.post(format!("{base}/auth/register")).json(&json!({"email":"restart@test.dev","display_name":"Restart","password":"test-password-123"})).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let auth: Value = response.json().await.unwrap();
    let response = client
        .post(format!("{base}/projects"))
        .bearer_auth(token(&auth))
        .json(&json!({"title":"重启项目"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let project: Value = response.json().await.unwrap();
    child.kill().await.unwrap();
    child.wait().await.unwrap();
    let (mut restarted, base) = start_binary(&temp).await;
    let response = client
        .get(format!(
            "{base}/projects/{}/previews",
            project["id"].as_str().unwrap()
        ))
        .bearer_auth(token(&auth))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    restarted.kill().await.unwrap();
    restarted.wait().await.unwrap();
}

#[tokio::test]
async fn revoke_is_scoped_to_grant_and_project_and_regrant_is_stable() {
    let server = Server::new().await;
    let owner = server.register("owner@test.dev").await;
    let a = server.register("a@test.dev").await;
    let b = server.register("b@test.dev").await;
    let project = server.project(token(&owner)).await;
    let other = server.project(token(&owner)).await;
    let ga = server
        .grant(token(&owner), &project, " A@TEST.DEV ", "reviewer")
        .await;
    let gb = server
        .grant(token(&owner), &project, "b@test.dev", "viewer")
        .await;
    let foreign = server
        .grant(token(&owner), &other, "a@test.dev", "viewer")
        .await;
    assert_eq!(
        server.revoke(token(&owner), &project, &foreign).await["revoked"],
        0
    );
    assert_eq!(
        server.revoke(token(&owner), &project, "missing").await["revoked"],
        0
    );
    assert_eq!(server.list_status(token(&a), &project).await, 200);
    assert_eq!(
        server.revoke(token(&owner), &project, &ga).await["revoked"],
        1
    );
    assert_eq!(server.list_status(token(&a), &project).await, 403);
    assert_eq!(server.list_status(token(&b), &project).await, 200);
    assert_eq!(server.list_status(token(&a), &other).await, 200);
    assert_eq!(server.list_status(token(&owner), &project).await, 200);
    assert_eq!(
        server.revoke(token(&owner), &project, &ga).await["revoked"],
        0
    );
    assert_eq!(
        server
            .grant(token(&owner), &project, "a@test.dev", "viewer")
            .await,
        ga
    );
    assert_eq!(server.list_status(token(&a), &project).await, 200);
    let states: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT state,revoked_at FROM project_grants WHERE id=?")
            .bind(&ga)
            .fetch_all(&server.pool)
            .await
            .unwrap();
    assert_eq!(states, vec![("active".into(), None)]);
    let response = server
        .client
        .delete(format!("{}/projects/{project}/grants/{gb}", server.base))
        .bearer_auth(token(&a))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 403);
    assert_eq!(server.list_status(token(&b), &project).await, 200);
}

#[tokio::test]
async fn auth_hashes_credentials_expires_sessions_and_logout_is_specific() {
    let server = Server::new().await;
    let user = server.register(" User@TEST.DEV ").await;
    let other = server.register("other@test.dev").await;
    let (email, hash): (String, String) =
        sqlx::query_as("SELECT email,password_hash FROM users WHERE id=?")
            .bind(user["user_id"].as_str().unwrap())
            .fetch_one(&server.pool)
            .await
            .unwrap();
    assert_eq!(email, "user@test.dev");
    assert!(hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
    let other_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id=?")
        .bind(other["user_id"].as_str().unwrap())
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_ne!(hash, other_hash, "相同密码必须使用不同随机盐");
    let stored: String = sqlx::query_scalar("SELECT token FROM sessions WHERE user_id=?")
        .bind(user["user_id"].as_str().unwrap())
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(stored, hex::encode(Sha256::digest(token(&user).as_bytes())));
    assert_ne!(stored, token(&user));
    for email in ["user@test.dev", "missing@test.dev"] {
        let response = server
            .client
            .post(format!("{}/auth/login", server.base))
            .json(&json!({"email":email,"password":"wrong-password"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 401);
        assert_eq!(
            response.json::<Value>().await.unwrap()["error"]["message"],
            "邮箱或密码错误"
        );
    }
    let response = server
        .client
        .post(format!("{}/auth/login", server.base))
        .json(&json!({"email":" USER@TEST.DEV ","password":"test-password-123"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let login: Value = response.json().await.unwrap();
    assert_ne!(token(&login), token(&user));
    let project = server.project(token(&user)).await;
    let response = server
        .client
        .post(format!("{}/auth/logout", server.base))
        .bearer_auth(token(&user))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(server.list_status(token(&user), &project).await, 401);
    assert_eq!(server.list_status(token(&login), &project).await, 200);
    let digest = hex::encode(Sha256::digest(token(&login).as_bytes()));
    for timestamp in ["2000-01-01 00:00:00", "2999-01-01 00:00:00", "invalid"] {
        sqlx::query("UPDATE sessions SET created_at=? WHERE token=?")
            .bind(timestamp)
            .bind(&digest)
            .execute(&server.pool)
            .await
            .unwrap();
        assert_eq!(server.list_status(token(&login), &project).await, 401);
    }
}

#[tokio::test]
async fn duplicate_registration_and_database_failure_do_not_leak_or_leave_orphans() {
    let server = Server::new().await;
    server.register("user@test.dev").await;
    let response = server.client.post(format!("{}/auth/register", server.base)).json(&json!({"email":" USER@TEST.DEV ","display_name":"duplicate","password":"test-password-123"})).send().await.unwrap();
    assert_eq!(response.status(), 409);
    let body = response.text().await.unwrap();
    assert!(!body.contains("UNIQUE") && !body.contains("password") && !body.contains("INSERT"));
    // 仅临时库注入第二次 INSERT 失败，验证用户和会话必须一起提交。
    sqlx::query("CREATE TRIGGER fail_session BEFORE INSERT ON sessions BEGIN SELECT RAISE(ABORT,'private-db-diagnostic'); END").execute(&server.pool).await.unwrap();
    let response = server.client.post(format!("{}/auth/register", server.base)).json(&json!({"email":"orphan@test.dev","display_name":"orphan","password":"test-password-123"})).send().await.unwrap();
    assert_eq!(response.status(), 500);
    assert!(!response
        .text()
        .await
        .unwrap()
        .contains("private-db-diagnostic"));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email='orphan@test.dev'")
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn legacy_fast_hash_and_raw_sessions_are_not_accepted() {
    let server = Server::new().await;
    let old_hash = hex::encode(Sha256::digest(b"salt:test-password-123"));
    sqlx::query("INSERT INTO users(id,email,salt,password_hash,display_name) VALUES('legacy','legacy@test.dev','salt',?,'Legacy')")
        .bind(old_hash).execute(&server.pool).await.unwrap();
    let old_token = "a".repeat(64);
    sqlx::query("INSERT INTO sessions(token,user_id) VALUES(?,'legacy')")
        .bind(&old_token)
        .execute(&server.pool)
        .await
        .unwrap();
    let response = server
        .client
        .post(format!("{}/auth/login", server.base))
        .json(&json!({"email":"legacy@test.dev","password":"test-password-123"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    assert_eq!(server.list_status(&old_token, "any").await, 401);
}

#[tokio::test]
async fn previews_require_exact_names_real_jpeg_dimensions_and_project_membership() {
    let server = Server::new().await;
    let owner = server.register("owner@test.dev").await;
    let project = server.project(token(&owner)).await;
    let other = server.project(token(&owner)).await;
    let sha = "a".repeat(64);
    let bytes = jpeg(14, 9);
    server.preview(&project, &sha, &bytes, 14, 9).await;
    let response = server
        .read(token(&owner), &project, &format!("{sha}.jpg"))
        .await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(response.bytes().await.unwrap().as_ref(), bytes);
    assert_eq!(
        server
            .read(token(&owner), &other, &format!("{sha}.jpg"))
            .await
            .status(),
        404
    );
    for file in [
        sha.clone(),
        format!("{sha}.jpg.jpg"),
        format!("{sha}.png"),
        format!("{}.jpg", sha.to_uppercase()),
        "short.jpg".into(),
        "%2e%2e%2fprivate.jpg".into(),
    ] {
        assert_eq!(
            server.read(token(&owner), &project, &file).await.status(),
            404,
            "不合法文件名 {file}"
        );
    }
    // 同样长度的伪 JPEG 不能借 MIME 声明通过。
    let path = server
        .temp
        .path()
        .join("previews")
        .join(format!("{sha}-1400.jpg"));
    std::fs::write(&path, vec![0; bytes.len()]).unwrap();
    assert_eq!(
        server
            .read(token(&owner), &project, &format!("{sha}.jpg"))
            .await
            .status(),
        404
    );
    std::fs::write(&path, jpeg(1401, 2)).unwrap();
    let size = std::fs::metadata(&path).unwrap().len() as i64;
    sqlx::query("UPDATE preview_files SET file_size=?")
        .bind(size)
        .execute(&server.pool)
        .await
        .unwrap();
    assert_eq!(
        server
            .read(token(&owner), &project, &format!("{sha}.jpg"))
            .await
            .status(),
        404
    );
    sqlx::query("UPDATE preview_files SET width=1401")
        .execute(&server.pool)
        .await
        .unwrap();
    assert_eq!(server.list_status(token(&owner), &project).await, 500);
    assert_eq!(
        server
            .read(token(&owner), &project, &format!("{sha}.jpg"))
            .await
            .status(),
        500
    );
}

#[tokio::test]
async fn malformed_database_digest_returns_error_instead_of_panicking() {
    let server = Server::new().await;
    let owner = server.register("owner@test.dev").await;
    let project = server.project(token(&owner)).await;
    server.preview(&project, "短", &[1, 2, 3], 14, 9).await;
    assert_eq!(server.list_status(token(&owner), &project).await, 500);
    assert_eq!(
        server
            .client
            .get(format!("{}/health", server.base))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_preview_cannot_read_a_file_outside_the_preview_directory() {
    let server = Server::new().await;
    let owner = server.register("owner@test.dev").await;
    let project = server.project(token(&owner)).await;
    let sha = "b".repeat(64);
    let bytes = jpeg(14, 9);
    server.preview(&project, &sha, &bytes, 14, 9).await;
    let target = server.temp.path().join("private.jpg");
    std::fs::write(&target, &bytes).unwrap();
    let link_sha = "c".repeat(64);
    sqlx::query("INSERT INTO preview_files(id,project_id,sha256,width,height,file_size) VALUES('link',?,?,14,9,?)")
        .bind(&project).bind(&link_sha).bind(bytes.len() as i64).execute(&server.pool).await.unwrap();
    std::os::unix::fs::symlink(
        &target,
        server
            .temp
            .path()
            .join("previews")
            .join(format!("{link_sha}-1400.jpg")),
    )
    .unwrap();
    assert_eq!(
        server
            .read(token(&owner), &project, &format!("{link_sha}.jpg"))
            .await
            .status(),
        404
    );
}

#[tokio::test]
async fn unreviewed_sqlite_v2_cannot_bypass_password_or_issue_shared_authority() {
    let server = Server::new().await;
    let account = server.register("existing-v2@example.com").await;
    let token = account["token"].as_str().unwrap();
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
        .fetch_one(&server.pool)
        .await
        .unwrap();
    let login = server
        .client
        .post(format!("{}/api/v2/auth/login", server.base))
        .json(&json!({"email":"existing-v2@example.com","password":"wrong-password"}))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::NOT_IMPLEMENTED);
    let value: Value = login.json().await.unwrap();
    assert_eq!(value["error"]["code"], "UNSUPPORTED_AUTHORITY");
    assert!(value.get("token").is_none());
    for (method, path) in [
        ("GET", "/api/v2/workspaces"),
        ("POST", "/api/v2/promotions"),
        ("PUT", "/api/v2/projects/project/previews/photo"),
        ("POST", "/api/v2/projects/project/finalize"),
        ("POST", "/api/v2/projects/project/abort"),
    ] {
        let response = server
            .client
            .request(method.parse().unwrap(), format!("{}{path}", server.base))
            .bearer_auth(token)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED, "{path}");
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions")
            .fetch_one(&server.pool)
            .await
            .unwrap(),
        sessions
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cloud_projects")
            .fetch_one(&server.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        std::fs::read_dir(server.temp.path().join("previews"))
            .unwrap()
            .count(),
        0
    );
}
