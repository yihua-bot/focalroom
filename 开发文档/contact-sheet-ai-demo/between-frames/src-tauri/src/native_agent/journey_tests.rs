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
                    if headers.starts_with("post /api/v2/ai/deepseek-flash-v1/responses/async ") {
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
