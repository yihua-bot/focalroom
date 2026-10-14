use super::{context::ModelTurnRequest, types::*};
use crate::domain::{AppError, Result};
use serde_json::{Value, json};
use std::{collections::HashSet, future::Future, pin::Pin};

pub trait ModelProvider: Send + Sync {
    fn generate<'a>(
        &'a self,
        input: &'a ModelTurnRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelOutcome>> + Send + 'a>>;
}
pub struct MockProvider;
impl ModelProvider for MockProvider {
    fn generate<'a>(
        &'a self,
        input: &'a ModelTurnRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelOutcome>> + Send + 'a>> {
        Box::pin(async move {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            decide(input)
        })
    }
}
fn decide(input: &ModelTurnRequest) -> Result<ModelOutcome> {
    let t = &input.task;
    let call = |name: &str, args: Value| {
        Ok(ModelOutcome::ToolCalls {
            calls: vec![ToolCall {
                call_id: format!("{}-{}", t.id, t.usage.model_turns + 1),
                name: name.into(),
                args,
            }],
        })
    };
    let records: Vec<(&str, Value)> = input
        .protocol
        .iter()
        .filter(|m| m.kind == "tool")
        .filter_map(|m| {
            let name = m.body["name"].as_str()?;
            let result = m.body["result"]["result"].clone();
            Some((name, result))
        })
        .collect();
    if input
        .protocol
        .iter()
        .rev()
        .find(|m| m.kind == "tool")
        .is_some_and(|m| m.body["result"]["ok"] == false)
    {
        return Err(AppError::new(
            "MODEL_UNAVAILABLE",
            "Mock 工具调用失败；请查看实际错误并重新委托",
        ));
    }
    if records.is_empty() {
        return call("get_project_context", json!({"project_id":t.project_id}));
    }
    let context = &records[0].1;
    let reorder = !t.selected_photo_ids.is_empty()
        && (t.goal.contains("排序") || t.goal.contains("重排") || t.goal.contains("节奏"));
    if t.goal.contains("先询问")
        && t.answer.is_none()
        && !records.iter().any(|(n, _)| *n == "request_feedback")
    {
        return call(
            "request_feedback",
            json!({"project_id":t.project_id,"question":"这次优先顺序还是扩大候选？","options":["只调整顺序","给出不同候选"]}),
        );
    }
    let last = records.last().unwrap();
    if last.0 == "get_project_context" || last.0 == "request_feedback" {
        if reorder {
            return call("get_sequence", json!({"project_id":t.project_id}));
        }
        if context["high_rated_count"].as_u64().unwrap_or(0) >= u64::from(t.target_count) {
            return call(
                "search_photos",
                json!({"project_id":t.project_id,"query":"rating:4","limit":100}),
            );
        }
        return call(
            "list_photos",
            json!({"project_id":t.project_id,"status":null,"limit":100,"cursor":null}),
        );
    }
    let pool: Vec<String> = records
        .iter()
        .find(|(n, _)| ["list_photos", "search_photos", "get_sequence"].contains(n))
        .map(|(_, r)| {
            if let Some(ids) = r["photo_ids"].as_array() {
                ids.iter()
                    .filter_map(|id| id.as_str().map(String::from))
                    .collect()
            } else {
                r["photos"]
                    .as_array()
                    .map(|p| {
                        p.iter()
                            .filter(|p| p["status"] != "reject")
                            .filter_map(|p| p["photo_id"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default()
            }
        })
        .unwrap_or_default();
    if pool.is_empty() {
        return Ok(ModelOutcome::NoAction {
            reason: "授权范围内没有可用候选，请人工补充或修改范围".into(),
        });
    }
    let created: Vec<&Value> = records
        .iter()
        .filter(|(n, _)| *n == "create_selection_draft")
        .map(|(_, r)| r)
        .collect();
    let count = (t.target_count as usize).min(pool.len());
    if created.is_empty() || last.0 == "evaluate_draft" {
        if last.0 == "evaluate_draft" {
            let duplicate_groups = last.1["duplicate_groups"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            if !duplicate_groups.is_empty() {
                let excluded: HashSet<String> = duplicate_groups
                    .iter()
                    .filter_map(Value::as_array)
                    .flat_map(|g| g.iter().skip(1))
                    .filter_map(|i| i.as_str().map(String::from))
                    .collect();
                let ids: Vec<_> = pool
                    .iter()
                    .filter(|id| !excluded.contains(*id))
                    .take(count)
                    .cloned()
                    .collect();
                let draft_id = last.1["draft_id"].as_str().unwrap();
                let revision = records
                    .iter()
                    .rev()
                    .find(|(n, r)| {
                        (*n == "create_selection_draft" || *n == "update_draft_sequence")
                            && r["draft_id"] == draft_id
                    })
                    .map(|(_, r)| r["draft_revision"].as_i64().unwrap_or(1))
                    .unwrap_or(1);
                return call(
                    "update_draft_sequence",
                    json!({"project_id":t.project_id,"draft_id":draft_id,"expected_draft_revision":revision,"ordered_photo_ids":ids}),
                );
            }
            if created.len() >= 2 {
                return call(
                    "finish_task",
                    json!({"project_id":t.project_id,"task_id":t.id,"draft_ids":created.iter().map(|r|r["draft_id"].clone()).collect::<Vec<_>>(),"summary":"两份本地 Mock 草案已核验；仅供交互审核，不代表视觉或审美理解"}),
                );
            }
        }
        let mut ids: Vec<_> = pool.iter().take(count).cloned().collect();
        if !created.is_empty() {
            ids.reverse();
        }
        let items:Vec<_>=ids.iter().enumerate().map(|(pos,id)|json!({"photo_id":id,"proposed_action":"keep","position":pos,"rationale":"Mock：依据授权候选与人工元数据；叙事作用未做视觉验证"})).collect();
        return call(
            "create_selection_draft",
            json!({"project_id":t.project_id,"base_project_revision":t.base_revision,"title":if created.is_empty(){"方案 A · 人工线索"}else{"方案 B · 反向节奏"},"items":items,"summary":"本地 Mock 编辑练习；保留人工最终决定"}),
        );
    }
    if last.0 == "create_selection_draft" || last.0 == "update_draft_sequence" {
        return call(
            "evaluate_draft",
            json!({"project_id":t.project_id,"draft_id":last.1["draft_id"]}),
        );
    }
    Err(AppError::new(
        "MODEL_UNAVAILABLE",
        "Mock 未能根据当前观察继续；没有固定流程回退",
    ))
}
