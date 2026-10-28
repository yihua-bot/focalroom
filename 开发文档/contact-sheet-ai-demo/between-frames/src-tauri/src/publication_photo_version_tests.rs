use super::*;
use crate::publication_commands::photo_versions::VersionCursor;
use sha2::{Digest, Sha256};

async fn rendition(f: &Fixture, asset: &str, index: usize, missing: bool) -> String {
    let path = f.temp.path().join(format!("version-{index}.jpg"));
    image::RgbImage::from_pixel(
        64,
        48,
        image::Rgb(if index.is_multiple_of(2) {
            [190, 30, 20]
        } else {
            [20, 50, 180]
        }),
    )
    .save(&path)
    .unwrap();
    let prepared = image_pipeline::prepare(&path, &f.temp.path().join("cache")).unwrap();
    let variant = domain::id();
    let id = domain::id();
    sqlx::query("INSERT INTO edit_variants(id,asset_id,name,external_software,is_current,created_at) VALUES(?,?,?,'lightroom_classic',0,'2026-10-25')").bind(&variant).bind(asset).bind(format!("修图 {index}")).execute(&f.db.pool).await.unwrap();
    sqlx::query("INSERT INTO renditions(id,variant_id,sha256,width,height,bit_depth,source_type,file_path,file_size,created_at) VALUES(?,?,?,64,48,8,'external_lightroom',?,?,'2026-10-25')").bind(&id).bind(&variant).bind(if missing{"f".repeat(64)}else{prepared.hash}).bind(path.to_string_lossy().as_ref()).bind(prepared.size as i64).execute(&f.db.pool).await.unwrap();
    id
}

#[tokio::test]
async fn version_catalog_is_read_only_and_selected_source_crop_history_and_restart_stay_fixed() {
    let f = Fixture::new().await;
    let mut frame = f.frame().await;
    frame.crop_zoom = 1.5;
    f.db.execute_publication(
        f.request(PublicationAction::UpdateFrame(frame.clone()))
            .await,
    )
    .await
    .unwrap();
    let asset = frame.asset_id.as_deref().unwrap();
    let a = rendition(&f, asset, 0, false).await;
    let b = rendition(&f, asset, 1, false).await;
    let before =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    let catalog =
        f.db.publication_photo_versions(
            &f.project,
            &f.publication,
            &frame.frame_id,
            before.revision,
            None,
        )
        .await
        .unwrap();
    assert_eq!(catalog.choices.len(), 2);
    assert!(catalog.original.rendition_id.is_none());
    assert!(catalog.current.rendition_id.is_none());
    assert!(
        catalog
            .choices
            .iter()
            .all(|v| v.thumbnail_path.as_ref().unwrap().ends_with(&format!(
                "{}-320.jpg",
                v.source.rendition_sha256.as_ref().unwrap()
            )))
    );
    assert_eq!(
        serde_json::to_value(
            f.db.publication_document(&f.project, &f.publication)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&before).unwrap()
    );
    assert_eq!(
        serde_json::to_value(
            f.db.publication_versions(&f.project, &f.publication, None)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(history).unwrap()
    );
    frame.rendition_id = Some(a.clone());
    let command = f
        .request(PublicationAction::UpdateFrame(frame.clone()))
        .await;
    let version = command.command_id.clone();
    f.db.execute_publication(command.clone()).await.unwrap();
    let saved =
        f.db.publication_version(&f.project, &f.publication, &version)
            .await
            .unwrap();
    let snapshot = serde_json::to_value(&saved).unwrap();
    assert_eq!(saved.layout.pages[0].frames[0].crop_zoom, 1.5);
    assert_eq!(
        saved.layout.pages[0].frames[0].rendition_id.as_deref(),
        Some(a.as_str())
    );
    sqlx::query("UPDATE edit_variants SET is_current=CASE WHEN id=(SELECT variant_id FROM renditions WHERE id=?) THEN 1 ELSE 0 END WHERE asset_id=?").bind(&b).bind(asset).execute(&f.db.pool).await.unwrap();
    let revision =
        f.db.publication_revision(&f.project, &f.publication)
            .await
            .unwrap();
    let catalog = f
        .db
        .publication_photo_versions(&f.project, &f.publication, &frame.frame_id, revision, None)
        .await
        .unwrap();
    assert_eq!(catalog.current.rendition_id.as_deref(), Some(a.as_str()));
    assert_eq!(
        serde_json::to_value(
            f.db.publication_version(&f.project, &f.publication, &version)
                .await
                .unwrap()
        )
        .unwrap(),
        snapshot
    );
    f.db.execute_publication(command).await.unwrap();
    assert_eq!(
        f.db.publication_revision(&f.project, &f.publication)
            .await
            .unwrap(),
        revision
    );
    let reopened = Store::open(&f.temp.path().join("publication.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(
            reopened
                .publication_version(&f.project, &f.publication, &version)
                .await
                .unwrap()
        )
        .unwrap(),
        snapshot
    );
    for (path, sha) in &f.source_hashes {
        assert_eq!(
            format!("{:x}", Sha256::digest(std::fs::read(path).unwrap())),
            *sha
        );
    }
}

#[tokio::test]
async fn version_catalog_paginates_stably_and_rejects_stale_foreign_or_unbound_targets() {
    let f = Fixture::new().await;
    let frame = f.frame().await;
    let asset = frame.asset_id.as_deref().unwrap();
    let mut expected = Vec::new();
    for index in 0..53 {
        expected.push(rendition(&f, asset, index, false).await);
    }
    expected.sort();
    expected.reverse();
    let revision =
        f.db.publication_revision(&f.project, &f.publication)
            .await
            .unwrap();
    let first = f
        .db
        .publication_photo_versions(&f.project, &f.publication, &frame.frame_id, revision, None)
        .await
        .unwrap();
    assert_eq!(first.choices.len(), 50);
    let cursor = first.next.unwrap();
    assert_eq!(cursor.id, expected[49]);
    let second =
        f.db.publication_photo_versions(
            &f.project,
            &f.publication,
            &frame.frame_id,
            revision,
            Some(cursor),
        )
        .await
        .unwrap();
    assert_eq!(second.choices.len(), 3);
    assert!(second.next.is_none());
    let actual: Vec<_> = first
        .choices
        .iter()
        .chain(&second.choices)
        .map(|v| v.rendition_id.clone().unwrap())
        .collect();
    assert_eq!(actual, expected);
    assert!(
        f.db.publication_photo_versions("other", &f.publication, &frame.frame_id, revision, None)
            .await
            .is_err()
    );
    assert!(
        f.db.publication_photo_versions(&f.project, &f.publication, &domain::id(), revision, None)
            .await
            .is_err()
    );
    assert!(
        f.db.publication_photo_versions(
            &f.project,
            &f.publication,
            &frame.frame_id,
            revision - 1,
            None
        )
        .await
        .is_err()
    );
    assert!(
        f.db.publication_photo_versions(
            &f.project,
            &f.publication,
            &frame.frame_id,
            revision,
            Some(VersionCursor {
                created_at: "now".into(),
                id: "not-an-id".into()
            })
        )
        .await
        .is_err()
    );
    let mut empty = frame;
    empty.asset_id = None;
    empty.rendition_id = None;
    f.db.execute_publication(
        f.request(PublicationAction::UpdateFrame(empty.clone()))
            .await,
    )
    .await
    .unwrap();
    assert!(
        f.db.publication_photo_versions(
            &f.project,
            &f.publication,
            &empty.frame_id,
            revision + 1,
            None
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn unavailable_version_never_falls_back_and_switching_cannot_duplicate_a_page_reference() {
    let f = Fixture::new().await;
    let frame = f.frame().await;
    let asset = frame.asset_id.as_deref().unwrap();
    let id = rendition(&f, asset, 0, true).await;
    let revision =
        f.db.publication_revision(&f.project, &f.publication)
            .await
            .unwrap();
    let catalog = f
        .db
        .publication_photo_versions(&f.project, &f.publication, &frame.frame_id, revision, None)
        .await
        .unwrap();
    assert!(catalog.original.thumbnail_path.is_some());
    assert!(catalog.choices[0].thumbnail_path.is_none());
    let layout =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let photo =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap()
            .previews
            .into_iter()
            .find(|p| p.frame_id == frame.frame_id)
            .unwrap()
            .photo_id
            .unwrap();
    let add = PhotoFrameAdd {
        frame_id: domain::id(),
        page_id: layout.pages[0].id.clone(),
        photo_id: photo,
        rendition_id: Some(id),
        x_mm: 15.,
        y_mm: 15.,
        width_mm: 40.,
        height_mm: 50.,
    };
    let added = add.frame_id.clone();
    f.db.execute_publication(f.request(PublicationAction::AddFrame(add)).await)
        .await
        .unwrap();
    let mut change = frame.clone();
    change.frame_id = added;
    change.rendition_id = None;
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(
        f.db.execute_publication(f.request(PublicationAction::UpdateFrame(change)).await)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );
}
