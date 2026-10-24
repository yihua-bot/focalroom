//! 修订中的历史照片观察：保留原调用和源像素身份，不当作本轮重新观察。
use super::types::{Task, ToolResult};
use crate::{
    command_audit::{encode, serialization},
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionObservation {
    pub photo_id: String,
    pub photo_hash: String,
    pub model_id: String,
    pub prompt_version: String,
    pub task_id: String,
    pub call_id: String,
    pub visible_facts: Vec<String>,
    pub interpretation: String,
    pub uncertainty: String,
    #[serde(default)]
    pub matched_at_delegation: bool,
}
pub(crate) async fn collect(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    creator: Option<&str>,
    inherited: Vec<RevisionObservation>,
) -> Result<Vec<RevisionObservation>> {
    let mut observations = std::collections::BTreeMap::new();
    for observation in inherited
        .into_iter()
        .filter(|o| task.allowed_photo_ids.contains(&o.photo_id))
    {
        observations.insert(observation.photo_id.clone(), observation);
    }
    if let Some(creator) = creator {
        // 先在同一读快照内检查字节数，再读取正文，避免先分配所有大记录再拒绝。
        let (count,bytes,largest):(i64,i64,i64)=sqlx::query_as("SELECT count(*),COALESCE(sum(size),0),COALESCE(max(size),0) FROM (SELECT length(CAST(result_json AS BLOB)) AS size FROM tool_calls WHERE task_id=? AND tool_name IN('inspect_photos','compare_photos') ORDER BY rowid LIMIT 1001)")
            .bind(creator).fetch_one(&mut **tx).await?;
        if count > 1000 || bytes > 8 * 1024 * 1024 || largest > 2 * 1024 * 1024 {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "这份方案的观察记录较多，请缩小修改范围",
            ));
        }
        let rows:Vec<(String,String)> = sqlx::query_as("SELECT call_id,result_json FROM tool_calls WHERE task_id=? AND tool_name IN('inspect_photos','compare_photos') ORDER BY rowid LIMIT 1000")
            .bind(creator).fetch_all(&mut **tx).await?;
        for (call_id, raw) in rows {
            let receipt: ToolResult = serde_json::from_str(&raw).map_err(serialization)?;
            let Some(data) = receipt
                .result
                .filter(|v| receipt.ok && v["mode"] == "vision_observed")
            else {
                continue;
            };
            for p in data["provenance"].as_array().into_iter().flatten() {
                let Some(photo) = p["photo_id"]
                    .as_str()
                    .filter(|id| task.allowed_photo_ids.iter().any(|allowed| allowed == id))
                else {
                    continue;
                };
                let Some(hash) = p["photo_hash"]
                    .as_str()
                    .filter(|hash| hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()))
                else {
                    continue;
                };
                let Some(value) = data["observations"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|v| v["photo_id"] == photo)
                else {
                    continue;
                };
                if !crate::agent::vision::valid_cached_observation(value, photo) {
                    continue;
                }
                observations.insert(
                    photo.to_owned(),
                    RevisionObservation {
                        photo_id: photo.into(),
                        photo_hash: hash.into(),
                        model_id: p["model_id"].as_str().unwrap_or_default().into(),
                        prompt_version: p["prompt_version"].as_str().unwrap_or_default().into(),
                        task_id: creator.into(),
                        call_id: call_id.clone(),
                        visible_facts: serde_json::from_value(value["visible_facts"].clone())
                            .map_err(serialization)?,
                        interpretation: value["interpretation"].as_str().unwrap_or_default().into(),
                        uncertainty: value["uncertainty"].as_str().unwrap_or_default().into(),
                        matched_at_delegation: false,
                    },
                );
            }
        }
    }
    let current: Vec<(String, String)> =
        sqlx::query_as("SELECT id,content_sha256 FROM photos WHERE project_id=? AND id IN(SELECT value FROM json_each(?))")
            .bind(&task.project_id)
            .bind(encode(&task.allowed_photo_ids)?)
            .fetch_all(&mut **tx)
            .await?;
    let current: HashMap<_, _> = current.into_iter().collect();
    for observation in observations.values_mut() {
        observation.matched_at_delegation =
            current.get(&observation.photo_id) == Some(&observation.photo_hash);
    }
    Ok(observations.into_values().collect())
}

// 仅供GUI投影；本地文件路径不进入模型上下文或冻结记录。
pub(crate) async fn project(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    observations: &[RevisionObservation],
    offset: usize,
    limit: usize,
) -> Result<Vec<Value>> {
    let mut projected = Vec::new();
    for o in observations.iter().skip(offset).take(limit) {
        let current: Option<(String, String, String)> = sqlx::query_as(
            "SELECT content_sha256,filename,preview_path FROM photos WHERE project_id=? AND id=?",
        )
        .bind(project)
        .bind(&o.photo_id)
        .fetch_optional(&mut **tx)
        .await?;
        let mut value = serde_json::to_value(o).map_err(serialization)?;
        let matches = current
            .as_ref()
            .is_some_and(|(hash, _, _)| hash == &o.photo_hash);
        value["currentHashMatches"] = json!(matches);
        value["photoName"] = json!(current.as_ref().map_or("", |(_, name, _)| name.as_str()));
        value["thumbnailPath"] = json!(
            current
                .as_ref()
                .filter(|_| matches)
                .map(|(_, _, preview)| preview)
        );
        projected.push(value);
    }
    Ok(projected)
}
