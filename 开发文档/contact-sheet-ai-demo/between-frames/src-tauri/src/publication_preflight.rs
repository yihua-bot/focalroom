//! 印前检查：只在已入库版面上做只读计算，不改版面、不写库、不生成交付物。
//! 判定结果回代码与数值，中文说明由界面按当前语言生成。
use crate::publication_versions::{LayoutFrame, LayoutPage, LayoutSource, PublicationLayout};
use serde::Serialize;
use std::collections::BTreeSet;

/// 界面未指定时的最低输出分辨率。
pub const DEFAULT_MIN_PPI: i64 = 300;
/// 界面可要求的最低输出分辨率范围。
const MIN_PPI_FLOOR: i64 = 72;
const MIN_PPI_CEIL: i64 = 1200;
/// 单次返回的上限，避免整本书逐页报错时把 IPC 撑爆。
const MAX_ISSUES: usize = 200;
/// 跨页页面中央按书脊处理的宽度（毫米）：装订方式不在数据里，只给人工确认提示。
const SPINE_ZONE_MM: f64 = 10.0;
/// 裁切放大倍数超过这个数就提示，即使分辨率仍然达标。
const CROP_ENLARGE_HINT: f64 = 2.0;
/// 毫米级容差：比这更小的越线属于浮点噪声，不作为问题上报。
const TOLERANCE_MM: f64 = 0.05;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightIssue {
    pub code: &'static str,
    /// blocking / warning / info；只有 blocking 算"不能送印"。
    pub severity: &'static str,
    /// 全书级问题为 None，界面显示"整本"。
    pub page_number: Option<i64>,
    pub frame_id: Option<String>,
    pub text_id: Option<String>,
    /// 实测值：PPI、毫米或倍数，含义随 `code` 变化。
    pub measured: Option<f64>,
    /// 要求值，与 `measured` 同单位。
    pub required: Option<f64>,
}

/// 检查汇总。`blocking > 0` 即这份书稿还不能送印。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightReport {
    pub min_ppi: i64,
    pub pages: usize,
    pub frames: usize,
    pub texts: usize,
    pub blocking: usize,
    pub warnings: usize,
    pub infos: usize,
    pub truncated: bool,
    pub issues: Vec<PreflightIssue>,
}

/// 版面之外需要现场核对的事实：文件是否还在、序列是否都进了书。
#[derive(Debug, Default)]
pub struct OfflineFacts {
    /// 原图文件已不存在的资产 ID。
    pub missing_assets: BTreeSet<String>,
    /// 图像版本文件已不存在的 rendition ID。
    pub missing_renditions: BTreeSet<String>,
    /// 序列里已选、但书稿中没有任何图片框使用的照片数。
    pub unplaced_sequence: usize,
}

pub fn validate_min_ppi(value: Option<i64>) -> crate::domain::Result<i64> {
    let value = value.unwrap_or(DEFAULT_MIN_PPI);
    if (MIN_PPI_FLOOR..=MIN_PPI_CEIL).contains(&value) {
        Ok(value)
    } else {
        Err(crate::domain::AppError::new(
            "VALIDATION_ERROR",
            format!("最低分辨率要求须为 {MIN_PPI_FLOOR}–{MIN_PPI_CEIL} PPI"),
        ))
    }
}

fn frame_issue(
    code: &'static str,
    severity: &'static str,
    page: &LayoutPage,
    frame_id: &str,
) -> PreflightIssue {
    PreflightIssue {
        code,
        severity,
        page_number: Some(page.page_number),
        frame_id: Some(frame_id.to_owned()),
        text_id: None,
        measured: None,
        required: None,
    }
}

/// 实际送印的那份像素尺寸：选了图像版本就用版本，否则用原图。
pub(crate) fn delivered(source: &LayoutSource) -> (f64, f64) {
    match (source.rendition_width, source.rendition_height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => (w as f64, h as f64),
        _ => (source.width.max(0) as f64, source.height.max(0) as f64),
    }
}

/// 有效 PPI。裁切是等比放大，两个方向同时减少可用像素。
/// fit 整张放进框（实际印得更小，取较宽松的那个方向）；fill 裁切填满与 stretch 拉伸
/// 都按最吃紧的方向算。
/// 作品清单导出也用这一个函数：自检说分辨率不够、清单却写着够，用户就拿着这张纸去开机了。
pub(crate) fn effective_ppi(source: &LayoutSource, frame: &LayoutFrame) -> Option<f64> {
    let (px_w, px_h) = delivered(source);
    let inch_w = frame.width_mm / 25.4;
    let inch_h = frame.height_mm / 25.4;
    if px_w <= 0.0 || px_h <= 0.0 || inch_w <= 0.0 || inch_h <= 0.0 {
        return None;
    }
    let zoom = frame.crop_zoom.max(1.0);
    let wide = px_w / zoom / inch_w;
    let tall = px_h / zoom / inch_h;
    Some(match frame.fit_mode.as_str() {
        "fit" => wide.max(tall),
        _ => wide.min(tall),
    })
}

/// 内容是否越过裁切线；越过时给出"离出血外沿还差多少"和"越出裁切线多少"。
fn bleed_state(page: &LayoutPage, x: f64, y: f64, w: f64, h: f64) -> Option<(f64, f64)> {
    let right = x + w;
    let bottom = y + h;
    // 只有真的越过裁切线的那一侧才要求补出血；没越线的一侧不算。
    let sides = [
        (x < -TOLERANCE_MM, x + page.bleed_mm, -x),
        (y < -TOLERANCE_MM, y + page.bleed_mm, -y),
        (
            right > page.width_mm + TOLERANCE_MM,
            page.width_mm + page.bleed_mm - right,
            right - page.width_mm,
        ),
        (
            bottom > page.height_mm + TOLERANCE_MM,
            page.height_mm + page.bleed_mm - bottom,
            bottom - page.height_mm,
        ),
    ];
    if !sides.iter().any(|(crosses, _, _)| *crosses) {
        return None;
    }
    let deficit = sides
        .iter()
        .filter(|(crosses, _, _)| *crosses)
        .map(|(_, need, _)| *need)
        .fold(0.0_f64, f64::max);
    let overhang = sides
        .iter()
        .filter(|(crosses, _, _)| *crosses)
        .map(|(_, _, over)| *over)
        .fold(0.0_f64, f64::max);
    Some((deficit, overhang))
}

pub fn check(layout: &PublicationLayout, min_ppi: i64, offline: &OfflineFacts) -> PreflightReport {
    let mut issues: Vec<PreflightIssue> = vec![];
    let mut frames = 0_usize;
    let mut texts = 0_usize;
    let mut numbers: Vec<i64> = vec![];
    // 展览按"站在多远看"判分辨率，摄影书与作品集仍按印刷基线——那两种是拿在手里和送进印厂的。
    // 没记录观看距离时不猜一个数：继续按基线判，同时把"这条判据还挂在书本阈值上"说出去。
    let exhibition = layout.publication.pub_type == "exhibition";
    let distance_ppi = exhibition
        .then(|| {
            crate::publication_spec::read(&layout.publication.spec_json)
                .required_ppi(min_ppi as f64)
        })
        .flatten();
    if exhibition && distance_ppi.is_none() {
        issues.push(PreflightIssue {
            code: "viewing_distance_missing",
            severity: "warning",
            page_number: None,
            frame_id: None,
            text_id: None,
            measured: None,
            required: None,
        });
    }
    let reference = layout
        .pages
        .first()
        .map(|page| (page.width_mm, page.height_mm));

    for page in &layout.pages {
        numbers.push(page.page_number);
        if page.frames.is_empty() && page.texts.is_empty() {
            issues.push(frame_free("empty_page", "warning", page));
        }
        if reference.is_some_and(|size| size != (page.width_mm, page.height_mm)) {
            let mut item = frame_free("mixed_page_size", "warning", page);
            item.measured = Some(page.width_mm);
            item.required = reference.map(|size| size.0);
            issues.push(item);
        }

        for frame in &page.frames {
            frames += 1;
            let Some(source) = frame.source.as_ref() else {
                // 没关联照片，或照片/图像版本已不在库里：印出来只会是空白框。
                let code = if frame.asset_id.is_none() {
                    "frame_unbound"
                } else {
                    "source_missing"
                };
                issues.push(frame_issue(code, "blocking", page, &frame.id));
                continue;
            };
            // 要印的那份文件先确认存在：文件没了再算 PPI 没有意义。
            let offline_hit = match &source.rendition_id {
                Some(rendition) if offline.missing_renditions.contains(rendition) => {
                    Some("rendition_offline")
                }
                None if frame
                    .asset_id
                    .as_ref()
                    .is_some_and(|asset| offline.missing_assets.contains(asset)) =>
                {
                    Some("original_offline")
                }
                _ => None,
            };
            if let Some(code) = offline_hit {
                issues.push(frame_issue(code, "blocking", page, &frame.id));
            } else {
                if let Some(ppi) = effective_ppi(source, frame) {
                    // 展览用推算出的要求，其余一律用基线；两条路都只报"低于要求"这一件事。
                    let required = distance_ppi.unwrap_or(min_ppi as f64);
                    if ppi < required {
                        let code = if distance_ppi.is_some() {
                            "exhibition_low_resolution"
                        } else {
                            "low_resolution"
                        };
                        let mut item = frame_issue(code, "blocking", page, &frame.id);
                        item.measured = Some(ppi.floor());
                        item.required = Some(required.floor());
                        issues.push(item);
                    } else if frame.crop_zoom > CROP_ENLARGE_HINT {
                        let mut item = frame_issue("crop_enlargement", "info", page, &frame.id);
                        item.measured = Some(frame.crop_zoom);
                        item.required = Some(CROP_ENLARGE_HINT);
                        issues.push(item);
                    }
                }
                // 色彩配置看实际送印的那一份。
                if source.rendition_id.is_some() {
                    if source.icc_digest.is_none() {
                        issues.push(frame_issue(
                            "untagged_rendition",
                            "warning",
                            page,
                            &frame.id,
                        ));
                    }
                    if source.bit_depth.is_some_and(|depth| depth < 8) {
                        let mut item = frame_issue("shallow_bit_depth", "warning", page, &frame.id);
                        item.measured = source.bit_depth.map(|depth| depth as f64);
                        issues.push(item);
                    }
                } else if source.source_profile_digest.is_none() {
                    issues.push(frame_issue("untagged_source", "warning", page, &frame.id));
                }
            }
            if frame.fit_mode == "stretch" {
                issues.push(frame_issue("stretched_fit", "warning", page, &frame.id));
            }
            if let Some((deficit, overhang)) = bleed_state(
                page,
                frame.x_mm,
                frame.y_mm,
                frame.width_mm,
                frame.height_mm,
            ) {
                if page.bleed_mm > 0.0 {
                    if deficit > TOLERANCE_MM {
                        let mut item = frame_issue("bleed_shortfall", "blocking", page, &frame.id);
                        item.measured = Some(deficit);
                        item.required = Some(page.bleed_mm);
                        issues.push(item);
                    }
                } else {
                    // 这页没设出血，越出裁切线的部分会被整段裁掉：提醒人工确认是否为有意的满页裁切。
                    let mut item =
                        frame_issue("trim_crossing_no_bleed", "warning", page, &frame.id);
                    item.measured = Some(overhang);
                    issues.push(item);
                }
            }
            if page.side == "spread" {
                let center = page.width_mm / 2.0;
                if frame.x_mm < center + SPINE_ZONE_MM / 2.0
                    && frame.x_mm + frame.width_mm > center - SPINE_ZONE_MM / 2.0
                {
                    let mut item = frame_issue("spine_crossing", "info", page, &frame.id);
                    item.measured = Some(SPINE_ZONE_MM);
                    issues.push(item);
                }
            }
        }

        for text in &page.texts {
            texts += 1;
            // 文字没有"裁掉一点也没关系"的余地：出安全区就是问题。
            let worst = [
                page.margin_mm - text.x_mm,
                text.x_mm + text.width_mm - (page.width_mm - page.margin_mm),
                page.margin_mm - text.y_mm,
                text.y_mm + text.height_mm - (page.height_mm - page.margin_mm),
            ]
            .into_iter()
            .fold(0.0_f64, f64::max);
            if worst > TOLERANCE_MM {
                issues.push(PreflightIssue {
                    code: "text_outside_safety",
                    severity: "warning",
                    page_number: Some(page.page_number),
                    frame_id: None,
                    text_id: Some(text.id.clone()),
                    measured: Some(worst),
                    required: Some(page.margin_mm),
                });
            }
        }
    }

    numbers.sort_unstable();
    for pair in numbers.windows(2) {
        // 只看正文页码的连续性：封面是 0，不参与连续性判断。
        if pair[0] >= 1 && pair[1] > pair[0] + 1 {
            issues.push(PreflightIssue {
                code: "page_gap",
                severity: "warning",
                page_number: Some(pair[1]),
                frame_id: None,
                text_id: None,
                measured: Some(pair[0] as f64),
                required: Some(pair[1] as f64),
            });
        }
    }
    if offline.unplaced_sequence > 0 {
        issues.push(PreflightIssue {
            code: "sequence_not_placed",
            severity: "warning",
            page_number: None,
            frame_id: None,
            text_id: None,
            measured: None,
            required: Some(offline.unplaced_sequence as f64),
        });
    }

    let blocking = issues.iter().filter(|i| i.severity == "blocking").count();
    let warnings = issues.iter().filter(|i| i.severity == "warning").count();
    let infos = issues.iter().filter(|i| i.severity == "info").count();
    let truncated = issues.len() > MAX_ISSUES;
    issues.truncate(MAX_ISSUES);
    PreflightReport {
        min_ppi,
        pages: layout.pages.len(),
        frames,
        texts,
        blocking,
        warnings,
        infos,
        truncated,
        issues,
    }
}

fn frame_free(code: &'static str, severity: &'static str, page: &LayoutPage) -> PreflightIssue {
    PreflightIssue {
        code,
        severity,
        page_number: Some(page.page_number),
        frame_id: None,
        text_id: None,
        measured: None,
        required: None,
    }
}

/// 版面之外的事实核对：文件还在不在、序列有没有漏进书。
/// 只看这份版面引用到的资产与图像版本，不做哈希校验（那是检视与回流的事）。
impl crate::db::Store {
    pub async fn publication_preflight(
        &self,
        project: &str,
        publication: &str,
        min_ppi: Option<i64>,
    ) -> crate::domain::Result<PreflightReport> {
        let min_ppi = validate_min_ppi(min_ppi)?;
        let layout = self.publication_layout(project, publication).await?;
        let wanted: BTreeSet<&str> = layout
            .pages
            .iter()
            .flat_map(|page| &page.frames)
            .filter_map(|frame| frame.source.as_ref())
            .flat_map(|source| {
                [
                    Some(source.asset_id.as_str()),
                    source.rendition_id.as_deref(),
                ]
                .into_iter()
            })
            .flatten()
            .collect();
        let originals: Vec<(String, String)> = sqlx::query_as(
            "SELECT al.asset_id AS id, MIN(p.original_path) FROM asset_aliases al JOIN photos p ON p.id=al.photo_id WHERE p.project_id=? GROUP BY al.asset_id",
        )
        .bind(project)
        .fetch_all(&self.pool)
        .await?;
        let renditions: Vec<(String, String)> = sqlx::query_as(
            "SELECT r.id, r.file_path FROM renditions r JOIN edit_variants v ON v.id=r.variant_id JOIN source_assets a ON a.id=v.asset_id WHERE a.project_id=?",
        )
        .bind(project)
        .fetch_all(&self.pool)
        .await?;
        // 展览与作品集的"序列未入册"没有意义，只有摄影书按整本选片核对。
        let unplaced: i64 = if layout.publication.pub_type == "photobook" {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM (SELECT DISTINCT al.asset_id AS asset_id FROM sequence_items s JOIN asset_aliases al ON al.photo_id=s.photo_id JOIN source_assets a ON a.id=al.asset_id AND a.project_id=s.project_id WHERE s.project_id=?) q WHERE NOT EXISTS(SELECT 1 FROM publication_frames f JOIN publication_pages pg ON pg.id=f.page_id WHERE pg.publication_id=? AND f.asset_id=q.asset_id)",
            )
            .bind(project)
            .bind(publication)
            .fetch_one(&self.pool)
            .await?
        } else {
            0
        };
        let wanted: BTreeSet<String> = wanted.into_iter().map(str::to_owned).collect();
        let offline = tokio::task::spawn_blocking(move || {
            // 拿不到元数据或不是普通文件的路径一律视作不在：宁可多提示一次，也不说"可以送印"。
            let gone = |path: &str| match std::fs::symlink_metadata(path) {
                Ok(metadata) => !metadata.is_file(),
                Err(_) => true,
            };
            let mut missing_assets = BTreeSet::new();
            let mut missing_renditions = BTreeSet::new();
            for (id, path) in originals {
                if wanted.contains(&id) && (path.is_empty() || gone(&path)) {
                    missing_assets.insert(id);
                }
            }
            for (id, path) in renditions {
                if wanted.contains(&id) && (path.is_empty() || gone(&path)) {
                    missing_renditions.insert(id);
                }
            }
            OfflineFacts {
                missing_assets,
                missing_renditions,
                unplaced_sequence: unplaced.max(0) as usize,
            }
        })
        .await
        .map_err(|_| crate::domain::AppError::new("BACKGROUND", "印前检查未能完成，请稍后重试"))?;
        Ok(check(&layout, min_ppi, &offline))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publication_versions::{LayoutPage, LayoutPublication, PublicationLayout};

    fn sheet(number: i64, side: &str) -> LayoutPage {
        LayoutPage {
            id: format!("page-{number}"),
            page_number: number,
            side: side.to_owned(),
            width_mm: 210.0,
            height_mm: 297.0,
            margin_mm: 15.0,
            bleed_mm: 3.0,
            layout_json: None,
            frames: vec![],
            texts: vec![],
        }
    }

    fn src(width: i64, height: i64) -> LayoutSource {
        LayoutSource {
            asset_id: "asset-1".to_owned(),
            source_sha256: "a".repeat(64),
            width,
            height,
            source_profile_digest: Some("icc".to_owned()),
            rendition_id: None,
            variant_id: None,
            rendition_sha256: None,
            rendition_width: None,
            rendition_height: None,
            bit_depth: None,
            icc_digest: None,
            preview_sha256: None,
        }
    }

    fn frame(width_mm: f64, height_mm: f64, source: Option<LayoutSource>) -> LayoutFrame {
        LayoutFrame {
            id: "frame-1".to_owned(),
            x_mm: 15.0,
            y_mm: 15.0,
            width_mm,
            height_mm,
            fit_mode: "fill".to_owned(),
            asset_id: source.as_ref().map(|s| s.asset_id.clone()),
            rendition_id: None,
            crop_x: 0.5,
            crop_y: 0.5,
            crop_zoom: 1.0,
            source,
        }
    }

    fn layout(pages: Vec<LayoutPage>) -> PublicationLayout {
        PublicationLayout {
            schema_version: 1,
            publication: LayoutPublication {
                id: "pub-1".to_owned(),
                project_id: "project-1".to_owned(),
                title: "书".to_owned(),
                pub_type: "photobook".to_owned(),
                spec_json: "{\"width_mm\":210,\"height_mm\":297}".to_owned(),
                state: "draft".to_owned(),
            },
            pages,
        }
    }

    fn codes(report: &PreflightReport) -> Vec<&'static str> {
        report.issues.iter().map(|issue| issue.code).collect()
    }

    /// 一面 1200mm 宽的展墙不是 210mm 的一张纸：夹具得按它自己的尺寸开，否则报的全是越界。
    fn wall_sheet(number: i64, width_mm: f64, height_mm: f64) -> LayoutPage {
        let mut page = sheet(number, "right");
        page.width_mm = width_mm;
        page.height_mm = height_mm;
        page.bleed_mm = 0.0;
        page
    }

    fn wall_page(width_mm: f64, height_mm: f64, source: Option<LayoutSource>) -> LayoutPage {
        let mut page = wall_sheet(1, width_mm + 30.0, height_mm + 30.0);
        page.frames.push(frame(width_mm, height_mm, source));
        page
    }

    fn typed(pub_type: &str, spec_json: &str, page: LayoutPage) -> PublicationLayout {
        let mut book = layout(vec![page]);
        book.publication.pub_type = pub_type.to_owned();
        book.publication.spec_json = spec_json.to_owned();
        book
    }

    const WALL: &str = "{\"width_mm\":210,\"height_mm\":297,\"viewing_distance_mm\":2500}";
    const NO_DISTANCE: &str = "{\"width_mm\":210,\"height_mm\":297}";

    #[test]
    fn a_print_meant_for_a_wall_is_not_judged_by_a_book_in_the_hand() {
        // 600×400mm 的放印配 3000×2000 像素：约 127 PPI。按书本基线它永远"分辨率不足"，
        // 而站在 2.5 米外看，所需只有约 35 PPI——这才是这面墙上的判法。
        let wall = typed(
            "exhibition",
            WALL,
            wall_page(600.0, 400.0, Some(src(3000, 2000))),
        );
        let report = check(&wall, 300, &OfflineFacts::default());
        assert_eq!(
            codes(&report),
            Vec::<&str>::new(),
            "该过的墙不该被书本阈值拦下：{:?}",
            report.issues
        );
        // 同一帧、同一份像素，装进摄影书就仍是那条 300 的线。
        let book = typed(
            "photobook",
            WALL,
            wall_page(600.0, 400.0, Some(src(3000, 2000))),
        );
        let by_book = check(&book, 300, &OfflineFacts::default());
        assert_eq!(codes(&by_book), ["low_resolution"], "摄影书不看观看距离");
    }

    #[test]
    fn a_wall_that_still_fails_its_own_distance_says_so_with_its_own_numbers() {
        // 1200×800mm 只用 1200×800 像素：约 25 PPI，低于 2.5 米推出来的约 35。
        let wall = typed(
            "exhibition",
            WALL,
            wall_page(1200.0, 800.0, Some(src(1200, 800))),
        );
        let report = check(&wall, 300, &OfflineFacts::default());
        let issue = report
            .issues
            .iter()
            .find(|item| item.code == "exhibition_low_resolution")
            .expect("按观看距离判定不足时要点名这一帧");
        assert_eq!(issue.severity, "blocking");
        assert_eq!(issue.measured, Some(25.0));
        // 报的是这面墙自己的要求，不是那个 300——用户要知道的是"站在 2.5 米看差多少"。
        assert_eq!(issue.required, Some(34.0));
        assert!(
            !codes(&report).contains(&"low_resolution"),
            "同一条帧不该被两条分辨率规则各报一次：{:?}",
            codes(&report)
        );
        assert_eq!(report.blocking, 1);
    }

    #[test]
    fn an_exhibition_without_a_viewing_distance_says_which_bar_it_is_using() {
        // 没记录观看距离时不猜一个数：仍按 300 判，但必须把"这条判据还挂在书本阈值上"说出去。
        let wall = typed(
            "exhibition",
            NO_DISTANCE,
            wall_page(600.0, 400.0, Some(src(3000, 2000))),
        );
        let report = check(&wall, 300, &OfflineFacts::default());
        assert_eq!(
            codes(&report),
            ["viewing_distance_missing", "low_resolution"]
        );
        let notice = report
            .issues
            .iter()
            .find(|item| item.code == "viewing_distance_missing")
            .unwrap();
        assert_eq!(notice.severity, "warning");
        assert_eq!(notice.page_number, None, "这是整本规格的缺口，不是某一页的");
        assert_eq!(report.warnings, 1);
        assert_eq!(report.blocking, 1);
    }

    #[test]
    fn a_stray_distance_in_a_photobook_spec_does_not_loosen_the_photobook() {
        // 摄影书是拿在手里和送进印厂的：spec 里混进观看距离也不该把它的线放低。
        let book = typed(
            "photobook",
            WALL,
            wall_page(1200.0, 800.0, Some(src(1200, 800))),
        );
        let report = check(&book, 300, &OfflineFacts::default());
        assert!(codes(&report).contains(&"low_resolution"));
        assert!(!codes(&report).contains(&"exhibition_low_resolution"));
        assert_eq!(report.warnings, 0, "摄影书不问观看距离缺没缺");
    }

    #[test]
    fn an_unreadable_spec_leaves_the_book_on_the_print_baseline_without_crashing() {
        for junk in ["", "[]", "{\"viewing_distance_mm\":\"很远\"}"] {
            let wall = typed(
                "exhibition",
                junk,
                wall_page(600.0, 400.0, Some(src(3000, 2000))),
            );
            let report = check(&wall, 300, &OfflineFacts::default());
            assert_eq!(
                codes(&report),
                ["viewing_distance_missing", "low_resolution"],
                "{junk}"
            );
        }
    }

    #[test]
    fn resolution_is_measured_against_the_printed_size_in_millimetres() {
        // 3000×2000 像素放在 100×66.7mm 的框里：100mm=3.937in → 762 PPI（宽向），
        // 66.7mm=2.626in → 762（高向）；取最吃紧方向。
        let mut page = sheet(1, "right");
        page.frames.push(frame(100.0, 66.7, Some(src(3000, 2000))));
        let report = check(&layout(vec![page]), 300, &OfflineFacts::default());
        assert_eq!(
            codes(&report),
            Vec::<&str>::new(),
            "达标版面不该报任何分辨率问题"
        );

        // 同样这张图放到 200mm 宽：3000/(200/25.4)=381 PPI 仍达标；3000/300mm=254 就不达标。
        let mut large = sheet(2, "right");
        large
            .frames
            .push(frame(300.0, 200.0, Some(src(3000, 2000))));
        let report = check(&layout(vec![large]), 300, &OfflineFacts::default());
        let low = report
            .issues
            .iter()
            .find(|issue| issue.code == "low_resolution")
            .expect("254 PPI 必须报不足");
        assert_eq!(low.severity, "blocking");
        assert_eq!(low.measured, Some(254.0), "按最吃紧方向算并向下取整");
        assert_eq!(low.required, Some(300.0));
    }

    #[test]
    fn cropping_reduces_available_pixels_and_fit_is_the_looser_case() {
        let mut page = sheet(1, "right");
        // 3000×2000 放进 150×100mm：fill 时宽向 508、高向 508，达标；放大 3 倍后只有 169。
        let mut zoomed = frame(150.0, 100.0, Some(src(3000, 2000)));
        zoomed.crop_zoom = 3.0;
        page.frames.push(zoomed);
        let report = check(&layout(vec![page]), 300, &OfflineFacts::default());
        let low = codes(&report);
        assert!(low.contains(&"low_resolution"), "3 倍裁切应报分辨率不足");

        // fit 是整张放进框：实际印得比框小，像素密度更高，同一个框同一个源不该再报。
        let mut fit_page = sheet(4, "right");
        let mut fit = frame(300.0, 60.0, Some(src(3000, 2000)));
        fit.fit_mode = "fit".to_owned();
        fit_page.frames.push(fit);
        let report = check(&layout(vec![fit_page]), 300, &OfflineFacts::default());
        assert!(
            !codes(&report).contains(&"low_resolution"),
            "fit 印得比框小，按框算是最坏情况"
        );
    }

    #[test]
    fn delivered_rendition_replaces_the_original_and_its_own_gaps_are_reported() {
        let mut page = sheet(1, "right");
        let mut source = src(3000, 2000);
        source.rendition_id = Some("rendition-1".to_owned());
        source.rendition_sha256 = Some("b".repeat(64));
        // 图像版本只有 600×400：送印按这份算，而不是原图。
        source.rendition_width = Some(600);
        source.rendition_height = Some(400);
        source.icc_digest = None;
        source.bit_depth = Some(8);
        let mut item = frame(150.0, 100.0, Some(source));
        item.rendition_id = Some("rendition-1".to_owned());
        page.frames.push(item);
        let report = check(&layout(vec![page]), 300, &OfflineFacts::default());
        let found = codes(&report);
        assert!(found.contains(&"low_resolution"), "按版本像素判定");
        assert!(
            found.contains(&"untagged_rendition"),
            "版本没带配置就报未标注，而不是看原图"
        );
        assert!(!found.contains(&"untagged_source"));

        // 版本文件已经不在了：报失联，不再给一个基于缺失文件的 PPI。
        let mut page = sheet(2, "right");
        let mut source = src(3000, 2000);
        source.rendition_id = Some("rendition-2".to_owned());
        source.rendition_width = Some(600);
        source.rendition_height = Some(400);
        source.icc_digest = Some("icc".to_owned());
        page.frames.push(frame(150.0, 100.0, Some(source)));
        let offline = OfflineFacts {
            missing_renditions: ["rendition-2".to_owned()].into_iter().collect(),
            ..Default::default()
        };
        let report = check(&layout(vec![page]), 300, &offline);
        assert_eq!(codes(&report), ["rendition_offline"]);
        assert_eq!(report.blocking, 1);
    }

    #[test]
    fn frames_without_a_photo_and_empty_pages_are_blocking_or_warned() {
        let mut page = sheet(1, "right");
        page.frames.push(frame(100.0, 100.0, None));
        let mut loose = frame(100.0, 100.0, None);
        loose.asset_id = Some("gone".to_owned());
        page.frames.push(loose);
        let report = check(&layout(vec![page]), 300, &OfflineFacts::default());
        let found = codes(&report);
        assert!(found.contains(&"frame_unbound"), "没关联照片是空白框");
        assert!(
            found.contains(&"source_missing"),
            "照片已不在库里也是空白框"
        );
        assert_eq!(report.blocking, 2);
        assert!(!found.contains(&"empty_page"), "有框就不算空页");

        let report = check(
            &layout(vec![sheet(7, "right")]),
            300,
            &OfflineFacts::default(),
        );
        assert_eq!(codes(&report), ["empty_page"]);
    }

    #[test]
    fn bleed_rules_separate_missing_bleed_from_zero_bleed_trim_crossing() {
        // 越过裁切线 1mm，但出血要求 3mm：还差 2mm → 阻断。
        let mut page = sheet(3, "right");
        let mut item = frame(100.0, 200.0, Some(src(6000, 4000)));
        item.x_mm = -1.0;
        item.y_mm = 0.0;
        page.frames.push(item);
        let report = check(&layout(vec![page]), 300, &OfflineFacts::default());
        let bleed = report
            .issues
            .iter()
            .find(|issue| issue.code == "bleed_shortfall")
            .expect("出血不足必须阻断");
        assert_eq!(bleed.severity, "blocking");
        assert_eq!(bleed.measured, Some(2.0));
        assert_eq!(bleed.required, Some(3.0));
        assert_eq!(report.blocking, 1, "只该有出血这一条阻断");

        // 铺满出血：不报出血问题。
        let mut page = sheet(4, "right");
        let mut item = frame(216.0, 303.0, Some(src(6000, 4000)));
        item.x_mm = -3.0;
        item.y_mm = -3.0;
        page.frames.push(item);
        let report = check(&layout(vec![page]), 300, &OfflineFacts::default());
        assert!(
            report.issues.is_empty(),
            "铺到出血外沿且分辨率达标的满页图不该任何问题：{:?}",
            report.issues
        );

        // 这页根本没设出血，内容却越出裁切线：会被裁掉，提醒而不是阻断。
        let mut page = sheet(5, "right");
        page.bleed_mm = 0.0;
        let mut item = frame(220.0, 300.0, Some(src(8000, 6000)));
        item.x_mm = -5.0;
        page.frames.push(item);
        let report = check(&layout(vec![page]), 300, &OfflineFacts::default());
        let found = codes(&report);
        assert!(found.contains(&"trim_crossing_no_bleed"));
        assert!(!found.contains(&"bleed_shortfall"));
        assert_eq!(report.blocking, 0, "无出血的满页裁切不该判成不能送印");
    }

    #[test]
    fn text_safety_spine_and_stretch_rules_fire_with_numbers() {
        let mut page = sheet(6, "spread");
        let mut item = frame(100.0, 100.0, Some(src(3000, 2000)));
        item.fit_mode = "stretch".to_owned();
        // 跨页中央：书脊区域正好压在图上。
        item.x_mm = 95.0;
        page.frames.push(item);
        page.texts.push(crate::publication_versions::LayoutText {
            id: "text-1".to_owned(),
            x_mm: 10.0,
            y_mm: 20.0,
            width_mm: 50.0,
            height_mm: 20.0,
            content_json: "{\"text\":\"图注\"}".to_owned(),
            style_json: None,
        });
        let report = check(&layout(vec![page]), 300, &OfflineFacts::default());
        let found = codes(&report);
        assert!(found.contains(&"stretched_fit"), "非等比拉伸");
        assert!(found.contains(&"spine_crossing"), "跨页中央是装订线");
        let safety = report
            .issues
            .iter()
            .find(|issue| issue.code == "text_outside_safety")
            .expect("文字进 bleed 前先进安全区外");
        assert_eq!(safety.measured, Some(5.0), "左侧超出安全区 5mm");
        assert_eq!(safety.required, Some(15.0));
        assert_eq!(safety.text_id.as_deref(), Some("text-1"));
    }

    #[test]
    fn book_level_findings_are_page_independent_and_severities_are_counted() {
        let mut first = sheet(1, "right");
        first
            .frames
            .push(frame(150.0, 100.0, Some(src(3000, 2000))));
        let mut third = sheet(3, "left");
        third.width_mm = 200.0;
        third
            .frames
            .push(frame(150.0, 100.0, Some(src(3000, 2000))));
        let offline = OfflineFacts {
            unplaced_sequence: 4,
            ..Default::default()
        };
        let report = check(&layout(vec![first, third]), 300, &offline);
        let found = codes(&report);
        assert!(found.contains(&"page_gap"), "页码 1、3 之间漏页");
        assert!(found.contains(&"mixed_page_size"), "各页开本不一致");
        let unplaced = report
            .issues
            .iter()
            .find(|issue| issue.code == "sequence_not_placed")
            .expect("序列里还有没进书的照片");
        assert_eq!(unplaced.page_number, None, "全书级问题不挂页码");
        assert_eq!(unplaced.required, Some(4.0));
        assert_eq!(report.pages, 2);
        assert_eq!(report.frames, 2);
        assert_eq!(report.blocking, 0);
        assert_eq!(report.warnings, 3);
    }

    #[test]
    fn requested_resolution_is_validated_and_report_is_bounded() {
        assert_eq!(validate_min_ppi(None).unwrap(), DEFAULT_MIN_PPI);
        assert_eq!(validate_min_ppi(Some(150)).unwrap(), 150);
        assert_eq!(
            validate_min_ppi(Some(60)).unwrap_err().code,
            "VALIDATION_ERROR"
        );
        assert_eq!(
            validate_min_ppi(Some(1201)).unwrap_err().code,
            "VALIDATION_ERROR"
        );

        // 250 页每页一个空白框：只回前 200 条并说明被截断。
        let pages = (1..=250_i64)
            .map(|number| {
                let mut page = sheet(number, "right");
                page.frames.push(frame(100.0, 100.0, None));
                page
            })
            .collect();
        let report = check(&layout(pages), 300, &OfflineFacts::default());
        assert_eq!(report.issues.len(), MAX_ISSUES);
        assert!(report.truncated);
    }
}
