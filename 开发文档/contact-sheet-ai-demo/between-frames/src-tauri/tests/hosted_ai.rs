#![cfg(feature = "integration-harness")]
use between_frames_lib::{
    agent::{
        context::ModelTurnRequest,
        openai::{ChatJob, OpenAiProvider},
        provider::ModelProvider,
        types::*,
    },
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
    queue: Arc<AtomicBool>,
    opt_ins: Arc<AtomicU32>,
    cancels: Arc<AtomicU32>,
    cancel_mode: Arc<AtomicU32>,
    main_reply: Arc<Mutex<Option<Value>>>,
    origin: String,
    calls: Arc<AtomicU32>,
    auth_calls: Arc<AtomicU32>,
    account_calls: Arc<AtomicU32>,
    stream_calls: Arc<AtomicU32>,
    /// 网关实际收到的流式请求编号：用来断言客户端带的是发送前留存的那个，而不是临时造的。
    stream_request_id: Arc<Mutex<String>>,
    polls: Arc<AtomicU32>,
    mode: Arc<AtomicU32>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Server {
    fn stream_request_id(&self) -> String {
        self.stream_request_id.lock().unwrap().clone()
    }
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(AtomicU32::new(0));
        let auth_calls = Arc::new(AtomicU32::new(0));
        let auth_count = auth_calls.clone();
        let account_calls = Arc::new(AtomicU32::new(0));
        let account_count = account_calls.clone();
        let stream_calls = Arc::new(AtomicU32::new(0));
        let stream_count = stream_calls.clone();
        let stream_request_id = Arc::new(Mutex::new(String::new()));
        let seen_stream_id = stream_request_id.clone();
        let polls = Arc::new(AtomicU32::new(0));
        let poll_count = polls.clone();
        let mode = Arc::new(AtomicU32::new(0));
        let queue = Arc::new(AtomicBool::new(false));
        let queue_enabled = queue.clone();
        let opt_ins = Arc::new(AtomicU32::new(0));
        let opted = opt_ins.clone();
        let cancels = Arc::new(AtomicU32::new(0));
        let cancelled = cancels.clone();
        let cancel_mode = Arc::new(AtomicU32::new(0));
        let cancel_reply = cancel_mode.clone();
        let main_reply = Arc::new(Mutex::new(None::<Value>));
        let next_reply = main_reply.clone();
        // 异步作业最近一次提交的是聊天还是视觉：决定轮询回哪种回执。
        let is_chat_job = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (count, status, stopping) = (calls.clone(), mode.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            let mut job_id = String::new();
            let mut photo_id = "photo-1".to_owned();
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
                let async_submit =
                    headers.starts_with("post /api/v2/ai/deepseek-flash-v1/responses/async ");
                let ai = async_submit
                    || headers.starts_with("post /api/v2/ai/deepseek-flash-v1/responses ");
                let (code, response) = if ai {
                    count.fetch_add(1, Ordering::SeqCst);
                    assert!(headers.contains("x-focalroom-ai-policy: deepseek-flash-v1"));
                    assert!(headers.contains("x-focalroom-image-consent: granted"));
                    assert!(headers.contains("x-focalroom-request-id: "));
                    let body: Value =
                        serde_json::from_slice(&bytes[boundary..boundary + len]).unwrap();
                    assert_eq!(body["model"], "deepseek-flash");
                    assert!(!body.to_string().contains("hostedAccountId"));
                    if async_submit {
                        if headers.contains("x-focalroom-recovery-consent: encrypted-queue-v1") {
                            opted.fetch_add(1, Ordering::SeqCst);
                        }
                        let parts = body["input"][0]["content"].as_array().unwrap();
                        let with_images = parts.iter().any(|part| part["type"] == "input_image");
                        // 聊天任务不挂图也不带工具，但仍走同一条"只提交一次、凭编号只读核对"的
                        // 异步链路——所以这里按正文分流，而不是假定异步提交一定是视觉任务。
                        is_chat_job.store(!with_images, Ordering::SeqCst);
                        if !with_images {
                            assert!(body["tools"].as_array().unwrap().is_empty());
                        }
                        job_id = headers
                            .lines()
                            .find_map(|line| line.strip_prefix("x-focalroom-request-id: "))
                            .unwrap()
                            .to_owned();
                        photo_id = parts
                            .iter()
                            .filter_map(|v| v["text"].as_str())
                            .find_map(|s| s.strip_prefix("photo_id: "))
                            .unwrap_or("photo-1")
                            .to_owned();
                        if status.load(Ordering::SeqCst) == 7 {
                            continue;
                        }
                        let ack_id = if status.load(Ordering::SeqCst) == 8 {
                            "wrong-job-id"
                        } else {
                            &job_id
                        };
                        let reply = json!({"jobId":ack_id,"state":"PENDING","result":null,"error":null,"pollUrl":"https://untrusted.invalid/not-followed"}).to_string();
                        write!(stream,"HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",reply.len()).unwrap();
                        continue;
                    }
                    if let Some(reply) = next_reply.lock().unwrap().take() {
                        let text = reply.to_string();
                        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",text.len()).unwrap();
                        continue;
                    }
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
                } else if headers.starts_with("post /api/v2/ai/deepseek-flash-v1/responses/stream ")
                {
                    assert!(headers.contains("x-focalroom-ai-policy: deepseek-flash-v1"));
                    assert!(headers.contains("x-focalroom-image-consent: granted"));
                    let request_id = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("x-focalroom-request-id: "))
                        .unwrap()
                        .to_owned();
                    *seen_stream_id.lock().unwrap() = request_id.clone();
                    let body: Value =
                        serde_json::from_slice(&bytes[boundary..boundary + len]).unwrap();
                    assert_eq!(body["model"], "deepseek-flash");
                    assert!(body["tools"].as_array().unwrap().is_empty());
                    assert!(!body.to_string().contains("input_image"));
                    stream_count.fetch_add(1, Ordering::SeqCst);
                    let frame = |payload: Value| format!("data: {}\n\n", payload);
                    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nConnection: close\r\n\r\n";
                    let opening = frame(json!({"type": "start", "request_id": request_id}));
                    match status.load(Ordering::SeqCst) {
                        // 网关没有这条路由：请求在路由匹配阶段就被拒，服务端没留任何回执，
                        // 客户端据此退回一次性请求，且只算一次模型调用。
                        21 => {
                            write!(stream,"HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}").unwrap();
                        }
                        // 半截断流：增量已经到手但没有 done 回执，结果不明，客户端不得重发。
                        // 服务端此刻已经按这个编号预留并记账，所以编号要留下来供核对。
                        22 => {
                            job_id = request_id.clone();
                            is_chat_job.store(true, Ordering::SeqCst);
                            write!(
                                stream,
                                "{head}{opening}{}",
                                frame(json!({"type":"delta","text":"逆光"}))
                            )
                            .unwrap();
                            let _ = stream.flush();
                        }
                        // 正常：两段增量后给出服务端结算的正式回执。
                        _ => {
                            job_id = request_id.clone();
                            is_chat_job.store(true, Ordering::SeqCst);
                            let reply = json!({"id":"stream-1","status":"completed","model":"deepseek-flash","error":null,"output":[{"type":"message","content":[{"type":"output_text","text":"逆光更好看"}]}],"usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}});
                            write!(
                                stream,
                                "{head}{opening}{}{}{}",
                                frame(json!({"type":"delta","text":"逆光"})),
                                frame(json!({"type":"delta","text":"更好看"})),
                                frame(json!({"type":"done","reply":reply}))
                            )
                            .unwrap();
                            let _ = stream.flush();
                        }
                    }
                    continue;
                } else if headers.starts_with("post /api/v2/ai/deepseek-flash-v1/responses/async/")
                    && headers.contains("/cancel ")
                {
                    assert!(headers.starts_with(&format!(
                        "post /api/v2/ai/deepseek-flash-v1/responses/async/{job_id}/cancel "
                    )));
                    cancelled.fetch_add(1, Ordering::SeqCst);
                    if cancel_reply.load(Ordering::SeqCst) == 2 {
                        continue;
                    }
                    (
                        200,
                        json!({"cancelled":cancel_reply.load(Ordering::SeqCst)==0}),
                    )
                } else if headers.starts_with("get /api/v2/ai/deepseek-flash-v1/responses/async/") {
                    assert!(headers.starts_with(&format!(
                        "get /api/v2/ai/deepseek-flash-v1/responses/async/{job_id} "
                    )));
                    let previous = poll_count.fetch_add(1, Ordering::SeqCst);
                    let mode = status.load(Ordering::SeqCst);
                    // 聊天回执：正文是纯文字，且可以按测试要求停在"仍在处理"，
                    // 用来验证客户端核对时拿到的是状态而不是又一趟模型调用。
                    if is_chat_job.load(Ordering::SeqCst) {
                        let reply = if mode == 24 {
                            json!({"jobId":job_id,"state":"PENDING","result":null,"error":null})
                        } else {
                            json!({"jobId":job_id,"state":"COMPLETED","error":null,"result":{
                                "status":"completed","model":"deepseek-flash","error":null,
                                "output":[{"type":"message","content":[{"type":"output_text","text":"逆光更好看"}]}],
                                "usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}})
                        }
                        .to_string();
                        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",reply.len()).unwrap();
                        continue;
                    }
                    if mode == 11 && previous == 0 {
                        continue;
                    }
                    if mode == 10 {
                        write!(stream,"HTTP/1.1 401 Unauthorized\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}").unwrap();
                        continue;
                    }
                    let observation = json!({"observations":[{"photo_id":photo_id,"visible_facts":["fixture only"],"interpretation":"fixture","uncertainty":"not real vision"}]}).to_string();
                    if mode == 12 || mode == 13 || mode == 14 {
                        let reply=match mode {
                            12=>json!({"jobId":job_id,"state":"PENDING","result":null,"error":null}),
                            13=>json!({"jobId":job_id,"state":"COMPLETED","result":null,"error":"AI_RESULT_EXPIRED"}),
                            _=>json!({"jobId":job_id,"state":"COMPLETED","error":null,"result":{"status":"completed","model":"deepseek-flash","output":[],"usage":{"input_tokens":10,"output_tokens":5,"total_tokens":1}}}),
                        }.to_string();
                        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",reply.len()).unwrap();
                        continue;
                    }
                    (
                        200,
                        json!({"jobId":job_id,"state":if mode == 9 {"UNKNOWN"} else {"COMPLETED"},"error":if mode == 9 {json!("AI_RESULT_UNKNOWN")} else {Value::Null},"result":{"status":"completed","model":"deepseek-flash","output":[{"type":"message","content":[{"type":"output_text","text":observation}]}],"usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}}),
                    )
                } else if headers.starts_with("get /api/v2/ai/account ") {
                    account_count.fetch_add(1, Ordering::SeqCst);
                    (
                        200,
                        json!({"policy":"deepseek-flash-v1","enabled":true,"verified":true,"available":250000,"recovery":if queue_enabled.load(Ordering::SeqCst){json!({"enabled":true,"policy":"encrypted-queue-v1","maxQueueSeconds":600})}else{json!({"enabled":false})}}),
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
            queue,
            opt_ins,
            cancels,
            cancel_mode,
            main_reply,
            origin,
            calls,
            auth_calls,
            account_calls,
            stream_calls,
            stream_request_id,
            polls,
            mode,
            stop,
            thread: Some(thread),
        }
    }
}
#[tokio::test]
async fn hosted_images_submit_once_and_poll_known_job_even_when_ack_or_read_is_lost() {
    for mode in [6, 7, 11] {
        let server = Server::new();
        let vault = Arc::new(Vault::default());
        vault.write("", &session("user-a")).unwrap();
        let provider = OpenAiProvider::hosted(Arc::new(
            CloudAuth::new(&server.origin, "fixture", vault).unwrap(),
        ))
        .bind_account()
        .unwrap();
        server.mode.store(mode, Ordering::SeqCst);
        let (result, tokens) = provider
            .observe(
                "fixture",
                "fixture",
                &[("photo-1".into(), "fixture-pixels".into())],
                128,
            )
            .await
            .unwrap();
        assert_eq!(result["observations"][0]["photo_id"], "photo-1");
        assert_eq!(tokens, 15);
        assert_eq!(server.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            server.polls.load(Ordering::SeqCst),
            if mode == 11 { 2 } else { 1 }
        );
    }
}
#[tokio::test]
async fn hosted_async_rejects_wrong_job_unknown_outcome_and_revoked_session_without_reposting() {
    for mode in [8, 9, 10] {
        let server = Server::new();
        let vault = Arc::new(Vault::default());
        vault.write("", &session("user-a")).unwrap();
        let provider = OpenAiProvider::hosted(Arc::new(
            CloudAuth::new(&server.origin, "fixture", vault).unwrap(),
        ))
        .bind_account()
        .unwrap();
        server.mode.store(mode, Ordering::SeqCst);
        let error = provider
            .observe(
                "fixture",
                "fixture",
                &[("photo-1".into(), "fixture-pixels".into())],
                128,
            )
            .await
            .err()
            .unwrap();
        assert_eq!(
            error.code,
            if mode == 10 {
                "AUTH_REQUIRED"
            } else {
                "AI_RESULT_UNKNOWN"
            }
        );
        assert!(!error.retriable);
        assert_eq!(server.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            server.polls.load(Ordering::SeqCst),
            if mode == 8 { 0 } else { 1 }
        );
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
async fn hosted_chat_send_burst_validates_identity_and_account_once_each() {
    // 一次聊天发送原本有三处都要"已校验的当前身份"：前端刷新配置走 ai_account、
    // 引擎账户校验走 ai_account、模型请求走 bearer。过去是 3 趟 /auth/me 加
    // 2 趟 /ai/account 串在模型请求前面，用户看到的就是"文字卡几秒才发出去"。
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    vault.write("", &session("user-a")).unwrap();
    let auth = Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap());
    assert_eq!(auth.ai_account().await.unwrap()["available"], 250000);
    let bound = OpenAiProvider::hosted(auth.clone()).bind_account().unwrap();
    bound.validate_hosted_account().await.unwrap();
    assert_eq!(
        bound.chat_text(&[("user", "你好")], 64).await.unwrap(),
        "fixture"
    );
    // 复用把两次账户读取和第三次身份读取都吸收掉了，只剩模型请求本身。
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    assert_eq!(server.auth_calls.load(Ordering::SeqCst), 1);
    assert_eq!(server.account_calls.load(Ordering::SeqCst), 1);
    // 复用不跨越会话改写：token 与账户 ID 必须逐字对上当前钥匙串会话。
    // 夹具的 /auth/me 恒返回 user-a，因此切到 user-b 后必须重新真实校验并拒绝，
    // 不能让已经通过校验的旧账户 token 顶到新账户上（换号不能续跑）。
    vault.write("", &session("user-b")).unwrap();
    assert_eq!(
        auth.bearer(auth.origin()).await.unwrap_err().code,
        "AUTH_REVOKED"
    );
    assert_eq!(server.auth_calls.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn hosted_chat_stream_forwards_deltas_and_returns_the_settled_reply() {
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    vault.write("", &session("user-a")).unwrap();
    let auth = Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap());
    let bound = OpenAiProvider::hosted(auth.clone()).bind_account().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let mut fragments: Vec<String> = Vec::new();
    let reply = bound
        .chat_text_stream(&[("user", "逆光怎么拍")], 64, &id, |fragment| {
            fragments.push(fragment.to_owned())
        })
        .await
        .unwrap();
    // 到一段交一段，不等整条回复拼完；最终正文以服务端结算过的 done 回执为准。
    assert_eq!(fragments, ["逆光", "更好看"]);
    assert_eq!(reply, "逆光更好看");
    assert_eq!(server.stream_calls.load(Ordering::SeqCst), 1);
    // 流式成功时不该再补一趟一次性请求。
    assert_eq!(server.calls.load(Ordering::SeqCst), 0);
    // 编号是发送前那个：核对用的就是客户端手里留着的这一串，不是服务内部临时造的。
    assert_eq!(
        server.stream_request_id(),
        id,
        "流式请求必须携带调用方留存的编号"
    );
}
#[tokio::test]
async fn hosted_chat_stream_falls_back_when_the_gateway_has_no_route() {
    // 网关未部署 /responses/stream：这种请求在路由匹配阶段就被拒，服务端没有预留额度，
    // 退回提交路径不构成重复计量，而且必须沿用同一个编号——换个编号就等于回执查无此人。
    let server = Server::new();
    server.mode.store(21, Ordering::SeqCst);
    let vault = Arc::new(Vault::default());
    vault.write("", &session("user-a")).unwrap();
    let auth = Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap());
    let bound = OpenAiProvider::hosted(auth.clone()).bind_account().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let mut fragments: Vec<String> = Vec::new();
    let reply = bound
        .chat_text_stream(&[("user", "你好")], 64, &id, |fragment| {
            fragments.push(fragment.to_owned())
        })
        .await
        .unwrap();
    assert_eq!(reply, "逆光更好看");
    assert!(fragments.is_empty());
    assert_eq!(server.stream_calls.load(Ordering::SeqCst), 1);
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    assert_eq!(server.stream_request_id(), id);
    // 退回的路径同样是"提交一次、凭同一编号核对"，所以这条回复断在半路也查得回来。
    assert!(
        matches!(bound.chat_job(&id).await.unwrap(), ChatJob::Completed(text) if text == "逆光更好看")
    );
}
#[tokio::test]
async fn hosted_chat_stream_broken_midway_is_unknown_and_never_resent() {
    // 已经收到 200 与增量后断流：服务端可能已把这趟记成计量结果，
    // 客户端只能报"结果不明"，不能改编号重发，也不能退回一次性路径。
    let server = Server::new();
    server.mode.store(22, Ordering::SeqCst);
    let vault = Arc::new(Vault::default());
    vault.write("", &session("user-a")).unwrap();
    let auth = Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap());
    let bound = OpenAiProvider::hosted(auth.clone()).bind_account().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let mut fragments: Vec<String> = Vec::new();
    let error = bound
        .chat_text_stream(&[("user", "你好")], 64, &id, |fragment| {
            fragments.push(fragment.to_owned())
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, "AI_RESULT_UNKNOWN");
    assert!(!error.retriable);
    assert_eq!(fragments, ["逆光"]);
    assert_eq!(server.stream_calls.load(Ordering::SeqCst), 1);
    assert_eq!(server.calls.load(Ordering::SeqCst), 0);
    // 断流后不重发，只凭手里那个编号核对一次：拿到服务端结算过的正文才正式上屏。
    server.mode.store(23, Ordering::SeqCst);
    let polls = server.polls.load(Ordering::SeqCst);
    match bound.chat_job(&id).await.unwrap() {
        ChatJob::Completed(text) => assert_eq!(text, "逆光更好看"),
        other => panic!("断流后应凭原编号核到已完成回执，实际 {other:?}"),
    }
    assert_eq!(server.polls.load(Ordering::SeqCst), polls + 1);
    // 核对是只读的：既没有再提交一趟模型，也没有再开一条流。
    assert_eq!(server.calls.load(Ordering::SeqCst), 0);
    assert_eq!(server.stream_calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn hosted_chat_reconcile_reports_pending_without_resenting() {
    // 服务端还在处理：核对只能回"仍在处理"，界面据此继续等待，绝不能顺手再发一条。
    let server = Server::new();
    server.mode.store(22, Ordering::SeqCst);
    let vault = Arc::new(Vault::default());
    vault.write("", &session("user-a")).unwrap();
    let auth = Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap());
    let bound = OpenAiProvider::hosted(auth.clone()).bind_account().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let error = bound
        .chat_text_stream(&[("user", "你好")], 64, &id, |_| {})
        .await
        .unwrap_err();
    assert_eq!(error.code, "AI_RESULT_UNKNOWN");
    server.mode.store(24, Ordering::SeqCst);
    assert!(matches!(
        bound.chat_job(&id).await.unwrap(),
        ChatJob::Pending
    ));
    assert_eq!(server.calls.load(Ordering::SeqCst), 0);
    assert_eq!(server.stream_calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn hosted_chat_rejects_an_id_it_could_never_reconcile() {
    // 编号不合法必须在联网之前拒绝：发出去的那条即使成功，客户端手里的编号也永远查不到回执。
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    vault.write("", &session("user-a")).unwrap();
    let auth = Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap());
    let bound = OpenAiProvider::hosted(auth.clone()).bind_account().unwrap();
    for id in ["", "not-a-uuid", "00000000-0000-0000-0000-00000000000g"] {
        let error = bound
            .chat_text_stream(&[("user", "你好")], 64, id, |_| {})
            .await
            .unwrap_err();
        assert_eq!(error.code, "RECOVERY_ERROR", "{id} 应在发送前被拒绝");
        let error = bound.chat_job(id).await.unwrap_err();
        assert_eq!(error.code, "RECOVERY_ERROR", "{id} 不应发起查询");
    }
    assert_eq!(server.stream_calls.load(Ordering::SeqCst), 0);
    assert_eq!(server.calls.load(Ordering::SeqCst), 0);
    assert_eq!(server.polls.load(Ordering::SeqCst), 0);
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

// 仅本机协议夹具：验证请求恢复，不作为真实照片质量评测。
async fn recovery_fixture(
    run_to_unknown: bool,
) -> (
    tempfile::TempDir,
    between_frames_lib::db::Store,
    Server,
    Arc<Vault>,
    String,
    Task,
) {
    recovery_fixture_with_options(run_to_unknown, false).await
}

async fn recovery_fixture_with_options(
    run_to_unknown: bool,
    allow_queued_recovery: bool,
) -> (
    tempfile::TempDir,
    between_frames_lib::db::Store,
    Server,
    Arc<Vault>,
    String,
    Task,
) {
    use between_frames_lib::{agent::runtime::AgentEngine, db::Store, image_pipeline};
    let root = tempfile::tempdir().unwrap();
    let db = Store::open(&root.path().join("recovery.sqlite"))
        .await
        .unwrap();
    let p = db.create("恢复验证", "只验证协议").await.unwrap();
    let source = root.path().join("fixture.jpg");
    image::RgbImage::from_pixel(32, 24, image::Rgb([1, 2, 3]))
        .save(&source)
        .unwrap();
    db.insert_photo(
        &p.id,
        &image_pipeline::prepare(&source, &root.path().join("cache")).unwrap(),
    )
    .await
    .unwrap();
    let p = db.get(&p.id).await.unwrap();
    let server = Server::new();
    server.queue.store(allow_queued_recovery, Ordering::SeqCst);
    let vault = Arc::new(Vault::default());
    vault.write("", &session("user-a")).unwrap();
    let engine = AgentEngine::hosted(
        db.clone(),
        Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap()),
    );
    let consent = engine
        .set_consent(ConsentRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            project_id: p.id.clone(),
            mode: "task".into(),
            provider: engine.ai_config().consent_provider.unwrap(),
            quota: 1,
        })
        .await
        .unwrap();
    let task = engine
        .request(AgentRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            project_id: p.id.clone(),
            command: if allow_queued_recovery {
                TaskCommand::StartConversation {
                    allow_queued_recovery: true,
                    expected_account_id: Some("user-a".into()),
                    goal: "观察后恢复".into(),
                    selected_photo_ids: vec![],
                    focus_photo_ids: vec![],
                    target_count: 1,
                    budget: Budget {
                        max_cloud_images: 1,
                        ..Budget::default()
                    },
                    consent_id: Some(consent.id),
                    source_board: None,
                    continuation: None,
                }
            } else {
                TaskCommand::StartTask {
                    goal: "观察后恢复".into(),
                    selected_photo_ids: vec![],
                    target_count: 1,
                    budget: Budget {
                        max_cloud_images: 1,
                        ..Budget::default()
                    },
                    consent_id: Some(consent.id),
                }
            },
        })
        .await
        .unwrap();
    *server.main_reply.lock().unwrap() = Some(
        json!({"status":"completed","output":[{"type":"function_call","call_id":"inspect-once","name":"inspect_photos","arguments":json!({"project_id":p.id,"photo_ids":[p.photos[0].id]}).to_string()}],"usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}),
    );
    server.mode.store(9, Ordering::SeqCst);
    if run_to_unknown {
        assert_eq!(
            engine.run(&task.id).await.unwrap_err().code,
            "AI_RESULT_UNKNOWN"
        );
        assert_eq!(server.calls.load(Ordering::SeqCst), 2);
    }
    (root, db, server, vault, p.id, task)
}

#[tokio::test]
async fn hosted_recovery_survives_store_restart_queries_only_and_resumes_the_saved_tool_once() {
    use between_frames_lib::{
        agent::{self, runtime::AgentEngine},
        db::Store,
    };
    let (root, db, server, vault, project, task) = recovery_fixture(true).await;
    let before = db.get(&project).await.unwrap();
    let projection = agent::projection(&db, &project).await.unwrap();
    assert_eq!(projection.tasks[0].state, "paused");
    assert_eq!(
        projection.tasks[0].vision_recovery.as_ref().unwrap().images,
        1
    );
    let raw: String = sqlx::query_scalar(
        "SELECT body_json FROM agent_steps WHERE task_id=? AND kind='vision_reserved'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let receipt: Value = serde_json::from_str(&raw).unwrap();
    assert!(uuid::Uuid::parse_str(receipt["hostedRequest"]["requestId"].as_str().unwrap()).is_ok());
    assert_eq!(receipt["hostedRequest"]["identity"]["accountId"], "user-a");
    assert!(!raw.contains("data:image"));
    assert!(!raw.contains(&"f".repeat(64)));
    db.pool.close().await;
    let db = Store::open(&root.path().join("recovery.sqlite"))
        .await
        .unwrap();
    let engine = AgentEngine::hosted(
        db.clone(),
        Arc::new(CloudAuth::new(&server.origin, "fixture", vault).unwrap()),
    );
    let calls = server.calls.load(Ordering::SeqCst);
    server.mode.store(12, Ordering::SeqCst);
    assert_eq!(
        engine
            .query_vision_result(&project, &task.id)
            .await
            .unwrap()
            .state,
        "pending"
    );
    assert_eq!(server.calls.load(Ordering::SeqCst), calls);
    server.mode.store(6, Ordering::SeqCst);
    assert_eq!(
        engine
            .query_vision_result(&project, &task.id)
            .await
            .unwrap()
            .state,
        "completed"
    );
    assert_eq!(server.calls.load(Ordering::SeqCst), calls);
    let projection = agent::projection(&db, &project).await.unwrap();
    assert_eq!(projection.tasks[0].state, "paused");
    assert!(projection.tasks[0].vision_recovery.is_none());
    assert_eq!(projection.tasks[0].usage.cloud_images, 1);
    let settled: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM agent_steps WHERE task_id=? AND kind='vision_settled'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(settled, 1);
    assert!(
        engine
            .query_vision_result(&project, &task.id)
            .await
            .is_err()
    );
    assert_eq!(server.calls.load(Ordering::SeqCst), calls);
    engine
        .request(AgentRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            project_id: project.clone(),
            command: TaskCommand::ResumeTask {
                task_id: task.id.clone(),
                budget: task.budget.clone(),
            },
        })
        .await
        .unwrap();
    engine.run(&task.id).await.unwrap();
    assert_eq!(server.calls.load(Ordering::SeqCst), calls + 1); // 只新增下一轮文字，不重复发图。
    let observations: i64 = sqlx::query_scalar("SELECT count(*) FROM photo_observations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(observations, 1);
    assert_eq!(db.get(&project).await.unwrap().revision, before.revision);
    assert_eq!(
        db.get(&project).await.unwrap().photos[0].status,
        before.photos[0].status
    );
}

#[tokio::test]
async fn hosted_recovery_rejects_other_accounts_projects_revoked_consent_and_changed_photos_before_get()
 {
    use between_frames_lib::agent::runtime::AgentEngine;
    let (_root, db, server, vault, project, task) = recovery_fixture(true).await;
    let engine = AgentEngine::hosted(
        db.clone(),
        Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap()),
    );
    let polls = server.polls.load(Ordering::SeqCst);
    vault.write("", &session("user-b")).unwrap();
    assert_eq!(
        engine
            .query_vision_result(&project, &task.id)
            .await
            .unwrap_err()
            .code,
        "AUTH_CHANGED"
    );
    vault.write("", &session("user-a")).unwrap();
    assert_eq!(
        engine
            .query_vision_result("another-project", &task.id)
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    let photo = db.get(&project).await.unwrap().photos[0].id.clone();
    let hash: String = sqlx::query_scalar("SELECT content_sha256 FROM photos WHERE id=?")
        .bind(&photo)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE photos SET content_sha256='changed' WHERE id=?")
        .bind(&photo)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        engine
            .query_vision_result(&project, &task.id)
            .await
            .unwrap_err()
            .code,
        "REVISION_CONFLICT"
    );
    sqlx::query("UPDATE photos SET content_sha256=? WHERE id=?")
        .bind(hash)
        .bind(photo)
        .execute(&db.pool)
        .await
        .unwrap();
    engine
        .set_consent(ConsentRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            project_id: project.clone(),
            mode: "deny".into(),
            provider: "openai".into(),
            quota: 0,
        })
        .await
        .unwrap();
    assert_eq!(
        engine
            .query_vision_result(&project, &task.id)
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    assert_eq!(server.polls.load(Ordering::SeqCst), polls);
    assert_eq!(server.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn hosted_recovery_keeps_unknown_expired_and_invalid_metering_unsettled_without_reupload() {
    use between_frames_lib::agent::runtime::AgentEngine;
    let (_root, db, server, vault, project, task) = recovery_fixture(true).await;
    let engine = AgentEngine::hosted(
        db.clone(),
        Arc::new(CloudAuth::new(&server.origin, "fixture", vault).unwrap()),
    );
    for (mode, state) in [(9, Some("unknown")), (13, Some("expired")), (14, None)] {
        server.mode.store(mode, Ordering::SeqCst);
        let result = engine.query_vision_result(&project, &task.id).await;
        if let Some(state) = state {
            assert_eq!(result.unwrap().state, state);
        } else {
            assert_eq!(result.unwrap_err().code, "AI_RESULT_UNKNOWN");
        }
    }
    let settled: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM agent_steps WHERE task_id=? AND kind='vision_settled'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(settled, 0);
    assert_eq!(server.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn interrupted_hosted_upload_can_be_queried_after_restart_without_resubmission() {
    use between_frames_lib::{
        agent::{self, runtime::AgentEngine},
        command_handler::CommandHandler,
        db::Store,
    };
    let (root, db, server, vault, project, task) = recovery_fixture(false).await;
    let engine = AgentEngine::hosted(
        db.clone(),
        Arc::new(CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap()),
    );
    let task_id = task.id.clone();
    let running = tokio::spawn(async move { engine.run(&task_id).await });
    for _ in 0..100 {
        if server.calls.load(Ordering::SeqCst) >= 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert_eq!(server.calls.load(Ordering::SeqCst), 2);
    // 图片已经接收、回执轮询仍在等待时模拟进程退出。
    running.abort();
    assert!(running.await.unwrap_err().is_cancelled());
    db.pool.close().await;
    let db = Store::open(&root.path().join("recovery.sqlite"))
        .await
        .unwrap();
    CommandHandler::new(&db).recover_agents().await.unwrap();
    let projection = agent::projection(&db, &project).await.unwrap();
    assert_eq!(projection.tasks[0].state, "interrupted");
    assert!(projection.tasks[0].vision_recovery.is_some());
    assert_eq!(server.calls.load(Ordering::SeqCst), 2);
    server.mode.store(6, Ordering::SeqCst);
    let engine = AgentEngine::hosted(
        db.clone(),
        Arc::new(CloudAuth::new(&server.origin, "fixture", vault).unwrap()),
    );
    assert_eq!(
        engine
            .query_vision_result(&project, &task.id)
            .await
            .unwrap()
            .state,
        "completed"
    );
    assert_eq!(server.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        agent::projection(&db, &project).await.unwrap().tasks[0].state,
        "interrupted"
    );
}

#[tokio::test]
async fn altered_hosted_receipt_cannot_redirect_a_recovery_lookup() {
    use between_frames_lib::agent::runtime::AgentEngine;
    let (_root, db, server, vault, project, task) = recovery_fixture(true).await;
    let raw: String = sqlx::query_scalar(
        "SELECT body_json FROM agent_steps WHERE task_id=? AND kind='vision_reserved'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let mut changed: Value = serde_json::from_str(&raw).unwrap();
    changed["hostedRequest"]["requestId"] = json!(uuid::Uuid::new_v4().to_string());
    assert!(
        sqlx::query(
            "UPDATE agent_steps SET body_json=? WHERE task_id=? AND kind='vision_reserved'"
        )
        .bind(changed.to_string())
        .bind(&task.id)
        .execute(&db.pool)
        .await
        .is_err()
    );
    // 不移除 append-only 保护；模拟额外注入伪造记录，检查点仍必须拒绝。
    sqlx::query("INSERT INTO agent_steps(task_id,kind,body_json,summary,created_at) SELECT task_id,kind,?,'forged receipt',created_at FROM agent_steps WHERE task_id=? AND kind='vision_reserved' LIMIT 1")
        .bind(changed.to_string()).bind(&task.id).execute(&db.pool).await.unwrap();
    let polls = server.polls.load(Ordering::SeqCst);
    let engine = AgentEngine::hosted(
        db.clone(),
        Arc::new(CloudAuth::new(&server.origin, "fixture", vault).unwrap()),
    );
    assert_eq!(
        engine
            .query_vision_result(&project, &task.id)
            .await
            .unwrap_err()
            .code,
        "RECOVERY_ERROR"
    );
    assert_eq!(server.polls.load(Ordering::SeqCst), polls);
    assert_eq!(server.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn queue_permission_is_explicit_frozen_and_absent_from_legacy_tasks() {
    use between_frames_lib::agent::runtime::AgentEngine;
    let (root, db, server, vault, project, task) = recovery_fixture_with_options(true, true).await;
    assert_eq!(server.opt_ins.load(Ordering::SeqCst), 1);
    let raw: String = sqlx::query_scalar(
        "SELECT body FROM task_messages WHERE task_id=? AND kind='conversation_context'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&raw).unwrap()["allowQueuedRecovery"],
        true
    );
    let raw: String = sqlx::query_scalar(
        "SELECT body_json FROM agent_steps WHERE task_id=? AND kind='vision_reserved'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&raw).unwrap()["hostedRequest"]["allowQueuedRecovery"],
        true
    );
    db.pool.close().await;
    let db = between_frames_lib::db::Store::open(&root.path().join("recovery.sqlite"))
        .await
        .unwrap();
    // 服务后来关闭不影响只读查询原回执，也不会再次发送图片。
    server.queue.store(false, Ordering::SeqCst);
    server.mode.store(6, Ordering::SeqCst);
    let engine = AgentEngine::hosted(
        db.clone(),
        Arc::new(CloudAuth::new(&server.origin, "fixture", vault).unwrap()),
    );
    assert_eq!(
        engine
            .query_vision_result(&project, &task.id)
            .await
            .unwrap()
            .state,
        "completed"
    );
    assert_eq!(server.opt_ins.load(Ordering::SeqCst), 1);
    let (_root, db, server, vault, _project, task) = recovery_fixture(false).await;
    server.queue.store(true, Ordering::SeqCst);
    let engine = AgentEngine::hosted(
        db,
        Arc::new(CloudAuth::new(&server.origin, "fixture", vault).unwrap()),
    );
    assert!(engine.run(&task.id).await.is_err());
    assert_eq!(server.opt_ins.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn queue_capability_change_is_rejected_before_delegation_or_image_reservation() {
    use between_frames_lib::agent::runtime::AgentEngine;
    let (_root, db, server, vault, project, task) =
        recovery_fixture_with_options(false, true).await;
    server.queue.store(false, Ordering::SeqCst);
    let engine = AgentEngine::hosted(
        db.clone(),
        Arc::new(CloudAuth::new(&server.origin, "fixture", vault).unwrap()),
    );
    let error = engine
        .request(AgentRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            project_id: project,
            command: TaskCommand::StartConversation {
                allow_queued_recovery: true,
                expected_account_id: Some("user-a".into()),
                goal: "不应开始".into(),
                selected_photo_ids: vec![],
                focus_photo_ids: vec![],
                target_count: 1,
                budget: task.budget.clone(),
                consent_id: task.consent_id,
                source_board: None,
                continuation: None,
            },
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, "AI_QUEUE_NOT_AVAILABLE");
    assert_eq!(
        engine.run(&task.id).await.unwrap_err().code,
        "AI_QUEUE_NOT_AVAILABLE"
    );
    let reserved: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM agent_steps WHERE task_id=? AND kind='vision_reserved'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(reserved, 0);
    assert_eq!(server.opt_ins.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn local_cancellation_records_confirmed_already_sent_or_unconfirmed_without_reuploading() {
    use between_frames_lib::agent::{self, runtime::AgentEngine};
    for (mode, expected) in [(0, "cancelled"), (1, "already_sent"), (2, "unconfirmed")] {
        let (_root, db, server, vault, project, task) =
            recovery_fixture_with_options(true, true).await;
        server.cancel_mode.store(mode, Ordering::SeqCst);
        let engine = AgentEngine::hosted(
            db.clone(),
            Arc::new(CloudAuth::new(&server.origin, "fixture", vault).unwrap()),
        );
        let request = AgentRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            project_id: project.clone(),
            command: TaskCommand::CancelTask {
                task_id: task.id.clone(),
            },
        };
        assert_eq!(
            engine.request(request.clone()).await.unwrap().state,
            "cancelled"
        );
        assert_eq!(engine.request(request).await.unwrap().state, "cancelled");
        let view = agent::projection(&db, &project).await.unwrap();
        assert_eq!(view.tasks[0].queue_cancellation.as_deref(), Some(expected));
        assert_eq!(server.cancels.load(Ordering::SeqCst), 1);
        assert_eq!(server.opt_ins.load(Ordering::SeqCst), 1);
        assert_eq!(server.calls.load(Ordering::SeqCst), 2);
    }
}
