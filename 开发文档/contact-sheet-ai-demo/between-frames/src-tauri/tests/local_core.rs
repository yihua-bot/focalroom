use between_frames_lib::{
    db::Store,
    domain::{ImportJob, ImportOutcome},
    export, image_pipeline,
    importer::ImportManager,
    sidecar,
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

/// 外部软件把评分与选片写进原图旁边的 XMP：本地先当成"待确认的事实"列出来，
/// 点头之后才落到这条记录自己的判断上，且从不改写边车文件。
#[tokio::test]
async fn sidecar_judgements_are_offered_as_facts_and_written_only_on_confirm() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let cache = directory.path().join("cache");
    let project = store.create("边车回流", "").await.unwrap();
    let photo = fixture(directory.path(), 41, ImageFormat::Jpeg);
    let prepared = image_pipeline::prepare(&photo, &cache).unwrap();
    store.insert_photo(&project.id, &prepared).await.unwrap();
    let project = store.get(&project.id).await.unwrap();
    let photo_id = project.photos[0].id.clone();

    std::fs::write(
        directory.path().join("frame-041.xmp"),
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="4" crs:Pick="1" crs:ColorLabel="6"/></rdf:RDF></x:xmpmeta>"#,
    )
    .unwrap();

    let rows = store.sidecar_rows(&project.id).await.unwrap();
    assert_eq!(
        rows[0].original_path,
        photo.canonicalize().unwrap().to_string_lossy().as_ref(),
        "台账存的是规范路径，边车就从这条路径推"
    );
    let proposals = sidecar::proposals_for(&rows).unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].photo_id, photo_id);
    assert_eq!(proposals[0].sidecar_rating, Some(4));
    assert_eq!(proposals[0].local_rating, 0);
    assert_eq!(proposals[0].local_status, "undecided");
    assert!(proposals[0].applies_rating && proposals[0].applies_status);
    assert_eq!(
        proposals[0].color_label,
        Some(6),
        "色标签只报事实，本地没有这一列"
    );

    async fn decisions(store: &Store, photo: &str) -> (i64, String) {
        sqlx::query_as::<_, (i64, String)>(
            "SELECT rating,status FROM photo_decisions WHERE photo_id=?",
        )
        .bind(photo)
        .fetch_one(&store.pool)
        .await
        .unwrap()
    }
    let stale = store
        .apply_sidecar_judgement(&project.id, &photo_id, project.revision + 5)
        .await
        .unwrap_err();
    assert_eq!(stale.code, "REVISION_CONFLICT");
    assert_eq!(
        decisions(&store, &photo_id).await,
        (0, "undecided".to_owned())
    );

    let updated = store
        .apply_sidecar_judgement(&project.id, &photo_id, project.revision)
        .await
        .unwrap();
    assert_eq!(updated.revision, project.revision + 1);
    assert_eq!(decisions(&store, &photo_id).await, (4, "keep".to_owned()));
    let sequenced: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sequence_items WHERE photo_id=?")
        .bind(&photo_id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(sequenced, 1, "选中要进序列，与人工点选片同一条路");

    // 一致之后再说一次：如实拒绝，不写第二条历史，修订号不涨。
    let again = store
        .apply_sidecar_judgement(&project.id, &photo_id, updated.revision)
        .await
        .unwrap_err();
    assert_eq!(again.code, "CONFLICT");
    assert_eq!(
        store.get(&project.id).await.unwrap().revision,
        updated.revision
    );

    std::fs::remove_file(directory.path().join("frame-041.xmp")).unwrap();
    let emptied = sidecar::proposals_for(&store.sidecar_rows(&project.id).await.unwrap()).unwrap();
    assert!(emptied.is_empty(), "边车没了就不该出现在清单上");
    let missing = store
        .apply_sidecar_judgement(&project.id, &photo_id, updated.revision)
        .await
        .unwrap_err();
    assert_eq!(missing.code, "SIDECAR_MISSING");

    // 两处评分互相矛盾时不挑一个：报得出来，落不下去。
    let conflicting = r#"<rdf:Description xmp:Rating="2" crs:Rating="5"/>"#;
    std::fs::write(directory.path().join("frame-041.xmp"), conflicting).unwrap();
    let ambiguous =
        sidecar::proposals_for(&store.sidecar_rows(&project.id).await.unwrap()).unwrap();
    assert_eq!(ambiguous.len(), 1);
    assert!(ambiguous[0].ambiguous_rating && !ambiguous[0].applies_rating);
    assert_eq!(
        store
            .apply_sidecar_judgement(&project.id, &photo_id, updated.revision)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );

    // 边车自始至终只被读过，没被这个程序改写过。
    assert_eq!(
        std::fs::read(directory.path().join("frame-041.xmp")).unwrap(),
        conflicting.as_bytes()
    );
    let photos: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photos WHERE project_id=?")
        .bind(&project.id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(photos, 1, "回填判断不该长出第二条记录");
}

/// 一键回填只碰"本地还空着"的那几条：人工评过分、判过淘汰的，还有两处评分互相矛盾的，
/// 一次点下去也一条不动——这些留给逐条确认。
#[tokio::test]
async fn batch_backfill_fills_only_the_judgements_local_has_not_made_yet() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let cache = directory.path().join("cache");
    let project = store.create("一键回填", "").await.unwrap();
    for index in [41u32, 42, 43, 44, 45] {
        let photo = fixture(directory.path(), index, ImageFormat::Jpeg);
        let prepared = image_pipeline::prepare(&photo, &cache).unwrap();
        store.insert_photo(&project.id, &prepared).await.unwrap();
    }
    let project = store.get(&project.id).await.unwrap();
    let ids: Vec<String> = project
        .photos
        .iter()
        .map(|photo| photo.id.clone())
        .collect();

    // 空白的一条：评分加选中，两样都该被一键填上。
    std::fs::write(
        directory.path().join("frame-041.xmp"),
        r#"<rdf:Description xmp:Rating="4" crs:Pick="1"/>"#,
    )
    .unwrap();
    // 只有旗标、没有评分的一条：状态填 keep，评分留空。
    std::fs::write(
        directory.path().join("frame-042.xmp"),
        r#"<rdf:Description crs:Pick="1"/>"#,
    )
    .unwrap();
    // 人工已经评过 2 星，外部写着 5 星：一键不替他改主意。
    std::fs::write(
        directory.path().join("frame-043.xmp"),
        r#"<rdf:Description xmp:Rating="5"/>"#,
    )
    .unwrap();
    // 两处评分互相矛盾：连本地空着也不猜一个数。
    std::fs::write(
        directory.path().join("frame-044.xmp"),
        r#"<rdf:Description xmp:Rating="2" crs:Rating="5"/>"#,
    )
    .unwrap();
    // 人工判过淘汰、外部写着选中：状态这一半留给逐条确认，评分这一半还能填。
    std::fs::write(
        directory.path().join("frame-045.xmp"),
        r#"<rdf:Description xmp:Rating="3" crs:Pick="1"/>"#,
    )
    .unwrap();
    sqlx::query("UPDATE photo_decisions SET rating=2 WHERE photo_id=?")
        .bind(&ids[2])
        .execute(&store.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE photo_decisions SET status='reject' WHERE photo_id=?")
        .bind(&ids[4])
        .execute(&store.pool)
        .await
        .unwrap();

    async fn decisions(store: &Store, photo: &str) -> (i64, String) {
        sqlx::query_as::<_, (i64, String)>(
            "SELECT rating,status FROM photo_decisions WHERE photo_id=?",
        )
        .bind(photo)
        .fetch_one(&store.pool)
        .await
        .unwrap()
    }
    async fn sequenced(store: &Store, photo: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM sequence_items WHERE photo_id=?")
            .bind(photo)
            .fetch_one(&store.pool)
            .await
            .unwrap()
    }

    let outcome = store
        .backfill_sidecar_judgements(&project.id, project.revision)
        .await
        .unwrap();
    assert_eq!(
        (outcome.applied, outcome.skipped),
        (3, 0),
        "空白的、只有旗标的、以及淘汰那条的评分——三条都填得上"
    );
    assert_eq!(
        outcome.project.revision,
        project.revision + 1,
        "一次一键只涨一格"
    );
    assert_eq!(decisions(&store, &ids[0]).await, (4, "keep".to_owned()));
    assert_eq!(decisions(&store, &ids[1]).await, (0, "keep".to_owned()));
    assert_eq!(
        decisions(&store, &ids[2]).await,
        (2, "undecided".to_owned()),
        "人工评过 2 星的那条一个字节没动"
    );
    assert_eq!(
        decisions(&store, &ids[3]).await,
        (0, "undecided".to_owned()),
        "评分矛盾的不猜数"
    );
    assert_eq!(
        decisions(&store, &ids[4]).await,
        (3, "reject".to_owned()),
        "淘汰是人工判的，外部写着选中也不能捞回保留"
    );
    assert_eq!(sequenced(&store, &ids[0]).await, 1);
    assert_eq!(sequenced(&store, &ids[1]).await, 1);
    assert_eq!(sequenced(&store, &ids[4]).await, 0, "没变成保留的不进序列");

    // 填完之后再来一次：能填的都填过了，如实说没有可回填的，修订号不动。
    let again = store
        .backfill_sidecar_judgements(&project.id, outcome.project.revision)
        .await
        .unwrap_err();
    assert_eq!(again.code, "CONFLICT");
    assert_eq!(
        store.get(&project.id).await.unwrap().revision,
        outcome.project.revision
    );

    // 边车仍然只是被读过，没有被这个程序改写过。
    assert_eq!(
        std::fs::read(directory.path().join("frame-041.xmp")).unwrap(),
        r#"<rdf:Description xmp:Rating="4" crs:Pick="1"/>"#.as_bytes()
    );
    let photos: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photos WHERE project_id=?")
        .bind(&project.id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(photos, 5, "批量回填不长出第二条记录");
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
        38
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
async fn identical_content_from_different_files_stays_two_independent_photos() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let project = store.create("同图重排", "").await.unwrap();
    let cache = directory.path().join("cache");
    let first = fixture(directory.path(), 7, ImageFormat::Jpeg);
    // 同一画面内容存在两个不同文件，重复保存到相册的原片就是这个形状。
    let second = directory.path().join("frame-007-copy.jpg");
    std::fs::copy(&first, &second).unwrap();
    for path in [&first, &second] {
        let prepared = image_pipeline::prepare(path, &cache).unwrap();
        assert!(
            store
                .insert_photo(&project.id, &prepared)
                .await
                .unwrap()
                .inserted()
        );
    }
    let mut project = store.get(&project.id).await.unwrap();
    assert_eq!(project.photos.len(), 2);
    assert_eq!(
        project.photos[0].content_sha256,
        project.photos[1].content_sha256
    );
    assert_ne!(project.photos[0].index, project.photos[1].index);
    let ids: Vec<_> = project
        .photos
        .iter()
        .map(|photo| photo.id.clone())
        .collect();
    // 两条记录各自保留决定与序列位置，不互相覆盖。
    project = store
        .decide_many(&project.id, &ids, "keep", project.revision)
        .await
        .unwrap();
    project = store
        .reorder(
            &project.id,
            &[ids[1].clone(), ids[0].clone()],
            project.revision,
        )
        .await
        .unwrap();
    assert_eq!(project.sequence, vec![ids[1].clone(), ids[0].clone()]);
    project = store
        .decide(&project.id, &ids[1], "reject", project.revision)
        .await
        .unwrap();
    let statuses: Vec<_> = project
        .photos
        .iter()
        .map(|photo| photo.status.clone())
        .collect();
    assert_eq!(statuses, vec!["keep".to_owned(), "reject".to_owned()]);
    // 同一来源文件重复导入仍跳过，不新增记录。
    let prepared = image_pipeline::prepare(&first, &cache).unwrap();
    assert!(
        matches!(
            store.insert_photo(&project.id, &prepared).await.unwrap(),
            ImportOutcome::Duplicate { .. }
        ),
        "同路径同内容应判为重复来源"
    );
    assert_eq!(store.get(&project.id).await.unwrap().photos.len(), 2);
    // 相同内容只对应一个母版资产，两条照片记录共用它及其派生缓存。
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM source_assets WHERE project_id=? AND content_sha256=?",
        )
        .bind(&project.id)
        .bind(&prepared.hash)
        .fetch_one(&store.pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM asset_aliases")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(DISTINCT asset_id) FROM asset_aliases")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM asset_representations")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        2
    );
    // 重复照片 ID 仍被主键拒绝。
    let duplicate = ids[0].clone();
    assert!(
        sqlx::query("INSERT INTO photos(id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,thumbnail_path,preview_path,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&duplicate).bind(&project.id).bind(99).bind(prepared.path.to_string_lossy().as_ref())
            .bind(&prepared.filename).bind(prepared.size as i64).bind(&prepared.hash)
            .bind(prepared.width as i64).bind(prepared.height as i64)
            .bind(prepared.thumbnail.to_string_lossy().as_ref()).bind(prepared.preview.to_string_lossy().as_ref())
            .bind("now").execute(&store.pool).await.is_err()
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&store.pool)
            .await
            .unwrap()
            .is_empty()
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

#[tokio::test]
async fn project_delete_requires_archive_then_clears_deletable_rows() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("delete.sqlite");
    let store = Store::open(&database).await.unwrap();
    let project = store.create("删除演练", "先归档再删除").await.unwrap();

    let unarchived = store.delete(&project.id).await;
    assert!(unarchived.is_err(), "未归档的项目不允许直接删除");

    store.archive(&project.id, project.revision).await.unwrap();
    store.delete(&project.id).await.unwrap();

    assert!(store.list(false).await.unwrap().is_empty());
    assert!(
        store.list(true).await.unwrap().is_empty(),
        "归档列表不应再显示已删除项目"
    );

    let pool = sqlx::SqlitePool::connect(&database.to_string_lossy())
        .await
        .unwrap();
    for table in ["photos", "source_assets", "drafts"] {
        let count: i64 =
            sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE project_id=?"))
                .bind(&project.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0, "{table} 可删数据应清空");
    }
    let purged_at: Option<String> = sqlx::query_scalar("SELECT purged_at FROM projects WHERE id=?")
        .bind(&project.id)
        .fetch_optional(&pool)
        .await
        .unwrap()
        .flatten();
    assert!(
        purged_at.is_some(),
        "项目行按审计架构保留并带 purged_at 标记"
    );
    let command_events: i64 =
        sqlx::query_scalar("SELECT count(*) FROM command_events WHERE project_id=?")
            .bind(&project.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        command_events >= 2,
        "create/archive 命令审计行应保留（append-only 架构）"
    );
    pool.close().await;
}

/// 外部编辑器把原图就地保存回同一路径时，导入通道只报事实、不静默落库；
/// 人确认后才把这一条记录换成文件现在的内容，id、选片、备注与序列位置都不动。
#[tokio::test]
async fn an_externally_rewritten_source_is_reported_before_it_lands_in_the_library() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let cache = directory.path().join("cache");
    let mut project = store.create("外部改写", "就地保存回原路径").await.unwrap();
    let source = fixture(directory.path(), 11, ImageFormat::Jpeg);
    let before = image_pipeline::prepare(&source, &cache).unwrap();
    assert!(
        store
            .insert_photo(&project.id, &before)
            .await
            .unwrap()
            .inserted()
    );
    project = store.get(&project.id).await.unwrap();
    let photo = project.photos[0].id.clone();
    let position = project.photos[0].index;
    project = store
        .decide(&project.id, &photo, "keep", project.revision)
        .await
        .unwrap();
    project = store
        .note(&project.id, &photo, "这张留作主图", project.revision)
        .await
        .unwrap();
    // 旧内容上有一条按内容指纹关联的 AI 观察。
    sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,created_at) VALUES(?,?,?,?,?,?,CURRENT_TIMESTAMP)")
        .bind(&photo)
        .bind(&before.hash)
        .bind("test-model")
        .bind("v1")
        .bind("inspect_photos")
        .bind("{}")
        .execute(&store.pool)
        .await
        .unwrap();

    // 外部编辑器就地改写同一个文件。
    RgbImage::from_fn(160, 120, |x, y| Rgb([(x % 256) as u8, (y % 256) as u8, 70]))
        .save_with_format(&source, ImageFormat::Jpeg)
        .unwrap();
    let after = image_pipeline::prepare(&source, &cache).unwrap();
    assert_ne!(before.hash, after.hash);

    let outcome = store.insert_photo(&project.id, &after).await.unwrap();
    let ImportOutcome::Changed(change) = outcome.clone() else {
        panic!("同名文件内容变化应报成 Changed，实际是 {outcome:?}");
    };
    assert_eq!(change.photo_id, photo);
    assert_eq!(change.previous_sha256, before.hash);
    assert_eq!(change.incoming_sha256, after.hash);
    assert_eq!(change.previous_size, before.size as i64);
    assert_eq!(change.incoming_size, after.size as i64);
    assert_eq!(change.path, after.path.to_string_lossy().as_ref());
    // 没有偷偷多出第二条记录。
    assert_eq!(store.get(&project.id).await.unwrap().photos.len(), 1);

    // 后台导入任务把同一件事单独列出来：既不算新增也不算重复跳过。
    let imports = ImportManager::default();
    let job = imports
        .start(
            store.clone(),
            cache.clone(),
            project.id.clone(),
            vec![source.clone()],
            |_| {},
        )
        .await
        .unwrap();
    let job = finished(&imports, &job).await;
    assert_eq!(job.imported, 0);
    assert_eq!(job.skipped, 0);
    assert_eq!(job.changes.len(), 1);
    assert_eq!(job.changes[0].photo_id, photo);
    assert_eq!(job.changes[0].previous_width, before.width as i64);

    // 版本对不上先拒，库里仍是旧内容。
    assert_eq!(
        store
            .apply_photo_change(&project.id, &photo, &after, project.revision + 7)
            .await
            .unwrap_err()
            .code,
        "REVISION_CONFLICT"
    );
    assert_eq!(
        store.get(&project.id).await.unwrap().photos[0].content_sha256,
        before.hash
    );

    let current = store.get(&project.id).await.unwrap();
    let updated = store
        .apply_photo_change(&project.id, &photo, &after, current.revision)
        .await
        .unwrap();
    assert_eq!(updated.photos.len(), 1);
    let record = &updated.photos[0];
    assert_eq!(record.id, photo);
    assert_eq!(record.content_sha256, after.hash);
    assert_eq!(record.width, after.width as i64);
    assert_eq!(record.height, after.height as i64);
    assert_eq!(record.index, position, "序列位置不因为内容更新而改变");
    assert_eq!(record.status, "keep");
    assert_eq!(record.note, "这张留作主图");
    assert!(!record.is_missing);
    // 显示缓存换到新指纹上，旧图不会再冒充这一张。
    assert_eq!(Path::new(&record.thumbnail_path), after.thumbnail.as_path());
    assert_eq!(Path::new(&record.preview_path), after.preview.as_path());
    assert!(
        std::fs::symlink_metadata(&record.thumbnail_path).is_ok_and(|m| m.is_file()),
        "新内容的缩略缓存要真的在盘上"
    );
    assert!(
        updated
            .events
            .iter()
            .any(|event| event.kind == "content_apply")
    );

    // 旧内容的那条 AI 观察不再命中这张图。
    let live: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photo_observations o JOIN photos p ON p.id=o.photo_id AND p.content_sha256=o.photo_hash WHERE p.id=?")
        .bind(&photo)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(live, 0);

    // 母版资产跟着换，且第一次落下指纹核对时间；两条资产表达指向新缓存。
    let master: (String, Option<String>) = sqlx::query_as("SELECT s.content_sha256,s.source_hash_verified_at FROM source_assets s JOIN asset_aliases a ON a.asset_id=s.id WHERE a.photo_id=?")
        .bind(&photo)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(master.0, after.hash);
    assert!(master.1.is_some(), "核对过内容才写 source_hash_verified_at");
    let representations: Vec<(String, String)> = sqlx::query_as("SELECT kind,file_path FROM asset_representations WHERE asset_id=(SELECT asset_id FROM asset_aliases WHERE photo_id=?) ORDER BY kind")
        .bind(&photo)
        .fetch_all(&store.pool)
        .await
        .unwrap();
    assert_eq!(representations.len(), 2);
    assert!(
        representations
            .iter()
            .all(|(_, path)| path.contains(&after.hash))
    );
    let variants: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM edit_variants WHERE asset_id=(SELECT asset_id FROM asset_aliases WHERE photo_id=?)")
        .bind(&photo)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(variants, 1);

    // 确认之后同一份内容再导入判为重复；内容已一致时重复确认被拒，不涨修订号。
    assert!(
        matches!(
            store.insert_photo(&project.id, &after).await.unwrap(),
            ImportOutcome::Duplicate { .. }
        ),
        "更新完再导入不应重复报变化"
    );
    assert_eq!(store.get(&project.id).await.unwrap().photos.len(), 1);
    let settled = store.get(&project.id).await.unwrap();
    assert_eq!(
        store
            .apply_photo_change(&project.id, &photo, &after, settled.revision)
            .await
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    assert_eq!(
        store.get(&project.id).await.unwrap().revision,
        settled.revision
    );
    // 原图始终没被程序改写过（只读它的当前内容）。
    assert_eq!(image_pipeline::fingerprint(&source).unwrap(), after.hash);
}

/// 两条记录共用一份母版时（同内容不同来源），就地改写其中一条不能替另一条改历史；
/// 项目里已经有这份新内容时改到同样内容的另一条共用它，也不立第三份母版。
#[tokio::test]
async fn updating_one_photos_content_never_rewrites_a_master_another_photo_shares() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(&directory.path().join("db.sqlite"))
        .await
        .unwrap();
    let cache = directory.path().join("cache");
    let project = store.create("共用母版", "").await.unwrap();
    let first = fixture(directory.path(), 21, ImageFormat::Jpeg);
    let second = directory.path().join("frame-021-copy.jpg");
    std::fs::copy(&first, &second).unwrap();
    let original = image_pipeline::fingerprint(&first).unwrap();
    for path in [&first, &second] {
        let prepared = image_pipeline::prepare(path, &cache).unwrap();
        store.insert_photo(&project.id, &prepared).await.unwrap();
    }
    let project = store.get(&project.id).await.unwrap();
    let one = project.photos[0].id.clone();
    let two = project.photos[1].id.clone();
    async fn master_of(store: &Store, photo: &str) -> String {
        sqlx::query_scalar::<_, String>("SELECT asset_id FROM asset_aliases WHERE photo_id=?")
            .bind(photo)
            .fetch_one(&store.pool)
            .await
            .unwrap()
    }
    assert_eq!(master_of(&store, &one).await, master_of(&store, &two).await);

    // 其中一条的原文件被就地改写并确认更新。
    RgbImage::from_fn(200, 150, |x, y| {
        Rgb([(x % 256) as u8, (y % 256) as u8, 120])
    })
    .save_with_format(&first, ImageFormat::Jpeg)
    .unwrap();
    let rewritten = image_pipeline::prepare(&first, &cache).unwrap();
    store
        .apply_photo_change(&project.id, &one, &rewritten, project.revision)
        .await
        .unwrap();

    // 另一条不受牵连：内容与母版都还是旧的那份。
    let project = store.get(&project.id).await.unwrap();
    let untouched = project.photos.iter().find(|p| p.id == two).unwrap();
    assert_eq!(untouched.content_sha256, original);
    assert_ne!(master_of(&store, &one).await, master_of(&store, &two).await);
    let shared_master: String = sqlx::query_scalar(
        "SELECT content_sha256 FROM source_assets WHERE id=(SELECT asset_id FROM asset_aliases WHERE photo_id=?)",
    )
    .bind(&two)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    assert_eq!(shared_master, original);
    let aliases: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM asset_aliases")
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(aliases, 2, "两条记录各自都有母版可挂，不留孤儿");

    // 另一条也改到同样的新内容：挂到已有母版，项目里不会长出第三份。
    std::fs::copy(&first, &second).unwrap();
    let same_content = image_pipeline::prepare(&second, &cache).unwrap();
    assert_eq!(same_content.hash, rewritten.hash);
    let project = store.get(&project.id).await.unwrap();
    store
        .apply_photo_change(&project.id, &two, &same_content, project.revision)
        .await
        .unwrap();
    assert_eq!(master_of(&store, &one).await, master_of(&store, &two).await);
    let masters: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM source_assets WHERE project_id=?")
        .bind(&project.id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(masters, 2);
    assert_eq!(
        store.get(&project.id).await.unwrap().photos.len(),
        2,
        "更新内容不会新增照片记录"
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&store.pool)
            .await
            .unwrap()
            .is_empty()
    );
}
