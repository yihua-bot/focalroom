use super::*;
use crate::domain;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::AtomicBool,
    time::Duration,
};

struct Gateway {
    origin: String,
    requests: Arc<Mutex<Vec<Value>>>,
    cut_stream: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Gateway {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let cut_stream = Arc::new(AtomicBool::new(false));
        let cut = cut_stream.clone();
        let thread = std::thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 8192];
                let boundary = loop {
                    let n = stream.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                    if let Some(i) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&bytes[..boundary]).to_lowercase();
                let len = headers
                    .lines()
                    .find_map(|s| {
                        s.strip_prefix("content-length: ")
                            .map(|n| n.parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                while bytes.len() < boundary + len {
                    let n = stream.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                }
                let response = if headers.starts_with("get /api/v2/auth/me ") {
                    json!({"id":"fixture-user","email":"fixture@example.test","name":"测试"})
                } else if headers.starts_with("get /api/v2/ai/account ") {
                    json!({"policy":"deepseek-flash-v1","enabled":true,"verified":true,"available":1000000,"recovery":{"enabled":false}})
                } else if headers.starts_with("get /api/v2/ai/deepseek-flash-v1/responses/async/") {
                    let id = headers
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .rsplit('/')
                        .next()
                        .unwrap();
                    json!({"jobId":id,"state":"COMPLETED","result":{"status":"completed","model":"deepseek-flash","output":[{"type":"message","content":[{"type":"output_text","text":"先从光线与画面关系谈起。"}]}],"usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}})
                } else {
                    assert!(headers.starts_with("post /api/v2/ai/deepseek-flash-v1/responses"));
                    let body: Value =
                        serde_json::from_slice(&bytes[boundary..boundary + len]).unwrap();
                    captured.lock().unwrap().push(body.clone());
                    assert_eq!(body["store"], false);
                    let output = if body["tools"].as_array().is_none_or(|t| t.is_empty()) {
                        json!([{"type":"message","content":[{"type":"output_text","text":"先从光线与画面关系谈起。"}]}])
                    } else {
                        let brief = body["input"][0]["content"][0]["text"].as_str().unwrap();
                        let project = brief
                            .lines()
                            .find_map(|l| l.strip_prefix("项目 ID："))
                            .unwrap();
                        let task = brief
                            .lines()
                            .find_map(|l| l.strip_prefix("任务 ID："))
                            .unwrap();
                        let created = body["input"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|p| p["type"] == "function_call_output")
                            .filter_map(|p| {
                                serde_json::from_str::<Value>(p["output"].as_str()?).ok()
                            })
                            .find_map(|p| p["result"]["draft_id"].as_str().map(str::to_owned));
                        let (name, args) = if let Some(draft) = created {
                            (
                                "finish_task",
                                json!({"project_id":project,"task_id":task,"draft_ids":[draft],"summary":"已生成一张照片的待审草案。"}),
                            )
                        } else {
                            let raw = body["input"][1]["content"][0]["text"].as_str().unwrap();
                            let context: Value =
                                serde_json::from_str(&raw[raw.find('{').unwrap()..]).unwrap();
                            let photo = context["gui"]["selected_photo_ids"][0].as_str().unwrap();
                            (
                                "create_selection_draft",
                                json!({"project_id":project,"title":"原生共享核心测试草案","base_project_revision":context["project"]["base_revision"],"items":[{"photo_id":photo,"proposed_action":"keep","position":0,"rationale":"仅测试元数据，不是视觉结论。"}],"summary":"等待人工确认"}),
                            )
                        };
                        json!([{"type":"function_call","call_id":format!("native-{}",name),"name":name,"arguments":args.to_string(),"status":"completed"}])
                    };
                    let result = json!({"status":"completed","model":"deepseek-flash","output":output,"usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}});
                    if headers.starts_with("post /api/v2/ai/deepseek-flash-v1/responses/stream ") {
                        let id = headers
                            .lines()
                            .find_map(|l| l.strip_prefix("x-focalroom-request-id: "))
                            .unwrap();
                        let mut events = format!(
                            "data: {}\n\ndata: {}\n\n",
                            json!({"type":"start","request_id":id}),
                            json!({"type":"delta","text":"先从光线"})
                        );
                        if !cut.load(Ordering::SeqCst) {
                            events.push_str(&format!(
                                "data: {}\n\n",
                                json!({"type":"done","reply":result})
                            ));
                        }
                        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",events.len(),events).unwrap();
                        continue;
                    } else if headers
                        .starts_with("post /api/v2/ai/deepseek-flash-v1/responses/async ")
                    {
                        let id = headers
                            .lines()
                            .find_map(|l| l.strip_prefix("x-focalroom-request-id: "))
                            .unwrap();
                        json!({"jobId":id,"state":"COMPLETED","result":result})
                    } else {
                        result
                    }
                };
                let data = response.to_string();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",data.len(),data).unwrap();
            }
        });
        Self {
            origin,
            cut_stream,
            requests,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Gateway {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.thread.take().unwrap().join();
    }
}
fn ffi(value: Value) -> Value {
    let input = CString::new(value.to_string()).unwrap();
    let output = unsafe { focalroom_agent_call(input.as_ptr()) };
    let value: Value =
        serde_json::from_slice(unsafe { CStr::from_ptr(output) }.to_bytes()).unwrap();
    unsafe { focalroom_agent_free(output) };
    assert_eq!(value["ok"], true, "{value}");
    value["result"].clone()
}

#[test]
fn c_abi_runs_shared_chat_and_real_tool_loop_and_recovers_saved_drafts() {
    let gateway = Gateway::new();
    let root = tempfile::tempdir().unwrap();
    let open = || {
        ffi(json!({"operation":"open","directory":root.path(),"origin":gateway.origin,"token":"f".repeat(64),"user":{"id":"fixture-user","email":"fixture@example.test","name":"测试"}}))["handle"].as_u64().unwrap()
    };
    let handle = open();
    let request_id = domain::id();
    let chat = ffi(
        json!({"operation":"chat","handle":handle,"requestID":request_id,"turns":[{"role":"user","content":"怎样观察画面里的光线？"}]}),
    );
    assert_eq!(chat["text"], "先从光线与画面关系谈起。");
    assert_eq!(
        gateway.requests.lock().unwrap()[0]["instructions"],
        include_str!("../../../prompts/focal-chat-v2.md")
    );
    assert_eq!(
        gateway.requests.lock().unwrap()[0]["max_output_tokens"],
        1024
    );
    let photos: Vec<Value> = (0..600).map(|index| {
        let id=format!("photo-{index}");
        json!({"id":id,"filename":format!("{index}.jpg"),"sourceHash":format!("{:x}",Sha256::digest(id.as_bytes())),"status":"undecided","previewPath":null,"width":1200,"height":800})
    }).collect();
    let context = ffi(
        json!({"operation":"syncContext","handle":handle,"context":{"reference":"local:p","title":"测试","brief":"仅元数据","photos":photos,"sequence":[]}}),
    );
    let task = ffi(
        json!({"operation":"agentRequest","handle":handle,"request":{"requestId":domain::id(),"projectId":context["projectId"],"command":{"type":"StartConversation","expectedAccountId":"fixture-user","goal":"请创建选片草案，不做视觉分析。","selectedPhotoIds":[context["photoIDs"]["photo-599"]],"focusPhotoIds":[],"targetCount":1,"budget":agent::types::Budget::default(),"consentId":null,"sourceBoard":null}}}),
    );
    let mut projection = Value::Null;
    for _ in 0..200 {
        projection =
            ffi(json!({"operation":"projection","handle":handle,"project":context["projectId"]}));
        if projection["tasks"][0]["state"] == "review_ready" {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        projection["tasks"][0]["state"], "review_ready",
        "{projection}"
    );
    assert_eq!(projection["tasks"][0]["id"], task["id"]);
    assert_eq!(
        projection["drafts"][0]["items"][0]["photo_id"],
        context["photoIDs"]["photo-599"]
    );
    assert!(
        projection["tasks"][0]["inspectedPhotos"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let project =
        ffi(json!({"operation":"getProject","handle":handle,"project":context["projectId"]}));
    assert_eq!(
        project["photos"][0]["status"], "undecided",
        "模型不能直接批准草案"
    );
    assert_eq!(project["photos"].as_array().unwrap().len(), 600);
    let calls = gateway.requests.lock().unwrap().len();
    ffi(json!({"operation":"close","handle":handle}));
    let restored = open();
    let saved =
        ffi(json!({"operation":"projection","handle":restored,"project":context["projectId"]}));
    assert_eq!(saved["drafts"][0]["id"], projection["drafts"][0]["id"]);
    assert_eq!(
        gateway.requests.lock().unwrap().len(),
        calls,
        "重开只能读记录，不重新请求模型"
    );
    ffi(json!({"operation":"close","handle":restored}));
}

/// 只读宿主不能靠挑一个"天生带画布权限"的命令绕过权限收紧。
#[tokio::test]
async fn readonly_host_cannot_open_a_canvas_task_without_canvas_grant() {
    use agent::types::{AgentRequest, Budget, TaskCommand};
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("readonly-canvas.sqlite"))
        .await
        .unwrap();
    let project = db.create("只读画布委托", "").await.unwrap();
    let engine =
        AgentEngine::with_providers(db.clone(), Arc::new(agent::provider::MockProvider), None);
    let denied = engine
        .request_with_canvas_access(
            AgentRequest {
                request_id: domain::id(),
                project_id: project.id.clone(),
                command: TaskCommand::StartCanvasTask {
                    goal: "把这一组重排成新的叙事".into(),
                    selected_photo_ids: vec![],
                    target_count: 1,
                    budget: Budget::default(),
                    consent_id: None,
                    source_board: None,
                },
            },
            false,
        )
        .await
        .expect_err("只读宿主请求画布任务必须被拒绝");
    assert_eq!(denied.code, "PERMISSION_DENIED", "{denied:?}");
    let tasks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let grants: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_task_grants")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(tasks, 0, "拒绝要干净，不能留下半个任务");
    assert_eq!(grants, 0, "拒绝要干净，不能留下冻结授权");
}

#[tokio::test]
async fn readonly_host_cannot_upgrade_frozen_canvas_grants_by_replaying() {
    use agent::types::{AgentRequest, Budget, TaskCommand};
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("readonly.sqlite"))
        .await
        .unwrap();
    let project = db.create("只读来源", "").await.unwrap();
    let engine =
        AgentEngine::with_providers(db.clone(), Arc::new(agent::provider::MockProvider), None);
    let request = AgentRequest {
        request_id: domain::id(),
        project_id: project.id.clone(),
        command: TaskCommand::StartConversation {
            allow_queued_recovery: false,
            expected_account_id: None,
            goal: "讨论构图".into(),
            selected_photo_ids: vec![],
            focus_photo_ids: vec![],
            detail_photo_ids: vec![],
            target_count: 1,
            budget: Budget::default(),
            consent_id: None,
            source_board: None,
            continuation: None,
        },
    };
    let task = engine
        .request_with_canvas_access(request.clone(), false)
        .await
        .unwrap();
    engine
        .request_with_canvas_access(request, true)
        .await
        .unwrap();
    let raw: String =
        sqlx::query_scalar("SELECT grant_json FROM agent_task_grants WHERE task_id=?")
            .bind(&task.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let grant: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(grant["canvas"], false);
    assert!(
        !grant["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool == "place_photos_on_board")
    );
}

/// iPhone 侧真实链路验收：应用把已登录令牌交给共享核心的 C ABI 入口之后，知识工具用的就是桌面同一套代码。
/// 由 `node apps/api/test/run.mjs apps/api/test/knowledge-desktop.mjs` 在隔离服务与临时 PostgreSQL 上显式驱动。
#[test]
#[ignore = "需要 FOCALROOM_KNOWLEDGE_ACCEPTANCE_ORIGIN 指向隔离回环 API 与已播种知识数据"]
fn real_pg_knowledge_session_over_native_iphone_entry() {
    use agent::{
        execution, lifecycle,
        openai::OpenAiProvider,
        types::{AgentRequest, Budget, TaskCommand, ToolCall},
    };
    use std::collections::HashMap;

    let origin = std::env::var("FOCALROOM_KNOWLEDGE_ACCEPTANCE_ORIGIN")
        .expect("原生知识验收只接受隔离回环 API 来源");
    let endpoint = reqwest::Url::parse(&origin).unwrap();
    assert_eq!(endpoint.scheme(), "http");
    assert_eq!(endpoint.host_str(), Some("127.0.0.1"));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // 会话袋与 iPhone 上一致：令牌只留在进程内存里，持久化由应用侧的钥匙串负责。
    let vault = Arc::new(SessionVault::default());
    let auth = Arc::new(
        CloudAuth::new(
            &origin,
            &format!("native-knowledge-{}", uuid::Uuid::new_v4()),
            vault.clone(),
        )
        .unwrap(),
    );
    let email = format!("native-knowledge-{}@fixture.test", uuid::Uuid::new_v4());
    let (token, user): (String, Value) = runtime.block_on(async {
        let registered: Value = http
            .post(format!("{origin}/api/v2/auth/register"))
            .json(&json!({"email":email,"display_name":"原生验收","password":"isolated-acceptance-password"}))
            .send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
        let login = reqwest::Url::parse(&auth.begin().await.unwrap()).unwrap();
        let query: HashMap<String, String> = login
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        let approved: Value = http
            .post(format!("{origin}/api/v2/auth/desktop/authorize"))
            .bearer_auth(registered["token"].as_str().unwrap())
            .json(&json!({"request_id":query["request"],"state":query["state"]}))
            .send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
        auth.complete(&format!(
            "focalroom://callback?code={}&state={}",
            approved["code"].as_str().unwrap(),
            query["state"]
        ))
        .await
        .unwrap();
        let record: Value = serde_json::from_str(&vault.read("").unwrap().unwrap()).unwrap();
        (
            record["session"]["token"].as_str().unwrap().to_owned(),
            record["session"]["user"].clone(),
        )
    });
    let account = user["id"].as_str().unwrap().to_owned();

    // 打开会话即由服务核实令牌；账户与工作目录一一对应，与桌面共用同一实现。
    let root = tempfile::tempdir().unwrap();
    let handle = ffi(
        json!({"operation":"open","directory":root.path(),"origin":origin,"token":token,"user":user}),
    )["handle"]
        .as_u64()
        .unwrap();
    let client = client(handle).unwrap();
    assert_eq!(client.auth.account_id().unwrap(), account);

    let provider = OpenAiProvider::hosted(client.auth.clone());
    let identity = provider.identity();
    let knowledge = provider
        .knowledge_client()
        .expect("已登录会话应能提供知识客户端");
    let project = runtime
        .block_on(client.db.create("原生知识验收", "只用公开摄影史条目"))
        .unwrap()
        .id;
    let task = runtime
        .block_on(lifecycle::request_bound(
            &client.db,
            AgentRequest {
                request_id: domain::id(),
                project_id: project.clone(),
                command: TaskCommand::StartConversation {
                    allow_queued_recovery: false,
                    expected_account_id: Some(account.clone()),
                    goal: "核对公开摄影史条目".into(),
                    selected_photo_ids: vec![],
                    focus_photo_ids: vec![],
                    detail_photo_ids: vec![],
                    target_count: 1,
                    budget: Budget::default(),
                    consent_id: None,
                    source_board: None,
                    continuation: None,
                },
            },
            Some(&identity),
        ))
        .unwrap();
    runtime
        .block_on(execution::state(&client.db, &task.id, "planning", "test"))
        .unwrap();
    runtime
        .block_on(execution::state(&client.db, &task.id, "running", "test"))
        .unwrap();
    let search = |call_id: &str, query: &str| {
        ToolCall {
            call_id: call_id.into(),
            name: "search_photographers".into(),
            // 严格契约要求三个参数都齐；模型不筛选流派时送的是空串，必须当成"不筛选"。
            args: json!({"project_id":project,"query":query,"movement":""}),
        }
    };
    let call = |call_id: &str, query: &str| {
        runtime
            .block_on(execution::tool_with_knowledge(
                &client.db,
                &task.id,
                &search(call_id, query),
                None,
                Some(&knowledge),
            ))
            .unwrap()
    };

    let hit = call("native-knowledge-1", "川内");
    assert!(hit.ok, "{:?}", hit.error);
    let found = hit.result.as_ref().unwrap();
    assert_eq!(found["available"], true, "{found}");
    assert_eq!(found["photographers"][0]["name"], "Rinko Kawauchi");

    // 无结果是服务正常应答后的空列表，不是异常。
    let absent = call("native-knowledge-2", "这条知识根本不存在zzz");
    assert!(absent.ok, "{:?}", absent.error);
    assert_eq!(absent.result.as_ref().unwrap()["available"], true);
    assert_eq!(absent.result.as_ref().unwrap()["photographers"], json!([]));

    // 真实服务注销同一会话之后，同一条工具链必须报异常码，且不带出令牌。
    runtime.block_on(async {
        http.post(format!("{origin}/api/v2/auth/logout"))
            .bearer_auth(&token)
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
    });
    let revoked = call("native-knowledge-3", "川内");
    assert!(revoked.ok, "{:?}", revoked.error);
    let failed = revoked.result.as_ref().unwrap();
    assert_eq!(failed["available"], false, "{failed}");
    assert_eq!(failed["error"]["code"], "AUTH_REVOKED");
    assert!(
        !failed.to_string().contains(&token),
        "结果里不得出现会话令牌"
    );

    ffi(json!({"operation":"close","handle":handle}));
}

#[test]
fn native_stream_delivers_deltas_and_recovers_cutoff_without_a_second_submission() {
    extern "C" fn delta(text: *const c_char, context: usize) {
        let output = unsafe { &*(context as *const Mutex<String>) };
        output
            .lock()
            .unwrap()
            .push_str(unsafe { CStr::from_ptr(text) }.to_str().unwrap());
    }
    let gateway = Gateway::new();
    let root = tempfile::tempdir().unwrap();
    let handle=ffi(json!({"operation":"open","directory":root.path(),"origin":gateway.origin,"token":"f".repeat(64),"user":{"id":"fixture-user","email":"fixture@example.test","name":"测试"}}))["handle"].as_u64().unwrap();
    for cut in [false, true] {
        gateway.cut_stream.store(cut, Ordering::SeqCst);
        let id = domain::id();
        let input=CString::new(json!({"operation":"chat","handle":handle,"requestID":id,"turns":[{"role":"user","content":"谈谈光线"}]}).to_string()).unwrap();
        let deltas = Mutex::new(String::new());
        let output = unsafe {
            focalroom_agent_chat_stream(input.as_ptr(), Some(delta), &deltas as *const _ as usize)
        };
        let result: Value =
            serde_json::from_slice(unsafe { CStr::from_ptr(output) }.to_bytes()).unwrap();
        unsafe { focalroom_agent_free(output) };
        assert_eq!(*deltas.lock().unwrap(), "先从光线");
        assert_eq!(result["ok"], !cut);
        if cut {
            assert_eq!(result["error"]["code"], "AI_RESULT_UNKNOWN");
        }
        let before = gateway.requests.lock().unwrap().len();
        let job = ffi(json!({"operation":"chatJob","handle":handle,"requestID":id}));
        assert_eq!(job["state"], "completed");
        assert_eq!(job["text"], "先从光线与画面关系谈起。");
        assert_eq!(gateway.requests.lock().unwrap().len(), before);
        assert_eq!(
            gateway.requests.lock().unwrap().last().unwrap()["max_output_tokens"],
            1024
        );
    }
    ffi(json!({"operation":"close","handle":handle}));
}
