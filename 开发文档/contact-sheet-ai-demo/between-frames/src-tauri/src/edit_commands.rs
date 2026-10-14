use crate::{
    command_audit::{encode, serialization},
    command_handler::GuiCommandRequest,
    commands_generated::GuiCommand,
    db,
    domain::{self, AppError, Result},
    edit_state::{EditState, apply_state, compatible, validate_selection},
};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::HashSet;
pub(crate) struct EditEffect {
    pub undoable: bool,
    pub force: bool,
    pub history_operation: bool,
}
pub(crate) async fn apply_gui(
    tx: &mut Transaction<'_, Sqlite>,
    request: &GuiCommandRequest,
    before: &Option<EditState>,
    revision: i64,
    now: &str,
    archived: bool,
) -> Result<EditEffect> {
    let mut undoable = true;
    let mut force = false;
    let mut history_operation = false;
    match &request.command {
        GuiCommand::CreateProject { title, brief } => {
            validate_project(title, brief)?;
            sqlx::query(
                "INSERT INTO projects(id,title,brief,created_at,updated_at) VALUES(?,?,?,?,?)",
            )
            .bind(&request.project_id)
            .bind(title.trim())
            .bind(brief)
            .bind(now)
            .bind(now)
            .execute(&mut **tx)
            .await?;
            undoable = false;
            force = true;
        }
        GuiCommand::UpdateProject { title, brief } => {
            validate_project(title, brief)?;
            sqlx::query("UPDATE projects SET title=?,brief=? WHERE id=?")
                .bind(title.trim())
                .bind(brief)
                .bind(&request.project_id)
                .execute(&mut **tx)
                .await?;
        }
        GuiCommand::UpdateProjectBrief { brief } => {
            validate_text(brief, 1500, "创作意图")?;
            sqlx::query("UPDATE projects SET brief=? WHERE id=?")
                .bind(brief)
                .bind(&request.project_id)
                .execute(&mut **tx)
                .await?;
        }
        GuiCommand::ArchiveProject {} => {
            sqlx::query("UPDATE projects SET deleted_at=? WHERE id=?")
                .bind(now)
                .bind(&request.project_id)
                .execute(&mut **tx)
                .await?;
        }
        GuiCommand::RestoreProject {} => {
            if !archived {
                return Err(AppError::new("INVALID_INPUT", "项目没有归档"));
            }
            sqlx::query("UPDATE projects SET deleted_at=NULL WHERE id=?")
                .bind(&request.project_id)
                .execute(&mut **tx)
                .await?;
        }
        GuiCommand::SetPhotoDecision { photo_ids, status } => {
            validate_selection(tx, &request.project_id, photo_ids).await?;
            if !["keep", "reject", "undecided"].contains(&status.as_str()) {
                return Err(AppError::new("INVALID_INPUT", "照片状态不合法"));
            }
            let mut sequence = db::sequence(tx, &request.project_id).await?;
            for photo in photo_ids {
                sqlx::query("UPDATE photo_decisions SET status=?,updated_at=? WHERE project_id=? AND photo_id=?").bind(status).bind(now).bind(&request.project_id).bind(photo).execute(&mut **tx).await?;
                if status == "keep" && !sequence.contains(photo) {
                    sequence.push(photo.clone());
                }
            }
            if status != "keep" {
                sequence.retain(|photo| !photo_ids.contains(photo));
            }
            if sequence != before.as_ref().unwrap().sequence {
                db::write_sequence(tx, &request.project_id, &sequence).await?;
            }
        }
        GuiCommand::SetPhotoBookMark { photo_id, mark } => {
            validate_selection(tx, &request.project_id, std::slice::from_ref(photo_id)).await?;
            let normalized = match mark.as_deref() {
                None | Some("") => None,
                Some("spread") => Some("spread"),
                Some("full") => Some("full"),
                Some(_) => return Err(AppError::new("INVALID_INPUT", "册页标记不合法")),
            };
            sqlx::query("UPDATE photo_decisions SET book_mark=?,updated_at=? WHERE project_id=? AND photo_id=?")
                .bind(normalized)
                .bind(now)
                .bind(&request.project_id)
                .bind(photo_id)
                .execute(&mut **tx)
                .await?;
        }
        GuiCommand::SetPhotoRating { photo_ids, rating } => {
            validate_selection(tx, &request.project_id, photo_ids).await?;
            if !(0..=5).contains(rating) {
                return Err(AppError::new("INVALID_INPUT", "人工评分必须为 0–5"));
            }
            for photo in photo_ids {
                sqlx::query("UPDATE photo_decisions SET rating=?,updated_at=? WHERE project_id=? AND photo_id=?").bind(rating).bind(now).bind(&request.project_id).bind(photo).execute(&mut **tx).await?;
            }
        }
        GuiCommand::SetPhotoNote { photo_id, note } => {
            validate_selection(tx, &request.project_id, std::slice::from_ref(photo_id)).await?;
            validate_text(note, 2000, "照片备注")?;
            sqlx::query("UPDATE photo_decisions SET user_note=?,updated_at=? WHERE project_id=? AND photo_id=?").bind(note).bind(now).bind(&request.project_id).bind(photo_id).execute(&mut **tx).await?;
        }
        GuiCommand::ReorderSequence { ordered_photo_ids } => {
            let previous = &before.as_ref().unwrap().sequence;
            if ordered_photo_ids.len() > 10_000
                || previous.len() != ordered_photo_ids.len()
                || ordered_photo_ids.iter().collect::<HashSet<_>>().len() != ordered_photo_ids.len()
                || ordered_photo_ids.iter().any(|id| !previous.contains(id))
            {
                return Err(AppError::new(
                    "INVALID_SEQUENCE",
                    "序列必须包含全部已保留照片，且不可重复",
                ));
            }
            if previous != ordered_photo_ids {
                db::write_sequence(tx, &request.project_id, ordered_photo_ids).await?;
            }
        }
        GuiCommand::RecordComparison {
            left_id,
            right_id,
            preference,
            reason,
        } => {
            validate_selection(
                tx,
                &request.project_id,
                &[left_id.clone(), right_id.clone()],
            )
            .await?;
            validate_text(reason, 2000, "比较理由")?;
            let preferred = match preference.as_str() {
                "left" => Some(left_id),
                "right" => Some(right_id),
                "uncertain" => None,
                _ => return Err(AppError::new("INVALID_INPUT", "比较偏好不合法")),
            };
            sqlx::query("INSERT INTO comparison_events VALUES(?,?,?,?,?,?,?)")
                .bind(domain::id())
                .bind(&request.project_id)
                .bind(left_id)
                .bind(right_id)
                .bind(preferred)
                .bind(reason)
                .bind(now)
                .execute(&mut **tx)
                .await?;
        }
        GuiCommand::SaveSequenceVersion { label } => {
            if label.trim().is_empty() {
                return Err(AppError::new("INVALID_INPUT", "请输入版本名称"));
            }
            validate_text(label, 80, "版本名称")?;
            sqlx::query("INSERT INTO sequence_versions VALUES(?,?,?,?,?,?)")
                .bind(domain::id())
                .bind(&request.project_id)
                .bind(label.trim())
                .bind(revision)
                .bind(encode(before.as_ref().unwrap())?)
                .bind(now)
                .execute(&mut **tx)
                .await?;
            undoable = false;
            force = true;
        }
        GuiCommand::RestoreSequenceVersion { version_id } => {
            let raw: String = sqlx::query_scalar(
                "SELECT snapshot_json FROM sequence_versions WHERE project_id=? AND id=?",
            )
            .bind(&request.project_id)
            .bind(version_id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| AppError::new("NOT_FOUND", "版本不属于该项目"))?;
            let saved: EditState = serde_json::from_str(&raw).map_err(serialization)?;
            let mut target = before.clone().unwrap();
            for photo in &mut target.photos {
                if let Some(old) = saved
                    .photos
                    .iter()
                    .find(|old| old.photo_id == photo.photo_id)
                {
                    *photo = old.clone();
                } else {
                    photo.status = "undecided".into();
                }
            }
            target.sequence = saved.sequence;
            apply_state(tx, &request.project_id, &target).await?;
        }
        GuiCommand::ApplyDraft { .. }
        | GuiCommand::RebaseDraft { .. }
        | GuiCommand::UpdateDraft { .. }
        | GuiCommand::DiscardDraft { .. } => {
            undoable =
                crate::agent::apply_human_draft(tx, request, before.as_ref().unwrap(), revision)
                    .await?;
            force = true;
        }
        GuiCommand::UndoCommandGroup {
            transaction_group_id,
        }
        | GuiCommand::RedoCommandGroup {
            transaction_group_id,
        } => {
            let redo = matches!(request.command, GuiCommand::RedoCommandGroup { .. });
            let cursor: i64 =
                sqlx::query_scalar("SELECT cursor FROM history_heads WHERE project_id=?")
                    .bind(&request.project_id)
                    .fetch_optional(&mut **tx)
                    .await?
                    .unwrap_or(0);
            let position = if redo { cursor + 1 } else { cursor };
            let row=sqlx::query("SELECT e.before_json,e.after_json FROM command_history h JOIN command_events e ON e.id=h.group_id WHERE h.project_id=? AND h.position=? AND h.group_id=? AND h.active=1").bind(&request.project_id).bind(position).bind(transaction_group_id).fetch_optional(&mut **tx).await?.ok_or_else(||AppError::new("HISTORY_CONFLICT","只能撤销或重做当前历史栈顶"))?;
            let target: EditState = serde_json::from_str(&row.get::<String, _>(if redo {
                "after_json"
            } else {
                "before_json"
            }))
            .map_err(serialization)?;
            let expected: EditState = serde_json::from_str(&row.get::<String, _>(if redo {
                "before_json"
            } else {
                "after_json"
            }))
            .map_err(serialization)?;
            if !compatible(before.as_ref().unwrap(), &expected) {
                return Err(AppError::new(
                    "HISTORY_CONFLICT",
                    "历史之后的状态已改变，不能强行覆盖",
                ));
            }
            apply_state(tx, &request.project_id, &target).await?;
            sqlx::query("UPDATE history_heads SET cursor=? WHERE project_id=?")
                .bind(if redo { cursor + 1 } else { cursor - 1 })
                .bind(&request.project_id)
                .execute(&mut **tx)
                .await?;
            undoable = false;
            force = true;
            history_operation = true;
        }
    }
    Ok(EditEffect {
        undoable,
        force,
        history_operation,
    })
}
fn validate_project(title: &str, brief: &str) -> Result<()> {
    if title.trim().is_empty() {
        return Err(AppError::new("INVALID_INPUT", "请输入项目名称"));
    }
    validate_text(title, 80, "项目名称")?;
    validate_text(brief, 1500, "创作意图")
}
fn validate_text(text: &str, max: usize, label: &str) -> Result<()> {
    if text.chars().count() > max {
        Err(AppError::new(
            "INVALID_INPUT",
            format!("{label}最多 {max} 字"),
        ))
    } else {
        Ok(())
    }
}
