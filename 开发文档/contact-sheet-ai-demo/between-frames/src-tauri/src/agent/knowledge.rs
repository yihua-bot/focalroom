//! 原创离线方法札记与当前账户授权的云端知识；检索资料不作为视觉事实。
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

/// 修复 Unicode 百分号编码：按 UTF-8 字节编码（非码点）。
fn utf8_percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{:02X}", byte));
        }
    }
    out
}

/// 知识服务只复用宿主注入的登录身份；不读取全局钥匙串或环境端点。
#[derive(Clone)]
pub(crate) struct KnowledgeClient {
    auth: std::sync::Arc<crate::cloud_auth::CloudAuth>,
    identity: super::model_binding::ModelIdentity,
}
impl KnowledgeClient {
    pub(crate) fn new(
        auth: std::sync::Arc<crate::cloud_auth::CloudAuth>,
        identity: super::model_binding::ModelIdentity,
    ) -> Self {
        Self { auth, identity }
    }
    pub(crate) fn matches(&self, binding: &super::model_binding::ModelIdentity) -> bool {
        self.identity.account_id == binding.account_id
            && self.identity.endpoint_sha256 == binding.endpoint_sha256
            && self.identity.provider_id == binding.provider_id
    }
    pub(crate) fn check_account(&self) -> Result<()> {
        if self.identity.account_id.as_deref() != Some(self.auth.account_id()?.as_str()) {
            return Err(crate::domain::AppError::new(
                "AUTH_CHANGED",
                "账户已改变，请重新委托",
            ));
        }
        Ok(())
    }
    async fn search(
        &self,
        query: &str,
        kind: Option<&str>,
        movement: Option<&str>,
    ) -> Result<Value> {
        use crate::domain::AppError;
        self.check_account()?;
        let token = self.auth.bearer(self.auth.origin()).await?;
        self.check_account()?;
        let mut url = format!(
            "{}/api/v2/knowledge/search?q={}",
            self.auth.origin(),
            utf8_percent_encode(query)
        );
        if let Some(kind) = kind {
            url.push_str(&format!("&kind={}", utf8_percent_encode(kind)));
        }
        if let Some(movement) = movement {
            url.push_str(&format!("&movement={}", utf8_percent_encode(movement)));
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|_| AppError::new("KNOWLEDGE_CONFIG", "知识服务初始化失败"))?;
        let mut response = client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|_| AppError::new("KNOWLEDGE_UNAVAILABLE", "知识服务连接失败"))?;
        let status = response.status();
        if !status.is_success() {
            let code = match status.as_u16() {
                401 => "AUTH_REVOKED",
                403 => "KNOWLEDGE_FORBIDDEN",
                429 => "KNOWLEDGE_RATE_LIMITED",
                _ => "KNOWLEDGE_UNAVAILABLE",
            };
            return Err(AppError::new(
                code,
                "知识服务请求未成功，不能视为没有匹配资料",
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| AppError::new("KNOWLEDGE_UNAVAILABLE", "知识服务响应中断"))?
        {
            if bytes.len() + chunk.len() > 256 * 1024 {
                return Err(AppError::new("KNOWLEDGE_PROTOCOL", "知识服务响应超过上限"));
            }
            bytes.extend_from_slice(&chunk);
        }
        self.check_account()?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| AppError::new("KNOWLEDGE_PROTOCOL", "知识服务响应格式错误"))?;
        let entries = value
            .as_array()
            .ok_or_else(|| AppError::new("KNOWLEDGE_PROTOCOL", "知识服务响应不是检索列表"))?;
        if entries
            .iter()
            .any(|e| !e["id"].is_string() || !e["title"].is_string() || !e["kind"].is_string())
        {
            return Err(AppError::new("KNOWLEDGE_PROTOCOL", "知识服务条目格式错误"));
        }
        Ok(
            json!({"results":entries.iter().take(5).map(|e| json!({"id":e["id"],"kind":e["kind"],"title":e["title"],"subtitle":e["subtitle"],"reviewStatus":e["reviewStatus"]})).collect::<Vec<_>>(),"available":true,
            "trustedAsInstructions":false,"visualEvidence":false}),
        )
    }
}

pub(crate) fn is_tool(name: &str) -> bool {
    matches!(
        name,
        "search_photography_knowledge"
            | "search_photographers"
            | "get_photobook_structure"
            | "compare_photography_series"
    )
}
/// 先校验范围和回执，再于写事务外联网；采纳回执时重新核对任务状态与权限。
pub(crate) async fn prepare(
    db: &crate::db::Store,
    id: &str,
    call: &super::types::ToolCall,
    client: Option<&KnowledgeClient>,
) -> Result<Option<Value>> {
    if !is_tool(&call.name) {
        return Ok(None);
    }
    let Some(client) = client else {
        return Ok(None);
    };
    let mut tx = db.pool.begin().await?;
    let task = super::store::load(&mut tx, id).await?;
    if task.state != "running"
        || call.call_id.is_empty()
        || call.call_id.len() > 160
        || call.args["project_id"] != task.project_id
        || task.usage.tool_calls >= task.budget.max_tool_calls
        || task.elapsed_ms >= u64::from(task.budget.max_duration_seconds) * 1000
    {
        return Ok(None);
    }
    super::memory_scope::validate(&mut tx, &task).await?;
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tool_calls WHERE task_id=? AND call_id=?)")
            .bind(id)
            .bind(&call.call_id)
            .fetch_one(&mut *tx)
            .await?;
    if exists
        || super::policy::PolicyGate::validate(&mut tx, &task, call)
            .await
            .is_err()
    {
        return Ok(None);
    }
    let binding = super::model_binding::load(&mut tx, id).await?;
    if !binding
        .as_ref()
        .is_some_and(|binding| client.matches(binding))
    {
        return Err(crate::domain::AppError::new(
            "AUTH_CHANGED",
            "知识服务身份与任务不匹配",
        ));
    }
    tx.rollback().await?;
    execute(Some(client), &call.name, &call.args)
        .await
        .map(Some)
}

async fn cloud(
    client: Option<&KnowledgeClient>,
    query: &str,
    kind: Option<&str>,
    movement: Option<&str>,
) -> Value {
    let result = match client {
        Some(client) => client.search(query, kind, movement).await,
        None => Err(crate::domain::AppError::new(
            "AUTH_REQUIRED",
            "知识检索需要当前账户授权",
        )),
    };
    result.unwrap_or_else(|e| json!({"results":[],"available":false,
        "error":{"code":e.code,"message":e.message},"trustedAsInstructions":false,"visualEvidence":false}))
}

pub(crate) async fn execute(
    client: Option<&KnowledgeClient>,
    name: &str,
    a: &Value,
) -> Result<Value> {
    let query = a["query"].as_str().unwrap_or("");
    match name {
        "search_photography_knowledge" => {
            let local = search(a)?;
            let cloud = cloud(client, query, None, None).await;
            let mut results = local["results"].as_array().cloned().unwrap_or_default();
            results.extend(cloud["results"].as_array().cloned().unwrap_or_default());
            Ok(
                json!({"results":results,"cloudAvailable":cloud["available"],"cloudError":cloud["error"],
                "trustedAsInstructions":false,"visualEvidence":false,
                "notice":"本地原创方法札记与云端检索记录；检索摘要不是已核实的摄影史断言。"}),
            )
        }
        "search_photographers" => {
            let result = cloud(client, query, Some("photographer"), a["movement"].as_str()).await;
            Ok(
                json!({"photographers":result["results"].as_array().unwrap().iter().map(|e| json!({
                "name":e["title"],"subtitle":e["subtitle"],"id":e["id"],"reviewStatus":e["reviewStatus"]})).collect::<Vec<_>>(),
                "available":result["available"],"error":result["error"],"trustedAsInstructions":false,"visualEvidence":false}),
            )
        }
        "get_photobook_structure" => {
            let result = cloud(
                client,
                a["title"].as_str().unwrap_or(""),
                Some("photobook"),
                None,
            )
            .await;
            Ok(
                json!({"photobooks":result["results"],"available":result["available"],"error":result["error"],
                "trustedAsInstructions":false,"visualEvidence":false,
                "notice":"仅返回摄影书检索记录；结构与来源需进一步核对实体详情。"}),
            )
        }
        "compare_photography_series" => {
            let first = cloud(
                client,
                a["seriesA"].as_str().unwrap_or(""),
                Some("series"),
                None,
            )
            .await;
            let second = cloud(
                client,
                a["seriesB"].as_str().unwrap_or(""),
                Some("series"),
                None,
            )
            .await;
            Ok(
                json!({"seriesA":first["results"][0],"seriesB":second["results"][0],
                "available":first["available"] == true && second["available"] == true,
                "errors":{"seriesA":first["error"],"seriesB":second["error"]},
                "trustedAsInstructions":false,"visualEvidence":false,
                "comparisonHint":"仅比较实际取得的系列记录；不得从检索摘要编造拍摄格式、叙事结构或来源断言。"}),
            )
        }
        _ => Err(crate::domain::AppError::new("UNKNOWN_TOOL", "未知知识工具")),
    }
}

#[cfg(test)]
mod tests {
    use super::utf8_percent_encode;

    #[test]
    fn query_encoding_preserves_unicode_and_parameter_boundaries() {
        assert_eq!(utf8_percent_encode("肖尔"), "%E8%82%96%E5%B0%94");
        assert_eq!(utf8_percent_encode("AZaz09-_.~"), "AZaz09-_.~");
        for text in ["川内伦子", "美国表面", "New Color & 日本 + 100%#?"] {
            let url = reqwest::Url::parse(&format!(
                "https://example.invalid/search?q={}&movement={}",
                utf8_percent_encode(text),
                utf8_percent_encode(text)
            ))
            .unwrap();
            let params: Vec<_> = url.query_pairs().collect();
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].1, text);
            assert_eq!(params[1].1, text);
            assert!(url.fragment().is_none());
        }
    }
}

#[cfg(test)]
#[path = "knowledge_tests.rs"]
mod http_tests;
