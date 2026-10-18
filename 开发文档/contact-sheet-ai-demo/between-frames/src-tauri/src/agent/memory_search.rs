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

/// 混合检索分数：BM25 关键词 0.6 + 句向量余弦 0.4 加权融合。
/// BM25 为负值（越负越相关），先取 -bm25 再 min-max 归一；余弦从 [-1,1] 映射到 [0,1]。
/// 字面命中是更强信号（权重更高，避免语义噪声在平局时反超）；语义作相关度细分。
/// 语义分缺失的候选退化为关键词分；两者都缺失不产生条目。
pub(crate) fn hybrid_scores(
    keyword: &HashMap<i64, f64>,
    semantic: &HashMap<i64, Option<f64>>,
) -> HashMap<i64, f64> {
    let keyword_norm = min_max(
        keyword
            .iter()
            .map(|(id, score)| (*id, (-score).max(0.0)))
            .collect(),
    );
    let semantic_norm: HashMap<_, _> = semantic
        .iter()
        .filter_map(|(id, score)| score.map(|cosine| (*id, (cosine + 1.0) / 2.0)))
        .collect();
    let mut merged = HashMap::new();
    for (id, keyword_score) in keyword_norm {
        let blended = match semantic_norm.get(&id) {
            Some(semantic_score) => 0.6 * keyword_score + 0.4 * semantic_score,
            None => keyword_score,
        };
        merged.insert(id, blended);
    }
    // 只在语义侧出现的候选（关键词未命中但语义相关）以 0.4 上限参与：
    // 低于任何关键词命中的最低可能分（0.6），语义发现不反超字面命中组。
    for (id, semantic_score) in semantic_norm {
        merged.entry(id).or_insert(semantic_score * 0.4);
    }
    merged
}
fn min_max(scores: HashMap<i64, f64>) -> HashMap<i64, f64> {
    let mut values: Vec<_> = scores.values().copied().filter(|v| v.is_finite()).collect();
    if values.is_empty() {
        return HashMap::new();
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let (min, max) = (values[0], values[values.len() - 1]);
    scores
        .into_iter()
        .map(|(id, score)| {
            let normalized = if (max - min).abs() < f64::EPSILON {
                1.0
            } else {
                ((score - min) / (max - min)).clamp(0.0, 1.0)
            };
            (id, normalized)
        })
        .collect()
}

#[cfg(test)]
mod hybrid_tests {
    use super::*;
    fn map(entries: &[(i64, f64)]) -> HashMap<i64, f64> {
        entries.iter().copied().collect()
    }

    #[test]
    fn hybrid_blends_keyword_and_semantic_equally() {
        // 关键词归一：a=1.0、b=0.0；语义：a=0.6、b=1.0（余弦 0.2 → 0.6、1.0 → 1.0）。
        let keyword = map(&[(1, -3.0), (2, -1.0)]);
        let semantic: HashMap<i64, Option<f64>> = [(1, Some(0.2)), (2, Some(1.0))].into();
        let blended = hybrid_scores(&keyword, &semantic);
        assert!((blended[&1] - 0.84).abs() < 1e-9);
        assert!((blended[&2] - 0.4).abs() < 1e-9);
    }

    #[test]
    fn hybrid_falls_back_to_keyword_without_semantics() {
        let keyword = map(&[(1, -2.0), (2, -1.0)]);
        let blended = hybrid_scores(&keyword, &HashMap::new());
        assert!(blended[&1] > blended[&2], "关键词更相关者胜出");
        // 语义缺失候选不产生虚高分。
        assert_eq!(blended.len(), 2);
    }

    #[test]
    fn hybrid_includes_semantic_only_hits() {
        let keyword = map(&[(1, -1.0)]);
        let semantic: HashMap<i64, Option<f64>> = [(2, Some(0.8))].into();
        let blended = hybrid_scores(&keyword, &semantic);
        assert!(blended.contains_key(&2), "关键词未命中但语义相关者参与");
        assert!(
            (blended[&2] - 0.36).abs() < 1e-9,
            "余弦 0.8 归一为 0.9 再 0.4 折减 = 0.36，不反超字面命中（最低 0.6）"
        );
    }

    #[test]
    fn equal_scores_normalize_without_division_by_zero() {
        let keyword = map(&[(1, -2.0), (2, -2.0)]);
        let blended = hybrid_scores(&keyword, &HashMap::new());
        assert_eq!(blended[&1], 1.0);
        assert_eq!(blended[&2], 1.0);
    }
}
