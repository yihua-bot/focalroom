use super::*;
use crate::image_pipeline;

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
async fn captions_are_added_revised_and_removed_without_faking_a_change_or_leaving_half_a_box() {
    let f = Fixture::new().await;
    let layout =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    // 正文页而非封面：封面只有一个铺满的图框，图注压在封面上看不出位置规则。
    let page = &layout.pages[1];
    let (page_id, page_width) = (page.id.clone(), page.width_mm);
    let text_id = domain::id();
    let caption = NewText {
        text_id: text_id.clone(),
        page_id: page_id.clone(),
        x_mm: 15.,
        y_mm: 200.,
        width_mm: 160.,
        height_mm: 18.,
        text: "北海道，第一场雪".into(),
        size_pt: 10.,
        align: "center".into(),
    };
    let request = f.request(PublicationAction::AddText(caption.clone())).await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert!(receipt.changed);
    assert_eq!(receipt.results, vec![Value::Null]);
    let stored =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let text = &stored.pages.iter().find(|p| p.id == page_id).unwrap().texts[0];
    assert_eq!(
        serde_json::from_str::<Value>(&text.content_json).unwrap()["text"],
        "北海道，第一场雪"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&text.style_json.clone().unwrap()).unwrap()["align"],
        "center"
    );
    assert_eq!(text.x_mm, 15.);

    // 同一命令重放：不重复插入，也不多记一个版本。
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len();
    let replay = f.db.execute_publication(request).await.unwrap();
    assert_eq!(encode(&replay).unwrap(), encode(&receipt).unwrap());
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history
    );

    // 原样再次提交不算改动：客户端只是把读回来的值回传，版本不该因此 +1。
    let unchanged =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateText(TextUpdate {
                text_id: text_id.clone(),
                x_mm: 15.,
                y_mm: 200.,
                width_mm: 160.,
                height_mm: 18.,
                text: "北海道，第一场雪".into(),
                size_pt: 10.,
                align: "center".into(),
            }))
            .await,
        )
        .await
        .unwrap();
    assert!(!unchanged.changed);
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history
    );

    let revised =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateText(TextUpdate {
                text_id: text_id.clone(),
                x_mm: 20.,
                y_mm: 205.,
                width_mm: 150.,
                height_mm: 14.,
                text: "十胜，雪原尽头".into(),
                size_pt: 12.,
                align: "left".into(),
            }))
            .await,
        )
        .await
        .unwrap();
    assert!(revised.changed);
    let stored =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let text = &stored.pages.iter().find(|p| p.id == page_id).unwrap().texts[0];
    assert_eq!(
        serde_json::from_str::<Value>(&text.content_json).unwrap()["text"],
        "十胜，雪原尽头"
    );
    assert_eq!(text.width_mm, 150.);

    // 越界的图注被版面校验拒绝，整笔回滚：既不留半个文字框，也不留版本。
    let before = stored;
    let published =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len();
    let error =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateText(TextUpdate {
                text_id: text_id.clone(),
                x_mm: page_width - 10.,
                y_mm: 205.,
                width_mm: 150.,
                height_mm: 14.,
                text: "超出页面".into(),
                size_pt: 12.,
                align: "left".into(),
            }))
            .await,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, "VALIDATION_ERROR");
    assert!(error.message.contains("出血"), "{}", error.message);
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        published
    );

    // 空字、超长、字号与对齐非法：一律拒绝，错误原因可直接上屏。
    let legal = TextUpdate {
        text_id: text_id.clone(),
        x_mm: 20.,
        y_mm: 205.,
        width_mm: 150.,
        height_mm: 14.,
        text: "十胜，雪原尽头".into(),
        size_pt: 12.,
        align: "left".into(),
    };
    let rejections: Vec<TextUpdate> = vec![
        TextUpdate {
            text: "   ".into(),
            ..legal.clone()
        },
        TextUpdate {
            text: "雪".repeat(MAX_TEXT_CHARS + 1),
            ..legal.clone()
        },
        TextUpdate {
            size_pt: 73.,
            ..legal.clone()
        },
        TextUpdate {
            align: "justify".into(),
            ..legal
        },
    ];
    for bad in rejections {
        let error =
            f.db.execute_publication(f.request(PublicationAction::UpdateText(bad)).await)
                .await
                .unwrap_err();
        assert_eq!(error.code, "VALIDATION_ERROR");
    }
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );

    // 挂到不存在的页面：拒绝，不新增游离文字框。
    let orphan = NewText {
        text_id: domain::id(),
        page_id: domain::id(),
        ..caption
    };
    assert_eq!(
        f.db.execute_publication(f.request(PublicationAction::AddText(orphan)).await)
            .await
            .unwrap_err()
            .code,
        "NOT_FOUND"
    );

    f.db.execute_publication(
        f.request(PublicationAction::DeleteText(TextReference {
            text_id: text_id.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    let after =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert!(
        after
            .pages
            .iter()
            .all(|p| p.texts.iter().all(|t| t.id != text_id))
    );
    assert_eq!(
        f.db.execute_publication(
            f.request(PublicationAction::DeleteText(TextReference { text_id }))
                .await
        )
        .await
        .unwrap_err()
        .code,
        "NOT_FOUND"
    );
    f.originals_unchanged();
}

#[tokio::test]
async fn a_page_can_be_resized_and_a_page_too_small_for_its_photo_rolls_back_wholesale() {
    let f = Fixture::new().await;
    let layout =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    // 挑真有照片的那一页：空页面缩多小都不会撞边界，证明不了"改纸张要连着看图框"。
    let page = layout
        .pages
        .iter()
        .find(|p| !p.frames.is_empty())
        .expect("生成的版面里应有带照片的页面");
    let (page_id, width, height) = (page.id.clone(), page.width_mm, page.height_mm);
    let frames = page.frames.clone();
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len();
    // 放大一档开本：四个毫米数原样落库，图框一格都不动，版本多一条。
    let larger = PageUpdate {
        page_id: page_id.clone(),
        width_mm: width + 20.,
        height_mm: height + 20.,
        margin_mm: page.margin_mm,
        bleed_mm: page.bleed_mm,
    };
    let request = f
        .request(PublicationAction::UpdatePage(larger.clone()))
        .await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert!(receipt.changed);
    assert_eq!(receipt.results, vec![Value::Null]);
    let grown =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let stored = grown.pages.iter().find(|p| p.id == page_id).unwrap();
    assert_eq!(
        (
            stored.width_mm,
            stored.height_mm,
            stored.margin_mm,
            stored.bleed_mm
        ),
        (width + 20., height + 20., page.margin_mm, page.bleed_mm)
    );
    // 换纸张不是重排：照片按绝对毫米定位，纸张变大只是留白变多。
    assert_eq!(stored.frames, frames);
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history + 1
    );
    // 同一命令重放：等价回执，不多记版本。
    let replay = f.db.execute_publication(request).await.unwrap();
    assert_eq!(encode(&replay).unwrap(), encode(&receipt).unwrap());
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history + 1
    );
    // 原样再次提交不算改动：客户端把读回来的值回传，不该因此多一条版本。
    let unchanged =
        f.db.execute_publication(f.request(PublicationAction::UpdatePage(larger)).await)
            .await
            .unwrap();
    assert!(!unchanged.changed);
    assert_eq!(unchanged.revision, receipt.revision);
    // 缩到 60×60：页面上的照片放不下，整笔回滚——纸张没改、版本没多、原图没动。
    let error =
        f.db.execute_publication(
            f.request(PublicationAction::UpdatePage(PageUpdate {
                page_id: page_id.clone(),
                width_mm: 60.,
                height_mm: 60.,
                margin_mm: 5.,
                bleed_mm: 3.,
            }))
            .await,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, "VALIDATION_ERROR");
    assert!(error.message.contains("出血"), "{}", error.message);
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        grown
    );
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history + 1
    );
    f.originals_unchanged();
}

#[tokio::test]
async fn one_batched_command_unifies_every_page_and_a_later_page_that_overflows_rolls_the_whole_batch_back()
 {
    let f = Fixture::new().await;
    // 先把末页单独放大一档，做出"整本开本不一致"的真实局面：逐页改过的书才需要一次统一。
    let start =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let last = start.pages.last().unwrap();
    let stray = PageUpdate {
        page_id: last.id.clone(),
        width_mm: last.width_mm + 50.,
        height_mm: last.height_mm + 63.,
        margin_mm: last.margin_mm,
        bleed_mm: last.bleed_mm,
    };
    f.db.execute_publication(f.request(PublicationAction::UpdatePage(stray)).await)
        .await
        .unwrap();
    let mixed =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_ne!(
        mixed.pages[0].width_mm,
        mixed.pages.last().unwrap().width_mm
    );
    assert!(mixed.pages.len() > 2, "夹具应留下一本多页的书");
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len();

    // "应用到全部页面"就是这一条命令：一页一条操作，整本一次改完。
    let updates: Vec<_> = mixed
        .pages
        .iter()
        .map(|page| PageUpdate {
            page_id: page.id.clone(),
            width_mm: 210.,
            height_mm: 297.,
            margin_mm: page.margin_mm,
            bleed_mm: page.bleed_mm,
        })
        .collect();
    let mut request = f
        .request(PublicationAction::UpdatePage(updates[0].clone()))
        .await;
    for update in &updates[1..] {
        request.operations.push(PublicationOperation {
            op_id: domain::id(),
            action: PublicationAction::UpdatePage(update.clone()),
        });
    }
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert!(receipt.changed);
    // 三条纸张改动只涨一个版本，回执按契约每操作回一个 null。
    assert_eq!(receipt.results, vec![Value::Null; updates.len()]);
    assert_eq!(receipt.revision, receipt.base_revision + 1);
    let unified =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    for page in &unified.pages {
        assert_eq!((page.width_mm, page.height_mm), (210., 297.));
    }
    // 统一纸张不是重排：每页的图框一格都没动。
    for (page, before) in unified.pages.iter().zip(&mixed.pages) {
        assert_eq!(page.frames, before.frames);
    }
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history + 1
    );
    // 同一命令重放：等价回执，不多记版本，也不把整本再改一遍。
    let replay = f.db.execute_publication(request).await.unwrap();
    assert_eq!(encode(&replay).unwrap(), encode(&receipt).unwrap());
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history + 1
    );

    // 批量里只要有一页放不下照片，整笔回滚。第一条操作本身当然合法：
    // 上一步刚把整本改成 A4，再放大一档不可能撞边界——被拒的是后面的末页。
    let mut broken = f
        .request(PublicationAction::UpdatePage(PageUpdate {
            page_id: unified.pages[0].id.clone(),
            width_mm: 220.,
            height_mm: 300.,
            margin_mm: 15.,
            bleed_mm: 3.,
        }))
        .await;
    broken.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::UpdatePage(PageUpdate {
            page_id: mixed.pages[1].id.clone(),
            width_mm: 60.,
            height_mm: 60.,
            margin_mm: 5.,
            bleed_mm: 3.,
        }),
    });
    let error = f.db.execute_publication(broken).await.unwrap_err();
    assert_eq!(error.code, "VALIDATION_ERROR");
    assert!(error.message.contains("出血"), "{}", error.message);
    // 服务端这句话是笼统的：不点名是哪一页，也不带任何页码数字。
    // 所以"哪几页放不下"只能在客户端出门前算出来，否则用户只能逐页试。
    assert!(
        error.message.chars().all(|c| !c.is_ascii_digit()),
        "{}",
        error.message
    );
    // 半本新纸张都不许留下：版面与上一步一致，版本计数也没动。
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        unified
    );
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history + 1
    );
    f.originals_unchanged();
}

#[tokio::test]
async fn review_states_move_one_step_at_a_time_and_keep_a_finalised_book_locked_until_it_is_returned()
 {
    let f = Fixture::new().await;
    async fn state_of(f: &Fixture) -> String {
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap()
            .publication
            .state
    }
    assert_eq!(state_of(&f).await, "draft");
    async fn labels(f: &Fixture) -> Vec<String> {
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .into_iter()
            .map(|v| v.label)
            .collect()
    }
    // 陌生状态连命令都解析不出来，不会变成"未知状态"写进库里。
    assert!(
        serde_json::from_value::<PublicationAction>(
            json!({"type":"UpdateState","data":{"state":"archived"}})
        )
        .is_err()
    );
    // 跳级不合法：草稿不能一步定稿，也不能一步交付。
    for skipped in [PublicationState::Finalized, PublicationState::Delivered] {
        let error =
            f.db.execute_publication(
                f.request(PublicationAction::UpdateState(StateChange {
                    state: skipped,
                }))
                .await,
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, "CONFLICT");
        assert!(error.message.contains("逐级"), "{}", error.message);
        assert_eq!(state_of(&f).await, "draft");
    }
    let history =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len();
    // 送审：一次命令、版本 +1、历史上留一条读得出来的动作名。
    let review = f
        .request(PublicationAction::UpdateState(StateChange {
            state: PublicationState::InReview,
        }))
        .await;
    let receipt = f.db.execute_publication(review.clone()).await.unwrap();
    assert!(receipt.changed);
    assert_eq!(receipt.results, vec![Value::Null]);
    assert_eq!(receipt.revision, receipt.base_revision + 1);
    assert_eq!(state_of(&f).await, "in_review");
    assert_eq!(
        f.db.publication_revision(&f.project, &f.publication)
            .await
            .unwrap(),
        receipt.revision
    );
    assert_eq!(labels(&f).await.first().unwrap(), "送审");
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history + 1
    );
    // 送审之后版面整本锁住：改图框、加图注、记版本都被服务端拒（不只是界面禁着），
    // 而且被拒的命令一条都不落库。
    let locked =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    for action in [
        PublicationAction::UpdateFrame(f.frame().await),
        PublicationAction::UpdatePage(PageUpdate {
            page_id: locked.pages[0].id.clone(),
            width_mm: 210.,
            height_mm: 297.,
            margin_mm: 15.,
            bleed_mm: 3.,
        }),
        PublicationAction::CaptureVersion(VersionLabel {
            label: "送印前".into(),
        }),
    ] {
        let error =
            f.db.execute_publication(f.request(action).await)
                .await
                .unwrap_err();
        assert_eq!(error.code, "CONFLICT");
    }
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        locked
    );
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history + 1
    );
    // 同一命令重放：等价回执，不多记版本。
    let replay = f.db.execute_publication(review).await.unwrap();
    assert_eq!(encode(&replay).unwrap(), encode(&receipt).unwrap());
    // 换一条命令再提交一次当前状态也不算改动：不涨版本，也不记一条新的历史。
    let noop =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateState(StateChange {
                state: PublicationState::InReview,
            }))
            .await,
        )
        .await
        .unwrap();
    assert!(!noop.changed);
    assert_eq!(noop.revision, receipt.revision);
    assert_eq!(
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .len(),
        history + 1
    );
    // 退回草稿这条路必须是通的：这正是以前只有手工改数据库才能救回来的局面。
    let back =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateState(StateChange {
                state: PublicationState::Draft,
            }))
            .await,
        )
        .await
        .unwrap();
    assert!(back.changed);
    assert_eq!(state_of(&f).await, "draft");
    assert_eq!(labels(&f).await.first().unwrap(), "退回草稿");
    f.db.execute_publication(
        f.request(PublicationAction::UpdateFrame(f.frame().await))
            .await,
    )
    .await
    .unwrap();
    // 一路走到交付，交付就是终点。
    for next in [
        PublicationState::InReview,
        PublicationState::Finalized,
        PublicationState::Delivered,
    ] {
        f.db.execute_publication(
            f.request(PublicationAction::UpdateState(StateChange { state: next }))
                .await,
        )
        .await
        .unwrap();
        assert_eq!(state_of(&f).await, next.as_str());
    }
    let error =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateState(StateChange {
                state: PublicationState::Draft,
            }))
            .await,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, "CONFLICT");
    assert_eq!(state_of(&f).await, "delivered");
    f.originals_unchanged();
}

#[tokio::test]
async fn the_delivery_sheet_repeats_the_stored_layout_and_the_preflight_numbers() {
    let f = Fixture::new().await;
    let page_id =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap()
            .pages[1]
            .id
            .clone();
    let caption = "北海道，第一场雪 \"雾\"之后";
    f.db.execute_publication(
        f.request(PublicationAction::AddText(NewText {
            text_id: domain::id(),
            page_id,
            x_mm: 15.,
            y_mm: 200.,
            width_mm: 160.,
            height_mm: 18.,
            text: caption.into(),
            size_pt: 10.,
            align: "center".into(),
        }))
        .await,
    )
    .await
    .unwrap();
    let stored =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let csv = crate::publication_export::render_worklist(&stored);
    use crate::publication_export::sheet_reader::{col, rows};
    let rows = rows(&csv);
    let items: usize = stored
        .pages
        .iter()
        .map(|page| page.frames.len() + page.texts.len())
        .sum();
    assert_eq!(rows.len(), items + 1);
    let captions: Vec<&Vec<String>> = rows[1..]
        .iter()
        .filter(|row| row[col("kind")] == "text")
        .collect();
    assert!(
        captions
            .iter()
            .any(|row| row[col("text")] == caption && !row[col("id")].is_empty())
    );
    // 清单与印前自检必须给同一个数：两份交付依据打架时，用户无从判断该信哪一个。
    for page in &stored.pages {
        for frame in &page.frames {
            let row = rows[1..]
                .iter()
                .find(|row| row[col("id")] == frame.id)
                .expect("这一条图框没进清单");
            assert_eq!(row[col("page_number")], page.page_number.to_string());
            assert_eq!(row[col("side")], page.side);
            let source = frame.source.as_ref().expect("生成的版面每条都带来源");
            let ppi = crate::publication_preflight::effective_ppi(source, frame).unwrap();
            assert!((row[col("effective_ppi")].parse::<f64>().unwrap() - ppi).abs() < 0.005);
            let (width, height) = crate::publication_preflight::delivered(source);
            assert!((row[col("delivered_width_px")].parse::<f64>().unwrap() - width).abs() < 0.005);
            assert!(
                (row[col("delivered_height_px")].parse::<f64>().unwrap() - height).abs() < 0.005
            );
            assert_eq!(row[col("asset_id")], source.asset_id);
        }
    }
    // 真落一次盘：交付出口给人的是一张能打开的表，不是内存里的一串字。
    let path = f.temp.path().join("输出/worklist.csv");
    let receipt = crate::publication_export::write(&csv, &path).unwrap();
    assert_eq!(receipt.bytes, csv.len());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), csv);
    assert_eq!(receipt.sha256, image_pipeline::fingerprint(&path).unwrap());
    f.originals_unchanged();
}

#[tokio::test]
async fn an_exhibition_keeps_its_own_viewing_distance_and_a_book_refuses_to_borrow_it() {
    use crate::publication_spec::ExhibitionSpec;
    let f = Fixture::new().await;
    let spec = |viewing_distance_mm: Option<f64>, hang_center_mm: Option<f64>| ExhibitionSpec {
        viewing_distance_mm,
        hang_center_mm,
    };
    async fn stored(f: &Fixture) -> serde_json::Value {
        let raw =
            f.db.publication_layout(&f.project, &f.publication)
                .await
                .unwrap()
                .publication
                .spec_json;
        serde_json::from_str(&raw).unwrap()
    }
    async fn labels(f: &Fixture) -> Vec<String> {
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .into_iter()
            .map(|version| version.label)
            .collect()
    }

    // 摄影书没有"站在多远看"这件事：把这两个数写进它的规格，得到的是一份看起来合法的假规格。
    let refused =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateSpec(spec(
                Some(2_500.0),
                Some(1_450.0),
            )))
            .await,
        )
        .await
        .unwrap_err();
    assert_eq!(refused.code, "CONFLICT");
    assert!(refused.message.contains("展览"), "{}", refused.message);
    assert_eq!(
        stored(&f).await.get("viewing_distance_mm"),
        None,
        "被拒的命令一个字也不该留下"
    );

    sqlx::query("UPDATE publications SET pub_type='exhibition' WHERE id=?")
        .bind(&f.publication)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let request = f
        .request(PublicationAction::UpdateSpec(spec(
            Some(2_500.0),
            Some(1_450.0),
        )))
        .await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert!(
        receipt.changed,
        "规格命令得真的改动这本，否则版本与判据都不会跟着走"
    );
    assert_eq!(receipt.revision, receipt.base_revision + 1);
    assert_eq!(receipt.results, vec![Value::Null]);
    let stored_spec = stored(&f).await;
    assert_eq!(stored_spec["viewing_distance_mm"], 2_500.0);
    assert_eq!(stored_spec["hang_center_mm"], 1_450.0);
    assert!(
        stored_spec.get("width_mm").is_some(),
        "开本那两个数不能被规格命令弄丢——那是页面尺寸的唯一出处"
    );
    assert_eq!(labels(&f).await.first().unwrap(), "定展览规格");
    // 同一条命令重放等价，不重复记版本。
    let replay = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(replay.revision, receipt.revision);
    // 原样再提交一次（换了命令编号）不算改动：键序与空格不是"用户又改了一次规格"。
    let repeated =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateSpec(spec(
                Some(2_500.0),
                Some(1_450.0),
            )))
            .await,
        )
        .await
        .unwrap();
    assert!(!repeated.changed, "同样的两个数不该白记一个版本");
    assert_eq!(repeated.revision, receipt.revision);

    // 留空是"明确不记录"：那个键就此消失，而不是留下一个会被当成 0 的数。
    let cleared =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateSpec(spec(Some(2_500.0), None)))
                .await,
        )
        .await
        .unwrap();
    assert!(cleared.changed);
    let after_clear = stored(&f).await;
    assert_eq!(after_clear["viewing_distance_mm"], 2_500.0);
    assert_eq!(after_clear.get("hang_center_mm"), None);
    assert!(
        after_clear.get("width_mm").is_some(),
        "清空展览字段不该顺手把开本一起清掉"
    );

    // 越界与脏值出不了门，且库里仍是上一份好数据。
    for bad in [
        spec(Some(120.0), None),
        spec(Some(90_000.0), None),
        spec(None, Some(4_500.0)),
    ] {
        let error =
            f.db.execute_publication(f.request(PublicationAction::UpdateSpec(bad)).await)
                .await
                .unwrap_err();
        assert_eq!(error.code, "VALIDATION_ERROR", "{}", error.message);
        assert!(error.message.contains("毫米"), "{}", error.message);
        assert_eq!(stored(&f).await["viewing_distance_mm"], 2_500.0);
    }

    // 规格是这本的元数据，跟着"只有草稿能改"这条规矩走：定稿流程里不许偷偷换判据。
    f.db.execute_publication(
        f.request(PublicationAction::UpdateState(StateChange {
            state: PublicationState::InReview,
        }))
        .await,
    )
    .await
    .unwrap();
    let locked =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateSpec(spec(Some(3_000.0), None)))
                .await,
        )
        .await
        .unwrap_err();
    assert_eq!(locked.code, "CONFLICT");
    assert_eq!(stored(&f).await["viewing_distance_mm"], 2_500.0);
    f.originals_unchanged();
}

#[tokio::test]
async fn open_review_opinions_are_bound_to_the_book_and_gate_the_finalize_action() {
    use crate::agent::comments::{
        PublicationReviewRequest, publication_review, publication_review_write,
    };
    let f = Fixture::new().await;
    async fn labels(f: &Fixture) -> Vec<String> {
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()
            .into_iter()
            .map(|version| version.label)
            .collect()
    }
    let body = "第 3 页留白太多，压不住后面的跨页";

    // 意见挂在"这一本 + 提它时那一版"上，作者落的是受信本地身份，不是界面自己填的名字。
    let listed = publication_review_write(
        &f.db,
        &f.project,
        &f.publication,
        PublicationReviewRequest::Add { body: body.into() },
    )
    .await
    .unwrap();
    assert_eq!(listed["threads"].as_array().unwrap().len(), 1);
    assert_eq!(listed["open"], 1);
    let thread = listed["threads"][0].clone();
    assert_eq!(thread["revision"], listed["revision"]);
    assert_eq!(thread["state"], "open");
    assert_eq!(thread["comments"][0]["author"], "user:local-owner");
    assert_eq!(thread["comments"][0]["body"], body);

    // 空正文与不存在的那本都留不下线程。
    for blank in ["", "   "] {
        let error = publication_review_write(
            &f.db,
            &f.project,
            &f.publication,
            PublicationReviewRequest::Add { body: blank.into() },
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "VALIDATION_ERROR", "{}", error.message);
    }
    let absent = publication_review_write(
        &f.db,
        &f.project,
        "00000000-0000-4000-8000-000000000000",
        PublicationReviewRequest::Add {
            body: "不存在的那本".into(),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(absent.code, "NOT_FOUND");
    assert_eq!(
        publication_review(&f.db, &f.project, &f.publication)
            .await
            .unwrap()["open"],
        1
    );

    // 送审照常：闸门只管"批准"这一步，不拦往前走的动作。
    f.db.execute_publication(
        f.request(PublicationAction::UpdateState(StateChange {
            state: PublicationState::InReview,
        }))
        .await,
    )
    .await
    .unwrap();

    // 定稿不许无声跨过未解决的意见：整条拒掉，状态、修订号、历史都保持原样。
    let revision_before =
        f.db.publication_revision(&f.project, &f.publication)
            .await
            .unwrap();
    let history_before = labels(&f).await;
    let blocked =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateState(StateChange {
                state: PublicationState::Finalized,
            }))
            .await,
        )
        .await
        .unwrap_err();
    assert_eq!(blocked.code, "CONFLICT");
    assert!(
        blocked.message.contains("未解决的审阅意见"),
        "{}",
        blocked.message
    );
    let state: String = sqlx::query_scalar("SELECT state FROM publications WHERE id=?")
        .bind(&f.publication)
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    assert_eq!(state, "in_review");
    assert_eq!(
        f.db.publication_revision(&f.project, &f.publication)
            .await
            .unwrap(),
        revision_before
    );
    assert_eq!(labels(&f).await, history_before);

    // 解决是明确表态：解决了闸门让开，重开立刻再关上。
    let thread_id = thread["id"].as_str().unwrap().to_owned();
    let settled = publication_review_write(
        &f.db,
        &f.project,
        &f.publication,
        PublicationReviewRequest::Resolve {
            thread_id: thread_id.clone(),
            resolved: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(settled["open"], 0);
    let reopened = publication_review_write(
        &f.db,
        &f.project,
        &f.publication,
        PublicationReviewRequest::Resolve {
            thread_id: thread_id.clone(),
            resolved: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(reopened["open"], 1);

    // 改一版版面绕不过去：意见锚在提它那一版，不会因为版本号变了就自动失效。
    f.db.execute_publication(
        f.request(PublicationAction::UpdateState(StateChange {
            state: PublicationState::Draft,
        }))
        .await,
    )
    .await
    .unwrap();
    let mut frame = f.frame().await;
    frame.x_mm += 4.;
    f.db.execute_publication(f.request(PublicationAction::UpdateFrame(frame)).await)
        .await
        .unwrap();
    let after_edit = publication_review(&f.db, &f.project, &f.publication)
        .await
        .unwrap();
    assert_eq!(after_edit["open"], 1);
    assert_eq!(after_edit["threads"][0]["revision"], thread["revision"]);
    assert_ne!(after_edit["revision"], thread["revision"]);
    // 重新送审照常，定稿仍被同一条意见拦着。
    f.db.execute_publication(
        f.request(PublicationAction::UpdateState(StateChange {
            state: PublicationState::InReview,
        }))
        .await,
    )
    .await
    .unwrap();
    let bypassed =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateState(StateChange {
                state: PublicationState::Finalized,
            }))
            .await,
        )
        .await
        .unwrap_err();
    assert_eq!(bypassed.code, "CONFLICT");

    // 真处理掉才放行，并且这一步照常留下能读出来的历史。
    publication_review_write(
        &f.db,
        &f.project,
        &f.publication,
        PublicationReviewRequest::Resolve {
            thread_id,
            resolved: true,
        },
    )
    .await
    .unwrap();
    let approved =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateState(StateChange {
                state: PublicationState::Finalized,
            }))
            .await,
        )
        .await
        .unwrap();
    assert!(approved.changed);
    assert_eq!(labels(&f).await.first().unwrap(), "定稿");

    // 闸门只数这本的未解决线程：别本的意见、照片上的意见都不算在它头上。
    let second =
        f.db.create_publication(
            &f.project,
            &CreatePublicationCommand {
                title: "第二本".into(),
                pub_type: None,
                width_mm: Some(190.),
                height_mm: Some(257.),
            },
        )
        .await
        .unwrap();
    publication_review_write(
        &f.db,
        &f.project,
        &second.id,
        PublicationReviewRequest::Add {
            body: "这本封面再想想".into(),
        },
    )
    .await
    .unwrap();
    let photo_id: String = sqlx::query_scalar("SELECT id FROM photos WHERE project_id=? LIMIT 1")
        .bind(&f.project)
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    let photo_thread = domain::id();
    sqlx::query("INSERT INTO comment_threads(id,project_id,anchor_json,state,created_at,updated_at) VALUES(?,?,?,'open','now','now')")
        .bind(&photo_thread)
        .bind(&f.project)
        .bind(format!(r#"{{"type":"photo","photo_id":"{photo_id}"}}"#))
        .execute(&f.db.pool)
        .await
        .unwrap();

    // 拿照片线程号走出版物通道：拒，且那条线程一个字节都没被改动。
    let wrong = publication_review_write(
        &f.db,
        &f.project,
        &f.publication,
        PublicationReviewRequest::Resolve {
            thread_id: photo_thread.clone(),
            resolved: true,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(wrong.code, "PERMISSION_DENIED");
    let untouched: String = sqlx::query_scalar("SELECT state FROM comment_threads WHERE id=?")
        .bind(&photo_thread)
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    assert_eq!(untouched, "open");

    // 退回审阅中再定一次稿，不该被那两条别处的意见挡住。
    f.db.execute_publication(
        f.request(PublicationAction::UpdateState(StateChange {
            state: PublicationState::InReview,
        }))
        .await,
    )
    .await
    .unwrap();
    let again =
        f.db.execute_publication(
            f.request(PublicationAction::UpdateState(StateChange {
                state: PublicationState::Finalized,
            }))
            .await,
        )
        .await
        .unwrap();
    assert!(again.changed);
    assert_eq!(
        publication_review(&f.db, &f.project, &f.publication)
            .await
            .unwrap()["open"],
        0
    );
    let other = publication_review(&f.db, &f.project, &second.id)
        .await
        .unwrap();
    assert_eq!(other["open"], 1);
    assert_eq!(other["threads"].as_array().unwrap().len(), 1);
    f.originals_unchanged();
}
