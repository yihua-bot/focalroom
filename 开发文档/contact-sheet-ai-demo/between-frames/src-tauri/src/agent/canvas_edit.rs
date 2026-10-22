//! 仅修改任务沙盒；正式作品的历史/偏好证据不会由这些工具产生。
use super::{tools::require_task_board, types::Task};
use crate::{
    canvas::validate_rectangle,
    domain::{self, AppError, Result},
};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};
use std::collections::HashSet;

pub(super) async fn execute(
    tx: &mut Transaction<'_, Sqlite>,
    name: &str,
    a: &Value,
    task: &Task,
) -> Result<Value> {
    super::task_scope::require_tool(tx, task, name).await?;
    let board = a["board_id"].as_str().unwrap_or("");
    require_task_board(tx, board, task).await?;
    let now = domain::timestamp();
    if matches!(name, "move_nodes" | "remove_nodes") {
        let ids: Vec<String> = serde_json::from_value(a["node_ids"].clone())
            .map_err(crate::command_audit::serialization)?;
        if ids.is_empty()
            || ids.len() > 300
            || ids.iter().collect::<HashSet<_>>().len() != ids.len()
        {
            return Err(AppError::new("VALIDATION_ERROR", "请选择不同的有效节点"));
        }
        let dx = a["dx"].as_f64().unwrap_or(0.);
        let dy = a["dy"].as_f64().unwrap_or(0.);
        for id in &ids {
            let existing:Option<(f64,f64,f64,f64)>=sqlx::query_as("SELECT x,y,width,height FROM canvas_nodes WHERE id=? AND board_id=? AND visible=1 AND locked=0")
                .bind(id).bind(board).fetch_optional(&mut **tx).await?;
            let (x, y, w, h) = existing.ok_or_else(|| {
                AppError::new("PERMISSION_DENIED", "只能编辑当前任务草案中未锁定的节点")
            })?;
            if name == "move_nodes" {
                // 连接与笔迹可能为零宽/高；位置仍须有限且有界。
                validate_rectangle(x + dx, y + dy, w.max(1.), h.max(1.))?;
            }
        }
        for id in &ids {
            if name == "move_nodes" {
                sqlx::query(
                    "UPDATE canvas_nodes SET x=x+?,y=y+?,updated_at=? WHERE id=? AND board_id=?",
                )
                .bind(dx)
                .bind(dy)
                .bind(&now)
                .bind(id)
                .bind(board)
                .execute(&mut **tx)
                .await?;
            } else {
                sqlx::query(
                    "UPDATE canvas_nodes SET visible=0,updated_at=? WHERE id=? AND board_id=?",
                )
                .bind(&now)
                .bind(id)
                .bind(board)
                .execute(&mut **tx)
                .await?;
            }
        }
        return Ok(json!({"board_id":board,"changed_nodes":ids,"rationale":a["rationale"]}));
    }
    let id = domain::id();
    let z: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(z_order),0)+1 FROM canvas_nodes WHERE board_id=?")
            .bind(board)
            .fetch_one(&mut **tx)
            .await?;
    match name {
        "create_connector" => {
            let from = a["from_node_id"].as_str().unwrap_or("");
            let to = a["to_node_id"].as_str().unwrap_or("");
            if from == to {
                return Err(AppError::new("VALIDATION_ERROR", "请选择两个不同节点"));
            }
            for node in [from, to] {
                let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM canvas_nodes WHERE id=? AND board_id=? AND visible=1)").bind(node).bind(board).fetch_one(&mut **tx).await?;
                if !valid {
                    return Err(AppError::new(
                        "PERMISSION_DENIED",
                        "连接只能引用当前任务草案中的节点",
                    ));
                }
            }
            sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,z_order,children_json,content_json,created_at,updated_at) VALUES(?,?,'arrow',0,0,0,0,?,?,?,?,?)")
                .bind(&id).bind(board).bind(z).bind(json!([from,to]).to_string()).bind(json!({"rationale":a["rationale"]}).to_string()).bind(&now).bind(&now).execute(&mut **tx).await?;
        }
        "add_editorial_note" => {
            let text = a["text"].as_str().unwrap_or("").trim();
            if text.is_empty() || text.chars().count() > 5000 {
                return Err(AppError::new("VALIDATION_ERROR", "便签须为 1–5000 字符"));
            }
            let x = a["x"].as_f64().unwrap_or(0.);
            let y = a["y"].as_f64().unwrap_or(0.);
            validate_rectangle(x, y, 220., 120.)?;
            let ids: Vec<String> = serde_json::from_value(a["photo_ids"].clone())
                .map_err(crate::command_audit::serialization)?;
            if ids.iter().any(|id| !task.allowed_photo_ids.contains(id)) {
                return Err(AppError::new(
                    "PERMISSION_DENIED",
                    "便签引用的照片不在任务范围内",
                ));
            }
            sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,z_order,content_json,style_json,created_at,updated_at) VALUES(?,?,'sticky',?,?,220,120,?,?,?,?,?)")
                .bind(&id).bind(board).bind(x).bind(y).bind(z).bind(json!({"text":text,"rationale":a["rationale"],"photoIds":ids}).to_string()).bind("{\"color\":\"#d8e9ef\"}").bind(&now).bind(&now).execute(&mut **tx).await?;
        }
        _ => return Err(AppError::new("PERMISSION_DENIED", "此任务不能执行该操作")),
    }
    Ok(json!({"board_id":board,"node_id":id,"rationale":a["rationale"]}))
}
