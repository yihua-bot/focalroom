//! 共享副本包仅从登记的派生预览重新编码；不读取或复制原图，不改变本地权威。
use crate::{
    domain::{AppError, Project, Result},
    export::ExportReceipt,
    image_pipeline::fingerprint,
};
use image::ImageReader;
use serde_json::json;
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

pub fn preview_bytes(path: &Path) -> Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true);
    if !std::fs::symlink_metadata(path)?.is_file() {
        return Err(AppError::new("MEDIA_POLICY", "预览必须为普通文件"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > 16 * 1024 * 1024 {
        return Err(AppError::new("MEDIA_POLICY", "派生预览大小不合法"));
    }
    let mut input = Vec::new();
    file.take(16 * 1024 * 1024 + 1).read_to_end(&mut input)?;
    let mut reader =
        ImageReader::with_format(std::io::Cursor::new(input), image::ImageFormat::Jpeg);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(2048);
    limits.max_image_height = Some(2048);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| {
        AppError::new(
            "MEDIA_POLICY",
            "登记预览无法解码或超过 2048px；请重新生成缓存",
        )
    })?;
    let mut output = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 88)
        .encode_image(&image::DynamicImage::ImageRgb8(image.to_rgb8()))
        .map_err(|_| AppError::new("MEDIA_POLICY", "预览编码失败"))?;
    Ok(output)
}
pub fn manifest(
    project: &Project,
    promotion_id: &str,
    workspace: &str,
) -> Result<serde_json::Value> {
    if project.photos.is_empty()
        || project.photos.len() > 10000
        || project.photos.iter().any(|p| {
            p.id.is_empty()
                || p.id.len() > 100
                || !p
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        })
    {
        return Err(AppError::new("MEDIA_POLICY", "共享需要 1–10000 张有效照片"));
    }
    Ok(
        json!({"workspace_id":workspace,"local_project_id":project.id,"promotion_id":promotion_id,"title":project.title,"from_revision":project.revision,
      "snapshot":{"photos":project.photos.iter().map(|p|json!({"id":p.id,"filename":p.name,"source_sha256":p.content_sha256,"selection":match p.status.as_str(){"keep"=>"KEEP","reject"=>"REJECT",_=>"UNDECIDED"}})).collect::<Vec<_>>(),
      "sequences":[{"id":"primary","title":"作品序列","items":project.sequence.iter().map(|id|json!({"id":format!("item-{id}"),"photo_id":id})).collect::<Vec<_>>()}]},
      "media_policy":{"previews_only":true,"strip_gps":true,"max_long_edge":2048,"ai_upload_authorized":false}}),
    )
}
pub fn write(project: &Project, path: &Path) -> Result<ExportReceipt> {
    let manifest = manifest(
        project,
        &format!("copy-{}", crate::domain::id()),
        "choose-in-web",
    )?;
    std::fs::create_dir(path)?; // 只创建新目录；失败产物保留，绝不覆盖已有文件。
    let mut bytes = 0;
    for photo in &project.photos {
        let data = preview_bytes(Path::new(&photo.preview_path))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.join(format!("{}.preview.jpg", photo.id)))?;
        file.write_all(&data)?;
        bytes += data.len();
    }
    let data = serde_json::to_vec_pretty(&manifest)
        .map_err(|_| AppError::new("EXPORT", "清单编码失败"))?;
    let target = path.join("manifest.json");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)?
        .write_all(&data)?;
    bytes += data.len();
    OpenOptions::new().write(true).create_new(true).open(path.join("README.txt"))?.write_all("FocalRoom 共享副本包\n在 Web /shared 选择 manifest.json 和全部 *.preview.jpg。仅上传重编码预览，不含原图、GPS、本机路径、评论或个人记忆。本地原项目保持独立；此包不切换本地权威。\n".as_bytes())?;
    Ok(ExportReceipt {
        path: path.display().to_string(),
        sha256: fingerprint(&target)?,
        bytes,
    })
}
