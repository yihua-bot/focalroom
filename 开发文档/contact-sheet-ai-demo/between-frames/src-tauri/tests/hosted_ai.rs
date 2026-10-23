#![cfg(feature = "integration-harness")]
use between_frames_lib::{
    agent::{context::ModelTurnRequest, openai::OpenAiProvider, provider::ModelProvider, types::*},
    cloud_auth::CloudAuth,
    cloud_credentials::CredentialStore,
    domain::Result,
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
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
    json!({"pending":null,"session":{"token":"f".repeat(64),"user":{"id":user,"email":"fixture@example.test","name":"Fixture"}}}).to_string()
}
struct Server {
    origin: String,
    calls: Arc<AtomicU32>,
    auth_calls: Arc<AtomicU32>,
    mode: Arc<AtomicU32>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Server {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(AtomicU32::new(0));
        let auth_calls = Arc::new(AtomicU32::new(0));
        let auth_count = auth_calls.clone();
        let mode = Arc::new(AtomicU32::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (count, status, stopping) = (calls.clone(), mode.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(s) => s,
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        continue;
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                let boundary;
                loop {
                    let n = stream.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                    if let Some(i) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                        boundary = i + 4;
                        break;
                    }
                }
                let headers = String::from_utf8_lossy(&bytes[..boundary]).to_lowercase();
                assert!(headers.contains(&format!("authorization: bearer {}", "f".repeat(64))));
                let len = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length: ")
                            .map(|v| v.parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                while bytes.len() < boundary + len {
                    let n = stream.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                }
                let ai = headers.starts_with("post /api/v2/ai/deepseek-flash-v1/responses ");
                let (code, response) = if ai {
                    count.fetch_add(1, Ordering::SeqCst);
                    assert!(headers.contains("x-focalroom-ai-policy: deepseek-flash-v1"));
                    assert!(headers.contains("x-focalroom-image-consent: granted"));
                    assert!(headers.contains("x-focalroom-request-id: "));
                    let body: Value =
                        serde_json::from_slice(&bytes[boundary..boundary + len]).unwrap();
                    assert_eq!(body["model"], "deepseek-flash");
                    assert!(!body.to_string().contains("hostedAccountId"));
                    match status.load(Ordering::SeqCst) {
                        1 => (402, json!({"error":{"code":"AI_QUOTA_EXHAUSTED"}})),
                        2 => {
                            continue;
                        }
                        _ => (
                            200,
                            json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"fixture"}]}],"usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}),
                        ),
                    }
                } else if headers.starts_with("get /api/v2/ai/account ") {
                    (
                        200,
                        json!({"policy":"deepseek-flash-v1","enabled":true,"verified":true,"available":250000}),
                    )
                } else {
                    assert!(headers.starts_with("get /api/v2/auth/me "));
                    auth_count.fetch_add(1, Ordering::SeqCst);
                    match status.load(Ordering::SeqCst) {
                        3 => {
                            status.store(0, Ordering::SeqCst);
                            continue;
                        }
                        4 => continue,
                        5 => {
                            write!(stream,"HTTP/1.1 401 Unauthorized\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}").unwrap();
                            continue;
                        }
                        _ => {}
                    }
                    (
                        200,
                        json!({"id":"user-a","email":"fixture@example.test","name":"Fixture"}),
                    )
                };
                let body = response.to_string();
                write!(stream,"HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        Self {
            origin,
            calls,
            auth_calls,
            mode,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let result = self.thread.take().unwrap().join();
        if !std::thread::panicking() {
            result.unwrap();
        }
    }
}
fn input() -> ModelTurnRequest {
    ModelTurnRequest {
        task: Task {
            id: "t".into(),
            project_id: "p".into(),
            goal: "fixture".into(),
            base_revision: 1,
            memory_revision: 0,
            allowed_photo_ids: vec![],
            selected_photo_ids: vec![],
            target_count: 2,
            state: "running".into(),
            reason: String::new(),
            budget: Budget::default(),
            usage: Usage::default(),
            elapsed_ms: 0,
            question: None,
            answer: None,
            final_draft_ids: vec![],
            pending: None,
            provider_id: "openai".into(),
            consent_id: None,
            created_at: String::new(),
            updated_at: String::new(),
        },
        layers: json!({"system":"fixture","execution":{"hostedAccountId":"user-a"}}),
        protocol: vec![],
        tool_specs: json!([]),
        prompt_version: "fixture".into(),
    }
}
#[tokio::test]
async fn only_readonly_authentication_connection_failure_is_retried_once() {
    for mode in [3, 4, 5] {
        let server = Server::new();
        let vault = Arc::new(Vault::default());
        vault.write("", &session("user-a")).unwrap();
        let provider = OpenAiProvider::hosted(Arc::new(
            CloudAuth::new(&server.origin, "fixture", vault).unwrap(),
        ))
        .bind_account()
        .unwrap();
        server.mode.store(mode, Ordering::SeqCst);
        let result = provider.generate_metered(&input()).await;
        if mode == 3 {
            assert!(result.is_ok());
            assert_eq!(server.calls.load(Ordering::SeqCst), 1);
        } else {
            assert_eq!(
                result.err().unwrap().code,
                if mode == 4 {
                    "CLOUD_UNAVAILABLE"
                } else {
                    "AUTH_REVOKED"
                }
            );
            assert_eq!(server.calls.load(Ordering::SeqCst), 0);
        }
        assert_eq!(
            server.auth_calls.load(Ordering::SeqCst),
            if mode == 5 { 1 } else { 2 }
        );
    }
}
#[tokio::test]
async fn hosted_calls_use_account_bearer_and_fixed_policy_without_a_provider_key() {
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    vault.write("", &session("user-a")).unwrap();
    let auth = Arc::new(CloudAuth::new(&server.origin, "fixture", vault).unwrap());
    let provider = OpenAiProvider::hosted(auth).bind_account().unwrap();
    assert!(provider.api_key_tail().is_empty());
    assert_eq!(
        provider.rate_card(),
        between_frames_lib::agent::pricing::RateCard::DeepseekFlashPeak20261016
    );
    assert_eq!(provider.identity().account_id.as_deref(), Some("user-a"));
    let result = provider.generate_metered(&input()).await.unwrap();
    assert_eq!(result.usage.unwrap().total_tokens, 15);
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn logout_account_switch_and_task_binding_mismatch_prevent_transmission() {
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    let auth = Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap());
    let provider = OpenAiProvider::hosted(auth);
    assert!(provider.bind_account().is_err());
    vault.write("", &session("user-a")).unwrap();
    let bound = provider.bind_account().unwrap();
    let identity = bound.identity();
    vault.write("", &session("user-b")).unwrap();
    assert_ne!(
        identity.consent_provider(),
        provider.identity().consent_provider()
    );
    let error = bound.generate_metered(&input()).await.err().unwrap();
    assert_eq!(error.code, "AUTH_REVOKED");
    let rebound = provider.bind_account().unwrap();
    assert_eq!(
        rebound.generate_metered(&input()).await.err().unwrap().code,
        "AUTH_CHANGED"
    );
    vault.delete("").unwrap();
    assert!(bound.generate_metered(&input()).await.is_err());
    assert_eq!(server.calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn quota_failure_and_unknown_network_results_are_never_automatically_retriable() {
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    vault.write("", &session("user-a")).unwrap();
    let provider = OpenAiProvider::hosted(Arc::new(
        CloudAuth::new(&server.origin, "fixture", vault).unwrap(),
    ))
    .bind_account()
    .unwrap();
    for (mode, code) in [(1, "AI_QUOTA_EXHAUSTED"), (2, "AI_RESULT_UNKNOWN")] {
        server.mode.store(mode, Ordering::SeqCst);
        let error = provider.generate_metered(&input()).await.err().unwrap();
        assert_eq!(error.code, code);
        assert!(!error.retriable);
    }
    assert_eq!(server.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn native_task_creation_freezes_hosted_provider_and_account_instead_of_running_mock() {
    use between_frames_lib::{
        agent::{context::ContextBuilder, runtime::AgentEngine},
        db::Store,
        image_pipeline,
    };
    let directory = tempfile::tempdir().unwrap();
    let db = Store::open(&directory.path().join("fixture.sqlite"))
        .await
        .unwrap();
    let project = db.create("fixture", "protocol only").await.unwrap();
    let source = directory.path().join("fixture.jpg");
    image::RgbImage::from_pixel(32, 24, image::Rgb([1, 2, 3]))
        .save(&source)
        .unwrap();
    db.insert_photo(
        &project.id,
        &image_pipeline::prepare(&source, &directory.path().join("cache")).unwrap(),
    )
    .await
    .unwrap();
    let before = db.get(&project.id).await.unwrap();
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    vault.write("", &session("user-a")).unwrap();
    let auth = Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap());
    let engine = AgentEngine::hosted(db.clone(), auth);
    let request = AgentRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        project_id: project.id.clone(),
        command: TaskCommand::StartTask {
            goal: "protocol only".into(),
            selected_photo_ids: vec![],
            target_count: 1,
            budget: Budget::default(),
            consent_id: None,
        },
    };
    let task = engine.request(request.clone()).await.unwrap();
    assert_eq!(task.provider_id, "openai");
    assert_eq!(engine.request(request.clone()).await.unwrap().id, task.id);
    let binding: String = sqlx::query_scalar(
        "SELECT body_json FROM agent_steps WHERE task_id=? AND kind='model_binding'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let binding: Value = serde_json::from_str(&binding).unwrap();
    assert_eq!(binding["accountId"], "user-a");
    assert_eq!(binding["modelId"], "deepseek-flash");
    let _result = engine.run(&task.id).await;
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    let loaded = ContextBuilder::build(&db, &task.id).await.unwrap();
    assert_eq!(
        loaded.task.usage.input_estimate + loaded.task.usage.output_estimate,
        15
    );
    assert!(
        between_frames_lib::agent::projection(&db, &project.id)
            .await
            .unwrap()
            .drafts
            .is_empty()
    );
    assert_eq!(db.get(&project.id).await.unwrap().revision, before.revision);
    vault.write("", &session("user-b")).unwrap();
    assert!(engine.request(request).await.is_err());
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
}
