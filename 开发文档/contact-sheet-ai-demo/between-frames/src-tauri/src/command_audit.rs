use crate::{
    command_handler::GuiCommandRequest,
    commands_generated::GuiCommand,
    domain::{self, AppError, Result},
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Sqlite, Transaction};
pub(crate) fn legacy_kind(command: &GuiCommand) -> &'static str {
    match command {
        GuiCommand::CreateProject { .. } => "project_create",
        GuiCommand::UpdateProject { .. } | GuiCommand::UpdateProjectBrief { .. } => "project",
        GuiCommand::ArchiveProject {} => "project_archive",
        GuiCommand::RestoreProject {} => "project_restore",
        GuiCommand::SetPhotoDecision { .. } => "decision",
        GuiCommand::SetPhotoRating { .. } => "rating",
        GuiCommand::SetPhotoNote { .. } => "note",
        GuiCommand::SetPhotoBookMark { .. } => "book_mark",
        GuiCommand::RecordComparison { .. } => "compare",
        GuiCommand::ReorderSequence { .. } => "reorder",
        GuiCommand::SaveSequenceVersion { .. } => "version_save",
        GuiCommand::RestoreSequenceVersion { .. } => "version_restore",
        GuiCommand::UndoCommandGroup { .. } => "undo",
        GuiCommand::RedoCommandGroup { .. } => "redo",
        GuiCommand::ApplyDraft { .. } => "draft_apply",
        GuiCommand::RebaseDraft { .. } => "draft_rebase",
        GuiCommand::UpdateDraft { .. } => "draft_edit",
        GuiCommand::DiscardDraft { .. } => "draft_discard",
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn legacy_event(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    photo: Option<&str>,
    kind: &str,
    before: Option<&str>,
    after: &str,
    reason: Option<&str>,
    actor: &str,
    context: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO decision_events VALUES(?,?,?,?,?,?,?,?,?,?)")
        .bind(domain::id())
        .bind(project)
        .bind(photo)
        .bind(actor)
        .bind(kind)
        .bind(before)
        .bind(after)
        .bind(reason)
        .bind(context)
        .bind(domain::timestamp())
        .execute(&mut **tx)
        .await?;
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn internal_event(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    photo: Option<&str>,
    kind: &str,
    revision: i64,
    before: &Value,
    after: &Value,
    source: &str,
) -> Result<()> {
    let id = domain::id();
    let key = domain::id();
    let actor = if source == "system_import" {
        "system"
    } else {
        "human"
    };
    let request = json!({"commandId":id,"projectId":project,"actorType":actor,"source":source,"expectedProjectRevision":revision,"causationId":null,"idempotencyKey":key,"command":{"type":if kind=="import" {"ImportPhoto"} else {"RelinkPhoto"},"photoId":photo,"metadata":after}});
    legacy_event(
        tx,
        project,
        photo,
        kind,
        Some(&before.to_string()),
        &after.to_string(),
        None,
        if actor == "system" { "system" } else { "user" },
        &json!({"revision":revision+1,"commandId":id,"source":source}).to_string(),
    )
    .await?;
    sqlx::query("INSERT INTO command_events VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&id)
        .bind(project)
        .bind(&id)
        .bind(&key)
        .bind(fingerprint(&request))
        .bind(actor)
        .bind(source)
        .bind(kind)
        .bind(request.to_string())
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .bind(Some(&id))
        .bind(before.to_string())
        .bind(after.to_string())
        .bind(json!({"revision":revision+1,"changed":true,"requestId":id}).to_string())
        .bind(revision)
        .bind(revision + 1)
        .bind(domain::timestamp())
        .execute(&mut **tx)
        .await?;
    crate::agent::store::mark_stale(tx, project, revision + 1).await?;
    Ok(())
}
pub(crate) fn fingerprint(value: &Value) -> String {
    format!("{:x}", Sha256::digest(value.to_string().as_bytes()))
}
pub(crate) fn encode(value: &impl Serialize) -> Result<String> {
    serde_json::to_string(value).map_err(serialization)
}
pub(crate) fn serialization(error: serde_json::Error) -> AppError {
    AppError::new("VALIDATION_ERROR", format!("数据契约不合法：{error}"))
}

pub(crate) async fn bind_aliases(
    tx: &mut Transaction<'_, Sqlite>,
    request: &GuiCommandRequest,
    event: &str,
) -> Result<()> {
    for (kind, value) in [
        ("command_id", &request.command_id),
        ("idempotency_key", &request.idempotency_key),
    ] {
        sqlx::query("INSERT INTO command_receipt_aliases(kind,value,event_id) VALUES(?,?,?) ON CONFLICT(kind,value) DO NOTHING").bind(kind).bind(value).bind(event).execute(&mut **tx).await?;
    }
    Ok(())
}

pub(crate) fn compensated_group(command: &GuiCommand) -> Option<&str> {
    match command {
        GuiCommand::UndoCommandGroup {
            transaction_group_id,
        }
        | GuiCommand::RedoCommandGroup {
            transaction_group_id,
        } => Some(transaction_group_id),
        _ => None,
    }
}
