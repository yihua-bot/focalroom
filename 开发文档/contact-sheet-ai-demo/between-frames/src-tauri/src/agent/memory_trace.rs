//! 按请求尝试和查询回执展示记忆来路；不推断模型是否采纳，不回填历史缺失字段。
use super::types::Task;
use crate::{
    command_audit::{encode, fingerprint},
    domain::Result,
};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};
use std::collections::HashMap;

/// 与当次读取同事务冻结人工确认的回执身份；不复制偏好或原始证据正文。
pub async fn capture(tx: &mut Transaction<'_, Sqlite>, hypotheses: &Value) -> Result<Value> {
    let mut reads = super::memory::read_trace(hypotheses);
    for read in reads.as_array_mut().unwrap() {
        read["confirmation"] = Value::Null;
        if read["statusAtRead"] != "confirmed" {
            continue;
        }
        let project = read["sourceProjectId"].as_str().unwrap_or("");
        let id = read["memoryId"].as_i64().unwrap_or(-1);
        let refs = evidence_ids(read);
        let row: Option<(String, String, String)> = sqlx::query_as(
            "SELECT request_id,created_at,json_extract(command_json,'$.type') FROM memory_commands c WHERE project_id=? AND changed=1 AND ((json_extract(command_json,'$.type')='ConfirmHypothesis' AND json_extract(command_json,'$.hypothesisId')=?) OR (json_extract(command_json,'$.type')='RememberPreference' AND EXISTS(SELECT 1 FROM preference_evidence e WHERE e.project_id=c.project_id AND e.id IN(SELECT value FROM json_each(?)) AND json_extract(e.detail_json,'$.hypothesisId')=? AND json_extract(e.detail_json,'$.explicitPreference')=1 AND json_extract(e.detail_json,'$.taskId')=json_extract(c.command_json,'$.taskId') AND json_extract(e.detail_json,'$.proposalId')=json_extract(c.command_json,'$.proposalId')))) ORDER BY rowid DESC LIMIT 1"
        ).bind(project).bind(id).bind(encode(&refs)?).bind(id).fetch_optional(&mut **tx).await?;
        if let Some((request, time, action)) = row {
            read["confirmation"] = json!({"requestId":request,"createdAt":time,"action":action});
        }
    }
    Ok(reads)
}

fn evidence_ids(read: &Value) -> Vec<i64> {
    let mut ids: Vec<i64> = if let Some(ids) = read["evidenceIds"].as_array() {
        ids.iter()
            .take(100)
            .filter_map(Value::as_i64)
            .filter(|id| *id >= 0)
            .collect()
    } else {
        read["evidenceRefs"]
            .as_array()
            .into_iter()
            .flatten()
            .take(100)
            .filter(|e| e["project_id"] == read["sourceProjectId"])
            .filter_map(|e| e["evidence_id"].as_i64())
            .collect()
    };
    ids.sort_unstable();
    ids.dedup();
    ids
}

async fn current(tx: &mut Transaction<'_, Sqlite>, read: &Value) -> Result<Value> {
    let row: Option<(String,String,String,i64,String)> = sqlx::query_as("SELECT h.project_id,h.statement,h.status,h.disabled_for_retrieval,p.title FROM preference_hypotheses h JOIN projects p ON p.id=h.project_id WHERE h.id=? AND p.deleted_at IS NULL")
        .bind(read["memoryId"].as_i64().unwrap_or(-1)).fetch_optional(&mut **tx).await?;
    let row = row.filter(|(project, _, _, _, _)| read["sourceProjectId"] == *project);
    let version = match (&row, read["statementSha256"].as_str()) {
        (None, _) => "source_unavailable",
        (Some((_, text, _, _, _)), Some(hash)) if fingerprint(&json!(text)) == hash => "verified",
        (Some(_), Some(_)) => "changed",
        _ => "unrecorded",
    };
    Ok(
        json!({"memoryTextVersion":version,"passage":row.as_ref().filter(|_|version=="verified").map(|(_,text,_,_,_)|text),"currentStatus":row.as_ref().map(|(_,_,state,_,_)|state),"disabled":row.as_ref().map(|(_,_,_,disabled,_)|*disabled!=0),"sourceProjectTitle":row.as_ref().map(|(_,_,_,_,title)|title)}),
    )
}

async fn resolve(tx: &mut Transaction<'_, Sqlite>, task: &Task, read: &Value) -> Result<Value> {
    let snapshot = current(tx, read).await?;
    let mut result = json!({"memoryId":read["memoryId"],"sourceProjectId":read["sourceProjectId"],"origin":read["origin"],"scope":read["scope"],"statusAtRead":read["statusAtRead"],"selectionReason":read["selectionReason"],"confirmation":read["confirmation"],"evidence":null,"evidenceRefsTruncated":read["evidenceRefsTruncated"].as_bool().unwrap_or(false),"applicability":null});
    result
        .as_object_mut()
        .unwrap()
        .extend(snapshot.as_object().unwrap().clone());
    if (!read["evidenceIds"].is_array() && !read["evidenceRefs"].is_array())
        || result["memoryTextVersion"] == "source_unavailable"
    {
        return Ok(result);
    }
    let ids = evidence_ids(read);
    let rows: Vec<(String,i64)> = sqlx::query_as("SELECT e.detail_json,COALESCE(s.excluded,0) FROM preference_evidence e LEFT JOIN memory_evidence_state s ON s.evidence_id=e.id WHERE e.project_id=? AND e.id IN(SELECT value FROM json_each(?))")
        .bind(read["sourceProjectId"].as_str()).bind(encode(&ids)?).fetch_all(&mut **tx).await?;
    let excluded = rows.iter().filter(|(_, excluded)| *excluded != 0).count();
    result["evidence"] = json!({"recorded":ids.len(),"available":rows.len()-excluded,"excluded":excluded,"missing":ids.len()-rows.len()});
    // 跨项目授权只展示来路和数量，不扩展为读取来源照片、备注或适用说明正文。
    if read["sourceProjectId"] == task.project_id
        && read["scope"] == "project_local"
        && result["memoryTextVersion"] == "verified"
    {
        result["applicability"] = rows
            .iter()
            .filter_map(|(raw, _)| serde_json::from_str::<Value>(raw).ok())
            .find(|e| e["explicitPreference"] == true && e["hypothesisId"] == read["memoryId"])
            .and_then(|e| {
                e["applicability"]
                    .as_str()
                    .map(|s| s.chars().take(500).collect::<String>())
            })
            .into();
    }
    Ok(result)
}

pub async fn projection(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<Value> {
    let rows:Vec<(i64,String,Option<String>,String)>=sqlx::query_as(
        "SELECT s.id,s.kind,CASE WHEN length(CAST(s.body_json AS BLOB))<=65536 THEN s.body_json END,s.created_at FROM agent_steps s WHERE s.task_id=? AND (s.kind IN('model_attempt','memory_query_trace') OR (s.kind='tool' AND json_extract(s.body_json,'$.name') IN('retrieve_preferences','retrieve_memory') AND NOT EXISTS(SELECT 1 FROM agent_steps q WHERE q.task_id=s.task_id AND q.kind='memory_query_trace' AND json_extract(q.body_json,'$.callId')=json_extract(s.body_json,'$.result.callId')))) ORDER BY s.id DESC LIMIT 41"
    ).bind(&task.id).fetch_all(&mut **tx).await?;
    let has_earlier = rows.len() > 40;
    let mut events = vec![];
    let mut cached: HashMap<String, Value> = HashMap::new();
    for (step, kind, raw, time) in rows
        .into_iter()
        .take(40)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let body = raw
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .unwrap_or(Value::Null);
        let legacy = kind == "tool";
        let reads = if legacy {
            if body["result"]["ok"] == true {
                super::memory::read_trace(&body["result"]["result"]["hypotheses"])
            } else {
                Value::Null
            }
        } else {
            body["memoryReads"].clone()
        };
        let recorded = if legacy {
            body["result"]["ok"] == true && body["result"]["result"]["hypotheses"].is_array()
        } else {
            reads.is_array()
        };
        let mut entries = vec![];
        let mut seen = std::collections::HashSet::new();
        for read in reads.as_array().into_iter().flatten().take(10) {
            if let Some(id) = read["memoryId"].as_i64().filter(|id| *id >= 0)
                && seen.insert(id)
            {
                let key = encode(read)?;
                let resolved = if let Some(value) = cached.get(&key) {
                    value.clone()
                } else {
                    let value = resolve(tx, task, read).await?;
                    cached.insert(key, value.clone());
                    value
                };
                entries.push(resolved);
            }
        }
        events.push(json!({"id":step,"kind":if kind=="model_attempt" {"context_attempt"} else {"query_result"},"round":body["round"].as_u64().filter(|n|*n>0),"createdAt":time,"callId":if legacy {body["result"]["callId"].clone()} else {body["callId"].clone()},"recorded":recorded,"reads":entries}));
    }
    Ok(json!({"events":events,"hasEarlierEvents":has_earlier}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captured_reference_ids_stay_bounded_without_copying_memory_text() {
        let project = "p".repeat(160);
        let hypotheses:Vec<Value>=(0..11).map(|id|json!({"memory_id":id,"source_project_id":project,"scope":"project_local","origin":"user_confirmed","status":"confirmed","statement":"不应复制的偏好原文","evidence_refs":(0..110).map(|id|json!({"project_id":project,"evidence_id":id})).collect::<Vec<_>>()})).collect();
        let trace = super::super::memory::read_trace(&json!(hypotheses));
        assert_eq!(trace.as_array().unwrap().len(), 10);
        assert_eq!(trace[0]["evidenceIds"].as_array().unwrap().len(), 100);
        assert_eq!(trace[0]["evidenceRefsTruncated"], true);
        let bytes = trace.to_string();
        assert!(bytes.len() < 65536);
        assert!(!bytes.contains("不应复制的偏好原文"));
    }
}
