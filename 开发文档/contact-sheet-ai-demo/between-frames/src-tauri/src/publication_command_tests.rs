use super::*;
use crate::image_pipeline;

#[tokio::test]
async fn regenerate_requires_explicit_replacement_and_preserves_previous_layout_atomically() {
    let f = Fixture::new().await;
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    let old =
        f.db.publication_version(&f.project, &f.publication, &history[0].id)
            .await
            .unwrap();
    let request = f
        .request(PublicationAction::Generate(GeneratePublication {
            photos_per_page: 1,
            replace_existing: false,
        }))
        .await;
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "CONFLICT"
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );
    let bad = f
        .request(PublicationAction::Generate(GeneratePublication {
            photos_per_page: 3,
            replace_existing: true,
        }))
        .await;
    assert!(f.db.execute_publication(bad).await.is_err());
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );
    let request = f
        .request(PublicationAction::Generate(GeneratePublication {
            photos_per_page: 1,
            replace_existing: true,
        }))
        .await;
    let result = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(result.results, vec![json!(3)]);
    assert_eq!(
        serde_json::to_value(f.db.execute_publication(request).await.unwrap()).unwrap(),
        serde_json::to_value(result).unwrap()
    );
    let after =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(after.pages.len(), 4);
    assert_ne!(after, before);
    assert_eq!(
        serde_json::to_value(
            f.db.publication_version(&f.project, &f.publication, &history[0].id)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(old).unwrap()
    );
    f.originals_unchanged();
}

struct Fixture {
    temp: tempfile::TempDir,
    db: Store,
    project: String,
    publication: String,
    source_hashes: Vec<(std::path::PathBuf, String)>,
}
impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("publication.sqlite"))
            .await
            .unwrap();
        let project = db.create("出版版本", "保持原图与历史版面").await.unwrap();
        let mut hashes = Vec::new();
        for i in 0..3 {
            let path = temp.path().join(format!("frame-{i}.jpg"));
            image::RgbImage::from_pixel(64, 48, image::Rgb([40 + i * 50, 90, 120]))
                .save(&path)
                .unwrap();
            let photo = image_pipeline::prepare(&path, &temp.path().join("cache")).unwrap();
            hashes.push((path, photo.hash.clone()));
            db.insert_photo(&project.id, &photo).await.unwrap();
        }
        let project = db.get(&project.id).await.unwrap();
        let ids: Vec<_> = project.photos.iter().map(|p| p.id.clone()).collect();
        db.decide_many(&project.id, &ids, "keep", project.revision)
            .await
            .unwrap();
        let publication = db
            .create_publication(
                &project.id,
                &CreatePublicationCommand {
                    title: "第一版".into(),
                    pub_type: None,
                    width_mm: Some(190.),
                    height_mm: Some(257.),
                },
            )
            .await
            .unwrap();
        db.generate_from_sequence(
            &project.id,
            &GenerateFromSequenceCommand {
                publication_id: publication.id.clone(),
                photos_per_page: Some(2),
            },
        )
        .await
        .unwrap();
        Self {
            temp,
            db,
            project: project.id,
            publication: publication.id,
            source_hashes: hashes,
        }
    }
    async fn request(&self, action: PublicationAction) -> PublicationCommandRequest {
        PublicationCommandRequest::new(
            &self.project,
            &self.publication,
            self.db
                .publication_revision(&self.project, &self.publication)
                .await
                .unwrap(),
            action,
        )
    }
    async fn frame(&self) -> FrameUpdate {
        let layout = self
            .db
            .publication_layout(&self.project, &self.publication)
            .await
            .unwrap();
        let f = &layout.pages[0].frames[0];
        FrameUpdate {
            frame_id: f.id.clone(),
            x_mm: f.x_mm,
            y_mm: f.y_mm,
            width_mm: f.width_mm,
            height_mm: f.height_mm,
            fit_mode: f.fit_mode.clone(),
            crop_x: f.crop_x,
            crop_y: f.crop_y,
            crop_zoom: f.crop_zoom,
            asset_id: f.asset_id.clone(),
            rendition_id: f.rendition_id.clone(),
        }
    }
    fn originals_unchanged(&self) {
        for (path, sha) in &self.source_hashes {
            assert_eq!(&image_pipeline::fingerprint(path).unwrap(), sha);
        }
    }
}

#[tokio::test]
async fn edit_save_restore_and_replay_keep_immutable_history_and_survive_restart() {
    let f = Fixture::new().await;
    let original =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let project = f.db.get(&f.project).await.unwrap();
    let saved = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "编辑确认版".into(),
        }))
        .await;
    let saved_id = saved.command_id.clone();
    f.db.execute_publication(saved).await.unwrap();
    let saved =
        f.db.publication_version(&f.project, &f.publication, &saved_id)
            .await
            .unwrap();
    assert_eq!(saved.layout, original);
    assert!(saved.previews.iter().all(|p| p.thumbnail_path.is_some()));
    let mut frame = f.frame().await;
    frame.x_mm += 5.;
    frame.width_mm -= 10.;
    frame.crop_zoom = 2.;
    let request = f.request(PublicationAction::UpdateFrame(frame)).await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert!(receipt.changed);
    let changed =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_ne!(changed, original);
    assert_eq!(changed.pages[1..], original.pages[1..]);
    let no_op = f
        .request(PublicationAction::UpdateFrame(f.frame().await))
        .await;
    let count =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len();
    let no_op = f.db.execute_publication(no_op).await.unwrap();
    assert!(!no_op.changed);
    assert_eq!(no_op.revision, receipt.revision);
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        count
    );
    let restore = f
        .request(PublicationAction::RestoreVersion(VersionReference {
            version_id: saved_id.clone(),
        }))
        .await;
    let restored = f.db.execute_publication(restore).await.unwrap();
    assert_eq!(restored.revision, receipt.revision + 1);
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        original
    );
    assert_eq!(
        encode(&f.db.execute_publication(request.clone()).await.unwrap()).unwrap(),
        encode(&receipt).unwrap()
    );
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &saved_id)
            .await
            .unwrap()
            .version
            .snapshot_sha256,
        saved.version.snapshot_sha256
    );
    assert_eq!(
        encode(&f.db.get(&f.project).await.unwrap()).unwrap(),
        encode(&project).unwrap()
    );
    f.originals_unchanged();
    f.db.pool.close().await;
    let db = Store::open(&f.temp.path().join("publication.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        original
    );
    assert_eq!(
        encode(&db.execute_publication(request).await.unwrap()).unwrap(),
        encode(&receipt).unwrap()
    );
    assert_eq!(
        CommandHandler::new(&db)
            .execute_publication(
                PublicationCommandRequest::new(
                    &f.project,
                    &f.publication,
                    restored.revision,
                    PublicationAction::CaptureVersion(VersionLabel {
                        label: "拒绝".into()
                    })
                ),
                &CommandContext::read_only(&f.project)
            )
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
}

#[tokio::test]
async fn concurrent_edits_have_one_winner_and_cannot_reuse_operation_or_idempotency_keys() {
    let f = Fixture::new().await;
    let mut frame = f.frame().await;
    frame.x_mm += 1.;
    frame.width_mm -= 2.;
    let a = f
        .request(PublicationAction::UpdateFrame(frame.clone()))
        .await;
    frame.x_mm += 1.;
    let b = f.request(PublicationAction::UpdateFrame(frame)).await;
    let (left, right) = tokio::join!(
        f.db.execute_publication(a.clone()),
        f.db.execute_publication(b)
    );
    assert_ne!(left.is_ok(), right.is_ok());
    assert_eq!(
        left.err().or(right.err()).unwrap().code,
        "REVISION_CONFLICT"
    );
    let mut request = f
        .request(PublicationAction::UpdateFrame(f.frame().await))
        .await;
    request.idempotency_key = a.idempotency_key;
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "IDEMPOTENCY_CONFLICT"
    );
    let used: String = sqlx::query_scalar(
        "SELECT op_id FROM publication_operation_ids ORDER BY rowid DESC LIMIT 1",
    )
    .fetch_one(&f.db.pool)
    .await
    .unwrap();
    let mut request = f
        .request(PublicationAction::UpdateFrame(f.frame().await))
        .await;
    request.operations[0].op_id = used;
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "IDEMPOTENCY_CONFLICT"
    );
}

#[tokio::test]
async fn invalid_batch_and_history_write_failures_roll_back_all_layout_and_receipts() {
    let f = Fixture::new().await;
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    let mut frame = f.frame().await;
    frame.x_mm += 1.;
    frame.width_mm -= 2.;
    let mut request = f
        .request(PublicationAction::UpdateFrame(frame.clone()))
        .await;
    request.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::UpdatePage(PageUpdate {
            page_id: before.pages[0].id.clone(),
            width_mm: 50.,
            height_mm: 50.,
            margin_mm: 30.,
            bleed_mm: 3.,
        }),
    });
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "VALIDATION_ERROR"
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );
    sqlx::query("CREATE TRIGGER fail_pub_history BEFORE INSERT ON publication_versions BEGIN SELECT RAISE(ABORT,'injected'); END").execute(&f.db.pool).await.unwrap();
    let request = f.request(PublicationAction::UpdateFrame(frame)).await;
    let id = request.command_id.clone();
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "DATABASE"
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
        encode(&history).unwrap()
    );
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM publication_command_events WHERE command_id=?)"
        )
        .bind(id)
        .fetch_one(&f.db.pool)
        .await
        .unwrap()
    );
}

#[tokio::test]
async fn foreign_assets_versions_frames_and_read_only_contexts_never_cross_projects() {
    let f = Fixture::new().await;
    let other = Fixture::new().await; // 不同Store的编号亦不能当作本项目资源。
    let mut frame = f.frame().await;
    frame.asset_id = other.frame().await.asset_id;
    let request = f.request(PublicationAction::UpdateFrame(frame)).await;
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "ASSET_NOT_FOUND"
    );
    let foreign = f.db.create("其他作者", "隔离").await.unwrap();
    let pub2 =
        f.db.create_publication(
            &foreign.id,
            &CreatePublicationCommand {
                title: "别的项目".into(),
                pub_type: None,
                width_mm: None,
                height_mm: None,
            },
        )
        .await
        .unwrap();
    let version =
        f.db.publication_versions(&foreign.id, &pub2.id, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    assert_eq!(
        f.db.execute_publication(
            f.request(PublicationAction::RestoreVersion(VersionReference {
                version_id: version
            }))
            .await
        )
        .await
        .unwrap_err()
        .code,
        "NOT_FOUND"
    );
    assert_eq!(
        f.db.publication_versions(&foreign.id, &f.publication, None)
            .await
            .unwrap_err()
            .code,
        "NOT_FOUND"
    );
    let mut request = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "只读".into(),
        }))
        .await;
    let original = request.clone();
    request.project_id = foreign.id.clone();
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "NOT_FOUND"
    );
    assert_eq!(
        CommandHandler::new(&f.db)
            .execute_publication(original, &CommandContext::human(&foreign.id))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    f.originals_unchanged();
}

#[tokio::test]
async fn old_previews_never_show_changed_cache_and_history_is_readable_when_current_layout_is_invalid()
 {
    let f = Fixture::new().await;
    let versions =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    let version = &versions[0];
    let original =
        f.db.publication_version(&f.project, &f.publication, &version.id)
            .await
            .unwrap();
    let path = original.previews[0].thumbnail_path.as_ref().unwrap();
    image::RgbImage::from_pixel(64, 48, image::Rgb([255, 0, 0]))
        .save(path)
        .unwrap();
    let old =
        f.db.publication_version(&f.project, &f.publication, &version.id)
            .await
            .unwrap();
    assert_eq!(old.layout, original.layout);
    assert!(
        old.previews
            .iter()
            .filter(|p| p.frame_id == old.layout.pages[0].frames[0].id)
            .all(|p| p.thumbnail_path.is_none())
    );
    let layout =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let frame = &layout.pages[0].frames[0];
    sqlx::query("UPDATE publication_frames SET width_mm=-1 WHERE id=?")
        .bind(&frame.id)
        .execute(&f.db.pool)
        .await
        .unwrap();
    assert!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .is_err()
    );
    assert!(
        !f.db
            .publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &version.id)
            .await
            .unwrap()
            .layout,
        original.layout
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: version.id.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    let restored =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(
        restored.pages[0].frames[0].width_mm,
        original.layout.pages[0].frames[0].width_mm
    );
    f.originals_unchanged();
}

#[tokio::test]
async fn versions_are_append_only_paginated_and_upgrade_from_v32_preserves_publications() {
    let f = Fixture::new().await;
    for n in 0..33 {
        f.db.execute_publication(
            f.request(PublicationAction::CaptureVersion(VersionLabel {
                label: format!("方案{n}"),
            }))
            .await,
        )
        .await
        .unwrap();
    }
    let first =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap();
    assert_eq!(first.len(), 30);
    let last =
        f.db.publication_versions(
            &f.project,
            &f.publication,
            Some(first.last().unwrap().revision),
        )
        .await
        .unwrap();
    assert_eq!(last.len(), 5);
    assert!(
        last.iter()
            .all(|v| v.revision < first.last().unwrap().revision)
    );
    assert!(
        sqlx::query("UPDATE publication_versions SET label='改写' WHERE id=?")
            .bind(&first[0].id)
            .execute(&f.db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM publication_versions WHERE id=?")
            .bind(&first[0].id)
            .execute(&f.db.pool)
            .await
            .is_err()
    );
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("v32.sqlite");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.migrations = std::borrow::Cow::Owned(
        migrator
            .iter()
            .filter(|m| m.version <= 32)
            .cloned()
            .collect(),
    );
    migrator.run(&pool).await.unwrap();
    let project = domain::id();
    let publication = domain::id();
    sqlx::query(
        "INSERT INTO projects(id,title,created_at,updated_at) VALUES(?,'旧项目','now','now')",
    )
    .bind(&project)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO publications(id,project_id,title,spec_json,created_at,updated_at) VALUES(?,?,'原出版物','{\"width_mm\":190,\"height_mm\":257}','now','now')").bind(&publication).bind(&project).execute(&pool).await.unwrap();
    pool.close().await;
    let db = Store::open(&path).await.unwrap();
    assert_eq!(db.list_publications(&project).await.unwrap()[0].revision, 1);
    assert!(
        db.publication_versions(&project, &publication, None)
            .await
            .unwrap()
            .is_empty()
    );
    let request = PublicationCommandRequest::new(
        &project,
        &publication,
        1,
        PublicationAction::CaptureVersion(VersionLabel {
            label: "迁移后确认".into(),
        }),
    );
    db.execute_publication(request).await.unwrap();
    assert_eq!(
        db.publication_versions(&project, &publication, None)
            .await
            .unwrap()
            .len(),
        2
    );
    assert!(std::fs::read_dir(temp.path()).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .contains("backup-before-v32")
    }));
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&db.pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn saved_rendition_stays_bound_when_current_variant_changes_and_rejects_wrong_identity() {
    let f = Fixture::new().await;
    let mut frame = f.frame().await;
    let asset = frame.asset_id.clone().unwrap();
    let mut renders = Vec::new();
    for (index, color) in [[180, 30, 20], [20, 40, 190]].into_iter().enumerate() {
        let path = f.temp.path().join(format!("render-{index}.jpg"));
        image::RgbImage::from_pixel(64, 48, image::Rgb(color))
            .save(&path)
            .unwrap();
        let prepared = image_pipeline::prepare(&path, &f.temp.path().join("cache")).unwrap();
        let variant = domain::id();
        let rendition = domain::id();
        sqlx::query("INSERT INTO edit_variants(id,asset_id,name,is_current,created_at) VALUES(?,?,?,?,'now')").bind(&variant).bind(&asset).bind(format!("方向{index}")).bind(index as i64).execute(&f.db.pool).await.unwrap();
        sqlx::query("INSERT INTO renditions(id,variant_id,sha256,width,height,bit_depth,source_type,file_path,file_size,created_at) VALUES(?,?,?,64,48,8,'self_render',?,?,'now')").bind(&rendition).bind(&variant).bind(&prepared.hash).bind(path.to_string_lossy().as_ref()).bind(prepared.size as i64).execute(&f.db.pool).await.unwrap();
        renders.push((rendition, variant, prepared.hash));
    }
    frame.rendition_id = Some(renders[0].0.clone());
    let command = f
        .request(PublicationAction::UpdateFrame(frame.clone()))
        .await;
    let version = command.command_id.clone();
    f.db.execute_publication(command).await.unwrap();
    let saved =
        f.db.publication_version(&f.project, &f.publication, &version)
            .await
            .unwrap();
    let old_source = saved.layout.pages[0].frames[0].source.as_ref().unwrap();
    assert_eq!(
        old_source.rendition_sha256.as_deref(),
        Some(renders[0].2.as_str())
    );
    assert!(
        saved.previews[0]
            .thumbnail_path
            .as_ref()
            .unwrap()
            .ends_with(&format!("{}-320.jpg", renders[0].2))
    );
    frame.rendition_id = Some(renders[1].0.clone());
    f.db.execute_publication(f.request(PublicationAction::UpdateFrame(frame)).await)
        .await
        .unwrap();
    let old =
        f.db.publication_version(&f.project, &f.publication, &version)
            .await
            .unwrap();
    assert_eq!(old.layout, saved.layout);
    assert_eq!(
        old.previews[0].thumbnail_path,
        saved.previews[0].thumbnail_path
    );
    let current =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(
        current.pages[0].frames[0].rendition_id.as_ref(),
        Some(&renders[1].0)
    );
    sqlx::query("UPDATE renditions SET sha256=? WHERE id=?")
        .bind("f".repeat(64))
        .bind(&renders[0].0)
        .execute(&f.db.pool)
        .await
        .unwrap();
    assert_eq!(
        f.db.execute_publication(
            f.request(PublicationAction::RestoreVersion(VersionReference {
                version_id: version.clone()
            }))
            .await
        )
        .await
        .unwrap_err()
        .code,
        "CONFLICT"
    );
    let old =
        f.db.publication_version(&f.project, &f.publication, &version)
            .await
            .unwrap();
    assert_eq!(old.layout, saved.layout);
    assert!(old.previews[0].thumbnail_path.is_none());
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        current
    );
    f.originals_unchanged();
}

#[path = "publication_editing_tests.rs"]
mod page_editing;

#[path = "publication_frame_tests.rs"]
mod frame_editing;

#[path = "publication_master_tests.rs"]
mod page_masters;

#[path = "publication_photo_version_tests.rs"]
mod photo_versions;

#[path = "publication_geometry_tests.rs"]
mod object_geometry;

#[path = "publication_spread_tests.rs"]
mod continuous_spreads;

#[path = "publication_layer_tests.rs"]
mod layer_editing;

#[path = "publication_guide_tests.rs"]
mod page_guides;

#[path = "publication_master_guide_tests.rs"]
mod master_guides;

#[path = "publication_chapter_tests.rs"]
mod chapter_editing;

#[path = "publication_page_removal_tests.rs"]
mod page_group_removal;

#[path = "publication_cover_tests.rs"]
mod cover_editing;

#[path = "publication_typography_tests.rs"]
mod typography_editing;
