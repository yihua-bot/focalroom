use super::*;
use typography::Direction;
async fn layout(f: &Fixture) -> versions::PublicationLayout {
    f.db.publication_layout(&f.project, &f.publication)
        .await
        .unwrap()
}
fn text(page: &str) -> TextUpdate {
    TextUpdate {
        text_id: domain::id(),
        page_id: page.into(),
        x_mm: 20.,
        y_mm: 30.,
        width_mm: 8.,
        height_mm: 160.,
        kind: "text".into(),
        text: "Night Walk · 夜行".into(),
        text_direction: Some(Direction::Rotate90),
        font_family: "serif".into(),
        font_size_pt: 10.,
        line_height: 1.2,
        align: "left".into(),
        bold: false,
        color: "#222222".into(),
    }
}
#[tokio::test]
async fn text_direction_is_versioned_replayable_and_geometry_edits_do_not_change_it() {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    let mut data = text(&before.pages[1].id);
    let request = f.request(PublicationAction::AddText(data.clone())).await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    let saved = layout(&f).await;
    assert_eq!(
        typography::stored(saved.pages[1].texts[0].style_json.as_deref()).unwrap(),
        Some(Direction::Rotate90)
    );
    assert_eq!(
        serde_json::to_value(f.db.execute_publication(request).await.unwrap()).unwrap(),
        serde_json::to_value(receipt).unwrap()
    );
    assert_eq!(layout(&f).await, saved);
    let version =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    let frozen =
        f.db.publication_version(&f.project, &f.publication, &version)
            .await
            .unwrap();
    f.db.execute_publication(
        f.request(PublicationAction::UpdateTextGeometry(TextGeometry {
            text_id: data.text_id.clone(),
            page_id: data.page_id.clone(),
            x_mm: 25.,
            y_mm: 35.,
            width_mm: 10.,
            height_mm: 150.,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(
        layout(&f).await.pages[1].texts[0].style_json,
        saved.pages[1].texts[0].style_json
    );
    data.text_direction = Some(Direction::VerticalRl);
    data.text = "夜行摄影书".into();
    f.db.execute_publication(f.request(PublicationAction::UpdateText(data)).await)
        .await
        .unwrap();
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &version)
            .await
            .unwrap()
            .layout,
        frozen.layout
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: version,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, saved);
    let restarted = Store::open(&f.temp.path().join("publication.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        restarted
            .publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        saved
    );
    assert_eq!(
        saved.pages.iter().map(|p| &p.frames).collect::<Vec<_>>(),
        before.pages.iter().map(|p| &p.frames).collect::<Vec<_>>()
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn legacy_text_update_preserves_stored_direction_and_old_plain_requests_do_not_gain_a_default()
 {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    let mut data = text(&before.pages[1].id);
    data.text_direction = Some(Direction::VerticalLr);
    f.db.execute_publication(f.request(PublicationAction::AddText(data.clone())).await)
        .await
        .unwrap();
    data.text_direction = None;
    data.text = "旧客户端修改内容".into();
    assert!(
        serde_json::to_value(&data)
            .unwrap()
            .get("textDirection")
            .is_none()
    );
    f.db.execute_publication(f.request(PublicationAction::UpdateText(data.clone())).await)
        .await
        .unwrap();
    assert_eq!(
        typography::stored(layout(&f).await.pages[1].texts[0].style_json.as_deref()).unwrap(),
        Some(Direction::VerticalLr)
    );
    data.text_direction = Some(Direction::Horizontal);
    f.db.execute_publication(f.request(PublicationAction::UpdateText(data.clone())).await)
        .await
        .unwrap();
    assert_eq!(
        typography::stored(layout(&f).await.pages[1].texts[0].style_json.as_deref()).unwrap(),
        Some(Direction::Horizontal)
    );
    data.text_id = domain::id();
    data.page_id = before.pages[2].id.clone();
    data.text_direction = None;
    f.db.execute_publication(f.request(PublicationAction::AddText(data)).await)
        .await
        .unwrap();
    assert!(
        typography::stored(layout(&f).await.pages[2].texts[0].style_json.as_deref())
            .unwrap()
            .is_none()
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn direction_updates_obey_hidden_inherited_locks_scopes_permissions_and_batch_rollback() {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    let data = text(&before.pages[1].id);
    f.db.execute_publication(f.request(PublicationAction::AddText(data.clone())).await)
        .await
        .unwrap();
    let before = layout(&f).await;
    let mut tree = layers::read(&before).unwrap();
    let group = domain::id();
    tree.nodes
        .iter_mut()
        .find(|n| n.id == data.text_id)
        .unwrap()
        .parent_id = Some(group.clone());
    tree.nodes.push(layers::Node {
        id: group.clone(),
        kind: "group".into(),
        parent_id: None,
        name: Some("书脊文字".into()),
        visible: false,
        locked: true,
    });
    f.db.execute_publication(
        f.request(PublicationAction::SetPublicationLayers(tree.clone()))
            .await,
    )
    .await
    .unwrap();
    let locked = layout(&f).await;
    let mut changed = data.clone();
    changed.text_direction = Some(Direction::Rotate270);
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::UpdateText(changed.clone()))
                .await
        )
        .await
        .is_err()
    );
    assert_eq!(layout(&f).await, locked);
    tree.nodes
        .iter_mut()
        .find(|n| n.id == group)
        .unwrap()
        .locked = false;
    f.db.execute_publication(
        f.request(PublicationAction::SetPublicationLayers(tree))
            .await,
    )
    .await
    .unwrap();
    let before = layout(&f).await;
    let mut bad = changed.clone();
    bad.page_id = before.pages[2].id.clone();
    let mut request = f
        .request(PublicationAction::UpdateText(changed.clone()))
        .await;
    request.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::UpdateText(bad),
    });
    assert!(f.db.execute_publication(request).await.is_err());
    assert_eq!(layout(&f).await, before);
    assert!(
        CommandHandler::new(&f.db)
            .execute_publication(
                f.request(PublicationAction::UpdateText(changed)).await,
                &CommandContext::read_only(&f.project)
            )
            .await
            .is_err()
    );
    assert_eq!(layout(&f).await, before);
    f.originals_unchanged();
}
#[test]
fn all_five_directions_are_strict_and_absent_is_distinct_from_null() {
    let data = text(&domain::id());
    for direction in [
        "horizontal",
        "vertical-rl",
        "vertical-lr",
        "rotate-90",
        "rotate-270",
    ] {
        let mut raw = serde_json::to_value(&data).unwrap();
        raw["textDirection"] = json!(direction);
        let decoded: TextUpdate = serde_json::from_value(raw).unwrap();
        assert!(decoded.text_direction.is_some());
    }
    for direction in [
        Value::Null,
        json!("diagonal"),
        json!(90),
        json!(""),
        json!({"mode":"horizontal"}),
    ] {
        let mut raw = serde_json::to_value(&data).unwrap();
        raw["textDirection"] = direction;
        assert!(serde_json::from_value::<TextUpdate>(raw).is_err());
    }
    let mut raw = serde_json::to_value(data).unwrap();
    raw.as_object_mut().unwrap().remove("textDirection");
    let decoded: TextUpdate = serde_json::from_value(raw.clone()).unwrap();
    assert!(decoded.text_direction.is_none());
    assert_eq!(serde_json::to_value(decoded).unwrap(), raw);
    assert!(typography::stored(Some("{\"textDirection\":null}")).is_err());
    assert!(typography::stored(Some("{\"textDirection\":\"diagonal\"}")).is_err());
}
#[tokio::test]
async fn master_preview_and_apply_preserve_direction_without_rasterizing_or_changing_source_text() {
    let f = Fixture::new().await;
    let initial = layout(&f).await;
    let source = initial.pages[1].id.clone();
    let target = initial.pages[2].id.clone();
    let data = text(&source);
    f.db.execute_publication(f.request(PublicationAction::AddText(data.clone())).await)
        .await
        .unwrap();
    let master = domain::id();
    f.db.execute_publication(
        f.request(PublicationAction::SaveMaster(SaveMaster {
            master_id: master.clone(),
            name: "书脊文字排法".into(),
            source_page_id: source.clone(),
            expected_master_revision: None,
            include_text: true,
            include_guides: None,
        }))
        .await,
    )
    .await
    .unwrap();
    let before = layout(&f).await;
    let command = ApplyMaster {
        master_id: master,
        master_revision: 1,
        page_ids: vec![target.clone()],
        instance_id: domain::id(),
        overwrite_local: false,
    };
    let predicted =
        f.db.publication_master_preview(
            &f.project,
            &f.publication,
            f.db.publication_revision(&f.project, &f.publication)
                .await
                .unwrap(),
            &command,
        )
        .await
        .unwrap();
    assert_eq!(layout(&f).await, before);
    f.db.execute_publication(f.request(PublicationAction::ApplyMaster(command)).await)
        .await
        .unwrap();
    let after = layout(&f).await;
    assert_eq!(after, predicted.layout);
    let target = after.pages.iter().find(|p| p.id == target).unwrap();
    assert_eq!(target.texts.len(), 1);
    assert_eq!(
        target.texts[0].style_json,
        before.pages[1].texts[0].style_json
    );
    assert_eq!(
        target.texts[0].content_json,
        before.pages[1].texts[0].content_json
    );
    assert_ne!(target.texts[0].id, data.text_id);
    f.originals_unchanged();
}
#[tokio::test]
async fn corrupted_direction_requires_valid_history_restore_and_late_history_failure_rolls_back() {
    let f = Fixture::new().await;
    let initial = layout(&f).await;
    let mut data = text(&initial.pages[1].id);
    f.db.execute_publication(f.request(PublicationAction::AddText(data.clone())).await)
        .await
        .unwrap();
    let before = layout(&f).await;
    let version =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    sqlx::query("CREATE TRIGGER reject_typography_history BEFORE INSERT ON publication_versions WHEN NEW.kind='working' BEGIN SELECT RAISE(FAIL,'synthetic typography history failure'); END").execute(&f.db.pool).await.unwrap();
    data.text_direction = Some(Direction::Rotate270);
    assert!(
        f.db.execute_publication(f.request(PublicationAction::UpdateText(data.clone())).await)
            .await
            .is_err()
    );
    assert_eq!(layout(&f).await, before);
    sqlx::query("DROP TRIGGER reject_typography_history")
        .execute(&f.db.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE publication_texts SET style_json=? WHERE id=?")
        .bind("{\"textDirection\":\"diagonal\"}")
        .bind(&data.text_id)
        .execute(&f.db.pool)
        .await
        .unwrap();
    assert!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .is_err()
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: version,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, before);
    f.originals_unchanged();
}
