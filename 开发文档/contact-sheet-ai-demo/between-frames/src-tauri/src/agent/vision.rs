use super::{consent, openai::OpenAiProvider, types::*};
use crate::domain::{AppError, Result};
use serde_json::{Value, json};

pub const VISION_PROMPT_VERSION: &str = "m3-vision-1";

/// 一次视觉工具调用的预取结果：缓存命中零上传；未命中部分经 HTTP 观察取得。
/// 预取只持读事务与网络，写事务在 tools::execute 内完成配额消耗与观察落库。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VisionPrefetch {
    pub kind: String,
    pub requested: Vec<String>,
    /// 每张照片一条观察；photo_id 均已验证属于请求集合。
    pub observations: Vec<(String, Value)>,
    /// 实际已向云端传输的图像数（含观察被丢弃的部分，授权配额按此扣减）。
    pub uploaded: u32,
    pub tokens: u32,
    pub consent_id: i64,
    pub cost_minor_usd: u32,
    pub model_id: String,
}
pub(crate) fn is_vision_tool(name: &str) -> bool {
    name == "inspect_photos" || name == "compare_photos"
}
pub(crate) async fn prefetch(
    db: &crate::db::Store,
    provider: &OpenAiProvider,
    task: &Task,
    call: &ToolCall,
    region: &super::router::RegionPolicy,
    tier: super::router::ServiceTier,
) -> Result<Option<VisionPrefetch>> {
    use sqlx::Row;
    if let Some(data) = super::cloud::replay(db, &task.id, call).await? {
        return Ok(Some(data));
    }
    let completed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls WHERE task_id=? AND call_id=?")
            .bind(&task.id)
            .bind(&call.call_id)
            .fetch_one(&db.pool)
            .await?;
    if completed > 0 {
        return Ok(None);
    }
    let ids: Vec<String> = serde_json::from_value(call.args["photo_ids"].clone())
        .map_err(crate::command_audit::serialization)?;
    if ids.is_empty() {
        return Err(AppError::new("VALIDATION_ERROR", "视觉工具需要照片"));
    }
    let mut tx = db.pool.begin().await?;
    // 无效工具交回统一工具通道记录拒绝；预取不能先产生网络副作用。
    if super::policy::PolicyGate::validate(&mut tx, task, call)
        .await
        .is_err()
    {
        return Ok(None);
    }
    let rows = sqlx::query(
        "SELECT p.id,p.content_sha256,p.preview_path FROM photos p WHERE p.project_id=?",
    )
    .bind(&task.project_id)
    .fetch_all(&mut *tx)
    .await?;
    let scoped: Vec<(String, String, String)> = rows
        .iter()
        .filter(|r| {
            let id: String = r.get("id");
            task.allowed_photo_ids.contains(&id)
        })
        .filter(|r| ids.iter().any(|want| want == &r.get::<String, _>("id")))
        .map(|r| (r.get("id"), r.get("content_sha256"), r.get("preview_path")))
        .collect();
    if scoped.len() != ids.len() {
        // 越权或不存在 ID 交由写事务的 PolicyGate 拒绝；此处不消耗上传。
        tx.commit().await?;
        return Ok(None);
    }
    let model_id = provider.model_id();
    let mut cached: Vec<(String, Value)> = vec![];
    let mut missing: Vec<(String, String, String)> = vec![];
    for photo in &scoped {
        let hit: Option<String> = sqlx::query_scalar(
            "SELECT observation_json FROM photo_observations WHERE photo_hash=? AND model_id=? AND prompt_version=? AND kind=? ORDER BY id DESC LIMIT 1",
        )
        .bind(&photo.1)
        .bind(model_id)
        .bind(VISION_PROMPT_VERSION)
        .bind(call.name.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        match hit {
            Some(raw) => {
                let value: Value =
                    serde_json::from_str(&raw).map_err(crate::command_audit::serialization)?;
                cached.push((photo.0.clone(), value));
            }
            None => missing.push(photo.clone()),
        }
    }
    tx.commit().await?;
    if missing.is_empty() {
        return Ok(Some(VisionPrefetch {
            kind: call.name.clone(),
            requested: ids,
            observations: cached,
            uploaded: 0,
            tokens: 0,
            consent_id: 0,
            cost_minor_usd: 0,
            model_id: model_id.to_owned(),
        }));
    }
    // 授权在预取阶段只读校验；配额扣减在写事务，同项目唯一执行任务避免并发超用。
    let mut read = db.pool.begin().await?;
    let authorized = consent::authorize(&mut read, task, missing.len() as i64).await?;
    read.commit().await?;
    let Some(_) = authorized else {
        return Ok(None);
    };
    super::router::route_for_provider(
        &super::router::RouteRequest {
            task_type: super::router::TaskType::VisionObserve,
            tier,
            region,
            budget_remaining_usd: super::router::remaining_budget(task, tier),
            images_required: true,
            image_consent: true,
        },
        &task.provider_id,
        true,
    )?;
    if task.usage.cloud_images + missing.len() as u32 > task.budget.max_cloud_images {
        return Err(AppError::new(
            "BUDGET_EXCEEDED",
            "云端图像预算不足，请调整预算或减少照片数",
        ));
    }
    let brief = project_brief(db, task).await?;
    let mut images = vec![];
    for photo in &missing {
        let data = std::fs::read(&photo.2)
            .map_err(|e| AppError::new("RESOURCE_MISSING", format!("预览缓存缺失：{e}")))?;
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data);
        images.push((photo.0.clone(), encoded));
    }
    let question = match call.name.as_str() {
        "compare_photos" => format!(
            "对照这几张照片与项目意图的差异与不确定性。{}",
            call.args["brief"].as_str().unwrap_or_default()
        ),
        _ => "描述每张照片的可观察事实与它在项目中可能的编辑作用。".into(),
    };
    let uploaded = missing.len() as u32;
    let input_bound = (brief.len()
        + question.len()
        + include_str!("../../../prompts/vision-observe-m3.md").len()
        + 512) as u32
        + uploaded * 4096;
    let upper_cost = uploaded * super::openai::IMAGE_COST_MINOR_USD
        + super::openai::text_cost_minor_usd(
            input_bound.saturating_add(task.budget.max_output_tokens),
        );
    let handler = crate::command_handler::CommandHandler::new(db);
    let (consent_id, remaining) = handler
        .reserve_vision(&task.id, call, uploaded, upper_cost)
        .await?;
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(remaining),
        provider.observe(&brief, &question, &images, task.budget.max_output_tokens),
    )
    .await;
    let (mut observation, tokens) = match result {
        Ok(Ok(value)) => value,
        other => {
            handler
                .settle_vision(
                    &task.id,
                    call,
                    None,
                    None,
                    started.elapsed().as_millis() as u64,
                )
                .await?;
            return Err(match other {
                Ok(Err(error)) => error,
                _ => AppError::new(
                    "BUDGET_EXCEEDED",
                    "视觉等待达到时间上限；已保留上传预留，不自动重传",
                ),
            });
        }
    };
    let cost =
        uploaded * super::openai::IMAGE_COST_MINOR_USD + super::openai::text_cost_minor_usd(tokens);
    let list = observation["observations"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let valid: Vec<(String, Value)> = list
        .into_iter()
        .filter_map(|item| {
            let id = item["photo_id"].as_str()?.to_owned();
            missing
                .iter()
                .any(|m| m.0 == id)
                .then(|| (id.clone(), item))
        })
        .collect();
    if valid.is_empty() {
        // 传输已实际发生：授权配额按真实传输扣减（隐私语义），再报告失败。
        handler
            .settle_vision(
                &task.id,
                call,
                None,
                Some(cost),
                started.elapsed().as_millis() as u64,
            )
            .await?;
        return Err(AppError::new(
            "MODEL_UNAVAILABLE",
            "视觉观察未引用任何请求照片，结果被丢弃",
        ));
    }
    observation["observations"] = json!(valid.iter().map(|(_, v)| v).collect::<Vec<_>>());
    let data = VisionPrefetch {
        kind: call.name.clone(),
        requested: ids,
        observations: [cached, valid].concat(),
        uploaded,
        tokens,
        consent_id,
        cost_minor_usd: cost,
        model_id: model_id.to_owned(),
    };
    handler
        .settle_vision(
            &task.id,
            call,
            Some(&data),
            Some(cost),
            started.elapsed().as_millis() as u64,
        )
        .await?;
    Ok(Some(data))
}
async fn project_brief(db: &crate::db::Store, task: &Task) -> Result<String> {
    let brief: String =
        sqlx::query_scalar("SELECT brief FROM projects WHERE id=? AND deleted_at IS NULL")
            .bind(&task.project_id)
            .fetch_optional(&db.pool)
            .await?
            .unwrap_or_default();
    Ok(brief)
}
