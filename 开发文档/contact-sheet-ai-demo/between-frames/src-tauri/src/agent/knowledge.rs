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
