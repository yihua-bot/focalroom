use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use between_frames_lib::{
    cloud_auth::CloudAuth, cloud_credentials::CredentialStore, domain::Result,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Default)]
struct Vault(Mutex<HashMap<String, String>>);
impl CredentialStore for Vault {
    fn read(&self, s: &str) -> Result<Option<String>> {
        Ok(self.0.lock().unwrap().get(s).cloned())
    }
    fn write(&self, s: &str, v: &str) -> Result<()> {
        self.0.lock().unwrap().insert(s.into(), v.into());
        Ok(())
    }
    fn delete(&self, s: &str) -> Result<()> {
        self.0.lock().unwrap().remove(s);
        Ok(())
    }
}
struct Server {
    origin: String,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    exchanges: Arc<Mutex<u32>>,
    reject: Arc<AtomicBool>,
    device_keys: Arc<Mutex<Vec<String>>>,
    device_bindings: Arc<Mutex<u32>>,
}
impl Server {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server_origin = origin.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let exchanges = Arc::new(Mutex::new(0));
        let calls = exchanges.clone();
        let reject = Arc::new(AtomicBool::new(false));
        let denied = reject.clone();
        let device_keys = Arc::new(Mutex::new(Vec::new()));
        let registered = device_keys.clone();
        let device_bindings = Arc::new(Mutex::new(0));
        let bound = device_bindings.clone();
        let thread = std::thread::spawn(move || {
            let mut state = String::new();
            let mut challenge = String::new();
            let mut used = false;
            let mut revoked = false;
            while !stopping.load(Ordering::SeqCst) {
                let (mut stream, _) = match listener.accept() {
                    Ok(x) => x,
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        continue;
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buf = [0; 2048];
                let boundary;
                loop {
                    let n = stream.read(&mut buf).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(i) = bytes.windows(4).position(|x| x == b"\r\n\r\n") {
                        boundary = i + 4;
                        break;
                    }
                }
                let headers = String::from_utf8_lossy(&bytes[..boundary]).to_string();
                let len = headers
                    .lines()
                    .find_map(|s| {
                        s.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(|s| s.parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                while bytes.len() < boundary + len {
                    let n = stream.read(&mut buf).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                }
                let body: Value =
                    serde_json::from_slice(&bytes[boundary..boundary + len]).unwrap_or(json!({}));
                let mut status = 200;
                let user =
                    json!({"id":"fixture-user","email":"user@fixture.test","name":"Fixture"});
                let response = if headers.starts_with("POST /api/v2/auth/desktop/start ") {
                    state = body["state"].as_str().unwrap().into();
                    challenge = body["code_challenge"].as_str().unwrap().into();
                    used = false;
                    json!({"request_id":"a".repeat(64),"login_url":format!("{server_origin}/shared?desktop=1&request={}&state={state}","a".repeat(64))})
                } else if headers.starts_with("POST /api/v2/auth/desktop/exchange ") {
                    *calls.lock().unwrap() += 1;
                    assert_eq!(body["state"], state);
                    assert_eq!(body["code"], "b".repeat(64));
                    assert_eq!(
                        URL_SAFE_NO_PAD.encode(Sha256::digest(
                            body["code_verifier"].as_str().unwrap().as_bytes()
                        )),
                        challenge
                    );
                    if used || denied.load(Ordering::SeqCst) {
                        status = 400;
                        json!({"error":{"code":"DESKTOP_LOGIN_INVALID"}})
                    } else {
                        used = true;
                        json!({"token":"f".repeat(64),"user":user})
                    }
                } else if headers.starts_with("POST /api/v2/devices ") && !revoked {
                    assert!(
                        headers
                            .to_lowercase()
                            .contains(&format!("authorization: bearer {}", "f".repeat(64)))
                    );
                    assert_eq!(body["platform"], "mac");
                    let key = body["device_key"].as_str().unwrap();
                    assert!(uuid::Uuid::parse_str(key).is_ok());
                    registered.lock().unwrap().push(key.to_owned());
                    json!({"id":"11111111-1111-4111-8111-111111111111"})
                } else if headers.starts_with(
                    "POST /api/v2/devices/11111111-1111-4111-8111-111111111111/session ",
                ) && !revoked
                {
                    *bound.lock().unwrap() += 1;
                    json!({"bound":true})
                } else if headers.starts_with("GET /api/v2/auth/me ") && !revoked {
                    assert!(
                        headers
                            .to_lowercase()
                            .contains(&format!("authorization: bearer {}", "f".repeat(64)))
                    );
                    user
                } else if headers.starts_with("POST /api/v2/auth/logout ") {
                    revoked = true;
                    json!({"logged_out":true})
                } else {
                    status = 401;
                    json!({"error":{"code":"AUTH_REVOKED"}})
                };
                let payload = response.to_string();
                write!(stream,"HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",payload.len()).unwrap();
            }
        });
        Self {
            origin,
            stop,
            thread: Some(thread),
            exchanges,
            reject,
            device_keys,
            device_bindings,
        }
    }
    fn callback(&self, url: &str) -> String {
        let u = reqwest::Url::parse(url).unwrap();
        let state = u
            .query_pairs()
            .find(|(k, _)| k == "state")
            .unwrap()
            .1
            .into_owned();
        format!("focalroom://callback?code={}&state={state}", "b".repeat(64))
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
#[tokio::test]
async fn desktop_device_identity_survives_restart_and_binds_each_session() {
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("device-id");
    let auth = CloudAuth::new(&server.origin, "device-fixture", vault.clone()).unwrap();
    auth.set_device_file(file.clone());
    let url = auth.begin().await.unwrap();
    auth.complete(&server.callback(&url)).await.unwrap();
    assert_eq!(*server.device_bindings.lock().unwrap(), 1);
    let original = std::fs::read_to_string(&file).unwrap();
    let restarted = CloudAuth::new(&server.origin, "device-fixture", vault).unwrap();
    restarted.set_device_file(file.clone());
    assert!(restarted.status().await.unwrap().authenticated);
    assert_eq!(*server.device_bindings.lock().unwrap(), 2);
    assert!(
        server
            .device_keys
            .lock()
            .unwrap()
            .iter()
            .all(|key| key == &original)
    );
    restarted.logout().await.unwrap();
    assert_eq!(std::fs::read_to_string(file).unwrap(), original);
}

#[tokio::test]
async fn pkce_callback_stores_only_in_vault_rejects_replay_and_reuses_session() {
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    let auth = CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap();
    let url = auth.begin().await.unwrap();
    assert!(url.contains("desktop=1"));
    assert!(auth.status().await.unwrap().waiting);
    let callback = server.callback(&url);
    let bad = callback.replace("state=", "extra=");
    assert!(auth.complete(&bad).await.is_err());
    assert_eq!(*server.exchanges.lock().unwrap(), 0);
    auth.complete(&callback).await.unwrap();
    let status = auth.status().await.unwrap();
    assert!(status.authenticated);
    assert!(!status.waiting);
    let public = serde_json::to_string(&status).unwrap();
    assert!(!public.contains(&"f".repeat(64)));
    assert!(!public.contains("verifier"));
    assert!(auth.complete(&callback).await.is_err());
    assert_eq!(*server.exchanges.lock().unwrap(), 1);
    assert!(auth.bearer("https://evil.test").await.is_err());
    assert_eq!(auth.bearer(&server.origin).await.unwrap(), "f".repeat(64));
    drop(auth);
    let restarted = CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap();
    assert!(restarted.status().await.unwrap().authenticated);
    assert!(restarted.logout().await.unwrap());
    assert!(!restarted.status().await.unwrap().authenticated);
    assert!(vault.0.lock().unwrap().is_empty());
}
#[tokio::test]
async fn pending_survives_restart_and_cancel_prevents_late_return() {
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    let first = CloudAuth::new(&server.origin, "fixture", vault.clone()).unwrap();
    let url = first.begin().await.unwrap();
    drop(first);
    let second = CloudAuth::new(&server.origin, "fixture", vault).unwrap();
    second.complete(&server.callback(&url)).await.unwrap();
    assert!(second.status().await.unwrap().authenticated);
    let url = second.begin().await.unwrap();
    second.cancel().await.unwrap();
    assert!(second.complete(&server.callback(&url)).await.is_err());
    assert_eq!(*server.exchanges.lock().unwrap(), 1);
}
#[tokio::test]
async fn failed_redemption_never_sets_logged_in_and_cannot_replay_uncertain_result() {
    let server = Server::new();
    server.reject.store(true, Ordering::SeqCst);
    let vault = Arc::new(Vault::default());
    let auth = CloudAuth::new(&server.origin, "fixture", vault).unwrap();
    let url = auth.begin().await.unwrap();
    let callback = server.callback(&url);
    assert!(auth.complete(&callback).await.is_err());
    assert!(!auth.status().await.unwrap().authenticated);
    assert!(auth.complete(&callback).await.is_err());
    assert_eq!(*server.exchanges.lock().unwrap(), 1);
}
#[cfg(target_os = "macos")]
#[test]
#[ignore = "显式使用独立随机服务名验证系统钥匙串，不触碰用户条目"]
fn native_cloud_keychain_roundtrip() {
    use between_frames_lib::cloud_credentials::SystemCredentials;
    let service = format!("studio.focalroom.login-test.{}", uuid::Uuid::new_v4());
    let vault = SystemCredentials;
    assert!(vault.read(&service).unwrap().is_none());
    vault.write(&service, "isolated-fixture-secret").unwrap();
    assert_eq!(
        vault.read(&service).unwrap().as_deref(),
        Some("isolated-fixture-secret")
    );
    vault.delete(&service).unwrap();
    assert!(vault.read(&service).unwrap().is_none());
}

#[tokio::test]
async fn offline_status_blocks_cloud_writes_but_allows_local_logout() {
    let server = Server::new();
    let vault = Arc::new(Vault::default());
    let auth = CloudAuth::new(&server.origin, "offline-fixture", vault.clone()).unwrap();
    let url = auth.begin().await.unwrap();
    auth.complete(&server.callback(&url)).await.unwrap();
    drop(server);
    let status = auth.status().await.unwrap();
    assert!(!status.authenticated);
    assert!(status.user.is_some());
    assert!(status.error.is_some());
    assert!(!auth.logout().await.unwrap());
    assert!(vault.0.lock().unwrap().is_empty());
    assert!(auth.status().await.unwrap().user.is_none());
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "需要显式启动隔离回环PG API；实际系统钥匙串与原生HTTP协议验收"]
async fn real_pg_api_and_keychain_complete_one_desktop_session() {
    use between_frames_lib::cloud_credentials::SystemCredentials;
    let origin = std::env::var("FOCALROOM_DESKTOP_TEST_ORIGIN").expect("缺少隔离API来源");
    let parsed = reqwest::Url::parse(&origin).unwrap();
    assert_eq!(parsed.scheme(), "http");
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    let namespace = format!("studio.focalroom.pg-login-test.{}", uuid::Uuid::new_v4());
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = SystemCredentials.delete(&self.0);
        }
    }
    let _cleanup = Cleanup(format!("{namespace}.cloud-session"));
    let auth = CloudAuth::new(&origin, &namespace, Arc::new(SystemCredentials)).unwrap();
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let response=client.post(format!("{origin}/api/v2/auth/register")).json(&json!({"email":format!("pg-desktop-{}@fixture.test",uuid::Uuid::new_v4()),"display_name":"Native PG fixture","password":"isolated-fixture-password"})).send().await.unwrap();
    assert!(response.status().is_success());
    let browser: Value = response.json().await.unwrap();
    let login = reqwest::Url::parse(&auth.begin().await.unwrap()).unwrap();
    let query: HashMap<String, String> = login
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let response = client
        .post(format!("{origin}/api/v2/auth/desktop/authorize"))
        .bearer_auth(browser["token"].as_str().unwrap())
        .json(&json!({"request_id":query["request"],"state":query["state"]}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    let approved: Value = response.json().await.unwrap();
    assert!(approved.get("token").is_none());
    let callback = format!(
        "focalroom://callback?code={}&state={}",
        approved["code"].as_str().unwrap(),
        query["state"]
    );
    auth.complete(&callback).await.unwrap();
    let status = auth.status().await.unwrap();
    assert!(status.authenticated);
    assert_eq!(
        status.user.unwrap().id,
        browser["user"]["id"].as_str().unwrap()
    );
    drop(auth);
    let restarted = CloudAuth::new(&origin, &namespace, Arc::new(SystemCredentials)).unwrap();
    assert!(restarted.status().await.unwrap().authenticated);
    assert!(restarted.complete(&callback).await.is_err());
    assert!(restarted.logout().await.unwrap());
    assert!(
        SystemCredentials
            .read(&format!("{namespace}.cloud-session"))
            .unwrap()
            .is_none()
    );
    let response = client
        .post(format!("{origin}/api/v2/auth/logout"))
        .bearer_auth(browser["token"].as_str().unwrap())
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
}
