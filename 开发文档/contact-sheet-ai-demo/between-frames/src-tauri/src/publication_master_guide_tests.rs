use super::*;
use guides::{PageGuides, SetPageGuides};
fn grid(x: f64) -> PageGuides {
    PageGuides {
        columns: 3,
        rows: 4,
        column_gap_mm: 5.,
        row_gap_mm: 4.,
        vertical_guides_mm: vec![x],
        horizontal_guides_mm: vec![30.],
    }
}
async fn layout(f: &Fixture) -> versions::PublicationLayout {
    f.db.publication_layout(&f.project, &f.publication)
        .await
        .unwrap()
}
async fn guide(f: &Fixture, page: &str, settings: PageGuides) {
    f.db.execute_publication(
        f.request(PublicationAction::SetPageGuides(SetPageGuides {
            page_id: page.into(),
            settings,
        }))
        .await,
    )
    .await
    .unwrap();
}
async fn save(
    f: &Fixture,
    page: &str,
    master: &str,
    revision: Option<i64>,
    include: Option<bool>,
) -> PublicationCommandRequest {
    f.request(PublicationAction::SaveMaster(SaveMaster {
        master_id: master.into(),
        source_page_id: page.into(),
        name: "网格母版".into(),
        expected_master_revision: revision,
        include_text: false,
        include_guides: include,
    }))
    .await
}
fn application(master: &str, page: &str, revision: i64, overwrite: bool) -> ApplyMaster {
    ApplyMaster {
        master_id: master.into(),
        master_revision: revision,
        page_ids: vec![page.into()],
        instance_id: domain::id(),
        overwrite_local: overwrite,
    }
}
fn sources_unchanged(before: &versions::LayoutPage, after: &versions::LayoutPage) {
    for old in before
        .frames
        .iter()
        .filter(|frame| frame.asset_id.is_some())
    {
        let new = after
            .frames
            .iter()
            .find(|frame| frame.id == old.id)
            .unwrap();
        assert_eq!(
            (
                &new.asset_id,
                &new.rendition_id,
                &new.source,
                new.crop_x,
                new.crop_y,
                new.crop_zoom
            ),
            (
                &old.asset_id,
                &old.rendition_id,
                &old.source,
                old.crop_x,
                old.crop_y,
                old.crop_zoom
            )
        );
    }
}

#[tokio::test]
async fn guided_master_previews_custom_override_then_applies_exact_settings_with_history_and_restart()
 {
    let f = Fixture::new().await;
    let initial = layout(&f).await;
    let source = initial.pages[1].id.clone();
    let target = initial.pages[2].id.clone();
    let master = domain::id();
    guide(&f, &source, grid(25.)).await;
    guide(&f, &target, grid(40.)).await;
    f.db.execute_publication(save(&f, &source, &master, None, Some(true)).await)
        .await
        .unwrap();
    let before = layout(&f).await;
    let definition = masters::masters(&before).unwrap().pop().unwrap();
    assert_eq!(definition.guides, Some(grid(25.)));
    let apply = application(&master, &target, 1, false);
    let revision =
        f.db.publication_revision(&f.project, &f.publication)
            .await
            .unwrap();
    let preview =
        f.db.publication_master_preview(&f.project, &f.publication, revision, &apply)
            .await
            .unwrap();
    assert_eq!(preview.overridden_page_ids, vec![target.clone()]);
    assert_eq!(layout(&f).await, before);
    assert_eq!(
        guides::read(
            preview
                .layout
                .pages
                .iter()
                .find(|p| p.id == target)
                .unwrap()
        )
        .unwrap(),
        Some(grid(25.))
    );
    assert_eq!(
        f.db.execute_publication(
            f.request(PublicationAction::ApplyMaster(apply.clone()))
                .await
        )
        .await
        .unwrap_err()
        .code,
        "CONFLICT"
    );
    assert_eq!(layout(&f).await, before);
    let request = f
        .request(PublicationAction::ApplyMaster(ApplyMaster {
            overwrite_local: true,
            ..apply
        }))
        .await;
    let receipt = f.db.execute_publication(request.clone()).await.unwrap();
    let after = layout(&f).await;
    assert_eq!(after, preview.layout);
    assert_eq!(
        encode(&f.db.execute_publication(request).await.unwrap()).unwrap(),
        encode(&receipt).unwrap()
    );
    sources_unchanged(
        before.pages.iter().find(|p| p.id == target).unwrap(),
        after.pages.iter().find(|p| p.id == target).unwrap(),
    );
    assert!(
        !masters::states(&after)
            .unwrap()
            .iter()
            .find(|s| s.page_id == target)
            .unwrap()
            .locally_changed
    );
    let saved = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "母版网格确认".into(),
        }))
        .await;
    let version = saved.command_id.clone();
    f.db.execute_publication(saved).await.unwrap();
    let frozen = layout(&f).await;
    guide(&f, &target, grid(55.)).await;
    assert!(
        masters::states(&layout(&f).await)
            .unwrap()
            .iter()
            .find(|s| s.page_id == target)
            .unwrap()
            .locally_changed
    );
    guide(&f, &source, grid(70.)).await;
    let before_update = layout(&f).await;
    f.db.execute_publication(save(&f, &source, &master, Some(1), Some(true)).await)
        .await
        .unwrap();
    assert_eq!(
        layout(&f).await.pages.iter().find(|p| p.id == target),
        before_update.pages.iter().find(|p| p.id == target)
    );
    let update = application(&master, &target, 2, false);
    assert_eq!(
        f.db.execute_publication(f.request(PublicationAction::ApplyMaster(update)).await)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
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
    assert_eq!(
        f.db.publication_version(&f.project, &f.publication, &version)
            .await
            .unwrap()
            .layout,
        frozen
    );
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
    f.originals_unchanged();
}
#[tokio::test]
async fn legacy_requests_preserve_managed_guides_and_unmanaged_masters_preserve_page_guides() {
    let f = Fixture::new().await;
    let initial = layout(&f).await;
    let source = &initial.pages[1].id;
    let target = &initial.pages[2].id;
    let master = domain::id();
    guide(&f, source, grid(25.)).await;
    f.db.execute_publication(save(&f, source, &master, None, Some(true)).await)
        .await
        .unwrap();
    guide(&f, source, grid(60.)).await;
    let legacy = save(&f, source, &master, Some(1), None).await;
    let encoded = serde_json::to_value(&legacy).unwrap();
    assert!(
        encoded["operations"][0]["action"]["data"]
            .get("includeGuides")
            .is_none()
    );
    let decoded: PublicationCommandRequest = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(&decoded).unwrap(), encoded);
    f.db.execute_publication(decoded).await.unwrap();
    let updated = layout(&f).await;
    assert_eq!(
        masters::masters(&updated).unwrap()[0].guides,
        Some(grid(25.))
    );
    assert_eq!(
        guides::read(updated.pages.iter().find(|p| &p.id == source).unwrap()).unwrap(),
        Some(grid(60.))
    );
    assert!(
        masters::states(&updated)
            .unwrap()
            .iter()
            .find(|s| &s.page_id == source)
            .unwrap()
            .locally_changed
    );
    f.db.execute_publication(save(&f, source, &master, Some(2), Some(false)).await)
        .await
        .unwrap();
    let unmanaged = layout(&f).await;
    assert!(masters::masters(&unmanaged).unwrap()[0].guides.is_none());
    let source_page = unmanaged.pages.iter().find(|p| &p.id == source).unwrap();
    assert!(
        masters::link(source_page)
            .unwrap()
            .unwrap()
            .guide_signature
            .is_none()
    );
    guide(&f, target, grid(90.)).await;
    let before = layout(&f).await;
    let data = application(&master, target, 3, false);
    let preview =
        f.db.publication_master_preview(
            &f.project,
            &f.publication,
            f.db.publication_revision(&f.project, &f.publication)
                .await
                .unwrap(),
            &data,
        )
        .await
        .unwrap();
    assert!(preview.overridden_page_ids.is_empty());
    f.db.execute_publication(f.request(PublicationAction::ApplyMaster(data)).await)
        .await
        .unwrap();
    let after = layout(&f).await;
    assert_eq!(
        guides::read(after.pages.iter().find(|p| &p.id == target).unwrap()).unwrap(),
        Some(grid(90.))
    );
    sources_unchanged(
        before.pages.iter().find(|p| &p.id == target).unwrap(),
        after.pages.iter().find(|p| &p.id == target).unwrap(),
    );
    assert!(
        serde_json::to_value(&masters::masters(&after).unwrap()[0])
            .unwrap()
            .get("guides")
            .is_none()
    );
    f.db.execute_publication(save(&f, source, &master, Some(3), Some(true)).await)
        .await
        .unwrap();
    let protected = layout(&f).await;
    let newer = application(&master, target, 4, false);
    let preview =
        f.db.publication_master_preview(
            &f.project,
            &f.publication,
            f.db.publication_revision(&f.project, &f.publication)
                .await
                .unwrap(),
            &newer,
        )
        .await
        .unwrap();
    assert_eq!(preview.overridden_page_ids, vec![target.clone()]);
    assert_eq!(
        f.db.execute_publication(f.request(PublicationAction::ApplyMaster(newer)).await)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    assert_eq!(layout(&f).await, protected);
    f.originals_unchanged();
}
#[tokio::test]
async fn guided_bulk_apply_is_atomic_and_untouched_managed_grid_does_not_require_manual_override() {
    let f = Fixture::new().await;
    let initial = layout(&f).await;
    let source = &initial.pages[1].id;
    let targets = vec![initial.pages[0].id.clone(), initial.pages[2].id.clone()];
    let master = domain::id();
    guide(&f, source, grid(25.)).await;
    f.db.execute_publication(save(&f, source, &master, None, Some(true)).await)
        .await
        .unwrap();
    let mut data = application(&master, &targets[0], 1, false);
    data.page_ids = targets.clone();
    f.db.execute_publication(
        f.request(PublicationAction::ApplyMaster(data.clone()))
            .await,
    )
    .await
    .unwrap();
    guide(&f, source, grid(70.)).await;
    f.db.execute_publication(save(&f, source, &master, Some(1), Some(true)).await)
        .await
        .unwrap();
    data.master_revision = 2;
    data.instance_id = domain::id();
    let before = layout(&f).await;
    let preview =
        f.db.publication_master_preview(
            &f.project,
            &f.publication,
            f.db.publication_revision(&f.project, &f.publication)
                .await
                .unwrap(),
            &data,
        )
        .await
        .unwrap();
    assert!(preview.overridden_page_ids.is_empty());
    f.db.execute_publication(f.request(PublicationAction::ApplyMaster(data)).await)
        .await
        .unwrap();
    let after = layout(&f).await;
    for id in &targets {
        let page = after.pages.iter().find(|p| &p.id == id).unwrap();
        assert_eq!(guides::read(page).unwrap(), Some(grid(70.)));
        assert!(!masters::locally_changed(page, &masters::link(page).unwrap().unwrap()).unwrap());
        sources_unchanged(before.pages.iter().find(|p| &p.id == id).unwrap(), page);
    }
    guide(&f, &targets[1], grid(85.)).await;
    guide(&f, source, grid(95.)).await;
    f.db.execute_publication(save(&f, source, &master, Some(2), Some(true)).await)
        .await
        .unwrap();
    let before = layout(&f).await;
    let mut blocked = application(&master, &targets[0], 3, false);
    blocked.page_ids = targets.clone();
    assert_eq!(
        f.db.execute_publication(f.request(PublicationAction::ApplyMaster(blocked)).await)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    assert_eq!(layout(&f).await, before);
    f.originals_unchanged();
}
#[tokio::test]
async fn capture_requires_stored_guides_and_new_optional_fields_reject_explicit_null() {
    let f = Fixture::new().await;
    let before = layout(&f).await;
    let source = &before.pages[1].id;
    let master = domain::id();
    let request = save(&f, source, &master, None, Some(true)).await;
    assert_eq!(
        f.db.execute_publication(request.clone())
            .await
            .unwrap_err()
            .code,
        "VALIDATION_ERROR"
    );
    assert_eq!(layout(&f).await, before);
    let mut bad = serde_json::to_value(request).unwrap();
    bad["operations"][0]["action"]["data"]["includeGuides"] = Value::Null;
    assert!(serde_json::from_value::<PublicationCommandRequest>(bad).is_err());
    guide(&f, source, grid(25.)).await;
    let allowed = save(&f, source, &master, None, Some(true)).await;
    assert_eq!(
        CommandHandler::new(&f.db)
            .execute_publication(allowed.clone(), &CommandContext::read_only(&f.project))
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    f.db.execute_publication(allowed).await.unwrap();
    let valid = layout(&f).await;
    let mut definition = serde_json::to_value(&masters::masters(&valid).unwrap()[0]).unwrap();
    definition["guides"] = Value::Null;
    assert!(serde_json::from_value::<masters::PageMaster>(definition).is_err());
    let mut link = serde_json::to_value(
        masters::link(valid.pages.iter().find(|p| &p.id == source).unwrap())
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    link["guideSignature"] = Value::Null;
    assert!(serde_json::from_value::<masters::MasterLink>(link).is_err());
    f.originals_unchanged();
}
#[tokio::test]
async fn corrupt_master_guide_definition_is_rejected_but_valid_history_can_recover_it() {
    let f = Fixture::new().await;
    let first = layout(&f).await;
    let source = &first.pages[1].id;
    let master = domain::id();
    guide(&f, source, grid(25.)).await;
    f.db.execute_publication(save(&f, source, &master, None, Some(true)).await)
        .await
        .unwrap();
    let frozen = layout(&f).await;
    let capture = f
        .request(PublicationAction::CaptureVersion(VersionLabel {
            label: "有效母版网格".into(),
        }))
        .await;
    let version = capture.command_id.clone();
    f.db.execute_publication(capture).await.unwrap();
    let mut spec: Value = serde_json::from_str(&frozen.publication.spec_json).unwrap();
    spec["focalroomPageMastersV1"][0]["guides"] = Value::Null;
    sqlx::query("UPDATE publications SET spec_json=? WHERE id=?")
        .bind(spec.to_string())
        .bind(&f.publication)
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
    f.db.execute_publication(
        f.request(PublicationAction::RestoreVersion(VersionReference {
            version_id: version,
        }))
        .await,
    )
    .await
    .unwrap();
    assert_eq!(layout(&f).await, frozen);
    f.originals_unchanged();
}

#[tokio::test]
async fn guided_master_capacity_failure_has_same_preview_and_apply_result_without_partial_changes()
{
    let f = Fixture::new().await;
    let initial = layout(&f).await;
    let source = &initial.pages[1].id;
    let target = &initial.pages[2].id;
    let master = domain::id();
    guide(&f, source, grid(25.)).await;
    f.db.execute_publication(save(&f, source, &master, None, Some(true)).await)
        .await
        .unwrap();
    sqlx::query("UPDATE publication_pages SET layout_json=? WHERE id=?")
        .bind(json!({"legacy":"汉".repeat(20000)}).to_string())
        .bind(target)
        .execute(&f.db.pool)
        .await
        .unwrap();
    let before = layout(&f).await;
    let data = application(&master, target, 1, true);
    let revision =
        f.db.publication_revision(&f.project, &f.publication)
            .await
            .unwrap();
    let preview =
        f.db.publication_master_preview(&f.project, &f.publication, revision, &data)
            .await
            .unwrap_err();
    let applied =
        f.db.execute_publication(f.request(PublicationAction::ApplyMaster(data)).await)
            .await
            .unwrap_err();
    assert_eq!(preview.code, applied.code);
    assert_eq!(preview.message, applied.message);
    assert!(applied.message.contains("原版面已保留"));
    assert_eq!(layout(&f).await, before);
    f.originals_unchanged();
}
#[test]
fn legacy_master_signature_fixture_is_unchanged_and_absent_guide_scope_stays_absent() {
    let mut page = versions::LayoutPage {
        id: "00000000-0000-4000-8000-000000000002".into(),
        page_number: 1,
        side: "right".into(),
        width_mm: 190.,
        height_mm: 257.,
        margin_mm: 15.,
        bleed_mm: 3.,
        layout_json: None,
        frames: vec![versions::LayoutFrame {
            id: "00000000-0000-4000-8000-000000000001".into(),
            x_mm: 15.,
            y_mm: 20.,
            width_mm: 100.,
            height_mm: 200.,
            fit_mode: "fit".into(),
            asset_id: None,
            rendition_id: None,
            crop_x: 0.5,
            crop_y: 0.5,
            crop_zoom: 1.,
            source: None,
        }],
        texts: vec![],
    };
    // 固定v1签名样本，由原格式的独立JSON/SHA-256计算记录，不在断言中重算新算法。
    let link = masters::MasterLink {
        id: "00000000-0000-4000-8000-000000000003".into(),
        revision: 1,
        signature: "49b04efbddd022de693f6cc31e89e4c30e85a7fe0cb50c99588804b42c2fa34b".into(),
        frames: vec![masters::Binding {
            slot_id: "00000000-0000-4000-8000-000000000004".into(),
            object_id: page.frames[0].id.clone(),
        }],
        texts: vec![],
        guide_signature: None,
    };
    assert!(!masters::locally_changed(&page, &link).unwrap());
    assert!(
        serde_json::to_value(&link)
            .unwrap()
            .get("guideSignature")
            .is_none()
    );
    guides::apply_to_page(&mut page, &grid(25.)).unwrap();
    assert!(!masters::locally_changed(&page, &link).unwrap());
}
