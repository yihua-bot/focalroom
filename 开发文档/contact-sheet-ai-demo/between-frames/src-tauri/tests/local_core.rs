use between_frames_lib::{
    db::Store, domain::ImportJob, export, image_pipeline, importer::ImportManager,
};
use image::{ImageFormat, Rgb, RgbImage};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

fn fixture(directory: &Path, index: u32, format: ImageFormat) -> PathBuf {
    let extension = match format {
        ImageFormat::Png => "png",
        ImageFormat::WebP => "webp",
        ImageFormat::Tiff => "tiff",
        _ => "jpg",
    };
    let path = directory.join(format!("frame-{index:03}.{extension}"));
    let image = RgbImage::from_fn(128, 96, |x, y| {
        Rgb([
            ((x + index) % 256) as u8,
            ((y + index * 3) % 256) as u8,
            (index % 256) as u8,
        ])
    });
    image.save_with_format(&path, format).unwrap();
    path
}

async fn finished(imports: &ImportManager, id: &str) -> ImportJob {
    for _ in 0..12000 {
        let job = imports.get(id).await.unwrap();
        if job.status != "running" {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("导入任务未在测试时限内完成");
}

#[tokio::test]
async fn two_hundred_photos_survive_restart_without_changing_sources() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("sources");
    std::fs::create_dir(&source).unwrap();
    let paths: Vec<_> = (0..200)
        .map(|index| fixture(&source, index, ImageFormat::Jpeg))
        .collect();
    let before: Vec<_> = paths
        .iter()
        .map(|path| image_pipeline::fingerprint(path).unwrap())
        .collect();
    let database = directory.path().join("projects.sqlite");
    let cache = directory.path().join("cache");
    let store = Store::open(&database).await.unwrap();
    let project = store.create("200 张重启验收", "观察光线").await.unwrap();
    let imports = ImportManager::default();
    let job = imports
        .start(
            store.clone(),
            cache.clone(),
            project.id.clone(),
            paths.clone(),
            |_| {},
        )
        .await
        .unwrap();
    let job = finished(&imports, &job).await;
    assert_eq!(job.imported, 200);
    assert!(job.failures.is_empty());
    let mut project = store.get(&project.id).await.unwrap();
    let selected: Vec<_> = project
        .photos
        .iter()
        .take(25)
        .map(|photo| photo.id.clone())
        .collect();
    project = store
        .decide_many(&project.id, &selected, "keep", project.revision)
        .await
        .unwrap();
    project = store
        .rate_many(&project.id, &selected[..2], 4, project.revision)
        .await
        .unwrap();
    let first = project.photos[0].id.clone();
    project = store
        .note(&project.id, &first, "画面之间的停顿", project.revision)
        .await
        .unwrap();
    let mut order = project.sequence.clone();
    order.reverse();
    project = store
        .reorder(&project.id, &order, project.revision)
        .await
        .unwrap();
    let expected = serde_json::to_value(&project).unwrap();
    store.pool.close().await;
    let reopened = Store::open(&database).await.unwrap();
    let restored = reopened.get(&project.id).await.unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), expected);
    assert_eq!(restored.photos.len(), 200);
    assert_eq!(restored.sequence.len(), 25);
    assert_eq!(restored.photos[0].rating, 4);
    assert_eq!(
        before,
        paths
            .iter()
            .map(|path| image_pipeline::fingerprint(path).unwrap())
            .collect::<Vec<_>>()
    );
    let thumb = image::open(&restored.photos[0].thumbnail_path).unwrap();
    assert!(thumb.width() <= 320 && thumb.height() <= 320);
    let preview = image::open(&restored.photos[0].preview_path).unwrap();
    assert!(preview.width() <= 1400 && preview.height() <= 1400);
    assert_eq!((preview.width(), preview.height()), (128, 96));
    println!(
        "PASS 200 JPEG -> 25 keep -> rating -> note -> reorder -> DB restart -> identical snapshot / 200 SHA256 unchanged"
    );
}

#[tokio::test]
async fn batch_edits_are_atomic_scoped_and_repeated_status_is_a_noop() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let a = store.create("A", "").await.unwrap();
    let b = store.create("B", "").await.unwrap();
    for index in 0..3 {
        let prepared = image_pipeline::prepare(
            &fixture(directory.path(), index, ImageFormat::Jpeg),
            &directory.path().join("cache"),
        )
        .unwrap();
        store
            .insert_photo(if index < 2 { &a.id } else { &b.id }, &prepared)
            .await
            .unwrap();
    }
    let a = store.get(&a.id).await.unwrap();
    let foreign = store.get(&b.id).await.unwrap().photos[0].id.clone();
    let ids: Vec<_> = a.photos.iter().map(|photo| photo.id.clone()).collect();
    let expected = serde_json::to_value(&a).unwrap();
    for rating in [false, true] {
        let failure = if rating {
            store
                .rate_many(&a.id, &[ids[0].clone(), foreign.clone()], 4, a.revision)
                .await
        } else {
            store
                .decide_many(
                    &a.id,
                    &[ids[0].clone(), foreign.clone()],
                    "keep",
                    a.revision,
                )
                .await
        };
        assert_eq!(failure.unwrap_err().code, "NOT_FOUND");
        assert_eq!(
            serde_json::to_value(store.get(&a.id).await.unwrap()).unwrap(),
            expected
        );
    }
    assert_eq!(
        store
            .decide_many(&a.id, &[ids[0].clone(), ids[0].clone()], "keep", a.revision)
            .await
            .unwrap_err()
            .code,
        "INVALID_INPUT"
    );
    let kept = store
        .decide_many(&a.id, &ids, "keep", a.revision)
        .await
        .unwrap();
    assert_eq!(kept.revision, a.revision + 1);
    assert_eq!(kept.events.len(), a.events.len() + 1);
    assert_eq!(kept.sequence, ids);
    let mut order = ids.clone();
    order.reverse();
    let reordered = store.reorder(&a.id, &order, kept.revision).await.unwrap();
    assert_eq!(
        serde_json::to_value(
            store
                .decide_many(&a.id, &ids, "keep", reordered.revision)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&reordered).unwrap()
    );
    let rated = store
        .rate_many(&a.id, &ids, 4, reordered.revision)
        .await
        .unwrap();
    assert_eq!(rated.sequence, order);
    assert!(
        rated
            .photos
            .iter()
            .all(|photo| photo.rating == 4 && photo.status == "keep")
    );
    assert_eq!(
        serde_json::to_value(
            store
                .rate_many(&a.id, &ids, 4, rated.revision)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&rated).unwrap()
    );
    assert_eq!(
        store
            .rate_many(&a.id, &ids, 6, rated.revision)
            .await
            .unwrap_err()
            .code,
        "INVALID_INPUT"
    );
    let cleared = store
        .decide_many(&a.id, &ids[..1], "undecided", rated.revision)
        .await
        .unwrap();
    assert_eq!(cleared.sequence, vec![ids[1].clone()]);
    assert_eq!(cleared.photos[0].rating, 4);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&store.pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn rating_migration_upgrades_old_database_without_changing_legacy_data() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("legacy.sqlite");
    let migrations = directory.path().join("legacy-migrations");
    std::fs::create_dir(&migrations).unwrap();
    std::fs::write(
        migrations.join("0001_local_core.sql"),
        include_str!("../migrations/0001_local_core.sql"),
    )
    .unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&database)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(migrations.as_path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    let prepared = image_pipeline::prepare(
        &fixture(directory.path(), 0, ImageFormat::Jpeg),
        &directory.path().join("cache"),
    )
    .unwrap();
    sqlx::query("INSERT INTO projects(id,title,brief,revision,created_at,updated_at) VALUES('p','旧项目','原意图',7,'before','before')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO photos(id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,thumbnail_path,preview_path,created_at) VALUES('a','p',1,?,?,?,?,?,?,?,?,'before')")
        .bind(prepared.path.to_string_lossy().as_ref()).bind(&prepared.filename).bind(prepared.size as i64).bind(&prepared.hash).bind(prepared.width as i64).bind(prepared.height as i64).bind(prepared.thumbnail.to_string_lossy().as_ref()).bind(prepared.preview.to_string_lossy().as_ref()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO photo_decisions(project_id,photo_id,status,user_note,updated_at) VALUES('p','a','keep','迁移前备注','before')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO sequence_items VALUES('p','a',0,'before')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO decision_events VALUES('e','p','a','user','note',NULL,'{}','reason','{}','before')").execute(&pool).await.unwrap();
    pool.close().await;
    let store = Store::open(&database).await.unwrap();
    let project = store.get("p").await.unwrap();
    assert_eq!((project.revision, project.photos[0].rating), (7, 0));
    assert_eq!(project.photos[0].note, "迁移前备注");
    assert_eq!(project.photos[0].content_sha256, prepared.hash);
    assert_eq!(project.sequence, vec!["a"]);
    assert_eq!(project.events[0].id, "e");
    let expected = serde_json::to_value(&project).unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        4
    );
    store.pool.close().await;
    let reopened = Store::open(&database).await.unwrap();
    assert_eq!(
        serde_json::to_value(reopened.get("p").await.unwrap()).unwrap(),
        expected
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&reopened.pool)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn revision_conflicts_and_invalid_operations_roll_back_every_projection_and_event() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let project = store.create("事务测试", "").await.unwrap();
    let prepared = image_pipeline::prepare(
        &fixture(directory.path(), 1, ImageFormat::Jpeg),
        &directory.path().join("cache"),
    )
    .unwrap();
    store.insert_photo(&project.id, &prepared).await.unwrap();
    let project = store.get(&project.id).await.unwrap();
    let photo = &project.photos[0].id;
    let kept = store
        .decide(&project.id, photo, "keep", project.revision)
        .await
        .unwrap();
    assert_eq!(
        store
            .note(&project.id, photo, "旧窗口修改", project.revision)
            .await
            .unwrap_err()
            .code,
        "REVISION_CONFLICT"
    );
    assert_eq!(
        store
            .reorder(&project.id, &[photo.clone(), photo.clone()], kept.revision)
            .await
            .unwrap_err()
            .code,
        "INVALID_SEQUENCE"
    );
    assert_eq!(
        store
            .decide(&project.id, "missing", "reject", kept.revision)
            .await
            .unwrap_err()
            .code,
        "NOT_FOUND"
    );
    assert_eq!(
        serde_json::to_value(store.get(&project.id).await.unwrap()).unwrap(),
        serde_json::to_value(kept).unwrap()
    );
}

#[tokio::test]
async fn comparisons_are_evidence_only_and_exports_are_escaped_and_read_only() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let mut project = store
        .create("<script>alert(1)</script>", "光 & 影")
        .await
        .unwrap();
    let mut paths = Vec::new();
    for index in 1..3 {
        let path = fixture(directory.path(), index, ImageFormat::Png);
        let prepared = image_pipeline::prepare(&path, &directory.path().join("cache")).unwrap();
        store.insert_photo(&project.id, &prepared).await.unwrap();
        paths.push(path);
    }
    project = store.get(&project.id).await.unwrap();
    let left = project.photos[0].id.clone();
    let right = project.photos[1].id.clone();
    project = store
        .decide(&project.id, &left, "keep", project.revision)
        .await
        .unwrap();
    project = store
        .decide(&project.id, &right, "reject", project.revision)
        .await
        .unwrap();
    let before = project.photos.clone();
    let sequence = project.sequence.clone();
    project = store
        .compare(
            &project.id,
            &left,
            &right,
            Some(&right),
            "留白",
            project.revision,
        )
        .await
        .unwrap();
    project = store
        .compare(&project.id, &left, &right, None, "不确定", project.revision)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(&project.photos).unwrap()
    );
    assert_eq!(sequence, project.sequence);
    project = store
        .note(&project.id, &left, "=HYPERLINK(\"bad\")", project.revision)
        .await
        .unwrap();
    let json = export::render(&project, "json").unwrap();
    let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert!(value["events"][0]["next"].is_object());
    assert!(value["photos"][0].get("originalPath").is_none());
    let private = String::from_utf8(json).unwrap();
    assert!(!private.contains(&paths[0].to_string_lossy().to_string()));
    let portable: serde_json::Value =
        serde_json::from_slice(&export::render_with_options(&project, "json", true).unwrap())
            .unwrap();
    assert!(portable["photos"][0]["originalPath"].is_string());
    assert!(
        String::from_utf8(export::render(&project, "csv").unwrap())
            .unwrap()
            .contains("'=HYPERLINK")
    );
    let html = String::from_utf8(export::render(&project, "html").unwrap()).unwrap();
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>"));
    let before = image_pipeline::fingerprint(&paths[0]).unwrap();
    assert_eq!(
        export::write(&project, "json", &paths[0]).unwrap_err().code,
        "ORIGINAL_PROTECTED"
    );
    let target = directory.path().join("project.json");
    let receipt = export::write(&project, "json", &target).unwrap();
    assert_eq!(
        receipt.sha256,
        image_pipeline::fingerprint(&target).unwrap()
    );
    assert!(export::write(&project, "json", &target).is_err());
    assert_eq!(before, image_pipeline::fingerprint(&paths[0]).unwrap());
    assert!(project.events.iter().all(|event| {
        serde_json::from_str::<serde_json::Value>(&event.context_json).unwrap()["revision"]
            .is_number()
    }));
}

#[tokio::test]
async fn missing_source_keeps_cache_and_relink_requires_identical_content() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let project = store.create("失联恢复", "").await.unwrap();
    let source = fixture(directory.path(), 1, ImageFormat::Tiff);
    let prepared = image_pipeline::prepare(&source, &directory.path().join("cache")).unwrap();
    store.insert_photo(&project.id, &prepared).await.unwrap();
    let relocated = directory.path().join("relocated.tiff");
    std::fs::rename(&source, &relocated).unwrap();
    let missing = store.get(&project.id).await.unwrap();
    assert!(missing.photos[0].is_missing);
    assert!(Path::new(&missing.photos[0].thumbnail_path).is_file());
    let wrong = image_pipeline::prepare(
        &fixture(directory.path(), 2, ImageFormat::Tiff),
        &directory.path().join("cache"),
    )
    .unwrap();
    assert_eq!(
        store
            .relink(&project.id, &missing.photos[0].id, &wrong, missing.revision)
            .await
            .unwrap_err()
            .code,
        "HASH_MISMATCH"
    );
    let correct = image_pipeline::prepare(&relocated, &directory.path().join("cache")).unwrap();
    let restored = store
        .relink(
            &project.id,
            &missing.photos[0].id,
            &correct,
            missing.revision,
        )
        .await
        .unwrap();
    assert!(!restored.photos[0].is_missing);
    assert_eq!(restored.photos[0].id, missing.photos[0].id);
    assert_eq!(
        restored.photos[0].content_sha256,
        missing.photos[0].content_sha256
    );
}

#[tokio::test]
async fn imports_report_duplicates_bad_files_and_cancel_without_losing_committed_data() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let project = store.create("批次", "").await.unwrap();
    let cache = directory.path().join("cache");
    let files: Vec<_> = [
        ImageFormat::Jpeg,
        ImageFormat::Png,
        ImageFormat::WebP,
        ImageFormat::Tiff,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, format)| fixture(directory.path(), i as u32, format))
    .collect();
    let imports = ImportManager::default();
    let first = imports
        .start(
            store.clone(),
            cache.clone(),
            project.id.clone(),
            files.clone(),
            |_| {},
        )
        .await
        .unwrap();
    assert_eq!(finished(&imports, &first).await.imported, 4);
    let bad = directory.path().join("broken.jpg");
    std::fs::write(&bad, b"not an image").unwrap();
    let mut repeated = files;
    repeated.push(bad);
    let second = imports
        .start(
            store.clone(),
            cache.clone(),
            project.id.clone(),
            repeated,
            |_| {},
        )
        .await
        .unwrap();
    let second = finished(&imports, &second).await;
    assert_eq!(second.skipped, 4);
    assert_eq!(second.failures.len(), 1);
    let more: Vec<_> = (100..300)
        .map(|i| fixture(directory.path(), i, ImageFormat::Jpeg))
        .collect();
    let cancelled = imports
        .start(store.clone(), cache, project.id.clone(), more, |_| {})
        .await
        .unwrap();
    for _ in 0..1000 {
        if imports.get(&cancelled).await.unwrap().completed >= 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    imports.cancel(&cancelled).await.unwrap();
    let final_job = finished(&imports, &cancelled).await;
    assert_eq!(final_job.status, "cancelled");
    assert!(final_job.imported >= 3 && final_job.imported < 200);
    assert_eq!(
        store.get(&project.id).await.unwrap().photos.len(),
        4 + final_job.imported
    );
}

#[tokio::test]
async fn archive_is_recoverable_and_keeps_source_and_journal() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let project = store.create("项目归档", "").await.unwrap();
    let project = store
        .edit(&project.id, "重命名", "意图", project.revision)
        .await
        .unwrap();
    store.archive(&project.id, project.revision).await.unwrap();
    assert!(store.list(false).await.unwrap().is_empty());
    assert_eq!(store.list(true).await.unwrap().len(), 1);
    let restored = store.restore(&project.id).await.unwrap();
    assert_eq!(restored.title, "重命名");
    assert_eq!(restored.events.len(), 4);
    assert_eq!(store.list(false).await.unwrap().len(), 1);
}

#[test]
fn jpeg_exif_orientation_is_applied_to_cache() {
    let directory = tempfile::tempdir().unwrap();
    let source = fixture(directory.path(), 1, ImageFormat::Jpeg);
    let bytes = std::fs::read(&source).unwrap();
    let exif: Vec<u8> = vec![
        b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 1, 3, 0, 1, 0, 0,
        0, 6, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut rotated = bytes[..2].to_vec();
    rotated.extend([0xff, 0xe1]);
    rotated.extend(((exif.len() + 2) as u16).to_be_bytes());
    rotated.extend(exif);
    rotated.extend(&bytes[2..]);
    std::fs::write(&source, rotated).unwrap();
    let prepared = image_pipeline::prepare(&source, &directory.path().join("cache")).unwrap();
    assert_eq!((prepared.width, prepared.height), (96, 128));
}

#[test]
#[cfg(target_os = "macos")]
fn native_heic_and_canon_raw_are_decoded_without_changing_originals() {
    let paths = [
        Path::new("/tmp/between-frames-heic-fixture.heic"),
        Path::new("/tmp/between-frames-raw-fixture.CR2"),
    ];
    if paths.iter().any(|path| !path.is_file()) {
        eprintln!("SKIP native fixtures: 请按测试报告准备 HEIC/CR2 样本");
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    for path in paths {
        let before = image_pipeline::fingerprint(path).unwrap();
        let prepared = image_pipeline::prepare(path, directory.path()).unwrap();
        assert!(prepared.width > 0 && prepared.height > 0);
        assert_eq!(before, image_pipeline::fingerprint(path).unwrap());
        assert!(image::open(prepared.thumbnail).is_ok());
        assert!(image::open(prepared.preview).is_ok());
        println!(
            "PASS native {}",
            path.extension().unwrap().to_string_lossy()
        );
    }
}
