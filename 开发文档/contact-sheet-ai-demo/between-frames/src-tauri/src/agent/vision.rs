use super::{consent, openai::OpenAiProvider, types::*};
use crate::domain::{AppError, Result};
use serde_json::{Value, json};

// 观察缓存身份包含像素准备方式，不能复用旧 1400px 或未标记的 512px 观察。
pub const DETAIL_PROMPT_VERSION: &str = "m3-vision-detail1400-q90-v1";
pub(crate) fn cache_version_for_tool(
    identity: &super::model_binding::ModelIdentity,
    tool: &str,
) -> String {
    if tool == "inspect_photo_detail" {
        format!("{}@{}", DETAIL_PROMPT_VERSION, identity.endpoint_sha256)
    } else {
        identity.cache_version()
    }
}
pub(crate) fn detail_cache(version: &str) -> bool {
    version.starts_with(&format!("{DETAIL_PROMPT_VERSION}@"))
}
pub const VISION_PROMPT_VERSION: &str = "m3-vision-5-upload512-q75";

/// 一次视觉工具调用的预取结果：缓存命中零上传；未命中部分经 HTTP 观察取得。
/// 预取只持读事务与网络，写事务在 tools::execute 内完成配额消耗与观察落库。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VisionPrefetch {
    pub kind: String,
    pub requested: Vec<String>,
    /// 每张照片一条观察；photo_id 均已验证属于请求集合。
    pub observations: Vec<(String, Value)>,
    /// 每条观察对应的像素版本。旧回执里没有这个字段，投影按"版本未记录"处理。
    #[serde(default)]
    pub image_versions: Vec<ObservationVersion>,
    /// 实际已向云端传输的图像数（含观察被丢弃的部分，授权配额按此扣减）。
    pub uploaded: u32,
    /// 传过去了但模型没有给出观察的照片。传输确实发生了，配额也照实扣了，可这几张没人看过画面，
    /// 不能混进"已观察"，也不能拿去推算缓存命中。
    #[serde(default)]
    pub dropped: Vec<String>,
    /// 本地预览读不出来、一个字节都没送出去的照片。它们既不算看过，也不占额度，
    /// 但和"送出去了没被回答"是两回事，得分开说。
    #[serde(default)]
    pub unreadable: Vec<String>,
    /// 本地预览坏掉、但已经从原图就地重算出像素并送出去的那几张。原图只读：
    /// 重算只写系统临时目录里的一份随机文件，用完即删，图库缓存不动。
    #[serde(default)]
    pub rebuilt: Vec<String>,
    /// 没能送出去的每张照片各自的原因，与 `unreadable` 同集合。
    #[serde(default)]
    pub unreadable_reasons: Vec<(String, String)>,
    pub tokens: u32,
    pub consent_id: i64,
    pub cost_minor_usd: u32,
    pub model_id: String,
    #[serde(default)]
    pub cache_version: String,
}
pub(crate) fn is_vision_tool(name: &str) -> bool {
    name == "inspect_photos" || name == "compare_photos" || name == "inspect_photo_detail"
}

/// 一条观察是在哪一份像素上做出来的。`preview_sha256` 是当时交给模型的字节指纹，
/// 不是原图指纹，也不是磁盘上预览文件的路径。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObservationVersion {
    pub photo_id: String,
    #[serde(default)]
    pub preview_sha256: Option<String>,
    /// verified=与当前要发送的字节核对过；unrecorded=旧缓存从未记录版本；unreadable=当前预览读不出来，没法核对。
    pub status: String,
}

/// 核对历史观察与当前派生像素；只读本地预览，不触发模型或重新上传。
pub(crate) async fn current_preview_status(recorded: Option<&str>, path: &str) -> &'static str {
    current_preview_status_for_version(recorded, path, "").await
}
pub(crate) async fn current_preview_status_for_version(
    recorded: Option<&str>,
    path: &str,
    version: &str,
) -> &'static str {
    let detail = detail_cache(version);
    let Some(recorded) = recorded else {
        return "unrecorded";
    };
    let path = std::path::PathBuf::from(path);
    let current = tokio::task::spawn_blocking(move || {
        super::upload_compress::compress_profile(&path, detail)
    })
    .await;
    match current {
        Ok(Ok((_, hash))) if hash == recorded => "verified",
        Ok(Ok(_)) => "mismatch",
        _ => "unreadable",
    }
}

/// 一轮评估最多哈希几张原图：原图指纹要读整个文件（大 RAW 几百 MB），而这段跑在写事务里，
/// 排不上的照实记"没查"，不许把整库锁住十几秒。
pub(crate) const MAX_ORIGINAL_CHECKS: usize = 8;

/// 被排除的那张，本地修预览到底救不救得回来：原图不在本机是零成本判死；原图在就哈希一遍
/// 和库里记的内容指纹比——对不上说明盘上换了内容，修复一律拒绝写，也救不回来。
/// 哈希读不出来或排不上名额都记 `unchecked`：不知道就说不知道，不替它猜一个结论。
pub(crate) async fn original_status(
    path: &str,
    recorded: &str,
    remaining: &mut usize,
) -> &'static str {
    let file = std::path::PathBuf::from(path);
    if !file.is_file() {
        return "missing";
    }
    if *remaining == 0 {
        return "unchecked";
    }
    *remaining -= 1;
    let recorded = recorded.to_string();
    match tokio::task::spawn_blocking(move || crate::image_pipeline::fingerprint(&file)).await {
        Ok(Ok(hash)) if hash == recorded => "matches",
        Ok(Ok(_)) => "changed",
        _ => "unchecked",
    }
}

/// 一张候选照片这一次要交给模型的字节。预览读不出来时留空：
/// 命中缓存的照片不必因此重传，确实需要上传的照片再由原本的错误如实报出。
#[derive(Clone)]
struct Prepared {
    photo_id: String,
    source_hash: String,
    original_path: String,
    preview_path: String,
    bytes: Option<Vec<u8>>,
    preview_sha: Option<String>,
}

/// 一批里最多从原图重算几份像素。重算要走完整解码（大 RAW 可能几秒），
/// 一次观察不能因为缓存坏了就把整轮对话拖住，超出上限的照实说"没来得及"。
const MAX_PREVIEW_REBUILDS: usize = 4;

/// 本地预览读不出来时的补救结果。`Rebuilt` 是这一张真正可以送出去的字节。
enum Repair {
    Rebuilt(Vec<u8>, String),
    Unusable(&'static str),
}

/// 只用原图重算一份本次观察要用的像素。原图只读：解出来的缓存写进系统临时目录里
/// 一个随机命名的目录，用完就删；图库缓存、原图、数据库都不动。
/// 内容与库里记录的指纹对不上时一律不用——那已经是另一张照片，宁可如实说读不出来。
fn repair_preview_from_original(photo: &Prepared, detail: bool) -> Repair {
    let original = std::path::Path::new(&photo.original_path);
    if !original.is_file() {
        return Repair::Unusable("original_missing");
    }
    let staging = std::env::temp_dir().join(format!(
        "between-frames-preview-repair-{}",
        crate::domain::id()
    ));
    let built = crate::image_pipeline::prepare(original, &staging);
    let rebuilt = match built {
        Ok(prepared) if prepared.hash != photo.source_hash => Repair::Unusable("original_changed"),
        Ok(prepared) => match super::upload_compress::compress_profile(&prepared.preview, detail) {
            Ok(bytes) => Repair::Rebuilt(bytes.0, bytes.1),
            Err(_) => Repair::Unusable("rebuild_failed"),
        },
        Err(_) => Repair::Unusable("original_unreadable"),
    };
    let _ = std::fs::remove_dir_all(&staging);
    rebuilt
}
pub(crate) fn valid_cached_observation(value: &Value, photo: &str) -> bool {
    value["photo_id"] == photo
        && super::openai::valid_observations(&json!({"observations":[value]}))
}
pub(crate) async fn prefetch(
    db: &crate::db::Store,
    provider: &OpenAiProvider,
    task: &Task,
    call: &ToolCall,
    region: &super::router::RegionPolicy,
    tier: super::router::ServiceTier,
    quality: &[super::router::QualityRecord],
) -> Result<Option<VisionPrefetch>> {
    let detail = call.name == "inspect_photo_detail";
    let cache_kind = if detail {
        "inspect_photos"
    } else {
        call.name.as_str()
    };
    let bound_provider = provider.bind_account()?;
    let provider = &bound_provider;
    use sqlx::Row;
    let mut binding_tx = db.pool.begin().await?;
    let binding = super::model_binding::load(&mut binding_tx, &task.id).await?;
    binding_tx.commit().await?;
    if binding.map(|bound| bound.observation_identity()) != Some(provider.identity()) {
        return Err(AppError::new(
            "RECOVERY_ERROR",
            "图像观察模型与任务绑定不一致；请重新委托，未上传",
        ));
    }
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
        "SELECT p.id,p.content_sha256,p.preview_path,p.original_path FROM photos p WHERE p.project_id=?",
    )
    .bind(&task.project_id)
    .fetch_all(&mut *tx)
    .await?;
    let scoped: Vec<(String, String, String, String)> = rows
        .iter()
        .filter(|r| {
            let id: String = r.get("id");
            task.allowed_photo_ids.contains(&id)
        })
        .filter(|r| ids.iter().any(|want| want == &r.get::<String, _>("id")))
        .map(|r| {
            (
                r.get("id"),
                r.get("content_sha256"),
                r.get("preview_path"),
                r.get("original_path"),
            )
        })
        .collect();
    if scoped.len() != ids.len() {
        // 越权或不存在 ID 交由写事务的 PolicyGate 拒绝；此处不消耗上传。
        tx.commit().await?;
        return Ok(None);
    }
    let model_id = provider.model_id();
    let cache_version = cache_version_for_tool(&provider.identity(), &call.name);
    // 缓存能否复用，取决于"现在要发的字节"和"当初发出去的字节"是不是同一份：
    // 原图指纹不变，预览也可能因为换解码器、修过透明合成或重新关联文件而换了版本，那时旧观察说的是另一张图。
    let mut prepared: Vec<Prepared> = vec![];
    for photo in &scoped {
        let (bytes, preview_sha) = match super::upload_compress::compress_profile(
            std::path::Path::new(&photo.2),
            detail,
        ) {
            Ok(value) => (Some(value.0), Some(value.1)),
            Err(_) => (None, None),
        };
        prepared.push(Prepared {
            photo_id: photo.0.clone(),
            source_hash: photo.1.clone(),
            original_path: photo.3.clone(),
            preview_path: photo.2.clone(),
            bytes,
            preview_sha,
        });
    }
    let mut cached: Vec<(String, Value)> = vec![];
    let mut versions: Vec<ObservationVersion> = vec![];
    let mut missing: Vec<usize> = vec![];
    for (index, photo) in prepared.iter().enumerate() {
        let row: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT observation_json,preview_sha256 FROM photo_observations WHERE photo_id=? AND photo_hash=? AND model_id=? AND prompt_version=? AND kind=? ORDER BY id DESC LIMIT 1",
        )
        .bind(&photo.photo_id)
        .bind(&photo.source_hash)
        .bind(model_id)
        .bind(&cache_version)
        .bind(cache_kind)
        .fetch_optional(&mut *tx)
        .await?;
        // 旧缓存也经过同一契约校验；无效项视为未命中，仍需授权/预算才能重新观察。
        let reused = row.and_then(|(raw, recorded)| {
            let value: Value = serde_json::from_str(&raw).ok()?;
            if !valid_cached_observation(&value, &photo.photo_id) {
                return None;
            }
            let (preview_sha256, status) = match (recorded.as_deref(), photo.preview_sha.as_deref())
            {
                (Some(stored), Some(current)) if stored == current => {
                    (Some(stored.to_owned()), "verified")
                }
                // 记录过版本而当前字节对不上：这份观察描述的是另一张图，必须重新观察。
                (Some(_), Some(_)) => return None,
                (Some(stored), None) => (Some(stored.to_owned()), "unreadable"),
                (None, _) => (None, "unrecorded"),
            };
            Some((value, preview_sha256, status.to_owned()))
        });
        match reused {
            Some((value, preview_sha256, status)) => {
                versions.push(ObservationVersion {
                    photo_id: photo.photo_id.clone(),
                    preview_sha256,
                    status,
                });
                cached.push((photo.photo_id.clone(), value));
            }
            None => missing.push(index),
        }
    }
    tx.commit().await?;
    if missing.is_empty() {
        return Ok(Some(VisionPrefetch {
            kind: call.name.clone(),
            requested: ids,
            observations: cached,
            image_versions: versions,
            uploaded: 0,
            dropped: vec![],
            unreadable: vec![],
            rebuilt: vec![],
            unreadable_reasons: vec![],
            tokens: 0,
            consent_id: 0,
            cost_minor_usd: 0,
            model_id: model_id.to_owned(),
            cache_version,
        }));
    }
    let mut read = db.pool.begin().await?;
    let allow_queued_recovery = crate::focal_chat::load(&mut read, &task.id)
        .await?
        .is_some_and(|c| c.allow_queued_recovery);
    read.commit().await?;
    if allow_queued_recovery {
        provider.validate_hosted_options(true).await?;
    }
    if detail {
        provider.require_detail_support().await?;
    }
    // 没命中缓存的那些张要真读一次本地预览。读不出来的先试"只读原图重算一份"：
    // 坏的是缓存，不是这张照片。重算不成的只降级那一张，不为它预留额度，
    // 也不让整批观察陪着失败。
    let mut sendable: Vec<usize> = vec![];
    let mut unreadable: Vec<String> = vec![];
    let mut unreadable_reasons: Vec<(String, String)> = vec![];
    let mut rebuilt: Vec<String> = vec![];
    let mut read_failure: Option<AppError> = None;
    let mut images: Vec<(String, String)> = vec![];
    let mut sent: Vec<ObservationVersion> = vec![];
    let mut repairs = 0usize;
    for index in &missing {
        let photo = &prepared[*index];
        // 指纹跟着真正交出去的字节走：缓存那份对不上时重新压缩，这次观察就绑在新版本上。
        let bytes = match (photo.bytes.clone(), photo.preview_sha.clone()) {
            (Some(bytes), Some(preview_sha)) => Ok((bytes, preview_sha)),
            _ => match super::upload_compress::compress_profile(
                std::path::Path::new(&photo.preview_path),
                detail,
            ) {
                Ok(value) => Ok(value),
                Err(error) => {
                    if read_failure.is_none() {
                        read_failure = Some(error);
                    }
                    if repairs >= MAX_PREVIEW_REBUILDS {
                        Err("rebuild_over_limit".to_owned())
                    } else {
                        repairs += 1;
                        let candidate = photo.clone();
                        match tokio::task::spawn_blocking(move || {
                            repair_preview_from_original(&candidate, detail)
                        })
                        .await
                        {
                            Ok(Repair::Rebuilt(bytes, preview_sha)) => {
                                rebuilt.push(photo.photo_id.clone());
                                Ok((bytes, preview_sha))
                            }
                            Ok(Repair::Unusable(reason)) => Err(reason.to_owned()),
                            Err(_) => Err("rebuild_failed".to_owned()),
                        }
                    }
                }
            },
        };
        match bytes {
            Ok((bytes, preview_sha)) => {
                sendable.push(*index);
                images.push((
                    photo.photo_id.clone(),
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes),
                ));
                sent.push(ObservationVersion {
                    photo_id: photo.photo_id.clone(),
                    preview_sha256: Some(preview_sha),
                    status: "verified".to_owned(),
                });
            }
            Err(reason) => {
                unreadable.push(photo.photo_id.clone());
                unreadable_reasons.push((photo.photo_id.clone(), reason));
            }
        }
    }
    let uploaded = images.len() as u32;
    if uploaded == 0 {
        // 一张都没读出来：保持原有报错口径，一个字节都不发出去。
        return Err(
            read_failure.unwrap_or_else(|| AppError::new("RESOURCE_MISSING", "照片预览不可读"))
        );
    }
    super::budget::vision_limits(task, uploaded, 0, u32::MAX)?;
    // 授权在预取阶段只读校验；配额扣减在写事务，同项目唯一执行任务避免并发超用。
    let mut read = db.pool.begin().await?;
    let authorized = consent::authorize(&mut read, task, uploaded as i64).await?;
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
            quality_scores: quality,
        },
        &task.provider_id,
        Some(provider.route()?),
    )?;
    let brief = project_brief(db, task).await?;
    let language_reference = language_reference(db, task, &brief).await?;
    // 用户原话决定表达语言；工具生成的比较要求与宿主包装不能覆盖它。
    let question = json!({
        "language_reference": language_reference,
        "operation": call.name,
        "comparison_focus": call.args.get("brief").and_then(Value::as_str),
    })
    .to_string();
    let input_bound = (brief.len()
        + question.len()
        + include_str!("../../../prompts/vision-observe-m3.md").len()
        + 512) as u32
        + uploaded * if detail { 16384 } else { 4096 };
    let rate = provider.rate_card();
    let cost_limit = (tier.max_cost_usd_per_task() * 100.0) as u32;
    let output_limit =
        super::budget::vision_output_limit(task, uploaded, input_bound, cost_limit, rate)?;
    let upper_cost = rate.vision_reservation(uploaded, input_bound, output_limit);
    let mut body = provider.observation_body(&brief, &question, &images, output_limit)?;
    if detail {
        for item in body["input"][0]["content"].as_array_mut().unwrap() {
            if item["type"] == "input_image" {
                item["detail"] = "high".into();
            }
        }
    }
    let receipt = provider
        .is_hosted()
        .then(|| super::vision_recovery::VisionReceipt {
            allow_queued_recovery,
            request_id: uuid::Uuid::new_v4().to_string(),
            identity: provider.identity(),
            body_sha256: crate::command_audit::fingerprint(&body),
            photo_hashes: scoped.iter().map(|p| (p.0.clone(), p.1.clone())).collect(),
            output_limit,
            prefetch: VisionPrefetch {
                kind: call.name.clone(),
                requested: ids.clone(),
                observations: cached.clone(),
                image_versions: [versions.clone(), sent.clone()].concat(),
                uploaded,
                // 这份回执在响应回来之前就写好了，此时还不知道模型会漏答哪几张；
                // 本地读不出像素的那几张在发出前已经点名，可以照实带上。
                dropped: vec![],
                unreadable: unreadable.clone(),
                rebuilt: rebuilt.clone(),
                unreadable_reasons: unreadable_reasons.clone(),
                tokens: 0,
                consent_id: 0,
                cost_minor_usd: upper_cost,
                model_id: model_id.into(),
                cache_version: cache_version.clone(),
            },
        });
    let handler = crate::command_handler::CommandHandler::new(db);
    let (consent_id, remaining) = handler
        .reserve_vision(
            &task.id,
            call,
            super::cloud::VisionReservation {
                images: uploaded,
                cost: upper_cost,
                cost_limit,
                rate,
                receipt: receipt.clone(),
            },
        )
        .await?;
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(std::time::Duration::from_millis(remaining), async {
        match &receipt {
            Some(receipt) => {
                provider
                    .observe_reserved(
                        &body,
                        &receipt.request_id,
                        receipt.allow_queued_recovery,
                        super::vision_recovery::before_dispatch(
                            db,
                            &task.id,
                            call,
                            &receipt.request_id,
                        ),
                    )
                    .await
            }
            None => provider.observe_prepared(&body).await,
        }
    })
    .await;
    let (mut observation, tokens) = match result {
        Ok(Ok(value)) => value,
        other => {
            if receipt.is_some() {
                let error = match other {
                    Ok(Err(error)) => error,
                    _ => AppError::new(
                        "AI_RESULT_UNKNOWN",
                        "照片分析仍未确认，请查询处理结果，勿重复发送",
                    ),
                };
                super::cloud::transport_failed(
                    db,
                    &task.id,
                    call,
                    &error,
                    started.elapsed().as_millis() as u64,
                )
                .await?;
                return Err(AppError::new(
                    "AI_RESULT_UNKNOWN",
                    "照片分析结果尚未确认，请查询处理结果，勿重复发送",
                ));
            }
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
    let cost = rate.vision_cost(uploaded, tokens);
    let list = observation["observations"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let valid: Vec<(String, Value)> = list
        .into_iter()
        .filter_map(|item| {
            let id = item["photo_id"].as_str()?.to_owned();
            sendable
                .iter()
                .any(|index| prepared[*index].photo_id == id)
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
    // 模型可以只回答其中几张：那些确实传过去了、却没被回答的，要单独点名，
    // 否则请求列表会把"上传过"冒充成"看过"。
    let dropped: Vec<String> = sendable
        .iter()
        .map(|index| prepared[*index].photo_id.clone())
        .filter(|id| !valid.iter().any(|(observed, _)| observed == id))
        .collect();
    let data = VisionPrefetch {
        kind: call.name.clone(),
        requested: ids,
        observations: [cached, valid].concat(),
        image_versions: [versions, sent].concat(),
        uploaded,
        dropped,
        unreadable,
        rebuilt,
        unreadable_reasons,
        tokens,
        consent_id,
        cost_minor_usd: cost,
        model_id: model_id.to_owned(),
        cache_version,
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

async fn language_reference(db: &crate::db::Store, task: &Task, brief: &str) -> Result<String> {
    // 只取当前任务的人工消息，不从助手回复、照片备注或其他项目猜测语言。
    let latest: Option<String> = sqlx::query_scalar(
        "SELECT body FROM task_messages WHERE task_id=? AND kind IN ('user_goal','clarification_answer') AND length(trim(body))>0 ORDER BY id DESC LIMIT 1",
    )
    .bind(&task.id)
    .fetch_optional(&db.pool)
    .await?;
    Ok(latest
        .as_deref()
        .into_iter()
        .chain(task.answer.as_deref())
        .chain([task.goal.as_str(), brief])
        .map(str::trim)
        .find(|text| !text.is_empty())
        .unwrap_or_default()
        .to_owned())
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_ORIGINAL_CHECKS, Prepared, Repair, VisionPrefetch, original_status,
        repair_preview_from_original,
    };
    use serde_json::json;

    /// 原图不在本机是零成本判死：不该花掉一个哈希名额（他当下整个真实项目都落这一格）。
    #[tokio::test]
    async fn missing_original_is_judged_without_spending_a_hash_budget() {
        let temp = tempfile::tempdir().unwrap();
        let gone = temp.path().join("gone.jpg");
        let mut remaining = MAX_ORIGINAL_CHECKS;
        assert_eq!(
            original_status(&gone.to_string_lossy(), "anything", &mut remaining).await,
            "missing"
        );
        assert_eq!(remaining, MAX_ORIGINAL_CHECKS, "判不在本机不该读文件");
    }

    /// 原图在就哈希一遍和库里记的内容指纹比：对得上修好预览还有救，对不上修复一律拒绝写。
    #[tokio::test]
    async fn present_original_is_hashed_against_the_recorded_fingerprint() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("original.jpg");
        std::fs::write(&file, b"pretend these are raw bytes").unwrap();
        let path = file.to_string_lossy().to_string();
        let recorded = crate::image_pipeline::fingerprint(&file).unwrap();
        let mut remaining = MAX_ORIGINAL_CHECKS;
        assert_eq!(
            original_status(&path, &recorded, &mut remaining).await,
            "matches"
        );
        assert_eq!(remaining, MAX_ORIGINAL_CHECKS - 1);
        // 盘上换了内容、库里还记着旧指纹：这正是能落进排除名单的那种"原图换过"。
        std::fs::write(&file, b"different bytes entirely").unwrap();
        assert_eq!(
            original_status(&path, &recorded, &mut remaining).await,
            "changed"
        );
        assert_eq!(remaining, MAX_ORIGINAL_CHECKS - 2);
    }

    /// 名额用光就说"没查"，不许把整库锁在写事务里十几秒，也不许替它猜一个结论。
    #[tokio::test]
    async fn exhausted_budget_says_unchecked_instead_of_guessing() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("original.jpg");
        std::fs::write(&file, b"bytes").unwrap();
        let mut remaining = 0;
        assert_eq!(
            original_status(&file.to_string_lossy(), "anything", &mut remaining).await,
            "unchecked"
        );
        assert_eq!(remaining, 0);
    }

    /// 崩溃恢复读回来的是上一版程序写下的回执，那里面还没有这两份名单。
    #[test]
    fn receipts_written_before_the_drop_list_existed_still_replay() {
        let legacy = json!({
            "kind":"inspect_photos",
            "requested":["p1","p2"],
            "observations":[["p1",{"photo_id":"p1"}]],
            "image_versions":[],
            "uploaded":2,
            "tokens":10,
            "consent_id":3,
            "cost_minor_usd":2,
            "model_id":"m",
            "cache_version":"c"
        })
        .to_string();
        let replayed: VisionPrefetch = serde_json::from_str(&legacy).unwrap();
        assert!(replayed.dropped.is_empty(), "旧回执不能凭空多出漏答的照片");
        assert!(
            replayed.unreadable.is_empty(),
            "旧回执不能凭空多出读不出的照片"
        );
        assert!(
            replayed.rebuilt.is_empty(),
            "旧回执不能凭空多出'已经从原图重算过'的照片"
        );
        assert_eq!(replayed.uploaded, 2, "旧回执里的上传数照用");
    }

    #[test]
    fn the_two_gap_lists_survive_the_receipt_round_trip() {
        let mut prefetch = serde_json::from_str::<VisionPrefetch>(
            &json!({"kind":"compare_photos","requested":["p1","p2"],"observations":[],"uploaded":2,
                   "tokens":0,"consent_id":0,"cost_minor_usd":0,"model_id":"m","cache_version":"c"})
            .to_string(),
        )
        .unwrap();
        prefetch.dropped = vec!["p2".to_owned()];
        prefetch.unreadable = vec!["p3".to_owned()];
        prefetch.rebuilt = vec!["p4".to_owned()];
        prefetch.unreadable_reasons = vec![("p3".to_owned(), "original_missing".to_owned())];
        let stored: VisionPrefetch =
            serde_json::from_str(&serde_json::to_string(&prefetch).unwrap()).unwrap();
        assert_eq!(stored.dropped, vec!["p2".to_owned()]);
        assert_eq!(stored.unreadable, vec!["p3".to_owned()]);
        // "从原图重算过"与"为什么还是没送出去"要能跟着崩溃恢复回执一起走回来。
        assert_eq!(stored.rebuilt, vec!["p4".to_owned()]);
        assert_eq!(
            stored.unreadable_reasons,
            vec![("p3".to_owned(), "original_missing".to_owned())]
        );
    }

    /// 重算只用原图，且只在原图确实还是这一张时才用。
    #[test]
    fn repair_reads_pixels_from_the_original_only_while_it_is_still_the_same_photo() {
        let temp = tempfile::tempdir().unwrap();
        let original = temp.path().join("a.jpg");
        image::RgbImage::from_pixel(32, 24, image::Rgb([10, 200, 30]))
            .save(&original)
            .unwrap();
        let built = crate::image_pipeline::prepare(&original, &temp.path().join("cache")).unwrap();
        let recorded = std::fs::read(&built.preview).unwrap();
        let photo = |path: &std::path::Path| Prepared {
            photo_id: "p1".to_owned(),
            source_hash: built.hash.clone(),
            original_path: path.to_string_lossy().into_owned(),
            preview_path: String::new(),
            bytes: None,
            preview_sha: None,
        };

        match repair_preview_from_original(&photo(&original), false) {
            Repair::Rebuilt(bytes, preview_sha) => {
                assert!(!bytes.is_empty(), "原图还在就要凑得出可送出的像素");
                assert_eq!(preview_sha.len(), 64);
            }
            Repair::Unusable(reason) => panic!("原图完好时不该失败：{reason}"),
        }

        match repair_preview_from_original(&photo(&temp.path().join("gone.jpg")), false) {
            Repair::Unusable(reason) => assert_eq!(reason, "original_missing"),
            Repair::Rebuilt(_, _) => panic!("原图不在机器上不能凭空造出像素"),
        }

        // 同一个路径、同一份文件名，内容已经不是当初那张了。
        image::RgbImage::from_pixel(32, 24, image::Rgb([99, 99, 99]))
            .save(&original)
            .unwrap();
        match repair_preview_from_original(&photo(&original), false) {
            Repair::Unusable(reason) => assert_eq!(reason, "original_changed"),
            Repair::Rebuilt(_, _) => panic!("内容变了还拿来观察，就是把另一张当成这张"),
        }

        assert_eq!(
            std::fs::read(&built.preview).unwrap(),
            recorded,
            "重算不改动图库里那份缓存"
        );
    }
}
