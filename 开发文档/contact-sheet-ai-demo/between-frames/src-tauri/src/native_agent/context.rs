//! 手机项目的只读执行副本。没有 GUI 正式写入口，不把镜像写入当作人工偏好证据。
use crate::{
    db::Store,
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ContextInput {
    pub reference: String,
    pub title: String,
    pub brief: String,
    pub photos: Vec<PhotoInput>,
    pub sequence: Vec<String>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PhotoInput {
    pub id: String,
    pub filename: String,
    pub source_hash: String,
    pub status: String,
    pub preview_path: Option<String>,
    pub width: u32,
    pub height: u32,
}
fn key(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
fn invalid() -> AppError {
    AppError::new("VALIDATION_ERROR", "项目上下文不合法，请刷新项目后重试")
}

pub(super) async fn sync(db: &Store, input: ContextInput) -> Result<Value> {
    let native_ids: HashSet<_> = input.photos.iter().map(|p| p.id.as_str()).collect();
    let hashes: HashSet<_> = input
        .photos
        .iter()
        .map(|p| p.source_hash.as_str())
        .collect();
    if input.reference.is_empty()
        || input.reference.len() > 300
        || input.title.is_empty()
        || input.title.len() > 1000
        || input.brief.len() > 100000
        || input.photos.len() > 10000
        || native_ids.len() != input.photos.len()
        || hashes.len() != input.photos.len()
        || input.photos.iter().any(|p| {
            p.id.is_empty()
                || p.id.len() > 300
                || p.filename.is_empty()
                || p.filename.len() > 1000
                || !crate::login_guard::hex64(&p.source_hash)
                || p.width == 0
                || p.height == 0
                || p.width > 100000
                || p.height > 100000
                || !matches!(p.status.as_str(), "keep" | "reject" | "undecided")
        })
        || input.sequence.iter().collect::<HashSet<_>>().len() != input.sequence.len()
        || input.sequence.iter().any(|id| {
            !input
                .photos
                .iter()
                .any(|p| &p.id == id && p.status == "keep")
        })
    {
        return Err(invalid());
    }
    // 空项目与曾有照片的副本分开，避免生命周期的“空范围=全项目”规则纳入已移除照片。
    let project = format!(
        "native-{}",
        key(&format!("{}:{}", input.reference, input.photos.is_empty()))
    );
    let ids: HashMap<_, _> = input
        .photos
        .iter()
        .map(|p| (p.id.clone(), key(&format!("{project}:{}", p.id))))
        .collect();
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM agent_tasks WHERE project_id=? AND state IN ('running','queued','planning')",
    )
    .bind(&project)
    .fetch_one(&mut *tx)
    .await?;
    let now = domain::timestamp();
    sqlx::query(
        "INSERT OR IGNORE INTO projects(id,title,brief,created_at,updated_at) VALUES(?,?,?,?,?)",
    )
    .bind(&project)
    .bind(&input.title)
    .bind(&input.brief)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    // 对比当前有效照片，旧负编号行保留作旧任务/草案的引用，不进入新任务的显式范围。
    let previous: Vec<(String,String,String,String,i64,i64)> = sqlx::query_as("SELECT p.id,p.filename,p.content_sha256,d.status,p.width,p.height FROM photos p JOIN photo_decisions d ON d.photo_id=p.id WHERE p.project_id=? AND p.import_index>0 ORDER BY p.import_index")
        .bind(&project).fetch_all(&mut *tx).await?;
    let previous_sequence: Vec<String> = sqlx::query_scalar(
        "SELECT photo_id FROM sequence_items WHERE project_id=? ORDER BY position",
    )
    .bind(&project)
    .fetch_all(&mut *tx)
    .await?;
    let previous_meta: (String, String) =
        sqlx::query_as("SELECT title,brief FROM projects WHERE id=?")
            .bind(&project)
            .fetch_one(&mut *tx)
            .await?;
    let incoming: Vec<_> = input
        .photos
        .iter()
        .map(|p| {
            (
                ids[&p.id].clone(),
                p.filename.clone(),
                p.source_hash.clone(),
                p.status.clone(),
                i64::from(p.width),
                i64::from(p.height),
            )
        })
        .collect();
    let sequence: Vec<_> = input.sequence.iter().map(|id| ids[id].clone()).collect();
    let changed = previous != incoming
        || previous_sequence != sequence
        || previous_meta != (input.title.clone(), input.brief.clone());
    let mut needs_previews = false;
    for photo in &input.photos {
        if let Some(path) = photo.preview_path.as_deref().filter(|p| !p.is_empty()) {
            let prior: Option<String> =
                sqlx::query_scalar("SELECT preview_path FROM photos WHERE id=?")
                    .bind(&ids[&photo.id])
                    .fetch_optional(&mut *tx)
                    .await?;
            needs_previews |= prior.as_deref() != Some(path);
        }
    }
    if active > 0 && (changed || needs_previews) {
        return Err(AppError::new(
            "TASK_ACTIVE",
            "项目仍有任务执行中，请先暂停或取消",
        ));
    }
    if !changed && !needs_previews {
        let revision: i64 = sqlx::query_scalar("SELECT revision FROM projects WHERE id=?")
            .bind(&project)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(json!({"projectId":project,"photoIDs":ids,"revision":revision}));
    }
    // 先让出所有有效序号；历史行保留独立负序号，避免中途重排触发唯一约束。
    sqlx::query("UPDATE photos SET import_index=-(rowid+1000000) WHERE project_id=?")
        .bind(&project)
        .execute(&mut *tx)
        .await?;
    for (index, p) in input.photos.iter().enumerate() {
        let id = &ids[&p.id];
        let path = p.preview_path.as_deref().unwrap_or("");
        if !path.is_empty() {
            let meta = std::fs::symlink_metadata(path)?;
            if !meta.is_file() || meta.len() > 16 * 1024 * 1024 {
                return Err(invalid());
            }
        }
        sqlx::query("INSERT INTO photos(id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,thumbnail_path,preview_path,created_at) VALUES(?,?,?,?,?,0,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET import_index=excluded.import_index,filename=excluded.filename,content_sha256=excluded.content_sha256,width=excluded.width,height=excluded.height,original_path=CASE WHEN excluded.preview_path='' AND photos.content_sha256=excluded.content_sha256 THEN photos.original_path ELSE excluded.original_path END,thumbnail_path=CASE WHEN excluded.preview_path='' AND photos.content_sha256=excluded.content_sha256 THEN photos.thumbnail_path ELSE excluded.thumbnail_path END,preview_path=CASE WHEN excluded.preview_path='' AND photos.content_sha256=excluded.content_sha256 THEN photos.preview_path ELSE excluded.preview_path END")
            .bind(id).bind(&project).bind(index as i64+1).bind(path).bind(&p.filename).bind(&p.source_hash).bind(i64::from(p.width)).bind(i64::from(p.height)).bind(path).bind(path).bind(&now).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photo_decisions(project_id,photo_id,status,updated_at) VALUES(?,?,?,?) ON CONFLICT(project_id,photo_id) DO UPDATE SET status=excluded.status,updated_at=excluded.updated_at")
            .bind(&project).bind(id).bind(&p.status).bind(&now).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO source_assets(id,project_id,content_sha256,original_locator,file_size,width,height,format,created_at) VALUES(?,?,?,'[]',0,?,?,'jpeg',?) ON CONFLICT(id) DO UPDATE SET content_sha256=excluded.content_sha256,width=excluded.width,height=excluded.height")
            .bind(id).bind(&project).bind(&p.source_hash).bind(i64::from(p.width)).bind(i64::from(p.height)).bind(&now).execute(&mut *tx).await?;
        sqlx::query("INSERT OR IGNORE INTO asset_aliases(photo_id,asset_id) VALUES(?,?)")
            .bind(id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO edit_variants(id,asset_id,name,is_current,created_at) VALUES(?,?,'original',1,?)").bind(id).bind(id).bind(&now).execute(&mut *tx).await?;
    }
    sqlx::query("DELETE FROM sequence_items WHERE project_id=?")
        .bind(&project)
        .execute(&mut *tx)
        .await?;
    for (index, id) in sequence.iter().enumerate() {
        sqlx::query(
            "INSERT INTO sequence_items(project_id,photo_id,position,updated_at) VALUES(?,?,?,?)",
        )
        .bind(&project)
        .bind(id)
        .bind(index as i64)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("UPDATE projects SET title=?,brief=?,revision=revision+?,updated_at=? WHERE id=?")
        .bind(input.title)
        .bind(input.brief)
        .bind(i64::from(changed))
        .bind(now)
        .bind(&project)
        .execute(&mut *tx)
        .await?;
    let revision: i64 = sqlx::query_scalar("SELECT revision FROM projects WHERE id=?")
        .bind(&project)
        .fetch_one(&mut *tx)
        .await?;
    if changed {
        crate::agent::store::mark_stale(&mut tx, &project, revision).await?;
    }
    tx.commit().await?;
    Ok(json!({"projectId":project,"photoIDs":ids,"revision":revision}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(reference: &str, photos: &[(&str, &str)]) -> ContextInput {
        ContextInput {
            reference: reference.into(),
            title: "项目".into(),
            brief: "".into(),
            sequence: vec![],
            photos: photos
                .iter()
                .map(|(id, hash)| PhotoInput {
                    id: (*id).into(),
                    filename: format!("{id}.jpg"),
                    source_hash: hash.repeat(64),
                    status: "undecided".into(),
                    preview_path: None,
                    width: 1200,
                    height: 800,
                })
                .collect(),
        }
    }
    #[tokio::test]
    async fn mirrors_preserve_ids_and_history_without_inventing_human_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("test.sqlite")).await.unwrap();
        let first = sync(&db, input("local:p", &[("a", "a"), ("b", "b")]))
            .await
            .unwrap();
        let second = sync(&db, input("local:p", &[("a", "a"), ("b", "b")]))
            .await
            .unwrap();
        assert_eq!(first, second);
        let removed = sync(&db, input("local:p", &[("b", "b")])).await.unwrap();
        assert_eq!(first["photoIDs"]["b"], removed["photoIDs"]["b"]);
        assert!(removed["revision"].as_i64() > first["revision"].as_i64());
        let other = sync(&db, input("cloud:p", &[("a", "a")])).await.unwrap();
        assert_ne!(first["projectId"], other["projectId"]);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM preference_evidence")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
        let empty = sync(&db, input("local:p", &[])).await.unwrap();
        assert_ne!(empty["projectId"], first["projectId"]);
    }
}
