use super::*;
async fn add(f: &Fixture, page: &str, photo: &str) -> PublicationCommandRequest {
    f.request(PublicationAction::AddFrame(PhotoFrameAdd {
        frame_id: domain::id(),
        page_id: page.into(),
        photo_id: photo.into(),
        rendition_id: None,
        x_mm: 80.,
        y_mm: 15.,
        width_mm: 70.,
        height_mm: 100.,
    }))
    .await
}
#[tokio::test]
async fn frames_add_replace_remove_replay_and_restore_keep_originals_and_fixed_sources() {
    let f = Fixture::new().await;
    let project = f.db.get(&f.project).await.unwrap();
    let old =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let target = &old.pages[2];
    let request = add(&f, &target.id, &project.photos[0].id).await;
    let frame = match &request.operations[0].action {
        PublicationAction::AddFrame(data) => data.frame_id.clone(),
        _ => unreachable!(),
    };
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&f.db.execute_publication(request).await.unwrap()).unwrap()
    );
    let added =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(added.pages[2].frames.len(), target.frames.len() + 1);
    let preview =
        f.db.publication_document(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(
        preview
            .previews
            .iter()
            .find(|p| p.frame_id == frame)
            .unwrap()
            .photo_id
            .as_ref(),
        Some(&project.photos[0].id)
    );
    let asset = added.pages[2]
        .frames
        .iter()
        .find(|x| x.id == frame)
        .unwrap()
        .asset_id
        .clone();
    assert!(asset.is_some());
    assert!(
        f.db.execute_publication(add(&f, &target.id, &project.photos[0].id).await)
            .await
            .is_err()
    );
    let version =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    f.db.execute_publication(
        f.request(PublicationAction::SetFramePhoto(FramePhoto {
            frame_id: frame.clone(),
            photo_id: project.photos[1].id.clone(),
            rendition_id: None,
        }))
        .await,
    )
    .await
    .unwrap();
    let replaced =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let replacement = replaced.pages[2]
        .frames
        .iter()
        .find(|x| x.id == frame)
        .unwrap();
    assert_ne!(replacement.asset_id, asset);
    assert_eq!(replacement.x_mm, 80.);
    assert_eq!(replacement.crop_zoom, 1.);
    f.db.execute_publication(
        f.request(PublicationAction::RemoveFrame(FrameReference {
            frame_id: frame.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    assert!(
        !f.db
            .publication_layout(&f.project, &f.publication)
            .await
            .unwrap()
            .pages[2]
            .frames
            .iter()
            .any(|x| x.id == frame)
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
        added
    );
    assert_eq!(
        f.db.get(&f.project).await.unwrap().sequence,
        project.sequence
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn invalid_addition_rolls_back_other_frame_updates_and_foreign_references_are_rejected() {
    let f = Fixture::new().await;
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let mut update = f.frame().await;
    update.width_mm = 60.;
    let mut request = f.request(PublicationAction::UpdateFrame(update)).await;
    request.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::AddFrame(PhotoFrameAdd {
            frame_id: domain::id(),
            page_id: before.pages[1].id.clone(),
            photo_id: "foreign-photo".into(),
            rendition_id: None,
            x_mm: 10.,
            y_mm: 10.,
            width_mm: 100.,
            height_mm: 100.,
        }),
    });
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "ASSET_NOT_FOUND"
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );
    let photo = f.db.get(&f.project).await.unwrap().photos[0].id.clone();
    let mut invalid = add(&f, &before.pages[2].id, &photo).await;
    if let PublicationAction::AddFrame(data) = &mut invalid.operations[0].action {
        data.rendition_id = Some(domain::id());
    }
    assert_eq!(
        f.db.execute_publication(invalid).await.unwrap_err().code,
        "ASSET_NOT_FOUND"
    );
    let mut invalid = add(&f, &before.pages[2].id, &photo).await;
    if let PublicationAction::AddFrame(data) = &mut invalid.operations[0].action {
        data.page_id = domain::id();
    }
    assert_eq!(
        f.db.execute_publication(invalid).await.unwrap_err().code,
        "NOT_FOUND"
    );
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::RemoveFrame(FrameReference {
                frame_id: domain::id()
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
        before
    );
}
#[tokio::test]
async fn add_frame_resolves_distinct_photo_asset_ids_within_the_project() {
    let f = Fixture::new().await;
    let photo = domain::id();
    let asset = domain::id();
    let hash = "f".repeat(64);
    sqlx::query("INSERT INTO photos(id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,orientation,thumbnail_path,preview_path,created_at) VALUES(?,?,4,'missing.jpg','missing.jpg',10,?,100,100,1,'missing-320.jpg','missing-1400.jpg','now')")
        .bind(&photo).bind(&f.project).bind(&hash).execute(&f.db.pool).await.unwrap();
    sqlx::query("INSERT INTO source_assets(id,project_id,content_sha256,original_locator,file_size,width,height,format,created_at) VALUES(?,?,?,'[]',10,100,100,'jpeg','now')")
        .bind(&asset).bind(&f.project).bind(hash).execute(&f.db.pool).await.unwrap();
    sqlx::query("INSERT INTO asset_aliases(photo_id,asset_id) VALUES(?,?)")
        .bind(&photo)
        .bind(&asset)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let page =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap()
            .pages[2]
            .id
            .clone();
    let request = add(&f, &page, &photo).await;
    f.db.execute_publication(request).await.unwrap();
    let after =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert!(
        after.pages[2]
            .frames
            .iter()
            .any(|frame| frame.asset_id.as_ref() == Some(&asset))
    );
}
