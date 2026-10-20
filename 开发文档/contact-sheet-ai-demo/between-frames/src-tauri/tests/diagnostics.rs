// M5 诊断必须只读、无业务文本泄漏，统计失败不能伪装成 0。
use between_frames_lib::{agent::types::AiConfig, db::Store, diagnostics};
use std::path::Path;

#[cfg(unix)]
#[tokio::test]
async fn v2_baseline_backup_health_rejects_symlink_root() {
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let db = store(temp.path()).await;
    let external = store(outside.path()).await;
    let backup = outside
        .path()
        .join("projects.backup-before-external.sqlite");
    sqlx::query("VACUUM INTO ?")
        .bind(backup.to_string_lossy().as_ref())
        .execute(&external.pool)
        .await
        .unwrap();
    let linked = temp.path().join("linked-root");
    std::os::unix::fs::symlink(outside.path(), &linked).unwrap();
    let data = diagnostics::collect(&db, &linked, config()).await.unwrap();
    assert!(!data.storage.complete);
    assert!(
        data.storage.backups_checked.is_empty(),
        "根目录是符号链接时也不得打开外部备份"
    );
}
fn config() -> AiConfig {
    AiConfig {
        observation: None,
        consent_provider: None,
        provider_id: "openai".into(),
        model_id: "private-model".into(),
        ready: true,
        api_key_tail: Some("secret-tail".into()),
        key_source: Some("env".into()),
        base_url: "https://secret-gateway.invalid".into(),
        prompt_version: "private-prompt".into(),
        capabilities: vec![],
        capability_source: None,
        configuration_error: None,
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
    assert_eq!(data.migrations_applied, 22);
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

#[tokio::test]
async fn backup_health_checks_report_recoverability() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    // 一份健康备份（VACUUM INTO 产出）+ 一份截断损坏 + 一份非 SQLite 文件。
    Store::open(&root.join("projects.sqlite")).await.unwrap();
    {
        let db = Store::open(&root.join("projects.sqlite")).await.unwrap();
        let good = root.join("projects.backup-before-v10-goodtest.sqlite");
        sqlx::query("VACUUM INTO ?")
            .bind(good.to_string_lossy().as_ref())
            .execute(&db.pool)
            .await
            .unwrap();
        db.pool.close().await;
    }
    let healthy_bytes =
        std::fs::read(root.join("projects.backup-before-v10-goodtest.sqlite")).unwrap();
    std::fs::write(
        root.join("projects.backup-before-v10-truncated.sqlite"),
        &healthy_bytes[..healthy_bytes.len() / 2],
    )
    .unwrap();
    std::fs::write(
        root.join("projects.backup-before-v10-notsqlite.sqlite"),
        b"not a database",
    )
    .unwrap();
    let store = Store::open(&root.join("projects.sqlite")).await.unwrap();
    let data = between_frames_lib::diagnostics::collect(
        &store,
        root,
        between_frames_lib::agent::runtime::AgentEngine::new(store.clone()).ai_config(),
    )
    .await
    .unwrap();
    let checked = &data.storage.backups_checked;
    assert_eq!(checked.len(), 3);
    let by_name = |fragment: &str| {
        checked
            .iter()
            .find(|item| item.name.contains(fragment))
            .unwrap()
    };
    assert_eq!(by_name("goodtest").status, "ok");
    assert_eq!(by_name("truncated").status, "corrupt");
    assert_eq!(by_name("notsqlite").status, "unreadable");
    // 名称只含文件名，不含目录路径。
    assert!(
        by_name("goodtest")
            .name
            .starts_with("projects.backup-before-")
    );
}

#[cfg(target_os = "macos")]
#[test]
fn backup_health_does_not_follow_symlinks() {
    // 符号链接命名的「备份」不打开目标，直接标 unreadable（评审问题 2）。
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real-target.sqlite");
    std::fs::write(&real, b"whatever").unwrap();
    let link = dir.path().join("projects.backup-before-v10-link.sqlite");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        // 直接调用巡检入口：check_backups 是私有——经由 collect 需要 Store；
        // 用最小 Store（迁移库）+ 同目录 root。
        let root = dir.path().to_path_buf();
        let store = between_frames_lib::db::Store::open(&root.join("projects.sqlite"))
            .await
            .unwrap();
        let data = between_frames_lib::diagnostics::collect(
            &store,
            &root,
            between_frames_lib::agent::runtime::AgentEngine::new(store.clone()).ai_config(),
        )
        .await
        .unwrap();
        let link_row = data
            .storage
            .backups_checked
            .iter()
            .find(|item| item.name.contains("link"))
            .expect("符号链接备份应出现在巡检中");
        assert_eq!(link_row.status, "unreadable");
    });
}
