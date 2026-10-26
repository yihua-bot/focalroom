//! 出版人工命令：事务内比较版本、更新版面、追加快照与回执。Agent 不调用此入口。
use crate::{
    command_audit::{encode, fingerprint, serialization},
    command_handler::{CommandContext, CommandHandler},
    db::Store,
    domain::{self, AppError, Result},
    publication_versions as versions,
    publishing::{CreatePublicationCommand, GenerateFromSequenceCommand, Publication},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationCommandRequest {
    pub schema_version: i64,
    pub command_id: String,
    pub idempotency_key: String,
    pub project_id: String,
    pub publication_id: String,
    pub base_revision: i64,
    pub operations: Vec<PublicationOperation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationOperation {
    pub op_id: String,
    pub action: PublicationAction,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", deny_unknown_fields)]
pub enum PublicationAction {
    Create(CreatePublicationCommand),
    Generate(GeneratePublication),
    UpdateFrame(FrameUpdate),
    UpdatePage(PageUpdate),
    CaptureVersion(VersionLabel),
    RestoreVersion(VersionReference),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratePublication {
    pub photos_per_page: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionLabel {
    pub label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionReference {
    pub version_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameUpdate {
    pub frame_id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub fit_mode: String,
    pub crop_x: f64,
    pub crop_y: f64,
    pub crop_zoom: f64,
    pub asset_id: Option<String>,
    pub rendition_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageUpdate {
    pub page_id: String,
    pub width_mm: f64,
    pub height_mm: f64,
    pub margin_mm: f64,
    pub bleed_mm: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationReceipt {
    pub command_id: String,
    pub project_id: String,
    pub publication_id: String,
    pub base_revision: i64,
    pub revision: i64,
    pub changed: bool,
    pub results: Vec<Value>,
}
fn invalid(message: &str) -> AppError {
    AppError::new("VALIDATION_ERROR", message)
}
fn collision() -> AppError {
    AppError::new(
        "IDEMPOTENCY_CONFLICT",
        "这次操作编号已用于其他内容，请重新打开出版物",
    )
}
fn validate(request: &PublicationCommandRequest) -> Result<()> {
    if request.schema_version != 3
        || request.project_id.is_empty()
        || request.project_id.len() > 160
        || request.idempotency_key.is_empty()
        || request.idempotency_key.len() > 160
        || request.base_revision < 0
        || request.base_revision > 9_007_199_254_739_000
        || request.operations.is_empty()
        || request.operations.len() > 100
        || uuid::Uuid::parse_str(&request.command_id).is_err()
        || uuid::Uuid::parse_str(&request.publication_id).is_err()
    {
        return Err(invalid("这次出版操作无效，请重新打开后重试"));
    }
    let mut seen = BTreeSet::new();
    for op in &request.operations {
        if uuid::Uuid::parse_str(&op.op_id).is_err() || !seen.insert(&op.op_id) {
            return Err(invalid("请勿重复提交同一项版面操作"));
        }
        if !matches!(
            op.action,
            PublicationAction::UpdateFrame(_) | PublicationAction::UpdatePage(_)
        ) && request.operations.len() != 1
        {
            return Err(invalid("保存或恢复版本需要单独确认"));
        }
        if let PublicationAction::CaptureVersion(data) = &op.action
            && (data.label.trim().is_empty() || data.label.chars().count() > 120)
        {
            return Err(invalid("版本名称须为1–120个字符"));
        }
    }
    Ok(())
}
impl CommandHandler<'_> {
    pub async fn execute_publication(
        &self,
        request: PublicationCommandRequest,
        context: &CommandContext,
    ) -> Result<PublicationReceipt> {
        if !context.permits_human(&request.project_id) {
            return Err(AppError::new("PERMISSION_DENIED", "没有编辑此出版物的权限"));
        }
        validate(&request)?;
        let raw = encode(&request)?;
        if raw.len() > 1024 * 1024 {
            return Err(invalid("这次版面操作过大，请分次保存"));
        }
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        let saved:Vec<(String,String)>=sqlx::query_as("SELECT request_json,receipt_json FROM publication_command_events WHERE command_id=? OR idempotency_key=?").bind(&request.command_id).bind(&request.idempotency_key).fetch_all(&mut *tx).await?;
        if !saved.is_empty() {
            if saved.len() != 1 || saved[0].0 != raw {
                return Err(collision());
            }
            let receipt = serde_json::from_str(&saved[0].1).map_err(serialization)?;
            tx.commit().await?;
            return Ok(receipt);
        }
        crate::agent::promotion::ensure_local_write(&mut tx, &request.project_id).await?;
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id=? AND deleted_at IS NULL)",
        )
        .bind(&request.project_id)
        .fetch_one(&mut *tx)
        .await?;
        if !active {
            return Err(AppError::new("NOT_FOUND", "项目不存在或已归档"));
        }
        for op in &request.operations {
            let used: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM publication_operation_ids WHERE op_id=?)",
            )
            .bind(&op.op_id)
            .fetch_one(&mut *tx)
            .await?;
            if used {
                return Err(collision());
            }
        }
        let state: Option<(i64, String)> =
            sqlx::query_as("SELECT revision,state FROM publications WHERE id=? AND project_id=?")
                .bind(&request.publication_id)
                .bind(&request.project_id)
                .fetch_optional(&mut *tx)
                .await?;
        let creating = matches!(request.operations[0].action, PublicationAction::Create(_));
        if creating && state.is_some() {
            return Err(collision());
        }
        if !creating && state.is_none() {
            return Err(AppError::new("NOT_FOUND", "出版物不存在或不属于当前项目"));
        }
        let base = state.as_ref().map_or(0, |s| s.0);
        if base != request.base_revision {
            let mut error =
                AppError::new("REVISION_CONFLICT", "出版物已有修改，请重新查看后再保存");
            error.current_revision = Some(base);
            return Err(error);
        }
        if state.as_ref().is_some_and(|s| s.1 != "draft") {
            return Err(AppError::new("CONFLICT", "请在可编辑的出版草稿中修改版面"));
        }
        let restoring = matches!(
            request.operations[0].action,
            PublicationAction::RestoreVersion(_)
        );
        let before = if creating {
            None
        } else {
            match versions::layout_tx(&mut tx, &request.project_id, &request.publication_id).await {
                Ok(layout) => Some(layout),
                Err(error)
                    if restoring
                        && matches!(
                            error.code.as_str(),
                            "VALIDATION_ERROR" | "ASSET_NOT_FOUND"
                        ) =>
                {
                    None
                }
                Err(error) => return Err(error),
            }
        };
        let mut results = Vec::new();
        let mut label = "调整版面".to_owned();
        let mut kind = "working";
        for op in &request.operations {
            results.push(match &op.action {
                PublicationAction::Create(data) => {
                    label = "创建出版物".into();
                    json!(
                        self.create_publication_tx(
                            &mut tx,
                            &request.project_id,
                            data,
                            &request.publication_id
                        )
                        .await?
                    )
                }
                PublicationAction::Generate(data) => {
                    label = "从作品序列生成".into();
                    json!(
                        self.generate_from_sequence_tx(
                            &mut tx,
                            &request.project_id,
                            &GenerateFromSequenceCommand {
                                publication_id: request.publication_id.clone(),
                                photos_per_page: Some(data.photos_per_page)
                            }
                        )
                        .await?
                    )
                }
                PublicationAction::UpdateFrame(data) => {
                    update_frame(&mut tx, &request.project_id, &request.publication_id, data)
                        .await?;
                    Value::Null
                }
                PublicationAction::UpdatePage(data) => {
                    update_page(&mut tx, &request.publication_id, data).await?;
                    Value::Null
                }
                PublicationAction::CaptureVersion(data) => {
                    label = data.label.trim().into();
                    kind = "saved";
                    Value::Null
                }
                PublicationAction::RestoreVersion(data) => {
                    let version = versions::version_tx(
                        &mut tx,
                        &request.project_id,
                        &request.publication_id,
                        &data.version_id,
                    )
                    .await?;
                    label = format!("恢复：{}", version.version.label);
                    versions::restore_tx(
                        &mut tx,
                        &request.project_id,
                        &request.publication_id,
                        &version.layout,
                    )
                    .await?;
                    Value::Null
                }
            });
        }
        let after =
            versions::layout_tx(&mut tx, &request.project_id, &request.publication_id).await?;
        let changed = before.as_ref() != Some(&after)
            || matches!(
                request.operations[0].action,
                PublicationAction::CaptureVersion(_) | PublicationAction::RestoreVersion(_)
            );
        let revision = base + i64::from(changed);
        if let Some(before) = &before {
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM publication_versions WHERE publication_id=? AND revision=?)").bind(&request.publication_id).bind(base).fetch_one(&mut *tx).await?;
            if !exists {
                save_version(
                    &mut tx,
                    &request,
                    base,
                    "开始记录版本",
                    "working",
                    before,
                    &domain::id(),
                )
                .await?;
            }
        }
        if changed {
            sqlx::query(
                "UPDATE publications SET revision=?,updated_at=? WHERE id=? AND project_id=?",
            )
            .bind(revision)
            .bind(domain::timestamp())
            .bind(&request.publication_id)
            .bind(&request.project_id)
            .execute(&mut *tx)
            .await?;
            save_version(
                &mut tx,
                &request,
                revision,
                &label,
                kind,
                &after,
                &request.command_id,
            )
            .await?;
        }
        let receipt = PublicationReceipt {
            command_id: request.command_id.clone(),
            project_id: request.project_id.clone(),
            publication_id: request.publication_id.clone(),
            base_revision: base,
            revision,
            changed,
            results,
        };
        sqlx::query("INSERT INTO publication_command_events(command_id,idempotency_key,project_id,publication_id,base_revision,revision,request_json,receipt_json,created_at) VALUES(?,?,?,?,?,?,?,?,?)")
            .bind(&request.command_id).bind(&request.idempotency_key).bind(&request.project_id).bind(&request.publication_id).bind(base).bind(revision).bind(raw).bind(encode(&receipt)?).bind(domain::timestamp()).execute(&mut *tx).await?;
        for op in &request.operations {
            sqlx::query("INSERT INTO publication_operation_ids(op_id,command_id) VALUES(?,?)")
                .bind(&op.op_id)
                .bind(&request.command_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(receipt)
    }
    // 兼容旧IPC与内部调用，仍委托相同的人工命令事务，不保留无审计的写入口。
    pub async fn create_publication(
        &self,
        project: &str,
        data: &CreatePublicationCommand,
    ) -> Result<Publication> {
        let request = PublicationCommandRequest::new(
            project,
            &domain::id(),
            0,
            PublicationAction::Create(data.clone()),
        );
        let receipt = self
            .execute_publication(request, &CommandContext::human(project))
            .await?;
        serde_json::from_value(receipt.results[0].clone()).map_err(serialization)
    }
    pub async fn generate_from_sequence(
        &self,
        project: &str,
        data: &GenerateFromSequenceCommand,
    ) -> Result<i64> {
        let revision = self
            .store
            .publication_revision(project, &data.publication_id)
            .await?;
        let request = PublicationCommandRequest::new(
            project,
            &data.publication_id,
            revision,
            PublicationAction::Generate(GeneratePublication {
                photos_per_page: data.photos_per_page.unwrap_or(1),
            }),
        );
        let receipt = self
            .execute_publication(request, &CommandContext::human(project))
            .await?;
        receipt.results[0]
            .as_i64()
            .ok_or_else(|| AppError::new("INTERNAL", "页面生成结果不完整"))
    }
}
async fn save_version(
    tx: &mut Transaction<'_, Sqlite>,
    request: &PublicationCommandRequest,
    revision: i64,
    label: &str,
    kind: &str,
    layout: &versions::PublicationLayout,
    id: &str,
) -> Result<()> {
    let raw = encode(layout)?;
    let digest = fingerprint(&serde_json::to_value(layout).map_err(serialization)?);
    sqlx::query("INSERT INTO publication_versions(id,publication_id,project_id,revision,label,kind,snapshot_json,snapshot_sha256,command_id,created_at) VALUES(?,?,?,?,?,?,?,?,?,?)")
        .bind(id).bind(&request.publication_id).bind(&request.project_id).bind(revision).bind(label).bind(kind).bind(raw).bind(digest).bind(&request.command_id).bind(domain::timestamp()).execute(&mut **tx).await?;
    Ok(())
}
async fn update_frame(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &FrameUpdate,
) -> Result<()> {
    let old:versions::LayoutFrame=sqlx::query_as("SELECT f.id,f.x_mm,f.y_mm,f.width_mm,f.height_mm,f.fit_mode,f.asset_id,f.rendition_id,f.crop_x,f.crop_y,f.crop_zoom FROM publication_frames f JOIN publication_pages p ON p.id=f.page_id WHERE f.id=? AND p.publication_id=?").bind(&data.frame_id).bind(publication).fetch_optional(&mut **tx).await?.ok_or_else(||AppError::new("NOT_FOUND","照片框已不存在，请重新打开页面"))?;
    versions::source_tx(
        tx,
        project,
        data.asset_id.as_deref(),
        data.rendition_id.as_deref(),
    )
    .await?;
    let next = versions::LayoutFrame {
        id: data.frame_id.clone(),
        x_mm: data.x_mm,
        y_mm: data.y_mm,
        width_mm: data.width_mm,
        height_mm: data.height_mm,
        fit_mode: data.fit_mode.clone(),
        asset_id: data.asset_id.clone(),
        rendition_id: data.rendition_id.clone(),
        crop_x: data.crop_x,
        crop_y: data.crop_y,
        crop_zoom: data.crop_zoom,
        source: None,
    };
    if old == next {
        return Ok(());
    }
    sqlx::query("UPDATE publication_frames SET x_mm=?,y_mm=?,width_mm=?,height_mm=?,fit_mode=?,asset_id=?,rendition_id=?,crop_x=?,crop_y=?,crop_zoom=?,updated_at=? WHERE id=?")
        .bind(data.x_mm).bind(data.y_mm).bind(data.width_mm).bind(data.height_mm).bind(&data.fit_mode).bind(&data.asset_id).bind(&data.rendition_id).bind(data.crop_x).bind(data.crop_y).bind(data.crop_zoom).bind(domain::timestamp()).bind(&data.frame_id).execute(&mut **tx).await?;
    Ok(())
}
async fn update_page(
    tx: &mut Transaction<'_, Sqlite>,
    publication: &str,
    data: &PageUpdate,
) -> Result<()> {
    let old:Option<(f64,f64,f64,f64)>=sqlx::query_as("SELECT width_mm,height_mm,margin_mm,bleed_mm FROM publication_pages WHERE id=? AND publication_id=?").bind(&data.page_id).bind(publication).fetch_optional(&mut **tx).await?;
    let old = old.ok_or_else(|| AppError::new("NOT_FOUND", "页面已不存在，请重新打开出版物"))?;
    if old == (data.width_mm, data.height_mm, data.margin_mm, data.bleed_mm) {
        return Ok(());
    }
    sqlx::query("UPDATE publication_pages SET width_mm=?,height_mm=?,margin_mm=?,bleed_mm=?,updated_at=? WHERE id=? AND publication_id=?")
        .bind(data.width_mm).bind(data.height_mm).bind(data.margin_mm).bind(data.bleed_mm).bind(domain::timestamp()).bind(&data.page_id).bind(publication).execute(&mut **tx).await?;
    Ok(())
}
impl PublicationCommandRequest {
    pub fn new(project: &str, publication: &str, revision: i64, action: PublicationAction) -> Self {
        let id = domain::id();
        Self {
            schema_version: 3,
            command_id: id.clone(),
            idempotency_key: id,
            project_id: project.into(),
            publication_id: publication.into(),
            base_revision: revision,
            operations: vec![PublicationOperation {
                op_id: domain::id(),
                action,
            }],
        }
    }
}
impl Store {
    pub async fn execute_publication(
        &self,
        request: PublicationCommandRequest,
    ) -> Result<PublicationReceipt> {
        let context = CommandContext::human(&request.project_id);
        CommandHandler::new(self)
            .execute_publication(request, &context)
            .await
    }
    pub async fn publication_revision(&self, project: &str, publication: &str) -> Result<i64> {
        sqlx::query_scalar("SELECT b.revision FROM publications b JOIN projects p ON p.id=b.project_id WHERE b.id=? AND b.project_id=? AND p.deleted_at IS NULL").bind(publication).bind(project).fetch_optional(&self.pool).await?.ok_or_else(||AppError::new("NOT_FOUND","出版物不存在或项目已归档"))
    }
}

#[cfg(test)]
#[path = "publication_command_tests.rs"]
mod tests;
