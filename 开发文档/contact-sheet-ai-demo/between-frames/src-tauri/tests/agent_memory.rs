// M4 记忆引擎集成测试：L2 证据因果去重、L3 假设确定性物化、
// AG-11 纠正后不复发、Agent 草案不计证据、检索排序与跨项目拒绝。
use between_frames_lib::{
    agent::{self, memory, runtime::AgentEngine, types::*},
    command_handler::{CommandHandler, GuiCommandRequest},
    commands_generated::GuiCommand,
    db::Store,
    domain::Project,
    image_pipeline,
};
use serde_json::{Value, json};
use std::path::Path;

async fn setup(path: &Path, count: usize) -> (Store, Project) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let p = db.create("M4", "偏好与证据").await.unwrap();
    for i in 0..count {
        let source = path.join(format!("f{i}.jpg"));
        image::RgbImage::from_pixel(32, 24, image::Rgb([i as u8, 42, 90]))
            .save(&source)
            .unwrap();
        db.insert_photo(
            &p.id,
            &image_pipeline::prepare(&source, &path.join("cache")).unwrap(),
        )
        .await
        .unwrap();
    }
    let p = db.get(&p.id).await.unwrap();
    (db, p)
}
async fn gui(db: &Store, p: &Project, revision: i64, command: GuiCommand) -> Project {
    CommandHandler::new(db)
        .execute_gui(GuiCommandRequest::new(&p.id, revision, command))
        .await
        .unwrap();
    db.get(&p.id).await.unwrap()
}
fn evidence_count(memory: &memory::MemoryProjection, kind: &str) -> usize {
    memory.evidence.iter().filter(|e| e.kind == kind).count()
}

#[tokio::test]
async fn evidence_is_deduplicated_and_hypotheses_are_deterministic() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 3).await;
    let ids: Vec<String> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    // 两次人工保留 + 同关键词备注（「留白」出现于两条不同照片备注）。
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![ids[0].clone()],
            status: "keep".into(),
        },
    )
    .await;
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoNote {
            photo_id: ids[0].clone(),
            note: "喜欢留白的构图".into(),
        },
    )
    .await;
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![ids[1].clone()],
            status: "keep".into(),
        },
    )
    .await;
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoNote {
            photo_id: ids[1].clone(),
            note: "这张也留白，更克制".into(),
        },
    )
    .await;
    let projection = memory::projection(&db, &p.id).await.unwrap();
    assert_eq!(evidence_count(&projection, "keep"), 2);
    assert_eq!(evidence_count(&projection, "note"), 2);
    // 确定性假设：「留白」2-gram 至少被两条证据支持。
    assert!(
        projection
            .hypotheses
            .iter()
            .any(|h| h.statement.contains('「')
                && h.statement.contains("留白")
                && h.evidence_ids.len() >= 2),
        "假设：{:?}",
        projection
            .hypotheses
            .iter()
            .map(|h| &h.statement)
            .collect::<Vec<_>>()
    );
    // 同命令幂等重放不增加证据（dedup_key=command_id）。
    let command_id = between_frames_lib::domain::id();
    let make = || GuiCommandRequest {
        command_id: command_id.clone(),
        project_id: p.id.clone(),
        expected_project_revision: p.revision,
        causation_id: Some(command_id.clone()),
        idempotency_key: command_id.clone(),
        command: GuiCommand::SetPhotoDecision {
            photo_ids: vec![ids[2].clone()],
            status: "reject".into(),
        },
    };
    let first = CommandHandler::new(&db).execute_gui(make()).await.unwrap();
    assert!(first.changed);
    // 幂等重放返回首次回执快照；去重语义由下方证据计数断言承担。
    let replayed = CommandHandler::new(&db).execute_gui(make()).await.unwrap();
    assert_eq!(replayed.request_id, first.request_id);
    let projection = memory::projection(&db, &p.id).await.unwrap();
    assert_eq!(evidence_count(&projection, "reject"), 1);
}

#[tokio::test]
async fn ag11_correction_stops_retrieval_and_does_not_revive() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let ids: Vec<String> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![ids[0].clone()],
            status: "keep".into(),
        },
    )
    .await;
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoNote {
            photo_id: ids[0].clone(),
            note: "失焦有种时间感".into(),
        },
    )
    .await;
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![ids[1].clone()],
            status: "keep".into(),
        },
    )
    .await;
    gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoNote {
            photo_id: ids[1].clone(),
            note: "失焦但克制".into(),
        },
    )
    .await;
    let projection = memory::projection(&db, &p.id).await.unwrap();
    let hypothesis = projection
        .hypotheses
        .iter()
        .find(|h| h.statement.contains("失焦"))
        .expect("应生成「失焦」假设");
    // 用户纠正：不同意把失焦当偏好。
    memory::correct(
        &db,
        memory::HypothesisCorrection {
            hypothesis_id: hypothesis.id,
            project_id: p.id.clone(),
            correction: "失焦只是这两张的巧合，不是我的偏好".into(),
        },
    )
    .await
    .unwrap();
    // 纠正后：检索不再包含；再次物化不复活（AG-11）。
    let mut tx = db.pool.begin().await.unwrap();
    let retrieved = memory::retrievable(&mut tx, &p.id, 10).await.unwrap();
    tx.commit().await.unwrap();
    let list = retrieved["hypotheses"].as_array().unwrap();
    assert!(
        list.iter()
            .all(|h| h["statement"].as_str().unwrap() != hypothesis.statement)
    );
    let projection = memory::projection(&db, &p.id).await.unwrap();
    let corrected = projection
        .hypotheses
        .iter()
        .find(|h| h.id == hypothesis.id)
        .unwrap();
    assert_eq!(corrected.status, "corrected");
    assert!(corrected.disabled);
    assert_eq!(
        corrected.user_correction.as_deref(),
        Some("失焦只是这两张的巧合，不是我的偏好")
    );
    let same_count = projection
        .hypotheses
        .iter()
        .filter(|h| h.statement == hypothesis.statement && h.id != hypothesis.id)
        .count();
    assert_eq!(same_count, 0, "纠正后的假设不得复活");
}

#[tokio::test]
async fn agent_drafts_are_not_evidence_until_human_applies() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 3).await;
    let ids: Vec<String> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    // Agent 全流程跑完两份草案（Agent 只写草案，不产生证据）。
    let task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: between_frames_lib::domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::StartTask {
                goal: "给两版 2 张序列".into(),
                selected_photo_ids: ids.clone(),
                target_count: 2,
                budget: Budget::default(),
                consent_id: None,
            },
        })
        .await
        .unwrap();
    AgentEngine::new(db.clone()).run(&task.id).await.unwrap();
    let before = memory::projection(&db, &p.id).await.unwrap();
    assert!(before.evidence.is_empty(), "Agent 草案不得构成偏好证据");
    // 人工采纳草案后产生 draft_apply 证据。
    let drafts = agent::projection(&db, &p.id).await.unwrap().drafts;
    let draft = &drafts[0];
    let applied = CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &p.id,
            p.revision,
            GuiCommand::ApplyDraft {
                draft_id: draft.id.clone(),
                approved_photo_ids: draft.items.iter().map(|i| i.photo_id.clone()).collect(),
                mode: "full".into(),
                expected_draft_revision: 1,
            },
        ))
        .await
        .unwrap()
        .project
        .unwrap();
    let after = memory::projection(&db, &applied.id).await.unwrap();
    assert_eq!(evidence_count(&after, "draft_apply"), 1);
}

#[tokio::test]
async fn retrieve_preferences_tool_returns_evidence_backed_and_rejects_cross_project() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let ids: Vec<String> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![ids[0].clone()],
            status: "keep".into(),
        },
    )
    .await;
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoNote {
            photo_id: ids[0].clone(),
            note: "逆光有层次".into(),
        },
    )
    .await;
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![ids[1].clone()],
            status: "keep".into(),
        },
    )
    .await;
    gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoNote {
            photo_id: ids[1].clone(),
            note: "逆光更克制".into(),
        },
    )
    .await;
    let p = db.get(&p.id).await.unwrap();
    // 脚本化 provider：先项目内检索（应带证据假设），再跨项目（应被拒），然后结束。
    struct RetrieveScript;
    impl between_frames_lib::agent::provider::ModelProvider for RetrieveScript {
        fn generate<'a>(
            &'a self,
            input: &'a between_frames_lib::agent::context::ModelTurnRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = between_frames_lib::domain::Result<ModelOutcome>>
                    + Send
                    + 'a,
            >,
        > {
            Box::pin(async move {
                let seen = input.protocol.iter().filter(|m| m.kind == "tool").count();
                let call = |suffix: &str, name: &str, args: Value| ModelOutcome::ToolCalls {
                    calls: vec![ToolCall {
                        call_id: format!("mem-{suffix}"),
                        name: name.into(),
                        args,
                    }],
                };
                Ok(if seen == 0 {
                    call(
                        "local",
                        "retrieve_preferences",
                        json!({"project_id": input.task.project_id, "scope": "project_local", "limit": 5}),
                    )
                } else if seen == 1 {
                    call(
                        "cross",
                        "retrieve_preferences",
                        json!({"project_id": input.task.project_id, "scope": "cross_project", "limit": 5}),
                    )
                } else {
                    ModelOutcome::NoAction {
                        reason: "检索完成".into(),
                    }
                })
            })
        }
    }
    let task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: between_frames_lib::domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::StartTask {
                goal: "检索偏好".into(),
                selected_photo_ids: ids.clone(),
                target_count: 2,
                budget: Budget::default(),
                consent_id: None,
            },
        })
        .await
        .unwrap();
    AgentEngine::with_provider(db.clone(), std::sync::Arc::new(RetrieveScript))
        .run(&task.id)
        .await
        .unwrap();
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT tool_name, result_json FROM tool_calls WHERE task_id=? ORDER BY rowid",
    )
    .bind(&task.id)
    .fetch_all(&db.pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    let first: Value = serde_json::from_str(&rows[0].1).unwrap();
    assert!(first["ok"].as_bool().unwrap());
    assert_eq!(first["result"]["mode"], "evidence_backed_project_local");
    assert!(
        first["result"]["hypotheses"]
            .as_array()
            .is_some_and(|list| list
                .iter()
                .any(|h| h["statement"].as_str().unwrap().contains("逆光")))
    );
    let second: Value = serde_json::from_str(&rows[1].1).unwrap();
    assert!(!second["ok"].as_bool().unwrap());
    // 跨项目 scope 在工具 schema 白名单层即被拒（契约只允许 project_local）。
    assert_eq!(second["error"]["code"], "VALIDATION_ERROR");
    // 撤销不产生新证据：undo 最后一条命令后证据数不变。
    let before_undo = memory::projection(&db, &p.id).await.unwrap().evidence.len();
    let head: (String,) = sqlx::query_as(
        "SELECT h.group_id FROM command_history h JOIN history_heads c ON c.project_id=h.project_id AND c.cursor=h.position WHERE h.project_id=? AND h.active=1",
    )
    .bind(&p.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let undo: i64 = sqlx::query_scalar("SELECT revision FROM projects WHERE id=?")
        .bind(&p.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let _ = undo;
    let latest = db.get(&p.id).await.unwrap();
    CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &latest.id,
            latest.revision,
            GuiCommand::UndoCommandGroup {
                transaction_group_id: head.0,
            },
        ))
        .await
        .unwrap();
    let after_undo = memory::projection(&db, &latest.id)
        .await
        .unwrap()
        .evidence
        .len();
    assert_eq!(before_undo, after_undo, "撤销不增减人工证据");
}
