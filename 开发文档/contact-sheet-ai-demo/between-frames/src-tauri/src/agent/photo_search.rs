//! 分层检索的本地切片：人工元数据 + 已有缓存可观察事实；不读像素、不触发模型。
use super::{model_binding, types::Task, vision};
use crate::{
    command_audit::encode,
    domain::{AppError, Result},
};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::HashMap;

const SCAN_LIMIT: usize = 500;
const MAX_OBSERVATION_BYTES: i64 = 65_536;

pub(crate) async fn search(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    query: &str,
    limit: usize,
    mut photos: Vec<Value>,
) -> Result<Value> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Err(AppError::new("VALIDATION_ERROR", "检索关键词不能为空白"));
    }
    let mut evidence = HashMap::new();
    let mut scanned = 0;
    let mut valid = 0;
    let mut truncated = false;
    if query != "rating:4" {
        let identity = model_binding::load(tx, &task.id).await?;
        let model = identity.as_ref().map(|id| id.model_id.as_str());
        let version = identity.as_ref().map(|id| id.cache_version());
        // 只为范围内最新记录读取 JSON 正文；历史聚合只处理 ID，不展开所有旧 payload。
        let rows = sqlx::query(r#"
            SELECT o.id,o.photo_id,o.model_id,o.prompt_version,o.kind,
                CASE WHEN length(CAST(o.observation_json AS BLOB))<=? THEN o.observation_json END AS payload
            FROM photo_observations o
            JOIN (
                SELECT MAX(eligible.id) AS id
                FROM photo_observations eligible
                JOIN photos p ON p.id=eligible.photo_id AND p.content_sha256=eligible.photo_hash
                JOIN json_each(?) scope ON scope.value=p.id
                WHERE p.project_id=? AND eligible.prompt_version LIKE ?
                  AND (? IS NULL OR eligible.model_id=?)
                  AND (? IS NULL OR eligible.prompt_version=?)
                GROUP BY eligible.photo_id
                ORDER BY MAX(eligible.id) DESC LIMIT ?
            ) latest ON latest.id=o.id
            ORDER BY o.id DESC
        "#)
            .bind(MAX_OBSERVATION_BYTES).bind(encode(&task.allowed_photo_ids)?).bind(&task.project_id)
            .bind(format!("{}@%", vision::VISION_PROMPT_VERSION)).bind(model).bind(model).bind(&version).bind(&version)
            .bind((SCAN_LIMIT + 1) as i64).fetch_all(&mut **tx).await?;
        truncated = rows.len() > SCAN_LIMIT;
        for row in rows.iter().take(SCAN_LIMIT) {
            scanned += 1;
            let photo: String = row.get("photo_id");
            let cache_version: String = row.get("prompt_version");
            let suffix = cache_version
                .strip_prefix(&format!("{}@", vision::VISION_PROMPT_VERSION))
                .unwrap_or("");
            if suffix.len() != 64
                || !suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                continue;
            }
            let Some(raw) = row.get::<Option<String>, _>("payload") else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            if !vision::valid_cached_observation(&value, &photo) {
                continue;
            }
            valid += 1;
            // 仅可观察事实参与关键词匹配；主观解读不进入索引，所有内容仍是不可信模型数据。
            let facts: Vec<_> = value["visible_facts"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .filter(|fact| fact.chars().count() <= 1000 && fact.to_lowercase().contains(&query))
                .take(2)
                .collect();
            if facts.is_empty() {
                continue;
            }
            evidence.insert(photo, json!({"source":"cached_model_observation","observation_id":row.get::<i64,_>("id"),"model_id":row.get::<String,_>("model_id"),"cache_version":cache_version,"kind":row.get::<String,_>("kind"),"visible_facts":facts,"uncertainty":value["uncertainty"].as_str().unwrap_or("").chars().take(300).collect::<String>(),"human_preference_evidence":false}));
        }
    }
    photos.retain_mut(|photo| {
        if query == "rating:4" {
            return photo["rating"].as_i64().unwrap_or(0) >= 4;
        }
        let metadata = format!(
            "{} {}",
            photo["name"].as_str().unwrap_or(""),
            photo["note"].as_str().unwrap_or("")
        )
        .to_lowercase()
        .contains(&query);
        let cached = photo["photo_id"].as_str().and_then(|id| evidence.get(id));
        if !metadata && cached.is_none() {
            return false;
        }
        photo["match_sources"] = json!(
            [
                metadata.then_some("metadata"),
                cached.map(|_| "cached_visual_fact")
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
        );
        if let Some(cached) = cached {
            photo["visual_evidence"] = cached.clone();
        }
        true
    });
    let total = photos.len();
    photos.truncate(limit);
    Ok(
        json!({"photos":photos,"remaining":total.saturating_sub(limit),"mode":"local_metadata_and_cached_facts","new_uploads":0,"cache_coverage":{"examined":scanned,"valid":valid,"scan_limit":SCAN_LIMIT,"limited":truncated},"notice":"本地关键词匹配；缓存事实是模型观察，可能有误，不是人工偏好证据。未命中不代表照片未包含该内容；未重新读图，未观察全图库。"}),
    )
}
