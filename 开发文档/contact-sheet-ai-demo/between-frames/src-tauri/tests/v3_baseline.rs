//! G0 仅用生成样例证明旧数据快照、升级副本与回退可恢复，不接触用户资料。
use between_frames_lib::{db::Store, image_pipeline};
use image::{Rgb, RgbImage};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool, sqlite::SqliteConnectOptions};
use std::path::Path;

fn sha(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}
async fn rows(pool: &SqlitePool) -> Value {
    let version =
        sqlx::query("SELECT id,label,based_on_revision,snapshot_json FROM sequence_versions")
            .fetch_one(pool)
            .await
            .unwrap();
    let comment = sqlx::query("SELECT c.thread_id,c.body,c.author,t.anchor_json,t.project_id,t.state FROM comments c JOIN comment_threads t ON t.id=c.thread_id")
        .fetch_one(pool)
        .await
        .unwrap();
    json!({
        "version": [version.get::<String,_>("id"),version.get::<String,_>("label"),version.get::<i64,_>("based_on_revision").to_string(),version.get::<String,_>("snapshot_json")],
        "comment": [comment.get::<String,_>("thread_id"),comment.get::<String,_>("body"),comment.get::<String,_>("author"),comment.get::<String,_>("anchor_json"),comment.get::<String,_>("project_id"),comment.get::<String,_>("state")]
    })
}
#[tokio::test]
async fn legacy_v11_snapshot_upgrade_and_restore_preserve_identity_and_sources() {
    let temp = tempfile::tempdir().unwrap();
    let migrations = temp.path().join("legacy-migrations");
    std::fs::create_dir(&migrations).unwrap();
    for entry in
        std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations")).unwrap()
    {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if path.extension().is_some_and(|v| v == "sql") && name[..4].parse::<u32>().unwrap() <= 11 {
            std::fs::copy(&path, migrations.join(name.as_ref())).unwrap();
        }
    }
    let old = temp.path().join("legacy.sqlite");
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(&old)
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
    sqlx::query(
        "INSERT INTO projects(id,title,brief,revision,created_at,updated_at,deleted_at) VALUES('legacy-project','旧项目','原有创作意图',7,'now','now',NULL)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let source = temp.path().join("source.jpg");
    RgbImage::from_pixel(80, 60, Rgb([24_u8, 76, 92]))
        .save(&source)
        .unwrap();
    let source_hash = sha(&source);
    let photo = image_pipeline::prepare(&source, &temp.path().join("cache")).unwrap();
    assert_eq!(photo.hash, source_hash);
    sqlx::query("INSERT INTO photos VALUES('legacy-photo','legacy-project',1,?,'source.jpg',?,?,80,60,1,?,?,'now')")
        .bind(source.to_string_lossy().as_ref()).bind(photo.size as i64).bind(&photo.hash)
        .bind(photo.thumbnail.to_string_lossy().as_ref()).bind(photo.preview.to_string_lossy().as_ref()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO photo_decisions(project_id,photo_id,status,user_note,updated_at,rating) VALUES('legacy-project','legacy-photo','keep','原有备注','now',4)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO sequence_items VALUES('legacy-project','legacy-photo',0,'now')")
        .execute(&pool)
        .await
        .unwrap();
    let version_state = json!({"title":"旧项目","brief":"原有创作意图","archivedAt":null,"photos":[{"photoId":"legacy-photo","status":"keep","rating":4,"note":"原有备注"}],"sequence":["legacy-photo"],"comparisons":[]});
    sqlx::query(
        "INSERT INTO sequence_versions VALUES('legacy-version','legacy-project','初选',7,?,'now')",
    )
    .bind(version_state.to_string())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO comment_threads VALUES('legacy-thread','legacy-project','{\"photoId\":\"legacy-photo\"}','open','now','now')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO comments(thread_id,body,author,created_at) VALUES('legacy-thread','保留这张的空间感','author','now')").execute(&pool).await.unwrap();
    let expected_rows = rows(&pool).await;
    let snapshot = temp.path().join("read-only-snapshot.sqlite");
    sqlx::query("VACUUM INTO ?")
        .bind(snapshot.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let snapshot_hash = sha(&snapshot);
    let working = temp.path().join("upgrade-copy.sqlite");
    std::fs::copy(&snapshot, &working).unwrap();
    let upgraded = Store::open(&working).await.unwrap();
    let project = upgraded.get("legacy-project").await.unwrap();
    assert_eq!(project.revision, 7);
    assert_eq!(project.photos[0].id, "legacy-photo");
    assert_eq!(project.photos[0].rating, 4);
    assert_eq!(project.photos[0].note, "原有备注");
    assert_eq!(project.sequence, vec!["legacy-photo"]);
    assert_eq!(rows(&upgraded.pool).await, expected_rows);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&upgraded.pool)
            .await
            .unwrap()
            .is_empty()
    );
    let tables: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .fetch_all(&upgraded.pool)
            .await
            .unwrap();
    let backups: Vec<_> = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("backup-before-v11")
        })
        .collect();
    assert_eq!(backups.len(), 1);
    let backup_hash = sha(&backups[0]);
    let restored = temp.path().join("restored.sqlite");
    std::fs::copy(&backups[0], &restored).unwrap();
    let raw = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(&restored)
            .read_only(true),
    )
    .await
    .unwrap();
    let legacy_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(legacy_count, 11);
    assert_eq!(rows(&raw).await, expected_rows);
    raw.close().await;
    let reopened = Store::open(&restored).await.unwrap();
    assert_eq!(
        serde_json::to_value(reopened.get(&project.id).await.unwrap()).unwrap(),
        serde_json::to_value(&project).unwrap()
    );
    assert_eq!(sha(&snapshot), snapshot_hash);
    assert_eq!(sha(&backups[0]), backup_hash);
    assert_eq!(sha(&source), source_hash);
    if let Ok(path) = std::env::var("FOCALROOM_G0_EVIDENCE_FILE") {
        let result = json!({"fixture":"generated JPEG and legacy v11 SQLite; no user data", "fixtureDirectory":temp.path(),"legacyMigrations":legacy_count,"tables":tables,"sourceSha256":source_hash,"snapshotSha256":snapshot_hash,"backupSha256":backup_hash,"snapshotUnchanged":true,"sourceUnchanged":true,"versionCommentRowsPreserved":true,"foreignKeyViolations":0,"restoredProjectionMatches":true});
        std::fs::write(path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
        let _retained_fixture = temp.keep();
    }
}
