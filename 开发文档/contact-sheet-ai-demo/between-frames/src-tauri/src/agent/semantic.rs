//! 本地文本向量计算与有界增量复用；不发网络请求、不将相似度当作事实置信度。
use super::semantic_index::{self, Index, Profile};
use serde::Serialize;

pub(crate) const MAX_DOCUMENTS: usize = 500;
#[cfg(target_os = "macos")]
const STRIDE: usize = 1024;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SemanticReport {
    pub status: &'static str,
    pub provider: &'static str,
    pub language: Option<&'static str>,
    pub revision: Option<u32>,
    pub dimensions: Option<usize>,
    pub candidates: usize,
    pub embedded: usize,
    pub limited: bool,
    pub quality: &'static str,
    pub cache_hits: usize,
    pub generated: usize,
    pub index_read: &'static str,
}
impl SemanticReport {
    pub fn skipped(status: &'static str, candidates: usize) -> Self {
        Self {
            status,
            provider: "apple_nl_sentence",
            language: None,
            revision: None,
            dimensions: None,
            candidates,
            embedded: 0,
            limited: candidates > 0,
            quality: "unmeasured",
            cache_hits: 0,
            generated: 0,
            index_read: "not_requested",
        }
    }
}

pub(crate) struct Ranking {
    pub scores: Vec<Option<f64>>,
    pub report: SemanticReport,
    pub vectors: Vec<Option<Vec<f32>>>,
    pub profile: Option<Profile>,
}

#[cfg(test)]
pub(crate) async fn rank(query: String, documents: Vec<String>, budget_ms: u32) -> Ranking {
    let keys = (0..documents.len()).map(|i| i.to_string()).collect();
    rank_cached(query, documents, budget_ms, None, keys).await
}
pub(crate) async fn rank_cached(
    query: String,
    documents: Vec<String>,
    budget_ms: u32,
    path: Option<std::path::PathBuf>,
    keys: Vec<String>,
) -> Ranking {
    let count = documents.len();
    if count == 0
        || count > MAX_DOCUMENTS
        || keys.len() != count
        || query.trim().is_empty()
        || query.chars().count() > 400
        || query.contains('\0')
        || budget_ms == 0
    {
        return Ranking {
            scores: vec![None; count],
            vectors: vec![None; count],
            profile: None,
            report: SemanticReport::skipped(
                if count == 0 {
                    "no_documents"
                } else {
                    "input_or_budget_limit"
                },
                count,
            ),
        };
    }
    static SLOTS: std::sync::LazyLock<std::sync::Arc<tokio::sync::Semaphore>> =
        std::sync::LazyLock::new(|| std::sync::Arc::new(tokio::sync::Semaphore::new(2)));
    let Ok(slot) = SLOTS.clone().try_acquire_owned() else {
        return Ranking {
            scores: vec![None; count],
            vectors: vec![None; count],
            profile: None,
            report: SemanticReport::skipped("busy", count),
        };
    };
    let result = tokio::task::spawn_blocking(move || {
        let _slot = slot;
        let started = std::time::Instant::now();
        let (cached, state) = semantic_index::load(path.as_deref());
        let remaining = budget_ms
            .saturating_sub(started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32);
        let mut ranking = native_rank(&query, &documents, remaining, &cached, &keys);
        ranking.report.index_read = state;
        ranking
    })
    .await;
    result.unwrap_or_else(|_| Ranking {
        scores: vec![None; count],
        vectors: vec![None; count],
        profile: None,
        report: SemanticReport::skipped("unavailable", count),
    })
}

#[cfg(any(target_os = "macos", test))]
fn cosine(left: &[f32], right: &[f32]) -> Option<f64> {
    if left.is_empty() || left.len() != right.len() {
        return None;
    }
    let (mut dot, mut a, mut b) = (0.0_f64, 0.0_f64, 0.0_f64);
    for (&x, &y) in left.iter().zip(right) {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let (x, y) = (f64::from(x), f64::from(y));
        dot += x * y;
        a += x * x;
        b += y * y;
    }
    if a == 0.0 || b == 0.0 {
        None
    } else {
        Some((dot / (a * b).sqrt()).clamp(-1.0, 1.0))
    }
}

#[cfg(target_os = "macos")]
fn native_rank(
    query: &str,
    documents: &[String],
    budget_ms: u32,
    cached: &Index,
    keys: &[String],
) -> Ranking {
    use std::ffi::{CString, c_char};
    #[repr(C)]
    #[derive(Default)]
    struct Info {
        dimension: u32,
        revision: u32,
        language: u32,
        processed: u32,
        status: u32,
    }
    unsafe extern "C" {
        fn bf_sentence_embeddings(
            texts: *const *const c_char,
            count: usize,
            vectors: *mut f32,
            available: *mut u8,
            budget_ms: u32,
            info: *mut Info,
            cached_vectors: *const f32,
            cached_available: *const u8,
            cached_info: *const Info,
            reused: *mut u8,
        );
    }
    let mut report = SemanticReport::skipped("unavailable", documents.len());
    let texts: Option<Vec<CString>> = std::iter::once(query)
        .chain(documents.iter().map(String::as_str))
        .map(|s| {
            (!s.is_empty() && s.chars().count() <= 1000)
                .then(|| CString::new(s).ok())
                .flatten()
        })
        .collect();
    let Some(texts) = texts else {
        return Ranking {
            scores: vec![None; documents.len()],
            vectors: vec![None; documents.len()],
            profile: None,
            report,
        };
    };
    let pointers: Vec<_> = texts.iter().map(|s| s.as_ptr()).collect();
    let mut vectors = vec![0_f32; texts.len() * STRIDE];
    let mut available = vec![0_u8; texts.len()];
    let mut info = Info::default();
    let mut previous = Info::default();
    if let Some(profile) = &cached.profile {
        previous.dimension = profile.dimensions as u32;
        previous.revision = profile.revision;
        previous.language = profile.language;
    }
    let mut cached_vectors = vec![0_f32; vectors.len()];
    let mut cached_available = vec![0_u8; texts.len()];
    for (index, key) in keys.iter().enumerate() {
        if let Some(vector) = cached.get(key) {
            cached_vectors[(index + 1) * STRIDE..(index + 1) * STRIDE + vector.len()]
                .copy_from_slice(vector);
            cached_available[index + 1] = 1;
        }
    }
    let mut reused = vec![0_u8; texts.len()];
    // SAFETY: CStrings 在调用期间存活；缓冲区分别有 count*1024 和 count 元素；
    // 桥只读取 count 个指针、写入固定行宽，所有输出在使用前再次校验。
    unsafe {
        bf_sentence_embeddings(
            pointers.as_ptr(),
            pointers.len(),
            vectors.as_mut_ptr(),
            available.as_mut_ptr(),
            budget_ms.min(3000),
            &mut info,
            cached_vectors.as_ptr(),
            cached_available.as_ptr(),
            &previous,
            reused.as_mut_ptr(),
        );
    }
    report.status = match info.status {
        1 => "available",
        2 => "time_limit",
        3 => "unsupported_language",
        _ => "unavailable",
    };
    if !(1..=STRIDE as u32).contains(&info.dimension)
        || ![1, 2].contains(&info.status)
        || available[0] != 1
    {
        return Ranking {
            scores: vec![None; documents.len()],
            vectors: vec![None; documents.len()],
            profile: None,
            report,
        };
    }
    report.language = match info.language {
        1 => Some("en"),
        2 => Some("zh-Hans"),
        _ => None,
    };
    report.revision = Some(info.revision);
    report.dimensions = Some(info.dimension as usize);
    let query_vector = &vectors[..info.dimension as usize];
    let scores: Vec<_> = (1..texts.len())
        .map(|index| {
            (available[index] == 1 && index < info.processed as usize)
                .then(|| {
                    cosine(
                        query_vector,
                        &vectors[index * STRIDE..index * STRIDE + info.dimension as usize],
                    )
                })
                .flatten()
        })
        .collect();
    let document_vectors = scores
        .iter()
        .enumerate()
        .map(|(i, score)| {
            score.map(|_| {
                vectors[(i + 1) * STRIDE..(i + 1) * STRIDE + info.dimension as usize].to_vec()
            })
        })
        .collect();
    report.cache_hits = scores
        .iter()
        .enumerate()
        .filter(|(i, score)| score.is_some() && reused[i + 1] == 1)
        .count();
    report.embedded = scores.iter().flatten().count();
    report.generated = report.embedded.saturating_sub(report.cache_hits);
    report.limited = report.embedded < documents.len();
    Ranking {
        scores,
        report,
        vectors: document_vectors,
        profile: Some(Profile {
            language: info.language,
            revision: info.revision,
            dimensions: info.dimension as usize,
        }),
    }
}

#[cfg(not(target_os = "macos"))]
fn native_rank(
    _query: &str,
    documents: &[String],
    _budget_ms: u32,
    _cached: &Index,
    _keys: &[String],
) -> Ranking {
    Ranking {
        scores: vec![None; documents.len()],
        vectors: vec![None; documents.len()],
        profile: None,
        report: SemanticReport::skipped("unsupported_platform", documents.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cosine_rejects_invalid_vectors_and_keeps_signed_similarity() {
        assert_eq!(cosine(&[1.0, 0.0], &[1.0, 0.0]), Some(1.0));
        assert_eq!(cosine(&[1.0, 0.0], &[-1.0, 0.0]), Some(-1.0));
        assert_eq!(cosine(&[1.0, 0.0], &[0.0, 1.0]), Some(0.0));
        for right in [
            vec![],
            vec![1.0],
            vec![0.0, 0.0],
            vec![f32::NAN, 1.0],
            vec![f32::INFINITY, 0.0],
        ] {
            assert!(cosine(&[1.0, 0.0], &right).is_none());
        }
    }
    #[tokio::test]
    async fn invalid_or_empty_requests_skip_native_work_explicitly() {
        assert_eq!(
            rank("test".into(), vec![], 100).await.report.status,
            "no_documents"
        );
        assert_eq!(
            rank("test".into(), vec!["doc".into()], 0)
                .await
                .report
                .status,
            "input_or_budget_limit"
        );
        assert_eq!(
            rank("长".repeat(401), vec!["doc".into()], 100)
                .await
                .report
                .status,
            "input_or_budget_limit"
        );
    }
    #[cfg(target_os = "macos")]
    #[tokio::test]
    #[ignore = "需本机已有 Apple 句向量；精确验证真实缓存复用，不下载资产"]
    async fn native_reuses_vectors_only_for_current_language_revision_and_dimensions() {
        use crate::command_audit::fingerprint;
        use serde_json::json;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("cache/index.json");
        let key = fingerprint(&json!("cached_document"));
        let run = || {
            rank_cached(
                "骑两轮车的人".into(),
                vec!["一位男子骑自行车经过街道".into()],
                3000,
                Some(path.clone()),
                vec![key.clone()],
            )
        };
        let cold = run().await;
        assert_eq!(cold.report.status, "available");
        assert_eq!(cold.report.generated, 1);
        assert_eq!(cold.report.cache_hits, 0);
        let profile = cold.profile.clone().unwrap();
        let entry = semantic_index::Entry {
            key: key.clone(),
            photo: fingerprint(&json!("photo")),
            vector: cold.vectors[0].clone().unwrap(),
        };
        semantic_index::publish(&path, profile.clone(), vec![entry.clone()]).unwrap();
        let warm = run().await;
        assert_eq!(warm.report.index_read, "loaded");
        assert_eq!(warm.report.cache_hits, 1);
        assert_eq!(warm.report.generated, 0);
        assert!((cold.scores[0].unwrap() - warm.scores[0].unwrap()).abs() < 1e-7);
        for invalid in [
            Profile {
                revision: profile.revision + 1,
                ..profile.clone()
            },
            Profile {
                language: 1,
                ..profile.clone()
            },
            Profile {
                dimensions: 2,
                ..profile.clone()
            },
        ] {
            let changed = semantic_index::Entry {
                vector: vec![1.0; invalid.dimensions],
                ..entry.clone()
            };
            semantic_index::publish(&path, invalid, vec![changed]).unwrap();
            let rebuilt = run().await;
            assert_eq!(rebuilt.report.cache_hits, 0);
            assert_eq!(rebuilt.report.generated, 1);
            assert_eq!(rebuilt.profile, Some(profile.clone()));
        }
        eprintln!(
            "real local index: cold generated=1; warm hits=1/generated=0; language/revision/dimensions mismatches rebuilt"
        );
    }
    #[cfg(target_os = "macos")]
    #[tokio::test]
    #[ignore = "需本机已有 Apple 中英文句向量；显式执行，不下载资产"]
    async fn native_chinese_and_english_rank_real_sentence_vectors() {
        for (query, documents) in [
            (
                "骑两轮车的人",
                vec![
                    "一位男子骑自行车经过街道",
                    "餐桌上放着一盘水果",
                    "雪山前停着一辆汽车",
                ],
            ),
            (
                "a person riding a bike",
                vec![
                    "A man is riding a bicycle on a street.",
                    "A plate of fruit sits on a dining table.",
                    "A snowy mountain beneath a blue sky.",
                ],
            ),
        ] {
            let ranked = rank(
                query.into(),
                documents.iter().map(|s| s.to_string()).collect(),
                3000,
            )
            .await;
            assert_eq!(
                ranked.report.status, "available",
                "需要已有本地模型，缺失不可算通过"
            );
            assert_eq!(ranked.report.embedded, 3);
            assert!(ranked.scores[0].unwrap() > ranked.scores[1].unwrap());
            assert!(ranked.scores[0].unwrap() > ranked.scores[2].unwrap());
            eprintln!(
                "local sentence embeddings: {}",
                serde_json::to_string(&ranked.report).unwrap()
            );
        }
        let limited = rank(
            "骑两轮车的人".into(),
            vec!["一位男子骑自行车经过街道".into(); 500],
            1,
        )
        .await;
        assert_eq!(limited.report.status, "time_limit");
        assert!(limited.report.limited && limited.report.embedded < 500);
        let unsupported = rank(
            "Ночной город".into(),
            vec!["一位男子骑自行车经过街道".into()],
            3000,
        )
        .await;
        assert_eq!(unsupported.report.status, "unsupported_language");
        assert_eq!(unsupported.report.embedded, 0);
    }
}
