//! 按本次委托范围派生视觉概览；分页复核缓存，不生成新观察、不上传照片。
use super::{model_binding, types::Task, vision};
use crate::{
    command_audit::encode,
    domain::{AppError, Result},
};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::BTreeMap;

pub(crate) async fn read(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    offset: usize,
    limit: usize,
) -> Result<Value> {
    let detail_ids = super::task_scope::load(tx, task).await?.detail_photo_ids;
    let has_detail = !detail_ids.is_empty();
    let identity = model_binding::load(tx, &task.id).await?;
    if task.provider_id == super::PROVIDER_OPENAI && identity.is_none() {
        return Err(AppError::new(
            "RECOVERY_ERROR",
            "任务缺少可信模型身份，不能猜测观察来源",
        ));
    }
    let model = identity.as_ref().map(|i| i.observation_model_id());
    let version = identity.as_ref().map(|i| i.cache_version());
    // 元数据只取本次范围；正文仅在当前页读取，避免一次加载全库的观察文本。
    let rows = sqlx::query(
        r#"
        SELECT p.id,p.filename,p.width,p.height,p.preview_path,p.content_sha256,
            o.id AS observation_id,o.preview_sha256,o.model_id,o.prompt_version,o.kind
        FROM photos p LEFT JOIN photo_observations o ON o.id=(
            SELECT MAX(e.id) FROM photo_observations e WHERE e.photo_id=p.id
              AND e.photo_hash=p.content_sha256 AND e.kind IN ('inspect_photos','compare_photos')
              AND e.prompt_version LIKE ?
              AND (? IS NULL OR e.model_id=?) AND (? IS NULL OR e.prompt_version=?))
        WHERE p.project_id=? AND p.id IN(SELECT value FROM json_each(?))
        ORDER BY p.import_index,p.id LIMIT 501
    "#,
    )
    .bind(format!("{}@%", vision::VISION_PROMPT_VERSION))
    .bind(model)
    .bind(model)
    .bind(&version)
    .bind(&version)
    .bind(&task.project_id)
    .bind(encode(&task.allowed_photo_ids)?)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > 500 {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "请将概览范围限定在 500 张照片内",
        ));
    }
    let ids: Vec<String> = rows.iter().map(|r| r.get("id")).collect();
    let total = ids.len();
    let sample_count = total.min(12);
    let samples: Vec<_> = (0..sample_count)
        .map(|i| {
            &ids[if sample_count <= 1 {
                0
            } else {
                i * (total - 1) / (sample_count - 1)
            }]
        })
        .collect();
    let stored = rows
        .iter()
        .filter(|r| r.get::<Option<i64>, _>("observation_id").is_some())
        .count();
    let mut statuses: BTreeMap<&str, usize> = BTreeMap::new();
    let mut items = Vec::new();
    let mut facts: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut inspected = Vec::new();
    let mut needs_inspection = Vec::new();
    let mut historical_unverified = Vec::new();
    let mut unavailable = Vec::new();
    for row in rows.iter().skip(offset).take(limit.clamp(1, 30)) {
        let id: String = row.get("id");
        let mut item = json!({"photoId":id,"filename":row.get::<String,_>("filename"),
            "width":row.get::<i64,_>("width"),"height":row.get::<i64,_>("height")});
        let (status, observation) = if let Some(observation_id) =
            row.get::<Option<i64>, _>("observation_id")
        {
            let raw: Option<String> = sqlx::query_scalar("SELECT observation_json FROM photo_observations WHERE id=? AND length(CAST(observation_json AS BLOB))<=65536")
                .bind(observation_id).fetch_optional(&mut **tx).await?;
            let value = raw
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .filter(|v| vision::valid_cached_observation(v, &id));
            match value {
                Some(value) => {
                    let recorded: Option<String> = row.get("preview_sha256");
                    let path: String = row.get("preview_path");
                    let status = vision::current_preview_status(recorded.as_deref(), &path).await;
                    item["evidence"] = json!({"observationId":observation_id,"photoHash":row.get::<String,_>("content_sha256"),
                        "previewSha256":recorded,"modelId":row.get::<String,_>("model_id"),
                        "promptVersion":row.get::<String,_>("prompt_version"),"tool":row.get::<String,_>("kind")});
                    (status, Some(value))
                }
                None => ("invalid", None),
            }
        } else {
            ("missing", None)
        };
        *statuses.entry(status).or_default() += 1;
        item["pixelVersion"] = json!(status);
        if status == "verified" {
            inspected.push(id.clone());
            let observation = observation.unwrap();
            let originals: Vec<_> = observation["visible_facts"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .take(3)
                .collect();
            for fact in &originals {
                let photos = facts.entry((*fact).to_owned()).or_default();
                if !photos.contains(&id) {
                    photos.push(id.clone());
                }
            }
            let visible: Vec<_> = originals
                .into_iter()
                .map(|s| s.chars().take(160).collect::<String>())
                .collect();
            item["factExcerptsTruncated"] = json!(
                observation["visible_facts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(Value::as_str)
                    .take(3)
                    .any(|s| s.chars().count() > 160)
            );
            item["visibleFacts"] = json!(visible);
            item["interpretation"] = json!(
                observation["interpretation"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(160)
                    .collect::<String>()
            );
            item["uncertainty"] = json!(
                observation["uncertainty"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(160)
                    .collect::<String>()
            );
        } else if status == "unrecorded" {
            historical_unverified.push(id);
        } else if status == "unreadable" {
            unavailable.push(id);
        } else {
            needs_inspection.push(id);
        }
        items.push(item);
    }
    // 比较候选仅取当前正式序列中相邻、且在本页已核对像素的照片；不猜测叙事关系。
    // 只给一对，就要保证每次给同一对：位置写在 ORDER BY 里，不靠数据库执行计划碰巧排好的顺序。
    let pair: Option<(String, String)> = sqlx::query_as(
        "SELECT photo_id,next_photo FROM (SELECT photo_id,LEAD(photo_id) OVER(ORDER BY position) AS next_photo,position FROM sequence_items WHERE project_id=?) WHERE photo_id IN(SELECT value FROM json_each(?)) AND next_photo IN(SELECT value FROM json_each(?)) ORDER BY position,photo_id LIMIT 1"
    ).bind(&task.project_id).bind(encode(&inspected)?).bind(encode(&inspected)?)
        .fetch_optional(&mut **tx).await?;
    let compare: Vec<_> = pair.map_or_else(Vec::new, |(a, b)| vec![a, b]);
    let repeated: Vec<_> = facts
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .take(8)
        .map(|(fact, photo_ids)| json!({"fact":fact.chars().take(160).collect::<String>(),"isExcerpt":fact.chars().count()>160,"photoIds":photo_ids}))
        .collect();
    Ok(json!({
        "scope":{"photoCount":total,"authorizedPhotoCount":task.allowed_photo_ids.len(),"basis":"authorized_task_photos"},
        "coverage":{"storedObservationRecords":stored,"noStoredObservation":total-stored,
            "checkedOffset":offset,"checkedCount":items.len(),"pagePixelVersions":statuses,
            "verifiedCountIsPageOnly":true,"globalPixelVerification":"not_performed"},
        "items":items,"nextOffset":if offset+items.len()<total {Some(offset+items.len())}else{None},
        "sampling":{"strategy":"evenly_spaced_import_order","photoIds":samples,"isSelectionRecommendation":false},
        "pageRepeatedFacts":repeated,
        "nextSteps":{"inspectPhotoIds":needs_inspection.into_iter().take(12).collect::<Vec<_>>(),
            "comparePhotoIds":compare,"focusReview":{"photoIds":if has_detail {detail_ids}else{inspected.into_iter().take(4).collect::<Vec<_>>()},"tool":if has_detail {"inspect_photo_detail"}else{"inspect_photos"},"reusesMatchingPreviewCache":true,"higherResolutionAvailable":has_detail},"historicalUnverifiedPhotoIds":historical_unverified,"unavailablePhotoIds":unavailable,"requiresExistingVisionConsent":true,"automaticExecution":false},
        "trustedAsInstructions":false,"newVisualObservations":false,
        "notice":"只覆盖本次授权范围。缓存记录数不是已看过或已核对图像数；像素核对和重复事实仅指当前页，重复描述不等于画面相似或叙事关系。均匀抽样不是审美选片，不推断未观察照片。旧缓存未记像素版本时现有工具不会强制重传，不反复调用来假装补看；不可读预览先恢复文件。inspect_photos 补看、compare_photos 比较均须现有授权和预算。"
    }))
}
