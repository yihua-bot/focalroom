use between_frames_lib::{
    command_handler::{CommandContext, CommandHandler, GuiCommandRequest},
    commands_generated::GuiCommand,
    db::Store,
    domain::Project,
    image_pipeline,
};
use image::{Rgb, RgbImage};
use serde_json::{Value, json};
use std::path::Path;

async fn setup(path: &Path) -> (Store, Project) {
    let store = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let mut project = store.create("M1 核心", "观察节奏").await.unwrap();
    for index in 0..3 {
        let source = path.join(format!("f{index}.jpg"));
        RgbImage::from_pixel(80, 60, Rgb([index * 70, 0, 0]))
            .save(&source)
            .unwrap();
        let prepared = image_pipeline::prepare(&source, &path.join("cache")).unwrap();
        store.insert_photo(&project.id, &prepared).await.unwrap();
    }
    project = store.get(&project.id).await.unwrap();
    (store, project)
}
fn request(project: &Project, command: GuiCommand) -> GuiCommandRequest {
    GuiCommandRequest::new(&project.id, project.revision, command)
}
async fn apply(store: &Store, project: &Project, command: GuiCommand) -> Project {
    CommandHandler::new(store)
        .execute_gui(request(project, command))
        .await
        .unwrap()
        .project
        .unwrap()
}
async fn undo(store: &Store, project: &Project) -> Project {
    apply(
        store,
        project,
        GuiCommand::UndoCommandGroup {
            transaction_group_id: project.history.undo_group_id.clone().unwrap(),
        },
    )
    .await
}
async fn redo(store: &Store, project: &Project) -> Project {
    apply(
        store,
        project,
        GuiCommand::RedoCommandGroup {
            transaction_group_id: project.history.redo_group_id.clone().unwrap(),
        },
    )
    .await
}
fn edits(project: &Project) -> Value {
    json!({"title":project.title,"brief":project.brief,"sequence":project.sequence,"photos":project.photos.iter().map(|p|json!({"id":p.id,"status":p.status,"rating":p.rating,"note":p.note})).collect::<Vec<_>>()})
}

#[tokio::test]
async fn durable_replay_checks_fingerprint_scope_and_returns_original_result() {
    let temp = tempfile::tempdir().unwrap();
    let (store, p) = setup(temp.path()).await;
    let req = request(
        &p,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![p.photos[0].id.clone()],
            status: "keep".into(),
        },
    );
    let result = CommandHandler::new(&store)
        .execute_gui(req.clone())
        .await
        .unwrap();
    let first = serde_json::to_value(&result).unwrap();
    let current = result.project.unwrap();
    let later = apply(
        &store,
        &current,
        GuiCommand::SetPhotoRating {
            photo_ids: vec![p.photos[0].id.clone()],
            rating: 4,
        },
    )
    .await;
    assert_eq!(
        serde_json::to_value(
            CommandHandler::new(&store)
                .execute_gui(req.clone())
                .await
                .unwrap()
        )
        .unwrap(),
        first
    );
    let mut changed = req.clone();
    changed.command = GuiCommand::SetPhotoDecision {
        photo_ids: vec![p.photos[1].id.clone()],
        status: "reject".into(),
    };
    assert_eq!(
        CommandHandler::new(&store)
            .execute_gui(changed)
            .await
            .unwrap_err()
            .code,
        "IDEMPOTENCY_CONFLICT"
    );
    let mut wrong = req.clone();
    wrong.project_id = "other".into();
    assert_eq!(
        CommandHandler::new(&store)
            .execute(wrong, &CommandContext::human(&p.id))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    assert_eq!(
        CommandHandler::new(&store)
            .execute(req.clone(), &CommandContext::read_only(&p.id))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    let mut forged = serde_json::to_value(&req).unwrap();
    forged["actorType"] = json!("human");
    assert!(serde_json::from_value::<GuiCommandRequest>(forged).is_err());
    let before = serde_json::to_value(store.get(&p.id).await.unwrap()).unwrap();
    let mut same_key = req.clone();
    same_key.command_id = between_frames_lib::domain::id();
    assert_eq!(
        serde_json::to_value(
            CommandHandler::new(&store)
                .execute_gui(same_key.clone())
                .await
                .unwrap()
        )
        .unwrap(),
        first
    );
    let mut reused_alias = same_key.clone();
    reused_alias.idempotency_key = between_frames_lib::domain::id();
    reused_alias.command = GuiCommand::SetPhotoNote {
        photo_id: p.photos[0].id.clone(),
        note: "不能重用新别名".into(),
    };
    assert_eq!(
        CommandHandler::new(&store)
            .execute_gui(reused_alias)
            .await
            .unwrap_err()
            .code,
        "IDEMPOTENCY_CONFLICT"
    );
    for field in [
        "commandId",
        "projectId",
        "expectedProjectRevision",
        "causationId",
        "idempotencyKey",
        "command",
    ] {
        let mut missing = serde_json::to_value(&req).unwrap();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<GuiCommandRequest>(missing).is_err(),
            "missing {field}"
        );
    }
    assert_eq!(
        serde_json::to_value(store.get(&p.id).await.unwrap()).unwrap(),
        before
    );
    store.pool.close().await;
    let reopened = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(
            CommandHandler::new(&reopened)
                .execute_gui(req)
                .await
                .unwrap()
        )
        .unwrap(),
        first
    );
    assert_eq!(store_revision(&reopened, &p.id).await, later.revision);
}
async fn store_revision(store: &Store, id: &str) -> i64 {
    store.get(id).await.unwrap().revision
}

#[tokio::test]
async fn concurrent_connections_have_one_success_and_deterministic_conflict() {
    let temp = tempfile::tempdir().unwrap();
    let (a, p) = setup(temp.path()).await;
    let b = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let one = request(
        &p,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![p.photos[0].id.clone()],
            status: "keep".into(),
        },
    );
    let two = request(
        &p,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![p.photos[1].id.clone()],
            status: "keep".into(),
        },
    );
    let ha = CommandHandler::new(&a);
    let hb = CommandHandler::new(&b);
    let (one, two) = tokio::join!(ha.execute_gui(one), hb.execute_gui(two));
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    let error = one.err().or_else(|| two.err()).unwrap();
    assert_eq!(error.code, "REVISION_CONFLICT");
    assert_eq!(error.current_revision, Some(p.revision + 1));
    assert_eq!(a.get(&p.id).await.unwrap().sequence.len(), 1);
}

#[tokio::test]
async fn batch_history_undo_redo_persists_and_new_edit_invalidates_redo_branch() {
    let temp = tempfile::tempdir().unwrap();
    let (store, initial) = setup(temp.path()).await;
    let ids: Vec<_> = initial.photos.iter().map(|p| p.id.clone()).collect();
    let kept = apply(
        &store,
        &initial,
        GuiCommand::SetPhotoDecision {
            photo_ids: ids.clone(),
            status: "keep".into(),
        },
    )
    .await;
    assert_eq!(kept.history.undo_count, 1);
    let rated = apply(
        &store,
        &kept,
        GuiCommand::SetPhotoRating {
            photo_ids: ids.clone(),
            rating: 5,
        },
    )
    .await;
    let rated_group = rated.history.undo_group_id.clone().unwrap();
    let mut p = undo(&store, &rated).await;
    assert_eq!(edits(&p), edits(&kept));
    p = undo(&store, &p).await;
    assert_eq!(edits(&p), edits(&initial));
    assert_eq!(p.history.redo_count, 2);
    store.pool.close().await;
    let store = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    p = store.get(&p.id).await.unwrap();
    p = redo(&store, &p).await;
    p = redo(&store, &p).await;
    assert_eq!(edits(&p), edits(&rated));
    assert!(p.revision > rated.revision);
    assert_eq!(
        CommandHandler::new(&store)
            .execute_gui(request(
                &p,
                GuiCommand::UndoCommandGroup {
                    transaction_group_id: kept.history.undo_group_id.unwrap()
                }
            ))
            .await
            .unwrap_err()
            .code,
        "HISTORY_CONFLICT"
    );
    p = undo(&store, &p).await;
    p = apply(
        &store,
        &p,
        GuiCommand::SetPhotoNote {
            photo_id: ids[0].clone(),
            note: "新分支".into(),
        },
    )
    .await;
    assert!(p.history.redo_group_id.is_none());
    assert_eq!(
        CommandHandler::new(&store)
            .execute_gui(request(
                &p,
                GuiCommand::RedoCommandGroup {
                    transaction_group_id: rated_group
                }
            ))
            .await
            .unwrap_err()
            .code,
        "HISTORY_CONFLICT"
    );
    assert!(
        sqlx::query("UPDATE command_events SET command_type='tampered'")
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM decision_events")
            .execute(&store.pool)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn named_version_restore_is_scoped_atomic_and_undoable() {
    let temp = tempfile::tempdir().unwrap();
    let (store, initial) = setup(temp.path()).await;
    let ids: Vec<_> = initial.photos.iter().map(|p| p.id.clone()).collect();
    let mut p = apply(
        &store,
        &initial,
        GuiCommand::SetPhotoDecision {
            photo_ids: ids[..2].to_vec(),
            status: "keep".into(),
        },
    )
    .await;
    p = apply(
        &store,
        &p,
        GuiCommand::SetPhotoNote {
            photo_id: ids[0].clone(),
            note: "版本备注".into(),
        },
    )
    .await;
    p = apply(
        &store,
        &p,
        GuiCommand::SaveSequenceVersion {
            label: "二张初选".into(),
        },
    )
    .await;
    let saved = edits(&p);
    let version = p.versions[0].id.clone();
    p = apply(
        &store,
        &p,
        GuiCommand::SetPhotoDecision {
            photo_ids: ids[0..1].to_vec(),
            status: "reject".into(),
        },
    )
    .await;
    p = apply(
        &store,
        &p,
        GuiCommand::SetPhotoRating {
            photo_ids: ids.clone(),
            rating: 3,
        },
    )
    .await;
    let before = edits(&p);
    p = apply(
        &store,
        &p,
        GuiCommand::RestoreSequenceVersion {
            version_id: version.clone(),
        },
    )
    .await;
    assert_eq!(edits(&p), saved);
    p = undo(&store, &p).await;
    assert_eq!(edits(&p), before);
    p = redo(&store, &p).await;
    assert_eq!(edits(&p), saved);
    let other = store.create("其他项目", "").await.unwrap();
    assert_eq!(
        CommandHandler::new(&store)
            .execute_gui(request(
                &other,
                GuiCommand::RestoreSequenceVersion {
                    version_id: version
                }
            ))
            .await
            .unwrap_err()
            .code,
        "NOT_FOUND"
    );
    assert!(
        sqlx::query("UPDATE sequence_versions SET label='tampered'")
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&store.pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn comparison_undo_is_logical_and_does_not_delete_evidence_rows() {
    let temp = tempfile::tempdir().unwrap();
    let (store, p) = setup(temp.path()).await;
    let compared = apply(
        &store,
        &p,
        GuiCommand::RecordComparison {
            left_id: p.photos[0].id.clone(),
            right_id: p.photos[1].id.clone(),
            preference: "left".into(),
            reason: "留白".into(),
        },
    )
    .await;
    let undone = undo(&store, &compared).await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comparison_events")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let active: i64 = sqlx::query_scalar("SELECT active FROM comparison_state")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(active, 0);
    redo(&store, &undone).await;
    let active: i64 = sqlx::query_scalar("SELECT active FROM comparison_state")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(active, 1);
}

#[tokio::test]
async fn receipt_write_failure_rolls_back_every_projection_event_and_stack() {
    let temp = tempfile::tempdir().unwrap();
    let (store, p) = setup(temp.path()).await;
    let before = serde_json::to_value(&p).unwrap();
    sqlx::query("CREATE TRIGGER injected_failure BEFORE INSERT ON command_events BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END").execute(&store.pool).await.unwrap();
    let result = CommandHandler::new(&store)
        .execute_gui(request(
            &p,
            GuiCommand::SetPhotoDecision {
                photo_ids: vec![p.photos[0].id.clone()],
                status: "keep".into(),
            },
        ))
        .await;
    assert_eq!(result.unwrap_err().code, "DATABASE");
    assert_eq!(
        serde_json::to_value(store.get(&p.id).await.unwrap()).unwrap(),
        before
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_history")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn database_ownership_triggers_reject_cross_project_rows() {
    let temp = tempfile::tempdir().unwrap();
    let (store, p) = setup(temp.path()).await;
    let other = store.create("其他", "").await.unwrap();
    assert!(
        sqlx::query("INSERT INTO photo_decisions(project_id,photo_id,updated_at) VALUES(?,?,?)")
            .bind(&other.id)
            .bind(&p.photos[0].id)
            .bind("now")
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("INSERT INTO sequence_items VALUES(?,?,0,'now')")
            .bind(&other.id)
            .bind(&p.photos[0].id)
            .execute(&store.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("INSERT INTO comparison_events VALUES('bad',?,?,?,NULL,'','now')")
            .bind(&other.id)
            .bind(&p.photos[0].id)
            .bind(&p.photos[1].id)
            .execute(&store.pool)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn old_wal_database_is_backed_up_before_upgrade_and_backup_can_restore_project() {
    use sqlx::{
        Row,
        sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    };
    let temp = tempfile::tempdir().unwrap();
    let migrations = temp.path().join("legacy");
    std::fs::create_dir(&migrations).unwrap();
    std::fs::write(
        migrations.join("0001_local_core.sql"),
        include_str!("../migrations/0001_local_core.sql"),
    )
    .unwrap();
    std::fs::write(
        migrations.join("0002_manual_rating.sql"),
        include_str!("../migrations/0002_manual_rating.sql"),
    )
    .unwrap();
    let path = temp.path().join("old.sqlite");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(migrations.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    let source = temp.path().join("legacy.jpg");
    RgbImage::from_pixel(80, 60, Rgb([0, 0, 90]))
        .save(&source)
        .unwrap();
    let photo = image_pipeline::prepare(&source, &temp.path().join("cache")).unwrap();
    sqlx::query(
        "INSERT INTO projects VALUES('legacy-project','旧项目','旧意图',7,'now','now',NULL)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO photos VALUES('legacy-photo','legacy-project',1,?,'legacy.jpg',?,?,80,60,1,?,?,'now')").bind(photo.path.to_string_lossy().as_ref()).bind(photo.size as i64).bind(&photo.hash).bind(photo.thumbnail.to_string_lossy().as_ref()).bind(photo.preview.to_string_lossy().as_ref()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO photo_decisions VALUES('legacy-project','legacy-photo','keep','旧备注','now',4)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO sequence_items VALUES('legacy-project','legacy-photo',0,'now')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO decision_events VALUES('legacy-event','legacy-project','legacy-photo','user','note',NULL,'{}','旧备注','{}','now')").execute(&pool).await.unwrap();
    // 保持旧连接打开，以验证备份包含未 checkpoint 的已提交 WAL。
    let upgraded = Store::open(&path).await.unwrap();
    let p = upgraded.get("legacy-project").await.unwrap();
    assert_eq!(p.revision, 7);
    assert_eq!(p.photos[0].rating, 4);
    assert_eq!(p.photos[0].note, "旧备注");
    assert_eq!(p.events[0].id, "legacy-event");
    let backups: Vec<_> = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("backup-before-m1")
        })
        .collect();
    assert_eq!(backups.len(), 1);
    let raw = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&backups[0])
                .read_only(true),
        )
        .await
        .unwrap();
    let row = sqlx::query("SELECT status,rating,user_note FROM photo_decisions")
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(row.get::<i64, _>("rating"), 4);
    assert_eq!(row.get::<String, _>("user_note"), "旧备注");
    let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(migration_count, 2);
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(integrity, "ok");
    raw.close().await;
    let restored_path = temp.path().join("restored.sqlite");
    std::fs::copy(&backups[0], &restored_path).unwrap();
    let restored = Store::open(&restored_path).await.unwrap();
    assert_eq!(
        serde_json::to_value(restored.get(&p.id).await.unwrap()).unwrap(),
        serde_json::to_value(&p).unwrap()
    );
    assert_eq!(image_pipeline::fingerprint(&source).unwrap(), photo.hash);
    pool.close().await;
}

#[test]
fn generated_request_contract_round_trips_all_m1_examples_and_rejects_extra_fields() {
    let examples: Vec<Value> =
        serde_json::from_str(include_str!("../../contracts/m1-command-examples.json")).unwrap();
    assert_eq!(examples.len(), 19);
    for example in examples {
        let request: GuiCommandRequest = serde_json::from_value(example.clone()).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), example);
        let mut injected = example.clone();
        injected["command"]["arbitraryPath"] = json!("/private/other-project");
        assert!(serde_json::from_value::<GuiCommandRequest>(injected).is_err());
    }
}

#[tokio::test]
async fn project_brief_and_archive_restore_have_compensating_history() {
    let temp = tempfile::tempdir().unwrap();
    let (store, p) = setup(temp.path()).await;
    let updated = apply(
        &store,
        &p,
        GuiCommand::UpdateProjectBrief {
            brief: "新意图".into(),
        },
    )
    .await;
    assert_eq!(undo(&store, &updated).await.brief, p.brief);
    let current = store.get(&p.id).await.unwrap();
    let archived = apply(&store, &current, GuiCommand::ArchiveProject {}).await;
    assert!(store.get(&p.id).await.is_err());
    let unarchived = undo(&store, &archived).await;
    assert_eq!(
        store.get(&p.id).await.unwrap().revision,
        unarchived.revision
    );
    let archived = redo(&store, &unarchived).await;
    let restored = apply(&store, &archived, GuiCommand::RestoreProject {}).await;
    let undone = undo(&store, &restored).await;
    assert!(store.get(&p.id).await.is_err());
    redo(&store, &undone).await;
    assert!(store.get(&p.id).await.is_ok());
}
