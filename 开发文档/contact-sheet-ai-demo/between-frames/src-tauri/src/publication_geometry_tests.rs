use super::*;
async fn legacy_text(f: &Fixture) -> (String, String, String, String) {
    let page =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap()
            .pages[0]
            .id
            .clone();
    let id = domain::id();
    let content =
        "{\"kind\":\"text\", \"text\":\"保持原文字\", \"future\":{\"runs\":[1,2]}}".to_string();
    let style =
        "{\"fontSizePt\":14, \"openType\":{\"liga\":true}, \"note\":\"保留扩展样式\"}".to_string();
    sqlx::query("INSERT INTO publication_texts(id,page_id,x_mm,y_mm,width_mm,height_mm,content_json,style_json,created_at,updated_at) VALUES(?,?,15,20,80,25,?,?,'now','now')").bind(&id).bind(&page).bind(&content).bind(&style).execute(&f.db.pool).await.unwrap();
    (id, page, content, style)
}
#[tokio::test]
async fn text_geometry_preserves_raw_content_styles_identity_history_and_restart() {
    let f = Fixture::new().await;
    let (text, page, content, style) = legacy_text(&f).await;
    let baseline = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "文字原版".into(),
        }))
        .await;
    let baseline_id = baseline.command_id.clone();
    f.db.execute_publication(baseline).await.unwrap();
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let command = f
        .request(PublicationAction::UpdateTextGeometry(TextGeometry {
            text_id: text.clone(),
            page_id: page,
            x_mm: 25.,
            y_mm: 35.,
            width_mm: 95.,
            height_mm: 30.,
        }))
        .await;
    let id = command.command_id.clone();
    let first = f.db.execute_publication(command.clone()).await.unwrap();
    let replay = f.db.execute_publication(command).await.unwrap();
    assert_eq!(encode(&first).unwrap(), encode(&replay).unwrap());
    let saved =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let changed = &saved.pages[0].texts[0];
    assert_eq!(changed.id, text);
    assert_eq!(changed.content_json, content);
    assert_eq!(changed.style_json.as_deref(), Some(style.as_str()));
    assert_eq!(
        (
            changed.x_mm,
            changed.y_mm,
            changed.width_mm,
            changed.height_mm
        ),
        (25., 35., 95., 30.)
    );
    assert_eq!(saved.pages[0].frames, before.pages[0].frames);
    let reopened = Store::open(&f.temp.path().join("publication.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        saved
    );
    assert_eq!(
        reopened
            .publication_version(&f.project, &f.publication, &id)
            .await
            .unwrap()
            .layout,
        saved
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: baseline_id,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn text_geometry_scope_bounds_permissions_and_batch_rollback_use_existing_authority() {
    let f = Fixture::new().await;
    let (text, page, _, _) = legacy_text(&f).await;
    let data = TextGeometry {
        text_id: text,
        page_id: page,
        x_mm: 25.,
        y_mm: 35.,
        width_mm: 95.,
        height_mm: 30.,
    };
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let history = encode(
        &f.db
            .publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap(),
    )
    .unwrap();
    for invalid in [
        TextGeometry {
            page_id: domain::id(),
            ..data.clone()
        },
        TextGeometry {
            text_id: domain::id(),
            ..data.clone()
        },
        TextGeometry {
            width_mm: 0.,
            ..data.clone()
        },
        TextGeometry {
            x_mm: -100.,
            ..data.clone()
        },
        TextGeometry {
            height_mm: 10000.,
            ..data.clone()
        },
    ] {
        assert!(
            f.db.execute_publication(
                f.request(PublicationAction::UpdateTextGeometry(invalid))
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
    let mut batch = f
        .request(PublicationAction::UpdateTextGeometry(data.clone()))
        .await;
    batch.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::UpdateTextGeometry(TextGeometry {
            text_id: domain::id(),
            ..data.clone()
        }),
    });
    assert!(f.db.execute_publication(batch).await.is_err());
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
    let request = f.request(PublicationAction::UpdateTextGeometry(data)).await;
    assert_eq!(
        CommandHandler::new(&f.db)
            .execute_publication(request, &CommandContext::read_only(&f.project))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
}

#[tokio::test]
async fn mixed_photo_and_text_arrangement_is_one_revision_replay_and_restore_keep_sources() {
    let f = Fixture::new().await;
    let (text, page, content, style) = legacy_text(&f).await;
    let baseline = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "整组编排之前".into(),
        }))
        .await;
    let baseline_id = baseline.command_id.clone();
    f.db.execute_publication(baseline).await.unwrap();
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let project = f.db.get(&f.project).await.unwrap();
    let mut frame = f.frame().await;
    frame.x_mm += 2.;
    frame.y_mm += 3.;
    let mut command = f.request(PublicationAction::UpdateFrame(frame)).await;
    command.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::UpdateTextGeometry(TextGeometry {
            text_id: text,
            page_id: page,
            x_mm: 25.,
            y_mm: 30.,
            width_mm: 80.,
            height_mm: 25.,
        }),
    });
    let revision = command.base_revision;
    let id = command.command_id.clone();
    let first = f.db.execute_publication(command.clone()).await.unwrap();
    assert_eq!(first.revision, revision + 1);
    assert_eq!(first.results.len(), 2);
    assert_eq!(
        encode(&first).unwrap(),
        encode(&f.db.execute_publication(command).await.unwrap()).unwrap()
    );
    let after =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let a = &after.pages[0].frames[0];
    let b = &before.pages[0].frames[0];
    assert_eq!(
        (
            a.asset_id.clone(),
            a.rendition_id.clone(),
            a.fit_mode.clone(),
            a.crop_x,
            a.crop_y,
            a.crop_zoom
        ),
        (
            b.asset_id.clone(),
            b.rendition_id.clone(),
            b.fit_mode.clone(),
            b.crop_x,
            b.crop_y,
            b.crop_zoom
        )
    );
    assert_eq!(after.pages[0].texts[0].content_json, content);
    assert_eq!(
        after.pages[0].texts[0].style_json.as_deref(),
        Some(style.as_str())
    );
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &baseline_id)
            .await
            .unwrap()
            .layout,
        before
    );
    let reopened = Store::open(&f.temp.path().join("publication.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .publication_version(&f.project, &f.publication, &id)
            .await
            .unwrap()
            .layout,
        after
    );
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: baseline_id,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );
    let current = f.db.get(&f.project).await.unwrap();
    assert_eq!(
        encode(&current.photos).unwrap(),
        encode(&project.photos).unwrap()
    );
    assert_eq!(current.sequence, project.sequence);
    f.originals_unchanged();
}

#[tokio::test]
async fn mixed_arrangement_late_foreign_text_and_excess_operations_rollback_everything() {
    let f = Fixture::new().await;
    let (text, page, _, _) = legacy_text(&f).await;
    let other =
        f.db.create_publication(
            &f.project,
            &CreatePublicationCommand {
                title: "另一册".into(),
                pub_type: None,
                width_mm: Some(190.),
                height_mm: Some(257.),
            },
        )
        .await
        .unwrap();
    f.db.generate_from_sequence(
        &f.project,
        &GenerateFromSequenceCommand {
            publication_id: other.id.clone(),
            photos_per_page: Some(1),
        },
    )
    .await
    .unwrap();
    let foreign_page =
        f.db.publication_layout(&f.project, &other.id)
            .await
            .unwrap()
            .pages[0]
            .id
            .clone();
    let before =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let history = encode(
        &f.db
            .publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap(),
    )
    .unwrap();
    let mut frame = f.frame().await;
    frame.x_mm += 1.;
    let mut bad = f
        .request(PublicationAction::UpdateFrame(frame.clone()))
        .await;
    bad.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::UpdateTextGeometry(TextGeometry {
            text_id: text.clone(),
            page_id: foreign_page,
            x_mm: 25.,
            y_mm: 30.,
            width_mm: 80.,
            height_mm: 25.,
        }),
    });
    assert!(f.db.execute_publication(bad).await.is_err());
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
    let mut oversized = f.request(PublicationAction::UpdateFrame(frame)).await;
    let action = PublicationAction::UpdateTextGeometry(TextGeometry {
        text_id: text,
        page_id: page,
        x_mm: 25.,
        y_mm: 30.,
        width_mm: 80.,
        height_mm: 25.,
    });
    oversized
        .operations
        .extend((0..100).map(|_| PublicationOperation {
            op_id: domain::id(),
            action: action.clone(),
        }));
    assert_eq!(
        f.db.execute_publication(oversized).await.unwrap_err().code,
        "VALIDATION_ERROR"
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        before
    );
}
