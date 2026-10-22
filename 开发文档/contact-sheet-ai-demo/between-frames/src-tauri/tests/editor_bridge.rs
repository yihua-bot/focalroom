use between_frames_lib::{
    db::Store, domain, editor_bridge, image_pipeline, importer::ImportManager,
};
use image::{Rgb, RgbImage};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

struct Fixture {
    temp: tempfile::TempDir,
    store: Store,
    project: String,
    source: PathBuf,
    batch: PathBuf,
    storage: PathBuf,
    manifest: Value,
}
async fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("原片 ' $().png");
    RgbImage::from_pixel(32, 24, Rgb([10, 20, 30]))
        .save(&source)
        .unwrap();
    let batch = temp.path().join("batch");
    std::fs::create_dir(&batch).unwrap();
    let render = batch.join("render.png");
    RgbImage::from_pixel(32, 24, Rgb([110, 120, 130]))
        .save(&render)
        .unwrap();
    let store = Store::open(&temp.path().join("db.sqlite")).await.unwrap();
    let project = store.create("外部照片", "光线").await.unwrap().id;
    let imports = ImportManager::default();
    let job = imports
        .start(
            store.clone(),
            temp.path().join("cache"),
            project.clone(),
            vec![source.clone()],
            |_| {},
        )
        .await
        .unwrap();
    loop {
        let job = imports.get(&job).await.unwrap();
        if job.status == "completed" {
            assert_eq!(job.imported, 1);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let manifest = json!({"schemaVersion":3,"transferBatchId":domain::id(),"provider":"LIGHTROOM_CLASSIC","catalogFingerprint":"c".repeat(64),"issuedAt":"2026-10-22T00:00:00Z","items":[{
        "sourceSha256":image_pipeline::fingerprint(&source).unwrap(),"externalPhotoId":"42","externalVariantId":"virtual-copy-42",
        "variantId":domain::id(),"renderId":domain::id(),"renderFilename":"render.png","renderSha256":image_pipeline::fingerprint(&render).unwrap(),"width":32,"height":24,"bitDepth":8,"mediaFormat":"PNG"
    }]});
    let storage = temp.path().join("editor-renders");
    Fixture {
        temp,
        store,
        project,
        source,
        batch,
        storage,
        manifest,
    }
}
fn write(batch: &Path, value: &Value) {
    std::fs::write(
        batch.join(".focalroom-manifest.json"),
        serde_json::to_vec(value).unwrap(),
    )
    .unwrap();
}
async fn count(store: &Store) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM renditions")
        .fetch_one(&store.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn import_replay_restart_and_new_batch_preserve_originals_and_formal_work() {
    let mut f = fixture().await;
    let before = serde_json::to_value(f.store.get(&f.project).await.unwrap()).unwrap();
    let source_sha = image_pipeline::fingerprint(&f.source).unwrap();
    write(&f.batch, &f.manifest);
    assert_eq!(
        editor_bridge::import_batch(&f.store, &f.project, &f.batch, &f.storage)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        editor_bridge::import_batch(&f.store, &f.project, &f.batch, &f.storage)
            .await
            .unwrap(),
        0
    );
    f.manifest["transferBatchId"] = json!(domain::id());
    write(&f.batch, &f.manifest);
    assert_eq!(
        editor_bridge::import_batch(&f.store, &f.project, &f.batch, &f.storage)
            .await
            .unwrap(),
        0
    );
    assert_eq!(count(&f.store).await, 1);
    let versions = editor_bridge::list_versions(&f.store, &f.project, &f.storage)
        .await
        .unwrap();
    assert_eq!(versions.len(), 1);
    assert!(Path::new(&versions[0].preview_path).is_file());
    assert_eq!(
        serde_json::to_value(f.store.get(&f.project).await.unwrap()).unwrap(),
        before
    );
    assert_eq!(image_pipeline::fingerprint(&f.source).unwrap(), source_sha);
    let other = f.store.create("另一个项目", "").await.unwrap();
    assert!(
        editor_bridge::import_batch(&f.store, &other.id, &f.batch, &f.storage)
            .await
            .is_err()
    );
    let db = f.temp.path().join("db.sqlite");
    f.store.pool.close().await;
    let reopened = Store::open(&db).await.unwrap();
    assert_eq!(
        editor_bridge::list_versions(&reopened, &f.project, &f.storage)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn rejects_tampered_render_metadata_provider_and_source_without_partial_database_writes() {
    for kind in [
        "hash",
        "dimensions",
        "source",
        "provider",
        "filename",
        "batch",
    ] {
        let mut f = fixture().await;
        match kind {
            "hash" => f.manifest["items"][0]["renderSha256"] = json!("0".repeat(64)),
            "dimensions" => f.manifest["items"][0]["width"] = json!(99),
            "source" => f.manifest["items"][0]["sourceSha256"] = json!("0".repeat(64)),
            "provider" => f.manifest["provider"] = json!("OTHER"),
            "filename" => f.manifest["items"][0]["renderFilename"] = json!("../outside.png"),
            _ => {
                let mut invalid = f.manifest["items"][0].clone();
                invalid["sourceSha256"] = json!("f".repeat(64));
                f.manifest["items"].as_array_mut().unwrap().push(invalid);
            }
        }
        write(&f.batch, &f.manifest);
        assert!(
            editor_bridge::import_batch(&f.store, &f.project, &f.batch, &f.storage)
                .await
                .is_err(),
            "{kind}"
        );
        assert_eq!(count(&f.store).await, 0, "{kind}");
    }
}

#[tokio::test]
async fn provider_and_catalog_keep_independent_branches() {
    let mut f = fixture().await;
    for (provider, catalog) in [
        ("LIGHTROOM_CLASSIC", "c"),
        ("CAPTURE_ONE", "c"),
        ("LIGHTROOM_CLASSIC", "d"),
    ] {
        f.manifest["provider"] = json!(provider);
        f.manifest["catalogFingerprint"] = json!(catalog.repeat(64));
        f.manifest["transferBatchId"] = json!(domain::id());
        write(&f.batch, &f.manifest);
        assert_eq!(
            editor_bridge::import_batch(&f.store, &f.project, &f.batch, &f.storage)
                .await
                .unwrap(),
            1
        );
    }
    assert_eq!(count(&f.store).await, 3);
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_escape_is_rejected() {
    let mut f = fixture().await;
    std::os::unix::fs::symlink(&f.source, f.batch.join("escape.png")).unwrap();
    f.manifest["items"][0]["renderFilename"] = json!("escape.png");
    write(&f.batch, &f.manifest);
    assert!(
        editor_bridge::import_batch(&f.store, &f.project, &f.batch, &f.storage)
            .await
            .is_err()
    );
    assert_eq!(count(&f.store).await, 0);
}

#[tokio::test]
#[ignore = "requires a locally built FOCALROOM_BRIDGE_BINARY; no editor or network"]
async fn actual_swift_bridge_batch_imports_in_rust() {
    let f = fixture().await;
    let input = f.temp.path().join("request.json");
    let result = f.temp.path().join("result.json");
    let request = json!({"provider":"LIGHTROOM_CLASSIC","catalogId":"测试.lrcat","destination":f.temp.path(),"items":[{"sourcePath":f.source,"renderPath":f.batch.join("render.png"),"externalPhotoId":"42","externalVariantId":"42"}]});
    std::fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let status = std::process::Command::new(
        std::env::var("FOCALROOM_BRIDGE_BINARY").expect("set FOCALROOM_BRIDGE_BINARY"),
    )
    .args([
        "--request",
        input.to_str().unwrap(),
        result.to_str().unwrap(),
    ])
    .status()
    .unwrap();
    assert!(status.success());
    let response: Value = serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap();
    let directory = PathBuf::from(response["directory"].as_str().unwrap());
    assert_eq!(
        editor_bridge::import_batch(&f.store, &f.project, &directory, &f.storage)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        editor_bridge::list_versions(&f.store, &f.project, &f.storage)
            .await
            .unwrap()
            .len(),
        1
    );
}
