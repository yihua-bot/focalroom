use super::{store, types::DraftItem};
use crate::{
    command_handler::GuiCommandRequest,
    commands_generated::GuiCommand,
    db,
    domain::{self, AppError, Result},
    edit_state::EditState,
};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::HashSet;

pub(crate) async fn human(
    tx: &mut Transaction<'_, Sqlite>,
    request: &GuiCommandRequest,
    before: &EditState,
    revision: i64,
) -> Result<bool> {
    let (id, expected) = match &request.command {
        GuiCommand::ApplyDraft {
            draft_id,
            expected_draft_revision,
            ..
        }
        | GuiCommand::RebaseDraft {
            draft_id,
            expected_draft_revision,
        }
        | GuiCommand::UpdateDraft {
            draft_id,
            expected_draft_revision,
            ..
        }
        | GuiCommand::DiscardDraft {
            draft_id,
            expected_draft_revision,
        } => (draft_id, expected_draft_revision),
        _ => unreachable!(),
    };
    let row = sqlx::query(
        "SELECT task_id,base_revision,revision,state,(SELECT state FROM agent_tasks WHERE id=drafts.task_id) AS task_state FROM drafts WHERE id=? AND project_id=?",
    )
    .bind(id)
    .bind(&request.project_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::new("NOT_FOUND", "草案不属于当前项目"))?;
    if row.get::<i64, _>("revision") != *expected {
        return Err(AppError::new("REVISION_CONFLICT", "草案已更新，请重新预览"));
    }
    if row.get::<String, _>("state") != "pending" {
        return Err(AppError::new("VALIDATION_ERROR", "草案已采纳或丢弃"));
    }
    if ["queued", "planning", "running"].contains(&row.get::<String, _>("task_state").as_str()) {
        return Err(AppError::new(
            "TASK_BUSY",
            "任务仍在写草案，请先暂停或等待提交审核",
        ));
    }
    let items = store::items(tx, id).await?;
    let now = domain::timestamp();
    let mut undoable = false;
    match &request.command {
        GuiCommand::ApplyDraft {
            approved_photo_ids,
            mode,
            ..
        } => {
            if row.get::<i64, _>("base_revision") != revision {
                return Err(AppError::new(
                    "REVISION_CONFLICT",
                    "草案基线过期，请明确重新比较基线",
                ));
            }
            let approved: HashSet<&String> = approved_photo_ids.iter().collect();
            if approved.is_empty()
                || approved.len() != approved_photo_ids.len()
                || approved.len() > 500
                || approved
                    .iter()
                    .any(|id| !items.iter().any(|i| &i.photo_id == *id))
            {
                return Err(AppError::new(
                    "VALIDATION_ERROR",
                    "只能采纳草案内不同的有效条目",
                ));
            }
            if !["partial", "full"].contains(&mode.as_str())
                || (mode == "full" && approved.len() != items.len())
            {
                return Err(AppError::new(
                    "VALIDATION_ERROR",
                    "全量采纳需要明确选择全部条目",
                ));
            }
            let mut sequence = before.sequence.clone();
            if mode == "full" {
                for photo in &before.photos {
                    if photo.status == "keep"
                        && !items
                            .iter()
                            .any(|i| i.photo_id == photo.photo_id && i.proposed_action == "keep")
                    {
                        sqlx::query("UPDATE photo_decisions SET status='undecided',updated_at=? WHERE project_id=? AND photo_id=?").bind(&now).bind(&request.project_id).bind(&photo.photo_id).execute(&mut **tx).await?;
                    }
                }
                sequence.clear();
            } else {
                sequence.retain(|id| !approved.contains(id));
            }
            for item in &items {
                if approved.contains(&item.photo_id) {
                    let updated=sqlx::query("UPDATE photo_decisions SET status=?,updated_at=? WHERE project_id=? AND photo_id=?").bind(&item.proposed_action).bind(&now).bind(&request.project_id).bind(&item.photo_id).execute(&mut **tx).await?;
                    if updated.rows_affected() != 1 {
                        return Err(AppError::new("PERMISSION_DENIED", "草案照片不属于项目"));
                    }
                    if item.proposed_action == "keep" {
                        let pos = (item.position as usize).min(sequence.len());
                        sequence.insert(pos, item.photo_id.clone());
                    }
                }
            }
            db::write_sequence(tx, &request.project_id, &sequence).await?;
            sqlx::query(
                "UPDATE drafts SET state='applied',revision=revision+1,updated_at=? WHERE id=?",
            )
            .bind(&now)
            .bind(id)
            .execute(&mut **tx)
            .await?;
            undoable = true;
        }
        GuiCommand::RebaseDraft { .. } => {
            sqlx::query(
                "UPDATE drafts SET base_revision=?,revision=revision+1,updated_at=? WHERE id=?",
            )
            .bind(revision + 1)
            .bind(&now)
            .bind(id)
            .execute(&mut **tx)
            .await?;
        }
        GuiCommand::UpdateDraft {
            ordered_photo_ids, ..
        } => {
            if ordered_photo_ids.len() != items.len()
                || ordered_photo_ids.iter().collect::<HashSet<_>>().len() != items.len()
                || ordered_photo_ids
                    .iter()
                    .any(|id| !items.iter().any(|i| &i.photo_id == id))
            {
                return Err(AppError::new(
                    "VALIDATION_ERROR",
                    "编辑草案顺序须包含全部条目",
                ));
            }
            let reordered: Vec<DraftItem> = ordered_photo_ids
                .iter()
                .enumerate()
                .map(|(pos, id)| {
                    let mut i = items.iter().find(|i| &i.photo_id == id).unwrap().clone();
                    i.position = pos as i64;
                    i
                })
                .collect();
            replace_items(tx, id, &reordered).await?;
            sqlx::query("UPDATE drafts SET revision=revision+1,updated_at=? WHERE id=?")
                .bind(&now)
                .bind(id)
                .execute(&mut **tx)
                .await?;
        }
        GuiCommand::DiscardDraft { .. } => {
            sqlx::query(
                "UPDATE drafts SET state='discarded',revision=revision+1,updated_at=? WHERE id=?",
            )
            .bind(&now)
            .bind(id)
            .execute(&mut **tx)
            .await?;
        }
        _ => unreachable!(),
    }
    // 草案排序是审核元数据，正式 revision 未变前的有效基线同步至本命令产生的 revision。
    if matches!(request.command, GuiCommand::UpdateDraft { .. })
        && row.get::<i64, _>("base_revision") == revision
    {
        sqlx::query("UPDATE drafts SET base_revision=? WHERE id=?")
            .bind(revision + 1)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(undoable)
}
pub(crate) async fn replace_items(
    tx: &mut Transaction<'_, Sqlite>,
    draft: &str,
    items: &[DraftItem],
) -> Result<()> {
    sqlx::query("DELETE FROM draft_items WHERE draft_id=?")
        .bind(draft)
        .execute(&mut **tx)
        .await?;
    for i in items {
        sqlx::query("INSERT INTO draft_items VALUES(?,?,?,?,?)")
            .bind(draft)
            .bind(&i.photo_id)
            .bind(&i.proposed_action)
            .bind(i.position)
            .bind(&i.rationale)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
