//! 分层检索的本地切片：人工元数据 + 已有缓存可观察事实；不读像素、不触发模型。
use super::{
    model_binding, semantic,
    types::{Task, ToolCall},
    vision,
};
use crate::{
    command_audit::encode,
    domain::{AppError, Result},
};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction, sqlite::SqliteRow};
use std::collections::HashMap;

const SCAN_LIMIT: usize = 500;
const MAX_OBSERVATION_BYTES: i64 = 65_536;

pub(crate) struct SemanticPrefetch {
    task_id: String,
    call_fingerprint: String,
    scores: HashMap<String, f64>,
    report: semantic::SemanticReport,
}

/// 在写事务前计算本地向量；写事务重新读取并核对观察内容摘要，不采信过期快照。
pub(crate) async fn prepare(
    db: &crate::db::Store,
    task_id: &str,
    call: &ToolCall,
) -> Result<Option<SemanticPrefetch>> {
    if call.name != "search_photos_semantic" {
        return Ok(None);
    }
    let mut tx = db.pool.begin().await?;
    let task = super::store::load(&mut tx, task_id).await?;
    let completed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls WHERE task_id=? AND call_id=?")
            .bind(task_id)
            .bind(&call.call_id)
            .fetch_one(&mut *tx)
            .await?;
    if completed > 0
        || super::policy::PolicyGate::validate(&mut tx, &task, call)
            .await
            .is_err()
        || task.usage.tool_calls >= task.budget.max_tool_calls
    {
        return Ok(None);
    }
    let query = call.args["query"].as_str().unwrap().trim().to_owned();
    if query.is_empty() || query.to_lowercase() == "rating:4" {
        return Ok(None);
    }
    let remaining_ms = (u64::from(task.budget.max_duration_seconds) * 1000)
        .saturating_sub(task.elapsed_ms)
        .min(3000) as u32;
    let rows = cache_rows(&mut tx, &task).await?;
    let mut documents = Vec::new();
    let mut keys = Vec::new();
    for row in rows.iter().take(SCAN_LIMIT) {
        let Some(value) = parse_observation(row) else {
            continue;
        };
        let facts = semantic_facts(&value);
        if facts.is_empty() {
            continue;
        }
        keys.push(document_key(row, &value));
        documents.push(facts.join("\n"));
    }
    tx.commit().await?;
    let ranking = semantic::rank(query, documents, remaining_ms).await;
    Ok(Some(SemanticPrefetch {
        task_id: task_id.into(),
        call_fingerprint: crate::command_audit::fingerprint(&serde_json::to_value(call).unwrap()),
        scores: keys
            .into_iter()
            .zip(ranking.scores)
            .filter_map(|(key, score)| score.map(|score| (key, score)))
            .collect(),
        report: ranking.report,
    }))
}

fn semantic_facts(value: &Value) -> Vec<&str> {
    value["visible_facts"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .filter(|fact| {
            !fact.trim().is_empty() && fact.chars().count() <= 400 && !fact.contains('\0')
        })
        .take(2)
        .collect()
}

fn document_key(row: &SqliteRow, value: &Value) -> String {
    crate::command_audit::fingerprint(
        &json!({"id":row.get::<i64,_>("id"),"photo":row.get::<String,_>("photo_id"),"model":row.get::<String,_>("model_id"),"version":row.get::<String,_>("prompt_version"),"kind":row.get::<String,_>("kind"),"value":value}),
    )
}

fn parse_observation(row: &SqliteRow) -> Option<Value> {
    let cache_version: String = row.get("prompt_version");
    let prefix = format!("{}@", vision::VISION_PROMPT_VERSION);
    let suffix = cache_version.strip_prefix(&prefix)?;
    if suffix.len() != 64
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let raw = row.get::<Option<String>, _>("payload")?;
    let value = serde_json::from_str(&raw).ok()?;
    vision::valid_cached_observation(&value, &row.get::<String, _>("photo_id")).then_some(value)
}

async fn cache_rows(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<Vec<SqliteRow>> {
    let identity = model_binding::load(tx, &task.id).await?;
    if task.provider_id == super::PROVIDER_OPENAI && identity.is_none() {
        return Err(AppError::new(
            "RECOVERY_ERROR",
            "云任务缺少模型身份，不能猜测缓存来源；请重新委托",
        ));
    }
    let model = identity.as_ref().map(|id| id.model_id.as_str());
    let version = identity.as_ref().map(|id| id.cache_version());
    // 仅取 scope 内每照片最新记录，正文有界；不展开全部历史 JSON。
    Ok(sqlx::query(r#"
        SELECT o.id,o.photo_id,o.model_id,o.prompt_version,o.kind,
            CASE WHEN length(CAST(o.observation_json AS BLOB))<=? THEN o.observation_json END AS payload
        FROM photo_observations o
        JOIN (
            SELECT MAX(eligible.id) AS id
            FROM photo_observations eligible
            JOIN photos p ON p.id=eligible.photo_id AND p.content_sha256=eligible.photo_hash
            JOIN json_each(?) scope ON scope.value=p.id
            WHERE p.project_id=? AND eligible.prompt_version LIKE ?
              AND (? IS NULL OR eligible.model_id=?) AND (? IS NULL OR eligible.prompt_version=?)
            GROUP BY eligible.photo_id ORDER BY MAX(eligible.id) DESC LIMIT ?
        ) latest ON latest.id=o.id ORDER BY o.id DESC
    "#).bind(MAX_OBSERVATION_BYTES).bind(encode(&task.allowed_photo_ids)?).bind(&task.project_id)
        .bind(format!("{}@%", vision::VISION_PROMPT_VERSION)).bind(model).bind(model).bind(&version).bind(&version)
        .bind((SCAN_LIMIT + 1) as i64).fetch_all(&mut **tx).await?)
}

pub(crate) async fn search(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    query: &str,
    limit: usize,
    mut photos: Vec<Value>,
    call: &ToolCall,
    semantic: Option<&SemanticPrefetch>,
) -> Result<Value> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Err(AppError::new("VALIDATION_ERROR", "检索关键词不能为空白"));
    }
    let semantic = semantic.filter(|data| {
        data.task_id == task.id
            && data.call_fingerprint
                == crate::command_audit::fingerprint(&serde_json::to_value(call).unwrap())
    });
    let mut evidence = HashMap::new();
    let mut scanned = 0;
    let mut valid = 0;
    let mut truncated = false;
    let mut revalidated = 0;
    if query != "rating:4" {
        let rows = cache_rows(tx, task).await?;
        truncated = rows.len() > SCAN_LIMIT;
        for row in rows.iter().take(SCAN_LIMIT) {
            scanned += 1;
            let photo: String = row.get("photo_id");
            let cache_version: String = row.get("prompt_version");
            let Some(value) = parse_observation(row) else {
                continue;
            };
            valid += 1;
            // 仅可观察事实参与关键词匹配；主观解读不进入索引，所有内容仍是不可信模型数据。
            let mut facts: Vec<_> = value["visible_facts"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .filter(|fact| fact.chars().count() <= 1000 && fact.to_lowercase().contains(&query))
                .take(2)
                .collect();
            let score = semantic
                .and_then(|data| data.scores.get(&document_key(row, &value)))
                .copied();
            revalidated += usize::from(score.is_some());
            let semantic_only = facts.is_empty();
            if semantic_only && score.is_none() {
                continue;
            }
            if semantic_only {
                facts = semantic_facts(&value);
            }
            let mut cited = json!({"source":"cached_model_observation","observation_id":row.get::<i64,_>("id"),"model_id":row.get::<String,_>("model_id"),"cache_version":cache_version,"kind":row.get::<String,_>("kind"),"visible_facts":facts,"uncertainty":value["uncertainty"].as_str().unwrap_or("").chars().take(300).collect::<String>(),"human_preference_evidence":false});
            if let Some(score) = score.filter(|_| semantic_only) {
                cited["semantic_similarity"] = json!(score);
            }
            evidence.insert(photo, (cited, semantic_only));
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
                cached.map(|(_, semantic_only)| if *semantic_only {
                    "semantic_cached_visual_fact"
                } else {
                    "cached_visual_fact"
                })
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
        );
        if let Some((cached, _)) = cached {
            photo["visual_evidence"] = cached.clone();
        }
        true
    });
    if semantic.is_some() {
        photos.sort_by(|a, b| {
            let rank = |photo: &Value| {
                if photo["match_sources"].as_array().is_some_and(|sources| {
                    sources.iter().any(|s| s != "semantic_cached_visual_fact")
                }) {
                    0
                } else {
                    1
                }
            };
            rank(a).cmp(&rank(b)).then_with(|| {
                b["visual_evidence"]["semantic_similarity"]
                    .as_f64()
                    .unwrap_or(-1.0)
                    .total_cmp(
                        &a["visual_evidence"]["semantic_similarity"]
                            .as_f64()
                            .unwrap_or(-1.0),
                    )
            })
        });
    }
    let total = photos.len();
    photos.truncate(limit);
    let mut result = json!({"photos":photos,"remaining":total.saturating_sub(limit),"mode":"local_metadata_and_cached_facts","new_uploads":0,"cache_coverage":{"examined":scanned,"valid":valid,"scan_limit":SCAN_LIMIT,"limited":truncated},"notice":"本地关键词匹配；缓存事实是模型观察，可能有误，不是人工偏好证据。未命中不代表照片未包含该内容；未重新读图，未观察全图库。"});
    if call.name == "search_photos_semantic" {
        result["mode"] = json!(if revalidated > 0 {
            "local_hybrid_sentence_embeddings"
        } else {
            "local_keyword_fallback"
        });
        result["semantic_retrieval"] = serde_json::to_value(
            semantic
                .map(|data| data.report.clone())
                .unwrap_or_else(|| semantic::SemanticReport::skipped("not_prepared", 0)),
        )
        .unwrap();
        result["semantic_retrieval"]["revalidated"] = json!(revalidated);
        result["semantic_retrieval"]["discarded_prepared"] =
            json!(semantic.map_or(0, |data| data.report.embedded.saturating_sub(revalidated)));
        result["semantic_facts_per_photo_limit"] = json!(2);
        result["notice"] = json!(
            "本地缓存描述的语义候选排序，关键词命中优先；每照片至多两条各400字事实。相似度不是内容置信度或审美评分，无相关性通过阈值，结果可能不相关；仅覆盖已生成向量的同语言描述，不读图、不上传、不新增偏好证据。请核对来源并按授权补充视觉观察。"
        );
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::{AgentRequest, Budget, TaskCommand};
    use crate::{command_handler::CommandHandler, db::Store, domain, image_pipeline};

    #[tokio::test]
    async fn prepared_scores_revalidate_payload_scope_and_call_then_cancellation_stops_writes() {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("scope.sqlite"))
            .await
            .unwrap();
        let project = db.create("合成范围测试", "不进行真实推断").await.unwrap();
        let path = temp.path().join("fixture.png");
        image::RgbImage::from_pixel(8, 8, image::Rgb([1, 2, 3]))
            .save(&path)
            .unwrap();
        db.insert_photo(
            &project.id,
            &image_pipeline::prepare(&path, &temp.path().join("cache")).unwrap(),
        )
        .await
        .unwrap();
        let project = db.get(&project.id).await.unwrap();
        let photo = &project.photos[0].id;
        let task = CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.id.clone(),
                command: TaskCommand::StartTask {
                    goal: "检索测试".into(),
                    selected_photo_ids: vec![],
                    target_count: 1,
                    budget: Budget::default(),
                    consent_id: None,
                },
            })
            .await
            .unwrap();
        let call = ToolCall {
            call_id: "semantic-unit".into(),
            name: "search_photos_semantic".into(),
            args: json!({"project_id":project.id,"query":"同义查找","limit":1}),
        };
        let value = json!({"photo_id":photo,"visible_facts":["自行车经过街道"],"interpretation":"不进入向量","uncertainty":"合成测试"});
        sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,created_at) SELECT id,content_sha256,'test-model',?,'inspect_photos',?,CURRENT_TIMESTAMP FROM photos WHERE id=?")
            .bind(format!("{}@{}",vision::VISION_PROMPT_VERSION,"a".repeat(64))).bind(value.to_string()).bind(photo).execute(&db.pool).await.unwrap();
        let mut tx = db.pool.begin().await.unwrap();
        let mut fixture_task = super::super::store::load(&mut tx, &task.id).await.unwrap();
        fixture_task.provider_id = "mock".into();
        let rows = cache_rows(&mut tx, &fixture_task).await.unwrap();
        // 这里只注入排序分数验证失效规则；真实向量另外以显式原生测试验收。
        let mut prepared = SemanticPrefetch {
            task_id: task.id.clone(),
            call_fingerprint: crate::command_audit::fingerprint(
                &serde_json::to_value(&call).unwrap(),
            ),
            scores: HashMap::from([(document_key(&rows[0], &value), 0.9)]),
            report: semantic::SemanticReport::skipped("test_fixture", 1),
        };
        prepared.report.embedded = 1;
        let photos = || vec![json!({"photo_id":photo,"name":"fixture.png","note":""})];
        let found = search(
            &mut tx,
            &fixture_task,
            "同义查找",
            1,
            photos(),
            &call,
            Some(&prepared),
        )
        .await
        .unwrap();
        assert_eq!(found["photos"][0]["photo_id"], *photo);
        assert_eq!(found["semantic_retrieval"]["revalidated"], 1);
        let mut other_call = call.clone();
        other_call.call_id = "different-call".into();
        assert_eq!(
            search(
                &mut tx,
                &fixture_task,
                "同义查找",
                1,
                photos(),
                &other_call,
                Some(&prepared)
            )
            .await
            .unwrap()["photos"],
            json!([])
        );
        let mut restricted = fixture_task.clone();
        restricted.allowed_photo_ids.clear();
        assert_eq!(
            search(
                &mut tx,
                &restricted,
                "同义查找",
                1,
                photos(),
                &call,
                Some(&prepared)
            )
            .await
            .unwrap()["photos"],
            json!([])
        );
        let mut changed = value.clone();
        changed["visible_facts"] = json!(["餐桌上有水果"]);
        sqlx::query("UPDATE photo_observations SET observation_json=?")
            .bind(changed.to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        let stale = search(
            &mut tx,
            &fixture_task,
            "同义查找",
            1,
            photos(),
            &call,
            Some(&prepared),
        )
        .await
        .unwrap();
        assert_eq!(stale["photos"], json!([]));
        assert_eq!(stale["semantic_retrieval"]["discarded_prepared"], 1);
        sqlx::query("DELETE FROM photo_observations")
            .execute(&mut *tx)
            .await
            .unwrap();
        assert_eq!(
            search(
                &mut tx,
                &fixture_task,
                "同义查找",
                1,
                photos(),
                &call,
                Some(&prepared)
            )
            .await
            .unwrap()["photos"],
            json!([])
        );
        tx.rollback().await.unwrap();
        CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.id,
                command: TaskCommand::CancelTask {
                    task_id: task.id.clone(),
                },
            })
            .await
            .unwrap();
        assert_eq!(
            CommandHandler::new(&db)
                .agent_tool(&task.id, &call, None)
                .await
                .unwrap_err()
                .code,
            "CANCELLED"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tool_calls")
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            0
        );
    }
}
