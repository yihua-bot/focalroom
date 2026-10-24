//! 集成测试固定剧本，只有 m1-harness 编译；不作为摄影判断或生产 Provider。
use between_frames_lib::{
    agent::{
        context::ModelTurnRequest,
        provider::{MockProvider, ModelProvider},
        types::{ModelOutcome, ToolCall},
    },
    domain::{AppError, Result},
};
use serde_json::{Value, json};
use std::{future::Future, pin::Pin};

pub struct CanvasScenarioProvider;
impl ModelProvider for CanvasScenarioProvider {
    fn generate<'a>(
        &'a self,
        input: &'a ModelTurnRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ModelOutcome>> + Send + 'a>> {
        Box::pin(async move {
            // 独立浏览器会话夹具；固定文字只验证回答与历史显示，不冒充真实模型判断。
            if input.layers["conversation"].is_object() && input.task.goal == "摄影中的节奏是什么？"
            {
                return Ok(ModelOutcome::NoAction {
                    reason: "可以从观看距离与画面密度的变化开始理解。具体照片需要查看后再讨论。"
                        .into(),
                });
            }
            if input.layers["creative"]["continuation"]["reference"].is_object() {
                return followup(input).await;
            }
            if input.task.goal == "这个项目请多留白，给我摄影书编辑的参考方法"
            {
                return preference(input).await;
            }
            // 离线场景按明确的测试目标选固定剧本；生产入口不使用此关键词分流。
            let canvas_scenario = if input.layers["conversation"].is_object() {
                input.task.goal.contains("画布")
                    || input.layers["task"]["canvas"]["source"].is_object()
            } else {
                input.layers["task"]["canvas"].is_object()
            };
            if !canvas_scenario {
                return MockProvider.generate(input).await;
            }
            let records: Vec<_> = input.protocol.iter().filter(|m| m.kind == "tool").collect();
            if records
                .last()
                .is_some_and(|m| m.body["result"]["ok"] == false)
            {
                return Err(AppError::new(
                    "MODEL_UNAVAILABLE",
                    "SIMULATED 画布剧本遇到安全拒绝",
                ));
            }
            let find = |name: &str| {
                records
                    .iter()
                    .find(|m| m.body["name"] == name)
                    .map(|m| m.body["result"]["result"].clone())
            };
            let call = |name: &str, mut args: Value| {
                args["project_id"] = json!(input.task.project_id);
                Ok(ModelOutcome::ToolCalls {
                    calls: vec![ToolCall {
                        call_id: format!(
                            "canvas-{}-{}",
                            input.task.id, input.task.usage.model_turns
                        ),
                        name: name.into(),
                        args,
                    }],
                })
            };
            if records.is_empty() {
                return match input.layers["task"]["canvas"]["source"]["boardId"].as_str() {
                    Some(id) => call("get_board", json!({"board_id":id})),
                    None => call("get_project_context", json!({})),
                };
            }
            let Some(created) = find("create_board_draft") else {
                return call(
                    "create_board_draft",
                    json!({"title":"SIMULATED · 空间编排方案"}),
                );
            };
            let board = created["board_id"].as_str().unwrap();
            let read = records
                .iter()
                .find(|m| {
                    m.body["name"] == "get_board" && m.body["result"]["result"]["board_id"] == board
                })
                .map(|m| m.body["result"]["result"].clone());
            let Some(read) = read else {
                return call("get_board", json!({"board_id":board}));
            };
            let nodes = read["nodes"].as_array().unwrap();
            if find("move_nodes").is_none() && !nodes.is_empty() {
                return call(
                    "move_nodes",
                    json!({"board_id":board,"node_ids":nodes.iter().map(|n|n["id"].clone()).collect::<Vec<_>>(),"dx":80,"dy":40,"rationale":"SIMULATED：固定几何位移，只验证独立草案与审批，不作摄影判断"}),
                );
            }
            if find("add_editorial_note").is_none() {
                return call(
                    "add_editorial_note",
                    json!({"board_id":board,"text":"保留原版，在此比较空间关系","x":360,"y":280,"photo_ids":[],"rationale":"SIMULATED：此便签为操作剧本，不是视觉分析"}),
                );
            }
            if find("create_connector").is_none() && nodes.len() > 1 {
                return call(
                    "create_connector",
                    json!({"board_id":board,"from_node_id":nodes[0]["id"],"to_node_id":nodes[1]["id"],"rationale":"SIMULATED：验证稳定节点连接"}),
                );
            }
            call(
                "finish_canvas_task",
                json!({"task_id":input.task.id,"board_ids":[board],"summary":"SIMULATED：独立画布方案已准备好，原版未变"}),
            )
        })
    }
}

fn scripted_call(input: &ModelTurnRequest, name: &str, mut args: Value) -> Result<ModelOutcome> {
    args["project_id"] = json!(input.task.project_id);
    Ok(ModelOutcome::ToolCalls {
        calls: vec![ToolCall {
            call_id: format!(
                "creative-{}-{}",
                input.task.id, input.task.usage.model_turns
            ),
            name: name.into(),
            args,
        }],
    })
}
async fn followup(input: &ModelTurnRequest) -> Result<ModelOutcome> {
    let records: Vec<_> = input.protocol.iter().filter(|m| m.kind == "tool").collect();
    if records.iter().any(|m| m.body["result"]["ok"] == false) {
        return Err(AppError::new(
            "MODEL_UNAVAILABLE",
            "SIMULATED 修订剧本遇到拒绝",
        ));
    }
    let find = |name: &str| {
        records
            .iter()
            .find(|m| m.body["name"] == name)
            .map(|m| m.body["result"]["result"].clone())
    };
    let review = &input.layers["creative"]["continuation"]["reviewFeedback"]["snapshot"];
    if review["totalMessages"].as_u64().unwrap_or(0) > 0 && find("read_creative_context").is_none()
    {
        return scripted_call(
            input,
            "read_creative_context",
            json!({"section":"board","object_id":input.layers["creative"]["continuation"]["reference"]["id"],"offset":0,"limit":5}),
        );
    }
    let Some(fork) = find("fork_creative_draft") else {
        return scripted_call(
            input,
            "fork_creative_draft",
            json!({"parent":input.layers["creative"]["continuation"]["reference"],"title":"SIMULATED · 修订方案","reason":"根据后续说明创建新分支"}),
        );
    };
    let board = fork["board_id"]
        .as_str()
        .ok_or_else(|| AppError::new("VALIDATION_ERROR", "该浏览器剧本只覆盖画布修订"))?;
    if find("get_board").is_none() {
        return scripted_call(input, "get_board", json!({"board_id":board}));
    }
    if find("add_editorial_note").is_none() {
        let text = find("read_creative_context")
            .and_then(|result| {
                result["data"]["reviewFeedback"]["snapshot"]["messages"]
                    .as_array()
                    .and_then(|messages| messages.last())
                    .and_then(|message| message["body"].as_str())
                    .map(|body| format!("SIMULATED · 收到的审稿意见：{body}"))
            })
            .unwrap_or_else(|| "这一版增加观看中的停顿".into());
        return scripted_call(
            input,
            "add_editorial_note",
            json!({"board_id":board,"text":text,"x":620,"y":100,"photo_ids":[],"rationale":"SIMULATED：验证跨轮独立修改"}),
        );
    }
    scripted_call(
        input,
        "finish_canvas_task",
        json!({"task_id":input.task.id,"board_ids":[board],"summary":"新的修订方案已准备好，原方案保留"}),
    )
}
async fn preference(input: &ModelTurnRequest) -> Result<ModelOutcome> {
    let records: Vec<_> = input.protocol.iter().filter(|m| m.kind == "tool").collect();
    if records.iter().any(|m| m.body["result"]["ok"] == false) {
        return Err(AppError::new(
            "MODEL_UNAVAILABLE",
            "SIMULATED 偏好剧本遇到拒绝",
        ));
    }
    let has = |name: &str| records.iter().any(|m| m.body["name"] == name);
    if !has("read_creative_context") {
        return scripted_call(
            input,
            "read_creative_context",
            json!({"section":"history","object_id":null,"offset":0,"limit":10}),
        );
    }
    if !has("search_photography_knowledge") {
        return scripted_call(
            input,
            "search_photography_knowledge",
            json!({"query":"摄影书 photobook","limit":1}),
        );
    }
    if !has("propose_preference") {
        let message = input.layers["conversation"]["current_turns"]
            .as_array()
            .and_then(|m| m.iter().find(|v| v["role"] == "user"))
            .ok_or_else(|| AppError::new("VALIDATION_ERROR", "缺少用户原话"))?;
        return scripted_call(
            input,
            "propose_preference",
            json!({"statement":"本项目的编排保留更多留白","applicability":"仅用于这个项目的叙事编排","message_ids":[message["id"]],"photo_ids":[]}),
        );
    }
    Ok(ModelOutcome::NoAction {
        reason: "我找到了摄影书编辑的方法札记，并准备了一条偏好建议，请你确认是否保存。".into(),
    })
}
