//! 单张判断的依据链：只引用本任务已取得的观察与方法，不认证模型推理的正确性。
//!
//! 拒绝必须指名断掉的那一环。依据链有七八种断法（照片不在本次范围、回执根本不存在、回执只有元数据、
//! 事实索引越界、引用编号是编的、云端摘要里没有断言、像素版本对不上），全部折叠成同一句话时，
//! 模型只能原样重试并继续消耗预算，用户也只看到"没有判断卡片"而不知道缺哪一步。
//! 消息里不回填模型提供的字符串、不带文件路径，只说明这一环缺什么、下一步该调哪个工具。
use super::types::{Task, ToolResult};
use crate::{
    command_audit::serialization,
    domain::{AppError, Result},
};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};

/// 单份回执的读取上限：超过它的内容不再被静默报成"没有这条回执"。
const RECEIPT_BYTES: i64 = 65536;

/// 同一条读取失败在观察依据和方法依据上意味着不同的下一步，所以按来路给出各自的说法。
#[derive(Clone, Copy)]
enum Purpose {
    Observation,
    Method,
}

impl Purpose {
    fn missing(self) -> AppError {
        match self {
            Self::Observation => AppError::new(
                "JUDGMENT_RECEIPT_MISSING",
                "本任务里没有这个编号的工具回执；请先用 inspect_photos 观察这张照片，再引用它返回的编号。",
            ),
            Self::Method => AppError::new(
                "JUDGMENT_RECEIPT_MISSING",
                "本任务里没有这个编号的工具回执；方法只能引用本轮知识检索实际返回的编号。",
            ),
        }
    }
    fn oversized(self) -> AppError {
        AppError::new(
            "JUDGMENT_RECEIPT_TOO_LARGE",
            "这条回执超过单份读取上限，内容无法核对；请改用范围更小的检索或观察回执，重复引用同一条不会成功。",
        )
    }
    fn failed(self) -> AppError {
        AppError::new(
            "JUDGMENT_RECEIPT_FAILED",
            "这个编号对应的是一次失败的调用，不能作为依据；请先让该工具成功返回一次。",
        )
    }
}

fn bounded_method(mut value: Value) -> Value {
    let passage = value["passage"].as_str().unwrap_or("");
    let excerpt: String = passage.chars().take(600).collect();
    value["excerpt"] = json!(passage.chars().count() > 600);
    value["passage"] = json!(excerpt);
    value
}

async fn receipt(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    id: &str,
    purpose: Purpose,
) -> Result<(String, Value)> {
    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT tool_name,length(CAST(result_json AS BLOB)) FROM tool_calls WHERE task_id=? AND call_id=?",
    )
    .bind(&task.id).bind(id).fetch_optional(&mut **tx).await?;
    let Some((name, size)) = row else {
        return Err(purpose.missing());
    };
    if size > RECEIPT_BYTES {
        return Err(purpose.oversized());
    }
    let raw: String =
        sqlx::query_scalar("SELECT result_json FROM tool_calls WHERE task_id=? AND call_id=?")
            .bind(&task.id)
            .bind(id)
            .fetch_one(&mut **tx)
            .await?;
    let result: ToolResult = serde_json::from_str(&raw).map_err(serialization)?;
    if !result.ok || result.call_id != id {
        return Err(purpose.failed());
    }
    let Some(data) = result.result else {
        return Err(purpose.failed());
    };
    Ok((name, data))
}

async fn method(tx: &mut Transaction<'_, Sqlite>, task: &Task, reference: &Value) -> Result<Value> {
    let call = reference["call_id"].as_str().ok_or_else(|| {
        AppError::new("JUDGMENT_ARGS", "方法引用必须给出本轮一次工具调用的编号。")
    })?;
    let citation = reference["citation_id"].as_str().ok_or_else(|| {
        AppError::new(
            "JUDGMENT_ARGS",
            "方法引用必须给出那次回执里出现过的资料编号。",
        )
    })?;
    let (name, data) = receipt(tx, task, call, Purpose::Method).await?;
    if name == "search_photography_knowledge" && reference["claim_index"].is_null() {
        let found = data["results"]
            .as_array()
            .and_then(|items| items.iter().find(|i| i["citationId"] == citation));
        let Some(item) = found else {
            // 通用检索会把云端摘要一起带回，那些条目只有编号、没有断言正文。
            // "编号不存在"与"编号存在但要走专门工具"分开报，模型才知道下一步该调谁。
            let cloud_summary = data["results"]
                .as_array()
                .is_some_and(|items| items.iter().any(|i| i["id"] == citation));
            return Err(if cloud_summary {
                AppError::new(
                    "JUDGMENT_CLOUD_SUMMARY_ONLY",
                    "这个编号只是通用知识检索带回的云端摘要条目，回执里没有它的断言正文；要引用摄影师、摄影书或系列的断言，请先调用 search_photographers、get_photobook_structure 或 compare_photography_series 取得详情。",
                )
            } else {
                AppError::new(
                    "JUDGMENT_CITATION_UNKNOWN",
                    "这个编号不在那次检索实际返回的记录里；不能凭资料名称自行编造编号。",
                )
            });
        };
        if item["passage"].as_str().is_none_or(str::is_empty) {
            return Err(AppError::new(
                "JUDGMENT_METHOD_EMPTY",
                "这条原创方法回执里没有正文，不能作为引用来源。",
            ));
        }
        return Ok(bounded_method(
            json!({"callId":call,"citationId":citation,"title":item["title"],"passage":item["passage"],"source":item["source"],"author":item["author"],"license":item["license"],"version":item["version"],"kind":"original_method","reviewStatus":"original","disputed":false,"matchReason":item["matchReason"]}),
        ));
    }
    let index = reference["claim_index"].as_u64().ok_or_else(|| {
        AppError::new(
            "JUDGMENT_ARGS",
            "引用云端断言必须给出它在那次回执里的序号；原创方法填 null。",
        )
    })? as usize;
    if !super::knowledge::is_tool(&name) {
        return Err(AppError::new(
            "JUDGMENT_RECEIPT_NOT_KNOWLEDGE",
            "方法出处只能来自知识检索类工具的回执；照片观察、画布读取与记忆查询都不算方法来源。",
        ));
    }
    let entity = super::creative::knowledge_entities(&name, &data)
        .into_iter()
        .find(|e| e["id"] == citation)
        .ok_or_else(|| AppError::new(
            "JUDGMENT_CITATION_UNKNOWN",
            "这个编号不在那次检索实际返回的记录里；每次检索只给最前两条补取断言，请引用回执中出现过的编号。",
        ))?;
    if entity["detailError"].is_object() {
        return Err(AppError::new(
            "JUDGMENT_DETAIL_UNAVAILABLE",
            "这条记录那一次的断言详情没有取得，回执里没有可引用的正文；请重新检索后再引用新回执，摘要条目本身不算出处。",
        ));
    }
    let claims = entity["citations"]["claims"].as_array();
    let count = claims.as_ref().map_or(0, |claims| claims.len());
    let Some(claim) = claims.as_ref().and_then(|claims| claims.get(index)) else {
        let reason = if count == 0 {
            "那次回执里这条记录没有可引用的断言正文；请改用取得断言详情的检索回执，或把方法数组留空。".to_string()
        } else {
            format!(
                "那次回执里这条记录只有 {count} 条断言，序号从 0 到 {}；请填实际存在的序号。",
                count - 1
            )
        };
        return Err(AppError::new("JUDGMENT_CLAIM_INDEX", reason));
    };
    if claim["text"].as_str().is_none_or(str::is_empty) {
        return Err(AppError::new(
            "JUDGMENT_METHOD_EMPTY",
            "这条断言正文为空，不能作为引用来源。",
        ));
    }
    Ok(bounded_method(
        json!({"callId":call,"citationId":citation,"claimIndex":index,"title":entity["name"].as_str().or(entity["title"].as_str()).unwrap_or(citation),"passage":claim["text"],"source":claim["sources"][0]["url"],"institution":claim["sources"][0]["institution"],"kind":"retrieved_claim","reviewStatus":claim["reviewStatus"],"disputed":claim["disputed"].as_bool().unwrap_or(false)}),
    ))
}

pub async fn explain(tx: &mut Transaction<'_, Sqlite>, task: &Task, args: &Value) -> Result<Value> {
    let id = args["photo_id"]
        .as_str()
        .ok_or_else(|| AppError::new("JUDGMENT_ARGS", "photo_id 必须是照片编号字符串。"))?;
    if !task.allowed_photo_ids.iter().any(|p| p == id) {
        return Err(AppError::new(
            "JUDGMENT_SCOPE",
            "这张照片不在本次委托的照片范围内，不能为它建立判断依据。",
        ));
    }
    let observation_call = args["observation_call_id"].as_str().ok_or_else(|| {
        AppError::new(
            "JUDGMENT_ARGS",
            "observation_call_id 必须是本轮一次工具调用的编号。",
        )
    })?;
    let (name, data) = receipt(tx, task, observation_call, Purpose::Observation).await?;
    if !super::vision::is_vision_tool(&name) {
        return Err(AppError::new(
            "JUDGMENT_RECEIPT_NOT_VISION",
            "这个编号的回执不是照片观察；请引用 inspect_photos、compare_photos 或 inspect_photo_detail 的回执。",
        ));
    }
    if data["mode"] != "vision_observed" {
        return Err(AppError::new(
            "JUDGMENT_METADATA_ONLY",
            "这次观察回执只有文件元数据、没有看过画面；请先取得图像授权并成功观察一次，再来绑定依据。",
        ));
    }
    let observation = data["observations"].as_array()
        .and_then(|items| items.iter().find(|p| p["photo_id"] == id))
        .ok_or_else(|| AppError::new(
            "JUDGMENT_NOT_OBSERVED",
            "这次观察回执里没有这张照片的记录；请引用实际包含它的观察回执，或先单独观察这张照片。",
        ))?;
    if !super::vision::valid_cached_observation(observation, id) {
        return Err(AppError::new(
            "JUDGMENT_OBSERVATION_INVALID",
            "这张照片的观察记录不符合结构要求，不能当作可见事实的来源。",
        ));
    }
    let provenance = data["provenance"]
        .as_array()
        .and_then(|items| items.iter().find(|p| p["photo_id"] == id))
        .ok_or_else(|| {
            AppError::new(
                "JUDGMENT_PROVENANCE_MISSING",
                "这张照片的观察缺少像素来路记录，无法核对图像版本；请重新观察一次再绑定依据。",
            )
        })?;
    let mut value = json!({"photoId":id,"observationCallId":observation_call,"judgment":args["judgment"],"reasoning":args["reasoning"],"uncertainty":args["uncertainty"],"observationUncertainty":observation["uncertainty"],"provenance":provenance,"facts":[],"methods":[],"inference":true});
    match pixel_state(tx, task, &value).await? {
        Pixel::Verified => {}
        Pixel::Original => {
            return Err(AppError::new(
                "JUDGMENT_ORIGINAL_CHANGED",
                "自这次观察之后原图内容已经改变，旧观察不再对应当前画面；请重新观察这张照片。",
            ));
        }
        Pixel::Preview => {
            return Err(AppError::new(
                "JUDGMENT_PREVIEW_CHANGED",
                "这次观察交给模型的预览字节与当前缓存不一致；请重新观察这张照片，不要引用旧回执。",
            ));
        }
        Pixel::Unrecorded => {
            return Err(AppError::new(
                "JUDGMENT_PIXEL_UNRECORDED",
                "这次观察没有记录预览字节指纹（旧缓存），无法核对像素；请用本轮重新观察得到的回执。",
            ));
        }
        Pixel::Unreadable => {
            return Err(AppError::new(
                "JUDGMENT_PREVIEW_UNREADABLE",
                "现在读不出这次观察对应的派生预览，无法核对像素；请先恢复项目的预览缓存。",
            ));
        }
        Pixel::Missing => {
            return Err(AppError::new(
                "JUDGMENT_PHOTO_MISSING",
                "这张照片的记录已不在本项目中。",
            ));
        }
    }
    let indices = args["fact_indices"]
        .as_array()
        .ok_or_else(|| AppError::new("JUDGMENT_ARGS", "fact_indices 必须是事实序号数组。"))?;
    let visible = observation["visible_facts"].as_array();
    let count = visible.as_ref().map_or(0, |visible| visible.len());
    let mut facts = vec![];
    for index in indices {
        let i = index.as_u64().ok_or_else(|| {
            AppError::new("JUDGMENT_ARGS", "fact_indices 的每一项都要是整数序号。")
        })? as usize;
        let Some(text) = visible.as_ref().and_then(|visible| visible.get(i)) else {
            let reason = if count == 0 {
                "这次观察没有记录任何可见事实，无法逐条引用；请重新观察这张照片。".to_string()
            } else {
                format!(
                    "这次观察的可见事实共 {count} 条，序号从 0 到 {}；请只引用其中真实存在的条目。",
                    count - 1
                )
            };
            return Err(AppError::new("JUDGMENT_FACT_INDEX", reason));
        };
        facts.push(json!({"index":i,"text":text}));
    }
    let mut methods = vec![];
    for reference in args["knowledge_refs"].as_array().ok_or_else(|| {
        AppError::new(
            "JUDGMENT_ARGS",
            "knowledge_refs 必须是引用数组，没有出处时留空。",
        )
    })? {
        methods.push(method(tx, task, reference).await?);
    }
    value["facts"] = json!(facts);
    value["methods"] = json!(methods);
    value["imageStatusAtJudgment"] = json!("verified");
    value["notice"] =
        json!("回执绑定只证明引用来路，不证明观察、推理或审美判断正确；方法不是照片的视觉证据。");
    Ok(value)
}

/// 像素这一环到底断在哪。界面两侧的 `imageStatus` 只认四种已知值，所以展示用的字符串
/// 仍由这里折叠而来；拒绝理由需要更细的口径，否则"原图换了"和"预览字节变了"会被说成同一件事。
#[derive(Clone, Copy)]
enum Pixel {
    Verified,
    Original,
    Preview,
    Unrecorded,
    Unreadable,
    Missing,
}

async fn pixel_state(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    value: &Value,
) -> Result<Pixel> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT content_sha256,preview_path FROM photos WHERE project_id=? AND id=?",
    )
    .bind(&task.project_id)
    .bind(value["photoId"].as_str().unwrap_or(""))
    .fetch_optional(&mut **tx)
    .await?;
    let Some((hash, path)) = row else {
        return Ok(Pixel::Missing);
    };
    let provenance = &value["provenance"];
    // 原图换内容和"交给模型的那份预览字节变了"是两件事，修法不同，不能都报成原图已改。
    if provenance["photo_hash"] != hash {
        return Ok(Pixel::Original);
    }
    let status = super::vision::current_preview_status_for_version(
        provenance["preview_sha256"].as_str(),
        &path,
        provenance["prompt_version"].as_str().unwrap_or(""),
    )
    .await;
    Ok(match status {
        "verified" => Pixel::Verified,
        // 原图指纹对得上、预览字节对不上：变的只是本地派生缓存，不是用户拍的那张。
        "mismatch" => Pixel::Preview,
        "unrecorded" => Pixel::Unrecorded,
        // 这个函数只有上面几种取值，走到这里只可能是现在读不出预览来核对。
        _ => Pixel::Unreadable,
    })
}

async fn image_status(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    value: &Value,
) -> Result<&'static str> {
    Ok(match pixel_state(tx, task, value).await? {
        Pixel::Verified => "verified",
        Pixel::Original | Pixel::Preview => "mismatch",
        Pixel::Unrecorded => "unrecorded",
        Pixel::Unreadable => "unreadable",
        Pixel::Missing => "unavailable",
    })
}

pub async fn projection(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<Vec<Value>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT call_id,result_json FROM tool_calls WHERE task_id=? AND tool_name='explain_photo_judgment' AND json_extract(result_json,'$.ok')=1 AND length(CAST(result_json AS BLOB))<=65536 ORDER BY rowid DESC LIMIT 20")
        .bind(&task.id).fetch_all(&mut **tx).await?;
    let mut values = vec![];
    for (id, raw) in rows.into_iter().rev() {
        let receipt: ToolResult = serde_json::from_str(&raw).map_err(serialization)?;
        if !receipt.ok {
            continue;
        }
        if let Some(mut value) = receipt.result {
            if !task
                .allowed_photo_ids
                .iter()
                .any(|id| value["photoId"] == *id)
            {
                continue;
            }
            value["id"] = json!(id);
            value["imageStatus"] = json!(image_status(tx, task, &value).await?);
            let Some(fields) = value.as_object_mut() else {
                return Err(AppError::new(
                    "JUDGMENT_RECEIPT_SHAPE",
                    "这条判断回执的结构不合法，未能展示。",
                ));
            };
            fields.remove("provenance");
            values.push(value);
        }
    }
    Ok(values)
}

#[cfg(test)]
#[path = "photo_judgment_tests.rs"]
mod tests;
