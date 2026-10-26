//! 版面历史的有界缓存预览。只核对已入库项目的缓存，路径不进入版面快照或Agent上下文。
use crate::{
    domain::{AppError, Result},
    publication_versions::{self, LayoutSource, PublicationLayout},
};
use serde::Serialize;
use sqlx::{Sqlite, Transaction};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FramePreview {
    pub frame_id: String,
    pub thumbnail_path: Option<String>,
}
fn key(source: &LayoutSource) -> String {
    format!(
        "{}:{}",
        source.asset_id,
        source
            .rendition_sha256
            .as_deref()
            .unwrap_or(&source.source_sha256)
    )
}
async fn candidates(
    tx: &mut Transaction<'_, Sqlite>,
    layout: &PublicationLayout,
) -> Result<BTreeMap<String, PathBuf>> {
    let mut result = BTreeMap::new();
    for source in layout
        .pages
        .iter()
        .flat_map(|p| &p.frames)
        .filter_map(|f| f.source.as_ref())
    {
        let id = key(source);
        if result.contains_key(&id) {
            continue;
        }
        let actual = match publication_versions::source_tx(
            tx,
            &layout.publication.project_id,
            Some(&source.asset_id),
            source.rendition_id.as_deref(),
        )
        .await
        {
            Ok(source) => source,
            Err(error) if error.code == "ASSET_NOT_FOUND" => continue,
            Err(error) => return Err(error),
        };
        if !publication_versions::same_source(actual.as_ref(), Some(source)) {
            continue;
        }
        let hash = source
            .rendition_sha256
            .as_deref()
            .unwrap_or(&source.source_sha256);
        if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let path:Option<String>=sqlx::query_scalar("SELECT MIN(p.thumbnail_path) FROM photos p JOIN asset_aliases a ON a.photo_id=p.id WHERE a.asset_id=? AND p.project_id=? AND p.content_sha256=?")
            .bind(&source.asset_id).bind(&layout.publication.project_id).bind(&source.source_sha256).fetch_one(&mut **tx).await?;
        let Some(path) = path else {
            continue;
        };
        let path = PathBuf::from(path);
        // 只允许导入管线的确定性文件名；选中调色版本不能回退到原图缩略图。
        if path.file_name().and_then(|p| p.to_str())
            != Some(&format!("{}-320.jpg", source.source_sha256))
        {
            continue;
        }
        let Some(parent) = path.parent() else {
            continue;
        };
        result.insert(id, parent.join(format!("{hash}-320.jpg")));
    }
    Ok(result)
}
fn read(candidates: BTreeMap<String, PathBuf>) -> Result<BTreeMap<String, (String, String)>> {
    use sha2::{Digest, Sha256};
    use std::io::{Cursor, Read};
    let mut total = 0_u64;
    let mut approved = Vec::new();
    for (id, path) in candidates {
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() > 1024 * 1024 {
            continue;
        }
        total += metadata.len();
        if total > 64 * 1024 * 1024 {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "版面预览过多，请拆分为多个分册",
            ));
        }
        approved.push((id, path));
    }
    let mut result = BTreeMap::new();
    for (id, path) in approved {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        let Ok(file) = options.open(&path) else {
            continue;
        };
        let mut bytes = Vec::new();
        if file.take(1024 * 1024 + 1).read_to_end(&mut bytes).is_err() || bytes.len() > 1024 * 1024
        {
            continue;
        }
        let Ok(reader) = image::ImageReader::new(Cursor::new(&bytes)).with_guessed_format() else {
            continue;
        };
        if reader.format() != Some(image::ImageFormat::Jpeg) {
            continue;
        }
        let Ok((w, h)) = reader.into_dimensions() else {
            continue;
        };
        if w == 0 || h == 0 || w > 320 || h > 320 {
            continue;
        }
        result.insert(
            id,
            (
                path.to_string_lossy().into_owned(),
                format!("{:x}", Sha256::digest(&bytes)),
            ),
        );
    }
    Ok(result)
}
async fn checked(
    tx: &mut Transaction<'_, Sqlite>,
    layout: &PublicationLayout,
) -> Result<BTreeMap<String, (String, String)>> {
    let candidates = candidates(tx, layout).await?;
    tokio::task::spawn_blocking(move || read(candidates))
        .await
        .map_err(|_| AppError::new("BACKGROUND", "读取版面预览失败，请稍后重试"))?
}
pub(crate) async fn capture(
    tx: &mut Transaction<'_, Sqlite>,
    layout: &mut PublicationLayout,
) -> Result<()> {
    let checked = checked(tx, layout).await?;
    for source in layout
        .pages
        .iter_mut()
        .flat_map(|p| &mut p.frames)
        .filter_map(|f| f.source.as_mut())
    {
        source.preview_sha256 = checked.get(&key(source)).map(|value| value.1.clone());
    }
    Ok(())
}
pub(crate) async fn project(
    tx: &mut Transaction<'_, Sqlite>,
    layout: &PublicationLayout,
) -> Result<Vec<FramePreview>> {
    let checked = checked(tx, layout).await?;
    Ok(layout
        .pages
        .iter()
        .flat_map(|p| &p.frames)
        .map(|frame| FramePreview {
            frame_id: frame.id.clone(),
            thumbnail_path: frame.source.as_ref().and_then(|source| {
                checked
                    .get(&key(source))
                    .filter(|(_, sha)| Some(sha) == source.preview_sha256.as_ref())
                    .map(|(path, _)| path.clone())
            }),
        })
        .collect())
}
