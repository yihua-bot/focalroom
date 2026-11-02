//! 原生客户端适配层：复用桌面 AgentEngine；不实现第二套推理或工具循环。
use crate::{
    agent::{self, runtime::AgentEngine, types::AgentRequest},
    cloud_auth::CloudAuth,
    cloud_credentials::CredentialStore,
    command_handler::CommandHandler,
    db::Store,
    domain::{AppError, Result},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    ffi::{CStr, CString, c_char},
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
mod context;
#[cfg(test)]
mod journey_tests;

#[derive(Default)]
struct SessionVault(Mutex<Option<String>>);
impl CredentialStore for SessionVault {
    fn read(&self, _: &str) -> Result<Option<String>> {
        Ok(self.0.lock().unwrap_or_else(|p| p.into_inner()).clone())
    }
    fn write(&self, _: &str, value: &str) -> Result<()> {
        *self.0.lock().unwrap_or_else(|p| p.into_inner()) = Some(value.into());
        Ok(())
    }
    fn delete(&self, _: &str) -> Result<()> {
        *self.0.lock().unwrap_or_else(|p| p.into_inner()) = None;
        Ok(())
    }
}
struct Client {
    directory: PathBuf,
    db: Store,
    engine: AgentEngine,
    auth: Arc<CloudAuth>,
    vault: Arc<SessionVault>,
}
static CLIENTS: OnceLock<Mutex<HashMap<u64, Arc<Client>>>> = OnceLock::new();
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static OPEN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
enum Operation {
    Open {
        directory: PathBuf,
        origin: String,
        token: String,
        user: crate::cloud_auth::CloudUser,
    },
    Close {
        handle: u64,
    },
    CreateProject {
        handle: u64,
        title: String,
        brief: String,
    },
    GetProject {
        handle: u64,
        project: String,
    },
    AgentRequest {
        handle: u64,
        request: AgentRequest,
        #[serde(rename = "canvasAccess", default = "canvas_access_default")]
        canvas_access: bool,
    },
    Projection {
        handle: u64,
        project: String,
    },
    Consent {
        handle: u64,
        request: agent::types::ConsentRequest,
    },
    QueryVisionResult {
        handle: u64,
        project: String,
        task: String,
    },
    Configuration {
        handle: u64,
    },
    Chat {
        handle: u64,
        turns: Vec<agent::types::ChatTurn>,
        #[serde(rename = "requestID")]
        request_id: String,
    },
    SyncContext {
        handle: u64,
        context: context::ContextInput,
    },
    Preference {
        handle: u64,
        project: String,
        task: String,
        proposal: String,
        statement: Option<String>,
    },
    BoardNodes {
        handle: u64,
        project: String,
        board: String,
        revision: i64,
    },
}
fn client(handle: u64) -> Result<Arc<Client>> {
    CLIENTS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&handle)
        .cloned()
        .ok_or_else(|| AppError::new("AUTH_REQUIRED", "请重新登录"))
}
fn canvas_access_default() -> bool {
    true
}
async fn execute(request: Operation) -> Result<Value> {
    match request {
        Operation::Open {
            directory,
            origin,
            token,
            user,
        } => {
            let _open_lock = OPEN_LOCK.lock().await;
            if !directory.is_absolute() || token.is_empty() || user.id.is_empty() {
                return Err(AppError::new("VALIDATION_ERROR", "原生会话配置不合法"));
            }
            let vault = Arc::new(SessionVault::default());
            vault.write(
                "",
                &json!({"session":{"token":token,"user":user}}).to_string(),
            )?;
            let auth = Arc::new(CloudAuth::new(
                &origin,
                "studio.focalroom.native-agent",
                vault.clone(),
            )?);
            // 账户身份由现有正式登录协议核实，不以客户端传入的 user.id 作为授权证明。
            auth.bearer(auth.origin()).await?;
            // 即使调用方误传同一个根目录，服务与账户仍不能共用执行记录。
            let directory = account_directory(&directory, auth.origin(), &auth.account_id()?);
            if CLIENTS
                .get_or_init(Default::default)
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .values()
                .any(|client| client.directory == directory)
            {
                return Err(AppError::new(
                    "SESSION_ALREADY_OPEN",
                    "请复用现有原生 AI 会话",
                ));
            }
            std::fs::create_dir_all(&directory)?;
            let db = Store::open(&directory.join("agent.sqlite")).await?;
            CommandHandler::new(&db).recover_agents().await?;
            let engine = AgentEngine::hosted(db.clone(), auth.clone());
            let handle = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            CLIENTS
                .get_or_init(Default::default)
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .insert(
                    handle,
                    Arc::new(Client {
                        directory,
                        db,
                        engine,
                        auth,
                        vault,
                    }),
                );
            Ok(json!({"handle":handle}))
        }
        Operation::Close { handle } => {
            let removed = CLIENTS
                .get_or_init(Default::default)
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .remove(&handle);
            if let Some(client) = removed {
                client.vault.delete("")?;
            }
            Ok(json!({}))
        }
        Operation::CreateProject {
            handle,
            title,
            brief,
        } => Ok(json!(client(handle)?.db.create(&title, &brief).await?)),
        Operation::GetProject { handle, project } => {
            Ok(json!(client(handle)?.db.get(&project).await?))
        }
        Operation::AgentRequest {
            handle,
            request,
            canvas_access,
        } => {
            let client = client(handle)?;
            let task = client
                .engine
                .request_with_canvas_access(request, canvas_access)
                .await?;
            if task.state == "queued" {
                client.engine.dispatch(&task.id).await;
            }
            Ok(json!(task))
        }
        Operation::Projection { handle, project } => {
            let client = client(handle)?;
            Ok(json!(
                agent::store::projection_for_account(
                    &client.db,
                    &project,
                    &client.auth.account_id()?
                )
                .await?
            ))
        }
        Operation::Consent { handle, request } => {
            Ok(json!(client(handle)?.engine.set_consent(request).await?))
        }
        Operation::QueryVisionResult {
            handle,
            project,
            task,
        } => Ok(json!(
            client(handle)?
                .engine
                .query_vision_result(&project, &task)
                .await?
        )),
        Operation::Configuration { handle } => Ok(json!(client(handle)?.engine.ai_config())),
        Operation::Chat {
            handle,
            turns,
            request_id,
        } => Ok(
            json!({"text": client(handle)?.engine.chat_turn_with_id(&turns, &request_id).await?}),
        ),
        Operation::SyncContext { handle, context } => {
            context::sync(&client(handle)?.db, context).await
        }
        Operation::Preference {
            handle,
            project,
            task,
            proposal,
            statement,
        } => {
            use agent::memory_control::{MemoryCommand, MemoryRequest};
            let client = client(handle)?;
            let expected_account_id = Some(client.auth.account_id()?);
            let command = match statement {
                Some(statement) => MemoryCommand::RememberPreference {
                    task_id: task,
                    proposal_id: proposal,
                    statement,
                    expected_account_id,
                },
                None => MemoryCommand::DismissPreferenceProposal {
                    task_id: task,
                    proposal_id: proposal,
                    expected_account_id,
                },
            };
            Ok(json!(
                CommandHandler::new(&client.db)
                    .execute_memory(MemoryRequest {
                        request_id: crate::domain::id(),
                        project_id: project,
                        command
                    })
                    .await?
            ))
        }
        Operation::BoardNodes {
            handle,
            project,
            board,
            revision,
        } => {
            let client = client(handle)?;
            let boards = client.db.list_boards(&project).await?;
            if !boards
                .iter()
                .any(|b| b.id == board && b.revision == revision)
            {
                return Err(AppError::new("PERMISSION_DENIED", "画布不属于此项目"));
            }
            Ok(json!(client.db.get_board_nodes(&board).await?))
        }
    }
}

fn account_directory(root: &std::path::Path, origin: &str, account: &str) -> PathBuf {
    use sha2::{Digest, Sha256};
    let identity = serde_json::to_vec(&(origin, account)).expect("字符串可序列化");
    root.join(format!("{:x}", Sha256::digest(identity)))
}

/// JSON 输入、带 ok/result 或 ok/error 的 JSON 输出。返回指针只可由 native_agent_free 释放。
///
/// # Safety
/// input 必须指向有效、以 NUL 结束的 UTF-8 字符串，调用期间不得修改。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn focalroom_agent_call(input: *const c_char) -> *mut c_char {
    let outcome = std::panic::catch_unwind(|| {
        if input.is_null() {
            return Err(AppError::new("VALIDATION_ERROR", "请求为空"));
        }
        let bytes = unsafe { CStr::from_ptr(input) }.to_bytes();
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(AppError::new("VALIDATION_ERROR", "请求过大"));
        }
        let request = serde_json::from_slice(bytes)
            .map_err(|_| AppError::new("VALIDATION_ERROR", "请求格式不合法"))?;
        let runtime = RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("原生 Agent runtime")
        });
        runtime.block_on(execute(request))
    })
    .unwrap_or_else(|_| {
        Err(AppError::new(
            "AI_RESULT_UNKNOWN",
            "分析意外中断，请先核对原任务",
        ))
    });
    let response = match outcome {
        Ok(value) => json!({"ok":true,"result":value}),
        Err(error) => json!({"ok":false,"error":error}),
    };
    CString::new(response.to_string())
        .expect("JSON 无 NUL 字节")
        .into_raw()
}

/// # Safety
/// output 必须是 focalroom_agent_call 返回的尚未释放的指针，或为空。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn focalroom_agent_free(output: *mut c_char) {
    if !output.is_null() {
        drop(unsafe { CString::from_raw(output) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ffi_rejects_invalid_requests_without_starting_a_model() {
        for input in ["{}", r#"{"operation":"configuration","handle":0}"#] {
            let input = CString::new(input).unwrap();
            let output = unsafe { focalroom_agent_call(input.as_ptr()) };
            let value: Value =
                serde_json::from_slice(unsafe { CStr::from_ptr(output) }.to_bytes()).unwrap();
            assert_eq!(value["ok"], false);
            assert!(value["error"]["code"].is_string());
            unsafe { focalroom_agent_free(output) };
        }
    }

    #[test]
    fn accounts_and_services_have_distinct_storage_directories() {
        let root = std::path::Path::new("/tmp/agent");
        let first = account_directory(root, "https://team.focroom.cc", "one");
        assert_eq!(
            first,
            account_directory(root, "https://team.focroom.cc", "one")
        );
        assert_ne!(
            first,
            account_directory(root, "https://team.focroom.cc", "two")
        );
        assert_ne!(
            first,
            account_directory(root, "http://localhost:1422", "one")
        );
        assert!(first.starts_with(root));
    }
}
