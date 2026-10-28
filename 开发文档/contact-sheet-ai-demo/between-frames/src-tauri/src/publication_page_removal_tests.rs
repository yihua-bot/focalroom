use super::*;
use page_removal::RemovePages;
async fn layout(f: &Fixture) -> versions::PublicationLayout {
    f.db.publication_layout(&f.project, &f.publication)
        .await
        .unwrap()
}
async fn fixture() -> Fixture {
    let f = Fixture::new().await;
    while layout(&f)
        .await
        .pages
        .iter()
        .filter(|p| p.page_number > 0)
        .count()
        < 6
    {
        f.db.execute_publication(
            f.request(PublicationAction::AddPage(AddPage {
                page_id: domain::id(),
                after_page_id: None,
                width_mm: 190.,
                height_mm: 257.,
                margin_mm: 15.,
                bleed_mm: 3.,
            }))
            .await,
        )
        .await
        .unwrap();
    }
    f
}
async fn mark(f: &Fixture, next_start: i64) -> Vec<chapters::Chapter> {
    let l = layout(f).await;
    let defs = vec![
        chapters::Chapter {
            id: domain::id(),
            title: "夜行".into(),
            start_page_id: l.pages[1].id.clone(),
        },
        chapters::Chapter {
            id: domain::id(),
            title: "归途".into(),
            start_page_id: l
                .pages
                .iter()
                .find(|p| p.page_number == next_start)
                .unwrap()
                .id
                .clone(),
        },
    ];
    f.db.execute_publication(
        f.request(PublicationAction::SetChapters(chapters::SetChapters {
            chapters: defs.clone(),
            page_ids: l.pages.iter().map(|p| p.id.clone()).collect(),
        }))
        .await,
    )
    .await
    .unwrap();
    defs
}
async fn spread(f: &Fixture, left_number: i64) -> spreads::SetSpreadFrame {
    let l = layout(f).await;
    let left = l
        .pages
        .iter()
        .find(|p| p.page_number == left_number)
        .unwrap();
    let right = l
        .pages
        .iter()
        .find(|p| p.page_number == left_number + 1)
        .unwrap();
    let id = domain::id();
    let photo = f.db.get(&f.project).await.unwrap().photos[0].id.clone();
    f.db.execute_publication(
        f.request(PublicationAction::AddFrame(PhotoFrameAdd {
            frame_id: id.clone(),
            page_id: left.id.clone(),
            photo_id: photo,
            rendition_id: None,
            x_mm: 20.,
            y_mm: 20.,
            width_mm: 100.,
            height_mm: 80.,
        }))
        .await,
    )
    .await
    .unwrap();
    let data = spreads::SetSpreadFrame {
        frame_id: id,
        left_page_id: left.id.clone(),
        right_page_id: right.id.clone(),
        geometry: spreads::FrameGeometry {
            x_mm: 40.,
            y_mm: 20.,
            width_mm: 250.,
            height_mm: 100.,
            fit_mode: "fill".into(),
            crop_x: 0.2,
            crop_y: 0.7,
            crop_zoom: 1.4,
        },
    };
    f.db.execute_publication(
        f.request(PublicationAction::SetSpreadFrame(data.clone()))
            .await,
    )
    .await
    .unwrap();
    data
}
async fn request(f: &Fixture, data: RemovePages) -> PublicationCommandRequest {
    f.request(PublicationAction::RemovePages(data)).await
}
#[tokio::test]
async fn chapter_page_removal_matches_preview_replays_and_restores_all_ids_after_restart() {
    let f = fixture().await;
    let defs = mark(&f, 3).await;
    let before = layout(&f).await;
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    let frozen =
        f.db.publication_version(&f.project, &f.publication, &history[0].id)
            .await
            .unwrap();
    let data = RemovePages {
        page_ids: page_removal::chapter_pages(&before, &defs[0].id).unwrap(),
        retain_blank_page_id: None,
        chapter_id: Some(defs[0].id.clone()),
    };
    let expected = page_removal::plan(&before, &data).unwrap().layout;
    let req = request(&f, data).await;
    let receipt = f.db.execute_publication(req.clone()).await.unwrap();
    assert!(receipt.changed);
    assert_eq!(
        f.db.execute_publication(req).await.unwrap().revision,
        receipt.revision
    );
    let after = layout(&f).await;
    assert_eq!(after, expected);
    assert_eq!(after.pages.len(), before.pages.len() - 2);
    assert_eq!(chapters::read(&after).unwrap(), vec![defs[1].clone()]);
    for p in &after.pages {
        assert_eq!(
            p.frames,
            before
                .pages
                .iter()
                .find(|old| old.id == p.id)
                .unwrap()
                .frames
        );
    }
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
            version_id: history[0].id.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, before);
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &history[0].id)
            .await
            .unwrap()
            .layout,
        frozen.layout
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn odd_chapter_requires_explicit_blank_to_preserve_downstream_continuous_photo() {
    let f = fixture().await;
    let pair = spread(&f, 4).await;
    let defs = mark(&f, 4).await;
    let before = layout(&f).await;
    let ids = page_removal::chapter_pages(&before, &defs[0].id).unwrap();
    assert_eq!(ids.len(), 3);
    let mut data = RemovePages {
        page_ids: ids.clone(),
        chapter_id: Some(defs[0].id.clone()),
        retain_blank_page_id: None,
    };
    assert!(
        f.db.execute_publication(request(&f, data.clone()).await)
            .await
            .is_err()
    );
    assert_eq!(layout(&f).await, before);
    data.retain_blank_page_id = Some(ids[0].clone());
    let expected = page_removal::plan(&before, &data).unwrap().layout;
    f.db.execute_publication(request(&f, data).await)
        .await
        .unwrap();
    let after = layout(&f).await;
    assert_eq!(after, expected);
    let blank = after.pages.iter().find(|p| p.id == ids[0]).unwrap();
    assert!(blank.frames.is_empty() && blank.texts.is_empty() && blank.layout_json.is_none());
    assert_eq!(blank.width_mm, 190.);
    assert_eq!(
        spreads::validate(&after).unwrap(),
        std::collections::BTreeSet::from([pair.frame_id.clone()])
    );
    assert_eq!(
        after
            .pages
            .iter()
            .find(|p| p.id == pair.left_page_id)
            .unwrap()
            .page_number,
        2
    );
    assert_eq!(
        after
            .pages
            .iter()
            .flat_map(|p| &p.frames)
            .find(|frame| frame.id == pair.frame_id),
        before
            .pages
            .iter()
            .flat_map(|p| &p.frames)
            .find(|frame| frame.id == pair.frame_id)
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn linked_pages_cannot_be_cut_but_complete_pair_removes_one_canonical_photo_and_preserves_others()
 {
    let f = fixture().await;
    let pair = spread(&f, 2).await;
    let before = layout(&f).await;
    assert_eq!(before.schema_version, 2);
    for id in [&pair.left_page_id, &pair.right_page_id] {
        let data = RemovePages {
            page_ids: vec![id.clone()],
            retain_blank_page_id: None,
            chapter_id: None,
        };
        assert!(
            f.db.execute_publication(request(&f, data).await)
                .await
                .is_err()
        );
        assert_eq!(layout(&f).await, before);
    }
    let data = RemovePages {
        page_ids: vec![pair.left_page_id.clone(), pair.right_page_id.clone()],
        retain_blank_page_id: None,
        chapter_id: None,
    };
    let expected = page_removal::plan(&before, &data).unwrap().layout;
    assert_eq!(expected.schema_version, 1);
    f.db.execute_publication(request(&f, data).await)
        .await
        .unwrap();
    let after = layout(&f).await;
    assert_eq!(after, expected);
    assert!(
        !after
            .pages
            .iter()
            .flat_map(|p| &p.frames)
            .any(|frame| frame.id == pair.frame_id)
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn inherited_hidden_locks_block_whole_page_removal_and_empty_groups_are_cleaned_only_after_unlock()
 {
    let f = fixture().await;
    let defs = mark(&f, 3).await;
    let l = layout(&f).await;
    let owner = &l.pages[1];
    let frame = owner.frames[0].id.clone();
    let mut tree = layers::read(&layout(&f).await).unwrap();
    let group = domain::id();
    tree.nodes
        .iter_mut()
        .find(|n| n.id == frame)
        .unwrap()
        .parent_id = Some(group.clone());
    tree.nodes.push(layers::Node {
        id: group.clone(),
        kind: "group".into(),
        parent_id: None,
        name: Some("锁定组".into()),
        visible: false,
        locked: true,
    });
    f.db.execute_publication(
        f.request(PublicationAction::SetPublicationLayers(tree.clone()))
            .await,
    )
    .await
    .unwrap();
    let before = layout(&f).await;
    let data = RemovePages {
        page_ids: page_removal::chapter_pages(&before, &defs[0].id).unwrap(),
        chapter_id: Some(defs[0].id.clone()),
        retain_blank_page_id: None,
    };
    assert_eq!(
        f.db.execute_publication(request(&f, data.clone()).await)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    assert_eq!(layout(&f).await, before);
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
    f.db.execute_publication(request(&f, data).await)
        .await
        .unwrap();
    assert!(
        !layers::read(&layout(&f).await)
            .unwrap()
            .nodes
            .iter()
            .any(|n| n.id == group || n.id == frame)
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn malformed_ranges_and_other_chapter_starts_are_atomic_and_request_flags_are_required() {
    let f = fixture().await;
    let defs = mark(&f, 3).await;
    let before = layout(&f).await;
    let missing = json!({"pageIds":[before.pages[1].id],"chapterId":null});
    assert!(serde_json::from_value::<RemovePages>(missing).is_err());
    let valid = RemovePages {
        page_ids: vec![before.pages[1].id.clone()],
        retain_blank_page_id: None,
        chapter_id: None,
    };
    for data in [
        RemovePages {
            page_ids: vec![],
            ..valid.clone()
        },
        RemovePages {
            page_ids: vec![before.pages[1].id.clone(), before.pages[1].id.clone()],
            ..valid.clone()
        },
        RemovePages {
            page_ids: vec![before.pages[1].id.clone(), before.pages[3].id.clone()],
            ..valid.clone()
        },
        RemovePages {
            page_ids: vec![before.pages[0].id.clone()],
            ..valid.clone()
        },
        RemovePages {
            page_ids: vec![domain::id()],
            ..valid.clone()
        },
        RemovePages {
            retain_blank_page_id: Some(before.pages[2].id.clone()),
            ..valid.clone()
        },
        RemovePages {
            chapter_id: Some(defs[0].id.clone()),
            ..valid.clone()
        },
        valid.clone(),
    ] {
        assert!(
            f.db.execute_publication(request(&f, data).await)
                .await
                .is_err()
        );
        assert_eq!(layout(&f).await, before);
    }
    let req = request(
        &f,
        RemovePages {
            page_ids: page_removal::chapter_pages(&before, &defs[0].id).unwrap(),
            chapter_id: Some(defs[0].id.clone()),
            retain_blank_page_id: None,
        },
    )
    .await;
    assert_eq!(
        CommandHandler::new(&f.db)
            .execute_publication(req, &CommandContext::read_only(&f.project))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    assert_eq!(layout(&f).await, before);
}
#[tokio::test]
async fn clear_to_blank_keeps_page_identity_and_dimensions_but_rejects_empty_noop() {
    let f = fixture().await;
    let before = layout(&f).await;
    let page = &before.pages[1];
    let data = RemovePages {
        page_ids: vec![page.id.clone()],
        chapter_id: None,
        retain_blank_page_id: Some(page.id.clone()),
    };
    f.db.execute_publication(request(&f, data.clone()).await)
        .await
        .unwrap();
    let after = layout(&f).await;
    assert_eq!(after.pages.len(), before.pages.len());
    let blank = after.pages.iter().find(|p| p.id == page.id).unwrap();
    assert!(blank.frames.is_empty() && blank.texts.is_empty());
    assert_eq!(blank.width_mm, page.width_mm);
    assert!(
        f.db.execute_publication(request(&f, data).await)
            .await
            .is_err()
    );
    assert_eq!(layout(&f).await, after);
    f.originals_unchanged();
}

#[tokio::test]
async fn stale_preview_and_late_history_failure_leave_all_pages_and_chapter_metadata_intact() {
    let f = fixture().await;
    let defs = mark(&f, 3).await;
    let before = layout(&f).await;
    let data = RemovePages {
        page_ids: page_removal::chapter_pages(&before, &defs[0].id).unwrap(),
        retain_blank_page_id: None,
        chapter_id: Some(defs[0].id.clone()),
    };
    let stale = request(&f, data.clone()).await;
    f.db.execute_publication(
        f.request(PublicationAction::CaptureVersion(VersionLabel {
            label: "并行保存".into(),
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(
        f.db.execute_publication(stale).await.unwrap_err().code,
        "REVISION_CONFLICT"
    );
    assert_eq!(layout(&f).await, before);
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    sqlx::query("CREATE TRIGGER reject_page_removal_history BEFORE INSERT ON publication_versions WHEN NEW.kind='working' BEGIN SELECT RAISE(FAIL,'synthetic history write failure'); END")
        .execute(&f.db.pool).await.unwrap();
    assert!(
        f.db.execute_publication(request(&f, data.clone()).await)
            .await
            .is_err()
    );
    assert_eq!(layout(&f).await, before);
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history.len()
    );
    sqlx::query("DROP TRIGGER reject_page_removal_history")
        .execute(&f.db.pool)
        .await
        .unwrap();
    f.db.execute_publication(request(&f, data).await)
        .await
        .unwrap();
    assert_eq!(layout(&f).await.pages.len(), before.pages.len() - 2);
    f.originals_unchanged();
}
