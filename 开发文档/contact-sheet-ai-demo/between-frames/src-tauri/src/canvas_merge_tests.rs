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
            sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,content_json,children_json,z_order,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,'now','now')")
                .bind(id).bind(&draft).bind(&node.node_type).bind(node.x + 50.).bind(node.y + 25.).bind(node.width).bind(node.height).bind(&node.content_json).bind(children).bind(node.z_order).execute(&db.pool).await.unwrap();
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
            resolutions: Vec::new(),
            fields_by_change: std::collections::BTreeMap::new(),
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
async fn three_way_merge_preserves_manual_size_and_other_nodes_and_requires_explicit_overlap_choice()
 {
    let f = Fixture::new(2).await;
    let comparison = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let change = &comparison.changes[0];
    let target = change.source_node_id.as_ref().unwrap();
    f.db.resize_node(
        &f.p,
        &ResizeNodesCommand {
            board_id: f.source.clone(),
            node_id: target.clone(),
            width: 350.,
            height: 160.,
        },
    )
    .await
    .unwrap();
    let selected = f.selection(1).await;
    let preview =
        f.db.simulate_editorial_merge(&f.p, &selected)
            .await
            .unwrap();
    assert!(preview.can_apply, "{:?}", preview.conflicts);
    let result = preview
        .merged_nodes
        .iter()
        .find(|n| &n.id == target)
        .unwrap();
    assert_eq!(
        (result.x, result.y),
        (
            change.after.as_ref().unwrap().x,
            change.after.as_ref().unwrap().y
        )
    );
    assert_eq!((result.width, result.height), (350., 160.));
    f.db.move_nodes(
        &f.p,
        &MoveNodesCommand {
            board_id: f.source.clone(),
            node_ids: vec![target.clone()],
            dx: 12.,
            dy: 0.,
        },
    )
    .await
    .unwrap();
    let mut selected = f.selection(1).await;
    let overlap =
        f.db.simulate_editorial_merge(&f.p, &selected)
            .await
            .unwrap();
    assert!(!overlap.can_apply);
    assert_eq!(overlap.attribute_conflicts.len(), 1);
    assert_eq!(overlap.attribute_conflicts[0].field, "position");
    let blocked = f.request(selected.clone()).await;
    assert_eq!(
        f.db.execute_canvas(blocked).await.unwrap_err().code,
        "CONFLICT"
    );
    selected.resolutions.push(EditorialResolution {
        change_id: change.id.clone(),
        field: "position".into(),
        choice: ResolutionChoice::KeepCurrent,
    });
    let keep =
        f.db.simulate_editorial_merge(&f.p, &selected)
            .await
            .unwrap();
    assert!(keep.can_apply);
    assert_eq!(
        keep.merged_nodes
            .iter()
            .find(|n| &n.id == target)
            .unwrap()
            .x,
        change.before.as_ref().unwrap().x + 12.
    );
    selected.resolutions[0].choice = ResolutionChoice::UseDraft;
    let request = f.request(selected.clone()).await;
    selected.resolutions.push(selected.resolutions[0].clone());
    assert_eq!(
        f.db.simulate_editorial_merge(&f.p, &selected)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    f.db.execute_canvas(request).await.unwrap();
    let result = f.db.get_board_nodes(&f.source).await.unwrap();
    let node = result.iter().find(|n| &n.id == target).unwrap();
    assert_eq!(node.x, change.after.as_ref().unwrap().x);
    assert_eq!(node.width, 350.);
}

#[tokio::test]
async fn successive_partial_approvals_and_decisions_survive_restart_and_undo() {
    let f = Fixture::new(3).await;
    let original = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let comparison = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let first = f.selection(1).await;
    f.db.execute_canvas(f.request(first).await).await.unwrap();
    let review = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(review.changes[0].decision, "accepted");
    let mut next = f.selection(0).await;
    next.selected_change_ids = vec![comparison.changes[1].id.clone()];
    let second = f.db.execute_canvas(f.request(next).await).await.unwrap();
    assert_eq!(second.revision, original.board.revision + 2);
    let review = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(
        review
            .changes
            .iter()
            .filter(|c| c.decision == "accepted")
            .count(),
        2
    );
    let decision = command(
        &f.p,
        &f.source,
        second.revision,
        CanvasAction::RecordEditorialDecision(RecordEditorialDecision {
            draft_id: f.draft.clone(),
            draft_revision: review.draft.revision,
            change_ids: vec![review.changes[2].id.clone()],
            decision: decisions::ReviewDecision::Deferred,
            fields_by_change: std::collections::BTreeMap::new(),
        }),
    );
    let snapshot = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let receipt = f.db.execute_canvas(decision.clone()).await.unwrap();
    assert!(!receipt.changed);
    assert_eq!(receipt.revision, second.revision);
    assert_eq!(
        encode(&snapshot).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap()).unwrap()
    );
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&f.db.execute_canvas(decision.clone()).await.unwrap()).unwrap()
    );
    f.db.pool.close().await;
    let db = Store::open(&f._temp.path().join("review.sqlite"))
        .await
        .unwrap();
    let review = db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(review.changes[2].decision, "deferred");
    db.execute_canvas(command(
        &f.p,
        &f.source,
        second.revision,
        CanvasAction::Undo(UndoCanvasCommand {
            board_id: f.source.clone(),
        }),
    ))
    .await
    .unwrap();
    let review = db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(review.changes[0].decision, "accepted");
    assert_eq!(review.changes[1].decision, "pending");
    assert_eq!(review.changes[2].decision, "deferred");
    assert_eq!(
        CommandHandler::new(&db)
            .execute_canvas(decision, &CommandContext::read_only(&f.p))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
}

#[tokio::test]
async fn rejection_does_not_change_work_and_cannot_relabel_an_accepted_change() {
    let f = Fixture::new(2).await;
    let c = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let selection = f.selection(1).await;
    let rejected = command(
        &f.p,
        &f.source,
        selection.expected_target_revision,
        CanvasAction::RecordEditorialDecision(RecordEditorialDecision {
            draft_id: f.draft.clone(),
            draft_revision: c.draft.revision,
            change_ids: vec![c.changes[0].id.clone()],
            decision: decisions::ReviewDecision::Rejected,
            fields_by_change: std::collections::BTreeMap::new(),
        }),
    );
    let before = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    f.db.execute_canvas(rejected.clone()).await.unwrap();
    assert_eq!(
        f.db.canvas_comparison(&f.p, &f.draft)
            .await
            .unwrap()
            .changes[0]
            .decision,
        "rejected"
    );
    assert_eq!(
        encode(&before).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap()).unwrap()
    );
    f.db.execute_canvas(f.request(selection).await)
        .await
        .unwrap();
    let mut invalid = rejected;
    invalid.command_id = domain::id();
    invalid.idempotency_key = domain::id();
    invalid.operations[0].op_id = domain::id();
    invalid.base_revision += 1;
    assert_eq!(
        f.db.execute_canvas(invalid).await.unwrap_err().code,
        "CONFLICT"
    );
    assert_eq!(
        f.db.canvas_comparison(&f.p, &f.draft)
            .await
            .unwrap()
            .changes[0]
            .decision,
        "accepted"
    );
}

#[tokio::test]
async fn individual_properties_can_be_accepted_in_separate_rounds_and_undone_independently() {
    let f = Fixture::new(1).await;
    sqlx::query("UPDATE canvas_nodes SET width=300,height=150 WHERE board_id=?")
        .bind(&f.draft)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let c = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(c.changes[0].fields, vec!["position", "size"]);
    let mut selection = f.selection(1).await;
    selection
        .fields_by_change
        .insert(c.changes[0].id.clone(), vec!["position".into()]);
    f.db.execute_canvas(f.request(selection).await)
        .await
        .unwrap();
    let c = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(c.changes[0].decision, "partial");
    assert_eq!(c.changes[0].accepted_fields, vec!["position"]);
    assert_eq!(c.current_source_nodes[0].width, 200.);
    let mut selection = f.selection(1).await;
    selection
        .fields_by_change
        .insert(c.changes[0].id.clone(), vec!["size".into()]);
    let second =
        f.db.execute_canvas(f.request(selection).await)
            .await
            .unwrap();
    let c = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(c.changes[0].decision, "accepted");
    assert_eq!(c.current_source_nodes[0].width, 300.);
    f.db.execute_canvas(command(
        &f.p,
        &f.source,
        second.revision,
        CanvasAction::Undo(UndoCanvasCommand {
            board_id: f.source.clone(),
        }),
    ))
    .await
    .unwrap();
    let c = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(c.changes[0].decision, "partial");
    assert_eq!(c.changes[0].accepted_fields, vec!["position"]);
    assert_eq!(
        c.current_source_nodes[0].x,
        c.changes[0].after.as_ref().unwrap().x
    );
    assert_eq!(c.current_source_nodes[0].width, 200.);
    let mut forged = f.selection(1).await;
    forged
        .fields_by_change
        .insert(c.changes[0].id.clone(), vec!["asset".into()]);
    assert_eq!(
        f.db.simulate_editorial_merge(&f.p, &forged)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
}

#[tokio::test]
async fn deferred_size_does_not_relabel_an_undone_position_decision() {
    let f = Fixture::new(1).await;
    sqlx::query("UPDATE canvas_nodes SET width=300 WHERE board_id=?")
        .bind(&f.draft)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let mut selection = f.selection(1).await;
    let id = selection.selected_change_ids[0].clone();
    selection
        .fields_by_change
        .insert(id.clone(), vec!["position".into()]);
    let applied =
        f.db.execute_canvas(f.request(selection).await)
            .await
            .unwrap();
    let c = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    f.db.execute_canvas(command(
        &f.p,
        &f.source,
        applied.revision,
        CanvasAction::RecordEditorialDecision(RecordEditorialDecision {
            draft_id: f.draft.clone(),
            draft_revision: c.draft.revision,
            change_ids: vec![id.clone()],
            decision: decisions::ReviewDecision::Deferred,
            fields_by_change: std::collections::BTreeMap::from([(id, vec!["size".into()])]),
        }),
    ))
    .await
    .unwrap();
    let c = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(c.changes[0].field_decisions["position"], "accepted");
    assert_eq!(c.changes[0].field_decisions["size"], "deferred");
    f.db.execute_canvas(command(
        &f.p,
        &f.source,
        applied.revision,
        CanvasAction::Undo(UndoCanvasCommand {
            board_id: f.source.clone(),
        }),
    ))
    .await
    .unwrap();
    let c = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    assert_eq!(c.changes[0].field_decisions["position"], "pending");
    assert_eq!(c.changes[0].field_decisions["size"], "deferred");
}

#[tokio::test]
async fn historical_compensation_preserves_later_unrelated_fields_and_is_itself_undoable() {
    let f = Fixture::new(2).await;
    let comparison = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let node = comparison.changes[0]
        .source_node_id
        .as_ref()
        .unwrap()
        .clone();
    let approval =
        f.db.execute_canvas(f.request(f.selection(1).await).await)
            .await
            .unwrap();
    let resized =
        f.db.execute_canvas(command(
            &f.p,
            &f.source,
            approval.revision,
            CanvasAction::ResizeNode(ResizeNodesCommand {
                board_id: f.source.clone(),
                node_id: node.clone(),
                width: 360.,
                height: 180.,
            }),
        ))
        .await
        .unwrap();
    let snapshot = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let preview =
        f.db.simulate_editorial_revert(&f.p, &f.source, &approval.command_id)
            .await
            .unwrap();
    assert!(preview.can_apply, "{:?}", preview.conflicts);
    assert_eq!(preview.target_revision, resized.revision);
    assert_eq!(
        encode(&snapshot).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap()).unwrap()
    );
    let restored = preview.merged_nodes.iter().find(|n| n.id == node).unwrap();
    assert_eq!(restored.x, comparison.changes[0].before.as_ref().unwrap().x);
    assert_eq!(restored.width, 360.);
    let request = command(
        &f.p,
        &f.source,
        preview.target_revision,
        CanvasAction::RevertEditorialChanges(RevertEditorialChanges {
            merge_command_id: approval.command_id.clone(),
            simulation_id: preview.simulation_id,
        }),
    );
    let receipt = f.db.execute_canvas(request.clone()).await.unwrap();
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&f.db.execute_canvas(request).await.unwrap()).unwrap()
    );
    let history = f.db.editorial_history(&f.p, &f.source).await.unwrap();
    assert_eq!(history.len(), 2);
    assert!(history[0].is_compensation);
    assert!(history[1].compensated);
    let older =
        f.db.editorial_history_page(&f.p, &f.source, Some(history[0].cursor))
            .await
            .unwrap();
    assert_eq!(older.len(), 1);
    assert_eq!(older[0].command_id, approval.command_id);
    assert_eq!(
        f.db.canvas_comparison(&f.p, &f.draft)
            .await
            .unwrap()
            .changes[0]
            .decision,
        "pending"
    );
    assert_eq!(
        f.db.simulate_editorial_revert(&f.p, &f.source, &approval.command_id)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
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
    let now = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let restored = now.nodes.iter().find(|n| n.id == node).unwrap();
    assert_eq!(restored.x, comparison.changes[0].after.as_ref().unwrap().x);
    assert_eq!(restored.width, 360.);
    assert_eq!(
        f.db.canvas_comparison(&f.p, &f.draft)
            .await
            .unwrap()
            .changes[0]
            .decision,
        "accepted"
    );
    assert!(!f.db.editorial_history(&f.p, &f.source).await.unwrap()[1].compensated);
}

#[tokio::test]
async fn compensation_conflicts_scope_and_stale_preview_never_overwrite_later_changes() {
    let f = Fixture::new(1).await;
    let approval =
        f.db.execute_canvas(f.request(f.selection(1).await).await)
            .await
            .unwrap();
    let preview =
        f.db.simulate_editorial_revert(&f.p, &f.source, &approval.command_id)
            .await
            .unwrap();
    assert_eq!(
        f.db.simulate_editorial_revert("foreign-project", &f.source, &approval.command_id)
            .await
            .unwrap_err()
            .code,
        "NOT_FOUND"
    );
    let node = preview.current_nodes[0].id.clone();
    f.db.move_nodes(
        &f.p,
        &MoveNodesCommand {
            board_id: f.source.clone(),
            node_ids: vec![node],
            dx: 7.,
            dy: 0.,
        },
    )
    .await
    .unwrap();
    let current = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let blocked =
        f.db.simulate_editorial_revert(&f.p, &f.source, &approval.command_id)
            .await
            .unwrap();
    assert!(!blocked.can_apply);
    assert!(!blocked.conflicts.is_empty());
    let command = command(
        &f.p,
        &f.source,
        preview.target_revision,
        CanvasAction::RevertEditorialChanges(RevertEditorialChanges {
            merge_command_id: approval.command_id,
            simulation_id: preview.simulation_id,
        }),
    );
    assert_eq!(
        f.db.execute_canvas(command).await.unwrap_err().code,
        "REVISION_CONFLICT"
    );
    assert_eq!(
        encode(&current).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap()).unwrap()
    );
    assert_eq!(
        f.db.editorial_history(&f.p, &f.source).await.unwrap().len(),
        1
    );
}

#[tokio::test]
async fn comments_replies_resolve_reopen_are_immutable_scoped_and_do_not_edit_the_draft() {
    let f = Fixture::new(1).await;
    let comparison = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let before = f.db.canvas_snapshot(&f.p, &f.draft).await.unwrap();
    let original = f.db.get(&f.p).await.unwrap();
    let root = command(
        &f.p,
        &f.draft,
        before.board.revision,
        CanvasAction::AddEditorialComment(AddEditorialComment {
            draft_id: f.draft.clone(),
            draft_revision: before.board.revision,
            change_id: Some(comparison.changes[0].id.clone()),
            thread_id: None,
            body: "开篇保留，中段请增加留白。".into(),
        }),
    );
    let thread = root.operations[0].op_id.clone();
    let receipt = f.db.execute_canvas(root.clone()).await.unwrap();
    assert!(!receipt.changed);
    assert_eq!(receipt.revision, before.board.revision);
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&f.db.execute_canvas(root.clone()).await.unwrap()).unwrap()
    );
    assert_eq!(
        CommandHandler::new(&f.db)
            .execute_canvas(root, &CommandContext::read_only(&f.p))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    let reply = |body: &str| {
        command(
            &f.p,
            &f.draft,
            before.board.revision,
            CanvasAction::AddEditorialComment(AddEditorialComment {
                draft_id: f.draft.clone(),
                draft_revision: before.board.revision,
                change_id: Some(comparison.changes[0].id.clone()),
                thread_id: Some(thread.clone()),
                body: body.into(),
            }),
        )
    };
    f.db.execute_canvas(reply("希望保留街道空镜作为过渡。"))
        .await
        .unwrap();
    let resolve = |resolved| {
        command(
            &f.p,
            &f.draft,
            before.board.revision,
            CanvasAction::ResolveEditorialThread(ResolveEditorialThread {
                draft_id: f.draft.clone(),
                draft_revision: before.board.revision,
                thread_id: thread.clone(),
                resolved,
            }),
        )
    };
    f.db.execute_canvas(resolve(true)).await.unwrap();
    assert_eq!(
        f.db.execute_canvas(reply("关闭的批注不能直接回复"))
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    assert!(
        f.db.editorial_discussion(&f.p, &f.draft, before.board.revision, 0, None)
            .await
            .unwrap()
            .messages
            .iter()
            .all(|m| m.resolved)
    );
    f.db.execute_canvas(resolve(false)).await.unwrap();
    f.db.execute_canvas(reply("重新讨论这个观看距离。"))
        .await
        .unwrap();
    assert_eq!(
        f.db.editorial_discussion("another-project", &f.draft, before.board.revision, 0, None)
            .await
            .unwrap_err()
            .code,
        "NOT_FOUND"
    );
    assert_eq!(
        encode(&before).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.draft).await.unwrap()).unwrap()
    );
    assert_eq!(
        encode(&original).unwrap(),
        encode(&f.db.get(&f.p).await.unwrap()).unwrap()
    );
    f.db.pool.close().await;
    let db = Store::open(&f._temp.path().join("review.sqlite"))
        .await
        .unwrap();
    let discussion = db
        .editorial_discussion(&f.p, &f.draft, before.board.revision, 0, None)
        .await
        .unwrap();
    assert_eq!(discussion.total_messages, 3);
    assert!(discussion.messages.iter().all(|m| !m.resolved));
    assert_eq!(
        discussion.messages[2].root_body,
        "开篇保留，中段请增加留白。"
    );
    let audit_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM canvas_command_events")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let invalid = command(
        &f.p,
        &f.draft,
        before.board.revision,
        CanvasAction::AddEditorialComment(AddEditorialComment {
            draft_id: f.draft.clone(),
            draft_revision: before.board.revision,
            change_id: None,
            thread_id: Some(thread),
            body: "改变批注所指建议不被接受".into(),
        }),
    );
    assert_eq!(
        db.execute_canvas(invalid).await.unwrap_err().code,
        "CONFLICT"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM canvas_command_events")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        audit_before
    );
}

#[tokio::test]
async fn discussion_paging_is_frozen_and_old_version_comments_are_never_reanchored() {
    let f = Fixture::new(1).await;
    let revision =
        f.db.canvas_snapshot(&f.p, &f.draft)
            .await
            .unwrap()
            .board
            .revision;
    for index in 0..35 {
        f.db.execute_canvas(command(
            &f.p,
            &f.draft,
            revision,
            CanvasAction::AddEditorialComment(AddEditorialComment {
                draft_id: f.draft.clone(),
                draft_revision: revision,
                change_id: None,
                thread_id: None,
                body: format!("批注 {index}"),
            }),
        ))
        .await
        .unwrap();
    }
    let first =
        f.db.editorial_discussion(&f.p, &f.draft, revision, 0, None)
            .await
            .unwrap();
    assert_eq!(first.messages.len(), 30);
    assert_eq!(first.next_offset, Some(30));
    let thread = first.messages[0].thread_id.clone();
    f.db.execute_canvas(command(
        &f.p,
        &f.draft,
        revision,
        CanvasAction::ResolveEditorialThread(ResolveEditorialThread {
            draft_id: f.draft.clone(),
            draft_revision: revision,
            thread_id: thread,
            resolved: true,
        }),
    ))
    .await
    .unwrap();
    let frozen =
        f.db.editorial_discussion(&f.p, &f.draft, revision, 0, Some(first.cursor))
            .await
            .unwrap();
    assert!(!frozen.messages[0].resolved);
    assert!(
        f.db.editorial_discussion(&f.p, &f.draft, revision, 0, None)
            .await
            .unwrap()
            .messages[0]
            .resolved
    );
    let next =
        f.db.editorial_discussion(
            &f.p,
            &f.draft,
            revision,
            first.next_offset.unwrap(),
            Some(first.cursor),
        )
        .await
        .unwrap();
    assert_eq!(next.messages.len(), 5);
    assert_eq!(next.next_offset, None);
    sqlx::query("UPDATE boards SET revision=revision+1 WHERE id=?")
        .bind(&f.draft)
        .execute(&f.db.pool)
        .await
        .unwrap();
    assert_eq!(
        f.db.editorial_discussion(&f.p, &f.draft, revision + 1, 0, None)
            .await
            .unwrap()
            .total_messages,
        0
    );
    assert_eq!(
        f.db.editorial_discussion(&f.p, &f.draft, revision, 0, None)
            .await
            .unwrap()
            .total_messages,
        35
    );
    let stale = command(
        &f.p,
        &f.draft,
        revision + 1,
        CanvasAction::AddEditorialComment(AddEditorialComment {
            draft_id: f.draft.clone(),
            draft_revision: revision,
            change_id: None,
            thread_id: None,
            body: "旧版意见不能落入新版".into(),
        }),
    );
    assert_eq!(
        f.db.execute_canvas(stale).await.unwrap_err().code,
        "REVISION_CONFLICT"
    );
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
    assert!(
        sim.can_apply,
        "其他节点的人工修改不应阻止无冲突采纳: {:?}",
        sim.conflicts
    );
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
