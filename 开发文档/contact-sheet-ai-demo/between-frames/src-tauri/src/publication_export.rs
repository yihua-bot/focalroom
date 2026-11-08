// 交付出口：把整本版面摊成一张逐条可核对的作品清单（CSV）。
// 像素数与有效 PPI 一律复用印前那套算法，不在这里另算一遍——两份交付依据数字打架时，
// 用户无从判断该信哪一个，而印厂只会按其中一份开机。
use std::path::Path;

use serde_json::Value;

use crate::domain::{AppError, Result};
use crate::export::ExportReceipt;
use crate::image_pipeline::fingerprint;
use crate::publication_preflight::{delivered, effective_ppi};
use crate::publication_versions::{LayoutFrame, LayoutPage, LayoutText, PublicationLayout};

const HEADER: [&str; 15] = [
    "page_number",
    "side",
    "kind",
    "id",
    "x_mm",
    "y_mm",
    "width_mm",
    "height_mm",
    "delivered_width_px",
    "delivered_height_px",
    "effective_ppi",
    "crop_zoom",
    "fit_mode",
    "text",
    "asset_id",
];

/// RFC4180：只有真的含分隔符、引号或换行时才加引号，其余保持可读。
fn cell(raw: &str) -> String {
    if raw.contains(['"', ',', '\n', '\r']) {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw.to_owned()
    }
}

fn number(value: f64) -> String {
    if !value.is_finite() {
        return String::new();
    }
    format!("{value:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

fn optional(value: Option<&String>) -> String {
    value.cloned().unwrap_or_default()
}

/// 版面里的文字格存的是 JSON；读不出结构时留空，不把原始 JSON 甩进表格冒充图注。
fn text_of(content_json: &str) -> String {
    match serde_json::from_str::<Value>(content_json) {
        Ok(Value::Object(map)) => match map.get("text") {
            Some(Value::String(text)) => text.clone(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

// 同一页上先照片后文字，各自按从上到下、从左到右的读序——展签与印厂清单都按这个顺序点作品。
fn frame_row(page: &LayoutPage, frame: &LayoutFrame) -> Vec<String> {
    let pixels = frame.source.as_ref().map(delivered);
    let ppi = frame.source.as_ref().and_then(|s| effective_ppi(s, frame));
    vec![
        page.page_number.to_string(),
        page.side.clone(),
        "frame".to_owned(),
        frame.id.clone(),
        number(frame.x_mm),
        number(frame.y_mm),
        number(frame.width_mm),
        number(frame.height_mm),
        pixels.map(|(w, _)| number(w)).unwrap_or_default(),
        pixels.map(|(_, h)| number(h)).unwrap_or_default(),
        ppi.map(number).unwrap_or_default(),
        number(frame.crop_zoom),
        frame.fit_mode.clone(),
        String::new(),
        optional(frame.asset_id.as_ref()),
    ]
}

fn text_row(page: &LayoutPage, text: &LayoutText) -> Vec<String> {
    vec![
        page.page_number.to_string(),
        page.side.clone(),
        "text".to_owned(),
        text.id.clone(),
        number(text.x_mm),
        number(text.y_mm),
        number(text.width_mm),
        number(text.height_mm),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        text_of(&text.content_json),
        String::new(),
    ]
}

fn sorted<T, K: Ord>(mut items: Vec<T>, key: impl Fn(&T) -> K) -> Vec<T> {
    items.sort_by_key(key);
    items
}

/// 毫米坐标按整毫米取整后排读序：版面拖拽产生的 0.001mm 级抖动不该把同一条图注甩到下一张照片后面。
fn read_key(x_mm: f64, y_mm: f64, id: &str) -> (i64, i64, String) {
    (y_mm.round() as i64, x_mm.round() as i64, id.to_owned())
}

fn row(lines: &mut Vec<String>, cells: &[String]) {
    let quoted: Vec<String> = cells.iter().map(String::as_str).map(cell).collect();
    lines.push(quoted.join(","));
}

pub fn render_worklist(layout: &PublicationLayout) -> String {
    let mut lines: Vec<String> = vec![HEADER.join(",")];
    for page in &layout.pages {
        let frames = sorted(page.frames.clone(), |frame| {
            read_key(frame.x_mm, frame.y_mm, &frame.id)
        });
        let texts = sorted(page.texts.clone(), |text| {
            read_key(text.x_mm, text.y_mm, &text.id)
        });
        for frame in &frames {
            row(&mut lines, &frame_row(page, frame));
        }
        for text in &texts {
            row(&mut lines, &text_row(page, text));
        }
    }
    // BOM：Excel 与 Numbers 缺了它会把中文图注按本地编码猜，第一列直接花屏。
    format!("\u{feff}{}\r\n", lines.join("\r\n"))
}

pub fn file_name(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | ':' | '\\' | '\0' => '-',
            other => other,
        })
        .collect();
    let trimmed = cleaned.trim();
    let base = if trimmed.is_empty() {
        "publication".to_owned()
    } else {
        trimmed.chars().take(80).collect::<String>()
    };
    format!("worklist-{base}.csv")
}

pub fn write(csv: &str, path: &Path) -> Result<ExportReceipt> {
    if !path.is_absolute() {
        return Err(AppError::new(
            "INVALID_PATH",
            "导出路径必须来自系统保存对话框",
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| AppError::new("INVALID_PATH", format!("无法准备导出目录：{error}")))?;
    }
    std::fs::write(path, csv.as_bytes())
        .map_err(|error| AppError::new("EXPORT", format!("写入作品清单失败：{error}")))?;
    Ok(ExportReceipt {
        path: path.display().to_string(),
        sha256: fingerprint(path)?,
        bytes: csv.len(),
    })
}

/// 只给测试用的读回工具：把导出的表拆回单元格，断言读的是内容，不是在数逗号。
#[cfg(test)]
pub(crate) mod sheet_reader {
    use super::HEADER;

    pub fn col(name: &str) -> usize {
        HEADER.iter().position(|h| *h == name).unwrap()
    }

    /// 按 RFC4180 还原：引号里出现的双引号算一个引号，逗号与换行都不再是分隔符。
    pub fn cells(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut current = String::new();
        let mut quoted = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' if quoted && chars.peek() == Some(&'"') => {
                    current.push('"');
                    chars.next();
                }
                '"' => quoted = !quoted,
                ',' if !quoted => out.push(std::mem::take(&mut current)),
                other => current.push(other),
            }
        }
        out.push(current);
        out
    }

    /// 一条记录可能因图注里的换行而跨多行，所以先按 RFC4180 的引号状态重组，再拆列。
    pub fn rows(csv: &str) -> Vec<Vec<String>> {
        let body = csv.trim_start_matches('\u{feff}');
        let mut records: Vec<String> = Vec::new();
        let mut current = String::new();
        let mut quoted = false;
        let mut chars = body.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' if quoted && chars.peek() == Some(&'"') => {
                    current.push('"');
                    current.push('"');
                    chars.next();
                }
                '"' => {
                    quoted = !quoted;
                    current.push('"');
                }
                '\r' if !quoted && chars.peek() == Some(&'\n') => {
                    chars.next();
                    if !current.is_empty() {
                        records.push(std::mem::take(&mut current));
                    }
                }
                other => current.push(other),
            }
        }
        if !current.is_empty() {
            records.push(current);
        }
        records.iter().map(|line| cells(line)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::sheet_reader::{col, rows};
    use super::*;
    use crate::publication_versions::{LayoutPublication, LayoutSource};

    fn source(width: i64, height: i64) -> LayoutSource {
        LayoutSource {
            asset_id: "asset-1".to_owned(),
            source_sha256: "b".repeat(64),
            width,
            height,
            source_profile_digest: None,
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

    fn frame(
        id: &str,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        src: Option<LayoutSource>,
    ) -> LayoutFrame {
        LayoutFrame {
            id: id.to_owned(),
            x_mm: 15.0,
            y_mm,
            width_mm,
            height_mm,
            fit_mode: "fill".to_owned(),
            asset_id: src.as_ref().map(|s| s.asset_id.clone()),
            rendition_id: None,
            crop_x: 0.5,
            crop_y: 0.5,
            crop_zoom: 1.0,
            source: src,
        }
    }

    fn caption(text: &str) -> String {
        serde_json::json!({ "text": text }).to_string()
    }

    fn text(id: &str, y_mm: f64, content_json: &str) -> LayoutText {
        LayoutText {
            id: id.to_owned(),
            x_mm: 15.0,
            y_mm,
            width_mm: 160.0,
            height_mm: 18.0,
            content_json: content_json.to_owned(),
            style_json: None,
        }
    }

    fn page(number: i64, frames: Vec<LayoutFrame>, texts: Vec<LayoutText>) -> LayoutPage {
        LayoutPage {
            id: format!("page-{number}"),
            page_number: number,
            side: if number % 2 == 1 { "right" } else { "left" }.to_owned(),
            width_mm: 190.0,
            height_mm: 257.0,
            margin_mm: 12.0,
            bleed_mm: 3.0,
            layout_json: None,
            frames,
            texts,
        }
    }

    fn book(pub_type: &str, pages: Vec<LayoutPage>) -> PublicationLayout {
        PublicationLayout {
            schema_version: 1,
            publication: LayoutPublication {
                id: "pub-1".to_owned(),
                project_id: "project-1".to_owned(),
                title: "第一场雪".to_owned(),
                pub_type: pub_type.to_owned(),
                spec_json: "{\"width_mm\":190}".to_owned(),
                state: "draft".to_owned(),
            },
            pages,
        }
    }

    #[test]
    fn header_then_one_row_per_frame_and_each_row_keeps_the_column_width() {
        let layout = book(
            "photobook",
            vec![page(
                3,
                vec![frame("f1", 15.0, 100.0, 100.0, Some(source(3000, 2000)))],
                vec![text("t1", 200.0, &caption("北海道"))],
            )],
        );
        let csv = render_worklist(&layout);
        let rows = rows(&csv);
        assert_eq!(
            rows[0],
            HEADER
                .iter()
                .map(|h| h.to_string())
                .collect::<Vec<String>>()
        );
        assert_eq!(rows.len(), 3);
        for row in &rows {
            assert_eq!(row.len(), HEADER.len());
        }
        assert_eq!(rows[1][col("kind")], "frame");
        assert_eq!(rows[1][col("page_number")], "3");
        assert_eq!(rows[1][col("side")], "right");
        assert_eq!(rows[2][col("kind")], "text");
        assert_eq!(rows[2][col("page_number")], "3");
        // 照片行与文字行共用同一套列：一张表里既能对作品也能对展签。
        assert_eq!(rows[2][col("text")], "北海道");
        assert_eq!(rows[1][col("text")], "");
    }

    #[test]
    fn commas_quotes_and_line_breaks_in_a_caption_survive_the_round_trip() {
        let tricky = "北海道，第一场雪\n\"雾\"之后";
        let layout = book(
            "exhibition",
            vec![page(2, vec![], vec![text("t1", 40.0, &caption(tricky))])],
        );
        let csv = render_worklist(&layout);
        let rows = rows(&csv);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1][col("text")], tricky);
        // 带分隔符的字段必须整体加引号，否则表格软件会把图注劈成两列。
        let raw_line = csv
            .trim_start_matches('\u{feff}')
            .lines()
            .find(|line| line.contains("北海道"))
            .unwrap();
        assert!(raw_line.starts_with("2,left,text,t1,"));
        assert!(raw_line.contains("\"北海道，第一场雪"));
    }

    #[test]
    fn an_empty_book_is_still_an_openable_sheet_with_just_the_header() {
        let csv = render_worklist(&book("photobook", vec![]));
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.starts_with("\u{feff}page_number,side,kind,"));
        assert!(csv.ends_with("\r\n"));
        assert_eq!(rows(&csv).len(), 1);
    }

    #[test]
    fn fill_reports_the_tightest_direction_and_fit_the_loosest() {
        let mut fill = frame("f1", 10.0, 105.0, 75.0, Some(source(3000, 2000)));
        fill.crop_zoom = 2.0;
        let mut fit = fill.clone();
        fit.id = "f2".to_owned();
        fit.fit_mode = "fit".to_owned();
        let csv = render_worklist(&book("photobook", vec![page(1, vec![fill, fit], vec![])]));
        let rows = rows(&csv);
        // 手工算：裁切 2 倍后 1500x1000 像素，框为 105x75mm。
        let wide = 1500.0 / (105.0 / 25.4);
        let tall = 1000.0 / (75.0 / 25.4);
        assert!(wide / tall > 1.05, "两个方向的分辨率要真的拉开才可分辨");
        assert_eq!(rows[1][col("effective_ppi")], number(tall));
        assert_eq!(rows[1][col("fit_mode")], "fill");
        assert_eq!(rows[2][col("effective_ppi")], number(wide));
        assert_eq!(rows[2][col("fit_mode")], "fit");
        assert_eq!(rows[1][col("delivered_width_px")], "3000");
        assert_eq!(rows[1][col("delivered_height_px")], "2000");
    }

    #[test]
    fn delivered_pixels_are_the_selected_rendition_not_the_original() {
        let mut src = source(4000, 3000);
        src.rendition_id = Some("r-1".to_owned());
        src.rendition_width = Some(1200);
        src.rendition_height = Some(900);
        let csv = render_worklist(&book(
            "photobook",
            vec![page(
                1,
                vec![frame("f1", 10.0, 100.0, 100.0, Some(src))],
                vec![],
            )],
        ));
        let rows = rows(&csv);
        assert_eq!(rows[1][col("delivered_width_px")], "1200");
        assert_eq!(rows[1][col("delivered_height_px")], "900");
        // 框是 100mm 见方、版本只有 1200x900：短边方向才是吃紧的那个。
        assert_eq!(
            rows[1][col("effective_ppi")],
            number(900.0 / (100.0 / 25.4))
        );
    }

    #[test]
    fn a_frame_without_its_picture_leaves_the_pixel_columns_blank_instead_of_guessing() {
        let mut zero = frame("f2", 20.0, 100.0, 100.0, Some(source(0, 0)));
        zero.id = "f3".to_owned();
        let csv = render_worklist(&book(
            "photobook",
            vec![page(
                1,
                vec![frame("f1", 10.0, 100.0, 100.0, None), zero],
                vec![],
            )],
        ));
        let rows = rows(&csv);
        assert_eq!(rows[1][col("delivered_width_px")], "");
        assert_eq!(rows[1][col("delivered_height_px")], "");
        assert_eq!(rows[1][col("effective_ppi")], "");
        assert_eq!(rows[1][col("asset_id")], "");
        // 读到像素为 0 的元数据时不算分辨率：0/0 会得到 inf，写进表里就成了"分辨率无穷"这种假话。
        assert_eq!(rows[2][col("delivered_width_px")], "0");
        assert_eq!(rows[2][col("effective_ppi")], "");
    }

    #[test]
    fn read_order_is_top_then_left_and_ignores_sub_millimetre_jitter() {
        let csv = render_worklist(&book(
            "photobook",
            vec![page(
                1,
                vec![
                    frame("c", 120.0, 60.0, 60.0, None),
                    frame("a", 15.0, 60.0, 60.0, None),
                    frame("b", 60.001, 60.0, 60.0, None),
                    frame("b2", 60.4, 60.0, 60.0, None),
                ],
                vec![
                    text("y", 210.0, &caption("下")),
                    text("x", 5.0, &caption("上")),
                ],
            )],
        ));
        let rows = rows(&csv);
        let ids: Vec<&str> = rows[1..]
            .iter()
            .map(|row| row[col("id")].as_str())
            .collect();
        assert_eq!(ids, vec!["a", "b", "b2", "c", "x", "y"]);
    }

    #[test]
    fn exhibition_and_photobook_still_share_one_sheet() {
        let build = || {
            book(
                "photobook",
                vec![page(
                    1,
                    vec![frame("f1", 10.0, 100.0, 100.0, Some(source(3000, 2000)))],
                    vec![text("t1", 200.0, &caption("展签"))],
                )],
            )
        };
        let mut exhibition = build();
        exhibition.publication.pub_type = "exhibition".to_owned();
        // 这条断言记的是"还没做到"：清单目前不含展位/装裱信息，两种类型只有印前阈值不同。
        // 真按展览出表时这里会响，届时改成各自的列。
        assert_eq!(render_worklist(&build()), render_worklist(&exhibition));
    }

    #[test]
    fn unreadable_text_payloads_become_blank_cells_not_raw_json() {
        let csv = render_worklist(&book(
            "photobook",
            vec![page(
                1,
                vec![],
                vec![
                    text("t1", 10.0, "{\"wrong\":1}"),
                    text("t2", 20.0, "不是 JSON"),
                    text("t3", 30.0, "{\"text\":\"正常\"}"),
                ],
            )],
        ));
        let rows = rows(&csv);
        assert_eq!(rows[1][col("text")], "");
        assert_eq!(rows[2][col("text")], "");
        assert_eq!(rows[3][col("text")], "正常");
    }

    #[test]
    fn file_name_drops_path_separators_and_falls_back_when_there_is_no_title() {
        assert_eq!(
            file_name("北海道/第一场雪:第 3 版"),
            "worklist-北海道-第一场雪-第 3 版.csv"
        );
        assert_eq!(file_name("   "), "worklist-publication.csv");
        assert_eq!(file_name("a\u{0}b"), "worklist-a-b.csv");
        let long = file_name(&"标".repeat(120));
        assert_eq!(long.chars().count(), 9 + 80 + 4);
        assert!(long.starts_with("worklist-"));
        assert!(long.ends_with(".csv"));
    }

    #[test]
    fn write_refuses_a_path_that_did_not_come_from_the_save_dialog() {
        let error = write("x", Path::new("worklist.csv")).unwrap_err();
        assert_eq!(error.code, "INVALID_PATH");
        assert!(!Path::new("worklist.csv").exists());
    }

    #[test]
    fn write_returns_a_receipt_that_matches_the_bytes_on_disk() {
        let temp = tempfile::tempdir().unwrap();
        let csv = render_worklist(&book(
            "photobook",
            vec![page(
                1,
                vec![frame("f1", 10.0, 100.0, 100.0, Some(source(3000, 2000)))],
                vec![text("t1", 200.0, &caption("北海道，第一场雪"))],
            )],
        ));
        let path = temp.path().join("输出/worklist.csv");
        let receipt = write(&csv, &path).unwrap();
        assert_eq!(receipt.bytes, csv.len());
        assert_eq!(std::fs::read(&path).unwrap(), csv.as_bytes());
        assert_eq!(receipt.sha256, fingerprint(&path).unwrap());
        assert_eq!(receipt.sha256.len(), 64);
        assert_eq!(receipt.path, path.display().to_string());
    }
}
