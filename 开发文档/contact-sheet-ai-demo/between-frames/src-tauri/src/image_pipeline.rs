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
    let mut scaled_source: Option<(u32, u32)> = None;
    let mut bitmap = match decode_scaled(&path, &extension) {
        Some((source, scaled)) => {
            scaled_source = Some(source);
            scaled
        }
        None => match decode(&path) {
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
        },
    };
    let (width, height) = (bitmap.width(), bitmap.height());
    let (source_width, source_height) = if native {
        platform_dimensions(&path)?
    } else if let Some(dims) = scaled_source {
        dims
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

/// G2-① 接入阈值：源长边超过该值走 ImageIO 缩放解码。2800 ≥ 预览 1400 的 2 倍，
/// 缩放后再缩到 1400 无可见质量损失；对照数据见 G2 评审包 §2（25MP：270ms/7.8MB 对 2324ms/100MB）。
#[cfg(target_os = "macos")]
const SCALED_DECODE_THRESHOLD: u32 = 2800;

/// 高分辨率光栅源在 macOS 经 ImageIO 按长边缩放解码，避免整图解码峰值内存；
/// 尺寸从文件头读取（不解码像素），ImageIO 不支持或失败时返回 None 回退标准解码。
#[cfg(target_os = "macos")]
fn decode_scaled(path: &Path, extension: &str) -> Option<((u32, u32), image::DynamicImage)> {
    if !matches!(extension, "jpg" | "jpeg" | "png" | "tif" | "tiff") {
        return None;
    }
    let header = image::image_dimensions(path).ok()?;
    if header.0.max(header.1) <= SCALED_DECODE_THRESHOLD {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let (width, height, rgba) =
        scaled_decode::decode_rgba_scaled(&bytes, SCALED_DECODE_THRESHOLD).ok()?;
    let buffer = image::RgbaImage::from_raw(width, height, rgba)?;
    // ImageIO 缩略图已应用 EXIF 方向：90°/270° 旋转时文件头尺寸与方向不一致，按输出纵横比对齐。
    let source = if (header.0 > header.1) != (width > height) {
        (header.1, header.0)
    } else {
        header
    };
    Some((
        source,
        image::DynamicImage::ImageRgba8(unpremultiplied(buffer)),
    ))
}

#[cfg(not(target_os = "macos"))]
fn decode_scaled(_path: &Path, _extension: &str) -> Option<((u32, u32), image::DynamicImage)> {
    None
}

/// CoreGraphics 位图上下文输出预乘 alpha；去预乘后复用既有直 alpha 白底合成。
#[cfg(target_os = "macos")]
fn unpremultiplied(buffer: image::RgbaImage) -> image::RgbaImage {
    image::RgbaImage::from_fn(buffer.width(), buffer.height(), |x, y| {
        let pixel = buffer.get_pixel(x, y);
        let alpha = pixel[3] as u32;
        if alpha == 0 || alpha >= 255 {
            return *pixel;
        }
        image::Rgba([
            ((pixel[0] as u32 * 255 + alpha / 2) / alpha).min(255) as u8,
            ((pixel[1] as u32 * 255 + alpha / 2) / alpha).min(255) as u8,
            ((pixel[2] as u32 * 255 + alpha / 2) / alpha).min(255) as u8,
            pixel[3],
        ])
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

/// FR3-015 瓦片金字塔原型：有界内存的多级缩放瓦片与内容寻址缓存键。
/// 当前为 CPU 级 2× 盒式降采样；真正局部解码与原生管线接入在 G2 评审后推进。
pub mod tile_pyramid {
    use crate::domain::{AppError, Result};

    /// 瓦片边长（像素）；金字塔每级 2× 缩小，直到最长边 ≤ TILE。
    pub const TILE: u32 = 256;
    /// 降采样算法版本：纳入缓存键，算法变化时旧瓦片自然失效。
    pub const PYRAMID_VERSION: u8 = 1;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct TileKey {
        pub source_sha256: String,
        pub level: u8,
        pub x: u32,
        pub y: u32,
    }

    impl TileKey {
        pub fn cache_string(&self) -> String {
            format!(
                "v{}.{}.{}.{}.{}",
                PYRAMID_VERSION, self.source_sha256, self.level, self.x, self.y
            )
        }
    }

    pub fn level_count(width: u32, height: u32) -> Result<u8> {
        if width == 0 || height == 0 {
            return Err(AppError::new("VALIDATION_ERROR", "金字塔尺寸必须为正"));
        }
        let mut levels = 1u8;
        let mut edge = width.max(height);
        while edge > TILE {
            edge = edge.div_ceil(2);
            levels += 1;
        }
        Ok(levels)
    }

    /// 2× 盒式降采样：每 2×2 块取算术平均（含 alpha）；奇数行列的边缘像素丢弃。
    pub fn downsample_2x(width: u32, height: u32, rgba: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
        if width == 0 || height == 0 {
            return Err(AppError::new("VALIDATION_ERROR", "降采样尺寸必须为正"));
        }
        if rgba.len() != width as usize * height as usize * 4 {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "像素缓冲长度与尺寸不匹配",
            ));
        }
        let (out_w, out_h) = (width / 2, height / 2);
        if out_w == 0 || out_h == 0 {
            return Ok((width, height, rgba.to_vec()));
        }
        let stride = width as usize * 4;
        let mut out = Vec::with_capacity(out_w as usize * out_h as usize * 4);
        for y in 0..out_h {
            for x in 0..out_w {
                let base = (y as usize * 2) * stride + (x as usize * 2) * 4;
                for component in 0..4 {
                    let sum = rgba[base + component] as u32
                        + rgba[base + 4 + component] as u32
                        + rgba[base + stride + component] as u32
                        + rgba[base + stride + 4 + component] as u32;
                    out.push((sum / 4) as u8);
                }
            }
        }
        Ok((out_w, out_h, out))
    }

    /// 提取 (x, y) 号瓦片；边缘瓦片可能小于 TILE×TILE，坐标完全越界则报错。
    pub fn tile_pixels(
        rgba: &[u8],
        width: u32,
        height: u32,
        x: u32,
        y: u32,
    ) -> Result<(u32, u32, Vec<u8>)> {
        if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "像素缓冲长度与尺寸不匹配",
            ));
        }
        let tile_w = TILE.min(width.saturating_sub(x * TILE));
        let tile_h = TILE.min(height.saturating_sub(y * TILE));
        if tile_w == 0 || tile_h == 0 {
            return Err(AppError::new("VALIDATION_ERROR", "瓦片坐标超出图像范围"));
        }
        let mut out = vec![0u8; tile_w as usize * tile_h as usize * 4];
        for row in 0..tile_h {
            let src = ((y * TILE + row) as usize * width as usize + (x * TILE) as usize) * 4;
            let dst = row as usize * tile_w as usize * 4;
            out[dst..dst + tile_w as usize * 4]
                .copy_from_slice(&rgba[src..src + tile_w as usize * 4]);
        }
        Ok((tile_w, tile_h, out))
    }

    /// 构建完整金字塔（level 0 为原图）；总内存 ≤ 原图 × 4/3（几何级数）。
    /// 构建完整金字塔（level 0 为原图）；总内存 ≤ 原图 × 4/3（几何级数）。
    pub fn build_levels(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<(u32, u32, Vec<u8>)>> {
        build_levels_with_cancel(width, height, rgba, &|| false)
    }

    /// FR3-015 取消：起点与每级之间检查回调；取消返回 CANCELLED 结构化错误，
    /// 不返回部分结果——调用方应放弃本次构建，已分配内存随返回值释放。
    pub fn build_levels_with_cancel(
        width: u32,
        height: u32,
        rgba: &[u8],
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<Vec<(u32, u32, Vec<u8>)>> {
        let count = level_count(width, height)?;
        if should_cancel() {
            return Err(AppError::new("CANCELLED", "金字塔构建已取消"));
        }
        let mut levels = Vec::with_capacity(count as usize);
        levels.push((width, height, rgba.to_vec()));
        while levels.len() < count as usize {
            if should_cancel() {
                return Err(AppError::new("CANCELLED", "金字塔构建已取消"));
            }
            let (w, h, data) = levels.last().expect("level 0 已推入");
            let (nw, nh, nd) = downsample_2x(*w, *h, data)?;
            // 一维尺寸为 1 时无法再缩小，提前终止，避免与 level_count 计数死循环。
            if nw == *w && nh == *h {
                break;
            }
            levels.push((nw, nh, nd));
        }
        Ok(levels)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::collections::HashSet;

        fn solid(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
            pixel
                .iter()
                .copied()
                .cycle()
                .take(width as usize * height as usize * 4)
                .collect()
        }

        #[test]
        fn level_count_halves_until_tile() {
            assert_eq!(level_count(256, 256).unwrap(), 1);
            assert_eq!(level_count(100, 80).unwrap(), 1);
            assert_eq!(level_count(1000, 800).unwrap(), 3);
            assert_eq!(level_count(5000, 300).unwrap(), 6);
            assert!(level_count(0, 10).is_err());
        }

        #[test]
        fn downsample_averages_2x2_blocks() {
            let rgba = vec![
                0, 0, 0, 255, 100, 100, 100, 255, //
                200, 200, 200, 255, 0, 0, 0, 255,
            ];
            let (w, h, out) = downsample_2x(2, 2, &rgba).unwrap();
            assert_eq!((w, h), (1, 1));
            assert_eq!(out, vec![75, 75, 75, 255]);
            let padded = solid(3, 3, [10, 20, 30, 255]);
            let (w, h, out) = downsample_2x(3, 3, &padded).unwrap();
            assert_eq!((w, h), (1, 1));
            assert_eq!(out, solid(1, 1, [10, 20, 30, 255]));
            assert!(downsample_2x(2, 2, &[0u8; 7]).is_err());
        }

        #[test]
        fn tile_extraction_clamps_edges() {
            let image = solid(300, 300, [1, 2, 3, 4]);
            let (w, h, t) = tile_pixels(&image, 300, 300, 0, 0).unwrap();
            assert_eq!((w, h), (256, 256));
            assert_eq!(t.len(), 256 * 256 * 4);
            let (w, h, t) = tile_pixels(&image, 300, 300, 1, 1).unwrap();
            assert_eq!((w, h), (44, 44));
            assert_eq!(t, solid(44, 44, [1, 2, 3, 4]));
            assert!(tile_pixels(&image, 300, 300, 2, 0).is_err());
        }

        #[test]
        fn cache_keys_are_stable_and_distinct() {
            let key = TileKey {
                source_sha256: "a".repeat(64),
                level: 0,
                x: 1,
                y: 2,
            };
            assert_eq!(key.cache_string(), key.clone().cache_string());
            assert!(key.cache_string().starts_with("v1."));
            let variants = [
                key.clone(),
                TileKey {
                    level: 1,
                    ..key.clone()
                },
                TileKey {
                    x: 2,
                    ..key.clone()
                },
                TileKey {
                    source_sha256: "b".repeat(64),
                    ..key.clone()
                },
            ];
            let unique: HashSet<_> = variants.iter().map(|k| k.cache_string()).collect();
            assert_eq!(unique.len(), variants.len());
        }

        #[test]
        fn pyramid_memory_stays_within_geometric_bound() {
            let (w, h) = (2048u32, 2048u32);
            let rgba = solid(w, h, [9, 8, 7, 255]);
            let levels = build_levels(w, h, &rgba).unwrap();
            assert_eq!(levels.len(), level_count(w, h).unwrap() as usize);
            let total: usize = levels.iter().map(|(_, _, data)| data.len()).sum();
            let bound = w as usize * h as usize * 4 * 4 / 3;
            assert!(total <= bound, "total={total} bound={bound}");
            let (coarse_w, _, _) = levels.last().unwrap();
            assert_eq!(*coarse_w, 256);
        }

        #[test]
        fn cancellable_build_matches_plain_when_never_cancelled() {
            let (w, h) = (1024u32, 768u32);
            let rgba = solid(w, h, [5, 6, 7, 255]);
            let plain = build_levels(w, h, &rgba).unwrap();
            let cancellable = build_levels_with_cancel(w, h, &rgba, &|| false).unwrap();
            assert_eq!(plain.len(), cancellable.len());
        }

        #[test]
        fn immediate_cancel_returns_structured_error_without_copy() {
            let rgba = solid(64, 64, [1, 1, 1, 255]);
            let calls = std::cell::Cell::new(0);
            let error = build_levels_with_cancel(64, 64, &rgba, &|| {
                calls.set(calls.get() + 1);
                true
            })
            .unwrap_err();
            assert_eq!(error.code, "CANCELLED");
            assert_eq!(calls.get(), 1);
        }

        #[test]
        fn mid_way_cancel_stops_between_levels() {
            let (w, h) = (2048u32, 2048u32);
            let rgba = solid(w, h, [3, 4, 5, 255]);
            let calls = std::cell::Cell::new(0);
            let error = build_levels_with_cancel(w, h, &rgba, &|| {
                calls.set(calls.get() + 1);
                calls.get() == 2
            })
            .unwrap_err();
            assert_eq!(error.code, "CANCELLED");
            assert!(calls.get() >= 2);
        }
    }
}

/// FR3-015 局部解码原型：经 ImageIO 缩略图按目标长边解码，不为缩小视图加载全分辨率像素。
/// 输出为预乘 alpha 的 RGBA8；区域级解码与原生管线接入在 G2 评审后推进。
#[cfg(target_os = "macos")]
pub mod scaled_decode {
    use crate::domain::{AppError, Result};
    use core_foundation::base::{CFRelease, CFType, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::data::{CFData, CFDataRef};
    use core_foundation::dictionary::{CFDictionaryRef, CFMutableDictionary};
    use core_foundation::number::CFNumber;
    use core_foundation::string::{CFString, CFStringRef};
    use core_graphics::color_space::CGColorSpace;
    use core_graphics::context::CGContext;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};
    use core_graphics::image::CGImage;
    use core_graphics::sys::CGImageRef;
    use foreign_types::ForeignType;
    use std::ffi::c_void;

    type CGImageSourceRef = *const c_void;

    #[link(name = "ImageIO", kind = "framework")]
    unsafe extern "C" {
        static kCGImageSourceThumbnailMaxPixelSize: CFStringRef;
        static kCGImageSourceCreateThumbnailFromImageAlways: CFStringRef;
        static kCGImageSourceCreateThumbnailWithTransform: CFStringRef;
        fn CGImageSourceCreateWithData(
            data: CFDataRef,
            options: CFDictionaryRef,
        ) -> CGImageSourceRef;
        fn CGImageSourceCreateThumbnailAtIndex(
            source: CGImageSourceRef,
            index: usize,
            options: CFDictionaryRef,
        ) -> CGImageRef;
    }

    struct Source(CGImageSourceRef);

    impl Drop for Source {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { CFRelease(self.0 as CFTypeRef) };
            }
        }
    }

    /// 就地把图像字节解码为长边不超过 max_edge 的预乘 RGBA8，保持纵横比。
    pub fn decode_rgba_scaled(image: &[u8], max_edge: u32) -> Result<(u32, u32, Vec<u8>)> {
        if max_edge == 0 {
            return Err(AppError::new("VALIDATION_ERROR", "max_edge 必须为正"));
        }
        let data = CFData::from_buffer(image);
        let raw =
            unsafe { CGImageSourceCreateWithData(data.as_concrete_TypeRef(), std::ptr::null()) };
        if raw.is_null() {
            return Err(AppError::new("IMAGE_DECODE", "图像数据无法解析"));
        }
        let source = Source(raw);
        let mut options = CFMutableDictionary::<CFString, CFType>::new();
        // 全局常量键用 get 规则包装：包时持有、drop 释放，与常量本身的永久存活配对。
        let size_key =
            unsafe { CFString::wrap_under_get_rule(kCGImageSourceThumbnailMaxPixelSize) };
        options.add(&size_key, &CFNumber::from(max_edge as i32).as_CFType());
        let always_key =
            unsafe { CFString::wrap_under_get_rule(kCGImageSourceCreateThumbnailFromImageAlways) };
        options.add(&always_key, &CFBoolean::true_value().as_CFType());
        let transform_key =
            unsafe { CFString::wrap_under_get_rule(kCGImageSourceCreateThumbnailWithTransform) };
        options.add(&transform_key, &CFBoolean::true_value().as_CFType());
        let options = options.to_immutable();
        let thumb = unsafe {
            CGImageSourceCreateThumbnailAtIndex(source.0, 0, options.as_concrete_TypeRef())
        };
        if thumb.is_null() {
            return Err(AppError::new("IMAGE_DECODE", "缩略图解码失败"));
        }
        let thumb = unsafe { CGImage::from_ptr(thumb) };
        let width = thumb.width() as u32;
        let height = thumb.height() as u32;
        if width == 0 || height == 0 {
            return Err(AppError::new("IMAGE_DECODE", "解码结果尺寸无效"));
        }
        let space = CGColorSpace::create_device_rgb();
        // kCGImageAlphaPremultipliedLast=1：预乘 alpha 的 32bpp RGBA——
        // macOS 位图不支持直连 alpha（kCGImageAlphaLast），Skip 系又会丢弃 alpha。
        let mut ctx = CGContext::create_bitmap_context(
            None,
            width as usize,
            height as usize,
            8,
            width as usize * 4,
            &space,
            1,
        );
        ctx.draw_image(
            CGRect::new(
                &CGPoint::new(0.0, 0.0),
                &CGSize::new(width as f64, height as f64),
            ),
            &thumb,
        );
        Ok((width, height, ctx.data().to_vec()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::io::Cursor;

        fn encode(width: u32, height: u32, color: [u8; 3], format: image::ImageFormat) -> Vec<u8> {
            let img = image::RgbImage::from_pixel(width, height, image::Rgb(color));
            let mut bytes = Vec::new();
            img.write_to(&mut Cursor::new(&mut bytes), format).unwrap();
            bytes
        }

        #[test]
        fn jpeg_decodes_to_requested_long_edge() {
            let jpeg = encode(2000, 1000, [200, 30, 40], image::ImageFormat::Jpeg);
            let (w, h, rgba) = decode_rgba_scaled(&jpeg, 500).unwrap();
            assert_eq!((w, h), (500, 250));
            assert_eq!(rgba.len(), 500 * 250 * 4);
            assert!(rgba[0] > 150 && rgba[1] < 90 && rgba[2] < 90);
        }

        #[test]
        fn png_decodes_and_original_size_is_respected() {
            let png = encode(300, 150, [10, 200, 20], image::ImageFormat::Png);
            let (w, h, rgba) = decode_rgba_scaled(&png, 100).unwrap();
            assert_eq!((w, h), (100, 50));
            assert_eq!(rgba.len(), 100 * 50 * 4);
            let (w, h, _) = decode_rgba_scaled(&png, 1000).unwrap();
            assert_eq!((w, h), (300, 150));
        }

        #[test]
        fn invalid_input_is_rejected_with_structured_errors() {
            assert_eq!(
                decode_rgba_scaled(b"not an image", 100).unwrap_err().code,
                "IMAGE_DECODE"
            );
            let jpeg = encode(64, 64, [0, 0, 0], image::ImageFormat::Jpeg);
            assert_eq!(
                decode_rgba_scaled(&jpeg, 0).unwrap_err().code,
                "VALIDATION_ERROR"
            );
        }

        #[test]
        fn stress_25mp_scaled_decode_vs_full_decode() {
            use std::time::Instant;
            // 25MP 确定性伪随机灰度：不可压缩内容让两条路径都付出真实解码/编码代价，
            // 避免纯色 JPEG 的退化解码。计时用 --nocapture 查看，作 G2 决策点 1 的对照数据。
            let edge = 5000u32;
            let mut img = image::GrayImage::new(edge, edge);
            let mut seed = 0x1234_5678u32;
            for pixel in img.pixels_mut() {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *pixel = image::Luma([(seed >> 24) as u8]);
            }
            let mut jpeg = Vec::new();
            img.write_to(&mut Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
                .unwrap();

            let start = Instant::now();
            let (w, h, rgba) = decode_rgba_scaled(&jpeg, 1400).unwrap();
            let scaled_ms = start.elapsed().as_millis();
            assert_eq!((w, h), (1400, 1400));
            assert_eq!(rgba.len(), 1400 * 1400 * 4);

            let start = Instant::now();
            let full = image::load_from_memory(&jpeg).unwrap();
            let full_ms = start.elapsed().as_millis();
            assert_eq!((full.width(), full.height()), (edge, edge));

            // 内存上界由 API 契约决定：全量缓冲 edge²×4 对缩放输出 1400²×4（约 12.8 倍）。
            let full_bytes = edge as u64 * edge as u64 * 4;
            let scaled_bytes = 1400u64 * 1400 * 4;
            println!(
                "25MP decode compare: scaled={scaled_ms}ms({scaled_bytes}B) full={full_ms}ms({full_bytes}B) ratio={:.1}x",
                full_bytes as f64 / scaled_bytes as f64
            );
            assert!(full_bytes / scaled_bytes > 10);
        }
    }
}

#[cfg(test)]
mod prepare_tests {
    use super::*;

    fn write_fixture(dir: &Path, name: &str, bitmap: image::DynamicImage) -> std::path::PathBuf {
        let path = dir.join(name);
        bitmap
            .save_with_format(&path, image::ImageFormat::from_path(&path).unwrap())
            .unwrap();
        path
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn high_resolution_sources_decode_scaled_but_report_source_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        // 6000×4000 超过 2800 阈值：走缩放解码；报告尺寸必须是真实源尺寸。
        let bitmap = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            6000,
            4000,
            image::Rgb([180, 60, 60]),
        ));
        let path = write_fixture(dir.path(), "large.jpg", bitmap);
        let cache = dir.path().join("cache");
        let prepared = prepare(&path, &cache).unwrap();
        assert_eq!((prepared.width, prepared.height), (6000, 4000));
        let preview = image::open(&prepared.preview).unwrap();
        assert_eq!(preview.width().max(preview.height()), 1400);
        let thumbnail = image::open(&prepared.thumbnail).unwrap();
        assert_eq!(thumbnail.width().max(thumbnail.height()), 320);
    }

    #[test]
    fn small_sources_keep_existing_decode_path() {
        let dir = tempfile::tempdir().unwrap();
        let bitmap = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            800,
            600,
            image::Rgb([30, 160, 90]),
        ));
        let path = write_fixture(dir.path(), "small.jpg", bitmap);
        let cache = dir.path().join("cache");
        let prepared = prepare(&path, &cache).unwrap();
        assert_eq!((prepared.width, prepared.height), (800, 600));
        assert!(prepared.preview.is_file());
        assert!(prepared.thumbnail.is_file());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn scaled_alpha_sources_composite_onto_white_correctly() {
        let dir = tempfile::tempdir().unwrap();
        // 3000×2000 半透明绿 PNG：去预乘后 50% 绿 over white 应接近 (128, 255, 128)。
        // 若漏做去预乘，绿色通道会落到 ~192——该断言用于区分。
        let mut rgba = image::RgbaImage::new(3000, 2000);
        for (_, _, pixel) in rgba.enumerate_pixels_mut() {
            *pixel = image::Rgba([0, 255, 0, 128]);
        }
        let path = write_fixture(
            dir.path(),
            "alpha.png",
            image::DynamicImage::ImageRgba8(rgba),
        );
        let cache = dir.path().join("cache");
        let prepared = prepare(&path, &cache).unwrap();
        let preview = image::open(&prepared.preview).unwrap().to_rgb8();
        let center = preview.get_pixel(preview.width() / 2, preview.height() / 2);
        assert!((center[0] as i32 - 128).abs() <= 12, "got {center:?}");
        assert!((center[1] as i32 - 255).abs() <= 12, "got {center:?}");
        assert!((center[2] as i32 - 128).abs() <= 12, "got {center:?}");
    }
}
