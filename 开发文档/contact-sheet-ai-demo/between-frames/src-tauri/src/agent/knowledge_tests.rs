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
    routes: Arc<Mutex<Vec<(String, u16, String)>>>,
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
        let routes = Arc::new(Mutex::new(Vec::<(String, u16, String)>::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (seen, response, routed, stopped) = (
            requests.clone(),
            reply.clone(),
            routes.clone(),
            stop.clone(),
        );
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
                let target = request.split_whitespace().nth(1).unwrap_or("").to_string();
                seen.lock().unwrap().push(request);
                let (status, body) = if auth {
                    (
                        200,
                        json!({"id":"account-a","email":"test@example.test","name":"Test"})
                            .to_string(),
                    )
                } else if let Some((_, status, body)) = routed
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|(prefix, _, _)| target.starts_with(prefix.as_str()))
                {
                    (*status, body.clone())
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
            routes,
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
async fn embedded_portfolio_method_keeps_provenance_and_never_searches_the_cloud() {
    let server = Server::new();
    let (client, _) = server.client();
    let result = execute(
        Some(&client),
        "search_photography_knowledge",
        &json!({"query":"portfolio-curation-v1","limit":1}),
    )
    .await
    .unwrap();
    let method = &result["results"][0];
    assert_eq!(method["passage"], PORTFOLIO_METHOD);
    assert_eq!(method["version"], "portfolio-curation-1");
    assert_ne!(method["license"], "CC0-1.0");
    assert_eq!(
        method["contentSha256"],
        fingerprint(&json!(PORTFOLIO_METHOD))
    );
    assert_eq!(method["sources"].as_array().unwrap().len(), 2);
    assert_eq!(result["visualEvidence"], false);
    assert!(server.requests.lock().unwrap().is_empty());
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
    // 四条工具 = 1 次身份 + 5 次检索；命中的实体各补取一次详情，共 10 次，不多打一次。
    assert_eq!(requests.len(), 10);
    for request in requests.iter() {
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer fixture-token")
        );
    }
    assert!(
        requests
            .iter()
            .filter(|r| r.starts_with("GET /api/v2/knowledge/photographers/one "))
            .count()
            == 4,
        "每条命中都按编号补取一次详情，且只取一次"
    );
    let request = requests.iter().find(|r| r.contains("movement=")).unwrap();
    let target = request.split_whitespace().nth(1).unwrap();
    let url = reqwest::Url::parse(&format!("{}{}", server.origin, target)).unwrap();
    let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(pairs["q"], "肖尔");
    assert_eq!(pairs["movement"], "New Color & 日本");
}
#[tokio::test]
async fn blank_genre_filter_omits_the_parameter_instead_of_failing_validation() {
    let server = Server::new();
    let (client, _) = server.client();
    *server.reply.lock().unwrap() = (
        200,
        json!([{"id":"one","title":"Rinko Kawauchi","kind":"photographer","reviewStatus":"verified"}])
            .to_string(),
    );
    // 工具契约是严格模式：不筛选流派时模型送空串，而不是省略字段。
    let result = execute(
        Some(&client),
        "search_photographers",
        &json!({"query":"川内","movement":""}),
    )
    .await
    .unwrap();
    assert_eq!(result["available"], true, "{result}");
    // 检索之后还会补取一次详情，所以要看的是那条检索请求，不是最后一条请求。
    let request = server
        .requests
        .lock()
        .unwrap()
        .iter()
        .find(|r| r.starts_with("GET /api/v2/knowledge/search"))
        .unwrap()
        .clone();
    let target = request.split_whitespace().nth(1).unwrap();
    assert!(!target.contains("movement="), "{target}");
    assert!(target.contains("q=%E5%B7%9D%E5%86%85"), "{target}");
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

/// 出处是加分项，不是前提：详情取不到时，已检索到的记录连同审校状态必须原样留下，并说明为什么没出处。
#[tokio::test]
async fn detail_failures_keep_search_results_and_never_invent_sources() {
    let server = Server::new();
    *server.reply.lock().unwrap() = (
        200,
        json!([{"id":"one","title":"肖尔","kind":"photographer","reviewStatus":"imported"}])
            .to_string(),
    );
    let (client, _) = server.client();
    for (status, code) in [
        (403u16, "KNOWLEDGE_FORBIDDEN"),
        (404, "KNOWLEDGE_UNAVAILABLE"),
    ] {
        *server.routes.lock().unwrap() = vec![(
            "/api/v2/knowledge/photographers/".to_string(),
            status,
            "secret-detail-body".to_string(),
        )];
        let blocked = execute(
            Some(&client),
            "search_photographers",
            &json!({"query":"肖尔","movement":"New Color"}),
        )
        .await
        .unwrap();
        assert_eq!(blocked["available"], true, "{blocked}");
        assert_eq!(blocked["photographers"][0]["detailError"]["code"], code);
        assert_eq!(blocked["photographers"][0]["reviewStatus"], "imported");
        assert!(blocked["photographers"][0]["citations"].is_null());
        assert!(!blocked.to_string().contains("secret-detail-body"));
        assert!(!blocked.to_string().contains("fixture-token"));
        assert!(
            !server
                .requests
                .lock()
                .unwrap()
                .iter()
                .any(|r| r.contains("unexpected-redirect")),
            "详情这一腿也不许跟着重定向跑"
        );
    }
}

/// 服务端给出未知实体类型时不拿它拼地址：当场拒绝，一次网络都不多发。
#[tokio::test]
async fn an_unknown_entity_kind_is_refused_before_the_detail_request() {
    let server = Server::new();
    *server.reply.lock().unwrap() = (
        200,
        json!([{"id":"one","title":"某展","kind":"exhibition","reviewStatus":"imported"}])
            .to_string(),
    );
    let (client, _) = server.client();
    let result = execute(
        Some(&client),
        "search_photographers",
        &json!({"query":"某展","movement":""}),
    )
    .await
    .unwrap();
    assert_eq!(result["photographers"][0]["name"], "某展");
    assert_eq!(
        result["photographers"][0]["detailError"]["code"],
        "KNOWLEDGE_PROTOCOL"
    );
    assert_eq!(
        server
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|request| request.starts_with("GET /api/v2/knowledge/"))
            .count(),
        1,
        "只该打检索那一次"
    );
}

/// 工具回执写进数据库后不可修改，长文与关联表必须在落库前封顶。
#[tokio::test]
async fn detail_citations_are_capped_before_they_enter_an_immutable_receipt() {
    let server = Server::new();
    *server.reply.lock().unwrap() = (
        200,
        json!([{"id":"one","title":"肖尔","kind":"photographer","reviewStatus":"imported"}])
            .to_string(),
    );
    let long = "长".repeat(600);
    let sources: Vec<Value> = (0..5)
        .map(|index| {
            json!({"url":format!("https://fixture.example/{index}"),"institution":"Fixture Museum","verified_at":"2026-11-03"})
        })
        .collect();
    let claims: Vec<Value> = (0..7)
        .map(|_| {
            json!({"claim_text":long,"claim_type":"historical_fact","disputed":false,"review_status":"imported","sources":sources})
        })
        .collect();
    *server.routes.lock().unwrap() = vec![(
        "/api/v2/knowledge/photographers/".to_string(),
        200u16,
        json!({"review_status":"imported","claims":claims,"sources":[]}).to_string(),
    )];
    let (client, _) = server.client();
    let result = execute(
        Some(&client),
        "search_photographers",
        &json!({"query":"肖尔","movement":""}),
    )
    .await
    .unwrap();
    let citations = &result["photographers"][0]["citations"];
    assert_eq!(citations["claims"].as_array().unwrap().len(), 4);
    assert_eq!(
        citations["claims"][0]["text"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        400
    );
    assert_eq!(
        citations["claims"][0]["sources"].as_array().unwrap().len(),
        3
    );
    assert!(
        serde_json::to_string(&result).unwrap().len() < 8 * 1024,
        "一次检索的回执不该把任务记录撑满"
    );
}

#[tokio::test]
async fn real_tool_pipeline_enforces_scope_and_replays_without_network() {
    use crate::agent::{execution, lifecycle, types::*};
    let server = Server::new();
    *server.reply.lock().unwrap() = (
        200,
        json!([{"id":"one","title":"肖尔","kind":"photographer","reviewStatus":"imported"}])
            .to_string(),
    );
    // 详情端点单独应答：断言、出处与审校状态都按服务端的 snake_case 原样返回。
    *server.routes.lock().unwrap() = vec![(
        "/api/v2/knowledge/photographers/".to_string(),
        200u16,
        json!({
          "id":"one","name_en":"Fixture Shore","review_status":"imported",
          "claims":[
            {"claim_text":"这条断言只用于验证投影。","claim_type":"historical_fact","disputed":false,"review_status":"verified","sources":[{"url":"https://fixture.example/collection","institution":"Fixture Museum","verified_at":"2026-11-03"}]},
            {"claim_text":"这条断言标着争议。","claim_type":"curatorial_interpretation","disputed":true,"dispute_notes":"两种说法并存","review_status":"imported","sources":[]}
          ],
          "sources":[{"url":"https://fixture.example/accession","institution":"Fixture Archive","accession_number":"FX-1","verified_at":"2026-11-03"}]
        })
        .to_string(),
    )];
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
                detail_photo_ids: vec![],
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
    // 1 次身份 + 1 次检索 + 1 次详情。
    assert_eq!(count, 3);
    let citations = result.result.as_ref().unwrap()["photographers"][0]["citations"].clone();
    assert_eq!(citations["claims"].as_array().unwrap().len(), 2);
    assert_eq!(
        citations["claims"][0]["sources"][0]["url"],
        "https://fixture.example/collection"
    );
    assert_eq!(citations["claims"][1]["disputed"], true);
    assert_eq!(citations["claims"][1]["reviewStatus"], "imported");
    // 出处要能进会话的"参考来源"，否则模型引用了也无人可查。
    let mut tx = db.pool.begin().await.unwrap();
    let projected = crate::agent::creative::references(&mut tx, &task.id)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    let verified = projected
        .iter()
        .find(|reference| reference["source"] == "https://fixture.example/collection")
        .unwrap_or_else(|| panic!("云端断言未投影为参考来源：{projected:?}"));
    assert_eq!(verified["kind"], "knowledge");
    assert_eq!(verified["reviewStatus"], "verified");
    assert_eq!(verified["citationId"], "one");
    assert_eq!(verified["visualEvidence"], false);
    assert!(
        projected
            .iter()
            .any(|reference| reference["disputed"] == true
                && reference["reviewStatus"] == "imported"),
        "存疑与未核实的标记不能在上屏时丢掉"
    );
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

/// 真实链路验收：macOS 系统钥匙串里的桌面会话 → 隔离回环 API → 已播种的 PostgreSQL 知识表。
/// 由 `node apps/api/test/run.mjs apps/api/test/knowledge-desktop.mjs` 显式驱动，不进离线 CI。
#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "需要 FOCALROOM_KNOWLEDGE_ACCEPTANCE_ORIGIN 指向隔离回环 API，并写入真实系统钥匙串"]
async fn real_pg_knowledge_session_over_macos_keychain() {
    use crate::cloud_credentials::SystemCredentials;
    use std::collections::HashMap;

    let origin = std::env::var("FOCALROOM_KNOWLEDGE_ACCEPTANCE_ORIGIN")
        .expect("真实知识验收只接受隔离回环 API 来源");
    let endpoint = reqwest::Url::parse(&origin).unwrap();
    assert_eq!(endpoint.scheme(), "http");
    assert_eq!(endpoint.host_str(), Some("127.0.0.1"));

    // 随机命名空间：验收只写自己这一条新条目，绝不读取或改写用户已有的登录条目。
    let namespace = format!(
        "studio.focalroom.knowledge-acceptance.{}",
        uuid::Uuid::new_v4()
    );
    let service = format!("{namespace}.cloud-session");
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = SystemCredentials.delete(&self.0);
        }
    }
    let _cleanup = Cleanup(service.clone());
    assert!(SystemCredentials.read(&service).unwrap().is_none());

    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();
    let email = format!("knowledge-acceptance-{}@fixture.test", uuid::Uuid::new_v4());
    let registered: serde_json::Value = json_request(
        &http,
        &format!("{origin}/api/v2/auth/register"),
        &json!({"email":email,"display_name":"知识验收","password":"isolated-acceptance-password"}),
        None,
    )
    .await;

    // 桌面登录协议全程走原生实现：PKCE 发起 → 浏览器侧授权 → 深链兑换 → 系统钥匙串。
    let auth = Arc::new(
        CloudAuth::new(&origin, &namespace, Arc::new(SystemCredentials)).expect("隔离验收来源"),
    );
    let login = reqwest::Url::parse(&auth.begin().await.unwrap()).unwrap();
    let query: HashMap<String, String> = login
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let approved = json_request(
        &http,
        &format!("{origin}/api/v2/auth/desktop/authorize"),
        &json!({"request_id":query["request"],"state":query["state"]}),
        Some(registered["token"].as_str().unwrap()),
    )
    .await;
    assert!(
        approved.get("token").is_none(),
        "桌面授权不得把会话令牌交给浏览器侧"
    );
    auth.complete(&format!(
        "focalroom://callback?code={}&state={}",
        approved["code"].as_str().unwrap(),
        query["state"]
    ))
    .await
    .unwrap();

    let stored: serde_json::Value = serde_json::from_str(
        &SystemCredentials
            .read(&service)
            .unwrap()
            .expect("桌面会话必须落在系统钥匙串"),
    )
    .unwrap();
    assert_eq!(
        stored["session"]["user"]["email"].as_str(),
        Some(email.as_str())
    );
    let token = stored["session"]["token"].as_str().unwrap().to_owned();

    let client = crate::agent::openai::OpenAiProvider::hosted(auth.clone())
        .knowledge_client()
        .expect("已登录会话应能提供知识客户端");

    // 命中：真实 PostgreSQL 里的中文姓名与流派数组过滤。
    let hit = cloud(Some(&client), "肖尔", Some("photographer"), None).await;
    assert_eq!(hit["available"], true, "{hit}");
    assert!(
        hit["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["title"] == "Stephen Shore" && entry["reviewStatus"] == "verified"),
        "{hit}"
    );
    let filtered = cloud(
        Some(&client),
        "肖尔",
        Some("photographer"),
        Some("New Color"),
    )
    .await;
    assert_eq!(filtered["available"], true);
    assert!(!filtered["results"].as_array().unwrap().is_empty());
    // 严格契约下模型总带流派字段，不筛选时是空串：等同不筛选，不能被服务拒掉。
    let blank = cloud(Some(&client), "肖尔", Some("photographer"), Some("")).await;
    assert_eq!(blank["available"], true, "{blank}");
    assert_eq!(blank["results"], hit["results"]);

    // 无结果：服务端正常应答但确实没有匹配资料，绝不能报成异常。
    for (query, kind, movement) in [
        ("肖尔", Some("photographer"), Some("北欧极简")),
        ("这条知识根本不存在zzz", None, None),
    ] {
        let absent = cloud(Some(&client), query, kind, movement).await;
        assert_eq!(absent["available"], true, "{absent}");
        assert_eq!(absent["results"], json!([]), "{absent}");
        assert!(absent.get("error").is_none(), "{absent}");
    }

    // 四个工具在真实服务上的形状：摘要可追溯，但不算视觉事实、也不当指令。
    for (name, args) in [
        ("search_photography_knowledge", json!({"query":"川内伦子"})),
        (
            "search_photographers",
            json!({"query":"川内","movement":"Contemporary Japanese Photography"}),
        ),
        (
            "get_photobook_structure",
            json!({"title":"Uncommon Places"}),
        ),
        (
            "compare_photography_series",
            json!({"seriesA":"美国表面","seriesB":"假寐"}),
        ),
    ] {
        let result = execute(Some(&client), name, &args).await.unwrap();
        assert_eq!(result["trustedAsInstructions"], false, "{result}");
        assert_eq!(result["visualEvidence"], false, "{result}");
        assert!(
            !result.to_string().contains(&token),
            "结果里不得出现会话令牌"
        );
    }
    let photographers = execute(
        Some(&client),
        "search_photographers",
        &json!({"query":"川内"}),
    )
    .await
    .unwrap();
    assert_eq!(photographers["available"], true, "{photographers}");
    assert_eq!(
        photographers["photographers"][0]["name"].as_str(),
        Some("Rinko Kawauchi")
    );
    let mixed = execute(
        Some(&client),
        "search_photography_knowledge",
        &json!({"query":"川内伦子"}),
    )
    .await
    .unwrap();
    assert_eq!(mixed["cloudAvailable"], true, "{mixed}");
    assert!(
        mixed["results"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["title"] == "Rinko Kawauchi"),
        "云端记录必须真的进入合并结果：{mixed}"
    );
    let compared = execute(
        Some(&client),
        "compare_photography_series",
        &json!({"seriesA":"美国表面","seriesB":"假寐"}),
    )
    .await
    .unwrap();
    assert_eq!(compared["available"], true, "{compared}");
    assert_eq!(
        compared["seriesA"]["title"].as_str(),
        Some("American Surfaces")
    );
    assert_eq!(compared["seriesB"]["title"].as_str(), Some("Utatane"));

    // 异常：另一处注销同一会话后，真实 401 必须报成不可用，而不是「没有资料」。
    json_request(
        &http,
        &format!("{origin}/api/v2/auth/logout"),
        &json!({}),
        Some(&token),
    )
    .await;
    let revoked = cloud(Some(&client), "肖尔", Some("photographer"), None).await;
    assert_eq!(revoked["available"], false, "{revoked}");
    assert_eq!(revoked["error"]["code"], "AUTH_REVOKED");
    assert_eq!(revoked["results"], json!([]));
    assert!(!revoked.to_string().contains(&token));

    // 本地登出之后，凭据当场消失，检索不再发出网络请求。
    assert!(auth.logout().await.unwrap());
    assert!(SystemCredentials.read(&service).unwrap().is_none());
    assert_eq!(
        cloud(Some(&client), "肖尔", None, None).await["error"]["code"],
        "AUTH_REQUIRED"
    );
}

#[cfg(target_os = "macos")]
async fn json_request(
    http: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
    token: Option<&str>,
) -> serde_json::Value {
    let mut request = http.post(url).json(body);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    request
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[test]
fn local_match_reason_reports_the_very_number_it_ranked_by() {
    let result = search(&json!({"query":"色彩 调色 ICC","limit":5})).unwrap();
    let rows = result["results"].as_array().unwrap();
    assert!(!rows.is_empty(), "{result}");
    assert_eq!(rows[0]["citationId"], "FR-K004", "{result}");
    let reason = &rows[0]["matchReason"];
    assert_eq!(reason["kind"], "local_terms");
    let terms: Vec<&str> = reason["matchedTerms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|term| term.as_str().unwrap())
        .collect();
    assert!(
        terms.contains(&"色彩") && terms.contains(&"icc"),
        "{reason}"
    );
    assert_eq!(reason["termCount"].as_u64().unwrap() as usize, terms.len());
    let fields: Vec<&str> = reason["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field.as_str().unwrap())
        .collect();
    assert!(
        fields.contains(&"tags") && fields.contains(&"title"),
        "{reason}"
    );
    // 报出来的命中词数必须就是排序用的那个数：从多到少，与实际带回的顺序一致。
    let counts: Vec<u64> = rows
        .iter()
        .map(|row| row["matchReason"]["termCount"].as_u64().unwrap())
        .collect();
    assert!(
        counts.windows(2).all(|pair| pair[0] >= pair[1]),
        "{counts:?}"
    );
    // 命中总数与带回条数分开报，不许让读者以为一共就这几条。
    assert_eq!(result["returned"].as_u64().unwrap() as usize, rows.len());
    assert!(result["matched"].as_u64().unwrap() >= result["returned"].as_u64().unwrap());
    for row in rows {
        assert_eq!(row["matchReason"]["matched"], result["matched"]);
        assert_eq!(row["matchReason"]["returned"], result["returned"]);
    }
    let capped = search(&json!({"query":"色彩 调色 ICC","limit":1})).unwrap();
    assert_eq!(capped["returned"], 1);
    assert_eq!(capped["matched"], result["matched"]);

    // 没命中就是没命中：条数与词数都如实报零，不编一条"相关资料"。
    let absent = search(&json!({"query":"zzzz不存在zzzz","limit":3})).unwrap();
    assert_eq!(absent["results"], json!([]));
    assert_eq!(absent["matched"], 0);
    assert_eq!(absent["returned"], 0);

    // 点名取用的自带方法不是词面命中，理由要分开说。
    let builtin = search(&json!({"query":"portfolio-curation-v1"})).unwrap();
    assert_eq!(
        builtin["results"][0]["matchReason"],
        json!({"kind":"builtin_method"})
    );
}

#[tokio::test]
async fn cloud_match_reason_carries_the_server_score_or_says_it_is_missing() {
    let server = Server::new();
    let (client, _) = server.client();
    *server.reply.lock().unwrap() = (
        200,
        json!([
            {"id":"scored","title":"Shore","kind":"photographer","reviewStatus":"verified","relevance":0.5},
            {"id":"unscored","title":"Araki","kind":"photographer","reviewStatus":"draft"}
        ])
        .to_string(),
    );
    let result = cloud(Some(&client), "shore", Some("photographer"), None).await;
    let rows = result["results"].as_array().unwrap();
    assert_eq!(
        rows[0]["matchReason"],
        json!({"kind":"cloud_search","relevance":0.5}),
        "{result}"
    );
    assert_eq!(
        rows[1]["matchReason"],
        json!({"kind":"cloud_search","relevance":null}),
        "服务端没给分数就说没给，不补一个自己编的数"
    );
}

#[tokio::test]
async fn every_receipt_keeps_the_wording_it_asked_for_even_when_nothing_comes_back() {
    // 没有客户端＝云端没授权：本地零命中也不能长成"没有资料"，而问过的词必须留在回执里，
    // 因为回执表只存参数指纹，事后没法从库里还原查的是哪个词。
    let empty = execute(
        None,
        "search_photography_knowledge",
        &json!({"query":"zzzzqqqq","limit":3}),
    )
    .await
    .unwrap();
    assert_eq!(empty["query"], "zzzzqqqq");
    assert!(empty["results"].as_array().unwrap().is_empty(), "{empty}");
    assert_eq!(
        empty["localMatched"], 0,
        "云端没答上来，本地命中数仍要单独报"
    );
    assert_eq!(empty["cloudAvailable"], false);
    assert_eq!(empty["cloudError"]["code"], "AUTH_REQUIRED");

    let server = Server::new();
    let (client, _) = server.client();
    *server.reply.lock().unwrap() = (
        200,
        json!([
            {"id":"one","title":"Shore","kind":"photographer"},
            {"id":"two","title":"Araki","kind":"photographer"}
        ])
        .to_string(),
    );
    let mixed = execute(
        Some(&client),
        "search_photography_knowledge",
        &json!({"query":"zzzzqqqq"}),
    )
    .await
    .unwrap();
    assert_eq!(mixed["query"], "zzzzqqqq");
    assert_eq!(mixed["results"].as_array().unwrap().len(), 2);
    // 云端给了两条，不能替本地札记说"这个词查过了、有命中"。
    assert_eq!(mixed["localMatched"], 0);
    assert_eq!(mixed["localReturned"], 0);
    assert_eq!(mixed["cloudAvailable"], true);
    assert!(mixed["cloudError"].is_null(), "{mixed}");

    let book = execute(
        None,
        "get_photobook_structure",
        &json!({"title":"The Americans"}),
    )
    .await
    .unwrap();
    assert_eq!(book["query"], "The Americans");
    let pair = execute(
        None,
        "compare_photography_series",
        &json!({"seriesA":"Americans","seriesB":"Surface"}),
    )
    .await
    .unwrap();
    assert_eq!(pair["query"], "Americans / Surface");
    let long = execute(
        None,
        "search_photography_knowledge",
        &json!({"query":"x".repeat(400)}),
    )
    .await
    .unwrap();
    assert_eq!(
        long["query"].as_str().unwrap().chars().count(),
        200,
        "回执不可变，问法也要限长"
    );
}
