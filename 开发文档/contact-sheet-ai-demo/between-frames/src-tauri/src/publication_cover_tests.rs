use super::*;
async fn layout(f: &Fixture) -> versions::PublicationLayout {
    f.db.publication_layout(&f.project, &f.publication)
        .await
        .unwrap()
}
fn data(layout: &versions::PublicationLayout) -> cover::SetCover {
    let page = layout.pages.iter().find(|p| p.page_number == 0).unwrap();
    cover::SetCover {
        page_id: page.id.clone(),
        back_page_id: layout
            .pages
            .iter()
            .find(|p| p.page_number == -1)
            .map(|p| p.id.clone()),
        settings: cover::Cover {
            panel_width_mm: 190.,
            spine_width_mm: 10.,
        },
        height_mm: 257.,
        margin_mm: 15.,
        bleed_mm: 3.,
        content_placement: "followPanels".into(),
        clear_page_settings_confirmed: false,
    }
}
#[tokio::test]
async fn cover_spread_matches_preview_replays_restores_sources_and_survives_restart() {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    let saved =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    let command = data(&before);
    let predicted = cover::plan(&before, &command).unwrap();
    let request = f.request(PublicationAction::SetCover(command)).await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    let after = layout(&f).await;
    assert_eq!(after, predicted);
    assert_eq!(after.pages[0].width_mm, 390.);
    assert_eq!(
        after.pages[0].frames[0].x_mm,
        before.pages[0].frames[0].x_mm + 200.
    );
    assert_eq!(
        after.pages[0].frames[0].source,
        before.pages[0].frames[0].source
    );
    assert_eq!(&after.pages[1..], &before.pages[1..]);
    assert!(receipt.changed);
    assert_eq!(
        serde_json::to_value(f.db.execute_publication(request).await.unwrap()).unwrap(),
        serde_json::to_value(receipt).unwrap()
    );
    assert_eq!(layout(&f).await, after);
    let restarted = Store::open(&f.temp.path().join("publication.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        restarted
            .publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        after
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: saved,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, before);
    f.originals_unchanged();
}
#[tokio::test]
async fn back_cover_merge_keeps_existing_frame_identity_and_rejects_implicit_or_foreign_back() {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    let back = domain::id();
    let now = domain::timestamp();
    sqlx::query("INSERT INTO publication_pages(id,publication_id,page_number,side,width_mm,height_mm,margin_mm,bleed_mm,created_at,updated_at) VALUES(?,?,-1,'back',190,257,15,3,?,?)").bind(&back).bind(&f.publication).bind(&now).bind(&now).execute(&f.db.pool).await.unwrap();
    let frame = &before.pages[1].frames[0];
    sqlx::query("UPDATE publication_frames SET page_id=? WHERE id=?")
        .bind(&back)
        .bind(&frame.id)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let before = layout(&f).await;
    let command = data(&before);
    let mut implicit = command.clone();
    implicit.back_page_id = None;
    assert!(
        f.db.execute_publication(f.request(PublicationAction::SetCover(implicit)).await)
            .await
            .is_err()
    );
    assert_eq!(layout(&f).await, before);
    let mut foreign = command.clone();
    foreign.back_page_id = Some(domain::id());
    assert!(cover::plan(&before, &foreign).is_err());
    let predicted = cover::plan(&before, &command).unwrap();
    f.db.execute_publication(f.request(PublicationAction::SetCover(command)).await)
        .await
        .unwrap();
    let after = layout(&f).await;
    assert_eq!(after, predicted);
    assert!(!after.pages.iter().any(|p| p.id == back));
    assert_eq!(
        after.pages[0]
            .frames
            .iter()
            .find(|x| x.id == frame.id)
            .unwrap(),
        frame
    );
    assert_eq!(&after.pages[1..], &before.pages[1..before.pages.len() - 1]);
    f.originals_unchanged();
}
#[tokio::test]
async fn cover_resize_follows_panels_but_cross_boundary_content_requires_explicit_fixed_positions()
{
    let f = Fixture::new().await;
    let command = data(&layout(&f).await);
    f.db.execute_publication(
        f.request(PublicationAction::SetCover(command.clone()))
            .await,
    )
    .await
    .unwrap();
    let before = layout(&f).await;
    let mut next = command.clone();
    next.settings.panel_width_mm = 200.;
    next.settings.spine_width_mm = 20.;
    let predicted = cover::plan(&before, &next).unwrap();
    assert_eq!(
        predicted.pages[0].frames[0].x_mm,
        before.pages[0].frames[0].x_mm + 20.
    );
    f.db.execute_publication(f.request(PublicationAction::SetCover(next.clone())).await)
        .await
        .unwrap();
    assert_eq!(layout(&f).await, predicted);
    let mut across = f.frame().await;
    across.x_mm = 10.;
    across.width_mm = 400.;
    f.db.execute_publication(f.request(PublicationAction::UpdateFrame(across)).await)
        .await
        .unwrap();
    let before = layout(&f).await;
    next.settings.spine_width_mm = 30.;
    assert!(
        f.db.execute_publication(f.request(PublicationAction::SetCover(next.clone())).await)
            .await
            .is_err()
    );
    assert_eq!(layout(&f).await, before);
    next.content_placement = "keepPosition".into();
    f.db.execute_publication(f.request(PublicationAction::SetCover(next.clone())).await)
        .await
        .unwrap();
    let after = layout(&f).await;
    assert_eq!(after.pages[0].frames, before.pages[0].frames);
    assert!(
        f.db.execute_publication(f.request(PublicationAction::SetCover(next)).await)
            .await
            .is_err()
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn cover_conversion_requires_explicit_page_settings_replacement_and_hidden_locks_block_it() {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    sqlx::query("UPDATE publication_pages SET layout_json=? WHERE id=?")
        .bind("{\"legacy\":true}")
        .bind(&before.pages[0].id)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let before = layout(&f).await;
    let mut command = data(&before);
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::SetCover(command.clone()))
                .await
        )
        .await
        .is_err()
    );
    assert_eq!(layout(&f).await, before);
    command.clear_page_settings_confirmed = true;
    let mut tree = layers::read(&before).unwrap();
    let node = tree
        .nodes
        .iter_mut()
        .find(|n| n.id == before.pages[0].frames[0].id)
        .unwrap();
    node.locked = true;
    node.visible = false;
    f.db.execute_publication(
        f.request(PublicationAction::SetPublicationLayers(tree.clone()))
            .await,
    )
    .await
    .unwrap();
    let locked = layout(&f).await;
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::SetCover(command.clone()))
                .await
        )
        .await
        .is_err()
    );
    assert_eq!(layout(&f).await, locked);
    tree.nodes
        .iter_mut()
        .find(|n| n.id == before.pages[0].frames[0].id)
        .unwrap()
        .locked = false;
    f.db.execute_publication(
        f.request(PublicationAction::SetPublicationLayers(tree))
            .await,
    )
    .await
    .unwrap();
    f.db.execute_publication(f.request(PublicationAction::SetCover(command)).await)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(layout(&f).await.pages[0].layout_json.as_ref().unwrap())
            .unwrap(),
        json!({"focalroomCoverV1":{"panelWidthMm":190.,"spineWidthMm":10.}})
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn cover_scope_strict_fields_bounds_stale_revision_and_permissions_do_not_partially_write() {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    let command = data(&before);
    for mutate in 0..5 {
        let mut bad = command.clone();
        match mutate {
            0 => bad.page_id = before.pages[1].id.clone(),
            1 => bad.settings.panel_width_mm = 1000.,
            2 => bad.height_mm = 50.,
            3 => bad.content_placement = "automatic".into(),
            _ => bad.content_placement = "keepPosition".into(),
        };
        assert!(
            f.db.execute_publication(f.request(PublicationAction::SetCover(bad)).await)
                .await
                .is_err()
        );
        assert_eq!(layout(&f).await, before);
    }
    let mut shape = serde_json::to_value(&command).unwrap();
    shape.as_object_mut().unwrap().remove("backPageId");
    assert!(serde_json::from_value::<cover::SetCover>(shape).is_err());
    let request = f
        .request(PublicationAction::SetCover(command.clone()))
        .await;
    f.db.execute_publication(
        f.request(PublicationAction::CaptureVersion(VersionLabel {
            label: "基线".into(),
        }))
        .await,
    )
    .await
    .unwrap();
    let current = layout(&f).await;
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "REVISION_CONFLICT"
    );
    assert_eq!(layout(&f).await, current);
    assert!(
        CommandHandler::new(&f.db)
            .execute_publication(
                f.request(PublicationAction::SetCover(command)).await,
                &CommandContext::read_only(&f.project)
            )
            .await
            .is_err()
    );
    assert_eq!(layout(&f).await, current);
}
#[tokio::test]
async fn cover_late_history_failure_rolls_back_geometry_and_damaged_current_cover_can_restore() {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    let saved =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    sqlx::query("CREATE TRIGGER reject_cover_history BEFORE INSERT ON publication_versions WHEN NEW.kind='working' BEGIN SELECT RAISE(FAIL,'synthetic cover history failure'); END").execute(&f.db.pool).await.unwrap();
    assert!(
        f.db.execute_publication(f.request(PublicationAction::SetCover(data(&before))).await)
            .await
            .is_err()
    );
    assert_eq!(layout(&f).await, before);
    sqlx::query("DROP TRIGGER reject_cover_history")
        .execute(&f.db.pool)
        .await
        .unwrap();
    f.db.execute_publication(f.request(PublicationAction::SetCover(data(&before))).await)
        .await
        .unwrap();
    let mut corrupt = layout(&f).await;
    corrupt.pages[0].width_mm += 1.;
    assert!(versions::validate_layout(&corrupt).is_err());
    sqlx::query("UPDATE publication_pages SET width_mm=width_mm+1 WHERE id=?")
        .bind(&corrupt.pages[0].id)
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
            version_id: saved,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, before);
    f.originals_unchanged();
}
#[tokio::test]
async fn empty_photobook_can_create_a_real_blank_cover_and_regeneration_clears_cover_metadata() {
    let mut f = Fixture::new().await;
    let before = layout(&f).await;
    let mut command = data(&before);
    f.publication =
        f.db.create_publication(
            &f.project,
            &CreatePublicationCommand {
                title: "空摄影书".into(),
                pub_type: None,
                width_mm: Some(190.),
                height_mm: Some(257.),
            },
        )
        .await
        .unwrap()
        .id;
    command.page_id = domain::id();
    let empty = layout(&f).await;
    assert!(empty.pages.is_empty());
    f.db.execute_publication(f.request(PublicationAction::SetCover(command)).await)
        .await
        .unwrap();
    let actual = layout(&f).await;
    assert_eq!(actual.pages.len(), 1);
    assert!(actual.pages[0].frames.is_empty());
    assert!(cover::read(&actual.pages[0]).unwrap().is_some());
    f.db.execute_publication(
        f.request(PublicationAction::Generate(GeneratePublication {
            photos_per_page: 1,
            replace_existing: true,
        }))
        .await,
    )
    .await
    .unwrap();
    let generated = layout(&f).await;
    assert!(cover::read(&generated.pages[0]).unwrap().is_none());
    assert_eq!(generated.pages[0].width_mm, 190.);
    f.originals_unchanged();
}
