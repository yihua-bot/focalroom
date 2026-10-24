use super::*;

#[test]
fn shared_protocol_examples_roundtrip_all_sixteen_actions_without_extra_fields() {
    let examples: Vec<Value> =
        serde_json::from_str(include_str!("../../contracts/canvas-command-examples.json")).unwrap();
    assert_eq!(examples.len(), 20);
    for example in examples {
        let request: CanvasCommandRequest = serde_json::from_value(example.clone()).unwrap();
        validate(&request).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), example);
        let mut invalid = example.clone();
        invalid["actor"] = Value::String("human".into());
        assert!(serde_json::from_value::<CanvasCommandRequest>(invalid).is_err());
    }
}

#[test]
fn legacy_project_ids_do_not_relax_command_and_board_identity_validation() {
    let b = domain::id();
    let mut req = request(
        "demo-first-walk",
        &b,
        0,
        vec![CanvasAction::CreateBoard(CreateBoardCommand {
            title: "演示画布".into(),
        })],
    );
    validate(&req).unwrap();
    for project in [String::new(), "p".repeat(161)] {
        req.project_id = project;
        assert_eq!(validate(&req).unwrap_err().code, "VALIDATION_ERROR");
    }
    req.project_id = "demo-first-walk".into();
    req.command_id = "not-a-command".into();
    assert!(validate(&req).is_err());
    req.command_id = domain::id();
    req.board_id = "not-a-board".into();
    assert!(validate(&req).is_err());
}

fn request(p: &str, b: &str, revision: i64, actions: Vec<CanvasAction>) -> CanvasCommandRequest {
    CanvasCommandRequest {
        schema_version: 3,
        command_id: domain::id(),
        idempotency_key: domain::id(),
        project_id: p.into(),
        board_id: b.into(),
        base_revision: revision,
        operations: actions
            .into_iter()
            .map(|action| CanvasOperation {
                op_id: domain::id(),
                action,
            })
            .collect(),
    }
}
fn text(b: &str, content: &str) -> CanvasAction {
    CanvasAction::AddText(AddTextNodeCommand {
        board_id: b.into(),
        node_type: "sticky".into(),
        x: 0.,
        y: 0.,
        width: 200.,
        height: 100.,
        content: content.into(),
        color: None,
    })
}
async fn fixture() -> (tempfile::TempDir, Store, String, String) {
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("commands.sqlite"))
        .await
        .unwrap();
    let p = db.create("命令回归", "SIMULATED").await.unwrap().id;
    let b = domain::id();
    let created = db
        .execute_canvas(request(
            &p,
            &b,
            0,
            vec![CanvasAction::CreateBoard(CreateBoardCommand {
                title: "画布".into(),
            })],
        ))
        .await
        .unwrap();
    assert_eq!(created.revision, 1);
    (temp, db, p, b)
}
async fn revision(db: &Store, p: &str, b: &str) -> i64 {
    db.canvas_snapshot(p, b).await.unwrap().board.revision
}

#[tokio::test]
async fn seeded_demo_supports_canvas_edit_undo_redo_and_replay_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let resources = temp.path().join("demo");
    let cache = temp.path().join("cache");
    std::fs::create_dir_all(&resources).unwrap();
    image::RgbImage::from_pixel(32, 24, image::Rgb([20, 30, 40]))
        .save(resources.join("demo.jpg"))
        .unwrap();
    let path = temp.path().join("demo.sqlite");
    let db = Store::open(&path).await.unwrap();
    crate::seed_demo::ensure_demo(&db, &resources, &cache)
        .await
        .unwrap();
    let projects = db.list(false).await.unwrap();
    assert_eq!(projects.len(), 1);
    let p = &projects[0].id;
    assert_eq!(p, "demo-first-walk");
    let b = domain::id();
    db.execute_canvas(request(
        p,
        &b,
        0,
        vec![CanvasAction::CreateBoard(CreateBoardCommand {
            title: "演示画布".into(),
        })],
    ))
    .await
    .unwrap();
    let asset_id: String = sqlx::query_scalar("SELECT id FROM source_assets WHERE project_id=?")
        .bind(p)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let edit = request(
        p,
        &b,
        1,
        vec![
            CanvasAction::PlacePhoto(PlacePhotoNodeCommand {
                board_id: b.clone(),
                asset_id,
                x: 0.,
                y: 0.,
                width: 200.,
                height: 150.,
            }),
            text(&b, "演示照片"),
        ],
    );
    let receipt = db.execute_canvas(edit.clone()).await.unwrap();
    let snapshot = db.canvas_snapshot(p, &b).await.unwrap();
    assert_eq!(snapshot.nodes.len(), 2);
    assert_eq!(snapshot.photo_sources.len(), 1);
    db.execute_canvas(request(
        p,
        &b,
        2,
        vec![CanvasAction::Undo(UndoCanvasCommand {
            board_id: b.clone(),
        })],
    ))
    .await
    .unwrap();
    assert!(db.canvas_snapshot(p, &b).await.unwrap().nodes.is_empty());
    db.execute_canvas(request(
        p,
        &b,
        3,
        vec![CanvasAction::Redo(RedoCanvasCommand {
            board_id: b.clone(),
        })],
    ))
    .await
    .unwrap();
    db.pool.close().await;
    let db = Store::open(&path).await.unwrap();
    assert_eq!(
        encode(&db.execute_canvas(edit).await.unwrap()).unwrap(),
        encode(&receipt).unwrap()
    );
    let restored = db.canvas_snapshot(p, &b).await.unwrap();
    assert_eq!(restored.nodes.len(), 2);
    assert_eq!(restored.board.revision, 4);
    // 项目编号格式兼容不授予其他项目的画布写权限。
    let foreign = db
        .execute_canvas(request("another-project", &b, 4, vec![text(&b, "拒绝")]))
        .await
        .unwrap_err();
    assert_eq!(foreign.code, "NOT_FOUND");
    assert_eq!(revision(&db, p, &b).await, 4);
}

#[tokio::test]
async fn retry_after_restart_returns_historical_receipt_without_reapplying_or_learning() {
    let (temp, db, p, b) = fixture().await;
    let first = request(&p, &b, 1, vec![text(&b, "first")]);
    let receipt = db.execute_canvas(first.clone()).await.unwrap();
    db.execute_canvas(request(&p, &b, 2, vec![text(&b, "second")]))
        .await
        .unwrap();
    let audit_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_events")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let evidence: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM preference_evidence")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    db.pool.close().await;
    let db = Store::open(&temp.path().join("commands.sqlite"))
        .await
        .unwrap();
    let replay = db.execute_canvas(first).await.unwrap();
    assert_eq!(encode(&replay).unwrap(), encode(&receipt).unwrap());
    assert_eq!(replay.revision, 2);
    let snapshot = db.canvas_snapshot(&p, &b).await.unwrap();
    assert_eq!(snapshot.board.revision, 3);
    assert_eq!(snapshot.nodes.len(), 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM canvas_command_events")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        3
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM command_events")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        audit_before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM preference_evidence")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        evidence
    );
}

#[tokio::test]
async fn identity_payload_and_operation_reuse_are_rejected() {
    let (_t, db, p, b) = fixture().await;
    let first = request(&p, &b, 1, vec![text(&b, "first")]);
    db.execute_canvas(first.clone()).await.unwrap();
    for variant in 0..5 {
        let mut retry = first.clone();
        match variant {
            0 => retry.command_id = domain::id(),
            1 => retry.idempotency_key = domain::id(),
            2 => retry.base_revision = 2,
            3 => retry.operations[0].action = text(&b, "different"),
            _ => {
                retry.command_id = domain::id();
                retry.idempotency_key = domain::id();
                retry.base_revision = 2;
            }
        }
        assert_eq!(
            db.execute_canvas(retry).await.unwrap_err().code,
            "IDEMPOTENCY_CONFLICT"
        );
    }
    assert_eq!(revision(&db, &p, &b).await, 2);
}

#[tokio::test]
async fn competing_writers_cannot_overwrite_stale_revision() {
    let (temp, db, p, b) = fixture().await;
    let other = Store::open(&temp.path().join("commands.sqlite"))
        .await
        .unwrap();
    let a = request(&p, &b, 1, vec![text(&b, "A")]);
    let c = request(&p, &b, 1, vec![text(&b, "B")]);
    let (a, c) = tokio::join!(db.execute_canvas(a), other.execute_canvas(c));
    assert_ne!(a.is_ok(), c.is_ok());
    let error = a.err().or(c.err()).unwrap();
    assert_eq!(error.code, "REVISION_CONFLICT");
    assert_eq!(error.current_revision, Some(2));
    assert_eq!(db.canvas_snapshot(&p, &b).await.unwrap().nodes.len(), 1);
}

#[tokio::test]
async fn batch_revision_and_undo_redo_are_atomic_and_persisted() {
    let (_t, db, p, b) = fixture().await;
    let batch = request(&p, &b, 1, vec![text(&b, "A"), text(&b, "B")]);
    let ids: Vec<_> = batch.operations.iter().map(|o| o.op_id.clone()).collect();
    assert_eq!(db.execute_canvas(batch).await.unwrap().revision, 2);
    let snapshot = db.canvas_snapshot(&p, &b).await.unwrap();
    assert_eq!(snapshot.nodes.len(), 2);
    assert!(snapshot.nodes.iter().all(|n| ids.contains(&n.id)));
    let undo = request(
        &p,
        &b,
        2,
        vec![CanvasAction::Undo(UndoCanvasCommand {
            board_id: b.clone(),
        })],
    );
    db.execute_canvas(undo.clone()).await.unwrap();
    assert!(db.canvas_snapshot(&p, &b).await.unwrap().nodes.is_empty());
    db.execute_canvas(undo).await.unwrap();
    assert_eq!(revision(&db, &p, &b).await, 3);
    db.execute_canvas(request(
        &p,
        &b,
        3,
        vec![CanvasAction::Redo(RedoCanvasCommand {
            board_id: b.clone(),
        })],
    ))
    .await
    .unwrap();
    assert_eq!(db.canvas_snapshot(&p, &b).await.unwrap().nodes.len(), 2);
    assert_eq!(revision(&db, &p, &b).await, 4);
    // 锁定一项后，整组撤销失败，其他项不会部分隐藏，版本与回执不前进。
    sqlx::query("UPDATE canvas_nodes SET locked=1 WHERE id=?")
        .bind(&ids[0])
        .execute(&db.pool)
        .await
        .unwrap();
    let before = revision(&db, &p, &b).await;
    assert_eq!(
        db.execute_canvas(request(
            &p,
            &b,
            before,
            vec![CanvasAction::Undo(UndoCanvasCommand {
                board_id: b.clone()
            })]
        ))
        .await
        .unwrap_err()
        .code,
        "CONFLICT"
    );
    assert_eq!(db.canvas_snapshot(&p, &b).await.unwrap().nodes.len(), 2);
    assert_eq!(revision(&db, &p, &b).await, before);
    sqlx::query("UPDATE canvas_nodes SET locked=0 WHERE id=?")
        .bind(&ids[0])
        .execute(&db.pool)
        .await
        .unwrap();
    let before = revision(&db, &p, &b).await;
    db.execute_canvas(request(
        &p,
        &b,
        before,
        vec![CanvasAction::Undo(UndoCanvasCommand {
            board_id: b.clone(),
        })],
    ))
    .await
    .unwrap();
    assert!(db.canvas_snapshot(&p, &b).await.unwrap().nodes.is_empty());
}

#[tokio::test]
async fn invalid_second_operation_and_audit_failure_roll_back_every_effect() {
    let (_t, db, p, b) = fixture().await;
    let invalid = request(
        &p,
        &b,
        1,
        vec![
            text(&b, "must rollback"),
            CanvasAction::DeleteNode(DeleteNodeCommand {
                board_id: b.clone(),
                node_id: domain::id(),
            }),
        ],
    );
    assert_eq!(
        db.execute_canvas(invalid).await.unwrap_err().code,
        "NOT_FOUND"
    );
    assert_eq!(revision(&db, &p, &b).await, 1);
    assert!(db.canvas_snapshot(&p, &b).await.unwrap().nodes.is_empty());
    sqlx::query("CREATE TRIGGER fail_canvas_receipt BEFORE INSERT ON canvas_command_events BEGIN SELECT RAISE(ABORT,'injected'); END").execute(&db.pool).await.unwrap();
    let retry = request(&p, &b, 1, vec![text(&b, "rollback receipt")]);
    assert_eq!(
        db.execute_canvas(retry.clone()).await.unwrap_err().code,
        "DATABASE"
    );
    assert_eq!(revision(&db, &p, &b).await, 1);
    assert!(db.canvas_snapshot(&p, &b).await.unwrap().nodes.is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM canvas_op_log")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
    sqlx::query("DROP TRIGGER fail_canvas_receipt")
        .execute(&db.pool)
        .await
        .unwrap();
    db.execute_canvas(retry).await.unwrap();
    assert_eq!(revision(&db, &p, &b).await, 2);
}

#[tokio::test]
async fn immutable_audit_and_boundaries_apply_before_replay() {
    let (_t, db, p, b) = fixture().await;
    let first = request(&p, &b, 1, vec![text(&b, "A")]);
    db.execute_canvas(first.clone()).await.unwrap();
    for context in [
        CommandContext::read_only(&p),
        CommandContext::human(&domain::id()),
    ] {
        assert_eq!(
            CommandHandler::new(&db)
                .execute_canvas(first.clone(), &context)
                .await
                .unwrap_err()
                .code,
            "PERMISSION_DENIED"
        );
    }
    assert!(
        sqlx::query("UPDATE canvas_command_events SET actor='human'")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM canvas_operation_ids")
            .execute(&db.pool)
            .await
            .is_err()
    );
    sqlx::query("UPDATE boards SET state='frozen' WHERE id=?")
        .bind(&b)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        db.execute_canvas(request(&p, &b, 2, vec![text(&b, "blocked")]))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    assert!(
        db.execute_canvas(first).await.is_ok(),
        "原回执可回放，不再改冻结状态"
    );
    let other = db.create_board(&p, "other").await.unwrap();
    assert_eq!(
        db.execute_canvas(request(&p, &b, 2, vec![text(&other.id, "foreign")]))
            .await
            .unwrap_err()
            .code,
        "VALIDATION_ERROR"
    );
}

#[tokio::test]
async fn annotations_use_real_insert_and_share_atomic_history() {
    let (_t, db, p, b) = fixture().await;
    let add = request(
        &p,
        &b,
        1,
        vec![CanvasAction::AddAnnotation(AddAnnotationCommand {
            board_id: b.clone(),
            x: 2.,
            y: 3.,
            body: "讨论构图".into(),
        })],
    );
    let id = add.operations[0].op_id.clone();
    db.execute_canvas(add).await.unwrap();
    let snapshot = db.canvas_snapshot(&p, &b).await.unwrap();
    assert_eq!(snapshot.annotations[0].body, "讨论构图");
    assert_eq!(snapshot.board.revision, 2);
    db.execute_canvas(request(
        &p,
        &b,
        2,
        vec![CanvasAction::ResolveAnnotation(ResolveAnnotationCommand {
            annotation_id: id,
            resolved: true,
        })],
    ))
    .await
    .unwrap();
    assert!(db.canvas_snapshot(&p, &b).await.unwrap().annotations[0].resolved);
    for rev in [3, 4] {
        db.execute_canvas(request(
            &p,
            &b,
            rev,
            vec![CanvasAction::Undo(UndoCanvasCommand {
                board_id: b.clone(),
            })],
        ))
        .await
        .unwrap();
    }
    assert!(db.get_annotations(&b).await.unwrap().is_empty());
    for rev in [5, 6] {
        db.execute_canvas(request(
            &p,
            &b,
            rev,
            vec![CanvasAction::Redo(RedoCanvasCommand {
                board_id: b.clone(),
            })],
        ))
        .await
        .unwrap();
    }
    assert!(db.canvas_snapshot(&p, &b).await.unwrap().annotations[0].resolved);
}

#[tokio::test]
async fn strict_contract_and_duplicate_nodes_are_rejected() {
    let (_t, db, p, b) = fixture().await;
    let cmd = request(&p, &b, 1, vec![text(&b, "A")]);
    let mut json = serde_json::to_value(&cmd).unwrap();
    json["actor"] = Value::String("human".into());
    assert!(serde_json::from_value::<CanvasCommandRequest>(json).is_err());
    let mut json = serde_json::to_value(&cmd).unwrap();
    json["operations"][0]["action"]["data"]["bypass"] = Value::Bool(true);
    assert!(serde_json::from_value::<CanvasCommandRequest>(json).is_err());
    let id = domain::id();
    let bad = request(
        &p,
        &b,
        1,
        vec![CanvasAction::MoveNodes(MoveNodesCommand {
            board_id: b.clone(),
            node_ids: vec![id.clone(), id],
            dx: 1.,
            dy: 1.,
        })],
    );
    assert_eq!(
        db.execute_canvas(bad).await.unwrap_err().code,
        "VALIDATION_ERROR"
    );
}

#[tokio::test]
async fn task_draft_adoption_is_scoped_ready_versioned_and_audited_once() {
    let (_t, db, p, b) = fixture().await;
    let task = domain::id();
    sqlx::query("INSERT INTO agent_tasks(id,project_id,goal,base_revision,allowed_ids_json,selected_ids_json,target_count,state,reason,budget_json,usage_json,created_at,updated_at) VALUES(?,?,'goal',1,'[]','[]',1,'running','','{}','{}','now','now')")
        .bind(&task).bind(&p).execute(&db.pool).await.unwrap();
    sqlx::query("UPDATE boards SET owning_task_id=? WHERE id=?")
        .bind(&task)
        .bind(&b)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        db.execute_canvas(request(&p, &b, 1, vec![text(&b, "not allowed")]))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    let approve = request(
        &p,
        &b,
        1,
        vec![CanvasAction::AdoptBoard(AdoptBoardDraftCommand {
            board_id: b.clone(),
        })],
    );
    assert_eq!(
        db.execute_canvas(approve.clone()).await.unwrap_err().code,
        "CONFLICT"
    );
    sqlx::query("UPDATE agent_tasks SET state='review_ready' WHERE id=?")
        .bind(&task)
        .execute(&db.pool)
        .await
        .unwrap();
    let result = db.execute_canvas(approve.clone()).await.unwrap();
    assert_eq!(result.revision, 2);
    let snapshot = db.canvas_snapshot(&p, &b).await.unwrap();
    assert!(snapshot.board.owning_task_id.is_none());
    assert_eq!(snapshot.board.state, "official");
    assert_eq!(
        encode(&db.execute_canvas(approve).await.unwrap()).unwrap(),
        encode(&result).unwrap()
    );
    db.execute_canvas(request(&p, &b, 2, vec![text(&b, "human edit")]))
        .await
        .unwrap();
    assert_eq!(db.canvas_snapshot(&p, &b).await.unwrap().nodes.len(), 1);
}

#[tokio::test]
async fn cross_board_annotation_and_archived_project_cannot_receive_writes() {
    let (_t, db, p, b) = fixture().await;
    let other = db.create_board(&p, "other").await.unwrap();
    let note = db
        .add_annotation(
            &p,
            &AddAnnotationCommand {
                board_id: other.id,
                x: 0.,
                y: 0.,
                body: "other".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        db.execute_canvas(request(
            &p,
            &b,
            1,
            vec![CanvasAction::ResolveAnnotation(ResolveAnnotationCommand {
                annotation_id: note.id,
                resolved: true
            })]
        ))
        .await
        .unwrap_err()
        .code,
        "NOT_FOUND"
    );
    assert!(db.canvas_snapshot(&domain::id(), &b).await.is_err());
    let version = db.get(&p).await.unwrap().revision;
    db.archive(&p, version).await.unwrap();
    assert_eq!(
        db.execute_canvas(request(&p, &b, 1, vec![text(&b, "archived")]))
            .await
            .unwrap_err()
            .code,
        "NOT_FOUND"
    );
    assert_eq!(revision(&db, &p, &b).await, 1);
}

#[tokio::test]
async fn v28_upgrade_keeps_history_rowids_comments_and_verified_backup() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("v28.sqlite");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let mut old = sqlx::migrate!("./migrations");
    old.migrations =
        std::borrow::Cow::Owned(old.iter().filter(|m| m.version <= 28).cloned().collect());
    old.run(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO projects(id,title,created_at,updated_at) VALUES('p','old','now','now')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO boards(id,project_id,title,created_at,updated_at) VALUES('b','p','old','now','now')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO board_annotations(id,board_id,body,created_at) VALUES('a','b','legacy comment','now')").execute(&pool).await.unwrap();
    for (rowid, id, undone, redone) in [(7_i64, "z", 0_i64, 1_i64), (19, "a", 1, 0)] {
        sqlx::query("INSERT INTO canvas_op_log(rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,redone,created_at) VALUES(?,?,'p','b','set_lock','{}','{}',?,?,'same')")
            .bind(rowid).bind(id).bind(undone).bind(redone).execute(&pool).await.unwrap();
    }
    let before: Vec<(i64, String, i64, i64)> =
        sqlx::query_as("SELECT rowid,id,undone,redone FROM canvas_op_log ORDER BY rowid")
            .fetch_all(&pool)
            .await
            .unwrap();
    pool.close().await;
    let db = Store::open(&path).await.unwrap();
    let after: Vec<(i64, String, i64, i64)> =
        sqlx::query_as("SELECT rowid,id,undone,redone FROM canvas_op_log ORDER BY rowid")
            .fetch_all(&db.pool)
            .await
            .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        db.get_annotations("b").await.unwrap()[0].body,
        "legacy comment"
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&db.pool)
            .await
            .unwrap()
            .is_empty()
    );
    let backup = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|p| p.unwrap().path())
        .find(|p| p.to_string_lossy().contains("backup-before-v28"))
        .unwrap();
    let saved = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(backup)
                .read_only(true),
        )
        .await
        .unwrap();
    let original: Vec<(i64, String, i64, i64)> =
        sqlx::query_as("SELECT rowid,id,undone,redone FROM canvas_op_log ORDER BY rowid")
            .fetch_all(&saved)
            .await
            .unwrap();
    assert_eq!(original, before);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT MAX(version) FROM _sqlx_migrations")
            .fetch_one(&saved)
            .await
            .unwrap(),
        28
    );
    saved.close().await;
}
