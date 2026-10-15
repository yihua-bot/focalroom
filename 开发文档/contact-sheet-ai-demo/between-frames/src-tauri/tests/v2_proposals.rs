// v2.0 M5 提案协议：create→submit→apply 完整闭环、nonce 防越权、部分采纳、
// base 漂移拒绝、Agent 可创建但只能人工审批执行。
use between_frames_lib::{db::Store, image_pipeline, v2_bridge};
use betweenframes_local_domain::parse_command;
use serde_json::{Value, json};
use std::path::Path;

async fn setup(path: &Path, count: usize) -> (Store, String, Vec<String>, i64) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let project = db.create("提案", "").await.unwrap();
    for i in 0..count {
        let source = path.join(format!("pr{i}.png"));
        image::RgbImage::from_pixel(32, 24, image::Rgb([(i * 91 % 256) as u8, 50, 140]))
            .save(&source)
            .unwrap();
        db.insert_photo(
            &project.id,
            &image_pipeline::prepare(&source, &path.join("cache")).unwrap(),
        )
        .await
        .unwrap();
    }
    let project = db.get(&project.id).await.unwrap();
    (
        db,
        project.id.clone(),
        project.photos.iter().map(|p| p.id.clone()).collect(),
        project.revision,
    )
}

async fn run(db: &Store, command: Value, agent: bool) -> Value {
    let project = command["project_id"].as_str().unwrap().to_owned();
    let principal = if agent {
        v2_bridge::agent_principal("local-workspace", &project, 4_102_444_800_000)
    } else {
        v2_bridge::local_owner_principal("local-workspace", &project)
    };
    let authority = v2_bridge::authority_for(db, &project).await.unwrap();
    let envelope = parse_command(command).expect("契约合法");
    v2_bridge::execute(
        db,
        v2_bridge::V2Execution {
            envelope,
            principal,
            authority,
        },
        1_900_000_000_000,
    )
    .await
}

fn proposal_envelope(project: &str, ops: Value, base: i64, key: &str) -> Value {
    json!({
        "version": "2.0", "command_id": format!("cmd-{key}"), "workspace_id": "local-workspace",
        "project_id": project, "expected_revision": base, "idempotency_key": format!("idem-{key}"),
        "kind": "proposal.create",
        "payload": {"base_revision": base, "title": "v2 提案", "operations": ops}
    })
}

#[tokio::test]
async fn proposal_full_cycle_with_partial_accept_and_nonce() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project, photos, revision) = setup(temp.path(), 4).await;
    let ops = json!([
        {"operation_id": "op-add-1", "photo_id": photos[0], "op": "add", "to_index": 0, "reason": "开篇"},
        {"operation_id": "op-add-2", "photo_id": photos[1], "op": "add", "to_index": 1, "reason": "承接"},
        {"operation_id": "op-bad", "photo_id": "photo-none", "op": "remove", "reason": "越权"},
    ]);
    // Agent 创建提案（可提案角色），先因跨项目照片被拒。
    let mut bad = proposal_envelope(&project, ops.clone(), revision, "agent-bad-01");
    let rejected = run(&db, bad.clone(), true).await;
    assert!(!rejected["ok"].as_bool().unwrap());
    assert_eq!(rejected["error"]["code"], "PERMISSION_DENIED");
    // 合法操作集创建。
    let legal: Vec<Value> = ops.as_array().unwrap().iter().take(2).cloned().collect();
    bad["payload"]["operations"] = json!(legal);
    bad["command_id"] = json!("cmd-agent-create1");
    bad["idempotency_key"] = json!("idem-agent-create1");
    let created = run(&db, bad, true).await;
    assert!(created["ok"].as_bool().unwrap(), "{created}");
    let proposal_id = created["proposal_id"].as_str().unwrap().to_owned();
    let nonce = created["approval_nonce"].as_str().unwrap().to_owned();
    // submit（Agent 无 submit 权限 → 拒绝；owner 提交）。
    let mut submit = json!({
        "version": "2.0", "command_id": "cmd-agent-submit1", "workspace_id": "local-workspace",
        "project_id": project, "expected_revision": revision, "idempotency_key": "idem-agent-submit1",
        "kind": "proposal.submit", "payload": {"proposal_id": proposal_id, "expected_proposal_revision": 1}
    });
    let denied = run(&db, submit.clone(), true).await;
    assert_eq!(
        denied["error"]["code"], "PERMISSION_DENIED",
        "Agent 不得提交审批"
    );
    submit["command_id"] = json!("cmd-owner-submit1");
    submit["idempotency_key"] = json!("idem-owner-submit1");
    let submitted = run(&db, submit, false).await;
    assert!(submitted["ok"].as_bool().unwrap(), "{submitted}");
    // apply：错误 nonce 拒绝。
    let apply_cmd = |accepted: Value, nonce_value: &str, key: &str| {
        json!({
            "version": "2.0", "command_id": format!("cmd-{key}"), "workspace_id": "local-workspace",
            "project_id": project, "expected_revision": revision, "idempotency_key": format!("idem-{key}"),
            "kind": "proposal.apply",
            "payload": {"proposal_id": proposal_id, "expected_proposal_revision": 2, "accepted_operation_ids": accepted, "approval_nonce": nonce_value}
        })
    };
    let wrong_nonce = run(
        &db,
        apply_cmd(json!(["op-add-1"]), "wrong-nonce", "apply-bad-nonce"),
        false,
    )
    .await;
    assert_eq!(wrong_nonce["error"]["code"], "PERMISSION_DENIED");
    // 部分采纳：只接受第一个操作。
    let applied = run(
        &db,
        apply_cmd(json!(["op-add-1"]), &nonce, "apply-partial-1"),
        false,
    )
    .await;
    assert!(applied["ok"].as_bool().unwrap(), "{applied}");
    assert_eq!(applied["outcome"]["applied"], 1);
    // 正式状态：只有 photos[0] 被 keep（部分采纳语义）。
    let after = db.get(&project).await.unwrap();
    assert_eq!(after.sequence, vec![photos[0].clone()]);
    // 可整体撤销（v1 历史语义继承）。
    let head: (String,) = sqlx::query_as(
        "SELECT h.group_id FROM command_history h JOIN history_heads c ON c.project_id=h.project_id AND c.cursor=h.position WHERE h.project_id=? AND h.active=1",
    ).bind(&project).fetch_one(&db.pool).await.unwrap();
    let undone = between_frames_lib::command_handler::CommandHandler::new(&db)
        .execute_gui(between_frames_lib::command_handler::GuiCommandRequest::new(
            &project,
            after.revision,
            between_frames_lib::commands_generated::GuiCommand::UndoCommandGroup {
                transaction_group_id: head.0,
            },
        ))
        .await
        .unwrap()
        .project
        .unwrap();
    assert!(undone.sequence.is_empty(), "提案执行经历史可撤销");
}

#[tokio::test]
async fn stale_base_revision_proposal_is_rejected_on_apply() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project, photos, revision) = setup(temp.path(), 2).await;
    let ops = json!([{"operation_id": "op-1", "photo_id": photos[0], "op": "add", "to_index": 0, "reason": ""}]);
    let created = run(
        &db,
        proposal_envelope(&project, ops, revision, "stale-create-1"),
        false,
    )
    .await;
    assert!(created["ok"].as_bool().unwrap());
    let proposal_id = created["proposal_id"].as_str().unwrap().to_owned();
    run(&db, json!({
        "version": "2.0", "command_id": "cmd-stale-submit", "workspace_id": "local-workspace",
        "project_id": project, "expected_revision": revision, "idempotency_key": "idem-stale-submit",
        "kind": "proposal.submit", "payload": {"proposal_id": proposal_id, "expected_proposal_revision": 1}
    }), false).await;
    // 项目在提案提交后变动 → apply 拒绝并要求重建。
    use between_frames_lib::command_handler::{CommandHandler, GuiCommandRequest};
    use between_frames_lib::commands_generated::GuiCommand;
    let current = db.get(&project).await.unwrap();
    CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &project,
            current.revision,
            GuiCommand::SetPhotoRating {
                photo_ids: vec![photos[1].clone()],
                rating: 5,
            },
        ))
        .await
        .unwrap();
    let applied = run(&db, json!({
        "version": "2.0", "command_id": "cmd-stale-apply-01", "workspace_id": "local-workspace",
        "project_id": project, "expected_revision": current.revision, "idempotency_key": "idem-stale-apply",
        "kind": "proposal.apply",
        "payload": {"proposal_id": proposal_id, "expected_proposal_revision": 2, "accepted_operation_ids": ["op-1"], "approval_nonce": created["approval_nonce"]}
    }), false).await;
    assert_eq!(applied["error"]["code"], "REVISION_CONFLICT");
    assert!(
        applied["error"]["message"]
            .as_str()
            .unwrap()
            .contains("重建")
    );
}
