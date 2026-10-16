//! 临时本地文本向量；不持久化、不发网络请求、不将相似度当作事实置信度。
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
        }
    }
}

pub(crate) struct Ranking {
    pub scores: Vec<Option<f64>>,
    pub report: SemanticReport,
}

pub(crate) async fn rank(query: String, documents: Vec<String>, budget_ms: u32) -> Ranking {
    let count = documents.len();
    if count == 0
        || count > MAX_DOCUMENTS
        || query.chars().count() > 400
        || query.contains('\0')
        || budget_ms == 0
    {
        return Ranking {
            scores: vec![None; count],
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
            report: SemanticReport::skipped("busy", count),
        };
    };
    let result = tokio::task::spawn_blocking(move || {
        let _slot = slot;
        native_rank(&query, &documents, budget_ms)
    })
    .await;
    result.unwrap_or_else(|_| Ranking {
        scores: vec![None; count],
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
fn native_rank(query: &str, documents: &[String], budget_ms: u32) -> Ranking {
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
            report,
        };
    };
    let pointers: Vec<_> = texts.iter().map(|s| s.as_ptr()).collect();
    let mut vectors = vec![0_f32; texts.len() * STRIDE];
    let mut available = vec![0_u8; texts.len()];
    let mut info = Info::default();
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
    report.embedded = scores.iter().flatten().count();
    report.limited = report.embedded < documents.len();
    Ranking { scores, report }
}

#[cfg(not(target_os = "macos"))]
fn native_rank(_query: &str, documents: &[String], _budget_ms: u32) -> Ranking {
    Ranking {
        scores: vec![None; documents.len()],
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
    }
}
