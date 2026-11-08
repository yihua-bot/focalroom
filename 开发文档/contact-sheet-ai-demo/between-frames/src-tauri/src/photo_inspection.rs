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
        // G2-③：默认路径仍是字节级直通，16-bit 保真、ICC 元数据保真与只读契约因此不受影响；
        // 按显示器色彩的渲染走 display_managed_inspection，由界面开关显式请求。
        Ok(InspectionSource::Original { path })
    })
    .await
    .map_err(|error| AppError::new("BACKGROUND", error.to_string()))?
}

/// 全分辨率按显示器渲染的结论。`path` 为 None 表示确实没变换，界面继续用受限的色彩预览，
/// 不会拿一张没变换的图冒充显示器色彩。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedInspection {
    pub status: &'static str,
    pub detail: Option<String>,
    pub source_profile: String,
    pub display_profile: String,
    pub path: Option<String>,
}

/// 缓存文件按"原图内容指纹 × 显示器配置指纹"命名：换屏或原图改动都会自然落到新文件，
/// 不需要猜旧文件还能不能用；渲染仍是临时文件加改名，读者不会看到半张图。
fn managed_target(preview: &std::path::Path, content_sha256: &str, display_icc: &[u8]) -> PathBuf {
    use sha2::{Digest, Sha256};
    let display = format!("{:x}", Sha256::digest(display_icc));
    let dir = preview
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let stem = &content_sha256[..content_sha256.len().min(24)];
    dir.join(format!("managed-v2-{stem}-{}.jpg", &display[..24]))
}

fn unusable(path: &std::path::Path) -> bool {
    !path.exists() || path.metadata().map(|m| m.len() == 0).unwrap_or(true)
}

/// 适合屏幕的显式色彩检视从原图生成，旧缓存不具备可信像素色彩身份。
/// 通过同样的在册、原图指纹、格式和大小门禁，不修改原图或原有预览。
#[cfg(target_os = "macos")]
pub async fn display_managed_preview(
    store: &Store,
    project_id: &str,
    photo_id: &str,
) -> Result<crate::color_engine::ManagedRender> {
    display_managed_preview_with_profile(
        store,
        project_id,
        photo_id,
        crate::color_engine::native_transform::display_profile_bytes(),
    )
    .await
}

#[cfg(target_os = "macos")]
async fn display_managed_preview_with_profile(
    store: &Store,
    project_id: &str,
    photo_id: &str,
    display_profile: Result<Vec<u8>>,
) -> Result<crate::color_engine::ManagedRender> {
    let source = original_path(store, project_id, photo_id).await?;
    let InspectionSource::Original { path } = source else {
        return Ok(crate::color_engine::ManagedRender {
            status: "source_not_original",
            detail: Some("该照片只有派生或嵌入预览，无法从原图核验色彩".into()),
            source_profile: "无法确定".into(),
            display_profile: "无法确定".into(),
            jpeg: None,
        });
    };
    let hash: String =
        sqlx::query_scalar("SELECT content_sha256 FROM photos WHERE id=? AND project_id=?")
            .bind(photo_id)
            .bind(project_id)
            .fetch_one(&store.pool)
            .await?;
    let expected = hash.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        let plan = match crate::color_engine::plan_managed_render(&path, display_profile) {
            Ok(plan) => plan,
            Err(stopped) => return Ok(stopped),
        };
        let result = crate::color_engine::render_source_jpeg(
            &path,
            &plan.source_icc,
            &plan.display_icc,
            1400,
            Some(&hash),
        );
        if image_pipeline::fingerprint(&path)? != hash {
            return Err(AppError::new("FILE_CHANGED", "色彩检视期间原图内容已改变"));
        }
        let (jpeg, detail, status) = match result {
            Ok(jpeg) => (Some(jpeg), None, plan.status),
            Err(error) => (None, Some(error.message), "transform_failed"),
        };
        Ok(crate::color_engine::ManagedRender {
            status,
            detail,
            source_profile: plan.source_profile,
            display_profile: plan.display_profile,
            jpeg,
        })
    })
    .await
    .map_err(|e| AppError::new("BACKGROUND", e.to_string()))??;
    verify_current_source(store, project_id, photo_id, &expected).await?;
    Ok(outcome)
}

#[cfg(target_os = "macos")]
async fn verify_current_source(
    store: &Store,
    project: &str,
    photo: &str,
    hash: &str,
) -> Result<()> {
    let matches:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM photos p JOIN projects j ON j.id=p.project_id WHERE p.id=? AND p.project_id=? AND p.content_sha256=? AND j.deleted_at IS NULL)")
        .bind(photo).bind(project).bind(hash).fetch_one(&store.pool).await?;
    if !matches {
        return Err(AppError::new(
            "FILE_CHANGED",
            "照片版本或项目状态已改变，请重新打开检视",
        ));
    }
    Ok(())
}

/// 100% 检视的显示器色彩渲染：先走 `original_path` 的全部门禁（在册、未失联、指纹未变、
/// 尺寸与格式上限），再把**原图全分辨率**变换到显示器色彩空间写成缓存文件。原图只读，不改动。
/// 状态判定与缓存预览那条链路共用 `plan_managed_render`，两条链路对"为什么没变换"说法一致。
#[cfg(target_os = "macos")]
pub async fn display_managed_inspection(
    store: &Store,
    project_id: &str,
    photo_id: &str,
) -> Result<ManagedInspection> {
    let source = original_path(store, project_id, photo_id).await?;
    let InspectionSource::Original { path } = source else {
        return Ok(ManagedInspection {
            status: "source_not_original",
            detail: Some("该照片的检视来源是派生或嵌入预览，没有可用的全分辨率原图".to_owned()),
            source_profile: "无法确定".to_owned(),
            display_profile: "无法确定".to_owned(),
            path: None,
        });
    };
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT p.preview_path, p.content_sha256 FROM photos p JOIN projects j ON j.id=p.project_id WHERE p.project_id=? AND p.id=? AND j.deleted_at IS NULL"
    )
    .bind(project_id)
    .bind(photo_id)
    .fetch_optional(&store.pool)
    .await?;
    let (preview_path, content_sha256) =
        row.ok_or_else(|| AppError::new("NOT_FOUND", "照片不属于该项目或项目已归档"))?;
    let expected = content_sha256.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        let plan = match crate::color_engine::plan_managed_render(
            &path,
            crate::color_engine::native_transform::display_profile_bytes(),
        ) {
            Ok(plan) => plan,
            Err(stopped) => {
                return Ok(ManagedInspection {
                    status: stopped.status,
                    detail: stopped.detail,
                    source_profile: stopped.source_profile,
                    display_profile: stopped.display_profile,
                    path: None,
                });
            }
        };
        let target = managed_target(
            std::path::Path::new(&preview_path),
            &content_sha256,
            &plan.display_icc,
        );
        if unusable(&target)
            && let Err(error) = render_display_managed(
                &path,
                &plan.source_icc,
                &plan.display_icc,
                &target,
                Some(&content_sha256),
            )
        {
            return Ok(ManagedInspection {
                status: "transform_failed",
                detail: Some(error.message),
                source_profile: plan.source_profile,
                display_profile: plan.display_profile,
                path: None,
            });
        }
        if image_pipeline::fingerprint(&path)? != content_sha256 {
            return Err(AppError::new("FILE_CHANGED", "色彩检视期间原图内容已改变"));
        }
        Ok(ManagedInspection {
            status: plan.status,
            detail: None,
            source_profile: plan.source_profile,
            display_profile: plan.display_profile,
            path: Some(target.to_string_lossy().into_owned()),
        })
    })
    .await
    .map_err(|error| AppError::new("BACKGROUND", error.to_string()))??;
    verify_current_source(store, project_id, photo_id, &expected).await?;
    Ok(outcome)
}

/// 解码→源ICC→显示ICC→带目标配置的 JPEG→tmp+rename；原图只读。
/// 写回显示器配置是必须的：文件里的数值已经是显示器色彩，不标注就会被渲染端按 sRGB 再管一次。
#[cfg(target_os = "macos")]
fn render_display_managed(
    original: &std::path::Path,
    source_icc: &[u8],
    display_icc: &[u8],
    target: &std::path::Path,
    expected_sha256: Option<&str>,
) -> Result<()> {
    let tagged = crate::color_engine::render_source_jpeg(
        original,
        source_icc,
        display_icc,
        30_000,
        expected_sha256,
    )?;
    let temporary = target.with_extension(format!("{}.tmp", crate::domain::id()));
    std::fs::write(&temporary, tagged)?;
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
            None,
        )
        .unwrap();
        let pixel = image::open(&target).unwrap().to_rgb8().get_pixel(1, 0).0;
        assert!(pixel[2] < 253 && pixel[2] >= 235, "blue got {pixel:?}");
        assert!(pixel[0] <= 8 && pixel[1] <= 8, "got {pixel:?}");
        // 数值已是 P3，标注必须是**目标**显示器配置；标成源或不标，渲染端都会再管一次。
        assert_eq!(
            crate::color_engine::embedded_icc_bytes(&target)
                .unwrap()
                .expect("渲染结果必须带色彩配置"),
            system_profile("Display P3.icc")
        );
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
        render_display_managed(&original, &srgb, &srgb, &target, None).unwrap();
        let pixel = image::open(&target).unwrap().to_rgb8().get_pixel(2, 0).0;
        assert!((pixel[0] as i32 - 200).abs() <= 8, "got {pixel:?}");
        assert!((pixel[1] as i32 - 30).abs() <= 8, "got {pixel:?}");
    }

    #[test]
    fn source_render_applies_orientation_and_composites_transparency_without_touching_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rotated.jpg");
        let mut jpeg = vec![];
        image::RgbImage::from_pixel(32, 16, image::Rgb([30, 90, 180]))
            .write_to(
                &mut std::io::Cursor::new(&mut jpeg),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        let tiff=b"II\x2a\x00\x08\x00\x00\x00\x01\x00\x12\x01\x03\x00\x01\x00\x00\x00\x06\x00\x00\x00\x00\x00\x00\x00";
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend_from_slice(tiff);
        let mut oriented = jpeg[..2].to_vec();
        oriented.extend_from_slice(&[0xff, 0xe1]);
        oriented.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
        oriented.extend(exif);
        oriented.extend_from_slice(&jpeg[2..]);
        std::fs::write(&path, &oriented).unwrap();
        let srgb = system_profile("sRGB Profile.icc");
        let hash = image_pipeline::fingerprint(&path).unwrap();
        let output =
            crate::color_engine::render_source_jpeg(&path, &srgb, &srgb, 1400, Some(&hash))
                .unwrap();
        let image = image::load_from_memory(&output).unwrap();
        assert_eq!((image.width(), image.height()), (16, 32));
        assert_eq!(std::fs::read(&path).unwrap(), oriented);
        let transparent = dir.path().join("alpha.png");
        image::RgbaImage::from_pixel(32, 16, image::Rgba([255, 0, 0, 0]))
            .save(&transparent)
            .unwrap();
        let hash = image_pipeline::fingerprint(&transparent).unwrap();
        let output =
            crate::color_engine::render_source_jpeg(&transparent, &srgb, &srgb, 1400, Some(&hash))
                .unwrap();
        assert!(
            image::load_from_memory(&output)
                .unwrap()
                .to_rgb8()
                .get_pixel(10, 10)
                .0
                .iter()
                .all(|c| *c >= 250)
        );
    }

    #[test]
    fn changed_snapshot_and_oversized_source_never_publish_a_render_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source.png");
        image::RgbImage::from_pixel(32, 16, image::Rgb([20, 100, 180]))
            .save(&path)
            .unwrap();
        let srgb = system_profile("sRGB Profile.icc");
        let target = dir.path().join("managed.jpg");
        assert_eq!(
            render_display_managed(&path, &srgb, &srgb, &target, Some(&"0".repeat(64)))
                .unwrap_err()
                .code,
            "FILE_CHANGED"
        );
        assert!(!target.exists());
        let large = dir.path().join("large.png");
        image::GrayImage::new(8000, 4001).save(&large).unwrap();
        assert_eq!(
            crate::color_engine::render_source_jpeg(&large, &srgb, &srgb, 1400, None)
                .unwrap_err()
                .code,
            "INSPECTION_LIMIT"
        );
    }

    #[tokio::test]
    async fn explicit_preview_ignores_old_cache_and_checks_project_and_source_identity() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("color.sqlite")).await.unwrap();
        let project = store.create("原图检视", "SIMULATED").await.unwrap();
        let source = dir.path().join("source.png");
        image::RgbImage::from_pixel(32, 16, image::Rgb([20, 100, 180]))
            .save(&source)
            .unwrap();
        let prepared = image_pipeline::prepare(&source, &dir.path().join("cache")).unwrap();
        store.insert_photo(&project.id, &prepared).await.unwrap();
        let photo = store.get(&project.id).await.unwrap().photos[0].id.clone();
        image::RgbImage::from_pixel(32, 16, image::Rgb([230, 10, 10]))
            .save(&prepared.preview)
            .unwrap();
        let cached = std::fs::read(&prepared.preview).unwrap();
        let original = std::fs::read(&source).unwrap();
        let rendered = display_managed_preview_with_profile(
            &store,
            &project.id,
            &photo,
            Ok(system_profile("sRGB Profile.icc")),
        )
        .await
        .unwrap();
        let decoded = image::load_from_memory(rendered.jpeg.as_ref().unwrap())
            .unwrap()
            .to_rgb8();
        let pixel = decoded.get_pixel(10, 10).0;
        assert!(
            pixel[0] < 40 && pixel[1] > 80 && pixel[2] > 160,
            "{pixel:?}"
        );
        assert_eq!(std::fs::read(&prepared.preview).unwrap(), cached);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        assert!(
            display_managed_preview_with_profile(
                &store,
                "foreign",
                &photo,
                Ok(system_profile("sRGB Profile.icc"))
            )
            .await
            .is_err()
        );
        image::RgbImage::from_pixel(32, 16, image::Rgb([200, 10, 10]))
            .save(&source)
            .unwrap();
        assert_eq!(
            display_managed_preview_with_profile(
                &store,
                &project.id,
                &photo,
                Ok(system_profile("sRGB Profile.icc"))
            )
            .await
            .unwrap_err()
            .code,
            "FILE_CHANGED"
        );
    }
}
