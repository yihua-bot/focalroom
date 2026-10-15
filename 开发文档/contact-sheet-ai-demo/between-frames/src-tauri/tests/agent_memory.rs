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

#[tokio::test]
async fn v2_baseline_preserves_pre_keyword_tombstones() {
    for suffix in ["", "（2 条证据）"] {
        let temp = tempfile::tempdir().unwrap();
        let (db, p) = setup(temp.path(), 2).await;
        let p = note(&db, p, 0, "留白").await;
        let p = note(&db, p, 1, "留白").await;
        let view = memory::projection(&db, &p.id).await.unwrap();
        let h = view
            .hypotheses
            .iter()
            .find(|h| h.statement.contains("留白"))
            .unwrap();
        // 复现 0a5c71b 之前已删除、仅留下完整表述墓碑的用户数据库。
        sqlx::query("INSERT INTO memory_tombstones(project_id,kind,ref_key,created_at) VALUES(?,'hypothesis',?,'now')")
            .bind(&p.id).bind(format!("{}{suffix}", h.statement)).execute(&db.pool).await.unwrap();
        sqlx::query("DELETE FROM preference_hypotheses WHERE id=?")
            .bind(h.id)
            .execute(&db.pool)
            .await
            .unwrap();
        let after = memory::projection(&db, &p.id).await.unwrap();
        assert!(
            after
                .hypotheses
                .iter()
                .all(|h| !h.statement.contains("留白")),
            "旧格式墓碑仍须有效"
        );
    }
}

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
async fn correction_survives_growing_evidence_legacy_duplicates_and_restart() {
    let temp = tempfile::tempdir().unwrap();
    let (db, mut p) = setup(temp.path(), 2).await;
    let ids: Vec<_> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    for id in &ids {
        p = gui(
            &db,
            &p,
            p.revision,
            GuiCommand::SetPhotoNote {
                photo_id: id.clone(),
                note: "留白".into(),
            },
        )
        .await;
    }
    let first = memory::projection(&db, &p.id).await.unwrap();
    let h = first
        .hypotheses
        .iter()
        .find(|h| h.statement.contains("留白"))
        .unwrap();
    // 重现已发布版本的含次数身份，以及已经复活过的同义记录。
    let legacy = format!("{}（2 条证据）", h.statement);
    sqlx::query("UPDATE preference_hypotheses SET statement=? WHERE id=?")
        .bind(&legacy)
        .bind(h.id)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO preference_hypotheses(project_id,statement,evidence_ids_json,created_at,updated_at) VALUES(?,?,?,?,?)")
        .bind(&p.id).bind(format!("{}（3 条证据）", h.statement)).bind("[]").bind("now").bind("now").execute(&db.pool).await.unwrap();
    memory::correct(
        &db,
        memory::HypothesisCorrection {
            hypothesis_id: h.id,
            project_id: p.id.clone(),
            correction: "这不是我的偏好".into(),
        },
    )
    .await
    .unwrap();
    gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoNote {
            photo_id: ids[0].clone(),
            note: "留白的新说明".into(),
        },
    )
    .await;
    db.pool.close().await;
    let db = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let result = memory::retrievable(&mut tx, &p.id, 10).await.unwrap();
    tx.commit().await.unwrap();
    assert!(
        result["hypotheses"]
            .as_array()
            .unwrap()
            .iter()
            .all(|h| !h["statement"].as_str().unwrap().contains("留白"))
    );
    let projected = memory::projection(&db, &p.id).await.unwrap();
    assert_eq!(
        projected
            .hypotheses
            .iter()
            .filter(|h| h.statement.contains("留白"))
            .count(),
        2,
        "不得再产生第三个同义假设"
    );
    assert!(projected.hypotheses.iter().any(|item| item.id == h.id
        && item.status == "corrected"
        && item.user_correction.as_deref() == Some("这不是我的偏好")));
}

#[tokio::test]
async fn growing_support_updates_the_same_hypothesis_and_preserves_disabled_state() {
    let temp = tempfile::tempdir().unwrap();
    let (db, mut p) = setup(temp.path(), 2).await;
    let ids: Vec<_> = p.photos.iter().map(|photo| photo.id.clone()).collect();
    for id in &ids {
        p = gui(
            &db,
            &p,
            p.revision,
            GuiCommand::SetPhotoNote {
                photo_id: id.clone(),
                note: "留白".into(),
            },
        )
        .await;
    }
    let first = memory::projection(&db, &p.id).await.unwrap();
    let h = first
        .hypotheses
        .iter()
        .find(|h| h.statement.contains("留白"))
        .unwrap();
    memory::set_enabled(&db, &p.id, h.id, false).await.unwrap();
    gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoNote {
            photo_id: ids[0].clone(),
            note: "仍然留白".into(),
        },
    )
    .await;
    let next = memory::projection(&db, &p.id).await.unwrap();
    let matching: Vec<_> = next
        .hypotheses
        .iter()
        .filter(|h| h.statement.contains("留白"))
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].id, h.id);
    assert_eq!(matching[0].evidence_ids.len(), 3);
    assert!(matching[0].disabled);
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

use agent::memory_control::{MemoryCommand, MemoryRequest};
async fn control(
    db: &Store,
    p: &Project,
    command: MemoryCommand,
) -> agent::memory_control::MemoryReceipt {
    CommandHandler::new(db)
        .execute_memory(MemoryRequest {
            request_id: between_frames_lib::domain::id(),
            project_id: p.id.clone(),
            command,
        })
        .await
        .unwrap()
}
async fn note(db: &Store, p: Project, index: usize, text: &str) -> Project {
    let id = p.photos[index].id.clone();
    gui(
        db,
        &p,
        p.revision,
        GuiCommand::SetPhotoNote {
            photo_id: id,
            note: text.into(),
        },
    )
    .await
}
async fn retrieved(db: &Store, p: &Project, goal: &str, limit: usize) -> Value {
    let mut tx = db.pool.begin().await.unwrap();
    let value = memory::retrievable_for_goal(&mut tx, &p.id, limit, goal)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    value
}
#[tokio::test]
async fn controls_are_idempotent_reversible_and_leave_formal_edit_history_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let p = note(&db, p, 0, "留白").await;
    let p = note(&db, p, 1, "留白").await;
    let initial = memory::projection(&db, &p.id).await.unwrap();
    let h = &initial.hypotheses[0];
    let formal = serde_json::to_value(&p).unwrap();
    let request = MemoryRequest {
        request_id: "confirm-once".into(),
        project_id: p.id.clone(),
        command: MemoryCommand::ConfirmHypothesis {
            hypothesis_id: h.id,
        },
    };
    let receipt = CommandHandler::new(&db)
        .execute_memory(request.clone())
        .await
        .unwrap();
    assert!(receipt.changed);
    assert_eq!(receipt.memory_revision, 1);
    control(
        &db,
        &p,
        MemoryCommand::SetHypothesisEnabled {
            hypothesis_id: h.id,
            enabled: false,
        },
    )
    .await;
    let replay = CommandHandler::new(&db)
        .execute_memory(request.clone())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(replay).unwrap(),
        serde_json::to_value(&receipt).unwrap()
    );
    let mut conflict = request;
    conflict.command = MemoryCommand::RetireHypothesis {
        hypothesis_id: h.id,
    };
    assert_eq!(
        CommandHandler::new(&db)
            .execute_memory(conflict)
            .await
            .unwrap_err()
            .code,
        "IDEMPOTENCY_CONFLICT"
    );
    control(
        &db,
        &p,
        MemoryCommand::SetHypothesisEnabled {
            hypothesis_id: h.id,
            enabled: true,
        },
    )
    .await;
    control(
        &db,
        &p,
        MemoryCommand::RetireHypothesis {
            hypothesis_id: h.id,
        },
    )
    .await;
    assert!(
        retrieved(&db, &p, "留白", 10).await["hypotheses"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    control(
        &db,
        &p,
        MemoryCommand::RestoreHypothesis {
            hypothesis_id: h.id,
        },
    )
    .await;
    let restored = memory::projection(&db, &p.id).await.unwrap();
    assert_eq!(restored.hypotheses[0].status, "unconfirmed");
    assert_eq!(restored.hypotheses[0].id, h.id);
    control(
        &db,
        &p,
        MemoryCommand::CorrectHypothesis {
            hypothesis_id: h.id,
            correction: "只是这两张的特点".into(),
        },
    )
    .await;
    control(
        &db,
        &p,
        MemoryCommand::RetireHypothesis {
            hypothesis_id: h.id,
        },
    )
    .await;
    control(
        &db,
        &p,
        MemoryCommand::RestoreHypothesis {
            hypothesis_id: h.id,
        },
    )
    .await;
    assert_eq!(
        memory::projection(&db, &p.id).await.unwrap().hypotheses[0].status,
        "corrected"
    );
    assert_eq!(
        serde_json::to_value(db.get(&p.id).await.unwrap()).unwrap(),
        formal
    );
    let no_op = control(
        &db,
        &p,
        MemoryCommand::SetHypothesisEnabled {
            hypothesis_id: h.id,
            enabled: false,
        },
    )
    .await;
    assert!(!no_op.changed);
    db.pool.close().await;
    let db = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    assert!(
        retrieved(&db, &p, "留白", 10).await["hypotheses"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn excluded_evidence_updates_support_counters_retrieval_and_survives_restart() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 3).await;
    let p = note(&db, p, 0, "留白").await;
    let p = note(&db, p, 1, "留白").await;
    let id = p.photos[2].id.clone();
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![id],
            status: "reject".into(),
        },
    )
    .await;
    let p = note(&db, p, 2, "留白太多").await;
    let first = memory::projection(&db, &p.id).await.unwrap();
    let h = first
        .hypotheses
        .iter()
        .find(|h| h.statement.contains("留白"))
        .unwrap();
    assert_eq!(h.evidence_ids.len(), 2);
    assert_eq!(h.counterexample_ids.len(), 1);
    let support = h.evidence_ids[0];
    let counter = h.counterexample_ids[0];
    control(
        &db,
        &p,
        MemoryCommand::SetEvidenceExcluded {
            evidence_id: support,
            excluded: true,
        },
    )
    .await;
    let excluded = memory::projection(&db, &p.id).await.unwrap();
    assert_eq!(excluded.hypotheses[0].evidence_ids.len(), 1);
    assert!(
        excluded
            .evidence
            .iter()
            .find(|e| e.id == support)
            .unwrap()
            .excluded
    );
    assert!(
        retrieved(&db, &p, "留白", 10).await["hypotheses"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    control(
        &db,
        &p,
        MemoryCommand::SetEvidenceExcluded {
            evidence_id: counter,
            excluded: true,
        },
    )
    .await;
    assert!(
        memory::projection(&db, &p.id).await.unwrap().hypotheses[0]
            .counterexample_ids
            .is_empty()
    );
    control(
        &db,
        &p,
        MemoryCommand::SetEvidenceExcluded {
            evidence_id: support,
            excluded: false,
        },
    )
    .await;
    assert_eq!(
        retrieved(&db, &p, "留白", 10).await["hypotheses"][0]["statement"],
        h.statement
    );
    let snapshot = memory::projection(&db, &p.id).await.unwrap();
    assert_eq!(snapshot.evidence.len(), first.evidence.len());
    assert_eq!(
        snapshot
            .evidence
            .iter()
            .find(|e| e.id == counter)
            .unwrap()
            .detail,
        first
            .evidence
            .iter()
            .find(|e| e.id == counter)
            .unwrap()
            .detail
    );
    db.pool.close().await;
    let db = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(memory::projection(&db, &p.id).await.unwrap()).unwrap(),
        serde_json::to_value(snapshot).unwrap()
    );
}

#[tokio::test]
async fn batch_decisions_keep_all_participants_and_do_not_invent_past_explanations() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let ids: Vec<_> = p.photos.iter().map(|p| p.id.clone()).collect();
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoDecision {
            photo_ids: ids.clone(),
            status: "keep".into(),
        },
    )
    .await;
    let p = note(&db, p, 0, "留白").await;
    let p = note(&db, p, 1, "留白").await;
    let before = memory::projection(&db, &p.id).await.unwrap();
    let keep = before.evidence.iter().find(|e| e.kind == "keep").unwrap();
    assert_eq!(keep.detail["photo_ids"], json!(ids));
    assert_eq!(keep.detail["notes"], json!([]));
    assert_eq!(
        before.hypotheses[0].evidence_ids.len(),
        2,
        "后来的备注不得成为旧选择的理由"
    );
    let p = gui(
        &db,
        &p,
        p.revision,
        GuiCommand::SetPhotoDecision {
            photo_ids: ids.clone(),
            status: "reject".into(),
        },
    )
    .await;
    let after = memory::projection(&db, &p.id).await.unwrap();
    let reject = after.evidence.iter().find(|e| e.kind == "reject").unwrap();
    assert_eq!(reject.detail["notes"].as_array().unwrap().len(), 2);
    assert_eq!(reject.detail["photo_ids"], json!(ids));
    assert_eq!(
        after.hypotheses[0].counterexample_ids,
        vec![reject.id],
        "批量操作按一条因果证据计数"
    );
}

#[tokio::test]
async fn fts_search_is_scoped_and_goal_relevance_precedes_confirmation() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let p = note(&db, p, 0, "留白 逆光").await;
    let p = note(&db, p, 1, "留白 逆光").await;
    let all = memory::projection(&db, &p.id).await.unwrap();
    let h = all
        .hypotheses
        .iter()
        .find(|h| h.statement.contains("留白"))
        .unwrap();
    control(
        &db,
        &p,
        MemoryCommand::ConfirmHypothesis {
            hypothesis_id: h.id,
        },
    )
    .await;

    sqlx::query("INSERT INTO preference_hypotheses(project_id,scope,statement,evidence_ids_json,created_at,updated_at) VALUES(?,'cross_project','跨项目「逆光」',?,'now','now')")
        .bind(&p.id).bind(serde_json::to_string(&h.evidence_ids).unwrap()).execute(&db.pool).await.unwrap();
    let cross_id: i64 = sqlx::query_scalar(
        "SELECT id FROM preference_hypotheses WHERE project_id=? AND scope='cross_project'",
    )
    .bind(&p.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        CommandHandler::new(&db)
            .execute_memory(MemoryRequest {
                request_id: between_frames_lib::domain::id(),
                project_id: p.id.clone(),
                command: MemoryCommand::ConfirmHypothesis {
                    hypothesis_id: cross_id
                }
            })
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    let q = memory::projection_for_query(&db, &p.id, "逆光")
        .await
        .unwrap();
    assert_eq!(q.hypotheses.len(), 1);
    assert!(q.hypotheses[0].statement.contains("逆光"));
    assert!(
        retrieved(&db, &p, "更偏向逆光", 1).await["hypotheses"][0]["statement"]
            .as_str()
            .unwrap()
            .contains("逆光")
    );
    assert!(
        memory::projection_for_query(&db, &p.id, "\" OR * ()",)
            .await
            .unwrap()
            .hypotheses
            .is_empty()
    );
    assert!(
        memory::projection_for_query(&db, &p.id, &"光".repeat(401))
            .await
            .is_err()
    );
    let other = db.create("另一个项目", "").await.unwrap();
    assert!(
        memory::projection_for_query(&db, &other.id, "逆光")
            .await
            .unwrap()
            .hypotheses
            .is_empty()
    );
}

#[tokio::test]
async fn memory_scope_schema_and_receipt_fault_rollback_are_enforced() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let p = note(&db, p, 0, "留白").await;
    let p = note(&db, p, 1, "留白").await;
    let initial = memory::projection(&db, &p.id).await.unwrap();
    let other = db.create("其他", "").await.unwrap();
    let request = |project: &Project, command| MemoryRequest {
        request_id: between_frames_lib::domain::id(),
        project_id: project.id.clone(),
        command,
    };
    for command in [
        MemoryCommand::SetEvidenceExcluded {
            evidence_id: initial.evidence[0].id,
            excluded: true,
        },
        MemoryCommand::ConfirmHypothesis {
            hypothesis_id: initial.hypotheses[0].id,
        },
        MemoryCommand::RetireHypothesis {
            hypothesis_id: initial.hypotheses[0].id,
        },
        MemoryCommand::RestoreHypothesis {
            hypothesis_id: initial.hypotheses[0].id,
        },
        MemoryCommand::CorrectHypothesis {
            hypothesis_id: initial.hypotheses[0].id,
            correction: "纠正".into(),
        },
        MemoryCommand::SetHypothesisEnabled {
            hypothesis_id: initial.hypotheses[0].id,
            enabled: false,
        },
    ] {
        assert_eq!(
            CommandHandler::new(&db)
                .execute_memory(request(&other, command))
                .await
                .unwrap_err()
                .code,
            "NOT_FOUND"
        );
    }
    assert_eq!(
        CommandHandler::new(&db)
            .execute_memory(request(
                &p,
                MemoryCommand::ConfirmHypothesis { hypothesis_id: 0 }
            ))
            .await
            .unwrap_err()
            .code,
        "VALIDATION_ERROR"
    );
    assert!(serde_json::from_value::<MemoryRequest>(json!({"requestId":"r","projectId":p.id,"actor":"human","command":{"type":"RetireHypothesis","hypothesisId":1}})).is_err());
    sqlx::query("CREATE TRIGGER fail_memory_receipt BEFORE INSERT ON memory_commands BEGIN SELECT RAISE(ABORT,'receipt-fault'); END").execute(&db.pool).await.unwrap();
    assert!(
        CommandHandler::new(&db)
            .execute_memory(request(
                &p,
                MemoryCommand::SetEvidenceExcluded {
                    evidence_id: initial.evidence[0].id,
                    excluded: true
                }
            ))
            .await
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(memory::projection(&db, &p.id).await.unwrap()).unwrap(),
        serde_json::to_value(initial).unwrap()
    );
}

#[tokio::test]
async fn memory_changes_pause_tasks_and_require_new_context_instead_of_old_history() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let p = note(&db, p, 0, "留白").await;
    let p = note(&db, p, 1, "留白").await;
    let projection = memory::projection(&db, &p.id).await.unwrap();
    let start = || AgentRequest {
        request_id: between_frames_lib::domain::id(),
        project_id: p.id.clone(),
        command: TaskCommand::StartTask {
            goal: "留白".into(),
            selected_photo_ids: vec![],
            target_count: 2,
            budget: Budget::default(),
            consent_id: None,
        },
    };
    let task = CommandHandler::new(&db)
        .agent_request(start())
        .await
        .unwrap();
    assert_eq!(task.memory_revision, 0);
    control(
        &db,
        &p,
        MemoryCommand::SetEvidenceExcluded {
            evidence_id: projection.evidence[0].id,
            excluded: true,
        },
    )
    .await;
    let state = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(state.tasks[0].state, "paused");
    assert!(
        agent::context::ContextBuilder::build(&db, &task.id)
            .await
            .unwrap_err()
            .message
            .contains("记忆")
    );
    for command in [
        TaskCommand::ResumeTask {
            task_id: task.id.clone(),
            budget: Budget::default(),
        },
        TaskCommand::SubmitFeedback {
            task_id: task.id.clone(),
            answer: "继续".into(),
        },
    ] {
        let result = CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: between_frames_lib::domain::id(),
                project_id: p.id.clone(),
                command,
            })
            .await
            .unwrap_err();
        assert_eq!(result.code, "REVISION_CONFLICT");
    }
    let next = CommandHandler::new(&db)
        .agent_request(start())
        .await
        .unwrap();
    assert_eq!(next.memory_revision, 1);
    let context = agent::context::ContextBuilder::build(&db, &next.id)
        .await
        .unwrap();
    assert!(
        !context
            .layers
            .to_string()
            .contains(&projection.hypotheses[0].statement)
    );

    sqlx::query("UPDATE agent_checkpoints SET context_json=json_set(context_json,'$.memoryRevision',0) WHERE task_id=?")
        .bind(&next.id).execute(&db.pool).await.unwrap();
    assert_eq!(
        agent::context::ContextBuilder::build(&db, &next.id)
            .await
            .unwrap_err()
            .code,
        "RECOVERY_ERROR",
        "检查点不得冒用旧记忆版本"
    );
    assert_eq!(db.get(&p.id).await.unwrap().revision, p.revision);
}

#[tokio::test]
async fn version_eight_upgrade_backs_up_committed_wal_before_new_schema() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("old.sqlite");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal),
        )
        .await
        .unwrap();
    let mut migrations = sqlx::migrate!("./migrations");
    migrations.migrations = std::borrow::Cow::Owned(
        migrations
            .iter()
            .filter(|m| m.version <= 8)
            .cloned()
            .collect(),
    );
    migrations.run(&pool).await.unwrap();
    sqlx::query("PRAGMA wal_autocheckpoint=0")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO projects(id,title,brief,created_at,updated_at,revision) VALUES('wal-project','已提交 WAL','原样保存','now','now',25)").execute(&pool).await.unwrap();
    let upgraded = Store::open(&path).await.unwrap();
    assert_eq!(upgraded.get("wal-project").await.unwrap().revision, 25);
    let backups: Vec<_> = std::fs::read_dir(temp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("backup-before-v8"))
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(backups[0].path())
            .read_only(true),
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_one(&backup)
            .await
            .unwrap(),
        8
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT brief FROM projects WHERE id='wal-project'")
            .fetch_one(&backup)
            .await
            .unwrap(),
        "原样保存"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='memory_commands'"
        )
        .fetch_one(&backup)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM memory_commands")
            .fetch_one(&upgraded.pool)
            .await
            .unwrap(),
        0
    );
    upgraded.pool.close().await;
    let reopened = Store::open(&path).await.unwrap();
    assert_eq!(reopened.get("wal-project").await.unwrap().revision, 25);
    assert_eq!(
        std::fs::read_dir(temp.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("backup-before-v8"))
            .count(),
        1
    );
}

#[tokio::test]
async fn memory_exclusion_discards_a_model_reply_already_in_flight() {
    use std::{future::Future, pin::Pin, sync::Arc};
    use tokio::sync::Notify;
    struct WaitingProvider {
        entered: Arc<Notify>,
        release: Arc<Notify>,
    }
    impl agent::provider::ModelProvider for WaitingProvider {
        fn generate<'a>(
            &'a self,
            input: &'a agent::context::ModelTurnRequest,
        ) -> Pin<
            Box<dyn Future<Output = between_frames_lib::domain::Result<ModelOutcome>> + Send + 'a>,
        > {
            Box::pin(async move {
                self.entered.notify_one();
                self.release.notified().await;
                Ok(ModelOutcome::ToolCalls {
                    calls: vec![ToolCall {
                        call_id: "late-memory-call".into(),
                        name: "create_selection_draft".into(),
                        args: json!({"project_id":input.task.project_id,"title":"旧记忆草案","summary":"晚到结果","items":[{"photo_id":input.task.allowed_photo_ids[0],"proposed_action":"keep","position":0,"rationale":"旧偏好"}]}),
                    }],
                })
            })
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 2).await;
    let p = note(&db, p, 0, "留白").await;
    let p = note(&db, p, 1, "留白").await;
    let evidence = memory::projection(&db, &p.id).await.unwrap().evidence[0].id;
    let task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: between_frames_lib::domain::id(),
            project_id: p.id.clone(),
            command: TaskCommand::StartTask {
                goal: "留白".into(),
                selected_photo_ids: vec![],
                target_count: 2,
                budget: Budget::default(),
                consent_id: None,
            },
        })
        .await
        .unwrap();
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
    control(
        &db,
        &p,
        MemoryCommand::SetEvidenceExcluded {
            evidence_id: evidence,
            excluded: true,
        },
    )
    .await;
    release.notify_one();
    let late = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(late.code, "CANCELLED");
    let result = agent::projection(&db, &p.id).await.unwrap();
    assert_eq!(result.tasks[0].state, "paused");
    assert!(
        result.drafts.is_empty(),
        "已排除记忆的晚到模型结果不得写草案"
    );
    assert_eq!(db.get(&p.id).await.unwrap().revision, p.revision);
}

#[tokio::test]
async fn physical_delete_hypothesis_writes_tombstone_and_never_revives() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 3).await;
    // 两张照片同关键词备注 → 物化假设。
    let p = note(&db, p, 0, "逆光有层次").await;
    let p = note(&db, p, 1, "逆光更克制").await;
    let view = memory::projection(&db, &p.id).await.unwrap();
    let hypothesis = view
        .hypotheses
        .iter()
        .find(|h| h.statement.contains("逆光"))
        .expect("应有假设")
        .id;
    // 物理删除。
    let receipt = control(
        &db,
        &p,
        MemoryCommand::DeleteHypothesis {
            hypothesis_id: hypothesis,
        },
    )
    .await;
    assert!(receipt.changed);
    let after = memory::projection(&db, &p.id).await.unwrap();
    assert!(after.hypotheses.iter().all(|h| h.id != hypothesis));
    // 再触发物化（新证据写入 + 投影）不复活：同表述候选被墓碑拦截。
    let p2 = note(&db, p.clone(), 2, "逆光也想要").await;
    let _ = p2;
    let again = memory::projection(&db, &p.id).await.unwrap();
    assert!(
        again.hypotheses.iter().all(|h| h.statement
            != view
                .hypotheses
                .iter()
                .find(|h| h.id == hypothesis)
                .unwrap()
                .statement),
        "物理删除的假设不得经物化复活"
    );
    // 行已不存在：明确 NOT_FOUND；同请求重放的幂等由 request_id 回执机制保障。
    let replay = CommandHandler::new(&db)
        .execute_memory(MemoryRequest {
            request_id: between_frames_lib::domain::id(),
            project_id: p.id.clone(),
            command: MemoryCommand::DeleteHypothesis {
                hypothesis_id: hypothesis,
            },
        })
        .await;
    assert!(replay.is_err());
}

#[tokio::test]
async fn delete_evidence_unlinks_hypotheses_and_purges_cache() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 3).await;
    let p = note(&db, p, 0, "窗边的光线有层次").await;
    let p = note(&db, p, 1, "窗边也留白").await;
    let view = memory::projection(&db, &p.id).await.unwrap();
    // 先造观察缓存：直接落两行（不经视觉链路）。
    for photo in &p.photos[..2] {
        sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,created_at) VALUES(?,?,?,?,?,?,?)")
            .bind(&photo.id).bind("hash-x").bind("mock").bind("m3-vision-1").bind("inspect_photos").bind("{}").bind(between_frames_lib::domain::timestamp())
            .execute(&db.pool).await.unwrap();
    }
    // 删除一条证据：引用它的假设要么失去引用、要么退役。
    let evidence_id = view.evidence[0].id;
    let linked: Vec<i64> = view
        .hypotheses
        .iter()
        .flat_map(|h| h.evidence_ids.clone())
        .collect();
    let receipt = control(&db, &p, MemoryCommand::DeleteEvidence { evidence_id }).await;
    assert!(receipt.changed);
    let after = memory::projection(&db, &p.id).await.unwrap();
    assert!(after.evidence.iter().all(|e| e.id != evidence_id));
    for hypothesis in &after.hypotheses {
        assert!(!hypothesis.evidence_ids.contains(&evidence_id));
        if hypothesis.evidence_ids.is_empty() {
            assert_eq!(hypothesis.status, "retired");
        }
    }
    let _ = linked;
    // 清空观察缓存。
    let purged = control(&db, &p, MemoryCommand::PurgeObservationCache {}).await;
    assert!(purged.changed);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photo_observations WHERE photo_id IN (SELECT id FROM photos WHERE project_id=?)")
        .bind(&p.id).fetch_one(&db.pool).await.unwrap();
    assert_eq!(count, 0);
    // 再次清空为 no-op（幂等）。
    let again = control(&db, &p, MemoryCommand::PurgeObservationCache {}).await;
    assert!(!again.changed);
}

#[tokio::test]
async fn deleted_hypothesis_stays_dead_when_evidence_count_changes() {
    let temp = tempfile::tempdir().unwrap();
    let (db, p) = setup(temp.path(), 3).await;
    // 两条同词证据 → 假设「（2 条证据）」→ 删除（墓碑按关键词）。
    let p = note(&db, p, 0, "窗边逆光有层次").await;
    let p = note(&db, p, 1, "逆光更克制").await;
    let view = memory::projection(&db, &p.id).await.unwrap();
    let hypothesis = view
        .hypotheses
        .iter()
        .find(|h| h.statement.contains("逆光"))
        .expect("应有假设")
        .id;
    control(
        &db,
        &p,
        MemoryCommand::DeleteHypothesis {
            hypothesis_id: hypothesis,
        },
    )
    .await;
    // 第三条同词证据加入：候选表述证据数变为 3——完整表述匹配会漏（评审问题 1），
    // 关键词墓碑必须仍然拦截。
    let _p2 = note(&db, p.clone(), 2, "这张也是逆光").await;
    let again = memory::projection(&db, &p.id).await.unwrap();
    assert!(
        again
            .hypotheses
            .iter()
            .all(|h| !h.statement.contains("逆光")),
        "证据数漂移后删除的假设不得复活：{:?}",
        again
            .hypotheses
            .iter()
            .map(|h| h.statement.clone())
            .collect::<Vec<_>>()
    );
}
