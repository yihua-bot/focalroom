//! 可重建的本机句向量缓存；文件不是授权、观察事实或正式状态的权威。
use crate::{
    command_audit::fingerprint,
    db::Store,
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashSet,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_ENTRIES: usize = 500;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    pub language: u32,
    pub revision: u32,
    pub dimensions: usize,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub key: String,
    pub photo: String,
    pub vector: Vec<f32>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Index {
    version: u32,
    pub profile: Option<Profile>,
    pub entries: Vec<Entry>,
}
impl Default for Index {
    fn default() -> Self {
        Self {
            version: 1,
            profile: None,
            entries: vec![],
        }
    }
}
impl Index {
    fn valid(&self) -> bool {
        if self.version != 1 || self.entries.len() > MAX_ENTRIES {
            return false;
        }
        let Some(profile) = &self.profile else {
            return self.entries.is_empty();
        };
        let mut keys = HashSet::new();
        [1, 2].contains(&profile.language)
            && profile.revision > 0
            && (1..=1024).contains(&profile.dimensions)
            && self.entries.iter().all(|entry| {
                digest(&entry.key)
                    && digest(&entry.photo)
                    && keys.insert(&entry.key)
                    && entry.vector.len() == profile.dimensions
                    && entry.vector.iter().all(|v| v.is_finite())
                    && entry.vector.iter().any(|v| *v != 0.0)
            })
    }
    pub fn get(&self, key: &str) -> Option<&[f32]> {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.vector.as_slice())
    }
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    checksum: String,
    data: Index,
}

/// 文件名同时隔离数据库和项目，不接受前端提供的路径或文件名。
pub(crate) fn path(db: &Store, project: &str) -> Option<PathBuf> {
    let options = db.pool.connect_options();
    let database = options.get_filename();
    if !std::fs::symlink_metadata(database).is_ok_and(|m| m.is_file()) {
        return None;
    }
    Some(database.parent()?.join("cache").join(format!(
        "semantic-v1-{}.json",
        fingerprint(&json!({"database": database, "project": project}))
    )))
}
fn safe_parent(path: &Path, create: bool) -> std::io::Result<()> {
    let parent = path.parent().ok_or(std::io::ErrorKind::InvalidInput)?;
    let root = parent.parent().ok_or(std::io::ErrorKind::InvalidInput)?;
    if !std::fs::symlink_metadata(root)?.is_dir() {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    if create && !parent.exists() {
        std::fs::create_dir(parent)?;
    }
    if !std::fs::symlink_metadata(parent)?.is_dir() {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    Ok(())
}
pub(crate) fn load(path: Option<&Path>) -> (Index, &'static str) {
    let Some(path) = path else {
        return (Index::default(), "unavailable");
    };
    match read(path) {
        Ok(index) => (index, "loaded"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (Index::default(), "miss"),
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            (Index::default(), "corrupt")
        }
        Err(_) => (Index::default(), "unavailable"),
    }
}
fn read(path: &Path) -> std::io::Result<Index> {
    safe_parent(path, false)?;
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_file() {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    if meta.len() > MAX_BYTES {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    let mut bytes = Vec::new();
    let file = std::fs::File::open(path)?;
    // 再核对打开的文件，拒绝已知链接/替换；不把缓存目录当任意路径读取入口。
    let opened = file.metadata()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if opened.dev() != meta.dev() || opened.ino() != meta.ino() {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
    }
    if !opened.is_file() {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    let envelope: Envelope =
        serde_json::from_slice(&bytes).map_err(|_| std::io::ErrorKind::InvalidData)?;
    if !envelope.data.valid()
        || fingerprint(&serde_json::to_value(&envelope.data)?) != envelope.checksum
    {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    Ok(envelope.data)
}
fn write(path: &Path, index: &Index) -> std::io::Result<()> {
    if !index.valid() {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    safe_parent(path, true)?;
    match std::fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() => return Err(std::io::ErrorKind::InvalidInput.into()),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    let bytes = serde_json::to_vec(&Envelope {
        checksum: fingerprint(&serde_json::to_value(index)?),
        data: index.clone(),
    })?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    let temp = path.with_extension(format!("{}.tmp", crate::domain::id()));
    let result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
/// 调用方持有该数据库的写事务；合并最新文件，避免并行任务覆盖彼此的增量。
pub(crate) fn publish(
    path: &Path,
    profile: Profile,
    entries: Vec<Entry>,
) -> std::io::Result<usize> {
    let (mut current, _) = load(Some(path));
    if current.profile.as_ref() != Some(&profile) {
        current.entries.clear();
    }
    current.profile = Some(profile);
    let photos: HashSet<_> = entries.iter().map(|entry| entry.photo.clone()).collect();
    current
        .entries
        .retain(|entry| !photos.contains(&entry.photo));
    current.entries.extend(entries);
    if current.entries.len() > MAX_ENTRIES {
        current.entries.drain(..current.entries.len() - MAX_ENTRIES);
    }
    let count = current.entries.len();
    write(path, &current)?;
    Ok(count)
}
/// 清空命令在持有数据库写锁时调用；空文件仍为有效缓存，失败则不宣称清空成功。
pub(crate) fn clear(db: &Store, project: &str) -> Result<()> {
    if let Some(path) = path(db, project) {
        match std::fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            _ => {}
        }
        write(&path, &Index::default())
            .map_err(|_| AppError::new("CACHE_WRITE", "本地检索缓存未能清空；请检查存储后重试"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> Profile {
        Profile {
            language: 1,
            revision: 1,
            dimensions: 2,
        }
    }
    fn entry(id: usize, photo: usize) -> Entry {
        Entry {
            key: fingerprint(&json!(id)),
            photo: fingerprint(&json!(photo)),
            vector: vec![1.0, 0.5],
        }
    }
    #[test]
    fn persists_incrementally_replaces_changed_photo_and_bounds_old_entries() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("cache/index.json");
        assert_eq!(load(Some(&path)).1, "miss");
        publish(&path, profile(), vec![entry(1, 1), entry(2, 2)]).unwrap();
        let reopened = load(Some(&path));
        assert_eq!(reopened.1, "loaded");
        assert_eq!(
            reopened.0.get(&entry(1, 1).key),
            Some([1.0, 0.5].as_slice())
        );
        publish(&path, profile(), vec![entry(3, 1)]).unwrap();
        let (index, _) = load(Some(&path));
        assert!(index.get(&entry(1, 1).key).is_none());
        assert!(index.get(&entry(2, 2).key).is_some());
        assert_eq!(index.entries.len(), 2);
        let next = Profile {
            revision: 2,
            ..profile()
        };
        publish(&path, next.clone(), vec![entry(4, 4)]).unwrap();
        let (index, _) = load(Some(&path));
        assert_eq!(index.profile, Some(next));
        assert_eq!(index.entries.len(), 1);
        publish(&path, profile(), (0..501).map(|id| entry(id, id)).collect()).unwrap();
        let (index, _) = load(Some(&path));
        assert_eq!(index.entries.len(), 500);
        assert!(index.get(&entry(0, 0).key).is_none());
        assert!(index.get(&entry(500, 500).key).is_some());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    #[test]
    fn corrupt_or_oversized_cache_is_a_miss_and_invalid_vectors_cannot_replace_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("cache/index.json");
        publish(&path, profile(), vec![entry(1, 1)]).unwrap();
        let good = std::fs::read(&path).unwrap();
        let mut invalid = entry(2, 2);
        invalid.vector = vec![f32::NAN, 0.0];
        assert!(publish(&path, profile(), vec![invalid]).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), good);
        let mut data: serde_json::Value = serde_json::from_slice(&good).unwrap();
        data["data"]["entries"][0]["vector"][0] = json!(0.75);
        std::fs::write(&path, data.to_string()).unwrap();
        assert_eq!(load(Some(&path)).1, "corrupt");
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(MAX_BYTES + 1).unwrap();
        assert_eq!(load(Some(&path)).1, "corrupt");
        publish(&path, profile(), vec![entry(2, 2)]).unwrap();
        assert_eq!(load(Some(&path)).0.entries.len(), 1);
        let invalid = Index {
            version: 1,
            profile: Some(profile()),
            entries: vec![entry(3, 3), entry(3, 3)],
        };
        assert!(!invalid.valid());
        assert!(
            !Index {
                profile: Some(Profile {
                    dimensions: 1025,
                    ..profile()
                }),
                ..Index::default()
            }
            .valid()
        );
    }
    #[cfg(unix)]
    #[test]
    fn refuses_linked_cache_file_and_directory_without_touching_target() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("private");
        std::fs::write(&target, b"sentinel").unwrap();
        std::fs::create_dir(temp.path().join("cache")).unwrap();
        let path = temp.path().join("cache/index.json");
        symlink(&target, &path).unwrap();
        assert_eq!(load(Some(&path)).1, "unavailable");
        assert!(publish(&path, profile(), vec![entry(1, 1)]).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"sentinel");
        let other = tempfile::tempdir().unwrap();
        symlink(outside.path(), other.path().join("cache")).unwrap();
        assert!(
            publish(
                &other.path().join("cache/index.json"),
                profile(),
                vec![entry(1, 1)]
            )
            .is_err()
        );
        assert!(!outside.path().join("index.json").exists());
    }
    #[tokio::test]
    async fn database_project_isolation_and_purge_command_clear_derived_vectors() {
        use crate::agent::memory_control::{MemoryCommand, MemoryRequest};
        use crate::command_handler::CommandHandler;
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("a.sqlite")).await.unwrap();
        let other = Store::open(&temp.path().join("b.sqlite")).await.unwrap();
        let project = db.create("合成索引测试", "").await.unwrap();
        let p = path(&db, &project.id).unwrap();
        assert_ne!(p, path(&other, &project.id).unwrap());
        assert_ne!(p, path(&db, "other-project").unwrap());
        publish(&p, profile(), vec![entry(1, 1)]).unwrap();
        let request = MemoryRequest {
            request_id: crate::domain::id(),
            project_id: project.id.clone(),
            command: MemoryCommand::PurgeObservationCache {},
        };
        let before = serde_json::to_value(db.get(&project.id).await.unwrap()).unwrap();
        CommandHandler::new(&db)
            .execute_memory(request.clone())
            .await
            .unwrap();
        assert_eq!(load(Some(&p)).1, "loaded");
        assert!(load(Some(&p)).0.entries.is_empty());
        publish(&p, profile(), vec![entry(2, 2)]).unwrap();
        CommandHandler::new(&db)
            .execute_memory(request)
            .await
            .unwrap();
        assert_eq!(
            load(Some(&p)).0.entries.len(),
            1,
            "回执重放不重复清空后续新缓存"
        );
        assert_eq!(
            serde_json::to_value(db.get(&project.id).await.unwrap()).unwrap(),
            before
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM preference_evidence")
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            0
        );
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn purge_failure_keeps_observations_and_receipt_unchanged() {
        use crate::agent::memory_control::{MemoryCommand, MemoryRequest};
        use crate::{command_handler::CommandHandler, image_pipeline};
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("a.sqlite")).await.unwrap();
        let project = db.create("合成清空故障测试", "").await.unwrap();
        let source = temp.path().join("fixture.png");
        image::RgbImage::from_pixel(8, 8, image::Rgb([1, 2, 3]))
            .save(&source)
            .unwrap();
        db.insert_photo(
            &project.id,
            &image_pipeline::prepare(&source, &temp.path().join("cache")).unwrap(),
        )
        .await
        .unwrap();
        sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,created_at) SELECT id,content_sha256,'test','test','inspect_photos','{}',CURRENT_TIMESTAMP FROM photos").execute(&db.pool).await.unwrap();
        let target = temp.path().join("sentinel");
        std::fs::write(&target, b"unchanged").unwrap();
        std::os::unix::fs::symlink(&target, path(&db, &project.id).unwrap()).unwrap();
        let error = CommandHandler::new(&db)
            .execute_memory(MemoryRequest {
                request_id: crate::domain::id(),
                project_id: project.id,
                command: MemoryCommand::PurgeObservationCache {},
            })
            .await
            .unwrap_err();
        assert_eq!(error.code, "CACHE_WRITE");
        assert_eq!(std::fs::read(target).unwrap(), b"unchanged");
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM photo_observations")
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM memory_commands")
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            0
        );
    }
}
