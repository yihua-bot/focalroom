// M5 只读诊断：共享桌面与 harness 的投影，不输出路径或业务文本。
use crate::{
    agent::types::AiConfig,
    db::Store,
    domain::{self, AppError, Result},
    image_pipeline,
};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

const SCAN_LIMIT: usize = 100_000;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub schema_version: u32,
    pub collected_at: String,
    pub app_version: &'static str,
    pub provider: Provider,
    pub database: BTreeMap<&'static str, i64>,
    pub failed_agent_tasks: i64,
    pub migrations_applied: i64,
    pub storage: Storage,
    pub notice: &'static str,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub provider_id: String,
    pub ready: bool,
    pub key_source: Option<String>,
}
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileUsage {
    pub bytes: u64,
    pub files: u64,
    pub skipped_entries: u64,
    pub complete: bool,
}
impl FileUsage {
    fn empty() -> Self {
        Self {
            complete: true,
            ..Self::default()
        }
    }
    fn skip(&mut self) {
        self.complete = false;
        self.skipped_entries += 1;
    }
    fn file(&mut self, path: &Path, optional: bool) {
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.is_file() => {
                self.bytes += meta.len();
                self.files += 1;
            }
            Err(error) if optional && error.kind() == std::io::ErrorKind::NotFound => {}
            _ => self.skip(),
        }
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupHealth {
    pub name: String,
    pub bytes: u64,
    /// ok = quick_check 通过；unreadable = 无法打开；corrupt = 完整性检查失败。
    pub status: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Storage {
    pub database: FileUsage,
    pub wal: FileUsage,
    pub shared_memory: FileUsage,
    pub cache: FileUsage,
    pub backups: FileUsage,
    /// 最近若干份升级前备份的可恢复性巡检（quick_check）；名称不含路径（ADR 0015）。
    pub backups_checked: Vec<BackupHealth>,
    pub total_bytes: u64,
    pub complete: bool,
    pub cache_limit_bytes: u64,
    pub scan_limit_entries: usize,
}
fn directory(path: &Path, backups_only: bool, limit: usize) -> FileUsage {
    let mut usage = FileUsage::empty();
    match std::fs::symlink_metadata(path) {
        Err(error) if !backups_only && error.kind() == std::io::ErrorKind::NotFound => {
            return usage;
        }
        Ok(meta) if meta.is_dir() => {}
        _ => {
            usage.skip();
            return usage;
        }
    }
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => {
            usage.skip();
            return usage;
        }
    };
    for (index, entry) in entries.enumerate() {
        if index >= limit {
            usage.skip();
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                usage.skip();
                continue;
            }
        };
        if backups_only {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("projects.backup-before-") || !name.ends_with(".sqlite") {
                continue;
            }
        }
        usage.file(&entry.path(), false);
    }
    usage
}
fn storage(root: &Path, limit: usize, cache_limit_bytes: u64) -> Storage {
    let mut database = FileUsage::empty();
    let mut wal = FileUsage::empty();
    let mut shared_memory = FileUsage::empty();
    // 应用目录本身被替换为链接时，不能将统计扩展到链接目标。
    let root_valid = std::fs::symlink_metadata(root).is_ok_and(|meta| meta.is_dir());
    let (cache, backups) = if root_valid {
        database.file(&root.join("projects.sqlite"), false);
        wal.file(&root.join("projects.sqlite-wal"), true);
        shared_memory.file(&root.join("projects.sqlite-shm"), true);
        (
            directory(&root.join("cache"), false, limit),
            directory(root, true, limit),
        )
    } else {
        database.skip();
        wal.skip();
        shared_memory.skip();
        let mut unavailable = FileUsage::empty();
        unavailable.skip();
        let mut backups = FileUsage::empty();
        backups.skip();
        (unavailable, backups)
    };
    let sections = [&database, &wal, &shared_memory, &cache, &backups];
    let total_bytes = sections.iter().map(|part| part.bytes).sum();
    let complete = sections.iter().all(|part| part.complete);
    Storage {
        backups_checked: Vec::new(),
        database,
        wal,
        shared_memory,
        cache,
        backups,
        total_bytes,
        complete,
        cache_limit_bytes,
        scan_limit_entries: limit,
    }
}

/// 最近 5 份升级前备份的可恢复性巡检（quick_check，新→旧）；打开失败标 unreadable。
async fn check_backups(root: &Path) -> Vec<BackupHealth> {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    let mut candidates: Vec<_> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("projects.backup-before-") && name.ends_with(".sqlite")
        })
        .collect();
    candidates.sort_by_key(|entry| {
        std::cmp::Reverse(
            entry
                .metadata()
                .and_then(|meta| meta.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        )
    });
    let mut checked = vec![];
    for entry in candidates.into_iter().take(5) {
        let name = entry.file_name().to_string_lossy().into_owned();
        // 与文件统计同规则：不跟随符号链接；非普通文件直接标记，不打开目标（ADR 0015 修订）。
        let meta = entry.metadata();
        if meta.as_ref().is_ok_and(|meta| !meta.is_file()) {
            checked.push(BackupHealth {
                name,
                bytes: 0,
                status: "unreadable".into(),
            });
            continue;
        }
        let bytes = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        let status = match SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(entry.path())
                    .read_only(true),
            )
            .await
        {
            Ok(pool) => {
                let result = sqlx::query_scalar::<_, String>("PRAGMA quick_check")
                    .fetch_one(&pool)
                    .await;
                pool.close().await;
                match result {
                    Ok(check) if check == "ok" => "ok".into(),
                    Ok(_) => "corrupt".into(),
                    // SQLITE_NOTADB(26)：文件根本不是可读数据库。
                    Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("26") => {
                        "unreadable".into()
                    }
                    Err(_) => "corrupt".into(),
                }
            }
            Err(_) => "unreadable".into(),
        };
        checked.push(BackupHealth {
            name,
            bytes,
            status,
        });
    }
    checked
}

pub async fn collect(store: &Store, root: &Path, config: AiConfig) -> Result<Diagnostics> {
    let mut tx = store.pool.begin().await.map_err(unavailable)?;
    let mut counts = BTreeMap::new();
    for (label, table) in [
        ("projects", "projects"),
        ("photos", "photos"),
        ("decisions", "photo_decisions"),
        ("agentTasks", "agent_tasks"),
        ("drafts", "drafts"),
        ("toolCalls", "tool_calls"),
        ("observations", "photo_observations"),
        ("evidence", "preference_evidence"),
        ("hypotheses", "preference_hypotheses"),
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
        counts.insert(label, count);
    }
    let failed_agent_tasks =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks WHERE state='failed'")
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
    let migrations_applied =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success=1")
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    let root = root.to_owned();
    let scan_root = root.clone();
    let cache_limit = image_pipeline::cache_limit_bytes();
    let mut storage =
        tokio::task::spawn_blocking(move || storage(&scan_root, SCAN_LIMIT, cache_limit))
            .await
            .map_err(|_| AppError::new("DIAGNOSTICS_UNAVAILABLE", "存储统计暂不可用，请重试"))?;
    storage.backups_checked = check_backups(&root).await;
    Ok(Diagnostics {
        schema_version: 1,
        collected_at: domain::timestamp(),
        app_version: env!("CARGO_PKG_VERSION"),
        provider: Provider {
            provider_id: config.provider_id,
            ready: config.ready,
            key_source: config.key_source,
        },
        database: counts,
        failed_agent_tasks,
        migrations_applied,
        storage,
        notice: "诊断仅含数量、文件大小与提供方就绪状态，不含照片内容、文件名、本地路径、备注或密钥；仅在主动复制后写入剪贴板，不会自动发送。",
    })
}
fn unavailable(_: sqlx::Error) -> AppError {
    AppError::new(
        "DIAGNOSTICS_UNAVAILABLE",
        "无法读取诊断计数，请重试；项目数据未修改",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scan_limit_is_reported_without_claiming_zero_usage() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), [0; 12]).unwrap();
        std::fs::write(dir.path().join("b"), [0; 12]).unwrap();
        let usage = directory(dir.path(), false, 1);
        assert_eq!(usage.files, 1);
        assert_eq!(usage.bytes, 12);
        assert!(!usage.complete);
        assert_eq!(usage.skipped_entries, 1);
    }
}
