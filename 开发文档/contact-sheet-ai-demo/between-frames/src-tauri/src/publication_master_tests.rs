use super::*;
async fn save(f: &Fixture, source: &str, master: &str, expected: Option<i64>) {
    f.db.execute_publication(
        f.request(PublicationAction::SaveMaster(SaveMaster {
            master_id: master.into(),
            name: "留白与页码".into(),
            source_page_id: source.into(),
            expected_master_revision: expected,
            include_text: true,
            include_guides: None,
        }))
        .await,
    )
    .await
    .unwrap();
}
fn apply(master: &str, page: &str, revision: i64) -> ApplyMaster {
    ApplyMaster {
        master_id: master.into(),
        master_revision: revision,
        page_ids: vec![page.into()],
        instance_id: domain::id(),
        overwrite_local: false,
    }
}
#[tokio::test]
async fn master_preview_is_pure_apply_matches_stable_ids_and_history_can_restore_everything() {
    let f = Fixture::new().await;
    let initial =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let source = initial.pages[1].id.clone();
    let target = initial.pages[2].id.clone();
    let old =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    let text = TextUpdate {
        text_id: domain::id(),
        page_id: source.clone(),
        x_mm: 15.,
        y_mm: 230.,
        width_mm: 100.,
        height_mm: 10.,
        kind: "page_number".into(),
        text: String::new(),
        text_direction: None,
        font_family: "serif".into(),
        font_size_pt: 10.,
        line_height: 1.4,
        align: "center".into(),
        bold: false,
        color: "#222222".into(),
    };
    f.db.execute_publication(f.request(PublicationAction::AddText(text)).await)
        .await
        .unwrap();
    let master = domain::id();
    save(&f, &source, &master, None).await;
    let before =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    let data = apply(&master, &target, 1);
    let preview =
        f.db.publication_master_preview(&f.project, &f.publication, before.revision, &data)
            .await
            .unwrap();
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before.layout
    );
    assert!(preview.overridden_page_ids.is_empty());
    let request = f
        .request(PublicationAction::ApplyMaster(data.clone()))
        .await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&f.db.execute_publication(request).await.unwrap()).unwrap()
    );
    let after =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(after.layout, preview.layout);
    assert_eq!(after.layout.pages[2].frames.len(), 2);
    assert_eq!(after.layout.pages[2].texts.len(), 1);
    assert!(
        after.layout.pages[2]
            .frames
            .iter()
            .any(|frame| frame.asset_id.is_none())
    );
    assert_eq!(
        after.layout.pages[2]
            .frames
            .iter()
            .filter_map(|frame| frame.asset_id.clone())
            .collect::<Vec<_>>(),
        before.layout.pages[2]
            .frames
            .iter()
            .filter_map(|frame| frame.asset_id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(after.layout.pages[0], before.layout.pages[0]);
    assert!(
        !after
            .master_states
            .iter()
            .find(|s| s.page_id == target)
            .unwrap()
            .locally_changed
    );
    let reopened = Store::open(&f.temp.path().join("publication.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        after.layout
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: old,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        initial
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn local_edits_require_explicit_overwrite_and_master_updates_do_not_modify_other_pages() {
    let f = Fixture::new().await;
    let layout =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let source = layout.pages[1].id.clone();
    let target = layout.pages[2].id.clone();
    let master = domain::id();
    save(&f, &source, &master, None).await;
    f.db.execute_publication(
        f.request(PublicationAction::ApplyMaster(apply(&master, &target, 1)))
            .await,
    )
    .await
    .unwrap();
    let doc =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    let frame = doc.layout.pages[2]
        .frames
        .iter()
        .find(|f| f.asset_id.is_some())
        .unwrap();
    f.db.execute_publication(
        f.request(PublicationAction::UpdateFrame(FrameUpdate {
            frame_id: frame.id.clone(),
            x_mm: frame.x_mm + 1.,
            y_mm: frame.y_mm,
            width_mm: frame.width_mm,
            height_mm: frame.height_mm,
            fit_mode: frame.fit_mode.clone(),
            crop_x: frame.crop_x,
            crop_y: frame.crop_y,
            crop_zoom: frame.crop_zoom,
            asset_id: frame.asset_id.clone(),
            rendition_id: frame.rendition_id.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    let manual =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    assert!(
        manual
            .master_states
            .iter()
            .find(|s| s.page_id == target)
            .unwrap()
            .locally_changed
    );
    save(&f, &source, &master, Some(1)).await;
    let updated =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(updated.layout.pages[2], manual.layout.pages[2]);
    let state = updated
        .master_states
        .iter()
        .find(|s| s.page_id == target)
        .unwrap();
    assert_eq!(state.current_revision, Some(2));
    assert_eq!(state.applied_revision, 1);
    let mut data = apply(&master, &target, 2);
    let preview =
        f.db.publication_master_preview(&f.project, &f.publication, updated.revision, &data)
            .await
            .unwrap();
    assert_eq!(preview.overridden_page_ids, vec![target.clone()]);
    assert_eq!(
        f.db.execute_publication(
            f.request(PublicationAction::ApplyMaster(data.clone()))
                .await
        )
        .await
        .unwrap_err()
        .code,
        "CONFLICT"
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        updated.layout
    );
    data.overwrite_local = true;
    f.db.execute_publication(f.request(PublicationAction::ApplyMaster(data)).await)
        .await
        .unwrap();
    let after =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    assert!(
        !after
            .master_states
            .iter()
            .find(|s| s.page_id == target)
            .unwrap()
            .locally_changed
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn filling_master_slots_preserves_geometry_link_and_source_versions() {
    let f = Fixture::new().await;
    let layout =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let source = layout.pages[1].id.clone();
    let target = layout.pages[2].id.clone();
    let master = domain::id();
    save(&f, &source, &master, None).await;
    f.db.execute_publication(
        f.request(PublicationAction::ApplyMaster(apply(&master, &target, 1)))
            .await,
    )
    .await
    .unwrap();
    let before =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    let empty = before.layout.pages[2]
        .frames
        .iter()
        .find(|f| f.asset_id.is_none())
        .unwrap();
    let photo = f.db.get(&f.project).await.unwrap().photos[0].id.clone();
    f.db.execute_publication(
        f.request(PublicationAction::SetFramePhoto(FramePhoto {
            frame_id: empty.id.clone(),
            photo_id: photo,
            rendition_id: None,
        }))
        .await,
    )
    .await
    .unwrap();
    let after =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    let filled = after.layout.pages[2]
        .frames
        .iter()
        .find(|f| f.id == empty.id)
        .unwrap();
    assert!(filled.asset_id.is_some());
    assert_eq!(filled.x_mm, empty.x_mm);
    assert_eq!(filled.width_mm, empty.width_mm);
    assert!(
        !after
            .master_states
            .iter()
            .find(|s| s.page_id == target)
            .unwrap()
            .locally_changed
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn invalid_targets_stale_master_and_insufficient_slots_never_discard_photos() {
    let f = Fixture::new().await;
    let layout =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let source = layout.pages[0].id.clone();
    let master = domain::id();
    save(&f, &source, &master, None).await;
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    for data in [
        apply(&master, &layout.pages[1].id, 1),
        apply(&master, &domain::id(), 1),
        apply(&master, &layout.pages[2].id, 99),
        ApplyMaster {
            page_ids: vec![source.clone(), source.clone()],
            ..apply(&master, &source, 1)
        },
    ] {
        assert!(
            f.db.execute_publication(f.request(PublicationAction::ApplyMaster(data)).await)
                .await
                .is_err()
        );
        assert_eq!(
            f.db.publication_layout(&f.project, &f.publication)
                .await
                .unwrap(),
            before
        );
    }
    let request = f
        .request(PublicationAction::ApplyMaster(apply(
            &master,
            &layout.pages[2].id,
            1,
        )))
        .await;
    assert_eq!(
        CommandHandler::new(&f.db)
            .execute_publication(request, &CommandContext::human("foreign-project"))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::SaveMaster(SaveMaster {
                master_id: master,
                name: "错误覆盖".into(),
                source_page_id: source,
                expected_master_revision: Some(99),
                include_text: false,
                include_guides: None,
            }))
            .await
        )
        .await
        .is_err()
    );
}
