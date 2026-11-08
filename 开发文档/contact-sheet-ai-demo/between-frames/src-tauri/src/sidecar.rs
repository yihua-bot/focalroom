//! 外部软件（Lightroom / Capture One）写在原图旁边的 XMP 边车：只读，
//! 取出摄影师的评分、选片旗标与色标签，落成"待确认的事实"，绝不自动改写本地判断。

use crate::domain::{AppError, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// 边车按文本扫；超过这个大小就不当成"一张照片的判断"，直接如实报错。
const MAX_SIDECAR_BYTES: u64 = 2 * 1024 * 1024;

/// 一条照片记录在台账里的当前判断，加上它自己的原图路径（边车位置由这条路径推出，不由界面提供）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhotoJudgement {
    pub photo_id: String,
    pub filename: String,
    pub original_path: String,
    pub status: String,
    pub rating: i64,
}

/// 边车里读到的判断。只认这三个字段，其余（曝光、裁剪、色标签的名字）一律不猜、不落库。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarJudgement {
    pub rating: Option<i64>,
    pub pick: Option<bool>,
    pub color_label: Option<i64>,
    /// 几个命名空间都写了评分但数值不同：宁可报出来让人去外部软件统一，也不自动挑一个。
    pub ambiguous_rating: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidecarSource {
    pub path: String,
    pub judgement: SidecarJudgement,
}

/// 待确认的一条回填建议：本地现在是什么、边车说的是什么、能不能落地。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarProposal {
    pub photo_id: String,
    pub filename: String,
    pub sidecar_path: String,
    pub sidecar_rating: Option<i64>,
    pub local_rating: i64,
    pub applies_rating: bool,
    pub sidecar_pick: Option<bool>,
    pub local_status: String,
    pub applies_status: bool,
    /// 本地台账没有色标签这一列，所以只报出来，不塞进别的字段里冒充。
    pub color_label: Option<i64>,
    pub ambiguous_rating: bool,
    /// 一键回填够得着的那部分：本地这条还空着，填进去不会盖掉人工的判断。
    pub batch_safe: bool,
}

impl SidecarProposal {
    /// 一键回填时这条该落成什么（评分、状态）。
    /// 只碰"本地还空着"的半边：评过分、或已经判过保留/淘汰的，一律留给逐条确认——
    /// 那是人自己表过态的地方，程序不该拿外部软件的一句话去改主意。
    pub fn batch_fill(&self) -> Option<(i64, &str)> {
        if self.ambiguous_rating {
            return None;
        }
        let fill_rating = self.applies_rating && self.local_rating == 0;
        let fill_status = self.applies_status && self.local_status == "undecided";
        if !(fill_rating || fill_status) {
            return None;
        }
        Some((
            if fill_rating {
                self.sidecar_rating.unwrap_or(self.local_rating)
            } else {
                self.local_rating
            },
            if fill_status {
                "keep"
            } else {
                self.local_status.as_str()
            },
        ))
    }
}

/// 一键回填的回执：落成几条、跳过几条，外加项目现在的样子。
/// 跳过的那几条不是失败，是"读盘之后你自己动过判断"，程序不盖。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarBatchOutcome {
    pub applied: i64,
    pub skipped: i64,
    pub project: crate::domain::Project,
}

/// 属性写法（`<x crs:Rating="4">`）与元素写法（`<crs:Rating>4</crs:Rating>`）都认；
/// 属性只在标签内部取，元素文本只在它自己的结束标签之间取，
/// 所以备注正文里出现 `crs:Rating="5"` 不会被当成评分。
fn attribute_values(text: &str, name: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('>') else {
            break;
        };
        let tag = &after_open[..close];
        let mut offset = 0usize;
        while let Some(relative) = tag[offset..].find(name) {
            let at = offset + relative;
            let value_start = at + name.len() + 2;
            // 名称前必须是空白（说明它是标签里的一个属性），名称后紧跟 ="。
            let quoted = (at > 0
                && tag.as_bytes()[at - 1].is_ascii_whitespace()
                && tag.get(at + name.len()..value_start) == Some("=\""))
            .then(|| tag.get(value_start..))
            .flatten()
            .and_then(|tail| {
                tail.find('"')
                    .map(|end| (tail[..end].to_owned(), value_start + end + 1))
            });
            match quoted {
                Some((value, next)) => {
                    found.push(value);
                    offset = next;
                }
                None => offset = at + name.len(),
            }
        }
        rest = &after_open[close + 1..];
    }
    found
}

fn element_text_values(text: &str, name: &str) -> Vec<String> {
    let open = format!("<{name}");
    let close = format!("</{name}>");
    let mut found = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative) = text[cursor..].find(open.as_str()) {
        let after = cursor + relative + open.len();
        let Some(tag_end) = text[after..].find('>') else {
            break;
        };
        let head = &text[after..after + tag_end];
        cursor = after + tag_end + 1;
        // 只认 <name> 与 <name 属性>；<nameX> 和自闭合标签都没有要取的文本。
        if !(head.is_empty() || head.as_bytes()[0].is_ascii_whitespace()) || head.ends_with('/') {
            continue;
        }
        let Some(body_end) = text[cursor..].find(close.as_str()) else {
            break;
        };
        let body = text[cursor..cursor + body_end].trim();
        if !body.is_empty() && !body.contains('<') {
            found.push(body.to_owned());
        }
        cursor += body_end + close.len();
    }
    found
}

fn read_ints(text: &str, name: &str) -> Vec<i64> {
    attribute_values(text, name)
        .into_iter()
        .chain(element_text_values(text, name))
        .filter_map(|raw| raw.trim().parse::<i64>().ok())
        .collect()
}

pub fn parse_xmp(text: &str) -> SidecarJudgement {
    let mut ratings: Vec<i64> = Vec::new();
    for name in [
        "xmp:Rating",
        "crs:Rating",
        "mp:Rating",
        "MicrosoftPhoto:Rating",
    ] {
        for value in read_ints(text, name) {
            // 超出 0–5 的值不是我们能落库的评分（有些软件写 255 表示"拒绝"），忽略。
            if (0..=5).contains(&value) && !ratings.contains(&value) {
                ratings.push(value);
            }
        }
    }
    let pick = ["crs:Pick", "xmp:Pick"]
        .iter()
        .find_map(|name| read_ints(text, name).into_iter().find_map(pick_of));
    SidecarJudgement {
        rating: if ratings.len() == 1 {
            ratings.first().copied()
        } else {
            None
        },
        pick,
        color_label: read_ints(text, "crs:ColorLabel")
            .into_iter()
            .find(|value| (0..=8).contains(value)),
        ambiguous_rating: ratings.len() > 1,
    }
}

fn pick_of(value: i64) -> Option<bool> {
    match value {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

/// Lightroom 存的是 `<名>.xmp`，也有软件在原文件名后再加一段 `.xmp`，两种都找。
fn candidates(original: &Path) -> Vec<PathBuf> {
    let Some(directory) = original.parent() else {
        return Vec::new();
    };
    let mut list = Vec::new();
    for base in [original.file_stem(), original.file_name()]
        .into_iter()
        .flatten()
    {
        let mut name = base.to_os_string();
        name.push(".xmp");
        list.push(directory.join(name));
    }
    list
}

pub fn read_sidecar(original: &Path) -> Result<Option<SidecarSource>> {
    for candidate in candidates(original) {
        let metadata = match std::fs::metadata(&candidate) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                return Err(AppError::new(
                    "SIDECAR_UNREADABLE",
                    "旁边的 XMP 文件读不到，请检查这个文件",
                ));
            }
        };
        if metadata.len() > MAX_SIDECAR_BYTES {
            return Err(AppError::new(
                "SIDECAR_TOO_LARGE",
                "旁边的 XMP 文件过大，不当成这张照片的判断",
            ));
        }
        let bytes = std::fs::read(&candidate).map_err(|_| {
            AppError::new(
                "SIDECAR_UNREADABLE",
                "旁边的 XMP 文件读不到，请检查这个文件",
            )
        })?;
        return Ok(Some(SidecarSource {
            path: candidate.to_string_lossy().into_owned(),
            judgement: parse_xmp(&String::from_utf8_lossy(&bytes)),
        }));
    }
    Ok(None)
}

/// 把"边车说的"与"台账记的"并列成待确认清单；落不落库由人点头。
pub fn proposals_for(rows: &[PhotoJudgement]) -> Result<Vec<SidecarProposal>> {
    let mut proposals = Vec::new();
    for row in rows {
        let Some(source) = read_sidecar(Path::new(&row.original_path))? else {
            continue;
        };
        let judgement = &source.judgement;
        let applies_rating = !judgement.ambiguous_rating
            && judgement.rating.is_some_and(|rating| rating != row.rating);
        // 旗标只有"选中"这一半能安全落地：边车写着没选中，不等于你判了淘汰。
        let applies_status = judgement.pick == Some(true) && row.status != "keep";
        if !(applies_rating || applies_status || judgement.ambiguous_rating) {
            continue;
        }
        proposals.push(SidecarProposal {
            photo_id: row.photo_id.clone(),
            filename: row.filename.clone(),
            sidecar_path: source.path,
            sidecar_rating: judgement.rating,
            local_rating: row.rating,
            applies_rating,
            sidecar_pick: judgement.pick,
            local_status: row.status.clone(),
            applies_status,
            color_label: judgement.color_label,
            ambiguous_rating: judgement.ambiguous_rating,
            batch_safe: false,
        });
    }
    // 批量够不够得着由同一套规则算，界面与落库不会各判各的。
    for proposal in &mut proposals {
        proposal.batch_safe = proposal.batch_fill().is_some();
    }
    Ok(proposals)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sidecar(directory: &Path, name: &str, body: &str) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn attribute_and_element_forms_both_read() {
        let attribute = parse_xmp(
            "<x:xmpmeta><rdf:RDF><rdf:Description xmp:Rating=\"4\" crs:Pick=\"1\" crs:ColorLabel=\"6\"/></rdf:RDF></x:xmpmeta>",
        );
        assert_eq!(
            attribute,
            SidecarJudgement {
                rating: Some(4),
                pick: Some(true),
                color_label: Some(6),
                ambiguous_rating: false
            }
        );
        let element =
            parse_xmp("<x:xmpmeta><crs:Rating>3</crs:Rating><crs:Pick>0</crs:Pick></x:xmpmeta>");
        assert_eq!(element.rating, Some(3));
        assert_eq!(element.pick, Some(false));
        assert_eq!(element.color_label, None);
    }

    #[test]
    fn two_namespaces_disagreeing_on_rating_are_reported_not_resolved() {
        let judgement = parse_xmp("<rdf:Description xmp:Rating=\"5\" crs:Rating=\"2\"/>");
        assert!(judgement.ambiguous_rating);
        assert_eq!(judgement.rating, None, "不一致时不挑一个当成摄影师的意思");
    }

    #[test]
    fn values_that_are_not_a_rating_never_become_one() {
        // 有些软件用 255 表示拒绝，用 -1 表示"未评"；这些都不能落成 0–5 里的某个数。
        let judgement =
            parse_xmp("<rdf:Description xmp:Rating=\"255\" crs:Pick=\"7\" crs:ColorLabel=\"99\"/>");
        assert_eq!(judgement, SidecarJudgement::default());
    }

    #[test]
    fn rating_looking_text_in_a_caption_is_not_a_judgement() {
        let parsed = parse_xmp(
            "<xmp:Description><dc:description>别把 crs:Rating=\"5\" 当成评分</dc:description></xmp:Description>",
        );
        assert_eq!(
            parsed.rating, None,
            "正文里的字符串不是属性，也不该被当成评分"
        );
    }

    #[test]
    fn a_sibling_named_after_the_full_filename_is_found_too() {
        let directory = tempfile::tempdir().unwrap();
        let photo = sidecar(directory.path(), "frame-007.jpg", "jpeg bytes");
        sidecar(
            directory.path(),
            "frame-007.jpg.xmp",
            "<rdf:Description crs:Rating=\"2\"/>",
        );
        let source = read_sidecar(&photo).unwrap().expect("两种命名都该找到");
        assert_eq!(source.judgement.rating, Some(2));
        assert!(source.path.ends_with("frame-007.jpg.xmp"));
    }

    #[test]
    fn proposals_separate_writable_facts_from_ones_there_is_no_column_for() {
        let directory = tempfile::tempdir().unwrap();
        let kept = sidecar(directory.path(), "kept.jpg", "x");
        sidecar(
            directory.path(),
            "kept.xmp",
            "<rdf:Description crs:Rating=\"4\" crs:Pick=\"1\" crs:ColorLabel=\"6\"/>",
        );
        let already_in_sync = sidecar(directory.path(), "synced.jpg", "x");
        sidecar(
            directory.path(),
            "synced.xmp",
            "<rdf:Description xmp:Rating=\"3\"/>",
        );
        let no_sidecar = sidecar(directory.path(), "lonely.jpg", "x");
        let conflicting = sidecar(directory.path(), "conflict.jpg", "x");
        sidecar(
            directory.path(),
            "conflict.xmp",
            "<rdf:Description xmp:Rating=\"5\" crs:Rating=\"1\"/>",
        );
        let rows = [
            PhotoJudgement {
                photo_id: "one".into(),
                filename: "kept.jpg".into(),
                original_path: kept.to_string_lossy().into_owned(),
                status: "undecided".into(),
                rating: 0,
            },
            PhotoJudgement {
                photo_id: "two".into(),
                filename: "synced.jpg".into(),
                original_path: already_in_sync.to_string_lossy().into_owned(),
                status: "keep".into(),
                rating: 3,
            },
            PhotoJudgement {
                photo_id: "three".into(),
                filename: "lonely.jpg".into(),
                original_path: no_sidecar.to_string_lossy().into_owned(),
                status: "undecided".into(),
                rating: 0,
            },
            PhotoJudgement {
                photo_id: "four".into(),
                filename: "conflict.jpg".into(),
                original_path: conflicting.to_string_lossy().into_owned(),
                status: "keep".into(),
                rating: 2,
            },
        ];
        let proposals = proposals_for(&rows).unwrap();
        assert_eq!(proposals.len(), 2, "一致的与没有边车的都不该出现在清单上");
        let first = &proposals[0];
        assert_eq!(first.photo_id, "one");
        assert!(first.applies_rating && first.applies_status);
        assert_eq!(first.sidecar_rating, Some(4));
        assert_eq!(first.local_rating, 0);
        assert_eq!(first.local_status, "undecided");
        assert_eq!(first.color_label, Some(6), "色标签只报事实，本地没有这一列");
        assert_eq!(proposals[1].photo_id, "four");
        assert!(
            !proposals[1].applies_rating && proposals[1].ambiguous_rating,
            "冲突的评分只报出来，不给可写的值"
        );
    }

    fn proposal(local_status: &str, local_rating: i64, body: &str) -> SidecarProposal {
        let directory = tempfile::tempdir().unwrap();
        let photo = sidecar(directory.path(), "frame.jpg", "x");
        // keep the tempdir alive until the sidecar is read
        let path = photo.to_string_lossy().into_owned();
        sidecar(directory.path(), "frame.xmp", body);
        let rows = [PhotoJudgement {
            photo_id: "one".into(),
            filename: "frame.jpg".into(),
            original_path: path,
            status: local_status.into(),
            rating: local_rating,
        }];
        let mut proposals = proposals_for(&rows).unwrap();
        assert_eq!(proposals.len(), 1, "夹具该出现在清单上");
        // 目录在这里还活着，读完才允许回收。
        let found = proposals.remove(0);
        drop(directory);
        found
    }

    #[test]
    fn batch_backfill_reaches_only_the_rows_the_photographer_has_not_touched() {
        // 本地空着：一键填进去，评分与选片都落。
        let blank = proposal(
            "undecided",
            0,
            "<rdf:Description crs:Rating=\"4\" crs:Pick=\"1\"/>",
        );
        assert!(blank.batch_safe);
        assert_eq!(blank.batch_fill(), Some((4, "keep")));
        // 人工已经评过分：这条只能自己点头，一键不替他改。
        let rated = proposal("undecided", 2, "<rdf:Description crs:Rating=\"4\"/>");
        assert!(rated.applies_rating, "逐条确认还是给得出这个数");
        assert!(!rated.batch_safe);
        assert_eq!(rated.batch_fill(), None);
        // 人工判过淘汰：外部写着"选中"也不盖状态，但空着的评分照样能填。
        let rejected = proposal(
            "reject",
            0,
            "<rdf:Description crs:Rating=\"3\" crs:Pick=\"1\"/>",
        );
        assert!(rejected.applies_status, "逐条确认可以把这条捞回保留");
        assert_eq!(rejected.batch_fill(), Some((3, "reject")));
        // 两处评分互相矛盾：一键填不出确定的数，连本地空着也不动。
        let conflicted = proposal(
            "undecided",
            0,
            "<rdf:Description xmp:Rating=\"5\" crs:Rating=\"1\"/>",
        );
        assert!(conflicted.ambiguous_rating);
        assert!(!conflicted.batch_safe);
        assert_eq!(conflicted.batch_fill(), None);
    }

    #[test]
    fn an_oversized_sidecar_is_refused_instead_of_guessed() {
        let directory = tempfile::tempdir().unwrap();
        let photo = sidecar(directory.path(), "huge.jpg", "x");
        let mut body = "<rdf:Description crs:Rating=\"4\"/>".to_string();
        body.push_str(&" ".repeat(MAX_SIDECAR_BYTES as usize));
        sidecar(directory.path(), "huge.xmp", &body);
        let error = read_sidecar(&photo).unwrap_err();
        assert_eq!(error.code, "SIDECAR_TOO_LARGE");
    }
}
