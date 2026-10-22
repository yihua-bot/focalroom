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
            if !input.layers["task"]["canvas"].is_object() {
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
