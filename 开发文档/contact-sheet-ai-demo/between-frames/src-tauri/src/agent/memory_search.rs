// 中文二元词 + 常规词语的项目内 FTS5 检索；关键词相关性，不冒充语义理解。
use crate::domain::Result;
use sqlx::{Sqlite, Transaction};
use std::collections::{BTreeSet, HashMap};

pub(crate) fn terms(text: &str) -> BTreeSet<String> {
    let normalized: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let mut terms = BTreeSet::new();
    for word in normalized.split_whitespace() {
        let chars: Vec<_> = word.chars().collect();
        if chars.len() >= 2 {
            terms.insert(word.to_owned());
        }
        for pair in chars.windows(2) {
            terms.insert(pair.iter().collect());
        }
    }
    terms
}
pub(crate) fn keyword(statement: &str) -> &str {
    statement
        .split_once('「')
        .and_then(|(_, rest)| rest.split_once('」').map(|(word, _)| word))
        .unwrap_or(statement)
}
pub(crate) async fn sync(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    id: i64,
    statement: &str,
) -> Result<()> {
    let text = terms(keyword(statement))
        .into_iter()
        .collect::<Vec<_>>()
        .join(" ");
    let old: Option<String> =
        sqlx::query_scalar("SELECT terms FROM memory_search WHERE rowid=? AND project_id=?")
            .bind(id)
            .bind(project)
            .fetch_optional(&mut **tx)
            .await?;
    if old.as_deref() != Some(&text) {
        sqlx::query("DELETE FROM memory_search WHERE rowid=? AND project_id=?")
            .bind(id)
            .bind(project)
            .execute(&mut **tx)
            .await?;
        sqlx::query("INSERT INTO memory_search(rowid,project_id,terms) VALUES(?,?,?)")
            .bind(id)
            .bind(project)
            .bind(text)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
pub(crate) async fn ranks(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    query: &str,
) -> Result<HashMap<i64, f64>> {
    let tokens = terms(query)
        .into_iter()
        .take(32)
        .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(i64, f64)> = sqlx::query_as("SELECT rowid,bm25(memory_search) FROM memory_search WHERE memory_search MATCH ? AND project_id=?")
        .bind(tokens.join(" OR ")).bind(project).fetch_all(&mut **tx).await?;
    Ok(rows.into_iter().collect())
}
