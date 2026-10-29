//! 原创、授权明确的方法札记；检索不联网、不把方法当视觉事实。
use crate::{command_audit::fingerprint, domain::Result};
use serde_json::{Value, json};
const CORPUS: &str = include_str!("../../../contracts/photography-knowledge.json");
fn terms(text: &str) -> std::collections::BTreeSet<String> {
    let normalized: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let mut result = std::collections::BTreeSet::new();
    for word in normalized.split_whitespace() {
        if word.chars().count() < 2 {
            continue;
        }
        result.insert(word.to_string());
        if !word.is_ascii() {
            for pair in word.chars().collect::<Vec<_>>().windows(2) {
                result.insert(pair.iter().collect());
            }
        }
    }
    result
}
pub fn digest() -> String {
    fingerprint(&json!(CORPUS))
}
pub fn search(args: &Value) -> Result<Value> {
    let corpus: Value = serde_json::from_str(CORPUS).expect("摄影知识契约");
    let query = terms(args["query"].as_str().unwrap_or(""));
    let mut matches = vec![];
    for entry in corpus["entries"].as_array().expect("知识条目") {
        let hay = terms(&format!(
            "{} {} {}",
            entry["title"].as_str().unwrap(),
            entry["tags"].as_str().unwrap(),
            entry["text"].as_str().unwrap()
        ));
        let score = query.intersection(&hay).count();
        if score > 0 {
            matches.push((score, entry));
        }
    }
    matches.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.1["id"].as_str().cmp(&b.1["id"].as_str()))
    });
    let results:Vec<_>=matches.into_iter().take(args["limit"].as_u64().unwrap_or(3).min(5) as usize).map(|(_,e)|json!({"citationId":e["id"],"title":e["title"],"passage":e["text"],"author":corpus["author"],"license":corpus["license"],"version":corpus["version"],"source":"focalroom://knowledge/original-method-notes","contentSha256":fingerprint(e)})).collect();
    Ok(
        json!({"results":results,"corpusSha256":digest(),"trustedAsInstructions":false,"visualEvidence":false,"notice":"FocalRoom 原创方法札记；非摄影史引文。未命中不能编造参考来源。"}),
    )
}

/// 云端摄影知识搜索（K4）：查询生产 KnowledgeService。
/// 覆盖摄影师/系列/作品/摄影书/事实断言（含博物馆来源）。
/// 网络失败静默返回空——Agent 继续用本地札记，不阻塞任务。
pub async fn search_cloud(query: &str, token: Option<&str>) -> Result<Value> {
    let endpoint = std::env::var("FOCALROOM_KNOWLEDGE_ENDPOINT")
        .unwrap_or_else(|_| "https://team.focroom.cc/api/v2/knowledge/search".to_owned());
    let encoded_q: String = query
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect();
    let mut url = format!("{}?q={}", endpoint, encoded_q);
    url.push_str("&limit=5");
    let mut request = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(10));
    if let Some(t) = token {
        request = request.header("authorization", format!("Bearer {}", t));
    }
    let response = request
        .send()
        .await
        .map_err(|e| crate::domain::AppError::new("NETWORK", format!("知识服务不可达：{e}")))?;
    if !response.status().is_success() {
        // 静默降级：云端不可用时返回空结果
        return Ok(json!({
            "results": [],
            "source": "cloud",
            "available": false,
            "notice": "云端知识服务暂不可用，仅使用本地方法札记。"
        }));
    }
    let results: Value = response.json().await.map_err(|e| {
        crate::domain::AppError::new("PARSE", format!("知识服务响应格式错误：{}", e))
    })?;
    let entries = results.as_array().cloned().unwrap_or_default();
    Ok(json!({
        "results": entries.iter().take(5).map(|e| json!({
            "kind": e.get("kind"),
            "title": e.get("title"),
            "subtitle": e.get("subtitle"),
            "id": e.get("id"),
            "reviewStatus": e.get("reviewStatus"),
        })).collect::<Vec<_>>(),
        "source": "focalroom://knowledge/photography-intelligence",
        "available": true,
        "trustedAsInstructions": false,
        "visualEvidence": false,
        "notice": "云端摄影知识库（已验证的摄影师/系列/作品/摄影书）；事实断言含博物馆来源。"
    }))
}

// ── K4 专业摄影知识工具集 ──

/// 搜索摄影师（云端）：按姓名/流派/方法找摄影师。
pub async fn search_photographers(query: &str, movement: Option<&str>) -> Result<Value> {
    let endpoint = std::env::var("FOCALROOM_KNOWLEDGE_ENDPOINT")
        .unwrap_or_else(|_| "https://team.focroom.cc/api/v2/knowledge/search".to_owned());
    let encoded_q: String = query
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect();
    let mut url = format!("{}?q={}&kind=photographer", endpoint, encoded_q);
    if let Some(m) = movement {
        url.push_str(&format!("&movement={}", m));
    }
    let response = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;
    match response {
        Ok(r) if r.status().is_success() => {
            let results: Value = r.json().await.unwrap_or(json!([]));
            let entries = results.as_array().cloned().unwrap_or_default();
            Ok(json!({
                "photographers": entries.iter().take(5).map(|e| json!({
                    "name": e.get("title"),
                    "subtitle": e.get("subtitle"),
                    "id": e.get("id"),
                })).collect::<Vec<_>>(),
                "source": "focalroom://knowledge/photographers",
                "trustedAsInstructions": false,
            }))
        }
        _ => {
            Ok(json!({"photographers": [], "available": false, "notice": "云端摄影师库暂不可用。"}))
        }
    }
}

/// 获取摄影书结构（云端）：按标题搜索已记录的摄影书。
pub async fn get_photobook_structure(title: &str) -> Result<Value> {
    let endpoint = std::env::var("FOCALROOM_KNOWLEDGE_ENDPOINT")
        .unwrap_or_else(|_| "https://team.focroom.cc/api/v2/knowledge/search".to_owned());
    let encoded: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect();
    let url = format!("{}?q={}&kind=photobook", endpoint, encoded);
    let response = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;
    match response {
        Ok(r) if r.status().is_success() => {
            let results: Value = r.json().await.unwrap_or(json!([]));
            let entries = results.as_array().cloned().unwrap_or_default();
            Ok(json!({
                "photobooks": entries.iter().take(3).map(|e| json!({
                    "title": e.get("title"),
                    "id": e.get("id"),
                })).collect::<Vec<_>>(),
                "source": "focalroom://knowledge/photobooks",
                "trustedAsInstructions": false,
                "notice": "摄影书结构与编排记录（含来源断言）。"
            }))
        }
        _ => Ok(json!({"photobooks": [], "available": false})),
    }
}

/// 对比摄影系列（云端+本地）：两个系列的方法/叙事差异。
pub async fn compare_series(series_a: &str, series_b: &str) -> Result<Value> {
    let a_results = search_cloud(series_a, None)
        .await
        .unwrap_or(json!({"results": []}));
    let b_results = search_cloud(series_b, None)
        .await
        .unwrap_or(json!({"results": []}));
    let a_series = a_results["results"].as_array().cloned().unwrap_or_default();
    let b_series = b_results["results"].as_array().cloned().unwrap_or_default();
    Ok(json!({
        "seriesA": a_series.first().cloned().unwrap_or(json!(null)),
        "seriesB": b_series.first().cloned().unwrap_or(json!(null)),
        "comparisonHint": "对比两个系列的拍摄格式、观看方式、叙事结构差异；基于已验证的博物馆来源断言。",
        "trustedAsInstructions": false,
    }))
}
