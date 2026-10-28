use super::*;
use chapters::{Chapter, CreateChapter, SetChapters};
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
        let original = layout(&f).await;
        let base = &original.pages[0];
        f.db.execute_publication(
            f.request(PublicationAction::AddPage(AddPage {
                page_id: domain::id(),
                after_page_id: None,
                width_mm: base.width_mm,
                height_mm: base.height_mm,
                margin_mm: base.margin_mm,
                bleed_mm: base.bleed_mm,
            }))
            .await,
        )
        .await
        .unwrap();
    }
    f
}
fn definitions(l: &versions::PublicationLayout) -> Vec<Chapter> {
    vec![
        Chapter {
            id: domain::id(),
            title: "夜行".into(),
            start_page_id: l
                .pages
                .iter()
                .find(|p| p.page_number == 1)
                .unwrap()
                .id
                .clone(),
        },
        Chapter {
            id: domain::id(),
            title: "归途".into(),
            start_page_id: l
                .pages
                .iter()
                .find(|p| p.page_number == 3)
                .unwrap()
                .id
                .clone(),
        },
    ]
}
async fn set(f: &Fixture, chapters: Vec<Chapter>) {
    let l = layout(f).await;
    f.db.execute_publication(
        f.request(PublicationAction::SetChapters(SetChapters {
            chapters,
            page_ids: l.pages.iter().map(|p| p.id.clone()).collect(),
        }))
        .await,
    )
    .await
    .unwrap();
}
fn opening(l: &versions::PublicationLayout, after: String, count: usize) -> CreateChapter {
    let base = &l.pages[0];
    let page_id = domain::id();
    let blank_id = domain::id();
    let at = l.pages.iter().position(|p| p.id == after).unwrap() + 1;
    let second = l.pages[..at].iter().filter(|p| p.page_number > 0).count() % 2 == 1 && count == 2;
    let first = if second {
        blank_id.clone()
    } else {
        page_id.clone()
    };
    let next = if second { page_id.clone() } else { blank_id };
    let add = |id, after| AddPage {
        page_id: id,
        after_page_id: Some(after),
        width_mm: base.width_mm,
        height_mm: base.height_mm,
        margin_mm: base.margin_mm,
        bleed_mm: base.bleed_mm,
    };
    let mut pages = vec![add(first.clone(), after)];
    if count == 2 {
        pages.push(add(next, first));
    }
    CreateChapter {
        chapter: Chapter {
            id: domain::id(),
            title: "第三章".into(),
            start_page_id: page_id.clone(),
        },
        pages,
        start_on_right: true,
        title: TextUpdate {
            text_id: domain::id(),
            page_id,
            x_mm: 15.,
            y_mm: 83.1,
            width_mm: 160.,
            height_mm: 79.45,
            kind: "text".into(),
            text: "第三章".into(),
            text_direction: None,
            font_family: "serif".into(),
            font_size_pt: 28.,
            line_height: 1.4,
            align: "left".into(),
            bold: false,
            color: "#222222".into(),
        },
    }
}
#[tokio::test]
async fn chapter_ranges_reorder_whole_pages_and_restore_stable_identity_after_restart() {
    let f = fixture().await;
    let initial = layout(&f).await;
    let definitions = definitions(&initial);
    set(&f, definitions.clone()).await;
    let saved = layout(&f).await;
    assert_eq!(saved.schema_version, 3);
    assert_eq!(saved.pages, initial.pages);
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    let frozen =
        f.db.publication_version(&f.project, &f.publication, &history[0].id)
            .await
            .unwrap();
    let mut ids = vec![saved.pages[0].id.clone()];
    ids.extend(saved.pages[3..].iter().map(|p| p.id.clone()));
    ids.extend(saved.pages[1..3].iter().map(|p| p.id.clone()));
    let request = f
        .request(PublicationAction::SetChapters(SetChapters {
            chapters: definitions.clone(),
            page_ids: ids.clone(),
        }))
        .await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(
        f.db.execute_publication(request).await.unwrap().revision,
        receipt.revision
    );
    let moved = layout(&f).await;
    assert_eq!(
        moved.pages.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
        ids
    );
    assert_eq!(chapters::read(&moved).unwrap()[0].id, definitions[1].id);
    for p in &initial.pages {
        let actual = moved.pages.iter().find(|n| n.id == p.id).unwrap();
        assert_eq!(actual.frames, p.frames);
        assert_eq!(actual.texts, p.texts);
    }
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: history[0].id.clone(),
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
        f.db.publication_version(&f.project, &f.publication, &history[0].id)
            .await
            .unwrap()
            .layout,
        frozen.layout
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn individual_page_moves_require_exact_command_consent_and_start_pages_cannot_be_removed() {
    let f = fixture().await;
    let l = layout(&f).await;
    let definitions = definitions(&l);
    set(&f, definitions.clone()).await;
    let saved = layout(&f).await;
    let mut ids = saved.pages.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
    ids.swap(2, 3);
    let old = f
        .request(PublicationAction::ReorderPages(PageOrder {
            page_ids: ids.clone(),
            chapter_changes_confirmed: None,
        }))
        .await;
    let raw = serde_json::to_value(&old).unwrap();
    assert!(
        raw["operations"][0]["action"]["data"]
            .get("chapterChangesConfirmed")
            .is_none()
    );
    assert_eq!(
        f.db.execute_publication(old.clone())
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    assert_eq!(layout(&f).await, saved);
    let mut null = raw;
    null["operations"][0]["action"]["data"]["chapterChangesConfirmed"] = Value::Null;
    assert!(serde_json::from_value::<PublicationCommandRequest>(null).is_err());
    let remove = f
        .request(PublicationAction::RemovePage(PageReference {
            page_id: definitions[0].start_page_id.clone(),
        }))
        .await;
    assert_eq!(
        f.db.execute_publication(remove).await.unwrap_err().code,
        "CONFLICT"
    );
    assert_eq!(layout(&f).await, saved);
    f.db.execute_publication(
        f.request(PublicationAction::ReorderPages(PageOrder {
            page_ids: ids,
            chapter_changes_confirmed: Some(true),
        }))
        .await,
    )
    .await
    .unwrap();
    assert_ne!(layout(&f).await.pages, saved.pages);
    f.originals_unchanged();
}
#[tokio::test]
async fn editable_opening_pages_title_and_chapter_are_one_atomic_replayable_command() {
    let f = fixture().await;
    let before = layout(&f).await;
    let data = opening(&before, before.pages[1].id.clone(), 2);
    let request = f
        .request(PublicationAction::CreateChapter(data.clone()))
        .await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(
        f.db.execute_publication(request).await.unwrap().revision,
        receipt.revision
    );
    let after = layout(&f).await;
    assert_eq!(after.pages.len(), before.pages.len() + 2);
    let start = after
        .pages
        .iter()
        .find(|p| p.id == data.chapter.start_page_id)
        .unwrap();
    assert_eq!(start.side, "right");
    assert_eq!(start.page_number, 3);
    assert_eq!(start.texts.len(), 1);
    assert_eq!(start.texts[0].id, data.title.text_id);
    assert_eq!(
        serde_json::from_str::<Value>(&start.texts[0].content_json).unwrap()["text"],
        "第三章"
    );
    assert_eq!(chapters::read(&after).unwrap(), vec![data.chapter.clone()]);
    assert_eq!(
        layers::read(&after)
            .unwrap()
            .nodes
            .iter()
            .filter(|n| n.id == data.title.text_id)
            .count(),
        1
    );
    for p in &before.pages {
        assert_eq!(
            after.pages.iter().find(|n| n.id == p.id).unwrap().frames,
            p.frames
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
    f.originals_unchanged();
}
#[tokio::test]
async fn invalid_opening_right_parity_foreign_anchor_and_title_mismatch_roll_back_everything() {
    let f = fixture().await;
    let before = layout(&f).await;
    let base = opening(&before, before.pages[1].id.clone(), 1);
    let mut foreign = base.clone();
    foreign.pages[0].after_page_id = Some(domain::id());
    let mut mismatch = base.clone();
    mismatch.start_on_right = false;
    mismatch.title.text = "不一致".into();
    let mut duplicate = base.clone();
    duplicate.start_on_right = false;
    duplicate.chapter.id = duplicate.title.text_id.clone();
    for data in [base, foreign, mismatch, duplicate] {
        let history =
            f.db.publication_versions(&f.project, &f.publication, None)
                .await
                .unwrap();
        assert!(
            f.db.execute_publication(f.request(PublicationAction::CreateChapter(data)).await)
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
    }
}
#[tokio::test]
async fn chapter_boundaries_cannot_split_continuous_images_in_either_operation_order() {
    let f = fixture().await;
    let l = layout(&f).await;
    let left = l.pages.iter().find(|p| p.page_number == 2).unwrap();
    let right = l.pages.iter().find(|p| p.page_number == 3).unwrap();
    let frame_id = domain::id();
    let photo = f.db.get(&f.project).await.unwrap().photos[0].id.clone();
    f.db.execute_publication(
        f.request(PublicationAction::AddFrame(PhotoFrameAdd {
            frame_id: frame_id.clone(),
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
        frame_id: frame_id.clone(),
        left_page_id: left.id.clone(),
        right_page_id: right.id.clone(),
        geometry: spreads::FrameGeometry {
            x_mm: 40.,
            y_mm: 20.,
            width_mm: 250.,
            height_mm: 100.,
            fit_mode: "fit".into(),
            crop_x: 0.5,
            crop_y: 0.5,
            crop_zoom: 1.,
        },
    };
    let chapter = Chapter {
        id: domain::id(),
        title: "归途".into(),
        start_page_id: right.id.clone(),
    };
    set(&f, vec![chapter.clone()]).await;
    let before = layout(&f).await;
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::SetSpreadFrame(data.clone()))
                .await
        )
        .await
        .is_err()
    );
    assert_eq!(layout(&f).await, before);
    set(&f, vec![]).await;
    f.db.execute_publication(f.request(PublicationAction::SetSpreadFrame(data)).await)
        .await
        .unwrap();
    let before = layout(&f).await;
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::SetChapters(SetChapters {
                chapters: vec![chapter],
                page_ids: before.pages.iter().map(|p| p.id.clone()).collect()
            }))
            .await
        )
        .await
        .is_err()
    );
    assert_eq!(layout(&f).await, before);
}
#[tokio::test]
async fn corrupt_chapter_metadata_blocks_edits_but_valid_history_recovers_and_regeneration_clears_stale_anchors()
 {
    let f = fixture().await;
    set(&f, definitions(&layout(&f).await)).await;
    let before = layout(&f).await;
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    let mut raw: Value = serde_json::from_str(&before.publication.spec_json).unwrap();
    raw[chapters::KEY] = Value::Null;
    sqlx::query("UPDATE publications SET spec_json=? WHERE id=?")
        .bind(raw.to_string())
        .bind(&f.publication)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let bad = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "坏数据".into(),
        }))
        .await;
    assert!(f.db.execute_publication(bad).await.is_err());
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: history[0].id.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, before);
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::Generate(GeneratePublication {
                photos_per_page: 3,
                replace_existing: true
            }))
            .await
        )
        .await
        .is_err()
    );
    assert_eq!(layout(&f).await, before);
    f.db.execute_publication(
        f.request(PublicationAction::Generate(GeneratePublication {
            photos_per_page: 1,
            replace_existing: true,
        }))
        .await,
    )
    .await
    .unwrap();
    assert!(chapters::read(&layout(&f).await).unwrap().is_empty());
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &history[0].id)
            .await
            .unwrap()
            .layout,
        before
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn invalid_chapter_shapes_and_id_collisions_cannot_create_partial_metadata() {
    let f = fixture().await;
    let before = layout(&f).await;
    let mut base = definitions(&before);
    let mut repeated = base.clone();
    repeated[1].start_page_id = repeated[0].start_page_id.clone();
    let mut collision = base.clone();
    collision[0].id = before.pages[0].id.clone();
    let mut cover = base.clone();
    cover[0].start_page_id = before.pages[0].id.clone();
    let mut compact = base.clone();
    compact[0].id = compact[0].id.replace('-', "");
    let mut unsupported = base.clone();
    unsupported[0].id = "00000000-0000-0000-8000-000000000001".into();
    base[0].title = "\n".into();
    for data in [base, repeated, collision, cover, compact, unsupported] {
        assert!(
            f.db.execute_publication(
                f.request(PublicationAction::SetChapters(SetChapters {
                    chapters: data,
                    page_ids: before.pages.iter().map(|p| p.id.clone()).collect()
                }))
                .await
            )
            .await
            .is_err()
        );
        assert_eq!(layout(&f).await, before);
    }
}

#[tokio::test]
async fn legacy_page_number_gaps_use_the_same_preview_and_persisted_order() {
    let f = fixture().await;
    let initial = layout(&f).await;
    sqlx::query("UPDATE publication_pages SET page_number=8 WHERE id=?")
        .bind(&initial.pages.last().unwrap().id)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let before = layout(&f).await;
    let ids = before
        .pages
        .iter()
        .map(|p| p.id.clone())
        .collect::<Vec<_>>();
    let expected = chapters::ordered_layout(&before, &ids).unwrap();
    set(&f, definitions(&before)).await;
    assert_eq!(layout(&f).await.pages, expected.pages);
    f.originals_unchanged();
}
