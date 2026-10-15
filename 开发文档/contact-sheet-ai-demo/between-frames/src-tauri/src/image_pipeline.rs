use crate::domain::{AppError, Result};
use image::{ImageDecoder, ImageReader};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

pub struct PreparedPhoto {
    pub path: PathBuf,
    pub filename: String,
    pub hash: String,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub thumbnail: PathBuf,
    pub preview: PathBuf,
}

pub const EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "tif", "tiff", "heic", "heif", "dng", "cr2", "cr3", "nef", "arw",
    "raf", "orf", "rw2", "raw",
];

pub fn fingerprint(path: &Path) -> Result<String> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut chunk = [0_u8; 64 * 1024];
    loop {
        let size = reader.read(&mut chunk)?;
        if size == 0 {
            break;
        }
        hasher.update(&chunk[..size]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub const DEFAULT_CACHE_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub fn cache_limit_bytes() -> u64 {
    std::env::var("BETWEEN_FRAMES_CACHE_LIMIT_BYTES")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_CACHE_LIMIT_BYTES)
}

/// 缓存配额清理（M1-09 / M5）：删除不被任何 photo 行引用的孤儿缓存
/// （重复导入、取消导入、照片删除后的残留）。被引用的缓存不动——
/// UI 路径始终有效，零体验影响；引用缓存的淘汰需要懒重建支持，留真实用户反馈后评估。
/// 返回清理的字节数；单文件删除失败跳过（只读卷等）不阻塞。
pub fn sweep_orphans(cache: &Path, live_hashes: &[String], limit_bytes: u64) -> Result<u64> {
    let entries = match std::fs::read_dir(cache) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(AppError::new(
                "INVALID_PATH",
                format!("缓存目录不可读：{error}"),
            ));
        }
    };
    let mut orphans: Vec<(PathBuf, std::time::SystemTime, u64)> = vec![];
    let mut live_bytes: u64 = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // 仅清理本应用命名规则的缓存文件（{hash}-320.jpg / {hash}-1400.jpg）。
        let owned = match name
            .strip_suffix("-320.jpg")
            .or(name.strip_suffix("-1400.jpg"))
        {
            Some(hash) => hash,
            None => continue,
        };
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let size = metadata.len();
        if live_hashes.iter().any(|live| live == owned) {
            live_bytes += size;
        } else {
            let modified = metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            orphans.push((path, modified, size));
        }
    }
    // 配额约束总占用（引用 + 孤儿）：超限时按最旧优先删孤儿，直到回到限额内。
    let mut total = live_bytes;
    for (_, _, size) in &orphans {
        total += size;
    }
    if total <= limit_bytes {
        return Ok(0);
    }
    orphans.sort_by_key(|(_, modified, _)| *modified);
    let mut reclaimed = 0_u64;
    for (path, _, size) in orphans {
        if total <= limit_bytes {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            reclaimed += size;
            total = total.saturating_sub(size);
        }
    }
    Ok(reclaimed)
}

pub fn prepare(path: &Path, cache: &Path) -> Result<PreparedPhoto> {
    let path = path.canonicalize()?;
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !EXTENSIONS.contains(&extension.as_str()) {
        return Err(AppError::new("UNSUPPORTED_FORMAT", "此文件格式不受支持"));
    }
    let metadata = path.metadata()?;
    if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
        return Err(AppError::new(
            "FILE_TOO_LARGE",
            "照片必须为不超过 512 MB 的文件",
        ));
    }
    let hash = fingerprint(&path)?;
    let thumbnail = cache.join(format!("{hash}-320.jpg"));
    let preview = cache.join(format!("{hash}-1400.jpg"));
    std::fs::create_dir_all(cache)?;
    let mut native = false;
    let mut bitmap = match decode(&path) {
        Ok(bitmap) => bitmap,
        Err(error)
            if matches!(
                extension.as_str(),
                "heic"
                    | "heif"
                    | "dng"
                    | "cr2"
                    | "cr3"
                    | "nef"
                    | "arw"
                    | "raf"
                    | "orf"
                    | "rw2"
                    | "raw"
            ) =>
        {
            native = true;
            platform_decode(&path, cache).map_err(|native| {
                AppError::new(
                    "PLATFORM_DECODE",
                    format!("平台解码不可用：{}；{}", error.message, native.message),
                )
            })?
        }
        Err(error) => return Err(error),
    };
    let (width, height) = (bitmap.width(), bitmap.height());
    let (source_width, source_height) = if native {
        platform_dimensions(&path)?
    } else {
        (width, height)
    };
    if width == 0 || height == 0 {
        return Err(AppError::new("DECODE", "照片没有有效图像尺寸"));
    }
    // 白底合成透明通道；缓存始终是缩小后的 JPEG，不改变源文件。
    if bitmap.color().has_alpha() {
        let mut rgb = image::RgbImage::new(width, height);
        for (x, y, pixel) in bitmap.into_rgba8().enumerate_pixels() {
            let alpha = pixel[3] as u32;
            rgb.put_pixel(
                x,
                y,
                image::Rgb(std::array::from_fn(|i| {
                    ((pixel[i] as u32 * alpha + 255 * (255 - alpha)) / 255) as u8
                })),
            );
        }
        bitmap = image::DynamicImage::ImageRgb8(rgb);
    } else {
        bitmap = image::DynamicImage::ImageRgb8(bitmap.into_rgb8());
    }
    // thumbnail() 会放大小图；只缩小，避免低分辨率图片产生无用的大缓存。
    if !thumbnail.is_file() {
        save_cache(
            &bitmap.thumbnail(width.min(320), height.min(320)),
            &thumbnail,
        )?;
    }
    if !preview.is_file() {
        save_cache(
            &bitmap.thumbnail(width.min(1400), height.min(1400)),
            &preview,
        )?;
    }
    if fingerprint(&path)? != hash {
        return Err(AppError::new(
            "FILE_CHANGED",
            "导入期间原图发生变化，请重试",
        ));
    }
    Ok(PreparedPhoto {
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        path,
        hash,
        size: metadata.len(),
        width: source_width,
        height: source_height,
        thumbnail,
        preview,
    })
}

fn decode(path: &Path) -> Result<image::DynamicImage> {
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    limits.max_image_width = Some(30_000);
    limits.max_image_height = Some(30_000);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| AppError::new("DECODE", error.to_string()))?;
    let orientation = decoder
        .orientation()
        .map_err(|error| AppError::new("DECODE", error.to_string()))?;
    let mut bitmap = image::DynamicImage::from_decoder(decoder)
        .map_err(|error| AppError::new("DECODE", error.to_string()))?;
    bitmap.apply_orientation(orientation);
    Ok(bitmap)
}

fn save_cache(bitmap: &image::DynamicImage, path: &Path) -> Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", crate::domain::id()));
    bitmap
        .save_with_format(&temporary, image::ImageFormat::Jpeg)
        .map_err(|error| AppError::new("CACHE_WRITE", error.to_string()))?;
    std::fs::rename(&temporary, path)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn platform_decode(path: &Path, cache: &Path) -> Result<image::DynamicImage> {
    let temporary = cache.join(format!("native-{}.png", crate::domain::id()));
    // 仅由业务后端调用固定系统解码器，路径作为参数，不交给 shell 解释。
    let result = std::process::Command::new("/usr/bin/sips")
        .args(["-s", "format", "png", "-Z", "1400"])
        .arg(path)
        .arg("--out")
        .arg(&temporary)
        .output()?;
    if !result.status.success() {
        return Err(AppError::new(
            "PLATFORM_DECODE",
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
        ));
    }
    let image = decode(&temporary);
    // 这里只清理应用自身生成的中间缓存，绝不删除原图。
    let _ = std::fs::remove_file(temporary);
    image
}

#[cfg(target_os = "macos")]
fn platform_dimensions(path: &Path) -> Result<(u32, u32)> {
    let output = std::process::Command::new("/usr/bin/sips")
        .args(["-g", "pixelWidth", "-g", "pixelHeight"])
        .arg(path)
        .output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let value = |name: &str| {
        text.lines().find_map(|line| {
            line.trim()
                .strip_prefix(name)
                .and_then(|value| value.trim().parse::<u32>().ok())
        })
    };
    match (value("pixelWidth:"), value("pixelHeight:")) {
        (Some(width), Some(height)) if width > 0 && height > 0 => Ok((width, height)),
        _ => Err(AppError::new("PLATFORM_DECODE", "无法读取原图尺寸")),
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_dimensions(_path: &Path) -> Result<(u32, u32)> {
    Err(AppError::new("PLATFORM_DECODE", "需要 macOS 图像解码器"))
}

#[cfg(not(target_os = "macos"))]
fn platform_decode(_path: &Path, _cache: &Path) -> Result<image::DynamicImage> {
    Err(AppError::new(
        "PLATFORM_DECODE",
        "HEIC/RAW 需要 macOS 平台解码器",
    ))
}
