//! 原图只读检视：路径来自已入库照片，前端不能提交文件路径。
use crate::{
    db::Store,
    domain::{AppError, Project, Result},
    image_pipeline,
};
use std::path::PathBuf;

pub fn compact_project(mut project: Project) -> Project {
    // 仅缩减 IPC 投影，数据库审计、幂等回执和导出仍保留完整状态。
    for event in &mut project.events {
        event.previous_json = None;
        event.next_json = None;
        event.context_json = "{}".into();
    }
    project
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InspectionSource {
    Original {
        path: PathBuf,
    },
    DerivedPreview {
        path: PathBuf,
    },
    EmbeddedPreview {
        #[serde(rename = "dataUrl")]
        data_url: String,
    },
    DisplayManaged {
        path: PathBuf,
    },
}

impl InspectionSource {
    pub fn path(&self) -> Option<&std::path::Path> {
        match self {
            Self::Original { path }
            | Self::DerivedPreview { path }
            | Self::DisplayManaged { path } => Some(path),
            Self::EmbeddedPreview { .. } => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Original { .. } => "original",
            Self::DerivedPreview { .. } => "derived_preview",
            Self::EmbeddedPreview { .. } => "embedded_preview",
            Self::DisplayManaged { .. } => "display_managed",
        }
    }
}

pub async fn original_path(
    store: &Store,
    project_id: &str,
    photo_id: &str,
) -> Result<InspectionSource> {
    let (original_path, content_sha256, width, height, preview_path): (String, String, i64, i64, String) = sqlx::query_as(
        "SELECT p.original_path,p.content_sha256,p.width,p.height,p.preview_path FROM photos p JOIN projects j ON j.id=p.project_id WHERE p.project_id=? AND p.id=? AND j.deleted_at IS NULL"
    ).bind(project_id).bind(photo_id).fetch_optional(&store.pool).await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "照片不属于该项目或项目已归档"))?;
    tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(original_path).canonicalize().map_err(|_| {
            AppError::new("ORIGINAL_MISSING", "原图失联，请重新定位；当前显示缓存预览")
        })?;
        let metadata = path.metadata()?;
        if !metadata.is_file() {
            return Err(AppError::new("ORIGINAL_MISSING", "原图失联，请重新定位"));
        }
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let is_raw = matches!(
            extension.as_str(),
            "cr2" | "cr3" | "nef" | "arw" | "dng" | "orf" | "rw2" | "raf" | "raw"
        );
        // 文件大小先于指纹读取检查；检视 RAW 不再启动系统解码进程。
        let file_limit = if is_raw { 512 } else { 64 } * 1024 * 1024;
        if metadata.len() > file_limit {
            return Err(AppError::new(
                "INSPECTION_LIMIT",
                "原图超过检视文件上限；当前显示缓存预览",
            ));
        }
        if image_pipeline::fingerprint(&path)? != content_sha256 {
            return Err(AppError::new(
                "FILE_CHANGED",
                "原图内容已改变，请重新导入；当前显示缓存预览",
            ));
        }
        if is_raw {
            if extension == "cr2"
                && let Some(jpeg) = crate::cr2_preview::extract(&path)
            {
                use base64::Engine;
                if image_pipeline::fingerprint(&path)? != content_sha256 {
                    return Err(AppError::new("FILE_CHANGED", "检视期间原图内容已改变"));
                }
                return Ok(InspectionSource::EmbeddedPreview {
                    data_url: format!(
                        "data:image/jpeg;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(jpeg)
                    ),
                });
            }
            return bounded_preview(&PathBuf::from(preview_path))
                .map(|path| InspectionSource::DerivedPreview { path });
        }
        if width <= 0 || height <= 0 || width.saturating_mul(height) > 32_000_000 {
            return Err(AppError::new(
                "INSPECTION_LIMIT",
                "原图超过检视上限（3200 万像素）；当前显示缓存预览",
            ));
        }
        let reader = image::ImageReader::open(&path)?.with_guessed_format()?;
        match reader.format() {
            Some(image::ImageFormat::Jpeg | image::ImageFormat::Png | image::ImageFormat::Tiff) => {
            }
            Some(_) => {
                // image crate 不识别 HEIC：魔数自行校验，交给 WKWebView 原生渲染。
                if is_heic(&path)? {
                } else {
                    return Err(AppError::new(
                        "INSPECTION_FORMAT",
                        "原图检视暂不支持该格式；当前显示缓存预览",
                    ));
                }
            }
            None => {
                if !is_heic(&path)? {
                    return Err(AppError::new(
                        "INSPECTION_FORMAT",
                        "无法识别原图格式；当前显示缓存预览",
                    ));
                }
            }
        }
        // G2-③：显示器色彩渲染机制已落地（render_display_managed + DisplayManaged 变体），
        // 但自动接线与 M1 直通契约冲突（16-bit 保真/ICC 元数据保真/直通只读，5 项既有回归）。
        // 激活方式（开关 opt-in 或替代直通）待产品决策；在此之前检视保持字节级直通。
        Ok(InspectionSource::Original { path })
    })
    .await
    .map_err(|error| AppError::new("BACKGROUND", error.to_string()))?
}

/// 解码→源ICC→显示ICC 变换→q92 JPEG→tmp+rename 原子写。ICC 参数注入便于测试。
/// 生产接线按上方说明暂缓；仅测试引用期间显式豁免 dead_code。
#[cfg_attr(not(test), allow(dead_code))]
#[cfg(target_os = "macos")]
fn render_display_managed(
    original: &std::path::Path,
    source_icc: &[u8],
    display_icc: &[u8],
    target: &std::path::Path,
) -> Result<()> {
    let bitmap = image::open(original).map_err(|e| AppError::new("DECODE", e.to_string()))?;
    let rgb = bitmap.to_rgb8();
    let (width, height) = rgb.dimensions();
    let mut pixels = rgb.into_raw();
    crate::color_engine::native_transform::transform_rgb8(
        source_icc,
        display_icc,
        &mut pixels,
        width as usize,
        height as usize,
    )?;
    let managed = image::RgbImage::from_raw(width, height, pixels)
        .ok_or_else(|| AppError::new("DECODE", "变换后缓冲尺寸不匹配"))?;
    let temporary = target.with_extension(format!("{}.tmp", crate::domain::id()));
    let file = std::fs::File::create(&temporary)?;
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 92);
    managed
        .write_with_encoder(encoder)
        .map_err(|e| AppError::new("CACHE_WRITE", e.to_string()))?;
    std::fs::rename(&temporary, target)?;
    Ok(())
}

/// ISO-BMFF ftyp 盒校验 HEIF 家族 brand（heic/heix/hevc/hevx/mif1/msf1）。
fn is_heic(path: &std::path::Path) -> Result<bool> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut header = [0_u8; 12];
    let read = file.read(&mut header)?;
    if read < 12 || &header[4..8] != b"ftyp" {
        return Ok(false);
    }
    Ok(matches!(
        &header[8..12],
        b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1"
    ))
}

/// 复用导入阶段生成的 JPEG；每次检视都重新验证尺寸和格式，避免缓存污染。
/// 不复制原图、不运行无界 RAW 转换，也不把派生预览声明成嵌入预览。
fn bounded_preview(preview: &std::path::Path) -> Result<PathBuf> {
    let path = preview
        .canonicalize()
        .map_err(|_| AppError::new("INSPECTION_FORMAT", "RAW 派生预览不可用，请重新导入"))?;
    let metadata = path.metadata()?;
    if !metadata.is_file() || metadata.len() > 16 * 1024 * 1024 {
        return Err(AppError::new(
            "INSPECTION_LIMIT",
            "RAW 派生预览超过检视上限",
        ));
    }
    let reader = image::ImageReader::open(&path)?.with_guessed_format()?;
    if reader.format() != Some(image::ImageFormat::Jpeg) {
        return Err(AppError::new(
            "INSPECTION_FORMAT",
            "RAW 派生预览不是有效 JPEG",
        ));
    }
    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| AppError::new("INSPECTION_FORMAT", error.to_string()))?;
    if width == 0 || height == 0 || width > 1400 || height > 1400 {
        return Err(AppError::new(
            "INSPECTION_LIMIT",
            "RAW 派生预览超过 1400px 检视上限",
        ));
    }
    Ok(path)
}

#[cfg(all(test, target_os = "macos"))]
mod display_tests {
    use super::*;

    fn system_profile(name: &str) -> Vec<u8> {
        std::fs::read(std::path::Path::new("/System/Library/ColorSync/Profiles").join(name))
            .unwrap_or_else(|error| panic!("缺少系统色彩配置 {name}: {error}"))
    }

    #[test]
    fn p3_display_shifts_srgb_blue_toward_reference() {
        let dir = tempfile::tempdir().unwrap();
        // 评审参考值：sRGB→P3 蓝 (0,0,255)→(0,0,~245)；JPEG q92 有损放宽到 ±8。
        let img = image::RgbImage::from_pixel(4, 1, image::Rgb([0, 0, 255]));
        let original = dir.path().join("blue.png");
        img.save_with_format(&original, image::ImageFormat::Png)
            .unwrap();
        let target = dir.path().join("managed.jpg");
        render_display_managed(
            &original,
            &system_profile("sRGB Profile.icc"),
            &system_profile("Display P3.icc"),
            &target,
        )
        .unwrap();
        let pixel = image::open(&target).unwrap().to_rgb8().get_pixel(1, 0).0;
        assert!(pixel[2] < 253 && pixel[2] >= 235, "blue got {pixel:?}");
        assert!(pixel[0] <= 8 && pixel[1] <= 8, "got {pixel:?}");
    }

    #[test]
    fn identical_profiles_keep_pixels() {
        let dir = tempfile::tempdir().unwrap();
        let img = image::RgbImage::from_pixel(4, 1, image::Rgb([200, 30, 40]));
        let original = dir.path().join("red.png");
        img.save_with_format(&original, image::ImageFormat::Png)
            .unwrap();
        let target = dir.path().join("managed.jpg");
        let srgb = system_profile("sRGB Profile.icc");
        render_display_managed(&original, &srgb, &srgb, &target).unwrap();
        let pixel = image::open(&target).unwrap().to_rgb8().get_pixel(2, 0).0;
        assert!((pixel[0] as i32 - 200).abs() <= 8, "got {pixel:?}");
        assert!((pixel[1] as i32 - 30).abs() <= 8, "got {pixel:?}");
    }
}
