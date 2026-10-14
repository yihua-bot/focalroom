use crate::{
    domain::{AppError, Project, Result},
    image_pipeline::fingerprint,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use serde_json::{Value, json};
use std::{io::Write, path::Path};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReceipt {
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
}

pub fn render(project: &Project, format: &str) -> Result<Vec<u8>> {
    render_with_options(project, format, false)
}
pub fn render_with_options(
    project: &Project,
    format: &str,
    include_paths: bool,
) -> Result<Vec<u8>> {
    match format {
        "json" => {
            let mut value = serde_json::to_value(project)
                .map_err(|error| AppError::new("EXPORT", error.to_string()))?;
            value["schemaVersion"] = json!(2);
            value["includesLocalPaths"] = json!(include_paths);
            value["exportedAt"] = json!(crate::domain::timestamp());
            value["warning"] = json!("仅含本地文件引用与编辑元数据，不包含原始照片二进制");
            value["events"]=json!(project.events.iter().map(|event|json!({
                "id":event.id,"photoId":event.photo_id,"actor":event.actor,"type":event.kind,
                "previous":event.previous_json.as_ref().and_then(|s|serde_json::from_str::<Value>(s).ok()),
                "next":event.next_json.as_ref().and_then(|s|serde_json::from_str::<Value>(s).ok()),
                "reason":event.reason,"context":serde_json::from_str::<Value>(&event.context_json).ok(),"at":event.at
            })).collect::<Vec<_>>());
            if !include_paths {
                redact_paths(&mut value);
            }
            serde_json::to_vec_pretty(&value)
                .map_err(|error| AppError::new("EXPORT", error.to_string()))
        }
        "csv" => {
            let header = if include_paths {
                "frame,id,filename,status,rating,sequence,note,original_path,sha256,missing"
            } else {
                "frame,id,filename,status,rating,sequence,note,sha256,missing"
            };
            let mut lines = vec![header.to_string()];
            for photo in &project.photos {
                let position = project
                    .sequence
                    .iter()
                    .position(|id| id == &photo.id)
                    .map(|n| (n + 1).to_string())
                    .unwrap_or_default();
                let mut cells = vec![
                    photo.index.to_string(),
                    photo.id.clone(),
                    photo.name.clone(),
                    photo.status.clone(),
                    photo.rating.to_string(),
                    position,
                    photo.note.clone(),
                ];
                if include_paths {
                    cells.push(photo.original_path.clone());
                }
                cells.extend([photo.content_sha256.clone(), photo.is_missing.to_string()]);
                lines.push(
                    cells
                        .iter()
                        .map(|cell| csv_cell(cell))
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
            Ok(format!("\u{feff}{}\r\n", lines.join("\r\n")).into_bytes())
        }
        "html" => {
            let mut cells = String::new();
            for (position, id) in project.sequence.iter().enumerate() {
                let photo = project
                    .photos
                    .iter()
                    .find(|photo| &photo.id == id)
                    .ok_or_else(|| AppError::new("EXPORT", "序列包含失效照片"))?;
                let thumbnail = std::fs::read(&photo.thumbnail_path)?;
                cells.push_str(&format!("<figure><img alt=\"{}\" src=\"data:image/jpeg;base64,{}\"><figcaption>{:02} / {}</figcaption></figure>",escape(&photo.name),STANDARD.encode(thumbnail),position+1,escape(&photo.name)));
            }
            Ok(format!("<!doctype html><html lang=\"zh-CN\"><meta charset=\"utf-8\"><title>{}</title><style>body{{font-family:Georgia,'Songti SC',serif;padding:32px;color:#222}}h1{{font-weight:400}}main{{display:grid;grid-template-columns:repeat(4,1fr);gap:16px}}figure{{margin:0;break-inside:avoid}}img{{width:100%;height:150px;object-fit:contain;background:#eee}}figcaption{{font-size:11px;margin-top:6px}}@media print{{body{{padding:0}}}}</style><h1>{}</h1><p>{}</p><p>已保留 {} 张 · 预览图接触印相</p><main>{}</main></html>",escape(&project.title),escape(&project.title),escape(&project.brief),project.sequence.len(),cells).into_bytes())
        }
        "print" => {
            // v1.2 印刷参考：A4 横版分页、封面 + 每页两图，浏览器「打印 → 存为 PDF」。
            let mut pages = String::new();
            let mut remaining = project.sequence.iter().enumerate().peekable();
            let mut page_number = 0;
            while let Some((position, id)) = remaining.next() {
                page_number += 1;
                let mut plates = String::new();
                let build = |position: usize, id: &str| -> Result<String> {
                    let photo = project
                        .photos
                        .iter()
                        .find(|photo| photo.id == id)
                        .ok_or_else(|| AppError::new("EXPORT", "序列包含失效照片"))?;
                    let preview = std::fs::read(&photo.preview_path)?;
                    Ok(format!(
                        "<figure class=\"plate\"><img alt=\"{}\" src=\"data:image/jpeg;base64,{}\"><figcaption><strong>{:02}</strong> {}</figcaption></figure>",
                        escape(&photo.name),
                        STANDARD.encode(preview),
                        position + 1,
                        escape(&photo.name)
                    ))
                };
                plates.push_str(&build(position, id)?);
                if let Some((next_position, next_id)) = remaining.peek() {
                    plates.push_str(&build(*next_position, next_id)?);
                    remaining.next();
                }
                pages.push_str(&format!(
                    "<section class=\"sheet\"><header><span>{}</span><span>第 {page_number} 页</span></header><div class=\"plates\">{plates}</div><footer><span>Between Frames · 印刷参考（非最终色彩管理）</span><span>{}</span></footer></section>",
                    escape(&project.title),
                    project.sequence.len()
                ));
            }
            Ok(format!(r#"<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>{} · 印刷参考</title><style>
@page {{ size: A4 landscape; margin: 12mm; }}
body {{ font-family: Georgia, 'Songti SC', serif; color: #222; margin: 0; }}
.cover {{ page-break-after: always; padding: 40mm 20mm; }}
.cover h1 {{ font-weight: 400; font-size: 34pt; margin: 0 0 8mm; }}
.cover p {{ color: #555; max-width: 140mm; line-height: 1.7; }}
.sheet {{ page-break-after: always; display: flex; flex-direction: column; height: 180mm; }}
.sheet header, .sheet footer {{ display: flex; justify-content: space-between; font-size: 9pt; color: #777; border-bottom: 0.4pt solid #bbb; padding-bottom: 2mm; }}
.sheet footer {{ border: none; border-top: 0.4pt solid #bbb; padding: 2mm 0 0; margin-top: auto; }}
.plates {{ display: flex; gap: 10mm; flex: 1; align-items: center; }}
.plate {{ flex: 1; margin: 0; text-align: center; }}
.plate img {{ max-width: 100%; max-height: 130mm; outline: 0.4pt dashed #bbb; outline-offset: 4mm; }}
.plate figcaption {{ font-size: 9.5pt; margin-top: 3mm; color: #444; }}
.plate strong {{ color: #000; margin-right: 2mm; }}
@media screen {{ body {{ background: #f3f2ee; }} .sheet, .cover {{ background: white; max-width: 260mm; margin: 10mm auto; padding: 12mm; }} }}
</style><div class="cover"><p>BETWEEN FRAMES · 印刷参考</p><h1>{}</h1><p>{}</p><p>作品序列 {} 张 · 导出 {}</p><p class="hint">使用浏览器「打印 → 存为 PDF」，纸张 A4 横向、勾选背景图形。虚线为裁切参考；本参考不做色彩管理，正式印刷请以原片校色。</p></div>{}</html>"#,
                escape(&project.title),
                escape(&project.title),
                escape(&project.brief),
                project.sequence.len(),
                crate::domain::timestamp(),
                pages
            ).into_bytes())
        }
        _ => Err(AppError::new("INVALID_INPUT", "导出格式不合法")),
    }
}

pub fn write(project: &Project, format: &str, path: &Path) -> Result<ExportReceipt> {
    write_with_options(project, format, path, false)
}
pub fn write_with_options(
    project: &Project,
    format: &str,
    path: &Path,
    include_paths: bool,
) -> Result<ExportReceipt> {
    if !path.is_absolute() {
        return Err(AppError::new(
            "INVALID_PATH",
            "导出路径必须来自系统保存对话框",
        ));
    }
    let resolved = if path.exists() {
        path.canonicalize()?
    } else {
        path.parent()
            .ok_or_else(|| AppError::new("INVALID_PATH", "无效导出目录"))?
            .canonicalize()?
            .join(
                path.file_name()
                    .ok_or_else(|| AppError::new("INVALID_PATH", "无效文件名"))?,
            )
    };
    let path = resolved.as_path();
    if project.photos.iter().any(|photo| {
        Path::new(&photo.original_path) == path
            || Path::new(&photo.thumbnail_path) == path
            || Path::new(&photo.preview_path) == path
    }) {
        return Err(AppError::new(
            "ORIGINAL_PROTECTED",
            "不能覆盖原图或图像缓存",
        ));
    }
    // v1.2 侧车导出：每张照片一个 JSON + manifest，path 作为新目录名。
    if format == "sidecar" {
        if path.exists() {
            return Err(AppError::new(
                "INVALID_PATH",
                "目标已存在；请换一个新文件夹名",
            ));
        }
        std::fs::create_dir(path)?;
        let mut written: Vec<Value> = vec![];
        for photo in &project.photos {
            let mut sidecar = json!({
                "schemaVersion": 1,
                "photoId": photo.id,
                "fileName": photo.name,
                "sha256": photo.content_sha256,
                "status": photo.status,
                "rating": photo.rating,
                "note": photo.note,
                "sequencePosition": project.sequence.iter().position(|id| id == &photo.id).map(|n| n + 1),
                "missing": photo.is_missing,
                "exportedAt": crate::domain::timestamp(),
            });
            if include_paths {
                sidecar["originalPath"] = json!(photo.original_path);
            }
            let file_name = format!("{:03}-{}.json", photo.index, sanitize(&photo.name));
            std::fs::write(
                path.join(&file_name),
                serde_json::to_vec_pretty(&sidecar)
                    .map_err(|error| AppError::new("EXPORT", error.to_string()))?,
            )?;
            written.push(json!({"file": file_name, "photoId": photo.id}));
        }
        let manifest = json!({
            "schemaVersion": 1,
            "project": {"id": project.id, "title": project.title, "revision": project.revision},
            "photos": written.len(),
            "sequence": project.sequence,
            "files": written,
            "notice": "侧车仅含编辑元数据，不含图像；原图永不修改",
        });
        let manifest_path = path.join("manifest.json");
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| AppError::new("EXPORT", error.to_string()))?;
        std::fs::write(&manifest_path, manifest_bytes)?;
        return Ok(ExportReceipt {
            path: path.to_string_lossy().into(),
            sha256: fingerprint(&manifest_path)?,
            bytes: written.len(),
        });
    }
    let bytes = render_with_options(project, format, include_paths)?;
    // 只新建文件：遇到已存在的目标明确失败，绝不覆盖原图、数据库或用户文件。
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(ExportReceipt {
        path: path.to_string_lossy().into(),
        sha256: fingerprint(path)?,
        bytes: bytes.len(),
    })
}
fn csv_cell(value: &str) -> String {
    let guarded = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.into()
    };
    format!("\"{}\"", guarded.replace('"', "\"\""))
}
fn sanitize(value: &str) -> String {
    let base = value
        .trim()
        .trim_end_matches(".jpg")
        .trim_end_matches(".jpeg");
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "photo".into()
    } else {
        cleaned.chars().take(60).collect()
    }
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// 路径字段也可能出现在历史 import/relink 事件中，必须递归剔除。
fn redact_paths(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for field in [
                "originalPath",
                "thumbnailPath",
                "previewPath",
                "original_path",
                "thumbnail_path",
                "preview_path",
            ] {
                object.remove(field);
            }
            for item in object.values_mut() {
                redact_paths(item);
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_paths(item);
            }
        }
        _ => {}
    }
}
