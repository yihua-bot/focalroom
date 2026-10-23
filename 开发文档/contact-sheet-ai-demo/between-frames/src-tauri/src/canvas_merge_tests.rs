use super::*;
use crate::{
    canvas::*,
    canvas_commands::*,
    command_handler::{CommandContext, CommandHandler},
    domain,
};

struct Fixture {
    _temp: tempfile::TempDir,
    db: Store,
    p: String,
    source: String,
    draft: String,
}

#[tokio::test]
async fn v31_upgrade_preserves_history_order_groups_and_undo_state() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("v31.sqlite");
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
        std::borrow::Cow::Owned(old.iter().filter(|m| m.version <= 31).cloned().collect());
    old.run(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO projects(id,title,created_at,updated_at) VALUES('p','old','now','now')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO boards(id,project_id,title,created_at,updated_at) VALUES('b','p','old','now','now')").execute(&pool).await.unwrap();
    for (row, id, undone, redone) in [(7_i64, "first", 0_i64, 1_i64), (19, "second", 1, 0)] {
        sqlx::query("INSERT INTO canvas_op_log(rowid,id,project_id,board_id,op_type,op_data,undo_data,undone,created_at,redone,command_group) VALUES(?,?,'p','b','move','{}','{}',?,'now',?,'group')")
            .bind(row).bind(id).bind(undone).bind(redone).execute(&pool).await.unwrap();
    }
    let before: Vec<(i64, String, i64, i64, String)> = sqlx::query_as(
        "SELECT rowid,id,undone,redone,command_group FROM canvas_op_log ORDER BY rowid",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    pool.close().await;
    let db = Store::open(&path).await.unwrap();
    let after: Vec<(i64, String, i64, i64, String)> = sqlx::query_as(
        "SELECT rowid,id,undone,redone,command_group FROM canvas_op_log ORDER BY rowid",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    assert_eq!(before, after);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&db.pool)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(std::fs::read_dir(temp.path()).unwrap().any(|e| {
        e.unwrap()
            .path()
            .to_string_lossy()
            .contains("backup-before-v31")
    }));
}
impl Fixture {
    async fn new(count: usize) -> Self {
        Self::build(count, false).await
    }
    async fn build(count: usize, with_arrow: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("review.sqlite"))
            .await
            .unwrap();
        let p = db.create("审稿回归", "SIMULATED").await.unwrap().id;
        let source = db.create_board(&p, "原画布").await.unwrap().id;
        for i in 0..count {
            db.add_text_node(
                &p,
                &AddTextNodeCommand {
                    board_id: source.clone(),
                    node_type: "sticky".into(),
                    x: i as f64 * 240.,
                    y: 0.,
                    width: 200.,
                    height: 100.,
                    content: format!("{i}"),
                    color: None,
                },
            )
            .await
            .unwrap();
        }
        if with_arrow {
            let nodes = db.get_board_nodes(&source).await.unwrap();
            db.add_arrow(
                &p,
                &AddArrowCommand {
                    board_id: source.clone(),
                    from_node_id: nodes[0].id.clone(),
                    to_node_id: nodes[1].id.clone(),
                    color: None,
                },
            )
            .await
            .unwrap();
        }
        let baseline = db.canvas_snapshot(&p, &source).await.unwrap();
        let draft = db.create_board(&p, "方案 A").await.unwrap().id;
        let task = domain::id();
        sqlx::query("INSERT INTO agent_tasks(id,project_id,goal,base_revision,allowed_ids_json,selected_ids_json,target_count,state,reason,budget_json,usage_json,created_at,updated_at) VALUES(?,?,'goal',?,'[]','[]',1,'review_ready','','{}','{}','now','now')")
            .bind(&task).bind(&p).bind(db.get(&p).await.unwrap().revision).execute(&db.pool).await.unwrap();
        sqlx::query("UPDATE boards SET owning_task_id=? WHERE id=?")
            .bind(&task)
            .bind(&draft)
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO board_draft_tasks(board_id,task_id,created_at) VALUES(?,?,'now')")
            .bind(&draft)
            .bind(&task)
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO board_draft_origins(board_id,task_id,source_board_id,source_revision,snapshot_sha256,created_at) VALUES(?,?,?,?,'fixture','now')")
            .bind(&draft).bind(&task).bind(&source).bind(baseline.board.revision).execute(&db.pool).await.unwrap();
        let mapping: HashMap<_, _> = baseline
            .nodes
            .iter()
            .map(|n| (n.id.clone(), domain::id()))
            .collect();
        for node in baseline.nodes {
            let id = &mapping[&node.id];
            let children = node.children_json.as_ref().map(|_| {
                encode(
                    &child_ids(&node)
                        .unwrap()
                        .iter()
                        .map(|id| &mapping[id])
                        .collect::<Vec<_>>(),
                )
                .unwrap()
            });
            sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,content_json,children_json,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,'now','now')")
                .bind(id).bind(&draft).bind(&node.node_type).bind(node.x + 50.).bind(node.y + 25.).bind(node.width).bind(node.height).bind(&node.content_json).bind(children).execute(&db.pool).await.unwrap();
            sqlx::query("INSERT INTO board_draft_node_origins(node_id,board_id,source_node_id,base_json) VALUES(?,?,?,?)")
                .bind(id).bind(&draft).bind(&node.id).bind(encode(&node).unwrap()).execute(&db.pool).await.unwrap();
        }
        Self {
            _temp: temp,
            db,
            p,
            source,
            draft,
        }
    }
    async fn selection(&self, count: usize) -> EditorialSelection {
        let c = self
            .db
            .canvas_comparison(&self.p, &self.draft)
            .await
            .unwrap();
        EditorialSelection {
            draft_id: self.draft.clone(),
            draft_revision: c.draft.revision,
            target_board_id: self.source.clone(),
            expected_target_revision: c.current_source_revision.unwrap(),
            selected_change_ids: c.changes.iter().take(count).map(|c| c.id.clone()).collect(),
        }
    }
    async fn request(&self, selection: EditorialSelection) -> CanvasCommandRequest {
        let simulation = self
            .db
            .simulate_editorial_merge(&self.p, &selection)
            .await
            .unwrap();
        command(
            &self.p,
            &self.source,
            selection.expected_target_revision,
            CanvasAction::ApplyEditorialChanges(ApplyEditorialChanges {
                selection,
                simulation_id: simulation.simulation_id,
            }),
        )
    }
}
fn command(p: &str, board: &str, revision: i64, action: CanvasAction) -> CanvasCommandRequest {
    CanvasCommandRequest {
        schema_version: 3,
        command_id: domain::id(),
        idempotency_key: domain::id(),
        project_id: p.into(),
        board_id: board.into(),
        base_revision: revision,
        operations: vec![CanvasOperation {
            op_id: domain::id(),
            action,
        }],
    }
}

#[tokio::test]
async fn twenty_proposals_three_selected_match_preview_atomic_undo_redo_and_restart() {
    let f = Fixture::new(20).await;
    let before = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let selection = f.selection(3).await;
    let first = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(
        encode(&first).unwrap(),
        encode(&f.db.canvas_comparison(&f.p, &f.draft).await.unwrap()).unwrap()
    );
    assert_eq!(first.changes.len(), 20);
    let preview =
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap();
    assert!(preview.can_apply, "{:?}", preview.conflicts);
    assert_eq!(
        encode(&before.nodes).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap().nodes).unwrap()
    );
    let request = f.request(selection).await;
    let receipt = f.db.execute_canvas(request.clone()).await.unwrap();
    let after = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    assert_eq!(receipt.revision, before.board.revision + 1);
    assert_eq!(
        after
            .nodes
            .iter()
            .filter(|n| before.nodes.iter().find(|old| old.id == n.id).unwrap().x != n.x)
            .count(),
        3
    );
    let normalize = |nodes: &[CanvasNode]| -> std::collections::BTreeMap<String, Value> {
        nodes
            .iter()
            .map(|n| (n.id.clone(), geometry(n, &HashMap::new()).unwrap()))
            .collect()
    };
    assert_eq!(normalize(&preview.merged_nodes), normalize(&after.nodes));
    f.db.execute_canvas(command(
        &f.p,
        &f.source,
        receipt.revision,
        CanvasAction::Undo(UndoCanvasCommand {
            board_id: f.source.clone(),
        }),
    ))
    .await
    .unwrap();
    assert_eq!(
        normalize(&before.nodes),
        normalize(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap().nodes)
    );
    f.db.execute_canvas(command(
        &f.p,
        &f.source,
        receipt.revision + 1,
        CanvasAction::Redo(RedoCanvasCommand {
            board_id: f.source.clone(),
        }),
    ))
    .await
    .unwrap();
    f.db.pool.close().await;
    let db = Store::open(&f._temp.path().join("review.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        normalize(&after.nodes),
        normalize(&db.canvas_snapshot(&f.p, &f.source).await.unwrap().nodes)
    );
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&db.execute_canvas(request).await.unwrap()).unwrap()
    );
    let audit: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*),SUM(undone) FROM canvas_op_log WHERE op_type='editorial_merge'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(audit, (1, 0));
}

#[tokio::test]
async fn dependencies_are_closed_and_empty_selection_cannot_apply() {
    let f = Fixture::new(0).await;
    let note = domain::id();
    let group = domain::id();
    for (id, kind, children) in [
        (&note, "sticky", None),
        (&group, "group", Some(json!([note]).to_string())),
    ] {
        sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,children_json,created_at,updated_at) VALUES(?,?,?,?,'now','now')")
            .bind(id).bind(&f.draft).bind(kind).bind(children).execute(&f.db.pool).await.unwrap();
    }
    let c = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let mut s = f.selection(0).await;
    assert!(
        !f.db
            .simulate_editorial_merge(&f.p, &s)
            .await
            .unwrap()
            .can_apply
    );
    s.selected_change_ids = vec![
        c.changes
            .iter()
            .find(|c| c.node_id == group)
            .unwrap()
            .id
            .clone(),
    ];
    let preview = f.db.simulate_editorial_merge(&f.p, &s).await.unwrap();
    assert!(preview.can_apply);
    assert_eq!(preview.auto_included_change_ids.len(), 1);
    assert_eq!(preview.merged_nodes.len(), 2);
    let g = preview
        .merged_nodes
        .iter()
        .find(|n| n.node_type == "group")
        .unwrap();
    assert!(
        preview
            .merged_nodes
            .iter()
            .any(|n| child_ids(g).unwrap().contains(&n.id) && n.node_type == "sticky")
    );
    f.db.execute_canvas(f.request(s).await).await.unwrap();
    assert_eq!(
        f.db.canvas_snapshot(&f.p, &f.source)
            .await
            .unwrap()
            .nodes
            .len(),
        2
    );
}

#[tokio::test]
async fn removing_a_node_includes_proposed_connector_cleanup_and_restores_as_one_group() {
    let f = Fixture::build(2, true).await;
    let before = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let nodes = f.db.get_board_nodes(&f.draft).await.unwrap();
    let note = nodes.iter().find(|n| n.node_type == "sticky").unwrap();
    let arrow = nodes.iter().find(|n| n.node_type == "arrow").unwrap();
    sqlx::query("UPDATE canvas_nodes SET visible=0 WHERE id IN (?,?)")
        .bind(&note.id)
        .bind(&arrow.id)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let c = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let mut s = f.selection(0).await;
    s.selected_change_ids = vec![
        c.changes
            .iter()
            .find(|c| c.node_id == note.id)
            .unwrap()
            .id
            .clone(),
    ];
    let simulation = f.db.simulate_editorial_merge(&f.p, &s).await.unwrap();
    assert!(simulation.can_apply, "{:?}", simulation.conflicts);
    assert_eq!(simulation.auto_included_change_ids.len(), 1);
    assert_eq!(simulation.merged_nodes.len(), 1);
    let result = f.db.execute_canvas(f.request(s).await).await.unwrap();
    f.db.execute_canvas(command(
        &f.p,
        &f.source,
        result.revision,
        CanvasAction::Undo(UndoCanvasCommand {
            board_id: f.source.clone(),
        }),
    ))
    .await
    .unwrap();
    let restored = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    assert_eq!(restored.nodes.len(), before.nodes.len());
    let source_arrow = restored
        .nodes
        .iter()
        .find(|n| n.node_type == "arrow")
        .unwrap();
    assert!(
        child_ids(source_arrow)
            .unwrap()
            .iter()
            .all(|id| restored.nodes.iter().any(|n| &n.id == id))
    );
}

#[tokio::test]
async fn source_or_draft_changes_invalidates_approval_instead_of_overwriting() {
    let f = Fixture::new(2).await;
    let request = f.request(f.selection(1).await).await;
    let original = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    f.db.move_nodes(
        &f.p,
        &MoveNodesCommand {
            board_id: f.source.clone(),
            node_ids: vec![original.nodes[1].id.clone()],
            dx: 10.,
            dy: 0.,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        f.db.execute_canvas(request).await.unwrap_err().code,
        "REVISION_CONFLICT"
    );
    let sim =
        f.db.simulate_editorial_merge(&f.p, &f.selection(1).await)
            .await
            .unwrap();
    assert!(!sim.can_apply);
    assert!(sim.conflicts.iter().any(|s| s.contains("原画布已有修改")));
    let f = Fixture::new(1).await;
    let request = f.request(f.selection(1).await).await;
    sqlx::query("UPDATE canvas_nodes SET x=x+1 WHERE board_id=?")
        .bind(&f.draft)
        .execute(&f.db.pool)
        .await
        .unwrap();
    assert_eq!(
        f.db.execute_canvas(request).await.unwrap_err().code,
        "CONFLICT"
    );
}

#[tokio::test]
async fn tampered_simulation_selection_and_authority_cannot_write() {
    let f = Fixture::new(1).await;
    let mut request = f.request(f.selection(1).await).await;
    if let CanvasAction::ApplyEditorialChanges(cmd) = &mut request.operations[0].action {
        cmd.simulation_id = "0".repeat(64);
    }
    assert_eq!(
        f.db.execute_canvas(request).await.unwrap_err().code,
        "CONFLICT"
    );
    let request = f.request(f.selection(1).await).await;
    assert_eq!(
        CommandHandler::new(&f.db)
            .execute_canvas(request, &CommandContext::read_only(&f.p))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    let mut selection = f.selection(1).await;
    selection.selected_change_ids = vec!["0".repeat(64)];
    assert_eq!(
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    let mut selection = f.selection(1).await;
    selection.target_board_id = f.draft.clone();
    assert!(
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn competing_merges_have_one_winner_and_audit_failure_rolls_back() {
    let f = Fixture::new(2).await;
    let a = f.request(f.selection(1).await).await;
    let b = f.request(f.selection(2).await).await;
    let other = Store::open(&f._temp.path().join("review.sqlite"))
        .await
        .unwrap();
    let (a, b) = tokio::join!(f.db.execute_canvas(a), other.execute_canvas(b));
    assert_ne!(a.is_ok(), b.is_ok());
    assert_eq!(a.err().or(b.err()).unwrap().code, "REVISION_CONFLICT");
    let f = Fixture::new(2).await;
    let original = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let request = f.request(f.selection(2).await).await;
    sqlx::query("CREATE TRIGGER fail_merge_history BEFORE INSERT ON canvas_op_log WHEN NEW.op_type='editorial_merge' BEGIN SELECT RAISE(ABORT,'injected'); END").execute(&f.db.pool).await.unwrap();
    assert_eq!(
        f.db.execute_canvas(request).await.unwrap_err().code,
        "DATABASE"
    );
    let after = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    assert_eq!(original.board.revision, after.board.revision);
    assert_eq!(
        encode(&original.nodes).unwrap(),
        encode(&after.nodes).unwrap()
    );
}

#[tokio::test]
async fn locked_nodes_dangling_references_and_node_comments_block_safely() {
    let f = Fixture::new(1).await;
    let source = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap().nodes[0]
        .id
        .clone();
    let request = f.request(f.selection(1).await).await;
    sqlx::query("UPDATE canvas_nodes SET locked=1 WHERE id=?")
        .bind(&source)
        .execute(&f.db.pool)
        .await
        .unwrap();
    assert!(f.db.execute_canvas(request).await.is_err());
    let f = Fixture::new(1).await;
    sqlx::query("UPDATE canvas_nodes SET visible=0 WHERE board_id=?")
        .bind(&f.draft)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let source = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap().nodes[0]
        .id
        .clone();
    sqlx::query("INSERT INTO board_annotations(id,board_id,node_id,anchor_type,body,created_at) VALUES(?,?,?,'node','保留评论','now')")
        .bind(domain::id()).bind(&f.source).bind(source).execute(&f.db.pool).await.unwrap();
    let preview =
        f.db.simulate_editorial_merge(&f.p, &f.selection(1).await)
            .await
            .unwrap();
    assert!(!preview.can_apply);
    assert!(preview.conflicts.iter().any(|s| s.contains("评论")));
    let f = Fixture::new(0).await;
    sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,children_json,created_at,updated_at) VALUES(?,?,'arrow',?,'now','now')")
        .bind(domain::id()).bind(&f.draft).bind(json!([domain::id(),domain::id()]).to_string()).execute(&f.db.pool).await.unwrap();
    assert!(
        !f.db
            .simulate_editorial_merge(&f.p, &f.selection(1).await)
            .await
            .unwrap()
            .can_apply
    );
}
