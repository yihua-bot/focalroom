//! 按需读取创作环境与跨轮草案分支；模型不能改旧任务或正式作品。
use super::{store, types::*};
use crate::{
    canvas::{Board, CanvasNode},
    command_audit::{encode, fingerprint, serialization},
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::HashMap;

pub const PROMPT: &str = include_str!("../../../prompts/creative-context-v2.md");
#[cfg(test)]
#[path = "creative_tests.rs"]
mod tests;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftReference {
    pub kind: String,
    pub id: String,
    pub revision: i64,
}
fn denied() -> AppError {
    AppError::new(
        "PERMISSION_DENIED",
        "这个方案不在当前讨论范围内，请重新选择",
    )
}

pub async fn authorized_tasks(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
) -> Result<Vec<String>> {
    let Some(context) = crate::focal_chat::load(tx, &task.id).await? else {
        return Ok(vec![task.id.clone()]);
    };
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM agent_tasks WHERE project_id=? ORDER BY rowid DESC LIMIT 200",
    )
    .bind(&task.project_id)
    .fetch_all(&mut **tx)
    .await?;
    let identity = super::model_binding::load(tx, &task.id).await?;
    let mut result = vec![];
    for id in ids {
        if id == task.id {
            result.push(id);
            continue;
        }
        let Some(prior) = crate::focal_chat::load(tx, &id).await? else {
            continue;
        };
        let t = store::load(tx, &id).await?;
        let prior_identity = super::model_binding::load(tx, &id).await?;
        if identity != prior_identity {
            continue;
        }
        if crate::focal_chat::same_recipient(&context, &prior)
            && (task.provider_id == super::PROVIDER_ID
                || super::model_binding::load(tx, &task.id).await?.is_some())
            && t.allowed_photo_ids
                .iter()
                .all(|p| task.allowed_photo_ids.contains(p))
        {
            result.push(id);
        }
    }
    Ok(result)
}
async fn board_allowed(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    board: &Board,
    allowed: &[String],
) -> Result<bool> {
    if board.project_id != task.project_id {
        return Ok(false);
    }
    let creator: Option<String> =
        sqlx::query_scalar("SELECT task_id FROM board_draft_tasks WHERE board_id=?")
            .bind(&board.id)
            .fetch_optional(&mut **tx)
            .await?;
    if let Some(creator) = creator
        && !allowed.contains(&creator)
    {
        return Ok(false);
    }
    let outside: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM canvas_nodes n WHERE n.board_id=? AND n.visible=1 AND n.asset_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM asset_aliases a JOIN photos p ON p.id=a.photo_id WHERE a.asset_id=n.asset_id AND p.project_id=? AND p.id IN(SELECT value FROM json_each(?))))")
        .bind(&board.id).bind(&task.project_id).bind(encode(&task.allowed_photo_ids)?).fetch_one(&mut **tx).await?;
    Ok(!outside)
}
pub async fn parent_snapshot(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    parent: &DraftReference,
) -> Result<Value> {
    let allowed = authorized_tasks(tx, task).await?;
    let value = match parent.kind.as_str() {
        "board" => {
            let board: Board = sqlx::query_as("SELECT * FROM boards WHERE id=? AND project_id=?")
                .bind(&parent.id)
                .bind(&task.project_id)
                .fetch_optional(&mut **tx)
                .await?
                .ok_or_else(denied)?;
            let creator: Option<String> =
                sqlx::query_scalar("SELECT task_id FROM board_draft_tasks WHERE board_id=?")
                    .bind(&parent.id)
                    .fetch_optional(&mut **tx)
                    .await?;
            if let Some(creator) = &creator {
                let ready: String = sqlx::query_scalar("SELECT state FROM agent_tasks WHERE id=?")
                    .bind(creator)
                    .fetch_one(&mut **tx)
                    .await?;
                if !allowed.contains(creator)
                    || creator == &task.id
                    || ["queued", "planning", "running", "needs_input"].contains(&ready.as_str())
                {
                    return Err(denied());
                }
            } else if board.owning_task_id.is_some() || board.state == "frozen" {
                return Err(denied());
            }
            if !board_allowed(tx, task, &board, &allowed).await? {
                return Err(denied());
            }
            if board.revision != parent.revision {
                return Err(AppError::new(
                    "REVISION_CONFLICT",
                    "方案已更新，请重新选择后再修改",
                ));
            }
            let nodes: Vec<CanvasNode> = sqlx::query_as("SELECT * FROM canvas_nodes WHERE board_id=? AND visible=1 ORDER BY z_order,id LIMIT 1001").bind(&parent.id).fetch_all(&mut **tx).await?;
            if nodes.len() > 1000 {
                return Err(AppError::new("VALIDATION_ERROR", "请缩小要继续修改的方案"));
            }
            for node in &nodes {
                crate::canvas::validate_rectangle(
                    node.x,
                    node.y,
                    node.width.max(1.),
                    node.height.max(1.),
                )?;
                if !node.rotation.is_finite() || node.rotation.abs() > 360. {
                    return Err(AppError::new(
                        "VALIDATION_ERROR",
                        "方案包含无效旋转，请先修正",
                    ));
                }
            }
            json!({"board":board,"nodes":nodes,"parentTaskId":creator})
        }
        "selection" => {
            let row = sqlx::query("SELECT task_id,revision,title,summary,state FROM drafts WHERE id=? AND project_id=?")
                .bind(&parent.id).bind(&task.project_id).fetch_optional(&mut **tx).await?.ok_or_else(denied)?;
            let creator: String = row.get("task_id");
            let state: String = sqlx::query_scalar("SELECT state FROM agent_tasks WHERE id=?")
                .bind(&creator)
                .fetch_one(&mut **tx)
                .await?;
            if !allowed.contains(&creator)
                || creator == task.id
                || ["queued", "planning", "running", "needs_input"].contains(&state.as_str())
            {
                return Err(denied());
            }
            if row.get::<i64, _>("revision") != parent.revision {
                return Err(AppError::new(
                    "REVISION_CONFLICT",
                    "方案已更新，请重新选择后再修改",
                ));
            }
            let items = store::items(tx, &parent.id).await?;
            if items
                .iter()
                .any(|i| !task.allowed_photo_ids.contains(&i.photo_id))
            {
                return Err(denied());
            }
            json!({"title":row.get::<String,_>("title"),"summary":row.get::<String,_>("summary"),"state":row.get::<String,_>("state"),"items":items,"parentTaskId":creator})
        }
        _ => return Err(denied()),
    };
    if encode(&value)?.len() > 2 * 1024 * 1024 {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "方案内容过多，请缩小范围",
        ));
    }
    Ok(value)
}
pub async fn bind_continuation(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    parent: &DraftReference,
) -> Result<()> {
    let snapshot = parent_snapshot(tx, task, parent).await?;
    let mut body = json!({"reference":parent,"snapshotSha256":fingerprint(&snapshot),"parentTaskId":snapshot["parentTaskId"]});
    if parent.kind == "board" && snapshot["parentTaskId"].is_string() {
        let feedback = crate::canvas_review::comments::discussion_tx(
            tx,
            &task.project_id,
            &parent.id,
            parent.revision,
            None,
            0,
            5,
        )
        .await?;
        body["reviewCursor"] =
            json!({"draftId":parent.id,"draftRevision":parent.revision,"cursor":feedback.cursor});
        let context = crate::canvas_review::revision_context::freeze(
            tx,
            task,
            parent,
            &snapshot,
            feedback.cursor,
        )
        .await?;
        let value = serde_json::to_value(&context).map_err(serialization)?;
        let raw = encode(&value)?;
        if raw.len() > 2 * 1024 * 1024 {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "修订依据较多，请缩小修改范围",
            ));
        }
        body["reviewContextSha256"] = json!(fingerprint(&value));
        body["reviewContextPreview"] =
            crate::canvas_review::revision_context::preview(&context, 0, 5);
        sqlx::query("INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'creative_review_context',?,?)")
            .bind(&task.id).bind(raw).bind(domain::timestamp()).execute(&mut **tx).await?;
        if feedback.total_messages > 0 {
            body["reviewFeedback"] = json!({"source":"human_editorial_review","visualEvidence":false,"snapshot":feedback});
        }
    } else if parent.kind == "selection" {
        let value = super::selection_revision_context::freeze(tx, task, parent, &snapshot).await?;
        let context: super::selection_revision_context::FrozenSelectionContext =
            serde_json::from_value(value.clone()).map_err(serialization)?;
        body["selectionContextSha256"] = json!(fingerprint(&value));
        body["selectionContextPreview"] =
            super::selection_revision_context::preview(&context, 0, 5);
    }
    sqlx::query("INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'creative_continuation',?,?)")
        .bind(&task.id).bind(encode(&body)?).bind(domain::timestamp()).execute(&mut **tx).await?;
    Ok(())
}
pub async fn continuation(tx: &mut Transaction<'_, Sqlite>, task: &str) -> Result<Value> {
    let raw: Option<String> = sqlx::query_scalar("SELECT body FROM task_messages WHERE task_id=? AND kind='creative_continuation' ORDER BY id LIMIT 1").bind(task).fetch_optional(&mut **tx).await?;
    raw.map(|s| serde_json::from_str(&s).map_err(serialization))
        .transpose()
        .map(|v| v.unwrap_or(Value::Null))
}
pub async fn binding(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<Value> {
    let grant = super::task_scope::load(tx, task).await?;
    if !grant.tools.contains(&"read_creative_context".into()) {
        return Ok(Value::Null);
    }
    crate::canvas_review::revision_context::saved(tx, &task.id).await?;
    super::selection_revision_context::saved(tx, &task.id).await?;
    Ok(
        json!({"version":1,"promptSha256":fingerprint(&json!(PROMPT)),"knowledgeSha256":super::knowledge::digest(),"continuation":continuation(tx,&task.id).await?}),
    )
}
pub async fn lineage(tx: &mut Transaction<'_, Sqlite>, task: &str) -> Result<Vec<Value>> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT body_json FROM agent_steps WHERE task_id=? AND kind='draft_lineage' ORDER BY id",
    )
    .bind(task)
    .fetch_all(&mut **tx)
    .await?;
    rows.into_iter()
        .map(|s| serde_json::from_str(&s).map_err(serialization))
        .collect()
}
/// 知识工具回执里能投影成"参考来源"的实体；不同工具用不同的结果键。
pub(super) fn knowledge_entities(tool: &str, value: &Value) -> Vec<Value> {
    match tool {
        "search_photographers" => value["photographers"]
            .as_array()
            .cloned()
            .unwrap_or_default(),
        "get_photobook_structure" => value["photobooks"].as_array().cloned().unwrap_or_default(),
        "compare_photography_series" => vec![&value["seriesA"], &value["seriesB"]]
            .into_iter()
            .filter(|entity| entity.is_object())
            .cloned()
            .collect(),
        _ => vec![],
    }
}

/// 本任务真正读过的创作记忆：只从每轮 model_attempt 的不可变读取回执和记忆查询回执聚合，
/// 不凭"当时可用"来推断。正文与当前状态按记忆 id 回查，且要求来源项目与回执记的一致；
/// 对不上或这条已被删除就只留 id 与来路，不能把另一行的文字当成这条记忆的当时内容。
pub async fn memory_reads(tx: &mut Transaction<'_, Sqlite>, task: &str) -> Result<Vec<Value>> {
    let rows: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT json_extract(body_json,'$.memoryReads') FROM agent_steps WHERE task_id=? AND kind='model_attempt' AND json_type(body_json,'$.memoryReads')='array' ORDER BY id",
    )
    .bind(task)
    .fetch_all(&mut **tx)
    .await?;
    let mut acc: std::collections::BTreeMap<i64, Value> = std::collections::BTreeMap::new();
    for raw in rows.iter().flatten() {
        let Ok(reads) = serde_json::from_str::<Vec<Value>>(raw) else {
            continue;
        };
        let mut in_round = std::collections::HashSet::new();
        for read in reads {
            let Some(id) = read["memoryId"].as_i64() else {
                continue;
            };
            // 同一轮里同一条只算一次，轮数统计的是"发起过几次带它的请求"。
            if !in_round.insert(id) {
                continue;
            }
            let rounds = acc
                .get(&id)
                .and_then(|prior| prior["rounds"].as_i64())
                .unwrap_or(0)
                + 1;
            acc.insert(id, json!({"memoryId":id,"origin":read["origin"],"scope":read["scope"],"sourceProjectId":read["sourceProjectId"],"statementSha256":read["statementSha256"],"rounds":rounds}));
        }
    }
    // 模型自己发起的记忆查询也会把条目带回上下文，只是不经过逐轮读取回执；
    // 来路与适用范围优先沿用读取回执记下的那份，查询回执只补它没记的字段。
    let receipts: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT json_extract(result_json,'$.result.hypotheses') FROM tool_calls WHERE task_id=? AND tool_name IN('retrieve_preferences','retrieve_memory') AND json_extract(result_json,'$.ok')=1 AND json_type(result_json,'$.result.hypotheses')='array' ORDER BY rowid LIMIT 100",
    )
    .bind(task)
    .fetch_all(&mut **tx)
    .await?;
    for raw in receipts.iter().flatten() {
        let Ok(items) = serde_json::from_str::<Vec<Value>>(raw) else {
            continue;
        };
        let mut in_call = std::collections::HashSet::new();
        for item in items {
            let Some(id) = item["memory_id"].as_i64() else {
                continue;
            };
            // 一次查询里同一条只算一次，次数统计的是"发起过几次带它的查询"。
            if !in_call.insert(id) {
                continue;
            }
            let mut entry = acc
                .get(&id)
                .cloned()
                .unwrap_or_else(|| json!({"rounds": 0}));
            let retrievals = entry["retrievals"].as_i64().unwrap_or(0) + 1;
            for (key, receipt_key) in [
                ("origin", "origin"),
                ("scope", "scope"),
                ("sourceProjectId", "source_project_id"),
            ] {
                if entry[key].is_null() {
                    entry[key] = item[receipt_key].clone();
                }
            }
            if entry["statementSha256"].is_null() {
                entry["statementSha256"] = item["statement"]
                    .as_str()
                    .map(|text| fingerprint(&json!(text)))
                    .into();
            }
            entry["memoryId"] = json!(id);
            entry["retrievals"] = json!(retrievals);
            acc.insert(id, entry);
        }
    }
    let mut list = vec![];
    for (id, mut item) in acc {
        let row: Option<(String, String, String, i64)> = sqlx::query_as(
            "SELECT project_id, statement, status, disabled_for_retrieval FROM preference_hypotheses WHERE id=?",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
        // 正文与"现在还生不生效"用的是同一道信任闸门：来源项目对不上，就一样都不认。
        let current = row.filter(|(project, _, _, _)| {
            Some(project.as_str()) == item["sourceProjectId"].as_str()
        });
        item["kind"] = json!("memory");
        item["label"] = json!("creative_memory");
        let text_version = match (&current, item["statementSha256"].as_str()) {
            (None, _) => "source_unavailable",
            (Some((_, statement, _, _)), Some(recorded))
                if fingerprint(&json!(statement)) == recorded =>
            {
                "verified"
            }
            (Some(_), Some(_)) => "changed",
            (Some(_), None) => "unrecorded",
        };
        item["memoryTextVersion"] = json!(text_version);
        item["passage"] = json!(
            current
                .as_ref()
                .filter(|_| text_version == "verified")
                .map(|(_, statement, _, _)| statement.clone())
        );
        // 指纹仅供本地核对，不增加模型上下文或向界面泄露无用的内部标识。
        item.as_object_mut().unwrap().remove("statementSha256");
        item["memoryStatus"] = json!(current.as_ref().map(|(_, _, status, _)| status.clone()));
        item["memoryDisabled"] = json!(current.as_ref().map(|(_, _, _, disabled)| *disabled != 0));
        item["visualEvidence"] = json!(false);
        list.push(item);
    }
    Ok(list)
}

pub async fn references(tx: &mut Transaction<'_, Sqlite>, task: &str) -> Result<Vec<Value>> {
    let rows:Vec<(String,String)>=sqlx::query_as("SELECT tool_name,result_json FROM tool_calls WHERE task_id=? AND tool_name IN('search_photography_knowledge','search_photographers','get_photobook_structure','compare_photography_series','read_creative_context') ORDER BY rowid LIMIT 100").bind(task).fetch_all(&mut **tx).await?;
    let mut references = std::collections::BTreeMap::new();
    for (tool, raw) in rows {
        let receipt: ToolResult = serde_json::from_str(&raw).map_err(serialization)?;
        if !receipt.ok {
            continue;
        }
        if let Some(value) = receipt.result {
            if tool == "search_photography_knowledge" {
                for item in value["results"].as_array().into_iter().flatten() {
                    // 通用检索会把云端摘要一起带回：那些条目只有实体编号、没有方法编号与正文。
                    // 早先两种都按方法编号入表，云端那几条会挤成同一条无名引用；现在各按自己的编号入表，
                    // 并明说"只有摘要"，读者不会把它当成已取到断言的出处。
                    if item["citationId"].is_string() {
                        references.insert(format!("{}:{}",item["citationId"],item["contentSha256"]),json!({"kind":"knowledge","label":item["title"],"citationId":item["citationId"],"version":item["version"],"passage":item["passage"],"source":item["source"],"license":item["license"],"sha256":item["contentSha256"],"matchReason":item["matchReason"]}));
                    } else if item["id"].is_string() {
                        references.insert(
                            format!("cloud-summary:{}", item["id"]),
                            json!({"kind":"knowledge","label":item["title"],"citationId":item["id"],"reviewStatus":item["reviewStatus"],"summaryOnly":true,"matchReason":item["matchReason"],"visualEvidence":false}),
                        );
                    }
                }
            } else if tool == "read_creative_context" {
                references.insert(format!("{}:{}",value["section"],value["data"]["boardId"]),json!({"kind":"context","label":value["section"],"version":value["data"]["revision"],"section":value["section"],"visualEvidence":false}));
            } else {
                // 云端断言与出处按原样投影：审校状态与存疑标记一并带上，否则"已导入"会在界面上长成"已核实"。
                for entity in knowledge_entities(&tool, &value) {
                    let label = entity["name"]
                        .as_str()
                        .or(entity["title"].as_str())
                        .unwrap_or("");
                    let citation_id = entity["id"].as_str().unwrap_or("");
                    let claims = entity["citations"]["claims"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    if claims.is_empty() {
                        references.insert(format!("cloud:{citation_id}:none"),json!({"kind":"knowledge","label":label,"citationId":citation_id,"reviewStatus":entity["reviewStatus"],"source":entity["citations"]["entitySources"][0]["url"],"detailError":entity["detailError"],"matchReason":entity["matchReason"],"visualEvidence":false}));
                    }
                    for (index, claim) in claims.iter().enumerate() {
                        references.insert(format!("cloud:{citation_id}:{index}:{}",fingerprint(claim)),json!({"kind":"knowledge","label":label,"citationId":citation_id,"passage":claim["text"],"source":claim["sources"][0]["url"],"institution":claim["sources"][0]["institution"],"reviewStatus":claim["reviewStatus"],"disputed":claim["disputed"],"matchReason":entity["matchReason"],"visualEvidence":false}));
                    }
                }
            }
        }
    }
    let mut list: Vec<Value> = references.into_values().collect();
    // 记忆条排在知识条之后：两者都进同一个抽屉，但出处上限优先留给外部知识。
    list.extend(memory_reads(tx, task).await?);
    Ok(list.into_iter().take(40).collect())
}

/// 检索台账：查过哪个词、取到几条、云端答没答上来，逐次留一句话。
/// 参考来源只投影真取到的条目，空手而归的那几次原来在界面上完全看不见；
/// 而"确实没有这个词"、"服务这次没答上来"、"这次检索根本没跑成"是三件事，说法不能混。
pub async fn searches(tx: &mut Transaction<'_, Sqlite>, task: &str) -> Result<Vec<Value>> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT call_id,tool_name,result_json FROM tool_calls WHERE task_id=? AND tool_name IN('search_photography_knowledge','search_photographers','get_photobook_structure','compare_photography_series') ORDER BY rowid LIMIT 100",
    )
    .bind(task)
    .fetch_all(&mut **tx)
    .await?;
    let mut list = Vec::with_capacity(rows.len());
    for (call_id, tool, raw) in rows {
        let receipt: ToolResult = serde_json::from_str(&raw).map_err(serialization)?;
        let value = receipt.result.unwrap_or(Value::Null);
        let returned = if tool == "search_photography_knowledge" {
            value["results"].as_array().map_or(0, Vec::len)
        } else {
            knowledge_entities(&tool, &value).len()
        };
        let available = if tool == "search_photography_knowledge" {
            value["cloudAvailable"].as_bool()
        } else {
            value["available"].as_bool()
        };
        let reported = if tool == "compare_photography_series" {
            // 两次检索只报先出错的那一次：另一次的结果已经按条数计入，不重复说成整次失败。
            [&value["errors"]["seriesA"], &value["errors"]["seriesB"]]
                .into_iter()
                .find(|error| error["code"].is_string())
                .cloned()
        } else if tool == "search_photography_knowledge" {
            Some(value["cloudError"].clone())
        } else {
            Some(value["error"].clone())
        };
        let cloud_error = reported.filter(|error| error["code"].is_string());
        let outcome = if !receipt.ok {
            "failed"
        } else if returned > 0 {
            "found"
        } else if available == Some(false) {
            "unavailable"
        } else {
            "no_match"
        };
        list.push(json!({
            "callId": call_id,
            "tool": tool,
            "query": value["query"].as_str().unwrap_or(""),
            "returned": returned,
            "localMatched": value["localMatched"].as_u64().or(value["matched"].as_u64()),
            "cloudAvailable": available,
            "cloudError": cloud_error,
            "error": receipt.error.map(|e| json!({"code": e.code, "message": e.message})),
            "outcome": outcome,
        }));
    }
    Ok(list)
}

pub async fn execute(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    call: &ToolCall,
) -> Result<Value> {
    if call.name == "fork_creative_draft" {
        return fork(tx, task, &call.args).await;
    }
    let a = &call.args;
    let section = a["section"].as_str().unwrap_or("");
    let object = a["object_id"].as_str().unwrap_or("");
    let limit = a["limit"].as_i64().unwrap_or(10).clamp(1, 30);
    let offset = a["offset"].as_i64().unwrap_or(0);
    let scope = encode(&task.allowed_photo_ids)?;
    let data = match section {
        "visual_overview" => {
            super::visual_overview::read(tx, task, offset as usize, limit as usize).await?
        }
        "boards" => {
            let allowed = authorized_tasks(tx, task).await?;
            let boards: Vec<Board> = sqlx::query_as(
                "SELECT * FROM boards WHERE project_id=? ORDER BY updated_at DESC,id LIMIT 2001",
            )
            .bind(&task.project_id)
            .fetch_all(&mut **tx)
            .await?;
            if boards.len() > 2000 {
                return Err(AppError::new("VALIDATION_ERROR", "请缩小画布目录范围"));
            }
            let mut visible = vec![];
            for b in boards {
                if board_allowed(tx, task, &b, &allowed).await? {
                    visible.push(json!({"id":b.id,"title":b.title,"revision":b.revision,"state":b.state,"taskId":b.owning_task_id}));
                }
            }
            json!({"items":visible.into_iter().skip(offset as usize).take(limit as usize).collect::<Vec<_>>()})
        }
        "board" | "board_history" => {
            let allowed = authorized_tasks(tx, task).await?;
            let board: Board = sqlx::query_as("SELECT * FROM boards WHERE id=? AND project_id=?")
                .bind(object)
                .bind(&task.project_id)
                .fetch_optional(&mut **tx)
                .await?
                .ok_or_else(denied)?;
            if !board_allowed(tx, task, &board, &allowed).await? {
                return Err(denied());
            }
            if section == "board_history" {
                let rows=sqlx::query("SELECT id,op_type,undone,created_at,command_group FROM canvas_op_log WHERE board_id=? ORDER BY rowid DESC LIMIT ? OFFSET ?").bind(object).bind(limit).bind(offset).fetch_all(&mut **tx).await?;
                json!({"boardId":object,"revision":board.revision,"items":rows.iter().map(|r|json!({"id":r.get::<String,_>("id"),"operation":r.get::<String,_>("op_type"),"undone":r.get::<bool,_>("undone"),"group":r.get::<Option<String>,_>("command_group"),"at":r.get::<String,_>("created_at")})).collect::<Vec<_>>()})
            } else {
                let nodes:Vec<CanvasNode>=sqlx::query_as("SELECT * FROM canvas_nodes WHERE board_id=? AND visible=1 ORDER BY z_order,id LIMIT ? OFFSET ?").bind(object).bind(limit).bind(offset).fetch_all(&mut **tx).await?;
                let mut items = vec![];
                for n in nodes {
                    let photo:Option<String>=sqlx::query_scalar("SELECT photo_id FROM asset_aliases WHERE asset_id=? AND photo_id IN(SELECT value FROM json_each(?)) ORDER BY photo_id LIMIT 1").bind(&n.asset_id).bind(&scope).fetch_optional(&mut **tx).await?;
                    items.push(json!({"id":n.id,"type":n.node_type,"photoId":photo,"renditionRef":n.rendition_ref,"x":n.x,"y":n.y,"width":n.width,"height":n.height,"content":n.content_json,"children":n.children_json,"locked":n.locked}));
                }
                let mut value = json!({"boardId":object,"revision":board.revision,"state":board.state,"items":items,"snapshot":"live_read_only; use a new draft to propose changes"});
                let is_draft: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM board_draft_tasks WHERE board_id=?)",
                )
                .bind(object)
                .fetch_one(&mut **tx)
                .await?;
                if is_draft {
                    let bound = continuation(tx, &task.id).await?;
                    let frozen = if bound["reference"]["kind"] == "board"
                        && bound["reference"]["id"] == object
                    {
                        bound["reviewCursor"]["cursor"].as_i64()
                    } else {
                        None
                    };
                    let revision = if frozen.is_some() {
                        bound["reference"]["revision"].as_i64().ok_or_else(denied)?
                    } else {
                        board.revision
                    };
                    let feedback = crate::canvas_review::comments::discussion_tx(
                        tx,
                        &task.project_id,
                        object,
                        revision,
                        frozen,
                        offset as usize,
                        limit as usize,
                    )
                    .await?;
                    if feedback.total_messages > 0 {
                        value["reviewFeedback"] = json!({"source":"human_editorial_review","visualEvidence":false,"frozenForThisTask":frozen.is_some(),"snapshot":feedback});
                    }
                    if frozen.is_some()
                        && let Some(context) =
                            crate::canvas_review::revision_context::saved(tx, &task.id).await?
                    {
                        value["parentReviewContext"] =
                            crate::canvas_review::revision_context::preview(
                                &context,
                                offset as usize,
                                limit as usize,
                            );
                    }
                }
                value
            }
        }
        "history" => {
            let rows=sqlx::query("SELECT id,command_type,project_revision_before,project_revision_after,created_at FROM command_events WHERE project_id=? ORDER BY rowid DESC LIMIT ? OFFSET ?").bind(&task.project_id).bind(limit).bind(offset).fetch_all(&mut **tx).await?;
            json!({"items":rows.iter().map(|r|json!({"id":r.get::<String,_>("id"),"operation":r.get::<String,_>("command_type"),"before":r.get::<i64,_>("project_revision_before"),"after":r.get::<i64,_>("project_revision_after"),"at":r.get::<String,_>("created_at")})).collect::<Vec<_>>(),"notice":"只返回操作来源和版本；不将历史决定当作永久偏好"})
        }
        "versions" | "version" => {
            let rows=sqlx::query("SELECT id,label,based_on_revision,snapshot_json,created_at FROM sequence_versions WHERE project_id=? AND (?='' OR id=?) AND length(snapshot_json)<=2097152 ORDER BY rowid DESC LIMIT ? OFFSET ?").bind(&task.project_id).bind(object).bind(object).bind(if section=="version"{1}else{limit}).bind(if section=="version"{0}else{offset}).fetch_all(&mut **tx).await?;
            let mut items = vec![];
            for r in rows {
                let raw: String = r.get("snapshot_json");
                let snapshot: crate::edit_state::EditState =
                    serde_json::from_str(&raw).map_err(serialization)?;
                let photos: Vec<_> = snapshot
                    .photos
                    .iter()
                    .filter(|p| task.allowed_photo_ids.contains(&p.photo_id))
                    .collect();
                let sequence: Vec<_> = snapshot
                    .sequence
                    .iter()
                    .filter(|id| task.allowed_photo_ids.contains(id))
                    .collect();
                let count = photos.len();
                let mut item = json!({"id":r.get::<String,_>("id"),"label":r.get::<String,_>("label"),"revision":r.get::<i64,_>("based_on_revision"),"at":r.get::<String,_>("created_at"),"scopedCount":sequence.len(),"partial":sequence.len()!=snapshot.sequence.len()||count!=snapshot.photos.len(),"snapshotSha256":fingerprint(&serde_json::to_value(&snapshot).map_err(serialization)?)});
                if section == "version" {
                    item["photoIds"] = json!(sequence);
                    item["photos"] = json!(
                        photos
                            .into_iter()
                            .skip(offset as usize)
                            .take(limit as usize)
                            .collect::<Vec<_>>()
                    );
                    item["photoCount"] = json!(count);
                    item["nextOffset"] = json!(if count > offset as usize + limit as usize {
                        Some(offset + limit)
                    } else {
                        None
                    });
                }
                items.push(item);
            }
            json!({"items":items})
        }
        "variants" => {
            let rows=sqlx::query("SELECT v.id,v.name,v.external_software,v.is_current,r.id AS rendition_id,r.sha256,r.width,r.height,r.bit_depth,r.icc_digest,r.created_at,alias.photo_id FROM edit_variants v JOIN asset_aliases alias ON alias.asset_id=v.asset_id JOIN photos p ON p.id=alias.photo_id AND p.project_id=? LEFT JOIN renditions r ON r.variant_id=v.id WHERE alias.photo_id IN(SELECT value FROM json_each(?)) AND (?='' OR alias.photo_id=?) ORDER BY v.id,r.created_at DESC,r.id LIMIT ? OFFSET ?").bind(&task.project_id).bind(&scope).bind(object).bind(object).bind(limit).bind(offset).fetch_all(&mut **tx).await?;
            json!({"items":rows.iter().map(|r|json!({"variantId":r.get::<String,_>("id"),"photoId":r.get::<String,_>("photo_id"),"name":r.get::<String,_>("name"),"software":r.get::<Option<String>,_>("external_software"),"current":r.get::<bool,_>("is_current"),"renditionId":r.get::<Option<String>,_>("rendition_id"),"sha256":r.get::<Option<String>,_>("sha256"),"width":r.get::<Option<i64>,_>("width"),"height":r.get::<Option<i64>,_>("height"),"bitDepth":r.get::<Option<i64>,_>("bit_depth"),"iccDigest":r.get::<Option<String>,_>("icc_digest")})).collect::<Vec<_>>(),"pixelInspection":false,"notice":"版本元数据，不证明已检查调色像素"})
        }
        "publications" | "publication" => {
            let rows=sqlx::query("SELECT id,title,state,pub_type,spec_json,updated_at FROM publications WHERE project_id=? AND (?='' OR id=?) ORDER BY id LIMIT ? OFFSET ?").bind(&task.project_id).bind(object).bind(object).bind(if section=="publication"{1}else{limit}).bind(if section=="publication"{0}else{offset}).fetch_all(&mut **tx).await?;
            let mut items = vec![];
            for r in rows {
                let id: String = r.get("id");
                let mut item = json!({"id":id,"title":r.get::<String,_>("title"),"state":r.get::<String,_>("state"),"type":r.get::<String,_>("pub_type"),"updatedAt":r.get::<String,_>("updated_at")});
                let spec: String = r.get("spec_json");
                item["spec"] = serde_json::from_str::<Value>(&spec).map_err(serialization)?;
                if section == "publication" {
                    let frames=sqlx::query("SELECT f.id,p.page_number,p.width_mm AS page_width,p.height_mm AS page_height,p.margin_mm,p.bleed_mm,f.x_mm,f.y_mm,f.width_mm,f.height_mm,f.fit_mode,f.crop_zoom,f.rendition_id,a.photo_id FROM publication_frames f JOIN publication_pages p ON p.id=f.page_id JOIN asset_aliases a ON a.asset_id=f.asset_id WHERE p.publication_id=? AND a.photo_id IN(SELECT value FROM json_each(?)) ORDER BY p.page_number,f.id LIMIT ? OFFSET ?").bind(&id).bind(&scope).bind(limit+1).bind(offset).fetch_all(&mut **tx).await?;
                    item["frames"]=json!(frames.iter().take(limit as usize).map(|r|json!({"id":r.get::<String,_>("id"),"page":r.get::<i64,_>("page_number"),"pageWidthMm":r.get::<f64,_>("page_width"),"pageHeightMm":r.get::<f64,_>("page_height"),"marginMm":r.get::<f64,_>("margin_mm"),"bleedMm":r.get::<f64,_>("bleed_mm"),"photoId":r.get::<String,_>("photo_id"),"renditionId":r.get::<Option<String>,_>("rendition_id"),"xMm":r.get::<f64,_>("x_mm"),"yMm":r.get::<f64,_>("y_mm"),"widthMm":r.get::<f64,_>("width_mm"),"heightMm":r.get::<f64,_>("height_mm"),"fit":r.get::<String,_>("fit_mode"),"cropZoom":r.get::<f64,_>("crop_zoom")})).collect::<Vec<_>>());
                    item["nextOffset"] = json!(if frames.len() > limit as usize {
                        Some(offset + limit)
                    } else {
                        None
                    });
                }
                items.push(item);
            }
            json!({"items":items,"notice":"只读版式，不是印刷色彩验收；只展示授权照片的图片框"})
        }
        "tool_receipt" => {
            let raw:Option<String>=sqlx::query_scalar("SELECT result_json FROM tool_calls WHERE task_id=? AND call_id=? AND length(result_json)<=2097152").bind(&task.id).bind(object).fetch_optional(&mut **tx).await?;
            let raw = raw.ok_or_else(|| AppError::new("NOT_FOUND", "此操作记录不在当前会话内"))?;
            let receipt: ToolResult = serde_json::from_str(&raw).map_err(serialization)?;
            let mut result = receipt.result.unwrap_or(Value::Null);
            for key in ["photos", "observations", "nodes", "items"] {
                if let Some(list) = result[key].as_array() {
                    let count = list.len();
                    let page: Vec<_> = list
                        .iter()
                        .skip(offset as usize)
                        .take(limit as usize)
                        .cloned()
                        .collect();
                    result[key] = json!(page);
                    result[format!("{key}Count")] = json!(count);
                }
            }
            json!({"callId":object,"ok":receipt.ok,"result":result,"error":receipt.error,"completeReceiptPreserved":true})
        }
        "selection_draft" => {
            let row=sqlx::query("SELECT task_id,revision,title,summary,base_revision FROM drafts WHERE id=? AND project_id=?").bind(object).bind(&task.project_id).fetch_optional(&mut **tx).await?.ok_or_else(denied)?;
            if !authorized_tasks(tx, task)
                .await?
                .contains(&row.get::<String, _>("task_id"))
            {
                return Err(denied());
            }
            let bound = continuation(tx, &task.id).await?;
            if bound["reference"]["kind"] == "selection"
                && bound["reference"]["id"] == object
                && let Some(context) =
                    super::selection_revision_context::saved(tx, &task.id).await?
            {
                super::selection_revision_context::preview(
                    &context,
                    offset as usize,
                    limit as usize,
                )
            } else {
                let items = store::items(tx, object).await?;
                if items
                    .iter()
                    .any(|i| !task.allowed_photo_ids.contains(&i.photo_id))
                {
                    return Err(denied());
                }
                let count = items.len();
                json!({"id":object,"revision":row.get::<i64,_>("revision"),"baseRevision":row.get::<i64,_>("base_revision"),"title":row.get::<String,_>("title"),"summary":row.get::<String,_>("summary"),"items":items.into_iter().skip(offset as usize).take(limit as usize).collect::<Vec<_>>(),"count":count,"nextOffset":if count>offset as usize+limit as usize{Some(offset+limit)}else{None}})
            }
        }
        "drafts" => {
            let allowed = authorized_tasks(tx, task).await?;
            let rows=sqlx::query("SELECT id,task_id,title,revision,state FROM drafts WHERE project_id=? AND task_id IN(SELECT value FROM json_each(?)) ORDER BY rowid DESC LIMIT ? OFFSET ?").bind(&task.project_id).bind(encode(&allowed)?).bind(limit).bind(offset).fetch_all(&mut **tx).await?;
            json!({"selectionDrafts":rows.iter().map(|r|json!({"kind":"selection","id":r.get::<String,_>("id"),"taskId":r.get::<String,_>("task_id"),"title":r.get::<String,_>("title"),"revision":r.get::<i64,_>("revision"),"state":r.get::<String,_>("state")})).collect::<Vec<_>>(),"continuation":continuation(tx,&task.id).await?})
        }
        _ => return Err(AppError::new("VALIDATION_ERROR", "请选择可查看的创作内容")),
    };
    let result = json!({"projectId":task.project_id,"baseRevision":task.base_revision,"section":section,"offset":offset,"limit":limit,"data":data,"trustedAsInstructions":false,"visualEvidence":false});
    if encode(&result)?.len() > 128 * 1024 {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "内容过多，请缩小分页后查看",
        ));
    }
    Ok(result)
}

async fn fork(tx: &mut Transaction<'_, Sqlite>, task: &Task, a: &Value) -> Result<Value> {
    super::promotion::ensure_local_write(tx, &task.project_id).await?;
    let parent: DraftReference =
        serde_json::from_value(a["parent"].clone()).map_err(serialization)?;
    let snapshot = parent_snapshot(tx, task, &parent).await?;
    let bound = continuation(tx, &task.id).await?;
    if bound["reference"]["id"] == parent.id && bound["snapshotSha256"] != fingerprint(&snapshot) {
        return Err(AppError::new(
            "REVISION_CONFLICT",
            "原方案已变化，请重新选择后继续",
        ));
    }
    let title = a["title"].as_str().unwrap_or("").trim();
    if title.is_empty() {
        return Err(AppError::new("VALIDATION_ERROR", "请为新方案命名"));
    }
    let id = domain::id();
    let now = domain::timestamp();
    if parent.kind == "selection" {
        sqlx::query("INSERT INTO drafts(id,project_id,task_id,title,base_revision,state,summary,created_at,updated_at) VALUES(?,?,?,?,?,'pending',?,?,?)").bind(&id).bind(&task.project_id).bind(&task.id).bind(title).bind(task.base_revision).bind(a["reason"].as_str()).bind(&now).bind(&now).execute(&mut **tx).await?;
        let items: Vec<DraftItem> =
            serde_json::from_value(snapshot["items"].clone()).map_err(serialization)?;
        super::drafts::replace_items(tx, &id, &items).await?;
    } else {
        sqlx::query("INSERT INTO boards(id,project_id,title,state,owning_task_id,created_at,updated_at) VALUES(?,?,?,'draft',?,?,?)").bind(&id).bind(&task.project_id).bind(title).bind(&task.id).bind(&now).bind(&now).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO board_draft_tasks(board_id,task_id,created_at) VALUES(?,?,?)")
            .bind(&id)
            .bind(&task.id)
            .bind(&now)
            .execute(&mut **tx)
            .await?;
        let mut origin:Option<(String,i64,String)>=sqlx::query_as("SELECT source_board_id,source_revision,snapshot_sha256 FROM board_draft_origins WHERE board_id=?").bind(&parent.id).fetch_optional(&mut **tx).await?;
        let formal_parent = snapshot["board"]["owning_task_id"].is_null();
        if formal_parent {
            origin = Some((
                parent.id.clone(),
                parent.revision,
                fingerprint(&json!({"board":snapshot["board"],"nodes":snapshot["nodes"]})),
            ));
        }
        if let Some((source, revision, hash)) = &origin {
            sqlx::query("INSERT INTO board_draft_origins VALUES(?,?,?,?,?,?)")
                .bind(&id)
                .bind(&task.id)
                .bind(source)
                .bind(revision)
                .bind(hash)
                .bind(&now)
                .execute(&mut **tx)
                .await?;
        }
        let nodes: Vec<CanvasNode> =
            serde_json::from_value(snapshot["nodes"].clone()).map_err(serialization)?;
        let ids: HashMap<_, _> = nodes.iter().map(|n| (n.id.clone(), domain::id())).collect();
        for node in nodes {
            let children = node
                .children_json
                .as_ref()
                .map(|raw| -> Result<String> {
                    let old: Vec<String> = serde_json::from_str(raw).map_err(serialization)?;
                    encode(
                        &old.iter()
                            .map(|n| ids.get(n).ok_or_else(denied))
                            .collect::<Result<Vec<_>>>()?,
                    )
                })
                .transpose()?;
            let new = &ids[&node.id];
            sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,asset_id,rendition_ref,content_json,style_json,children_json,locked,visible,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,1,?,?)").bind(new).bind(&id).bind(&node.node_type).bind(node.x).bind(node.y).bind(node.width).bind(node.height).bind(node.rotation).bind(node.z_order).bind(&node.asset_id).bind(&node.rendition_ref).bind(&node.content_json).bind(&node.style_json).bind(children).bind(node.locked).bind(&now).bind(&now).execute(&mut **tx).await?;
            if origin.is_some() {
                let prior:Option<(String,String)>=sqlx::query_as("SELECT source_node_id,base_json FROM board_draft_node_origins WHERE node_id=? AND board_id=?").bind(&node.id).bind(&parent.id).fetch_optional(&mut **tx).await?;
                let prior = if formal_parent {
                    Some((node.id.clone(), encode(&node)?))
                } else {
                    prior
                };
                if let Some((source, base)) = prior {
                    sqlx::query("INSERT INTO board_draft_node_origins VALUES(?,?,?,?)")
                        .bind(new)
                        .bind(&id)
                        .bind(source)
                        .bind(base)
                        .execute(&mut **tx)
                        .await?;
                }
            }
        }
        // 父方案已移除的基线节点也保留为不可见节点，使下一轮仍能准确提出删除建议。
        if origin.is_some() && !formal_parent {
            let removed:Vec<(String,String)>=sqlx::query_as("SELECT o.source_node_id,o.base_json FROM board_draft_node_origins o JOIN canvas_nodes n ON n.id=o.node_id WHERE o.board_id=? AND n.visible=0").bind(&parent.id).fetch_all(&mut **tx).await?;
            for (source, raw) in removed {
                let n: CanvasNode = serde_json::from_str(&raw).map_err(serialization)?;
                let new = domain::id();
                sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,asset_id,rendition_ref,content_json,style_json,children_json,locked,visible,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,0,?,?)").bind(&new).bind(&id).bind(&n.node_type).bind(n.x).bind(n.y).bind(n.width).bind(n.height).bind(n.rotation).bind(n.z_order).bind(&n.asset_id).bind(&n.rendition_ref).bind(&n.content_json).bind(&n.style_json).bind(&n.children_json).bind(n.locked).bind(&now).bind(&now).execute(&mut **tx).await?;
                sqlx::query("INSERT INTO board_draft_node_origins VALUES(?,?,?,?)")
                    .bind(&new)
                    .bind(&id)
                    .bind(source)
                    .bind(raw)
                    .execute(&mut **tx)
                    .await?;
            }
        }
    }
    let lineage = json!({"childId":id,"kind":parent.kind,"parent":parent,"parentTaskId":snapshot["parentTaskId"],"parentSnapshotSha256":fingerprint(&snapshot),"reason":a["reason"],"createdAt":now});
    store::step(
        tx,
        &task.id,
        "draft_lineage",
        lineage,
        "已基于原方案创建独立修订",
    )
    .await?;
    Ok(
        json!({"kind":parent.kind,"board_id":if parent.kind=="board"{json!(id)}else{Value::Null},"draft_id":if parent.kind=="selection"{json!(id)}else{Value::Null},"revision":1,"parent":parent,"originalUnchanged":true}),
    )
}
