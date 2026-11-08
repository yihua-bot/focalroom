// 作者记忆仅在临时本地库测试；不读取用户素材或调用外部模型。
use between_frames_lib::{
    agent::{
        self,
        context::ContextBuilder,
        memory,
        memory_control::{MemoryCommand, MemoryRequest},
        provider::{ModelProvider, ModelTokenUsage, ModelTurnResult},
        runtime::AgentEngine,
        types::*,
    },
    command_handler::{CommandHandler, GuiCommandRequest},
    commands_generated::GuiCommand,
    db::Store,
    domain::{self, Project, Result},
    image_pipeline,
};
use serde_json::{Value, json};
use std::{future::Future, path::Path, pin::Pin, sync::Arc};
use tokio::sync::Notify;

async fn project(db: &Store, path: &Path, title: &str) -> Project {
    let p = db.create(title, "").await.unwrap();
    for index in 0..2 {
        let file = path.join(format!("{}-{index}.jpg", p.id));
        image::RgbImage::from_pixel(32, 24, image::Rgb([index, 42, 90]))
            .save(&file)
            .unwrap();
        db.insert_photo(
            &p.id,
            &image_pipeline::prepare(&file, &path.join("cache")).unwrap(),
        )
        .await
        .unwrap();
    }
    db.get(&p.id).await.unwrap()
}
async fn note(db: &Store, p: &Project, index: usize, text: &str) -> Project {
    let current = db.get(&p.id).await.unwrap();
    CommandHandler::new(db)
        .execute_gui(GuiCommandRequest::new(
            &p.id,
            current.revision,
            GuiCommand::SetPhotoNote {
                photo_id: p.photos[index].id.clone(),
                note: text.into(),
            },
        ))
        .await
        .unwrap();
    db.get(&p.id).await.unwrap()
}
async fn control(
    db: &Store,
    p: &Project,
    command: MemoryCommand,
) -> agent::memory_control::MemoryReceipt {
    CommandHandler::new(db)
        .execute_memory(MemoryRequest {
            request_id: domain::id(),
            project_id: p.id.clone(),
            command,
        })
        .await
        .unwrap()
}
async fn confirmed(db: &Store, p: &Project, word: &str) -> memory::HypothesisView {
    note(db, p, 0, word).await;
    note(db, p, 1, word).await;
    let h = memory::projection(db, &p.id)
        .await
        .unwrap()
        .hypotheses
        .into_iter()
        .find(|h| h.statement.contains(word))
        .unwrap();
    control(
        db,
        p,
        MemoryCommand::ConfirmHypothesis {
            hypothesis_id: h.id,
        },
    )
    .await;
    h
}
async fn start(db: &Store, p: &Project, goal: &str) -> Task {
    CommandHandler::new(db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::StartTask {
                goal: goal.into(),
                selected_photo_ids: vec![],
                target_count: 1,
                budget: Budget::default(),
                consent_id: None,
            },
        })
        .await
        .unwrap()
}
async fn state(db: &Store, task: &Task) -> String {
    sqlx::query_scalar("SELECT state FROM agent_tasks WHERE id=?")
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .unwrap()
}
async fn setup(path: &Path) -> (Store, Project, Project) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let source = project(&db, path, "来源").await;
    let target = project(&db, path, "当前").await;
    (db, source, target)
}

#[tokio::test]
async fn author_opt_in_is_audited_idempotent_and_does_not_accept_legacy_true() {
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    confirmed(&db, &source, "留白").await;
    assert!(
        !memory::projection(&db, &target.id)
            .await
            .unwrap()
            .cross_project_enabled
    );
    sqlx::query("INSERT INTO memory_scope_settings VALUES('local',1,'legacy')")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(
        !agent::memory_scope::get_settings(&db, "local")
            .await
            .unwrap()
            .cross_project_enabled
    );
    assert!(
        memory::projection(&db, &target.id)
            .await
            .unwrap()
            .author_hypotheses
            .is_empty()
    );
    let request = MemoryRequest {
        request_id: domain::id(),
        project_id: target.id.clone(),
        command: MemoryCommand::SetCrossProjectMemory { enabled: true },
    };
    let receipt = CommandHandler::new(&db)
        .execute_memory(request.clone())
        .await
        .unwrap();
    assert!(receipt.changed);
    let settings = agent::memory_scope::get_settings(&db, "local")
        .await
        .unwrap();
    assert!(settings.cross_project_enabled);
    let replay = CommandHandler::new(&db)
        .execute_memory(request.clone())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(receipt).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    assert!(
        !control(
            &db,
            &target,
            MemoryCommand::SetCrossProjectMemory { enabled: true }
        )
        .await
        .changed
    );
    assert_eq!(
        settings,
        agent::memory_scope::get_settings(&db, "local")
            .await
            .unwrap()
    );
    let conflict = CommandHandler::new(&db)
        .execute_memory(MemoryRequest {
            command: MemoryCommand::SetCrossProjectMemory { enabled: false },
            ..request
        })
        .await
        .unwrap_err();
    assert_eq!(conflict.code, "IDEMPOTENCY_CONFLICT");
    assert!(
        !agent::memory_scope::get_settings(&db, "another-user")
            .await
            .unwrap()
            .cross_project_enabled
    );
    let view = memory::projection(&db, &target.id).await.unwrap();
    assert!(view.hypotheses.is_empty());
    assert!(view.evidence.is_empty());
    assert_eq!(view.author_hypotheses.len(), 1);
    assert_eq!(view.author_hypotheses[0].project_id, source.id);
}

#[tokio::test]
async fn consent_failure_rolls_back_setting_and_task_pause() {
    let temp = tempfile::tempdir().unwrap();
    let (db, _, target) = setup(temp.path()).await;
    let task = start(&db, &target, "留白").await;
    sqlx::query("CREATE TRIGGER reject_memory BEFORE INSERT ON memory_commands BEGIN SELECT RAISE(ABORT,'receipt fault'); END")
        .execute(&db.pool).await.unwrap();
    assert!(
        CommandHandler::new(&db)
            .execute_memory(MemoryRequest {
                request_id: domain::id(),
                project_id: target.id.clone(),
                command: MemoryCommand::SetCrossProjectMemory { enabled: true },
            })
            .await
            .is_err()
    );
    assert!(
        !agent::memory_scope::get_settings(&db, "local")
            .await
            .unwrap()
            .cross_project_enabled
    );
    assert_eq!(state(&db, &task).await, "queued");
    ContextBuilder::build(&db, &task.id).await.unwrap();
}

#[tokio::test]
async fn only_relevant_confirmed_local_sources_are_returned_with_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    let h = confirmed(&db, &source, "留白").await;
    let other = project(&db, temp.path(), "其他").await;
    note(&db, &other, 0, "留白").await;
    note(&db, &other, 1, "留白").await; // 未确认不参与。
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let formal = serde_json::to_value(db.get(&target.id).await.unwrap()).unwrap();
    let task = start(&db, &target, "留白").await;
    let context = ContextBuilder::build(&db, &task.id).await.unwrap();
    let list = context.layers["human_evidence"]["hypotheses"]
        .as_array()
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["memory_id"], h.id);
    assert_eq!(list[0]["scope"], "cross_project");
    assert_eq!(list[0]["origin"], "user_confirmed");
    assert_eq!(list[0]["source_project_id"], source.id);
    assert_eq!(list[0]["evidence_refs"][0]["project_id"], source.id);
    let evidence = memory::projection(&db, &source.id)
        .await
        .unwrap()
        .hypotheses[0]
        .evidence_ids
        .clone();
    assert_eq!(list[0]["evidence_ids"], json!(evidence));
    for photo in &source.photos {
        assert!(!context.layers.to_string().contains(&photo.id));
        assert!(!context.layers.to_string().contains(&photo.name));
    }
    let mut tx = db.pool.begin().await.unwrap();
    let mut unrelated = task.clone();
    unrelated.goal = "逆光".into();
    assert!(
        memory::retrievable_author(&mut tx, &unrelated, 10)
            .await
            .unwrap()["hypotheses"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut cloud = task.clone();
    cloud.provider_id = agent::PROVIDER_OPENAI.into();
    assert_eq!(
        memory::retrievable_author(&mut tx, &cloud, 10)
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    assert!(
        memory::retrievable_for_task(&mut tx, &cloud, 3)
            .await
            .unwrap()["hypotheses"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    tx.commit().await.unwrap();
    assert_eq!(
        serde_json::to_value(db.get(&target.id).await.unwrap()).unwrap(),
        formal
    );
    for mode in ["pending", "completed"] {
        sqlx::query("INSERT INTO v2_promotions(id,project_id,initiated_by,state,from_revision,created_at,updated_at) VALUES(?,?,'test',?,0,'now','now')")
            .bind(domain::id()).bind(&source.id).bind(mode).execute(&db.pool).await.unwrap();
        assert!(
            memory::projection(&db, &target.id)
                .await
                .unwrap()
                .author_hypotheses
                .is_empty()
        );
    }
    // 当前项目本身已共享时也禁止作者记忆。
    sqlx::query("INSERT INTO v2_promotions(id,project_id,initiated_by,state,from_revision,created_at,updated_at) VALUES(?,?,'test','completed',0,'now','now')")
        .bind(domain::id()).bind(&target.id).execute(&db.pool).await.unwrap();
    assert!(
        memory::projection(&db, &target.id)
            .await
            .unwrap()
            .author_hypotheses
            .is_empty()
    );
}

#[tokio::test]
async fn corrections_disabling_and_tombstones_block_duplicate_author_claims() {
    for operation in ["correct", "disable", "retire", "delete", "exclude"] {
        let temp = tempfile::tempdir().unwrap();
        let (db, source, target) = setup(temp.path()).await;
        let h = confirmed(&db, &source, "留白").await;
        let duplicate = project(&db, temp.path(), "同词来源").await;
        confirmed(&db, &duplicate, "留白").await;
        control(
            &db,
            &target,
            MemoryCommand::SetCrossProjectMemory { enabled: true },
        )
        .await;
        assert_eq!(
            memory::projection(&db, &target.id)
                .await
                .unwrap()
                .author_hypotheses
                .len(),
            1
        );
        let command = match operation {
            "correct" => MemoryCommand::CorrectHypothesis {
                hypothesis_id: h.id,
                correction: "只是当时的需要".into(),
            },
            "disable" => MemoryCommand::SetHypothesisEnabled {
                hypothesis_id: h.id,
                enabled: false,
            },
            "retire" => MemoryCommand::RetireHypothesis {
                hypothesis_id: h.id,
            },
            "delete" => MemoryCommand::DeleteHypothesis {
                hypothesis_id: h.id,
            },
            _ => {
                let evidence = memory::projection(&db, &source.id).await.unwrap().evidence;
                for e in evidence {
                    control(
                        &db,
                        &source,
                        MemoryCommand::SetEvidenceExcluded {
                            evidence_id: e.id,
                            excluded: true,
                        },
                    )
                    .await;
                }
                // 无证据来源不参与；另一个有效来源仍可引用。
                assert_eq!(
                    memory::projection(&db, &target.id)
                        .await
                        .unwrap()
                        .author_hypotheses[0]
                        .project_id,
                    duplicate.id
                );
                continue;
            }
        };
        control(&db, &source, command).await;
        assert!(
            memory::projection(&db, &target.id)
                .await
                .unwrap()
                .author_hypotheses
                .is_empty(),
            "{operation}"
        );
    }
}

struct RetrieveScript;
impl ModelProvider for RetrieveScript {
    fn generate<'a>(
        &'a self,
        input: &'a agent::context::ModelTurnRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelOutcome>> + Send + 'a>> {
        Box::pin(async move {
            let round = input.protocol.iter().filter(|m| m.kind == "tool").count();
            Ok(if round < 2 {
                ModelOutcome::ToolCalls {
                    calls: vec![ToolCall {
                        call_id: format!("author-{round}"),
                        name: "retrieve_memory".into(),
                        args: json!({"project_id":input.task.project_id,"scope":if round == 0 {"project_local"} else {"global_confirmed"},"limit":5}),
                    }],
                }
            } else {
                ModelOutcome::NoAction {
                    reason: "只读检索完成".into(),
                }
            })
        })
    }
}

#[tokio::test]
async fn tool_alias_remains_scoped_and_revocation_blocks_receipt_replay_and_aba_resume() {
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    confirmed(&db, &source, "留白").await;
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let task = start(&db, &target, "留白").await;
    AgentEngine::with_provider(db.clone(), Arc::new(RetrieveScript))
        .run(&task.id)
        .await
        .unwrap();
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT result_json FROM tool_calls WHERE task_id=? ORDER BY rowid")
            .bind(&task.id)
            .fetch_all(&db.pool)
            .await
            .unwrap();
    let local: Value = serde_json::from_str(&rows[0]).unwrap();
    let author: Value = serde_json::from_str(&rows[1]).unwrap();
    assert_eq!(local["result"]["hypotheses"], json!([]));
    assert_eq!(
        author["result"]["hypotheses"][0]["source_project_id"],
        source.id
    );
    assert_eq!(author["result"]["human_facts"], json!([]));
    let call = ToolCall {
        call_id: "author-1".into(),
        name: "retrieve_memory".into(),
        args: json!({"project_id":target.id,"scope":"global_confirmed","limit":5}),
    };
    assert!(
        CommandHandler::new(&db)
            .agent_tool(&task.id, &call, None)
            .await
            .unwrap()
            .ok
    );
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: false },
    )
    .await;
    assert_eq!(
        CommandHandler::new(&db)
            .agent_tool(&task.id, &call, None)
            .await
            .unwrap_err()
            .code,
        "MEMORY_SCOPE_CHANGED"
    );
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    assert_eq!(
        ContextBuilder::build(&db, &task.id).await.unwrap_err().code,
        "CANCELLED"
    ); // 当前项目记忆版本亦变化。
    assert_eq!(
        CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: target.id.clone(),
                command: TaskCommand::ResumeTask {
                    task_id: task.id,
                    budget: Budget::default()
                }
            })
            .await
            .unwrap_err()
            .code,
        "REVISION_CONFLICT"
    );
    let fresh = start(&db, &target, "留白").await;
    ContextBuilder::build(&db, &fresh.id).await.unwrap();
    let foreign = ToolCall {
        call_id: "foreign".into(),
        name: "retrieve_memory".into(),
        args: json!({"project_id":source.id,"scope":"global_confirmed","limit":5}),
    };
    assert_eq!(
        CommandHandler::new(&db)
            .agent_tool(&fresh.id, &foreign, None)
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
}

#[tokio::test]
async fn source_controls_pause_consumers_and_archiving_invalidates_future_context() {
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    let h = confirmed(&db, &source, "留白").await;
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let task = start(&db, &target, "留白").await;
    control(
        &db,
        &source,
        MemoryCommand::CorrectHypothesis {
            hypothesis_id: h.id,
            correction: "不同意跨项目推断".into(),
        },
    )
    .await;
    assert_eq!(state(&db, &task).await, "paused");
    assert_eq!(
        ContextBuilder::build(&db, &task.id).await.unwrap_err().code,
        "MEMORY_SCOPE_CHANGED"
    );
    let task = start(&db, &target, "留白").await;
    sqlx::query("UPDATE projects SET deleted_at='now' WHERE id=?")
        .bind(&source.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let error = AgentEngine::new(db.clone())
        .run(&task.id)
        .await
        .unwrap_err();
    assert_eq!(error.code, "MEMORY_SCOPE_CHANGED");
    assert_eq!(state(&db, &task).await, "paused");
    assert!(
        memory::projection(&db, &target.id)
            .await
            .unwrap()
            .author_hypotheses
            .is_empty()
    );
}

struct WaitingProvider {
    entered: Arc<Notify>,
    release: Arc<Notify>,
}
impl ModelProvider for WaitingProvider {
    fn generate<'a>(
        &'a self,
        _: &'a agent::context::ModelTurnRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelOutcome>> + Send + 'a>> {
        Box::pin(async { unreachable!() })
    }
    fn generate_metered<'a>(
        &'a self,
        _: &'a agent::context::ModelTurnRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurnResult>> + Send + 'a>> {
        Box::pin(async move {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(ModelTurnResult {
                outcome: ModelOutcome::NoAction {
                    reason: "晚到回复".into(),
                },
                usage: Some(ModelTokenUsage {
                    input_tokens: 13,
                    output_tokens: 7,
                    total_tokens: 20,
                    model_id: "offline-test".into(),
                }),
            })
        })
    }
}
#[tokio::test]
async fn source_note_change_discards_inflight_reply_but_settles_reported_usage() {
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    let h = confirmed(&db, &source, "留白").await;
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let task = start(&db, &target, "留白").await;
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let engine = AgentEngine::with_provider(
        db.clone(),
        Arc::new(WaitingProvider {
            entered: entered.clone(),
            release: release.clone(),
        }),
    );
    let id = task.id.clone();
    let handle = tokio::spawn(async move { engine.run(&id).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    note(&db, &source, 0, "新的留白判断").await;
    release.notify_one();
    let error = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.code, "MEMORY_SCOPE_CHANGED");
    assert_eq!(state(&db, &task).await, "paused");
    let usage: String = sqlx::query_scalar("SELECT usage_json FROM agent_tasks WHERE id=?")
        .bind(&task.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let usage: Value = serde_json::from_str(&usage).unwrap();
    assert_eq!(usage["inputEstimate"], 13);
    assert_eq!(usage["outputEstimate"], 7);
    // 每一轮请求记下真正读到的记忆及其来源与适用范围，事后能核对模型当时看见了什么。
    let raw: String = sqlx::query_scalar(
        "SELECT body_json FROM agent_steps WHERE task_id=? AND kind='model_attempt' ORDER BY id LIMIT 1",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let attempt: Value = serde_json::from_str(&raw).unwrap();
    let reads = attempt["memoryReads"].as_array().unwrap();
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0]["memoryId"], h.id);
    assert_eq!(reads[0]["sourceProjectId"], source.id);
    assert_eq!(reads[0]["scope"], "cross_project");
    assert_eq!(reads[0]["origin"], "user_confirmed");
    assert!(!raw.contains("留白"), "只记 ID 与来源，不复制记忆正文");
    let accepted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_steps WHERE task_id=? AND kind IN('model','tool')",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(accepted, 0);
}

#[tokio::test]
async fn memory_reads_surface_in_references_and_stop_showing_text_when_source_moves() {
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    let h = confirmed(&db, &source, "留白").await;
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let task = start(&db, &target, "留白").await;
    // 走主动查询脚本：同一条记忆既被逐轮读进上下文，也被模型自己查回来过，两条来路要分开记。
    AgentEngine::with_provider(db.clone(), Arc::new(RetrieveScript))
        .run(&task.id)
        .await
        .unwrap();
    let reference = memory_reference(&db, &target, &task.id).await;
    assert_eq!(reference["memoryId"].as_i64().unwrap(), h.id);
    assert_eq!(reference["origin"], "user_confirmed");
    assert_eq!(reference["scope"], "cross_project");
    assert_eq!(reference["sourceProjectId"], source.id);
    assert!(reference["passage"].as_str().unwrap().contains("留白"));
    assert_eq!(reference["memoryTextVersion"], "verified");
    let original_statement: String =
        sqlx::query_scalar("SELECT statement FROM preference_hypotheses WHERE id=?")
            .bind(h.id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    sqlx::query("UPDATE preference_hypotheses SET statement='后来改写的内容' WHERE id=?")
        .bind(h.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let changed = memory_reference(&db, &target, &task.id).await;
    assert_eq!(changed["memoryTextVersion"], "changed");
    assert!(
        changed["passage"].is_null(),
        "同一个 ID 的新文字不能冒充历史读取"
    );
    sqlx::query("UPDATE preference_hypotheses SET statement=? WHERE id=?")
        .bind(&original_statement)
        .bind(h.id)
        .execute(&db.pool)
        .await
        .unwrap();
    // 轮数取自真实回执条数：同一条在同一轮里只算一次。
    let rounds: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_steps WHERE task_id=? AND kind='model_attempt' AND json_array_length(json_extract(body_json,'$.memoryReads'))>0",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(rounds >= 1);
    assert_eq!(reference["rounds"].as_i64().unwrap(), rounds);
    // 查询次数同样只认回执：数的是真带回这条记忆的查询回执条数。
    let retrievals: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tool_calls WHERE task_id=? AND EXISTS(SELECT 1 FROM json_each(result_json,'$.result.hypotheses') WHERE json_extract(value,'$.memory_id')=?)",
    )
    .bind(&task.id)
    .bind(h.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(retrievals >= 1);
    assert_eq!(reference["retrievals"].as_i64().unwrap(), retrievals);
    // 当前状态跟着一起出来：这条现在还在生效，界面就不该有多余的提醒。
    assert_eq!(reference["memoryStatus"], "confirmed");
    assert_eq!(reference["memoryDisabled"], false);
    sqlx::query("UPDATE preference_hypotheses SET disabled_for_retrieval=1 WHERE id=?")
        .bind(h.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let reference = memory_reference(&db, &target, &task.id).await;
    assert_eq!(reference["memoryDisabled"], true);
    assert_eq!(reference["memoryStatus"], "confirmed");
    assert!(reference["passage"].as_str().unwrap().contains("留白"));
    sqlx::query("UPDATE preference_hypotheses SET status='retired' WHERE id=?")
        .bind(h.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let reference = memory_reference(&db, &target, &task.id).await;
    assert_eq!(reference["memoryStatus"], "retired");
    // 这条记忆被挪到另一个项目：来源对不上就不能把另一行的文字当成它当时的正文，
    // "现在还作不作数"也一样不能拿另一行上的状态冒充。
    sqlx::query("UPDATE preference_hypotheses SET project_id=? WHERE id=?")
        .bind(&target.id)
        .bind(h.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let reference = memory_reference(&db, &target, &task.id).await;
    assert_eq!(reference["memoryId"].as_i64().unwrap(), h.id);
    assert_eq!(reference["origin"], "user_confirmed");
    assert!(reference["passage"].is_null(), "来源对不上时只留 id 与来路");
    assert!(reference["memoryStatus"].is_null());
    assert!(reference["memoryDisabled"].is_null());
    // 回执本身仍然是不可变的历史：改动记忆不重写已记录的读取。
    let step: String = sqlx::query_scalar(
        "SELECT body_json FROM agent_steps WHERE task_id=? AND kind='model_attempt' ORDER BY id LIMIT 1",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&step).unwrap()["memoryReads"][0]["sourceProjectId"],
        source.id
    );
}

async fn memory_reference(db: &Store, project: &Project, task_id: &str) -> Value {
    let view = agent::projection(db, &project.id).await.unwrap();
    let task = view.tasks.iter().find(|item| item.id == task_id).unwrap();
    task.creative_references
        .iter()
        .find(|reference| reference["kind"] == "memory")
        .cloned()
        .unwrap_or_else(|| panic!("没有记忆条目：{:?}", task.creative_references))
}

async fn memory_trace(db: &Store, project: &Project, task_id: &str) -> Value {
    agent::projection(db, &project.id)
        .await
        .unwrap()
        .tasks
        .into_iter()
        .find(|t| t.id == task_id)
        .unwrap()
        .memory_trace
}

#[tokio::test]
async fn memory_timeline_freezes_rounds_confirmations_and_query_results_without_claiming_adoption()
{
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    let h = confirmed(&db, &source, "留白").await;
    let confirmation:String=sqlx::query_scalar("SELECT request_id FROM memory_commands WHERE project_id=? AND changed=1 AND json_extract(command_json,'$.type')='ConfirmHypothesis' AND json_extract(command_json,'$.hypothesisId')=? ORDER BY rowid DESC LIMIT 1")
        .bind(&source.id).bind(h.id).fetch_one(&db.pool).await.unwrap();
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let task = start(&db, &target, "留白").await;
    AgentEngine::with_provider(db.clone(), Arc::new(RetrieveScript))
        .run(&task.id)
        .await
        .unwrap();
    let trace = memory_trace(&db, &target, &task.id).await;
    let events = trace["events"].as_array().unwrap();
    assert!(!events.is_empty());
    let queries: Vec<_> = events
        .iter()
        .filter(|e| e["kind"] == "query_result")
        .collect();
    assert_eq!(queries.len(), 2, "新查询轨迹与旧 tool 步骤不能重复展示");
    assert!(queries[0]["round"].as_i64().unwrap() > 0);
    let read = &events
        .iter()
        .find(|e| e["kind"] == "context_attempt")
        .unwrap()["reads"][0];
    assert_eq!(read["confirmation"]["requestId"], confirmation);
    assert_eq!(read["confirmation"]["action"], "ConfirmHypothesis");
    assert_eq!(read["statusAtRead"], "confirmed");
    assert_eq!(read["selectionReason"], "author_keyword_ranked");
    assert_eq!(read["sourceProjectTitle"], "来源");
    assert_eq!(read["memoryTextVersion"], "verified");
    assert_eq!(read["evidence"]["available"], h.evidence_ids.len());
    assert!(read["applicability"].is_null());
    let raw:String=sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='model_attempt' ORDER BY id LIMIT 1").bind(&task.id).fetch_one(&db.pool).await.unwrap();
    assert!(!raw.contains("留白"), "冻结来路，不复制偏好正文或来源备注");
    control(
        &db,
        &source,
        MemoryCommand::SetHypothesisEnabled {
            hypothesis_id: h.id,
            enabled: false,
        },
    )
    .await;
    control(
        &db,
        &source,
        MemoryCommand::SetEvidenceExcluded {
            evidence_id: h.evidence_ids[0],
            excluded: true,
        },
    )
    .await;
    let after = memory_trace(&db, &target, &task.id).await;
    let current = &after["events"][0]["reads"][0];
    assert_eq!(current["confirmation"]["requestId"], confirmation);
    assert_eq!(current["statusAtRead"], "confirmed");
    assert_eq!(current["disabled"], true);
    assert_eq!(current["evidence"]["excluded"], 1);
}

#[tokio::test]
async fn memory_timeline_never_backfills_legacy_reads_from_later_versions_or_confirmations() {
    use sha2::{Digest, Sha256};
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    let h = confirmed(&db, &source, "留白").await;
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let task = start(&db, &target, "留白").await;
    let old = json!({"memoryId":h.id,"sourceProjectId":source.id,"scope":"cross_project","origin":"user_confirmed"});
    let mut recorded = old.clone();
    recorded["statementSha256"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&h.statement).unwrap())
    ));
    recorded["statusAtRead"] = json!("unconfirmed");
    for (i, read) in [old, recorded].into_iter().enumerate() {
        sqlx::query("INSERT INTO agent_steps(task_id,kind,body_json,summary,created_at) VALUES(?,'model_attempt',?,'legacy',?)")
            .bind(&task.id).bind(json!({"round":i+1,"memoryReads":[read]}).to_string()).bind(domain::timestamp()).execute(&db.pool).await.unwrap();
    }
    let trace = memory_trace(&db, &target, &task.id).await;
    assert_eq!(
        trace["events"][0]["reads"][0]["memoryTextVersion"],
        "unrecorded"
    );
    assert!(trace["events"][0]["reads"][0]["passage"].is_null());
    let read = &trace["events"][1]["reads"][0];
    assert_eq!(read["memoryTextVersion"], "verified");
    assert_eq!(read["statusAtRead"], "unconfirmed");
    assert_eq!(read["currentStatus"], "confirmed");
    assert!(
        read["confirmation"].is_null(),
        "当前已有确认，不能伪造成旧读取记录过确认回执"
    );
    assert!(read["evidence"].is_null());
    sqlx::query("UPDATE preference_hypotheses SET statement='改写后的当前正文' WHERE id=?")
        .bind(h.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let changed = memory_trace(&db, &target, &task.id).await;
    assert_eq!(
        changed["events"][1]["reads"][0]["memoryTextVersion"],
        "changed"
    );
    assert!(changed["events"][1]["reads"][0]["passage"].is_null());
    sqlx::query("UPDATE preference_hypotheses SET project_id=? WHERE id=?")
        .bind(&target.id)
        .bind(h.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let moved = memory_trace(&db, &target, &task.id).await;
    assert_eq!(
        moved["events"][1]["reads"][0]["memoryTextVersion"],
        "source_unavailable"
    );
    assert!(moved["events"][1]["reads"][0]["currentStatus"].is_null());
    assert!(moved["events"][1]["reads"][0]["sourceProjectTitle"].is_null());
}

#[tokio::test]
async fn memory_timeline_bounds_history_and_marks_unreadable_records_instead_of_claiming_empty() {
    let temp = tempfile::tempdir().unwrap();
    let (db, _source, target) = setup(temp.path()).await;
    let task = start(&db, &target, "摄影讨论").await;
    for round in 1..=44 {
        let mut body = json!({"round":round,"memoryReads":[]});
        if round == 44 {
            body["padding"] = json!("x".repeat(65537));
        }
        sqlx::query("INSERT INTO agent_steps(task_id,kind,body_json,summary,created_at) VALUES(?,'model_attempt',?,'fixture',?)")
            .bind(&task.id).bind(body.to_string()).bind(domain::timestamp()).execute(&db.pool).await.unwrap();
    }
    let trace = memory_trace(&db, &target, &task.id).await;
    assert_eq!(trace["events"].as_array().unwrap().len(), 40);
    assert_eq!(trace["hasEarlierEvents"], true);
    assert_eq!(trace["events"][0]["round"], 5);
    assert_eq!(trace["events"][0]["recorded"], true);
    assert_eq!(trace["events"][39]["recorded"], false);
}

#[tokio::test]
async fn author_scope_limits_fail_closed_but_allow_disabling() {
    let temp = tempfile::tempdir().unwrap();
    let (db, _, target) = setup(temp.path()).await;
    for index in 0..100 {
        db.create(&format!("source-{index}"), "").await.unwrap();
    }
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let view = memory::projection(&db, &target.id).await.unwrap();
    assert!(view.author_hypotheses.is_empty());
    assert!(view.author_memory_notice.contains("100"));
    let error = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: target.id.clone(),
            command: TaskCommand::StartTask {
                goal: "留白".into(),
                selected_photo_ids: vec![],
                target_count: 1,
                budget: Budget::default(),
                consent_id: None,
            },
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, "MEMORY_SCOPE_LIMIT");
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: false },
    )
    .await;
    let task = start(&db, &target, "留白").await;
    ContextBuilder::build(&db, &task.id).await.unwrap();
}

#[tokio::test]
async fn checkpoint_cannot_replace_immutable_author_binding() {
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    confirmed(&db, &source, "留白").await;
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let task = start(&db, &target, "留白").await;
    sqlx::query("UPDATE agent_checkpoints SET context_json=json_remove(context_json,'$.authorMemoryBinding') WHERE task_id=?")
        .bind(&task.id).execute(&db.pool).await.unwrap();
    assert_eq!(
        ContextBuilder::build(&db, &task.id).await.unwrap_err().code,
        "RECOVERY_ERROR"
    );
    assert!(
        sqlx::query(
            "UPDATE agent_steps SET body_json='{}' WHERE task_id=? AND kind='author_memory_binding'"
        )
        .bind(&task.id)
        .execute(&db.pool)
        .await
        .is_err()
    );
    // 恢复只复制原绑定，不能重新授权变化后的来源。
    CommandHandler::new(&db).recover_agents().await.unwrap();
    note(&db, &source, 0, "新留白判断").await;
    assert_eq!(
        CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: target.id.clone(),
                command: TaskCommand::ResumeTask {
                    task_id: task.id,
                    budget: Budget::default()
                },
            })
            .await
            .unwrap_err()
            .code,
        "REVISION_CONFLICT"
    );
}

#[tokio::test]
async fn revoking_from_another_project_invalidates_old_history_even_after_regrant() {
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    confirmed(&db, &source, "留白").await;
    control(
        &db,
        &source,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let task = start(&db, &target, "留白").await;
    control(
        &db,
        &source,
        MemoryCommand::SetCrossProjectMemory { enabled: false },
    )
    .await;
    control(
        &db,
        &source,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    assert_eq!(state(&db, &task).await, "paused");
    // 当前项目 revision 未改也必须拒绝；不能只检查当前项目的记忆版本。
    assert_eq!(
        ContextBuilder::build(&db, &task.id).await.unwrap_err().code,
        "MEMORY_SCOPE_CHANGED"
    );
}

#[tokio::test]
async fn legacy_memory_read_without_text_version_does_not_invent_historical_wording() {
    let temp = tempfile::tempdir().unwrap();
    let (db, source, target) = setup(temp.path()).await;
    let h = confirmed(&db, &source, "留白").await;
    control(
        &db,
        &target,
        MemoryCommand::SetCrossProjectMemory { enabled: true },
    )
    .await;
    let task = start(&db, &target, "留白").await;
    sqlx::query("INSERT INTO agent_steps(task_id,kind,body_json,summary,created_at) VALUES(?,'model_attempt',?,'legacy fixture',?)")
        .bind(&task.id)
        .bind(json!({"memoryReads":[{"memoryId":h.id,"sourceProjectId":source.id,"scope":"cross_project","origin":"user_confirmed"}]}).to_string())
        .bind(domain::timestamp()).execute(&db.pool).await.unwrap();
    let reference = memory_reference(&db, &target, &task.id).await;
    assert_eq!(reference["memoryId"], h.id);
    assert_eq!(reference["memoryTextVersion"], "unrecorded");
    assert!(reference["passage"].is_null());
}
