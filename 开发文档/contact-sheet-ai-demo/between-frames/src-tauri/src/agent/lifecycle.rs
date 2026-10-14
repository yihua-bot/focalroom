use super::{policy::validate_schema, store, types::*};
use crate::{
    command_audit::{encode, fingerprint},
    db::Store,
    domain::{self, AppError, Result},
    edit_state::validate_selection,
};
use serde_json::json;
use sqlx::Row;

pub(crate) async fn request(db: &Store, r: AgentRequest) -> Result<Task> {
    validate_schema(
        &serde_json::from_str(include_str!(
            "../../../contracts/agent-task-request.schema.json"
        ))
        .unwrap(),
        &serde_json::to_value(&r).unwrap(),
    )?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let revision: i64 =
        sqlx::query_scalar("SELECT revision FROM projects WHERE id=? AND deleted_at IS NULL")
            .bind(&r.project_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::new("PERMISSION_DENIED", "项目不可访问"))?;
    let fp = fingerprint(&serde_json::to_value(&r).unwrap());
    if let Some(row) = sqlx::query(
        "SELECT project_id,fingerprint,result_json FROM agent_request_receipts WHERE request_id=?",
    )
    .bind(&r.request_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        if row.get::<String, _>("project_id") != r.project_id
            || row.get::<String, _>("fingerprint") != fp
        {
            return Err(AppError::new(
                "IDEMPOTENCY_CONFLICT",
                "任务请求 ID 已用于其他请求",
            ));
        }
        return serde_json::from_str(&row.get::<String, _>("result_json"))
            .map_err(crate::command_audit::serialization);
    }
    let mut task = if let Some(id) = r.command.task_id() {
        let t = store::load(&mut tx, id).await?;
        if t.project_id != r.project_id {
            return Err(AppError::new("PERMISSION_DENIED", "任务不属于此项目"));
        }
        t
    } else {
        let TaskCommand::StartTask {
            goal,
            selected_photo_ids,
            target_count,
            budget,
        } = &r.command
        else {
            unreachable!()
        };
        budget.validate()?;
        if goal.trim().is_empty() {
            return Err(AppError::new("VALIDATION_ERROR", "编辑目标不能为空"));
        }
        if !selected_photo_ids.is_empty() {
            validate_selection(&mut tx, &r.project_id, selected_photo_ids).await?;
        }
        let all: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM photos WHERE project_id=? ORDER BY import_index LIMIT 501",
        )
        .bind(&r.project_id)
        .fetch_all(&mut *tx)
        .await?;
        let allowed = if selected_photo_ids.is_empty() {
            all
        } else {
            selected_photo_ids.clone()
        };
        if allowed.is_empty() || allowed.len() > 500 {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "任务需限定 1–500 张照片；较大图库请先选择照片",
            ));
        }
        let exists:i64=sqlx::query_scalar("SELECT COUNT(*) FROM agent_tasks WHERE project_id=? AND state IN('queued','planning','running')").bind(&r.project_id).fetch_one(&mut *tx).await?;
        if exists > 0 {
            return Err(AppError::new("TASK_BUSY", "此项目已有正在执行的任务"));
        }
        let now = domain::timestamp();
        let t = Task {
            id: domain::id(),
            project_id: r.project_id.clone(),
            goal: goal.trim().into(),
            base_revision: revision,
            allowed_photo_ids: allowed,
            selected_photo_ids: selected_photo_ids.clone(),
            target_count: *target_count,
            state: "queued".into(),
            reason: "已排队 · 本地 Mock，无图像上传".into(),
            budget: budget.clone(),
            usage: Usage::default(),
            elapsed_ms: 0,
            question: None,
            answer: None,
            final_draft_ids: vec![],
            pending: None,
            created_at: now.clone(),
            updated_at: now,
        };
        sqlx::query("INSERT INTO agent_tasks(id,project_id,goal,base_revision,allowed_ids_json,selected_ids_json,target_count,state,reason,budget_json,usage_json,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)")
 .bind(&t.id).bind(&t.project_id).bind(&t.goal).bind(t.base_revision).bind(encode(&t.allowed_photo_ids)?).bind(encode(&t.selected_photo_ids)?).bind(t.target_count as i64).bind(&t.state).bind(&t.reason).bind(encode(&t.budget)?).bind(encode(&t.usage)?).bind(&t.created_at).bind(&t.updated_at).execute(&mut *tx).await?;
        store::step(&mut tx, &t.id, "state", json!({"to":"queued","providerId":super::PROVIDER_ID,"modelId":super::MODEL_ID,"promptVersion":super::PROMPT_VERSION}), &t.reason).await?;
        sqlx::query(
            "INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'user_goal',?,?)",
        )
        .bind(&t.id)
        .bind(&t.goal)
        .bind(domain::timestamp())
        .execute(&mut *tx)
        .await?;
        t
    };
    if matches!(
        r.command,
        TaskCommand::PauseTask { .. } | TaskCommand::CancelTask { .. }
    ) {
        store::settle_waiting(&mut tx, &mut task, "暂停/取消前结算未完成等待").await?;
    }
    match &r.command {
        TaskCommand::StartTask { .. } => {}
        TaskCommand::PauseTask { .. } => {
            store::transition(&mut tx, &mut task, "paused", "用户暂停；已完成操作保留").await?
        }
        TaskCommand::CancelTask { .. } => {
            store::transition(
                &mut tx,
                &mut task,
                "cancelled",
                "用户取消；已有草案保留供审核",
            )
            .await?
        }
        TaskCommand::ResumeTask { budget, .. } => {
            budget.validate()?;
            if revision != task.base_revision {
                return Err(AppError::new(
                    "REVISION_CONFLICT",
                    "项目已变化，请保留旧草案并基于新版本重新委托",
                ));
            }
            task.budget = budget.clone();
            store::transition(
                &mut tx,
                &mut task,
                "queued",
                "用户恢复；从已保存调用结果继续",
            )
            .await?;
        }
        TaskCommand::SubmitFeedback { answer, .. } => {
            if answer.trim().is_empty() {
                return Err(AppError::new("VALIDATION_ERROR", "答复不能为空"));
            }
            if task.state != "needs_input" {
                return Err(AppError::new("INVALID_TRANSITION", "任务未等待反馈"));
            }
            task.answer = Some(answer.trim().into());
            task.question = None;
            sqlx::query("INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'clarification_answer',?,?)").bind(&task.id).bind(answer).bind(domain::timestamp()).execute(&mut *tx).await?;
            store::transition(&mut tx, &mut task, "queued", "已收到人工答复").await?;
        }
        TaskCommand::CloseTask { .. } => {
            store::transition(&mut tx, &mut task, "completed", "用户结束审核").await?
        }
    }
    store::checkpoint(&mut tx, &task).await?;
    sqlx::query("INSERT INTO agent_request_receipts VALUES(?,?,?,?)")
        .bind(&r.request_id)
        .bind(&r.project_id)
        .bind(fp)
        .bind(encode(&task)?)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(task)
}
pub(crate) async fn recover(db: &Store) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM agent_tasks WHERE state IN('queued','planning','running')",
    )
    .fetch_all(&mut *tx)
    .await?;
    for id in ids {
        let mut t = store::load(&mut tx, &id).await?;
        store::settle_waiting(&mut tx, &mut t, "中断恢复：未确认等待已保守计入时长预算").await?;
        store::transition(
            &mut tx,
            &mut t,
            "interrupted",
            "应用中断；请明确恢复，已完成调用可回放",
        )
        .await?;
        store::checkpoint(&mut tx, &t).await?;
    }
    tx.commit().await?;
    Ok(())
}
