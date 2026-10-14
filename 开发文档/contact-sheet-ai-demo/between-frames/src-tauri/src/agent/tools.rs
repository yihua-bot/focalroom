use super::{drafts::replace_items, store, types::*};
use crate::{
    command_audit::serialization,
    domain::{self, AppError, Result},
};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::{HashMap, HashSet};

pub(crate) async fn execute(
    tx: &mut Transaction<'_, Sqlite>,
    task: &mut Task,
    call: &ToolCall,
) -> Result<Value> {
    let a = &call.args;
    match call.name.as_str() {
        "get_project_context" => {
            let row = sqlx::query("SELECT brief,revision FROM projects WHERE id=?")
                .bind(&task.project_id)
                .fetch_one(&mut **tx)
                .await?;
            let mut facts = photos(tx, task).await?;
            let high = facts
                .iter()
                .filter(|p| p["rating"].as_i64().unwrap_or(0) >= 4)
                .count();
            facts.truncate(8);
            Ok(
                json!({"project_id":task.project_id,"revision":row.get::<i64,_>("revision"),"brief":row.get::<String,_>("brief"),"scope_count":task.allowed_photo_ids.len(),"selected_ids":task.selected_photo_ids,"high_rated_count":high,"human_facts":facts,"mock":true}),
            )
        }
        "get_sequence" => {
            let ids: Vec<String> = sqlx::query_scalar(
                "SELECT photo_id FROM sequence_items WHERE project_id=? ORDER BY position",
            )
            .bind(&task.project_id)
            .fetch_all(&mut **tx)
            .await?;
            let mut scoped: Vec<_> = ids
                .into_iter()
                .filter(|id| task.allowed_photo_ids.contains(id))
                .collect();
            for id in &task.selected_photo_ids {
                if !scoped.contains(id) {
                    scoped.push(id.clone());
                }
            }
            Ok(json!({"photo_ids":scoped,"base_revision":task.base_revision}))
        }
        "list_photos" | "search_photos" => {
            let mut data = photos(tx, task).await?;
            if call.name == "list_photos" {
                if let Some(status) = a["status"].as_str() {
                    data.retain(|p| p["status"] == status);
                }
                if let Some(cursor) = a["cursor"].as_str() {
                    let n = cursor
                        .parse::<usize>()
                        .map_err(|_| AppError::new("VALIDATION_ERROR", "分页 cursor 不合法"))?;
                    data = data.into_iter().skip(n).collect();
                }
            } else {
                let query = a["query"].as_str().unwrap().to_lowercase();
                if query == "rating:4" {
                    data.retain(|p| p["rating"].as_i64().unwrap_or(0) >= 4);
                } else {
                    data.retain(|p| {
                        format!(
                            "{} {}",
                            p["name"].as_str().unwrap_or(""),
                            p["note"].as_str().unwrap_or("")
                        )
                        .to_lowercase()
                        .contains(&query)
                    });
                }
            }
            let limit = a["limit"].as_u64().unwrap() as usize;
            let total = data.len();
            data.truncate(limit);
            Ok(json!({"photos":data,"remaining":total.saturating_sub(limit),"mock":true}))
        }
        "inspect_photos" | "compare_photos" => {
            let ids: Vec<String> =
                serde_json::from_value(a["photo_ids"].clone()).map_err(serialization)?;
            let mut data = photos(tx, task).await?;
            data.retain(|p| ids.iter().any(|id| p["photo_id"] == *id));
            Ok(
                json!({"photos":data,"mode":"mock_metadata_only","uncertainty":"未读取像素；仅对照人工评分/备注"}),
            )
        }
        "retrieve_preferences" => {
            if a["scope"] != "project_local" {
                return Err(AppError::new(
                    "PERMISSION_DENIED",
                    "跨项目记忆未实现，不能读取其他项目",
                ));
            }
            let mut facts = photos(tx, task).await?;
            facts.retain(|p| {
                p["rating"].as_i64().unwrap_or(0) > 0
                    || p["note"].as_str().is_some_and(|s| !s.is_empty())
            });
            facts.truncate(a["limit"].as_u64().unwrap() as usize);
            Ok(
                json!({"human_facts":facts,"hypotheses":[],"mode":"project_facts_only","notice":"未生成长期偏好"}),
            )
        }
        "create_selection_draft" => {
            if a["base_project_revision"].as_i64() != Some(task.base_revision) {
                return Err(AppError::new(
                    "REVISION_CONFLICT",
                    "草案必须使用任务原始基线",
                ));
            }
            let items: Vec<DraftItem> =
                serde_json::from_value(a["items"].clone()).map_err(serialization)?;
            validate_items(&items)?;
            let id = domain::id();
            let now = domain::timestamp();
            sqlx::query("INSERT INTO drafts(id,project_id,task_id,title,base_revision,state,summary,created_at,updated_at) VALUES(?,?,?,?,?,'pending',?,?,?)").bind(&id).bind(&task.project_id).bind(&task.id).bind(a["title"].as_str()).bind(task.base_revision).bind(a["summary"].as_str()).bind(&now).bind(&now).execute(&mut **tx).await?;
            replace_items(tx, &id, &items).await?;
            Ok(json!({"draft_id":id,"draft_revision":1,"count":items.len(),"title":a["title"]}))
        }
        "update_draft_sequence" => {
            let id = a["draft_id"].as_str().unwrap();
            let revision: i64 = sqlx::query_scalar("SELECT revision FROM drafts WHERE id=?")
                .bind(id)
                .fetch_one(&mut **tx)
                .await?;
            if Some(revision) != a["expected_draft_revision"].as_i64() {
                return Err(AppError::new("REVISION_CONFLICT", "草案已更新"));
            }
            let old = store::items(tx, id).await?;
            let ids: Vec<String> =
                serde_json::from_value(a["ordered_photo_ids"].clone()).map_err(serialization)?;
            if ids.is_empty() {
                return Err(AppError::new("VALIDATION_ERROR", "草案不能空"));
            }
            let items: Vec<_> = ids
                .iter()
                .enumerate()
                .map(|(position, id)| {
                    old.iter()
                        .find(|i| &i.photo_id == id)
                        .cloned()
                        .map(|mut i| {
                            i.position = position as i64;
                            i
                        })
                        .unwrap_or(DraftItem {
                            photo_id: id.clone(),
                            proposed_action: "keep".into(),
                            position: position as i64,
                            rationale: "Mock：按已授权元数据修订候选".into(),
                        })
                })
                .collect();
            validate_items(&items)?;
            replace_items(tx, a["draft_id"].as_str().unwrap(), &items).await?;
            sqlx::query("UPDATE drafts SET revision=revision+1,updated_at=? WHERE id=?")
                .bind(domain::timestamp())
                .bind(id)
                .execute(&mut **tx)
                .await?;
            Ok(json!({"draft_id":id,"draft_revision":revision+1,"count":items.len()}))
        }
        "evaluate_draft" => {
            let items = store::items(tx, a["draft_id"].as_str().unwrap()).await?;
            let facts = photos(tx, task).await?;
            let mut groups: HashMap<String, Vec<String>> = HashMap::new();
            for i in &items {
                if let Some(note) = facts
                    .iter()
                    .find(|p| p["photo_id"] == i.photo_id)
                    .and_then(|p| p["note"].as_str())
                    && let Some(group) = note
                        .split("重复组:")
                        .nth(1)
                        .and_then(|s| s.split_whitespace().next())
                {
                    groups
                        .entry(group.into())
                        .or_default()
                        .push(i.photo_id.clone());
                }
            }
            let mut duplicates: Vec<_> = groups.into_values().filter(|ids| ids.len() > 1).collect();
            duplicates.sort();
            Ok(
                json!({"draft_id":a["draft_id"],"valid":true,"count":items.len(),"duplicate_groups":duplicates,"mode":"mock_human_annotation_only","uncertainty":"重复组来自人工备注，未做像素相似性分析"}),
            )
        }
        "request_feedback" => {
            task.question = Some(a["question"].as_str().unwrap().into());
            store::transition(tx, task, "needs_input", "等待人工答复").await?;
            sqlx::query("INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'clarification_question',?,?)").bind(&task.id).bind(&task.question).bind(domain::timestamp()).execute(&mut **tx).await?;
            Ok(json!({"question":task.question,"options":a["options"]}))
        }
        "finish_task" => {
            if a["task_id"] != task.id {
                return Err(AppError::new("PERMISSION_DENIED", "不能结束其他任务"));
            }
            let ids: Vec<String> =
                serde_json::from_value(a["draft_ids"].clone()).map_err(serialization)?;
            validate_final(tx, task, &ids).await?;
            task.final_draft_ids = ids;
            store::transition(tx, task, "review_ready", a["summary"].as_str().unwrap()).await?;
            let revision: i64 = sqlx::query_scalar("SELECT revision FROM projects WHERE id=?")
                .bind(&task.project_id)
                .fetch_one(&mut **tx)
                .await?;
            if revision != task.base_revision {
                store::transition(tx, task, "stale", "正式编辑已更新，请重新比较草案基线").await?;
            }
            Ok(json!({"draft_ids":task.final_draft_ids,"formal_state_unchanged":true}))
        }
        _ => Err(AppError::new("PERMISSION_DENIED", "未知工具")),
    }
}
pub(crate) async fn validate_final(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    ids: &[String],
) -> Result<()> {
    if ids.is_empty() || ids.len() > 10 || ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err(AppError::new("VALIDATION_ERROR", "完成必须引用真实草案"));
    }
    for id in ids {
        let owned:i64=sqlx::query_scalar("SELECT COUNT(*) FROM drafts WHERE id=? AND task_id=? AND project_id=? AND state='pending'").bind(id).bind(&task.id).bind(&task.project_id).fetch_one(&mut **tx).await?;
        if owned != 1 {
            return Err(AppError::new(
                "NOT_FOUND",
                "最终结果引用了不存在或其他任务的草案",
            ));
        }
    }
    Ok(())
}
fn validate_items(items: &[DraftItem]) -> Result<()> {
    if items.is_empty()
        || items.len() > 500
        || items
            .iter()
            .map(|i| &i.photo_id)
            .collect::<HashSet<_>>()
            .len()
            != items.len()
        || items
            .iter()
            .enumerate()
            .any(|(p, i)| i.position != p as i64)
    {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "草案位置须连续且照片不得重复",
        ));
    }
    Ok(())
}
pub(crate) async fn photos(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<Vec<Value>> {
    let rows=sqlx::query("SELECT p.id,p.filename,d.status,d.rating,d.user_note FROM photos p JOIN photo_decisions d ON d.photo_id=p.id AND d.project_id=p.project_id WHERE p.project_id=? ORDER BY d.rating DESC,p.import_index").bind(&task.project_id).fetch_all(&mut **tx).await?;
    Ok(rows.iter().filter(|r|task.allowed_photo_ids.contains(&r.get::<String,_>("id"))).map(|r|json!({"photo_id":r.get::<String,_>("id"),"name":r.get::<String,_>("filename").chars().take(80).collect::<String>(),"status":r.get::<String,_>("status"),"rating":r.get::<i64,_>("rating"),"note":r.get::<String,_>("user_note").chars().take(160).collect::<String>()})).collect())
}
