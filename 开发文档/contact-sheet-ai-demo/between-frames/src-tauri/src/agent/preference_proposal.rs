//! 模型只提建议；人工确认经记忆命令形成可撤回证据，不从模型回执自动学习。
use super::types::Task;
use crate::{
    command_audit::{encode, serialization},
    domain::{self, AppError, Result},
};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};

pub async fn propose(tx: &mut Transaction<'_, Sqlite>, task: &Task, a: &Value) -> Result<Value> {
    let context = crate::focal_chat::load(tx, &task.id)
        .await?
        .ok_or_else(|| AppError::new("PERMISSION_DENIED", "请在当前对话中提出记忆建议"))?;
    let ids: Vec<i64> = serde_json::from_value(a["message_ids"].clone()).map_err(serialization)?;
    let mut sources = vec![];
    for id in ids {
        let row=sqlx::query("SELECT id,body,created_at FROM task_messages WHERE id=? AND task_id=? AND kind IN('user_goal','clarification_answer')")
            .bind(id).bind(&task.id).fetch_optional(&mut **tx).await?.ok_or_else(||AppError::new("PERMISSION_DENIED","记忆建议需要引用当前用户的说明"))?;
        sources.push(json!({"messageId":row.get::<i64,_>("id"),"text":row.get::<String,_>("body"),"createdAt":row.get::<String,_>("created_at")}));
    }
    let photos: Vec<String> =
        serde_json::from_value(a["photo_ids"].clone()).map_err(serialization)?;
    if photos.iter().any(|id| !task.allowed_photo_ids.contains(id)) {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "记忆证据照片超出本次范围",
        ));
    }
    let body = json!({"id":domain::id(),"statement":a["statement"],"applicability":a["applicability"],"photoIds":photos,"sources":sources,"owner":crate::focal_chat::owner(&context),"createdAt":domain::timestamp()});
    super::store::step(
        tx,
        &task.id,
        "preference_proposal",
        body.clone(),
        "有一条偏好建议等待确认",
    )
    .await?;
    Ok(json!({"proposalId":body["id"],"requiresHumanConfirmation":true,"savedAsMemory":false}))
}
pub async fn projection(tx: &mut Transaction<'_, Sqlite>, task: &str) -> Result<Vec<Value>> {
    let raw:Vec<String>=sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='preference_proposal' ORDER BY id LIMIT 20").bind(task).fetch_all(&mut **tx).await?;
    let mut result = vec![];
    for raw in raw {
        let mut v: Value = serde_json::from_str(&raw).map_err(serialization)?;
        let key = format!("preference:{task}:{}", v["id"].as_str().unwrap_or(""));
        let evidence: Option<i64> =
            sqlx::query_scalar("SELECT id FROM preference_evidence WHERE dedup_key=?")
                .bind(&key)
                .fetch_optional(&mut **tx)
                .await?;
        let dismissed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memory_commands WHERE json_extract(command_json,'$.type')='DismissPreferenceProposal' AND json_extract(command_json,'$.taskId')=? AND json_extract(command_json,'$.proposalId')=?)").bind(task).bind(v["id"].as_str()).fetch_one(&mut **tx).await?;
        let confirmed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memory_commands WHERE json_extract(command_json,'$.type')='RememberPreference' AND json_extract(command_json,'$.taskId')=? AND json_extract(command_json,'$.proposalId')=?)").bind(task).bind(v["id"].as_str()).fetch_one(&mut **tx).await?;
        if confirmed {
            let saved:Option<String>=sqlx::query_scalar("SELECT json_extract(command_json,'$.statement') FROM memory_commands WHERE changed=1 AND json_extract(command_json,'$.type')='RememberPreference' AND json_extract(command_json,'$.taskId')=? AND json_extract(command_json,'$.proposalId')=? ORDER BY rowid LIMIT 1").bind(task).bind(v["id"].as_str()).fetch_optional(&mut **tx).await?;
            if let Some(saved) = saved {
                v["statement"] = json!(saved);
            }
        }
        let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM preference_hypotheses h JOIN json_each(h.evidence_ids_json) ref ON ref.value=? WHERE h.disabled_for_retrieval=0 AND h.status='confirmed')").bind(evidence).fetch_one(&mut **tx).await?;
        v["state"] = json!(if active {
            "saved"
        } else if confirmed {
            "removed"
        } else if dismissed {
            "dismissed"
        } else {
            "pending"
        });
        v.as_object_mut().unwrap().remove("owner");
        result.push(v);
    }
    Ok(result)
}
pub async fn confirm(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    task_id: &str,
    proposal: &str,
    statement: Option<&str>,
    expected: Option<&str>,
) -> Result<bool> {
    let task = super::store::load(tx, task_id).await?;
    if task.project_id != project {
        return Err(AppError::new("PERMISSION_DENIED", "记忆建议不属于本项目"));
    }
    let context = crate::focal_chat::load(tx, task_id)
        .await?
        .ok_or_else(|| AppError::new("PERMISSION_DENIED", "对话来源不可用"))?;
    if crate::focal_chat::owner(&context) != expected {
        return Err(AppError::new(
            "AUTH_REQUIRED",
            "账户已改变，请重新打开这条建议",
        ));
    }
    let raw:Option<String>=sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='preference_proposal' AND json_extract(body_json,'$.id')=? ORDER BY id LIMIT 1").bind(task_id).bind(proposal).fetch_optional(&mut **tx).await?;
    let raw = raw.ok_or_else(|| AppError::new("NOT_FOUND", "记忆建议不存在"))?;
    let body: Value = serde_json::from_str(&raw).map_err(serialization)?;
    let Some(statement) = statement else {
        return Ok(false);
    };
    let statement = statement.trim();
    if statement.is_empty() || statement.chars().count() > 500 {
        return Err(AppError::new("VALIDATION_ERROR", "偏好说明须为 1–500 字符"));
    }
    let key = format!("preference:{task_id}:{proposal}");
    let consumed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memory_commands WHERE project_id=? AND json_extract(command_json,'$.type')='RememberPreference' AND json_extract(command_json,'$.taskId')=? AND json_extract(command_json,'$.proposalId')=?)").bind(project).bind(task_id).bind(proposal).fetch_one(&mut **tx).await?;
    if consumed {
        return Ok(false);
    }
    let hypothesis: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(id),0)+1 FROM preference_hypotheses")
            .fetch_one(&mut **tx)
            .await?;
    let detail = json!({"hypothesisId":hypothesis,"note":statement,"explicitPreference":true,"taskId":task_id,"proposalId":proposal,"applicability":body["applicability"],"sources":body["sources"],"photoIds":body["photoIds"],"owner":expected,"scope":"project_local"});
    let now = domain::timestamp();
    let evidence=sqlx::query("INSERT INTO preference_evidence(project_id,kind,detail_json,dedup_key,created_at) VALUES(?,'note',?,?,?)").bind(project).bind(encode(&detail)?).bind(&key).bind(&now).execute(&mut **tx).await?.last_insert_rowid();
    sqlx::query("INSERT INTO preference_hypotheses(id,project_id,scope,statement,evidence_ids_json,status,confidence,created_at,updated_at) VALUES(?,?,'project_local',?,?,'confirmed','tentative',?,?)").bind(hypothesis).bind(project).bind(statement).bind(encode(&vec![evidence])?).bind(&now).bind(&now).execute(&mut **tx).await?;
    Ok(true)
}
/// 只能将同账户的显式偏好发送给当前模型；旧人工编辑证据保留项目语义。
pub async fn filter_for_task(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    memory: &mut Value,
) -> Result<()> {
    let owner = super::model_binding::load(tx, &task.id)
        .await?
        .and_then(|i| i.account_id);
    let mut usable = vec![];
    for h in memory["hypotheses"].as_array().into_iter().flatten() {
        let ids = h["evidence_ids"].clone();
        let mismatch:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM preference_evidence WHERE id IN(SELECT value FROM json_each(?)) AND json_extract(detail_json,'$.explicitPreference')=1 AND (json_extract(detail_json,'$.owner') IS NOT ? OR EXISTS(SELECT 1 FROM json_each(json_extract(detail_json,'$.photoIds')) photo WHERE photo.value NOT IN(SELECT value FROM json_each(?)))))").bind(encode(&ids)?).bind(&owner).bind(encode(&task.allowed_photo_ids)?).fetch_one(&mut **tx).await?;
        if !mismatch {
            usable.push(h.clone());
        }
    }
    memory["hypotheses"] = json!(usable);
    Ok(())
}
