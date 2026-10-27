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
