use between_frames_lib::{db::Store, shared_cloud};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

struct Stub {
    origin: String,
    requests: Arc<Mutex<Vec<String>>>,
}

fn spawn_stub(responses: Vec<(u16, String)>) -> Stub {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    std::thread::spawn(move || {
        for (status, body) in responses {
            let Ok((mut socket, _)) = listener.accept() else {
                break;
            };
            let mut head = Vec::new();
            let mut chunk = [0u8; 512];
            while !head.ends_with(b"\r\n\r\n") {
                match socket.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if head.len() + n > 64 * 1024 {
                            break;
                        }
                        head.extend_from_slice(&chunk[..n]);
                    }
                }
            }
            captured
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&head).to_string());
            let reason = if status == 200 { "OK" } else { "Error" };
            let reply = format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(reply.as_bytes());
        }
    });
    Stub { origin, requests }
}

async fn shared_project(store: &Store, base: &str) -> String {
    let project = store
        .create("作者记忆项目", "云端跨项目只读")
        .await
        .unwrap();
    let binding = json!({"remote": {"binding": {"baseUrl": base, "webUrl": base, "userId": "u1", "workspaceId": "w1"}, "projectId": "11111111-2222-4333-8444-555555555555"}});
    sqlx::query("INSERT INTO v2_promotions(id,project_id,initiated_by,state,from_revision,detail_json,created_at,updated_at) VALUES(?,?,?,'completed',0,?,?,?)")
        .bind("promo-author-memory")
        .bind(&project.id)
        .bind("local")
        .bind(binding.to_string())
        .bind("2026-10-27T00:00:00Z")
        .bind("2026-10-27T00:00:00Z")
        .execute(&store.pool)
        .await
        .unwrap();
    project.id
}

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[tokio::test]
async fn shared_project_fetches_creator_scoped_memories() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(&temp.path().join("db.sqlite")).await.unwrap();
    let body = r#"[{"id":"h1","statement":"偏好低饱和冷色调","status":"confirmed","confidence":"moderate","project_id":"22222222-3333-4444-8555-666666666666","project_title":"另一个项目","created_at":"2026-10-27T00:00:00Z","updated_at":"2026-10-27T00:00:00Z"}]"#;
    let stub = spawn_stub(vec![(200, body.to_string())]);
    let project = shared_project(&store, &stub.origin).await;
    let value = shared_cloud::author_memories(&store, &project, TOKEN)
        .await
        .unwrap();
    assert_eq!(value.as_array().unwrap().len(), 1);
    let requests = stub.requests.lock().unwrap();
    let head = requests[0].to_lowercase();
    assert!(
        head.starts_with("get /api/v2/author-memory?project=11111111-2222-4333-8444-555555555555 "),
        "{head}"
    );
    assert!(
        head.contains(&format!("authorization: bearer {TOKEN}")),
        "{head}"
    );
}

#[tokio::test]
async fn local_project_rejected_with_authority_mismatch() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(&temp.path().join("db.sqlite")).await.unwrap();
    let project = store.create("本地项目", "不应有云端记忆").await.unwrap();
    let error = shared_cloud::author_memories(&store, &project.id, TOKEN)
        .await
        .unwrap_err();
    assert_eq!(error.code, "AUTHORITY_MISMATCH");
}

#[tokio::test]
async fn invalid_token_rejected_before_any_request() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(&temp.path().join("db.sqlite")).await.unwrap();
    let stub = spawn_stub(vec![]);
    let project = shared_project(&store, &stub.origin).await;
    let error = shared_cloud::author_memories(&store, &project, "short")
        .await
        .unwrap_err();
    assert_eq!(error.code, "AUTH_REQUIRED");
    assert!(stub.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn non_array_reply_rejected_as_protocol_error() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(&temp.path().join("db.sqlite")).await.unwrap();
    let stub = spawn_stub(vec![(200, r#"{"unexpected":true}"#.to_string())]);
    let project = shared_project(&store, &stub.origin).await;
    let error = shared_cloud::author_memories(&store, &project, TOKEN)
        .await
        .unwrap_err();
    assert_eq!(error.code, "CLOUD_PROTOCOL");
}

#[tokio::test]
async fn server_error_code_passes_through() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(&temp.path().join("db.sqlite")).await.unwrap();
    let stub = spawn_stub(vec![(
        403,
        r#"{"error":{"code":"PERMISSION_DENIED","message":"只有项目所有者可以访问作者记忆"}}"#
            .to_string(),
    )]);
    let project = shared_project(&store, &stub.origin).await;
    let error = shared_cloud::author_memories(&store, &project, TOKEN)
        .await
        .unwrap_err();
    assert_eq!(error.code, "PERMISSION_DENIED");
}
