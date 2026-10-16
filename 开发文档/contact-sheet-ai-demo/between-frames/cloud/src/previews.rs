//! 只读取项目登记的有界派生 JPEG；目录内容仍由可信本地预览制备流程管理。
use crate::{
    auth::CurrentUser,
    error::AppError,
    handlers::{can_read, AppState},
};
use axum::{
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde_json::{json, Value};
use std::io::Read;

const MAX_BYTES: u64 = 16 * 1024 * 1024;
type PreviewRow = (String, i64, i64, i64);

fn valid_sha(sha: &str) -> bool {
    sha.len() == 64
        && sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn valid_metadata(sha: &str, width: i64, height: i64, size: i64) -> bool {
    valid_sha(sha)
        && (1..=1400).contains(&width)
        && (1..=1400).contains(&height)
        && (1..=MAX_BYTES as i64).contains(&size)
}

pub(crate) async fn list_previews(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if !can_read(&state.pool, &user.id, &project_id).await? {
        return Err(AppError::forbidden("无权访问此项目"));
    }
    let files: Vec<PreviewRow> = sqlx::query_as(
        "SELECT sha256,width,height,file_size FROM preview_files WHERE project_id=? ORDER BY id",
    )
    .bind(&project_id)
    .fetch_all(&state.pool)
    .await?;
    if files
        .iter()
        .any(|(sha, w, h, size)| !valid_metadata(sha, *w, *h, *size))
    {
        return Err(AppError::internal());
    }
    Ok(Json(
        json!({"project_id": project_id, "previews": files.iter().map(|(sha, w, h, size)| json!({
        "sha256_prefix": &sha[..16], "width": w, "height": h, "size": size,
        "url": format!("/projects/{project_id}/previews/{sha}.jpg")
    })).collect::<Vec<_>>()}),
    ))
}

pub(crate) async fn serve_preview(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((project_id, filename)): Path<(String, String)>,
) -> Result<Response, AppError> {
    if !can_read(&state.pool, &user.id, &project_id).await? {
        return Err(AppError::forbidden("访问已被撤销"));
    }
    let sha = filename
        .strip_suffix(".jpg")
        .filter(|sha| valid_sha(sha))
        .ok_or_else(|| AppError::not_found("预览不存在"))?;
    let row: Option<(i64, i64, i64)> = sqlx::query_as(
        "SELECT width,height,file_size FROM preview_files WHERE project_id=? AND sha256=?",
    )
    .bind(&project_id)
    .bind(sha)
    .fetch_optional(&state.pool)
    .await?;
    let (width, height, size) = row.ok_or_else(|| AppError::not_found("预览不存在"))?;
    if !valid_metadata(sha, width, height, size) {
        return Err(AppError::internal());
    }
    let path = state.preview_dir.join(format!("{sha}-1400.jpg"));
    let bytes = tokio::task::spawn_blocking(move || read_preview(&path, width, height, size))
        .await
        .map_err(|_| AppError::internal())??;
    // IO 期间可能发生撤销；发送前再次读取权威授权，已发出的像素无法收回。
    if !can_read(&state.pool, &user.id, &project_id).await? {
        return Err(AppError::forbidden("访问已被撤销"));
    }
    Ok(([(header::CONTENT_TYPE, "image/jpeg")], bytes).into_response())
}

fn read_preview(
    path: &std::path::Path,
    width: i64,
    height: i64,
    size: i64,
) -> Result<Vec<u8>, AppError> {
    let missing = || AppError::not_found("预览文件不可用");
    if !std::fs::symlink_metadata(path)
        .map_err(|_| missing())?
        .is_file()
    {
        return Err(missing());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|_| missing())?;
    let metadata = file.metadata().map_err(|_| missing())?;
    if !metadata.is_file() || metadata.len() != size as u64 || metadata.len() > MAX_BYTES {
        return Err(missing());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| missing())?;
    if bytes.len() != size as usize {
        return Err(missing());
    }
    // 先查维度，再做实际解码；禁止把损坏文件或超大原图伪装成 1400px 预览。
    let dimensions =
        image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Jpeg)
            .into_dimensions()
            .map_err(|_| missing())?;
    if dimensions != (width as u32, height as u32) {
        return Err(missing());
    }
    image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg).map_err(|_| missing())?;
    Ok(bytes)
}
