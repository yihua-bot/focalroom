use super::*;
#[tokio::test]
async fn empty_book_can_start_with_cover_and_rejects_other_books_text_targets() {
    let f = Fixture::new().await;
    let publication =
        f.db.create_publication(
            &f.project,
            &CreatePublicationCommand {
                title: "空白摄影书".into(),
                pub_type: None,
                width_mm: Some(150.),
                height_mm: Some(200.),
            },
        )
        .await
        .unwrap();
    let page_id = domain::id();
    let request = PublicationCommandRequest::new(
        &f.project,
        &publication.id,
        1,
        PublicationAction::AddPage(AddPage {
            page_id: page_id.clone(),
            after_page_id: None,
            width_mm: 150.,
            height_mm: 200.,
            margin_mm: 10.,
            bleed_mm: 3.,
        }),
    );
    assert_eq!(
        CommandHandler::new(&f.db)
            .execute_publication(request.clone(), &CommandContext::human("another-project"))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    let receipt = f.db.execute_publication(request).await.unwrap();
    let layout =
        f.db.publication_layout(&f.project, &publication.id)
            .await
            .unwrap();
    assert_eq!(layout.pages.len(), 1);
    assert_eq!(layout.pages[0].page_number, 0);
    assert_eq!(layout.pages[0].side, "cover");
    let foreign =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let request = PublicationCommandRequest::new(
        &f.project,
        &publication.id,
        receipt.revision,
        PublicationAction::AddText(text(&foreign.pages[1].id)),
    );
    assert_eq!(
        f.db.execute_publication(request).await.unwrap_err().code,
        "NOT_FOUND"
    );
    assert_eq!(
        f.db.publication_layout(&f.project, &publication.id)
            .await
            .unwrap(),
        layout
    );
}
fn text(page: &str) -> TextUpdate {
    TextUpdate {
        text_id: domain::id(),
        page_id: page.into(),
        x_mm: 15.,
        y_mm: 220.,
        width_mm: 100.,
        height_mm: 20.,
        kind: "text".into(),
        text: "<script>原样文字</script>".into(),
        text_direction: None,
        font_family: "serif".into(),
        font_size_pt: 12.,
        line_height: 1.5,
        align: "center".into(),
        bold: false,
        color: "#112233".into(),
    }
}
#[tokio::test]
async fn insert_reorder_remove_and_restore_preserve_ids_sources_and_history() {
    let f = Fixture::new().await;
    let original =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let old_version =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    let id = domain::id();
    let request = f
        .request(PublicationAction::AddPage(AddPage {
            page_id: id.clone(),
            after_page_id: Some(original.pages[0].id.clone()),
            width_mm: 190.,
            height_mm: 257.,
            margin_mm: 15.,
            bleed_mm: 3.,
        }))
        .await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(
        encode(&receipt).unwrap(),
        encode(&f.db.execute_publication(request).await.unwrap()).unwrap()
    );
    let inserted =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(inserted.pages.len(), 4);
    assert_eq!(inserted.pages[1].id, id);
    assert_eq!(inserted.pages[2].frames, original.pages[1].frames);
    let mut ids: Vec<_> = inserted.pages.iter().map(|p| p.id.clone()).collect();
    ids.swap(1, 2);
    f.db.execute_publication(
        f.request(PublicationAction::ReorderPages(PageOrder {
            chapter_changes_confirmed: None,
            page_ids: ids.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    let reordered =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(
        reordered
            .pages
            .iter()
            .map(|p| p.id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        reordered
            .pages
            .iter()
            .map(|p| p.page_number)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(reordered.pages[2].side, "left");
    let t = text(&id);
    f.db.execute_publication(f.request(PublicationAction::AddText(t.clone())).await)
        .await
        .unwrap();
    let saved =
        f.db.publication_versions(&f.project, &f.publication, None)
            .await
            .unwrap()[0]
            .id
            .clone();
    f.db.execute_publication(
        f.request(PublicationAction::RemovePage(PageReference {
            page_id: id.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    let removed =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(
        removed
            .pages
            .iter()
            .map(|p| p.page_number)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(!removed.pages.iter().any(|p| p.id == id));
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: saved,
        }))
        .await,
    )
    .await
    .unwrap();
    let restored =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    assert_eq!(restored.pages[2].texts[0].id, t.text_id);
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &old_version)
            .await
            .unwrap()
            .layout,
        original
    );
    let reopened = Store::open(&f.temp.path().join("publication.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        restored
    );
    f.originals_unchanged();
}
#[tokio::test]
async fn order_must_be_complete_unique_scoped_and_keep_cover_first() {
    let f = Fixture::new().await;
    let original =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let ids: Vec<_> = original.pages.iter().map(|p| p.id.clone()).collect();
    for wrong in [
        vec![ids[0].clone()],
        vec![ids[0].clone(), ids[1].clone(), ids[1].clone()],
        vec![ids[1].clone(), ids[0].clone(), ids[2].clone()],
        vec![ids[0].clone(), ids[1].clone(), domain::id()],
    ] {
        assert!(
            f.db.execute_publication(
                f.request(PublicationAction::ReorderPages(PageOrder {
                    chapter_changes_confirmed: None,
                    page_ids: wrong
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
            original
        );
    }
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::RemovePage(PageReference {
                page_id: ids[0].clone()
            }))
            .await
        )
        .await
        .is_err()
    );
    assert!(
        f.db.execute_publication(
            f.request(PublicationAction::RemovePage(PageReference {
                page_id: domain::id()
            }))
            .await
        )
        .await
        .is_err()
    );
}
#[tokio::test]
async fn text_validation_atomic_rollback_and_remove_are_audited() {
    let f = Fixture::new().await;
    let original =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let t = text(&original.pages[1].id);
    for bad in [
        TextUpdate {
            font_size_pt: 0.,
            ..t.clone()
        },
        TextUpdate {
            color: "url(x)".into(),
            ..t.clone()
        },
        TextUpdate {
            kind: "html".into(),
            ..t.clone()
        },
        TextUpdate {
            width_mm: 2000.,
            ..t.clone()
        },
        TextUpdate {
            page_id: domain::id(),
            ..t.clone()
        },
    ] {
        assert!(
            f.db.execute_publication(f.request(PublicationAction::AddText(bad)).await)
                .await
                .is_err()
        );
    }
    let mut request = f.request(PublicationAction::AddText(t.clone())).await;
    request.operations.push(PublicationOperation {
        op_id: domain::id(),
        action: PublicationAction::UpdatePage(PageUpdate {
            page_id: domain::id(),
            width_mm: 190.,
            height_mm: 257.,
            margin_mm: 15.,
            bleed_mm: 3.,
        }),
    });
    assert!(f.db.execute_publication(request).await.is_err());
    assert_eq!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap(),
        original
    );
    let request = f.request(PublicationAction::AddText(t.clone())).await;
    let result = f.db.execute_publication(request.clone()).await.unwrap();
    assert_eq!(
        encode(&result).unwrap(),
        encode(&f.db.execute_publication(request).await.unwrap()).unwrap()
    );
    let edited = TextUpdate {
        kind: "page_number".into(),
        text: String::new(),
        font_size_pt: 16.,
        bold: true,
        ..t.clone()
    };
    f.db.execute_publication(f.request(PublicationAction::UpdateText(edited)).await)
        .await
        .unwrap();
    let after =
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap();
    let content: Value = serde_json::from_str(&after.pages[1].texts[0].content_json).unwrap();
    assert_eq!(content["kind"], "page_number");
    f.db.execute_publication(
        f.request(PublicationAction::RemoveText(TextReference {
            text_id: t.text_id.clone(),
        }))
        .await,
    )
    .await
    .unwrap();
    assert!(
        f.db.publication_layout(&f.project, &f.publication)
            .await
            .unwrap()
            .pages[1]
            .texts
            .is_empty()
    );
    assert!(
        f.db.execute_publication(f.request(PublicationAction::UpdateText(t)).await)
            .await
            .is_err()
    );
    f.originals_unchanged();
}
