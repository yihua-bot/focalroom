use super::*;
use layers::{Layers, Node};
async fn fixture() -> Fixture {
    let f = Fixture::new().await;
    let page =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap()
            .pages[0]
            .id
            .clone();
    f.db.execute_publication(
        f.request(PublicationAction::AddText(TextUpdate {
            text_id: domain::id(),
            page_id: page,
            x_mm: 15.,
            y_mm: 20.,
            width_mm: 100.,
            height_mm: 20.,
            kind: "text".into(),
            text: "保留正文".into(),
            text_direction: None,
            font_family: "serif".into(),
            font_size_pt: 12.,
            line_height: 1.4,
            align: "left".into(),
            bold: false,
            color: "#112233".into(),
        }))
        .await,
    )
    .await
    .unwrap();
    f
}
async fn layout(f: &Fixture) -> versions::PublicationLayout {
    f.db.publication_layout(&f.project, &f.publication)
        .await
        .unwrap()
}
async fn save(f: &Fixture, data: Layers) {
    f.db.execute_publication(
        f.request(PublicationAction::SetPublicationLayers(data))
            .await,
    )
    .await
    .unwrap();
}
fn grouping(l: &versions::PublicationLayout) -> Layers {
    let mut tree = layers::read(l).unwrap();
    let group = domain::id();
    let frame = &l.pages[0].frames[0].id;
    let text = &l.pages[0].texts[0].id;
    for n in &mut tree.nodes {
        if &n.id == frame || &n.id == text {
            n.parent_id = Some(group.clone());
        }
    }
    tree.nodes.push(Node {
        id: group,
        kind: "group".into(),
        parent_id: None,
        name: Some("封面组合".into()),
        visible: true,
        locked: false,
    });
    tree
}
fn frame_change(l: &versions::PublicationLayout) -> FrameUpdate {
    let f = &l.pages[0].frames[0];
    FrameUpdate {
        frame_id: f.id.clone(),
        x_mm: f.x_mm + 1.,
        y_mm: f.y_mm,
        width_mm: f.width_mm - 1.,
        height_mm: f.height_mm,
        fit_mode: f.fit_mode.clone(),
        asset_id: f.asset_id.clone(),
        rendition_id: f.rendition_id.clone(),
        crop_x: f.crop_x,
        crop_y: f.crop_y,
        crop_zoom: f.crop_zoom,
    }
}
#[tokio::test]
async fn layers_and_nested_groups_are_versioned_idempotent_restorable_and_restart_safe() {
    let f = fixture().await;
    let original = layout(&f).await;
    let mut data = grouping(&original);
    let group = data.nodes.last().unwrap().id.clone();
    let parent = domain::id();
    data.nodes.last_mut().unwrap().parent_id = Some(parent.clone());
    data.nodes.push(Node {
        id: parent,
        kind: "group".into(),
        parent_id: None,
        name: Some("章节标题".into()),
        visible: false,
        locked: true,
    });
    let request = f
        .request(PublicationAction::SetPublicationLayers(data.clone()))
        .await;
    let version = request.command_id.clone();
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&f.db.execute_publication(request).await.unwrap()).unwrap()
    );
    let frozen = layout(&f).await;
    assert_eq!(frozen.schema_version, 3);
    assert_eq!(layers::read(&frozen).unwrap(), data);
    assert_eq!(
        layers::effective(&data, &original.pages[0].frames[0].id).unwrap(),
        (false, true)
    );
    assert_eq!(frozen.pages, original.pages);
    assert!(
        f.db.publication_pages(&f.project, &f.publication)
            .await
            .is_err()
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: version.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, frozen);
    let reopened = Store::open(&f.temp.path().join("publication.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        frozen
    );
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &version)
            .await
            .unwrap()
            .layout,
        frozen
    );
    assert!(
        layers::read(&frozen)
            .unwrap()
            .nodes
            .iter()
            .any(|n| n.id == group)
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn malformed_foreign_missing_duplicate_cross_page_cycle_and_deep_groups_roll_back() {
    let f = fixture().await;
    let original = layout(&f).await;
    let base = grouping(&original);
    let mut cases = Vec::new();
    let mut bad = base.clone();
    bad.nodes[0].id = domain::id();
    cases.push(bad);
    let mut bad = base.clone();
    bad.nodes.remove(0);
    cases.push(bad);
    let mut bad = base.clone();
    bad.nodes.push(bad.nodes[0].clone());
    cases.push(bad);
    let mut bad = base.clone();
    let id = bad.nodes.last().unwrap().id.clone();
    bad.nodes.last_mut().unwrap().parent_id = Some(id);
    cases.push(bad);
    let mut bad = base.clone();
    bad.nodes.last_mut().unwrap().name = Some(" ".into());
    cases.push(bad);
    let mut bad = base.clone();
    bad.nodes.last_mut().unwrap().id = original.pages[0].id.clone();
    cases.push(bad);
    let mut bad = base.clone();
    let group = bad.nodes.last().unwrap().id.clone();
    let other = original.pages[1].frames[0].id.clone();
    bad.nodes
        .iter_mut()
        .find(|n| n.id == other)
        .unwrap()
        .parent_id = Some(group);
    cases.push(bad);
    let mut bad = base.clone();
    for _ in 0..8 {
        let id = domain::id();
        bad.nodes.last_mut().unwrap().parent_id = Some(id.clone());
        bad.nodes.push(Node {
            id,
            kind: "group".into(),
            parent_id: None,
            name: None,
            visible: true,
            locked: false,
        });
    }
    cases.push(bad);
    let history = encode(
        &f.db
            .publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap(),
    )
    .unwrap();
    for data in cases {
        assert!(
            f.db.execute_publication(
                f.request(PublicationAction::SetPublicationLayers(data))
                    .await
            )
            .await
            .is_err()
        );
        assert_eq!(layout(&f).await, original);
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
    f.originals_unchanged();
}
#[tokio::test]
async fn inherited_lock_is_enforced_across_geometry_source_deletion_generate_and_bulk_rollback() {
    let f = fixture().await;
    let initial = layout(&f).await;
    let mut data = grouping(&initial);
    data.nodes.last_mut().unwrap().locked = true;
    save(&f, data.clone()).await;
    let before = layout(&f).await;
    let text = &before.pages[0].texts[0];
    let actions = vec![
        PublicationAction::UpdateFrame(frame_change(&before)),
        PublicationAction::RemoveFrame(FrameReference {
            frame_id: before.pages[0].frames[0].id.clone(),
        }),
        PublicationAction::RemoveText(TextReference {
            text_id: text.id.clone(),
        }),
        PublicationAction::UpdateTextGeometry(TextGeometry {
            text_id: text.id.clone(),
            page_id: before.pages[0].id.clone(),
            x_mm: 16.,
            y_mm: 20.,
            width_mm: 100.,
            height_mm: 20.,
        }),
        PublicationAction::Generate(GeneratePublication {
            photos_per_page: 1,
            replace_existing: true,
        }),
    ];
    for action in actions {
        assert_eq!(
            f.db.execute_publication(f.request(action).await)
                .await
                .unwrap_err()
                .code,
            "CONFLICT"
        );
        assert_eq!(layout(&f).await, before);
    }

    let photo = f.db.get(&f.project).await.unwrap().photos[1].id.clone();
    assert_eq!(
        f.db.execute_publication(
            f.request(PublicationAction::SetFramePhoto(FramePhoto {
                frame_id: before.pages[0].frames[0].id.clone(),
                photo_id: photo,
                rendition_id: None
            }))
            .await
        )
        .await
        .unwrap_err()
        .code,
        "CONFLICT"
    );
    let mut partial = frame_change(&before);
    let other = &before.pages[1].frames[0];
    partial.frame_id = other.id.clone();
    partial.asset_id = other.asset_id.clone();
    partial.rendition_id = other.rendition_id.clone();
    let mut bulk = f.request(PublicationAction::UpdateFrame(partial)).await;
    bulk.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::RemoveText(TextReference {
            text_id: text.id.clone(),
        }),
    });
    assert!(f.db.execute_publication(bulk).await.is_err());
    assert_eq!(layout(&f).await, before);
    data.nodes.last_mut().unwrap().locked = false;
    save(&f, data).await;
    f.db.execute_publication(
        f.request(PublicationAction::UpdateFrame(frame_change(&before)))
            .await,
    )
    .await
    .unwrap();
    assert_ne!(layout(&f).await.pages[0].frames, before.pages[0].frames);
    f.originals_unchanged();
}
#[tokio::test]
async fn deletion_prunes_empty_groups_and_new_content_gets_one_default_layer() {
    let f = fixture().await;
    let initial = layout(&f).await;
    let grouping_request = f
        .request(PublicationAction::SetPublicationLayers(grouping(&initial)))
        .await;
    let group_version = grouping_request.command_id.clone();
    f.db.execute_publication(grouping_request).await.unwrap();
    let first = layout(&f).await;
    let mut request = f
        .request(PublicationAction::RemoveFrame(FrameReference {
            frame_id: first.pages[0].frames[0].id.clone(),
        }))
        .await;
    request.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::RemoveText(TextReference {
            text_id: first.pages[0].texts[0].id.clone(),
        }),
    });
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&f.db.execute_publication(request).await.unwrap()).unwrap()
    );
    let deleted = layout(&f).await;
    assert!(
        layers::read(&deleted)
            .unwrap()
            .nodes
            .iter()
            .all(|n| n.kind != "group")
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: group_version.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, first);
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &group_version)
            .await
            .unwrap()
            .layout,
        first
    );
    let photo = f.db.get(&f.project).await.unwrap().photos[1].id.clone();
    let id = domain::id();
    f.db.execute_publication(
        f.request(PublicationAction::AddFrame(PhotoFrameAdd {
            frame_id: id.clone(),
            page_id: first.pages[0].id.clone(),
            photo_id: photo,
            rendition_id: None,
            x_mm: 15.,
            y_mm: 15.,
            width_mm: 100.,
            height_mm: 100.,
        }))
        .await,
    )
    .await
    .unwrap();
    let nodes = layers::read(&layout(&f).await).unwrap();
    let matches: Vec<_> = nodes.nodes.iter().filter(|n| n.id == id).collect();
    assert_eq!(matches.len(), 1);
    assert!(matches[0].visible);
    assert!(!matches[0].locked);
    f.originals_unchanged();
}

#[tokio::test]
async fn master_application_preserves_layers_and_lock_checks_are_shared_with_preview() {
    let f = fixture().await;
    let first = layout(&f).await;
    let data = grouping(&first);
    save(&f, data.clone()).await;
    let master = domain::id();
    f.db.execute_publication(
        f.request(PublicationAction::SaveMaster(SaveMaster {
            master_id: master.clone(),
            name: "封面母版".into(),
            source_page_id: first.pages[0].id.clone(),
            expected_master_revision: None,
            include_text: true,
            include_guides: None,
        }))
        .await,
    )
    .await
    .unwrap();
    let apply = ApplyMaster {
        master_id: master,
        master_revision: 1,
        page_ids: vec![first.pages[0].id.clone()],
        instance_id: domain::id(),
        overwrite_local: true,
    };
    let baseline = layout(&f).await;
    let revision =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap()
            .revision;
    let preview =
        f.db.publication_master_preview(&f.project, &f.publication, revision, &apply)
            .await
            .unwrap();
    assert_eq!(layers::read(&preview.layout).unwrap(), data);
    f.db.execute_publication(
        f.request(PublicationAction::ApplyMaster(apply.clone()))
            .await,
    )
    .await
    .unwrap();
    assert_eq!(layers::read(&layout(&f).await).unwrap(), data);
    let mut locked = data.clone();
    locked.nodes.last_mut().unwrap().locked = true;
    save(&f, locked).await;
    let before = layout(&f).await;
    let revision =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap()
            .revision;
    assert!(
        f.db.publication_master_preview(&f.project, &f.publication, revision, &apply)
            .await
            .is_err()
    );
    assert!(
        f.db.execute_publication(f.request(PublicationAction::ApplyMaster(apply)).await)
            .await
            .is_err()
    );
    assert_eq!(layout(&f).await, before);
    assert_eq!(
        baseline.pages[0].frames[0].source,
        before.pages[0].frames[0].source
    );
    f.originals_unchanged();
}

#[tokio::test]
async fn damaged_layer_encoding_can_be_recovered_only_by_an_explicit_valid_history_restore() {
    let f = fixture().await;
    let initial = layout(&f).await;
    save(&f, grouping(&initial)).await;
    let frozen = layout(&f).await;
    let version =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    for raw in [
        "{broken",
        "{\"focalroomLayersV1\":null}",
        "{\"focalroomLayersV1\":{\"nodes\":[{\"unexpected\":true}]}}",
    ] {
        let request = f
            .request(PublicationAction::RestoreVersion(VersionReference {
                version_id: version.clone(),
            }))
            .await;
        sqlx::query("UPDATE publications SET spec_json=? WHERE id=?")
            .bind(raw)
            .bind(&f.publication)
            .execute(&f.db.pool)
            .await
            .unwrap();
        assert!(
            f.db.publication_document(&f.project, &f.publication)
                .await
                .is_err()
        );
        f.db.execute_publication(request).await.unwrap();
        assert_eq!(layout(&f).await, frozen);
        assert_eq!(
            f.db.publication_version(&f.project, &f.publication, &version)
                .await
                .unwrap()
                .layout,
            frozen
        );
    }
    f.originals_unchanged();
}
