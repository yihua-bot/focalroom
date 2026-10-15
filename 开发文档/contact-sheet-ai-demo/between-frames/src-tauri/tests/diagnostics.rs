// M5 诊断必须只读、无业务文本泄漏，统计失败不能伪装成 0。
use between_frames_lib::{agent::types::AiConfig, db::Store, diagnostics};
use std::path::Path;
fn config() -> AiConfig {
    AiConfig {
        provider_id: "openai".into(),
        model_id: "private-model".into(),
        ready: true,
        api_key_tail: Some("secret-tail".into()),
        key_source: Some("env".into()),
        base_url: "https://secret-gateway.invalid".into(),
        prompt_version: "private-prompt".into(),
    }
}
async fn store(root: &Path) -> Store {
    Store::open(&root.join("projects.sqlite")).await.unwrap()
}
#[tokio::test]
async fn diagnostics_counts_sizes_are_read_only_and_redacted() {
    let temp = tempfile::tempdir().unwrap();
    let db = store(temp.path()).await;
    let project = db.create("private-project", "private-note").await.unwrap();
    std::fs::create_dir(temp.path().join("cache")).unwrap();
    std::fs::write(temp.path().join("cache/private-photo-320.jpg"), [0; 17]).unwrap();
    std::fs::write(temp.path().join("cache/private-photo-1400.jpg"), [0; 23]).unwrap();
    std::fs::write(
        temp.path().join("projects.backup-before-v8-test.sqlite"),
        [0; 31],
    )
    .unwrap();
    std::fs::write(temp.path().join("user-export.json"), "private-note").unwrap();
    let before = serde_json::to_value(&project).unwrap();
    let data = diagnostics::collect(&db, temp.path(), config())
        .await
        .unwrap();
    assert_eq!(data.database["projects"], 1);
    assert_eq!(data.database["photos"], 0);
    assert_eq!(data.migrations_applied, 9);
    assert_eq!(data.storage.cache.bytes, 40);
    assert_eq!(data.storage.cache.files, 2);
    assert_eq!(data.storage.backups.bytes, 31);
    assert_eq!(data.storage.backups.files, 1);
    assert!(data.storage.database.bytes > 0);
    assert!(data.storage.complete);
    assert_eq!(
        data.storage.total_bytes,
        data.storage.database.bytes
            + data.storage.wal.bytes
            + data.storage.shared_memory.bytes
            + 40
            + 31
    );
    let raw = serde_json::to_string(&data).unwrap();
    for secret in [
        "private-project",
        "private-note",
        "private-photo",
        "secret-tail",
        "private-model",
        "secret-gateway",
        "private-prompt",
        temp.path().to_str().unwrap(),
    ] {
        assert!(!raw.contains(secret), "{secret} 不得出现在诊断");
    }
    assert_eq!(
        serde_json::to_value(db.get(&project.id).await.unwrap()).unwrap(),
        before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_tasks")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        std::fs::read(temp.path().join("cache/private-photo-320.jpg")).unwrap(),
        [0; 17]
    );
}
#[tokio::test]
async fn unavailable_storage_is_partial_and_database_failure_is_not_zero() {
    let temp = tempfile::tempdir().unwrap();
    let db = store(temp.path()).await;
    let invalid = temp.path().join("not-a-directory");
    std::fs::write(&invalid, [0; 1]).unwrap();
    let data = diagnostics::collect(&db, &invalid, config()).await.unwrap();
    assert!(!data.storage.complete);
    assert!(!data.storage.database.complete);
    assert_eq!(data.storage.cache.files, 0);
    assert!(data.storage.cache.skipped_entries > 0);
    db.pool.close().await;
    let error = diagnostics::collect(&db, temp.path(), config())
        .await
        .unwrap_err();
    assert_eq!(error.code, "DIAGNOSTICS_UNAVAILABLE");
    assert!(!error.message.contains(temp.path().to_str().unwrap()));
}
#[tokio::test]
async fn missing_cache_is_empty_but_missing_required_database_is_unknown() {
    let temp = tempfile::tempdir().unwrap();
    let db = store(temp.path()).await;
    let data = diagnostics::collect(&db, temp.path(), config())
        .await
        .unwrap();
    assert!(data.storage.cache.complete);
    assert_eq!(data.storage.cache.bytes, 0);
    assert!(data.storage.backups.complete);
    assert_eq!(data.storage.backups.files, 0);
    // 另一空目录没有必需的主库，不能将其包装为完整的 0B 数据库。
    let empty = temp.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    let data = diagnostics::collect(&db, &empty, config()).await.unwrap();
    assert!(!data.storage.database.complete);
    assert!(data.storage.wal.complete && data.storage.shared_memory.complete);
}
#[cfg(unix)]
#[tokio::test]
async fn cache_and_backup_symlinks_and_nested_dirs_are_not_followed() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let db = store(temp.path()).await;
    let cache = temp.path().join("cache");
    std::fs::create_dir(&cache).unwrap();
    std::fs::write(external.path().join("original.jpg"), [0; 100]).unwrap();
    std::fs::write(cache.join("owned-320.jpg"), [0; 9]).unwrap();
    symlink(
        external.path().join("original.jpg"),
        cache.join("link-320.jpg"),
    )
    .unwrap();
    symlink(external.path(), cache.join("linked-directory")).unwrap();
    std::fs::create_dir(cache.join("nested")).unwrap();
    std::fs::write(cache.join("nested/hidden.jpg"), [0; 88]).unwrap();
    symlink(
        external.path().join("original.jpg"),
        temp.path().join("projects.backup-before-link.sqlite"),
    )
    .unwrap();
    let data = diagnostics::collect(&db, temp.path(), config())
        .await
        .unwrap();
    assert_eq!(data.storage.cache.bytes, 9);
    assert_eq!(data.storage.cache.files, 1);
    assert_eq!(data.storage.cache.skipped_entries, 3);
    assert!(!data.storage.cache.complete);
    assert_eq!(data.storage.backups.bytes, 0);
    assert!(!data.storage.backups.complete);
    let linked = temp.path().join("linked-root");
    symlink(external.path(), &linked).unwrap();
    assert!(
        !diagnostics::collect(&db, &linked, config())
            .await
            .unwrap()
            .storage
            .complete
    );
    assert_eq!(
        std::fs::read(external.path().join("original.jpg")).unwrap(),
        [0; 100]
    );
}
