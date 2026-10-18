use between_frames_lib::{db::Store, export, image_pipeline, shared_cloud, shared_export};

#[tokio::test]
async fn shared_copy_export_uses_derived_previews_and_redacts_private_state() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = store
        .create("共享副本", "私有意图不进入共享包")
        .await
        .unwrap();
    let source = dir.path().join("source.jpg");
    image::RgbImage::from_pixel(48, 32, image::Rgb([12, 100, 180]))
        .save(&source)
        .unwrap();
    store
        .insert_photo(
            &project.id,
            &image_pipeline::prepare(&source, &dir.path().join("cache")).unwrap(),
        )
        .await
        .unwrap();
    let hash = image_pipeline::fingerprint(&source).unwrap();
    let project = store.get(&project.id).await.unwrap();
    let destination = dir.path().join("shared-copy");
    export::write_with_options(&project, "shared", &destination, true).unwrap();
    let manifest = std::fs::read_to_string(destination.join("manifest.json")).unwrap();
    assert!(!manifest.contains(dir.path().to_str().unwrap()));
    assert!(!manifest.contains("私有意图"));
    let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(manifest["media_policy"]["ai_upload_authorized"], false);
    let bytes =
        std::fs::read(destination.join(format!("{}.preview.jpg", project.photos[0].id))).unwrap();
    let preview = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg).unwrap();
    assert!(preview.width() <= 2048 && preview.height() <= 2048);
    assert!(export::write_with_options(&project, "shared", &destination, false).is_err());
    assert_eq!(image_pipeline::fingerprint(&source).unwrap(), hash);
}
#[test]
fn shared_preview_rejects_oversize_corrupt_and_symbolic_sources() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("too-wide.jpg");
    image::RgbImage::new(2049, 2).save(&path).unwrap();
    assert!(shared_export::preview_bytes(&path).is_err());
    let corrupt = dir.path().join("corrupt.jpg");
    std::fs::write(&corrupt, b"not jpeg").unwrap();
    assert!(shared_export::preview_bytes(&corrupt).is_err());
    #[cfg(unix)]
    {
        let link = dir.path().join("link.jpg");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(shared_export::preview_bytes(&link).is_err());
    }
}
#[tokio::test]
async fn sharing_rejects_missing_consent_and_insecure_remote_before_freezing() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("test.sqlite")).await.unwrap();
    let project = store.create("local", "").await.unwrap();
    let input = |consent| shared_cloud::ShareRequest {
        project_id: project.id.clone(),
        expected_revision: project.revision,
        base_url: "http://remote.example.com".into(),
        web_url: "https://example.com".into(),
        action: "resume".into(),
        preview_upload_authorized: consent,
    };
    assert_eq!(
        shared_cloud::share(&store, input(false), "")
            .await
            .unwrap_err()
            .code,
        "CONSENT_REQUIRED"
    );
    assert_eq!(
        shared_cloud::share(&store, input(true), "")
            .await
            .unwrap_err()
            .code,
        "CONFIG_INVALID"
    );
    assert_eq!(
        shared_cloud::status(&store, &project.id).await.unwrap()["authority"],
        "local"
    );
}
