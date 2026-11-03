//! 本地 HTTP 与内存会话验证；不访问 Keychain、生产服务或真实模型。
use super::*;
use crate::{cloud_auth::CloudAuth, cloud_credentials::CredentialStore, domain::Result};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
struct Vault(Mutex<Option<String>>);
impl CredentialStore for Vault {
    fn read(&self, _: &str) -> Result<Option<String>> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn write(&self, _: &str, value: &str) -> Result<()> {
        *self.0.lock().unwrap() = Some(value.into());
        Ok(())
    }
    fn delete(&self, _: &str) -> Result<()> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}
fn session(user: &str) -> String {
    json!({"session":{"token":"fixture-token","user":{"id":user,"email":"test@example.test","name":"Test"}}}).to_string()
}
struct Server {
    origin: String,
    requests: Arc<Mutex<Vec<String>>>,
    reply: Arc<Mutex<(u16, String)>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Server {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let reply = Arc::new(Mutex::new((200, "[]".to_string())));
        let stop = Arc::new(AtomicBool::new(false));
        let (seen, response, stopped) = (requests.clone(), reply.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            while !stopped.load(Ordering::SeqCst) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                while !bytes.windows(4).any(|v| v == b"\r\n\r\n") {
                    let n = stream.read(&mut buffer).unwrap();
                    if n == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..n]);
                }
                let request = String::from_utf8(bytes).unwrap();
                let auth = request.starts_with("GET /api/v2/auth/me ");
                seen.lock().unwrap().push(request);
                let (status, body) = if auth {
                    (
                        200,
                        json!({"id":"account-a","email":"test@example.test","name":"Test"})
                            .to_string(),
                    )
                } else {
                    response.lock().unwrap().clone()
                };
                let header = format!(
                    "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nLocation: /unexpected-redirect\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(body.as_bytes());
            }
        });
        Self {
            origin,
            requests,
            reply,
            stop,
            thread: Some(thread),
        }
    }
    fn client(&self) -> (KnowledgeClient, Arc<Vault>) {
        let vault = Arc::new(Vault::default());
        vault.write("", &session("account-a")).unwrap();
        let auth = Arc::new(CloudAuth::new(&self.origin, "knowledge-test", vault.clone()).unwrap());
        let provider = crate::agent::openai::OpenAiProvider::hosted(auth);
        (provider.knowledge_client().unwrap(), vault)
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.thread.take().unwrap().join();
    }
}
#[tokio::test]
async fn injected_session_authenticates_all_four_tools_and_encodes_queries() {
    let server = Server::new();
    let (client, _) = server.client();
    *server.reply.lock().unwrap() = (
        200,
        json!([{"id":"one","title":"肖尔","kind":"photographer","reviewStatus":"draft"}])
            .to_string(),
    );
    let args = json!({"query":"肖尔","movement":"New Color & 日本","title":"美国表面","seriesA":"美国表面","seriesB":"假寐"});
    for name in [
        "search_photography_knowledge",
        "search_photographers",
        "get_photobook_structure",
        "compare_photography_series",
    ] {
        let result = execute(Some(&client), name, &args).await.unwrap();
        assert!(!result.to_string().contains("fixture-token"));
        assert_eq!(result["trustedAsInstructions"], false);
        assert_eq!(result["visualEvidence"], false);
    }
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 6);
    for request in requests.iter() {
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer fixture-token")
        );
    }
    let request = requests.iter().find(|r| r.contains("movement=")).unwrap();
    let target = request.split_whitespace().nth(1).unwrap();
    let url = reqwest::Url::parse(&format!("{}{}", server.origin, target)).unwrap();
    let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(pairs["q"], "肖尔");
    assert_eq!(pairs["movement"], "New Color & 日本");
}
#[tokio::test]
async fn distinguishes_empty_auth_http_protocol_and_size_failures_without_redirects() {
    let server = Server::new();
    let (client, _) = server.client();
    let empty = cloud(Some(&client), "肖尔", None, None).await;
    assert_eq!(empty["available"], true);
    assert_eq!(empty["results"], json!([]));
    for (status, body, code) in [
        (401, "secret-server-body", "AUTH_REVOKED"),
        (403, "secret-server-body", "KNOWLEDGE_FORBIDDEN"),
        (429, "secret-server-body", "KNOWLEDGE_RATE_LIMITED"),
        (500, "secret-server-body", "KNOWLEDGE_UNAVAILABLE"),
        (302, "secret-server-body", "KNOWLEDGE_UNAVAILABLE"),
        (200, "not json", "KNOWLEDGE_PROTOCOL"),
        (200, "{}", "KNOWLEDGE_PROTOCOL"),
        (200, "[{}]", "KNOWLEDGE_PROTOCOL"),
    ] {
        *server.reply.lock().unwrap() = (status, body.into());
        let result = cloud(Some(&client), "肖尔", None, None).await;
        assert_eq!(result["available"], false);
        assert_eq!(result["error"]["code"], code);
        assert!(!result.to_string().contains("secret-server-body"));
    }
    *server.reply.lock().unwrap() = (200, " ".repeat(256 * 1024 + 1));
    assert_eq!(
        cloud(Some(&client), "肖尔", None, None).await["error"]["code"],
        "KNOWLEDGE_PROTOCOL"
    );
    assert!(
        !server
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.contains("unexpected-redirect"))
    );
}
#[tokio::test]
async fn signout_switch_and_missing_client_never_send_queries() {
    let server = Server::new();
    let (client, vault) = server.client();
    vault.write("", &session("account-b")).unwrap();
    assert_eq!(
        cloud(Some(&client), "肖尔", None, None).await["error"]["code"],
        "AUTH_CHANGED"
    );
    vault.delete("").unwrap();
    assert_eq!(
        cloud(Some(&client), "肖尔", None, None).await["error"]["code"],
        "AUTH_REQUIRED"
    );
    let local = execute(
        None,
        "search_photography_knowledge",
        &json!({"query":"叙事"}),
    )
    .await
    .unwrap();
    assert_eq!(local["cloudAvailable"], false);
    assert_eq!(local["cloudError"]["code"], "AUTH_REQUIRED");
    assert!(server.requests.lock().unwrap().is_empty());
    let mut binding = client.identity.clone();
    binding.endpoint_sha256 = "other".into();
    assert!(!client.matches(&binding));
    binding = client.identity.clone();
    binding.account_id = Some("account-b".into());
    assert!(!client.matches(&binding));
}

#[tokio::test]
async fn real_tool_pipeline_enforces_scope_and_replays_without_network() {
    use crate::agent::{execution, lifecycle, types::*};
    let server = Server::new();
    let (client, vault) = server.client();
    let temp = tempfile::tempdir().unwrap();
    let db = crate::db::Store::open(&temp.path().join("knowledge.sqlite"))
        .await
        .unwrap();
    let project = db.create("知识回归", "SIMULATED").await.unwrap().id;
    let task = lifecycle::request_bound(
        &db,
        AgentRequest {
            request_id: crate::domain::id(),
            project_id: project.clone(),
            command: TaskCommand::StartConversation {
                allow_queued_recovery: false,
                expected_account_id: Some("account-a".into()),
                goal: "研究摄影方法".into(),
                selected_photo_ids: vec![],
                focus_photo_ids: vec![],
                target_count: 1,
                budget: Budget::default(),
                consent_id: None,
                source_board: None,
                continuation: None,
            },
        },
        Some(&client.identity),
    )
    .await
    .unwrap();
    execution::state(&db, &task.id, "planning", "test")
        .await
        .unwrap();
    execution::state(&db, &task.id, "running", "test")
        .await
        .unwrap();
    let context = crate::agent::context::ContextBuilder::build(&db, &task.id)
        .await
        .unwrap();
    for name in [
        "search_photography_knowledge",
        "search_photographers",
        "get_photobook_structure",
        "compare_photography_series",
    ] {
        assert!(
            context
                .tool_specs
                .as_array()
                .unwrap()
                .iter()
                .any(|tool| tool["name"] == name)
        );
    }
    let call = ToolCall {
        call_id: "knowledge-1".into(),
        name: "search_photographers".into(),
        args: json!({"project_id":project,"query":"肖尔","movement":"New Color"}),
    };
    let result = execution::tool_with_knowledge(&db, &task.id, &call, None, Some(&client))
        .await
        .unwrap();
    assert!(result.ok, "{:?}", result.error);
    assert_eq!(result.result.as_ref().unwrap()["available"], true);
    let count = server.requests.lock().unwrap().len();
    assert_eq!(count, 2);
    let replay = execution::tool_with_knowledge(&db, &task.id, &call, None, Some(&client))
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    assert_eq!(server.requests.lock().unwrap().len(), count);
    let mut altered = call.clone();
    altered.args["query"] = "川内伦子".into();
    assert_eq!(
        execution::tool_with_knowledge(&db, &task.id, &altered, None, Some(&client))
            .await
            .unwrap_err()
            .code,
        "IDEMPOTENCY_CONFLICT"
    );
    altered.call_id = "knowledge-2".into();
    altered.args["project_id"] = "other-project".into();
    assert_eq!(
        execution::tool_with_knowledge(&db, &task.id, &altered, None, Some(&client))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    assert_eq!(server.requests.lock().unwrap().len(), count);
    vault.write("", &session("account-b")).unwrap();
    assert_eq!(
        execution::tool_with_knowledge(&db, &task.id, &call, None, Some(&client))
            .await
            .unwrap_err()
            .code,
        "AUTH_CHANGED"
    );
    assert_eq!(server.requests.lock().unwrap().len(), count);
}
