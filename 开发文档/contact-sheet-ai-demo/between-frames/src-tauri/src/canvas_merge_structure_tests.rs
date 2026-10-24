use super::*;
use crate::canvas_review::merge::structure::{StructuralChoice, StructuralResolution};

async fn remove_suggestion(f: &Fixture) -> (EditorialSelection, String, String) {
    let comparison = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let change = &comparison.changes[0];
    let (draft_node, source) = (
        change.node_id.clone(),
        change.source_node_id.clone().unwrap(),
    );
    sqlx::query("UPDATE canvas_nodes SET visible=0 WHERE id=?")
        .bind(draft_node)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let comparison = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let change = comparison
        .changes
        .iter()
        .find(|c| c.source_node_id.as_ref() == Some(&source))
        .unwrap();
    let mut selection = f.selection(0).await;
    selection.selected_change_ids = vec![change.id.clone()];
    (selection, source, change.id.clone())
}
async fn undo(f: &Fixture, redo: bool) -> crate::domain::Result<CanvasReceipt> {
    let revision = f.db.canvas_snapshot(&f.p, &f.source).await?.board.revision;
    let data = UndoCanvasCommand {
        board_id: f.source.clone(),
    };
    f.db.execute_canvas(command(
        &f.p,
        &f.source,
        revision,
        if redo {
            CanvasAction::Redo(RedoCanvasCommand {
                board_id: f.source.clone(),
            })
        } else {
            CanvasAction::Undo(data)
        },
    ))
    .await
}

#[tokio::test]
async fn removed_content_needs_explicit_restore_and_preserves_identity_restart_and_undo() {
    let f = Fixture::new(2).await;
    let comparison = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let change = &comparison.changes[0];
    let source = change.source_node_id.as_ref().unwrap();
    let other = comparison
        .baseline_nodes
        .iter()
        .find(|n| &n.id != source)
        .unwrap()
        .clone();
    f.db.delete_node(
        &f.p,
        &DeleteNodeCommand {
            board_id: f.source.clone(),
            node_id: source.clone(),
        },
    )
    .await
    .unwrap();
    let mut selection = f.selection(1).await;
    let preview =
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap();
    assert!(!preview.can_apply);
    assert_eq!(preview.structural_conflicts[0].kind, "current_removed");
    assert!(!preview.merged_nodes.iter().any(|n| &n.id == source));
    assert_eq!(
        f.db.execute_canvas(f.request(selection.clone()).await)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    selection.structural_resolutions.push(StructuralResolution {
        change_id: change.id.clone(),
        choice: StructuralChoice::RestoreProposed,
    });
    let preview =
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap();
    assert!(preview.can_apply, "{:?}", preview.conflicts);
    let restored = preview
        .merged_nodes
        .iter()
        .find(|n| &n.id == source)
        .unwrap();
    assert_eq!(restored.x, change.after.as_ref().unwrap().x);
    assert_eq!(restored.asset_id, change.before.as_ref().unwrap().asset_id);
    let request = f.request(selection).await;
    let receipt = f.db.execute_canvas(request.clone()).await.unwrap();
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&f.db.execute_canvas(request).await.unwrap()).unwrap()
    );
    assert_eq!(
        f.db.get_board_nodes(&f.source)
            .await
            .unwrap()
            .iter()
            .find(|n| n.id == other.id)
            .unwrap()
            .x,
        other.x
    );
    undo(&f, false).await.unwrap();
    assert!(
        !f.db
            .get_board_nodes(&f.source)
            .await
            .unwrap()
            .iter()
            .any(|n| &n.id == source)
    );
    undo(&f, true).await.unwrap();
    f.db.pool.close().await;
    let db = Store::open(&f._temp.path().join("review.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        db.get_board_nodes(&f.source)
            .await
            .unwrap()
            .iter()
            .find(|n| &n.id == source)
            .unwrap()
            .x,
        change.after.as_ref().unwrap().x
    );
    assert_eq!(
        db.canvas_comparison(&f.p, &f.draft).await.unwrap().changes[0].decision,
        "accepted"
    );
}

#[tokio::test]
async fn removal_previews_related_groups_connectors_and_comments_and_restores_them_atomically() {
    let f = Fixture::new(2).await;
    let (_, source, _) = remove_suggestion(&f).await;
    let other =
        f.db.get_board_nodes(&f.source)
            .await
            .unwrap()
            .into_iter()
            .find(|n| n.id != source)
            .unwrap();
    let group =
        f.db.group_nodes_cmd(
            &f.p,
            &GroupNodesCommand {
                board_id: f.source.clone(),
                node_ids: vec![source.clone(), other.id.clone()],
                label: "保留分组".into(),
            },
        )
        .await
        .unwrap();
    let arrow =
        f.db.add_arrow(
            &f.p,
            &AddArrowCommand {
                board_id: f.source.clone(),
                from_node_id: source.clone(),
                to_node_id: other.id.clone(),
                color: None,
            },
        )
        .await
        .unwrap();
    let outer =
        f.db.group_nodes_cmd(
            &f.p,
            &GroupNodesCommand {
                board_id: f.source.clone(),
                node_ids: vec![group.id.clone(), arrow.id.clone()],
                label: "外层".into(),
            },
        )
        .await
        .unwrap();
    f.db.resize_node(
        &f.p,
        &ResizeNodesCommand {
            board_id: f.source.clone(),
            node_id: source.clone(),
            width: 350.,
            height: 160.,
        },
    )
    .await
    .unwrap();
    sqlx::query("UPDATE canvas_nodes SET rotation=90 WHERE id=?")
        .bind(&source)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let comment = domain::id();
    sqlx::query("INSERT INTO board_annotations(id,board_id,node_id,anchor_type,nx,ny,body,created_at) VALUES(?,?,?,'normalized',1,0.5,'不要丢弃这段意见','now')").bind(&comment).bind(&f.source).bind(&source).execute(&f.db.pool).await.unwrap();
    let comparison = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    let change = comparison
        .changes
        .iter()
        .find(|c| c.kind == "remove")
        .unwrap();
    let mut selection = f.selection(0).await;
    selection.selected_change_ids = vec![change.id.clone()];
    let original = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let preview =
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap();
    assert!(!preview.can_apply);
    let found = &preview.structural_conflicts[0];
    assert!(found.current_modified);
    assert_eq!(found.preserved_comment_count, 1);
    assert_eq!(found.related_nodes.len(), 3);
    assert!(
        found
            .related_nodes
            .iter()
            .any(|n| n.node_id == arrow.id && n.effect == "remove")
    );
    assert_eq!(
        encode(&original).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap()).unwrap()
    );
    selection.structural_resolutions.push(StructuralResolution {
        change_id: change.id.clone(),
        choice: StructuralChoice::RemoveProposed,
    });
    let preview =
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap();
    assert!(preview.can_apply, "{:?}", preview.conflicts);
    let request = f.request(selection).await;
    let merge_id = request.command_id.clone();
    f.db.execute_canvas(request).await.unwrap();
    let after = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    assert!(
        !after
            .nodes
            .iter()
            .any(|n| n.id == source || n.id == arrow.id)
    );
    assert_eq!(
        child_ids(after.nodes.iter().find(|n| n.id == group.id).unwrap()).unwrap(),
        vec![other.id.clone()]
    );
    assert_eq!(
        child_ids(after.nodes.iter().find(|n| n.id == outer.id).unwrap()).unwrap(),
        vec![group.id.clone()]
    );
    let comment_after = after.annotations.iter().find(|a| a.id == comment).unwrap();
    assert_eq!(comment_after.anchor_type, "board");
    assert_eq!(comment_after.node_id, None);
    let source_before = original.nodes.iter().find(|n| n.id == source).unwrap();
    assert_eq!(comment_after.x, Some(source_before.x + 175.));
    assert_eq!(comment_after.y, Some(source_before.y + 255.));
    assert_eq!(comment_after.body, "不要丢弃这段意见");
    sqlx::query("UPDATE board_annotations SET body='后续补充，仍应保留',resolved=1 WHERE id=?")
        .bind(&comment)
        .execute(&f.db.pool)
        .await
        .unwrap();
    undo(&f, false).await.unwrap();
    let restored = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    assert_eq!(restored.nodes.len(), original.nodes.len());
    let restored_comment = restored
        .annotations
        .iter()
        .find(|a| a.id == comment)
        .unwrap();
    assert_eq!(restored_comment.node_id.as_ref(), Some(&source));
    assert_eq!(restored_comment.nx, Some(1.));
    assert_eq!(restored_comment.body, "后续补充，仍应保留");
    assert!(restored_comment.resolved);
    undo(&f, true).await.unwrap();
    let revert =
        f.db.simulate_editorial_revert(&f.p, &f.source, &merge_id)
            .await
            .unwrap();
    assert!(revert.can_apply, "{:?}", revert.conflicts);
    assert_eq!(revert.restored_comment_count, 1);
    let revision = revert.target_revision;
    f.db.execute_canvas(command(
        &f.p,
        &f.source,
        revision,
        CanvasAction::RevertEditorialChanges(RevertEditorialChanges {
            merge_command_id: merge_id,
            simulation_id: revert.simulation_id,
        }),
    ))
    .await
    .unwrap();
    assert_eq!(
        f.db.canvas_snapshot(&f.p, &f.source)
            .await
            .unwrap()
            .annotations
            .iter()
            .find(|a| a.id == comment)
            .unwrap()
            .node_id
            .as_ref(),
        Some(&source)
    );
    undo(&f, false).await.unwrap();
    assert_eq!(
        f.db.canvas_snapshot(&f.p, &f.source)
            .await
            .unwrap()
            .annotations
            .iter()
            .find(|a| a.id == comment)
            .unwrap()
            .anchor_type,
        "board"
    );
    undo(&f, true).await.unwrap();
    f.db.pool.close().await;
    let db = Store::open(&f._temp.path().join("review.sqlite"))
        .await
        .unwrap();
    let restored = db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    assert_eq!(restored.nodes.len(), original.nodes.len());
    assert_eq!(restored.annotations[0].body, "后续补充，仍应保留");
}

#[tokio::test]
async fn explicit_removal_cannot_bypass_locked_dependencies_or_hidden_identity() {
    let f = Fixture::new(2).await;
    let (_, source, _) = remove_suggestion(&f).await;
    let ids =
        f.db.get_board_nodes(&f.source)
            .await
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
    let group =
        f.db.group_nodes_cmd(
            &f.p,
            &GroupNodesCommand {
                board_id: f.source.clone(),
                node_ids: ids,
                label: "锁定".into(),
            },
        )
        .await
        .unwrap();
    f.db.set_node_locked(
        &f.p,
        &SetNodeLockedCommand {
            board_id: f.source.clone(),
            node_id: group.id,
            locked: true,
        },
    )
    .await
    .unwrap();
    let mut selection = f.selection(0).await;
    let comparison = f.db.canvas_comparison(&f.p, &f.draft).await.unwrap();
    selection.selected_change_ids = vec![
        comparison
            .changes
            .iter()
            .find(|c| c.source_node_id.as_ref() == Some(&source))
            .unwrap()
            .id
            .clone(),
    ];
    selection.structural_resolutions.push(StructuralResolution {
        change_id: selection.selected_change_ids[0].clone(),
        choice: StructuralChoice::RemoveProposed,
    });
    // 有锁定关联时不能以确认按钮绕过；不存在可解析冲突时也不接受伪造选择。
    let original = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let blocked = f.db.simulate_editorial_merge(&f.p, &selection).await;
    assert!(blocked.is_err() || !blocked.unwrap().can_apply);
    assert_eq!(
        encode(&original).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap()).unwrap()
    );
    let f = Fixture::new(1).await;
    let source = f.db.get_board_nodes(&f.source).await.unwrap()[0].id.clone();
    f.db.delete_node(
        &f.p,
        &DeleteNodeCommand {
            board_id: f.source.clone(),
            node_id: source.clone(),
        },
    )
    .await
    .unwrap();
    sqlx::query("UPDATE canvas_nodes SET locked=1 WHERE id=?")
        .bind(&source)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let mut selection = f.selection(1).await;
    selection.structural_resolutions.push(StructuralResolution {
        change_id: selection.selected_change_ids[0].clone(),
        choice: StructuralChoice::RestoreProposed,
    });
    let blocked =
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap();
    assert!(!blocked.can_apply);
    assert!(blocked.conflicts.iter().any(|s| s.contains("锁定")));
    sqlx::query("UPDATE canvas_nodes SET locked=0,node_type='shape' WHERE id=?")
        .bind(&source)
        .execute(&f.db.pool)
        .await
        .unwrap();
    assert!(
        !f.db
            .simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap()
            .can_apply
    );
}

#[tokio::test]
async fn altered_comment_anchor_blocks_undo_without_consuming_history_or_partial_restore() {
    let f = Fixture::new(1).await;
    let (_, source, _) = remove_suggestion(&f).await;
    let comment = domain::id();
    sqlx::query("INSERT INTO board_annotations(id,board_id,node_id,anchor_type,body,created_at) VALUES(?,?,?,'node','保留','now')").bind(&comment).bind(&f.source).bind(&source).execute(&f.db.pool).await.unwrap();
    let mut selection = f.selection(1).await;
    selection.structural_resolutions.push(StructuralResolution {
        change_id: selection.selected_change_ids[0].clone(),
        choice: StructuralChoice::RemoveProposed,
    });
    let request = f.request(selection).await;
    let command_id = request.command_id.clone();
    f.db.execute_canvas(request).await.unwrap();
    sqlx::query("UPDATE board_annotations SET x=x+25 WHERE id=?")
        .bind(&comment)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let before = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    let history = f.db.editorial_history(&f.p, &f.source).await.unwrap();
    assert_eq!(undo(&f, false).await.unwrap_err().code, "CONFLICT");
    assert_eq!(
        encode(&before).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap()).unwrap()
    );
    assert_eq!(
        encode(&history).unwrap(),
        encode(&f.db.editorial_history(&f.p, &f.source).await.unwrap()).unwrap()
    );
    let revert =
        f.db.simulate_editorial_revert(&f.p, &f.source, &command_id)
            .await
            .unwrap();
    assert!(!revert.can_apply);
}

#[tokio::test]
async fn anchor_write_failure_rolls_back_nodes_comments_receipt_and_history() {
    let f = Fixture::new(1).await;
    let (_, source, _) = remove_suggestion(&f).await;
    sqlx::query("INSERT INTO board_annotations(id,board_id,node_id,anchor_type,body,created_at) VALUES(?,?,?,'node','保留','now')").bind(domain::id()).bind(&f.source).bind(&source).execute(&f.db.pool).await.unwrap();
    let mut selection = f.selection(1).await;
    selection.structural_resolutions.push(StructuralResolution {
        change_id: selection.selected_change_ids[0].clone(),
        choice: StructuralChoice::RemoveProposed,
    });
    let request = f.request(selection).await;
    let id = request.command_id.clone();
    let before = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_anchor BEFORE UPDATE OF anchor_type ON board_annotations BEGIN SELECT RAISE(ABORT,'injected'); END").execute(&f.db.pool).await.unwrap();
    assert_eq!(
        f.db.execute_canvas(request).await.unwrap_err().code,
        "DATABASE"
    );
    assert_eq!(
        encode(&before).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap()).unwrap()
    );
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM canvas_command_events WHERE command_id=?)"
        )
        .bind(&id)
        .fetch_one(&f.db.pool)
        .await
        .unwrap()
    );
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM canvas_op_log WHERE command_group=?)"
        )
        .bind(&id)
        .fetch_one(&f.db.pool)
        .await
        .unwrap()
    );
}

#[tokio::test]
async fn structural_choices_are_scoped_nonduplicate_and_require_full_restore() {
    let f = Fixture::new(1).await;
    let mut selection = f.selection(1).await;
    selection.structural_resolutions.push(StructuralResolution {
        change_id: selection.selected_change_ids[0].clone(),
        choice: StructuralChoice::RestoreProposed,
    });
    assert_eq!(
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    let source = f.db.get_board_nodes(&f.source).await.unwrap()[0].id.clone();
    f.db.delete_node(
        &f.p,
        &DeleteNodeCommand {
            board_id: f.source.clone(),
            node_id: source,
        },
    )
    .await
    .unwrap();
    selection.expected_target_revision =
        f.db.canvas_snapshot(&f.p, &f.source)
            .await
            .unwrap()
            .board
            .revision;
    assert!(
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap()
            .can_apply
    );
    selection.structural_resolutions[0].choice = StructuralChoice::RemoveProposed;
    assert_eq!(
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    selection.structural_resolutions[0].choice = StructuralChoice::RestoreProposed;
    selection
        .structural_resolutions
        .push(selection.structural_resolutions[0].clone());
    assert_eq!(
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    selection.structural_resolutions.pop();
    selection.selected_change_ids.clear();
    assert_eq!(
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    // 缩小到一个属性不能伪装为对完整复原的同意。
    let f = Fixture::new(1).await;
    sqlx::query("UPDATE canvas_nodes SET width=width+20 WHERE board_id=?")
        .bind(&f.draft)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let source = f.db.get_board_nodes(&f.source).await.unwrap()[0].id.clone();
    f.db.delete_node(
        &f.p,
        &DeleteNodeCommand {
            board_id: f.source.clone(),
            node_id: source,
        },
    )
    .await
    .unwrap();
    let mut selection = f.selection(1).await;
    let id = selection.selected_change_ids[0].clone();
    selection
        .fields_by_change
        .insert(id.clone(), vec!["position".into()]);
    selection.structural_resolutions.push(StructuralResolution {
        change_id: id,
        choice: StructuralChoice::RestoreProposed,
    });
    let preview =
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap();
    assert!(!preview.can_apply);
    assert!(preview.conflicts.iter().any(|s| s.contains("全部属性")));
}

#[tokio::test]
async fn invalid_or_excessive_comment_anchors_and_duplicate_group_members_are_rejected() {
    let f = Fixture::new(1).await;
    let (_, source, _) = remove_suggestion(&f).await;
    assert_eq!(
        f.db.group_nodes_cmd(
            &f.p,
            &GroupNodesCommand {
                board_id: f.source.clone(),
                node_ids: vec![source.clone(), source.clone()],
                label: "不能重复".into()
            }
        )
        .await
        .unwrap_err()
        .code,
        "VALIDATION_ERROR"
    );
    let comment = domain::id();
    sqlx::query("INSERT INTO board_annotations(id,board_id,node_id,anchor_type,nx,ny,body,created_at) VALUES(?,?,?,'normalized',2,0.5,'不能猜测位置','now')").bind(&comment).bind(&f.source).bind(&source).execute(&f.db.pool).await.unwrap();
    let mut selection = f.selection(1).await;
    selection.structural_resolutions.push(StructuralResolution {
        change_id: selection.selected_change_ids[0].clone(),
        choice: StructuralChoice::RemoveProposed,
    });
    assert_eq!(
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    sqlx::query("UPDATE board_annotations SET anchor_type='node',nx=NULL,ny=NULL WHERE id=?")
        .bind(&comment)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let mut tx = f.db.pool.begin().await.unwrap();
    for _ in 0..1000 {
        sqlx::query("INSERT INTO board_annotations(id,board_id,node_id,anchor_type,body,created_at) VALUES(?,?,?,'node','容量边界','now')").bind(domain::id()).bind(&f.source).bind(&source).execute(&mut *tx).await.unwrap();
    }
    tx.commit().await.unwrap();
    let before = f.db.canvas_snapshot(&f.p, &f.source).await.unwrap();
    assert_eq!(
        f.db.simulate_editorial_merge(&f.p, &selection)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    assert_eq!(
        encode(&before).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &f.source).await.unwrap()).unwrap()
    );
}
