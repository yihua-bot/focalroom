// v2.0 M5 评论切片：人工写入（桥）、同锚点聚合、resolve 重开、跨项目锚点拒绝、Agent 只读。
use between_frames_lib::{
    agent::{runtime::AgentEngine, types::*},
    command_handler::CommandHandler,
    db::Store,
    image_pipeline,
};
use betweenframes_local_domain::parse_command;
use serde_json::{Value, json};
use std::{path::Path, sync::Arc};

async fn setup(path: &Path, count: usize) -> (Store, String, Vec<String>) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let project = db.create("评论", "").await.unwrap();
    let mut ids = vec![];
    for i in 0..count {
        let source = path.join(format!("c{i}.png"));
        image::RgbImage::from_pixel(32, 24, image::Rgb([(i * 53 % 256) as u8, 70, 110]))
            .save(&source)
            .unwrap();
        let prepared = image_pipeline::prepare(&source, &path.join("cache")).unwrap();
        db.insert_photo(&project.id, &prepared).await.unwrap();
        ids.push(prepared.hash);
    }
    let project = db.get(&project.id).await.unwrap();
    (
        db,
        project.id,
        project.photos.iter().map(|p| p.id.clone()).collect(),
    )
}

async fn bridge_run(db: &Store, command: Value) -> Value {
    let principal = between_frames_lib::v2_bridge::local_owner_principal(
        "local-workspace",
        command["project_id"].as_str().unwrap(),
    );
    let authority =
        between_frames_lib::v2_bridge::authority_for(db, command["project_id"].as_str().unwrap())
            .await
            .unwrap();
    let envelope = parse_command(command).expect("契约合法");
    between_frames_lib::v2_bridge::execute(
        db,
        between_frames_lib::v2_bridge::V2Execution {
            envelope,
            principal,
            authority,
        },
        1_900_000_000_000,
    )
    .await
}

fn comment_envelope(project: &str, revision: i64, body: &str, photo: &str, key: &str) -> Value {
    json!({
        "version": "2.0", "command_id": format!("cmd-{key}"), "workspace_id": "local-workspace",
        "project_id": project, "expected_revision": revision, "idempotency_key": format!("idem-{key}"),
        "kind": "comment.add",
        "payload": {"body": body, "anchor": {"type": "photo", "photo_id": photo, "x": 0.5, "y": 0.5}}
    })
}

#[tokio::test]
async fn comments_thread_aggregate_resolve_and_agent_read() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project, photos) = setup(temp.path(), 2).await;
    let revision = db.get(&project).await.unwrap().revision;
    // 同锚点两条评论聚合进同一线程。
    let first = bridge_run(
        &db,
        comment_envelope(&project, revision, "左图更克制", &photos[0], "cmt-first-01"),
    )
    .await;
    assert!(first["ok"].as_bool().unwrap(), "{first}");
    let thread_id = first["thread_id"].as_str().unwrap().to_owned();
    let second = bridge_run(
        &db,
        comment_envelope(
            &project,
            revision,
            "同意，光也更好",
            &photos[0],
            "cmt-second-2",
        ),
    )
    .await;
    assert_eq!(
        second["thread_id"].as_str().unwrap(),
        thread_id,
        "同锚点聚合"
    );
    // resolve → 关闭；再发同锚点 → 新线程。
    let mut resolve_cmd = json!({
        "version": "2.0", "command_id": "cmd-resolve-001", "workspace_id": "local-workspace",
        "project_id": project, "expected_revision": revision, "idempotency_key": "idem-resolve-001",
        "kind": "comment.resolve", "payload": {"thread_id": thread_id, "state": "RESOLVED"}
    });
    let _ = &mut resolve_cmd;
    let resolved = bridge_run(&db, resolve_cmd).await;
    assert!(resolved["ok"].as_bool().unwrap(), "{resolved}");
    assert_eq!(resolved["state"].as_str().unwrap(), "resolved");
    let third = bridge_run(
        &db,
        comment_envelope(&project, revision, "重新讨论", &photos[0], "cmt-third-003"),
    )
    .await;
    assert_ne!(
        third["thread_id"].as_str().unwrap(),
        thread_id,
        "关闭后新线程"
    );
    // 跨项目锚点拒绝。
    let foreign = comment_envelope(
        &project,
        revision,
        "越权",
        "photo-not-exist",
        "cmt-foreign-4",
    );
    let rejected = bridge_run(&db, foreign).await;
    assert!(!rejected["ok"].as_bool().unwrap());
    assert_eq!(rejected["error"]["code"], "PERMISSION_DENIED");
    // Agent 只读：经任务循环内的 get_comments 工具读取（评论进上下文，不构成证据）。
    struct ReadComments;
    impl between_frames_lib::agent::provider::ModelProvider for ReadComments {
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
                let seen = input.protocol.iter().any(|m| m.kind == "tool");
                Ok(if seen {
                    ModelOutcome::NoAction {
                        reason: "评论已读".into(),
                    }
                } else {
                    ModelOutcome::ToolCalls {
                        calls: vec![ToolCall {
                            call_id: "cmt-read-01".into(),
                            name: "get_comments".into(),
                            args: json!({"project_id": input.task.project_id, "photo_id": null, "limit": 10}),
                        }],
                    }
                })
            })
        }
    }
    let task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: between_frames_lib::domain::id(),
            project_id: project.clone(),
            command: TaskCommand::StartTask {
                goal: "读评论".into(),
                selected_photo_ids: photos.clone(),
                target_count: 1,
                budget: Budget::default(),
                consent_id: None,
            },
        })
        .await
        .unwrap();
    AgentEngine::with_provider(db.clone(), Arc::new(ReadComments))
        .run(&task.id)
        .await
        .unwrap();
    let tool_receipt: String = sqlx::query_scalar(
        "SELECT result_json FROM tool_calls WHERE task_id=? AND tool_name='get_comments'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let receipt: Value = serde_json::from_str(&tool_receipt).unwrap();
    assert!(receipt["ok"].as_bool().unwrap());
    let threads = receipt["result"]["threads"].as_array().unwrap();
    assert!(threads.len() >= 2, "应读到新旧两个线程");
    assert!(
        threads
            .iter()
            .any(|t| t["comments"].as_array().unwrap().len() == 2),
        "首线程含两条"
    );
    // 评论不进偏好证据。
    let evidence: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM preference_evidence")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(evidence, 0);
    let _ = AgentEngine::new(db.clone());
}
