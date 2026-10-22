//! 插件结果只增加独立版本，不替换原片、选片或序列。
use crate::{
    db::Store,
    domain::{self, AppError, Result},
    image_pipeline,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Batch {
    schema_version: u32,
    transfer_batch_id: String,
    provider: String,
    catalog_fingerprint: String,
    issued_at: String,
    items: Vec<Item>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Item {
    source_sha256: String,
    external_photo_id: String,
    external_variant_id: String,
    variant_id: String,
    render_id: String,
    render_filename: String,
    render_sha256: String,
    width: u32,
    height: u32,
    bit_depth: u8,
    media_format: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorVersion {
    pub id: String,
    pub photo_name: String,
    pub provider: String,
    pub preview_path: String,
    pub created_at: String,
}
fn invalid(message: &str) -> AppError {
    AppError::new("VALIDATION_ERROR", message)
}
fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn uuid(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok()
}
struct Prepared {
    asset: String,
    file: PathBuf,
    item: Item,
    size: u64,
}

pub async fn import_batch(
    store: &Store,
    project: &str,
    directory: &Path,
    storage: &Path,
) -> Result<usize> {
    store.get(project).await?;
    let root = directory.canonicalize()?;
    let manifest = root.join(".focalroom-manifest.json").canonicalize()?;
    if manifest.parent() != Some(root.as_path()) || manifest.metadata()?.len() > 4 * 1024 * 1024 {
        return Err(invalid("请选择插件生成的完整照片文件夹。"));
    }
    let bytes = std::fs::read(manifest)?;
    let batch: Batch = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("照片信息无法读取，请重新从插件导出。"))?;
    if batch.schema_version != 3
        || !uuid(&batch.transfer_batch_id)
        || !["LIGHTROOM_CLASSIC", "CAPTURE_ONE"].contains(&batch.provider.as_str())
        || !valid_sha(&batch.catalog_fingerprint)
        || batch.items.is_empty()
        || batch.items.len() > 500
        || chrono::DateTime::parse_from_rfc3339(&batch.issued_at).is_err()
    {
        return Err(invalid("照片信息不完整，请重新导出（每次最多 500 张）。"));
    }
    let provider = if batch.provider == "LIGHTROOM_CLASSIC" {
        "lightroom_classic"
    } else {
        "capture_one"
    };
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let mut prepared = Vec::new();
    std::fs::create_dir_all(storage)?;
    for item in batch.items {
        if !valid_sha(&item.source_sha256)
            || !valid_sha(&item.render_sha256)
            || !uuid(&item.variant_id)
            || !uuid(&item.render_id)
            || item.external_photo_id.is_empty()
            || item.external_photo_id.len() > 512
            || item.external_variant_id.is_empty()
            || item.external_variant_id.len() > 512
            || item.render_filename.is_empty()
            || item.render_filename.contains(['/', '\\'])
            || item.render_filename.contains("..")
            || !["JPEG", "PNG", "TIFF"].contains(&item.media_format.as_str())
        {
            return Err(invalid("照片对应信息无效，请重新导出。"));
        }
        let file = root.join(&item.render_filename).canonicalize()?;
        if file.parent() != Some(root.as_path())
            || !file.is_file()
            || file.metadata()?.len() > 512 * 1024 * 1024
        {
            return Err(invalid("照片文件不完整，或超过 512 MB，请重新导出。"));
        }
        let asset: Option<String> = sqlx::query_scalar(
            "SELECT id FROM source_assets WHERE project_id=? AND content_sha256=?",
        )
        .bind(project)
        .bind(&item.source_sha256)
        .fetch_optional(&store.pool)
        .await?;
        let asset = asset.ok_or_else(|| {
            invalid("当前项目中没有对应的原照片。请先导入原照片，或检查是否选错项目。")
        })?;
        let suffix = match item.media_format.as_str() {
            "JPEG" => "jpg",
            "PNG" => "png",
            _ => "tif",
        };
        let dest = storage.join(format!("{}.{}", item.render_sha256, suffix));
        let source = file.clone();
        let target = dest.clone();
        let expected = item.render_sha256.clone();
        let (width, height, depth, format, size) =
            tokio::task::spawn_blocking(move || -> Result<_> {
                if image_pipeline::fingerprint(&source)? != expected {
                    return Err(invalid("照片已改变或损坏，请重新导出。"));
                }
                if !target.exists() {
                    let temp = target.with_extension(format!("{}.partial", domain::id()));
                    std::fs::copy(&source, &temp)?;
                    if image_pipeline::fingerprint(&temp)? != expected {
                        return Err(invalid("照片在导入期间发生变化，请重试。"));
                    }
                    std::fs::rename(temp, &target)?;
                }
                if image_pipeline::fingerprint(&target)? != expected {
                    return Err(invalid("已保存照片校验失败，请联系支持。"));
                }
                let reader = image::ImageReader::open(&target)?.with_guessed_format()?;
                let format = reader.format();
                let decoder = reader
                    .into_decoder()
                    .map_err(|_| invalid("无法读取导出照片，请使用 JPEG、PNG 或 TIFF。"))?;
                use image::ImageDecoder;
                let (w, h) = decoder.dimensions();
                let color = decoder.color_type();
                let depth = (color.bits_per_pixel() / u16::from(color.channel_count())) as u8;
                image_pipeline::prepare(&target, &target.parent().unwrap().join("previews"))?;
                Ok((w, h, depth, format, target.metadata()?.len()))
            })
            .await
            .map_err(|_| invalid("照片准备未完成，请重试。"))??;
        let actual = match format {
            Some(image::ImageFormat::Jpeg) => "JPEG",
            Some(image::ImageFormat::Png) => "PNG",
            Some(image::ImageFormat::Tiff) => "TIFF",
            _ => "",
        };
        if width != item.width
            || height != item.height
            || depth != item.bit_depth
            || actual != item.media_format
        {
            return Err(invalid("照片尺寸或格式与导出信息不一致，请重新导出。"));
        }
        prepared.push(Prepared {
            asset,
            file: dest,
            item,
            size,
        });
    }
    let mut tx = store.pool.begin().await?;
    crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
    if let Some(row) =
        sqlx::query("SELECT project_id,manifest_sha256 FROM transfer_jobs WHERE id=?")
            .bind(&batch.transfer_batch_id)
            .fetch_optional(&mut *tx)
            .await?
    {
        if row.get::<String, _>("project_id") != project
            || row.get::<Option<String>, _>("manifest_sha256").as_deref() != Some(&digest)
        {
            return Err(invalid("这批照片的导入记录不一致，请重新导出。"));
        }
        return Ok(0);
    }
    let now = domain::timestamp();
    let mut added = 0;
    for p in prepared {
        // 项目和源 SHA 是权威，不信任插件传入的资产/版本标识。
        let valid: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM source_assets WHERE id=? AND project_id=? AND content_sha256=?",
        )
        .bind(&p.asset)
        .bind(project)
        .bind(&p.item.source_sha256)
        .fetch_one(&mut *tx)
        .await?;
        if valid != 1 {
            return Err(invalid("原照片对应关系已改变，请重新导入。"));
        }
        let identity = format!(
            "{:x}",
            Sha256::digest(format!(
                "{provider}\n{}\n{}",
                batch.catalog_fingerprint, p.item.external_variant_id
            ))
        );
        let name = format!("{provider}-{identity}");
        sqlx::query("INSERT OR IGNORE INTO edit_variants(id,asset_id,name,external_software,external_variant_id,is_current,created_at) VALUES(?,?,?,?,?,0,?)")
            .bind(domain::id()).bind(&p.asset).bind(&name).bind(provider).bind(&p.item.external_variant_id).bind(&now).execute(&mut *tx).await?;
        let variant: String =
            sqlx::query_scalar("SELECT id FROM edit_variants WHERE asset_id=? AND name=?")
                .bind(&p.asset)
                .bind(name)
                .fetch_one(&mut *tx)
                .await?;
        let kind = if provider == "lightroom_classic" {
            "external_lightroom"
        } else {
            "external_capture_one"
        };
        let result = sqlx::query("INSERT OR IGNORE INTO renditions(id,variant_id,sha256,width,height,bit_depth,source_type,source_ref,file_path,file_size,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)")
            .bind(domain::id()).bind(variant).bind(&p.item.render_sha256).bind(p.item.width).bind(p.item.height).bind(p.item.bit_depth)
            .bind(kind).bind(&batch.transfer_batch_id).bind(p.file.to_string_lossy().as_ref()).bind(p.size as i64).bind(&now).execute(&mut *tx).await?;
        added += result.rows_affected() as usize;
    }
    sqlx::query("INSERT INTO transfer_jobs(id,project_id,direction,provider,state,items_json,manifest_sha256,created_at,updated_at) VALUES(?,?,'incoming',?,'review_pending','[]',?,?,?)")
        .bind(batch.transfer_batch_id).bind(project).bind(provider).bind(digest).bind(&now).bind(&now).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(added)
}

pub async fn list_versions(
    store: &Store,
    project: &str,
    storage: &Path,
) -> Result<Vec<EditorVersion>> {
    store.get(project).await?;
    let rows = sqlx::query("SELECT r.id,r.sha256,r.created_at,v.external_software,p.name FROM renditions r JOIN edit_variants v ON v.id=r.variant_id JOIN source_assets a ON a.id=v.asset_id JOIN asset_aliases x ON x.asset_id=a.id JOIN photos p ON p.id=x.photo_id WHERE a.project_id=? AND r.source_type IN ('external_lightroom','external_capture_one') ORDER BY r.created_at DESC,r.id DESC LIMIT 500")
        .bind(project).fetch_all(&store.pool).await?;
    let mut versions = Vec::new();
    for row in rows {
        let hash: String = row.get("sha256");
        if !valid_sha(&hash) {
            continue;
        }
        let preview = storage.join("previews").join(format!("{hash}-1400.jpg"));
        if !preview.is_file() {
            continue;
        }
        versions.push(EditorVersion {
            id: row.get("id"),
            photo_name: row.get("name"),
            provider: row.get("external_software"),
            preview_path: preview.to_string_lossy().to_string(),
            created_at: row.get("created_at"),
        });
    }
    Ok(versions)
}
#[tauri::command]
pub async fn import_editor_results(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::commands::Backend>,
    project_id: String,
) -> Result<Option<usize>> {
    use tauri_plugin_dialog::DialogExt;
    state.store.get(&project_id).await?;
    let choice = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("选择插件生成的 FocalRoom 照片文件夹")
            .blocking_pick_folder()
    })
    .await
    .map_err(|_| invalid("文件夹选择未完成，请重试。"))?;
    let Some(choice) = choice else {
        return Ok(None);
    };
    let directory = choice
        .into_path()
        .map_err(|_| invalid("请选择本机文件夹。"))?;
    let storage = state.cache.parent().unwrap().join("editor-renders");
    import_batch(&state.store, &project_id, &directory, &storage)
        .await
        .map(Some)
}
#[tauri::command]
pub async fn list_editor_versions(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::commands::Backend>,
    project_id: String,
) -> Result<Vec<EditorVersion>> {
    use tauri::Manager;
    let storage = state.cache.parent().unwrap().join("editor-renders");
    let versions = list_versions(&state.store, &project_id, &storage).await?;
    for version in &versions {
        app.asset_protocol_scope()
            .allow_file(&version.preview_path)
            .map_err(|_| invalid("无法显示调色照片。"))?;
    }
    Ok(versions)
}
