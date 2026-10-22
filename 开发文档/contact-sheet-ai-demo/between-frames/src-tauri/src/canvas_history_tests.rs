use super::*;

async fn fixture() -> (tempfile::TempDir, Store, String, String, String) {
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("history.sqlite"))
        .await
        .unwrap();
    let project = db.create("历史回归", "本地操作模拟").await.unwrap();
    let board = db.create_board(&project.id, "画布").await.unwrap();
    let node = db
        .add_text_node(
            &project.id,
            &AddTextNodeCommand {
                board_id: board.id.clone(),
                node_type: "sticky".into(),
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 100.0,
                content: "note".into(),
                color: None,
            },
        )
        .await
        .unwrap();
    (temp, db, project.id, board.id, node.id)
}

async fn move_by(db: &Store, p: &str, b: &str, n: &str, dx: f64, dy: f64) {
    db.move_nodes(
        p,
        &MoveNodesCommand {
            board_id: b.into(),
            node_ids: vec![n.into()],
            dx,
            dy,
        },
    )
    .await
    .unwrap();
}

async fn position(db: &Store, n: &str) -> (f64, f64) {
    sqlx::query_as("SELECT x,y FROM canvas_nodes WHERE id=?")
        .bind(n)
        .fetch_one(&db.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn redo_replays_multiple_undos_in_original_order_even_with_equal_timestamps() {
    let (_temp, db, p, b, n) = fixture().await;
    move_by(&db, &p, &b, &n, 10.0, 0.0).await;
    move_by(&db, &p, &b, &n, 0.0, 20.0).await;
    sqlx::query("UPDATE canvas_op_log SET created_at='2026-01-01' WHERE board_id=?")
        .bind(&b)
        .execute(&db.pool)
        .await
        .unwrap();
    let undo = UndoCanvasCommand {
        board_id: b.clone(),
    };
    let redo = RedoCanvasCommand {
        board_id: b.clone(),
    };
    assert!(db.undo_canvas(&p, &undo).await.unwrap());
    assert_eq!(position(&db, &n).await, (10.0, 0.0));
    assert!(db.undo_canvas(&p, &undo).await.unwrap());
    assert_eq!(position(&db, &n).await, (0.0, 0.0));
    assert!(db.redo_canvas(&p, &redo).await.unwrap());
    assert_eq!(position(&db, &n).await, (10.0, 0.0));
    assert!(db.redo_canvas(&p, &redo).await.unwrap());
    assert_eq!(position(&db, &n).await, (10.0, 20.0));
    assert!(!db.redo_canvas(&p, &redo).await.unwrap());
}

#[tokio::test]
async fn new_edit_discards_redo_branch_without_resurrecting_it_after_more_undos() {
    let (_temp, db, p, b, n) = fixture().await;
    move_by(&db, &p, &b, &n, 10.0, 0.0).await;
    db.undo_canvas(
        &p,
        &UndoCanvasCommand {
            board_id: b.clone(),
        },
    )
    .await
    .unwrap();
    move_by(&db, &p, &b, &n, 0.0, 20.0).await;
    let redo = RedoCanvasCommand {
        board_id: b.clone(),
    };
    assert!(!db.redo_canvas(&p, &redo).await.unwrap());
    db.undo_canvas(
        &p,
        &UndoCanvasCommand {
            board_id: b.clone(),
        },
    )
    .await
    .unwrap();
    assert!(db.redo_canvas(&p, &redo).await.unwrap());
    assert_eq!(position(&db, &n).await, (0.0, 20.0));
    assert!(!db.redo_canvas(&p, &redo).await.unwrap());
}

#[tokio::test]
async fn locked_redo_rolls_back_all_nodes_and_does_not_consume_history() {
    let (_temp, db, p, b, n) = fixture().await;
    let second = db
        .add_text_node(
            &p,
            &AddTextNodeCommand {
                board_id: b.clone(),
                node_type: "sticky".into(),
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
                content: "second".into(),
                color: None,
            },
        )
        .await
        .unwrap();
    db.move_nodes(
        &p,
        &MoveNodesCommand {
            board_id: b.clone(),
            node_ids: vec![n.clone(), second.id.clone()],
            dx: 10.0,
            dy: 0.0,
        },
    )
    .await
    .unwrap();
    db.undo_canvas(
        &p,
        &UndoCanvasCommand {
            board_id: b.clone(),
        },
    )
    .await
    .unwrap();
    // 模拟别的编辑上下文锁定，不追加当前操作栈。
    sqlx::query("UPDATE canvas_nodes SET locked=1 WHERE id=?")
        .bind(&second.id)
        .execute(&db.pool)
        .await
        .unwrap();
    let redo = RedoCanvasCommand {
        board_id: b.clone(),
    };
    assert_eq!(
        db.redo_canvas(&p, &redo).await.unwrap_err().code,
        "CONFLICT"
    );
    assert_eq!(position(&db, &n).await, (0.0, 0.0));
    sqlx::query("UPDATE canvas_nodes SET locked=0 WHERE id=?")
        .bind(&second.id)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(db.redo_canvas(&p, &redo).await.unwrap());
    assert_eq!(position(&db, &n).await, (10.0, 0.0));
    assert_eq!(position(&db, &second.id).await, (10.0, 0.0));
}

#[tokio::test]
async fn resize_and_lock_history_survives_database_reopen() {
    let (temp, db, p, b, n) = fixture().await;
    db.resize_node(
        &p,
        &ResizeNodesCommand {
            board_id: b.clone(),
            node_id: n.clone(),
            width: 400.0,
            height: 300.0,
        },
    )
    .await
    .unwrap();
    db.set_node_locked(
        &p,
        &SetNodeLockedCommand {
            board_id: b.clone(),
            node_id: n.clone(),
            locked: true,
        },
    )
    .await
    .unwrap();
    db.pool.close().await;
    let db = Store::open(&temp.path().join("history.sqlite"))
        .await
        .unwrap();
    let undo = UndoCanvasCommand {
        board_id: b.clone(),
    };
    let redo = RedoCanvasCommand {
        board_id: b.clone(),
    };
    assert!(db.undo_canvas(&p, &undo).await.unwrap());
    assert!(db.undo_canvas(&p, &undo).await.unwrap());
    let values: (f64, f64, bool) =
        sqlx::query_as("SELECT width,height,locked FROM canvas_nodes WHERE id=?")
            .bind(&n)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(values, (200.0, 100.0, false));
    assert!(db.redo_canvas(&p, &redo).await.unwrap());
    assert!(db.redo_canvas(&p, &redo).await.unwrap());
    let values: (f64, f64, bool) =
        sqlx::query_as("SELECT width,height,locked FROM canvas_nodes WHERE id=?")
            .bind(&n)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(values, (400.0, 300.0, true));
}

#[tokio::test]
async fn migration_preserves_legacy_history_order_and_redo_state() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("legacy.sqlite");
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
        std::borrow::Cow::Owned(old.iter().filter(|m| m.version <= 27).cloned().collect());
    old.run(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO projects(id,title,created_at,updated_at) VALUES('p','legacy','now','now')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO boards(id,project_id,title,created_at,updated_at) VALUES('b','p','legacy','now','now')").execute(&pool).await.unwrap();
    for (rowid, id, undone, redone) in [(3_i64, "z", 0_i64, 1_i64), (8, "a", 1, 0)] {
        sqlx::query("INSERT INTO canvas_op_log(rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,created_at,redone) VALUES(?,?,'p','b','move','{}','{}',?,'same',?)")
            .bind(rowid).bind(id).bind(undone).bind(redone).execute(&pool).await.unwrap();
    }
    let before: Vec<(i64, String, i64, i64)> =
        sqlx::query_as("SELECT rowid,id,undone,redone FROM canvas_op_log ORDER BY rowid")
            .fetch_all(&pool)
            .await
            .unwrap();
    pool.close().await;
    let upgraded = Store::open(&path).await.unwrap();
    let after: Vec<(i64, String, i64, i64)> =
        sqlx::query_as("SELECT rowid,id,undone,redone FROM canvas_op_log ORDER BY rowid")
            .fetch_all(&upgraded.pool)
            .await
            .unwrap();
    assert_eq!(before, after);
    let issues: Vec<sqlx::sqlite::SqliteRow> = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&upgraded.pool)
        .await
        .unwrap();
    assert!(issues.is_empty());
    let backup = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().contains("backup-before-v27"))
        .expect("升级前应自动备份");
    let backup_pool = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(backup)
            .read_only(true),
    )
    .await
    .unwrap();
    let backup_rows: Vec<(i64, String, i64, i64)> =
        sqlx::query_as("SELECT rowid,id,undone,redone FROM canvas_op_log ORDER BY rowid")
            .fetch_all(&backup_pool)
            .await
            .unwrap();
    assert_eq!(before, backup_rows);
    backup_pool.close().await;
}
