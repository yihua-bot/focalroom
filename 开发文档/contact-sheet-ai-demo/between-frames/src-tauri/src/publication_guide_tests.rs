use super::*;
use guides::{PageGuides, SetPageGuides};
fn settings() -> PageGuides {
    PageGuides {
        columns: 3,
        rows: 4,
        column_gap_mm: 5.,
        row_gap_mm: 4.,
        vertical_guides_mm: vec![20.25, 100.],
        horizontal_guides_mm: vec![30., 80.],
    }
}
async fn layout(f: &Fixture) -> versions::PublicationLayout {
    f.db.publication_layout(&f.project, &f.publication)
        .await
        .unwrap()
}
#[tokio::test]
async fn page_guides_preserve_content_metadata_history_and_replay_after_restart() {
    let f = Fixture::new().await;
    let first = layout(&f).await;
    let page = first.pages[0].id.clone();
    sqlx::query("UPDATE publication_pages SET layout_json=? WHERE id=?")
        .bind("{\"legacy\":{\"preserve\":[1,2]}}")
        .bind(&page)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let before = layout(&f).await;
    let saved = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "参考线前".into(),
        }))
        .await;
    let old_id = saved.command_id.clone();
    f.db.execute_publication(saved).await.unwrap();
    let request = f
        .request(PublicationAction::SetPageGuides(SetPageGuides {
            page_id: page,
            settings: settings(),
        }))
        .await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    let after = layout(&f).await;
    assert_eq!(guides::read(&after.pages[0]).unwrap(), Some(settings()));
    assert_eq!(after.publication, before.publication);
    assert_eq!(after.pages[0].frames, before.pages[0].frames);
    assert_eq!(after.pages[0].texts, before.pages[0].texts);
    assert_eq!(after.pages[1..], before.pages[1..]);
    assert_eq!(
        serde_json::from_str::<Value>(after.pages[0].layout_json.as_ref().unwrap()).unwrap()["legacy"],
        json!({"preserve":[1,2]})
    );
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&f.db.execute_publication(request.clone()).await.unwrap()).unwrap()
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
    assert_eq!(
        encode(&reopened.execute_publication(request).await.unwrap()).unwrap(),
        encode(&receipt).unwrap()
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: old_id.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, before);
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &old_id)
            .await
            .unwrap()
            .layout,
        before
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn invalid_guides_and_foreign_page_roll_back_without_new_revision_or_history() {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    let mut bad = Vec::new();
    let mut s = settings();
    s.columns = 0;
    bad.push(s);
    let mut s = settings();
    s.rows = 25;
    bad.push(s);
    let mut s = settings();
    s.column_gap_mm = 100.;
    bad.push(s);
    let mut s = settings();
    s.vertical_guides_mm = vec![10., 10.];
    bad.push(s);
    let mut s = settings();
    s.horizontal_guides_mm = vec![300.];
    bad.push(s);
    let mut s = settings();
    s.vertical_guides_mm = (0..33).map(f64::from).collect();
    bad.push(s);
    let mut s = settings();
    s.row_gap_mm = -1.;
    bad.push(s);
    for settings in bad {
        let request = f
            .request(PublicationAction::SetPageGuides(SetPageGuides {
                page_id: before.pages[0].id.clone(),
                settings,
            }))
            .await;
        assert_eq!(
            f.db.execute_publication(request).await.unwrap_err().code,
            "VALIDATION_ERROR"
        );
        assert_eq!(layout(&f).await, before);
    }
    let foreign = f
        .request(PublicationAction::SetPageGuides(SetPageGuides {
            page_id: domain::id(),
            settings: settings(),
        }))
        .await;
    assert!(f.db.execute_publication(foreign).await.is_err());
    assert_eq!(
        encode(
            &f.db
                .publication_versions(&f.project, &f.publication, None)
                .await
                .unwrap()
        )
        .unwrap(),
        encode(&history).unwrap()
    );
    let mut value = json!(settings());
    value["source"] = json!("foreign");
    assert!(serde_json::from_value::<PageGuides>(value).is_err());
    f.originals_unchanged();
}
#[tokio::test]
async fn guide_writes_obey_human_revision_and_page_resize_cannot_silently_drop_lines() {
    let f = Fixture::new().await;
    let mut frame = f.frame().await;
    frame.width_mm -= 20.;
    f.db.execute_publication(f.request(PublicationAction::UpdateFrame(frame)).await)
        .await
        .unwrap();
    let first = layout(&f).await;
    let page = &first.pages[0];
    let mut data = settings();
    data.vertical_guides_mm = vec![page.width_mm - 1.];
    let request = f
        .request(PublicationAction::SetPageGuides(SetPageGuides {
            page_id: page.id.clone(),
            settings: data,
        }))
        .await;
    assert_eq!(
        CommandHandler::new(&f.db)
            .execute_publication(request.clone(), &CommandContext::read_only(&f.project))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    let stale = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "过期".into(),
        }))
        .await;
    f.db.execute_publication(request).await.unwrap();
    assert_eq!(
        f.db.execute_publication(stale).await.unwrap_err().code,
        "REVISION_CONFLICT"
    );
    let saved = layout(&f).await;
    let resize = f
        .request(PublicationAction::UpdatePage(PageUpdate {
            page_id: page.id.clone(),
            width_mm: page.width_mm - 2.,
            height_mm: page.height_mm,
            margin_mm: page.margin_mm,
            bleed_mm: page.bleed_mm,
        }))
        .await;
    assert_eq!(
        f.db.execute_publication(resize).await.unwrap_err().code,
        "VALIDATION_ERROR"
    );
    assert_eq!(layout(&f).await, saved);
    f.originals_unchanged();
}
#[tokio::test]
async fn corrupt_current_guide_settings_require_explicit_valid_history_restore() {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    let saved = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "有效参考线".into(),
        }))
        .await;
    let id = saved.command_id.clone();
    f.db.execute_publication(saved).await.unwrap();
    sqlx::query("UPDATE publication_pages SET layout_json=? WHERE id=?")
        .bind("{\"focalroomPageGuidesV1\":null}")
        .bind(&before.pages[0].id)
        .execute(&f.db.pool)
        .await
        .unwrap();
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap_err()
            .code,
        "VALIDATION_ERROR"
    );
    let ordinary = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "不能自动修复".into(),
        }))
        .await;
    assert!(f.db.execute_publication(ordinary).await.is_err());
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: id,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, before);
    f.originals_unchanged();
}

#[tokio::test]
async fn existing_large_unknown_metadata_is_readable_but_guide_write_cannot_truncate_it() {
    let f = Fixture::new().await;
    let original = layout(&f).await;
    let raw = json!({"legacy":"汉".repeat(20000)}).to_string();
    sqlx::query("UPDATE publication_pages SET layout_json=? WHERE id=?")
        .bind(&raw)
        .bind(&original.pages[0].id)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let before = layout(&f).await;
    assert!(guides::read(&before.pages[0]).unwrap().is_none());
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    let request = f
        .request(PublicationAction::SetPageGuides(SetPageGuides {
            page_id: original.pages[0].id.clone(),
            settings: settings(),
        }))
        .await;
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "VALIDATION_ERROR"
    );
    assert_eq!(layout(&f).await, before);
    assert_eq!(
        encode(
            &f.db
                .publication_versions(&f.project, &f.publication, None)
                .await
                .unwrap()
        )
        .unwrap(),
        encode(&history).unwrap()
    );
    assert_eq!(before.pages[0].layout_json.as_ref().unwrap(), &raw);
    f.originals_unchanged();
}
