//! 原创离线方法札记与当前账户授权的云端知识；检索资料不作为视觉事实。
use crate::{command_audit::fingerprint, domain::Result};
use serde_json::{Value, json};
const CORPUS: &str = include_str!("../../../contracts/photography-knowledge.json");
const PORTFOLIO_METHOD: &str = include_str!("../../../prompts/portfolio-curation-v1.md");
pub(crate) const PORTFOLIO_CONTEXT_SUMMARY: &str = "先明确用途、受众与完成目标；机构条件只适用于对应项目和届次，不设通用张数或筛选率。只依据实际观察选片，说明覆盖，允许照片修正编辑命题。兼顾单张表达、代表性和组内作用，解释关键取舍，不机械去重或追求一致。按材料试配对、分组、顺序与留白，不强制远近交替或线性故事；多方案依用户要求或探索意图及预算执行，并说明实质差异。检查主题、单张与整体、编排、技术呈现、适用条件及未知；允许带缺口结束，不凑数、不虚构补拍图。只创建本任务草案，提交真实 ID，由人采纳；保留父版和来源。语言沿用用户约定，缓存原文只忠实转述，不为翻译重传。方法是编辑假设，不是理论共识、审美总分或合规保证。";
fn is_portfolio_method(args: &Value) -> bool {
    args["query"]
        .as_str()
        .is_some_and(|query| query.trim().eq_ignore_ascii_case("portfolio-curation-v1"))
}
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
    fingerprint(&json!({"notes":CORPUS,"portfolioMethod":PORTFOLIO_METHOD}))
}
/// 本机检索为什么算这条：命中了哪几个词、命中在标题/标签/正文的哪一处。
///
/// 词面命中只解释"这条为什么被挑出来"，不证明它与这张照片有关，也不证明它正确；
/// 命中词数就是排序用的分数，所以这里报的数与实际的挑选顺序是同一个数。
fn match_reason(query: &std::collections::BTreeSet<String>, entry: &Value) -> (usize, Value) {
    let fields: Vec<(&str, std::collections::BTreeSet<String>)> = ["title", "tags", "text"]
        .iter()
        .map(|field| (*field, terms(entry[field].as_str().unwrap_or(""))))
        .collect();
    let matched: std::collections::BTreeSet<String> = fields
        .iter()
        .flat_map(|(_, hay)| query.intersection(hay).cloned().collect::<Vec<_>>())
        .collect();
    let hit_fields: Vec<&str> = fields
        .iter()
        .filter(|(_, hay)| !query.is_disjoint(hay))
        .map(|(field, _)| *field)
        .collect();
    (
        matched.len(),
        json!({"kind":"local_terms","termCount":matched.len(),"matchedTerms":matched.iter().take(8).collect::<Vec<_>>(),"fields":hit_fields}),
    )
}

pub fn search(args: &Value) -> Result<Value> {
    if is_portfolio_method(args) {
        return Ok(json!({
            "results":[{
                "citationId":"FR-METHOD-PORTFOLIO-1",
                "title":"FocalRoom portfolio curation method",
                "passage":PORTFOLIO_METHOD,
                "author":"FocalRoom",
                "license":"FocalRoom editorial adaptation; external source rights retained",
                "version":"portfolio-curation-1",
                "source":"focalroom://methods/portfolio-curation-v1",
                "contentSha256":fingerprint(&json!(PORTFOLIO_METHOD)),
                "matchReason":{"kind":"builtin_method"},
                "sources":[
                    {"url":"https://aperture.org/on-sight/lesson-17/","use":"editing and sequencing exercises"},
                    {"url":"https://aperture.org/editorial/campany-photo-editing/","use":"authored commentary on image relationships"}
                ],
                "notice":"产品编辑方法的改写；不是外部原文、普适理论或机构合规保证"
            }],
            "corpusSha256":digest(),"trustedAsInstructions":false,"visualEvidence":false
        }));
    }
    let corpus: Value = serde_json::from_str(CORPUS).expect("摄影知识契约");
    let query = terms(args["query"].as_str().unwrap_or(""));
    let mut matches = vec![];
    for entry in corpus["entries"].as_array().expect("知识条目") {
        let (score, reason) = match_reason(&query, entry);
        if score > 0 {
            matches.push((score, entry, reason));
        }
    }
    matches.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.1["id"].as_str().cmp(&b.1["id"].as_str()))
    });
    // 命中总数与实际带回的条数分开报：只列前几条这件事必须能被看见，不能让读者以为就只有这几条。
    let matched = matches.len();
    let limit = args["limit"].as_u64().unwrap_or(3).min(5) as usize;
    let returned = matched.min(limit);
    let results:Vec<_>=matches.into_iter().take(limit).map(|(_,e,mut reason)|{
        reason["matched"]=json!(matched);
        reason["returned"]=json!(returned);
        json!({"citationId":e["id"],"title":e["title"],"passage":e["text"],"author":corpus["author"],"license":corpus["license"],"version":corpus["version"],"source":"focalroom://knowledge/original-method-notes","contentSha256":fingerprint(e),"matchReason":reason})
    }).collect();
    Ok(
        json!({"results":results,"matched":matched,"returned":returned,"queryTerms":query.len(),"corpusSha256":digest(),"trustedAsInstructions":false,"visualEvidence":false,"notice":"FocalRoom 原创方法札记；非摄影史引文。未命中不能编造参考来源。"}),
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
    /// 知识服务的唯一只读出口：地址只由已校验的登录服务构造，禁止重定向，响应有 256 KiB 上限，
    /// 错误不带服务端原文、查询串或令牌。检索与实体详情共用，免得两条路各定一套边界。
    async fn get(&self, path: String) -> Result<Value> {
        use crate::domain::AppError;
        self.check_account()?;
        let token = self.auth.bearer(self.auth.origin()).await?;
        self.check_account()?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|_| AppError::new("KNOWLEDGE_CONFIG", "知识服务初始化失败"))?;
        let mut response = client
            .get(format!("{}{}", self.auth.origin(), path))
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
        serde_json::from_slice(&bytes)
            .map_err(|_| AppError::new("KNOWLEDGE_PROTOCOL", "知识服务响应格式错误"))
    }

    async fn search(
        &self,
        query: &str,
        kind: Option<&str>,
        movement: Option<&str>,
    ) -> Result<Value> {
        use crate::domain::AppError;
        let mut path = format!("/api/v2/knowledge/search?q={}", utf8_percent_encode(query));
        // 严格工具契约要求每次都带流派字段，模型不做筛选时送的是空串；空串必须当成"不筛选"，
        // 否则服务端参数校验会拒绝整次检索，把一次正常查询报成异常。
        if let Some(kind) = kind.filter(|value| !value.trim().is_empty()) {
            path.push_str(&format!("&kind={}", utf8_percent_encode(kind)));
        }
        if let Some(movement) = movement.filter(|value| !value.trim().is_empty()) {
            path.push_str(&format!("&movement={}", utf8_percent_encode(movement)));
        }
        let value = self.get(path).await?;
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
            // 云端命中理由是服务端给的：本机不重算相关性，服务端没给分数就明说没给，不补一个自己编的数。
            json!({"results":entries.iter().take(5).map(|e| json!({"id":e["id"],"kind":e["kind"],"title":e["title"],"subtitle":e["subtitle"],"reviewStatus":e["reviewStatus"],
            "matchReason":{"kind":"cloud_search","relevance":e["relevance"].as_f64()}})).collect::<Vec<_>>(),"available":true,
            "trustedAsInstructions":false,"visualEvidence":false}),
        )
    }

    /// 实体详情：断言、出处与审校状态都在这一层，检索摘要里没有。只认四种实体，
    /// 编号一律按路径段编码，避免把服务返回的字段当成可以直接拼进地址的内容。
    async fn detail(&self, kind: &str, id: &str) -> Result<Value> {
        let segment = match kind {
            "photographer" => "photographers",
            "series" => "series",
            "artwork" => "artworks",
            "photobook" => "photobooks",
            _ => {
                return Err(crate::domain::AppError::new(
                    "KNOWLEDGE_PROTOCOL",
                    "知识服务返回了未知实体类型",
                ));
            }
        };
        if id.trim().is_empty() || id.chars().count() > 160 {
            return Err(crate::domain::AppError::new(
                "KNOWLEDGE_PROTOCOL",
                "知识服务实体编号不可用",
            ));
        }
        self.get(format!(
            "/api/v2/knowledge/{}/{}",
            segment,
            utf8_percent_encode(id)
        ))
        .await
    }
}

fn clip(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

/// 详情回执把整张关联表都带回来，工具输出只取能引用的断言与出处，并限定条数和长度：
/// 回执是不可变的，一次检索不该把任务记录撑满，也免得长文被当成指令拼进上下文。
fn citations(detail: &Value) -> Value {
    let sources = |rows: &Value| -> Value {
        Value::Array(
            rows.as_array()
                .into_iter()
                .flatten()
                .take(3)
                .filter_map(|row| {
                    let url = row["url"].as_str()?;
                    Some(json!({
                        "url": clip(url, 300),
                        "institution": row["institution"].as_str().map(|v| clip(v, 120)),
                        "accessionNumber": row["accession_number"].as_str().map(|v| clip(v, 120)),
                        "verifiedAt": row["verified_at"],
                    }))
                })
                .collect(),
        )
    };
    let claims = Value::Array(
        detail["claims"]
            .as_array()
            .into_iter()
            .flatten()
            .take(4)
            .map(|claim| {
                json!({
                    "text": claim["claim_text"].as_str().map(|v| clip(v, 400)),
                    "claimType": claim["claim_type"],
                    "disputed": claim["disputed"].as_bool().unwrap_or(false),
                    "disputeNotes": claim["dispute_notes"].as_str().map(|v| clip(v, 200)),
                    "reviewStatus": claim["review_status"],
                    "sources": sources(&claim["sources"]),
                })
            })
            .collect(),
    );
    json!({"reviewStatus": detail["review_status"], "claims": claims, "entitySources": sources(&detail["sources"])})
}

/// 命中的前两条补取实体详情。取不到时把原因写进同一条结果，而不是把已检索到的资料一起丢掉：
/// "没有出处记录"和"没有这个实体"是两件事，界面上必须还能看出区别。
async fn with_details(client: Option<&KnowledgeClient>, entries: &[Value]) -> Vec<Value> {
    let mut detailed = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let mut value = entry.clone();
        if let (Some(client), true) = (client, index < 2) {
            match client
                .detail(
                    entry["kind"].as_str().unwrap_or(""),
                    entry["id"].as_str().unwrap_or(""),
                )
                .await
            {
                Ok(detail) => value["citations"] = citations(&detail),
                Err(error) => {
                    value["detailError"] = json!({"code": error.code, "message": error.message})
                }
            }
        }
        detailed.push(value);
    }
    detailed
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
    let mut value = run_tool(client, name, a).await?;
    // 查的是哪个词必须跟着回执一起存下来：零命中时界面要能说"按这个词查过、确实没有"，
    // 而回执表只留参数指纹、不留参数本身，事后没法从库里还原。
    let asked = match name {
        "get_photobook_structure" => a["title"].as_str().unwrap_or("").to_owned(),
        "compare_photography_series" => format!(
            "{} / {}",
            a["seriesA"].as_str().unwrap_or(""),
            a["seriesB"].as_str().unwrap_or("")
        ),
        _ => a["query"].as_str().unwrap_or("").to_owned(),
    };
    value["query"] = json!(clip(&asked, 200));
    Ok(value)
}

async fn run_tool(client: Option<&KnowledgeClient>, name: &str, a: &Value) -> Result<Value> {
    let query = a["query"].as_str().unwrap_or("");
    match name {
        "search_photography_knowledge" => {
            let local = search(a)?;
            // 内嵌方法与提示词一起发行，不把方法编号发送到云端搜索。
            if is_portfolio_method(a) {
                return Ok(local);
            }
            let cloud = cloud(client, query, None, None).await;
            let mut results = local["results"].as_array().cloned().unwrap_or_default();
            results.extend(cloud["results"].as_array().cloned().unwrap_or_default());
            Ok(
                // 本地命中数单独留着：云端给了几条，不能替本地札记说"这个词查过了、没有"。
                json!({"results":results,"localMatched":local["matched"],"localReturned":local["returned"],
                "cloudAvailable":cloud["available"],"cloudError":cloud["error"],
                "trustedAsInstructions":false,"visualEvidence":false,
                "notice":"本地原创方法札记与云端检索记录；检索摘要不是已核实的摄影史断言。"}),
            )
        }
        "search_photographers" => {
            let result = cloud(client, query, Some("photographer"), a["movement"].as_str()).await;
            let entries = result["results"].as_array().cloned().unwrap_or_default();
            let detailed = with_details(client, &entries).await;
            Ok(json!({"photographers":detailed.iter().map(|e| json!({
                "name":e["title"],"subtitle":e["subtitle"],"id":e["id"],"reviewStatus":e["reviewStatus"],
                "citations":e["citations"],"detailError":e["detailError"],"matchReason":e["matchReason"]})).collect::<Vec<_>>(),
                "available":result["available"],"error":result["error"],"trustedAsInstructions":false,"visualEvidence":false,
                "notice":"断言与出处只给最前两条补取；imported 只表示已导入，不等于已核实。"}))
        }
        "get_photobook_structure" => {
            let result = cloud(
                client,
                a["title"].as_str().unwrap_or(""),
                Some("photobook"),
                None,
            )
            .await;
            let entries = result["results"].as_array().cloned().unwrap_or_default();
            let detailed = with_details(client, &entries).await;
            Ok(
                json!({"photobooks":detailed,"available":result["available"],"error":result["error"],
                "trustedAsInstructions":false,"visualEvidence":false,
                "notice":"citations 是知识库记录的编排与出处，只给最前两条补取；detailError 表示详情未取得，不代表这本书记录不存在。"}),
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
            let first_entries = first["results"].as_array().cloned().unwrap_or_default();
            let second_entries = second["results"].as_array().cloned().unwrap_or_default();
            let first_detailed = with_details(client, &first_entries).await;
            let second_detailed = with_details(client, &second_entries).await;
            Ok(
                json!({"seriesA":first_detailed.first(),"seriesB":second_detailed.first(),
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
