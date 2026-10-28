use super::*;
use spreads::{FrameGeometry, SetSpreadFrame, UnsetSpreadFrame};

async fn fixture() -> Fixture {
    let f = Fixture::new().await;
    f.db.execute_publication(
        f.request(PublicationAction::Generate(GeneratePublication {
            photos_per_page: 1,
            replace_existing: true,
        }))
        .await,
    )
    .await
    .unwrap();
    f
}
async fn link_data(f: &Fixture) -> SetSpreadFrame {
    let layout =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    SetSpreadFrame {
        frame_id: layout.pages[3].frames[0].id.clone(),
        left_page_id: layout.pages[2].id.clone(),
        right_page_id: layout.pages[3].id.clone(),
        geometry: FrameGeometry {
            x_mm: 15.,
            y_mm: 20.,
            width_mm: 350.,
            height_mm: 210.,
            fit_mode: "fill".into(),
            crop_x: 0.3,
            crop_y: 0.7,
            crop_zoom: 1.4,
        },
    }
}
async fn make(f: &Fixture) -> SetSpreadFrame {
    let data = link_data(f).await;
    f.db.execute_publication(
        f.request(PublicationAction::SetSpreadFrame(data.clone()))
            .await,
    )
    .await
    .unwrap();
    data
}
fn update(frame: &versions::LayoutFrame) -> FrameUpdate {
    FrameUpdate {
        frame_id: frame.id.clone(),
        x_mm: frame.x_mm,
        y_mm: frame.y_mm,
        width_mm: frame.width_mm,
        height_mm: frame.height_mm,
        fit_mode: frame.fit_mode.clone(),
        crop_x: frame.crop_x,
        crop_y: frame.crop_y,
        crop_zoom: frame.crop_zoom,
        asset_id: frame.asset_id.clone(),
        rendition_id: frame.rendition_id.clone(),
    }
}

#[tokio::test]
async fn continuous_frame_has_one_canonical_source_immutable_history_replay_and_restart() {
    let f = fixture().await;
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let project = f.db.get(&f.project).await.unwrap();
    let data = link_data(&f).await;
    let request = f
        .request(PublicationAction::SetSpreadFrame(data.clone()))
        .await;
    let id = request.command_id.clone();
    let first = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(
        encode(&first).unwrap(),
        encode(&f.db.execute_publication(request).await.unwrap()).unwrap()
    );
    let after =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(after.schema_version, 2);
    assert_eq!(before.schema_version, 1);
    assert_eq!(
        after
            .pages
            .iter()
            .map(|page| page.frames.len())
            .sum::<usize>(),
        before
            .pages
            .iter()
            .map(|page| page.frames.len())
            .sum::<usize>()
    );
    assert_eq!(after.pages[3].frames.len(), 0);
    let frame = after.pages[2]
        .frames
        .iter()
        .find(|frame| frame.id == data.frame_id)
        .unwrap();
    assert_eq!(frame.source, before.pages[3].frames[0].source);
    assert_eq!(frame.asset_id, before.pages[3].frames[0].asset_id);
    assert_eq!(frame.rendition_id, before.pages[3].frames[0].rendition_id);
    assert_eq!((frame.width_mm, frame.crop_zoom), (350., 1.4));
    assert_eq!(
        spreads::visible(&after, &after.pages[3]).unwrap()[0].id,
        frame.id
    );
    assert_eq!(
        spreads::links(&after.pages[2]).unwrap()[0].right_page_id,
        after.pages[3].id
    );
    let frozen =
        f.db.publication_version(&f.project, &f.publication, &id)
            .await
            .unwrap();
    assert_eq!(frozen.layout, after);
    assert_eq!(
        frozen
            .previews
            .iter()
            .filter(|preview| preview.frame_id == data.frame_id)
            .count(),
        1
    );
    assert!(
        f.db.publication_pages(&f.project, &f.publication)
            .await
            .is_err()
    );
    let mut moved = update(frame);
    moved.x_mm = 5.;
    moved.width_mm = 365.;
    moved.crop_zoom = 1.8;
    f.db.execute_publication(f.request(PublicationAction::UpdateFrame(moved)).await)
        .await
        .unwrap();
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &id)
            .await
            .unwrap()
            .layout,
        after
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: id,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        after
    );
    let reopened = Store::open(&f.temp.path().join("publication.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        after
    );
    let unchanged = f.db.get(&f.project).await.unwrap();
    assert_eq!(
        encode(&unchanged.photos).unwrap(),
        encode(&project.photos).unwrap()
    );
    assert_eq!(unchanged.sequence, project.sequence);
    f.originals_unchanged();
}

#[tokio::test]
async fn malformed_pair_geometry_scope_and_topology_roll_back_the_whole_command() {
    let f = fixture().await;
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let data = link_data(&f).await;
    let history = encode(
        &f.db
            .publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap(),
    )
    .unwrap();
    let mut cases = Vec::new();
    let mut bad = data.clone();
    bad.left_page_id = before.pages[1].id.clone();
    cases.push(bad);
    let mut bad = data.clone();
    bad.right_page_id = before.pages[0].id.clone();
    cases.push(bad);
    let mut bad = data.clone();
    bad.frame_id = before.pages[0].frames[0].id.clone();
    cases.push(bad);
    let mut bad = data.clone();
    bad.geometry.width_mm = 100.;
    cases.push(bad);
    let mut bad = data.clone();
    bad.geometry.crop_zoom = f64::NAN;
    cases.push(bad);
    let mut bad = data.clone();
    bad.geometry.width_mm = 1000.;
    cases.push(bad);
    for data in cases {
        assert!(
            f.db.execute_publication(f.request(PublicationAction::SetSpreadFrame(data)).await)
                .await
                .is_err()
        );
        assert_eq!(
            f.db.publication_layout(&f.project, &f.publication)
                .await
                .unwrap(),
            before
        );
        assert_eq!(
            encode(
                &f.db
                    .publication_versions(&f.project, &f.publication, None)
                    .await
                    .unwrap()
            )
            .unwrap(),
            history
        );
    }
    make(&f).await;
    let linked =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    for page in [&data.left_page_id, &data.right_page_id] {
        assert!(
            f.db.execute_publication(
                f.request(PublicationAction::RemovePage(PageReference {
                    page_id: page.clone()
                }))
                .await
            )
            .await
            .is_err()
        );
        assert_eq!(
            f.db.publication_layout(&f.project, &f.publication)
                .await
                .unwrap(),
            linked
        );
    }
    let mut ids: Vec<_> = linked.pages.iter().map(|page| page.id.clone()).collect();
    ids.swap(1, 2);
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::ReorderPages(PageOrder {
                page_ids: ids,
                chapter_changes_confirmed: None
            }))
            .await
        )
        .await
        .is_err()
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        linked
    );
    let add = AddPage {
        page_id: domain::id(),
        after_page_id: Some(data.left_page_id),
        width_mm: 190.,
        height_mm: 257.,
        margin_mm: 15.,
        bleed_mm: 3.,
    };
    assert!(
        f.db.execute_publication(f.request(PublicationAction::AddPage(add)).await)
            .await
            .is_err()
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        linked
    );
    f.originals_unchanged();
}

#[tokio::test]
async fn right_projection_participates_in_duplicate_checks_and_unlink_preserves_source_identity() {
    let f = fixture().await;
    let data = make(&f).await;
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let frame = before.pages[2]
        .frames
        .iter()
        .find(|frame| frame.id == data.frame_id)
        .unwrap();
    let photo =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap()
            .previews
            .iter()
            .find(|preview| preview.frame_id == data.frame_id)
            .unwrap()
            .photo_id
            .clone()
            .unwrap();
    let addition = PhotoFrameAdd {
        frame_id: domain::id(),
        page_id: data.right_page_id.clone(),
        photo_id: photo,
        rendition_id: frame.rendition_id.clone(),
        x_mm: 15.,
        y_mm: 15.,
        width_mm: 70.,
        height_mm: 90.,
    };
    assert!(
        f.db.execute_publication(f.request(PublicationAction::AddFrame(addition)).await)
            .await
            .is_err()
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );
    let other_photo =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap()
            .previews
            .iter()
            .find(|preview| preview.frame_id == before.pages[0].frames[0].id)
            .unwrap()
            .photo_id
            .clone()
            .unwrap();
    f.db.execute_publication(
        f.request(PublicationAction::AddFrame(PhotoFrameAdd {
            frame_id: domain::id(),
            page_id: data.right_page_id.clone(),
            photo_id: other_photo.clone(),
            rendition_id: None,
            x_mm: 15.,
            y_mm: 15.,
            width_mm: 70.,
            height_mm: 90.,
        }))
        .await,
    )
    .await
    .unwrap();
    let with_other =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let mut changed = update(frame);
    changed.asset_id = before.pages[0].frames[0].asset_id.clone();
    changed.rendition_id = None;
    assert!(
        f.db.execute_publication(f.request(PublicationAction::UpdateFrame(changed)).await)
            .await
            .is_err()
    );
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::SetFramePhoto(FramePhoto {
                frame_id: data.frame_id.clone(),
                photo_id: other_photo,
                rendition_id: None
            }))
            .await
        )
        .await
        .is_err()
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        with_other
    );
    let local = UnsetSpreadFrame {
        frame_id: data.frame_id.clone(),
        target_page_id: data.right_page_id.clone(),
        geometry: FrameGeometry {
            x_mm: 20.,
            y_mm: 25.,
            width_mm: 140.,
            height_mm: 190.,
            fit_mode: "fit".into(),
            crop_x: 0.5,
            crop_y: 0.5,
            crop_zoom: 1.,
        },
    };
    f.db.execute_publication(f.request(PublicationAction::UnsetSpreadFrame(local)).await)
        .await
        .unwrap();
    let after =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(after.schema_version, 1);
    assert!(spreads::links(&after.pages[2]).unwrap().is_empty());
    let moved = after.pages[3]
        .frames
        .iter()
        .find(|frame| frame.id == data.frame_id)
        .unwrap();
    assert_eq!(moved.source, frame.source);
    assert_eq!((moved.x_mm, moved.width_mm), (20., 140.));
    assert!(
        !after.pages[2]
            .frames
            .iter()
            .any(|frame| frame.id == data.frame_id)
    );
    f.originals_unchanged();
}

#[tokio::test]
async fn masters_preserve_independent_spread_frames_and_legacy_metadata() {
    let f = fixture().await;
    let data = link_data(&f).await;
    sqlx::query("UPDATE publication_pages SET layout_json=? WHERE id=?")
        .bind("{\"legacy\":{\"keep\":[1,2]}}")
        .bind(&data.left_page_id)
        .execute(&f.db.pool)
        .await
        .unwrap();
    f.db.execute_publication(
        f.request(PublicationAction::SetSpreadFrame(data.clone()))
            .await,
    )
    .await
    .unwrap();
    let linked =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let frame = linked.pages[2]
        .frames
        .iter()
        .find(|frame| frame.id == data.frame_id)
        .unwrap()
        .clone();
    let master = domain::id();
    f.db.execute_publication(
        f.request(PublicationAction::SaveMaster(SaveMaster {
            master_id: master.clone(),
            name: "单页框与跨页背景".into(),
            source_page_id: data.left_page_id.clone(),
            expected_master_revision: None,
            include_text: true,
            include_guides: None,
        }))
        .await,
    )
    .await
    .unwrap();
    let saved =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(masters::masters(&saved).unwrap()[0].frames.len(), 1);
    f.db.execute_publication(
        f.request(PublicationAction::ApplyMaster(ApplyMaster {
            master_id: master,
            master_revision: 1,
            page_ids: vec![data.left_page_id.clone(), data.right_page_id.clone()],
            instance_id: domain::id(),
            overwrite_local: false,
        }))
        .await,
    )
    .await
    .unwrap();
    let applied =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(
        *applied.pages[2]
            .frames
            .iter()
            .find(|candidate| candidate.id == data.frame_id)
            .unwrap(),
        frame
    );
    assert_eq!(
        spreads::visible(&applied, &applied.pages[3])
            .unwrap()
            .iter()
            .filter(|candidate| candidate.id == data.frame_id)
            .count(),
        1
    );
    let meta: Value = serde_json::from_str(applied.pages[2].layout_json.as_ref().unwrap()).unwrap();
    assert_eq!(meta["legacy"], json!({"keep":[1,2]}));
    assert!(
        !masters::states(&applied)
            .unwrap()
            .iter()
            .any(|state| state.locally_changed)
    );
    f.originals_unchanged();
}

#[tokio::test]
async fn removal_is_atomic_with_registry_and_restoring_format_two_keeps_one_reference() {
    let f = fixture().await;
    let data = make(&f).await;
    let linked =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let version =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    let mut invalid = f
        .request(PublicationAction::RemoveFrame(FrameReference {
            frame_id: data.frame_id.clone(),
        }))
        .await;
    invalid.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::RemoveText(TextReference {
            text_id: domain::id(),
        }),
    });
    assert!(f.db.execute_publication(invalid).await.is_err());
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        linked
    );
    f.db.execute_publication(
        f.request(PublicationAction::RemoveFrame(FrameReference {
            frame_id: data.frame_id.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    let removed =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(removed.schema_version, 1);
    assert!(spreads::links(&removed.pages[2]).unwrap().is_empty());
    assert!(
        !spreads::visible(&removed, &removed.pages[3])
            .unwrap()
            .iter()
            .any(|frame| frame.id == data.frame_id)
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: version,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        linked
    );
    f.originals_unchanged();
}

#[tokio::test]
async fn linked_pages_resize_insert_and_move_together_as_one_revision() {
    let f = fixture().await;
    let data = make(&f).await;
    let before =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    let new_left = domain::id();
    let new_right = domain::id();
    let mut insert = f
        .request(PublicationAction::AddPage(AddPage {
            page_id: new_left.clone(),
            after_page_id: Some(before.layout.pages[1].id.clone()),
            width_mm: 190.,
            height_mm: 257.,
            margin_mm: 15.,
            bleed_mm: 3.,
        }))
        .await;
    insert.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::AddPage(AddPage {
            page_id: new_right.clone(),
            after_page_id: Some(new_left.clone()),
            width_mm: 190.,
            height_mm: 257.,
            margin_mm: 15.,
            bleed_mm: 3.,
        }),
    });
    let saved = f.db.execute_publication(insert.clone()).await.unwrap();
    assert_eq!(saved.revision, before.revision + 1);
    assert_eq!(
        encode(&saved).unwrap(),
        encode(&f.db.execute_publication(insert).await.unwrap()).unwrap()
    );
    let inserted =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(inserted.pages[4].id, data.left_page_id);
    assert_eq!(inserted.pages[5].id, data.right_page_id);
    let mut order: Vec<_> = inserted.pages.iter().map(|page| page.id.clone()).collect();
    order.splice(
        2..6,
        [
            data.left_page_id.clone(),
            data.right_page_id.clone(),
            new_left,
            new_right,
        ],
    );
    f.db.execute_publication(
        f.request(PublicationAction::ReorderPages(PageOrder {
            chapter_changes_confirmed: None,
            page_ids: order,
        }))
        .await,
    )
    .await
    .unwrap();
    let mut resize = f
        .request(PublicationAction::UpdatePage(PageUpdate {
            page_id: data.left_page_id.clone(),
            width_mm: 200.,
            height_mm: 270.,
            margin_mm: 17.,
            bleed_mm: 4.,
        }))
        .await;
    resize.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::UpdatePage(PageUpdate {
            page_id: data.right_page_id.clone(),
            width_mm: 200.,
            height_mm: 270.,
            margin_mm: 15.,
            bleed_mm: 4.,
        }),
    });
    f.db.execute_publication(resize).await.unwrap();
    let after =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let pair = spreads::frame_pair(&after, &data.frame_id)
        .unwrap()
        .unwrap();
    assert_eq!(pair.0.id, data.left_page_id);
    assert_eq!(pair.1.id, data.right_page_id);
    assert_eq!(pair.0.width_mm, 200.);
    assert_eq!(pair.1.height_mm, 270.);
    assert_eq!(pair.0.margin_mm, 17.);
    assert_eq!(pair.1.margin_mm, 15.);
    assert_eq!(
        after
            .pages
            .iter()
            .flat_map(|p| &p.frames)
            .find(|frame| frame.id == data.frame_id)
            .unwrap()
            .source,
        before
            .layout
            .pages
            .iter()
            .flat_map(|p| &p.frames)
            .find(|frame| frame.id == data.frame_id)
            .unwrap()
            .source
    );
    f.originals_unchanged();
}
