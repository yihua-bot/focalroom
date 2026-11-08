use crate::domain::{AppError, Result};
use image::{ImageDecoder, ImageReader};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufReader, Cursor, Read},
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

/// 渲染临时文件至少要活过这么久才可能被回收。
const DERIVED_TEMP_MIN_AGE: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// 缓存配额清理（M1-09 / M5）：删除不被任何 photo 行引用的孤儿缓存
/// （重复导入、取消导入、照片删除后的残留）。被引用的缓存不动——
/// UI 路径始终有效，零体验影响；引用缓存的淘汰需要懒重建支持，留真实用户反馈后评估。
/// 显示器渲染产物（`managed-*`）不写进数据库，没有"被引用"可言：计入配额并按最旧回收，
/// 下次 100% 检视会按同样规则重渲染。
/// 画布瓦片（`v{版本}.{源SHA}.*`）同一条路：版本过期的直接清掉，当前版本的按照片是否仍在库
/// 决定算引用还是算孤儿。
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
    let mut stale_reclaimed = 0_u64;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // 只认本应用的命名规则；目录里别人放的东西一律不动。
        let managed_render =
            name.starts_with("managed-") && (name.ends_with(".jpg") || name.ends_with(".tmp"));
        let tile = tile_parts(name);
        let owned = match name
            .strip_suffix("-320.jpg")
            .or(name.strip_suffix("-1400.jpg"))
        {
            Some(hash) => Some(hash),
            None if managed_render || tile.is_some() => None,
            None => continue,
        };
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let size = metadata.len();
        let modified = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        // 临时文件是渲染或瓦片构建中途失败、进程被杀留下的；刚写出的不动，免得和仍在进行的任务争同一个文件。
        if name.ends_with(".tmp")
            && !matches!(modified.elapsed(), Ok(age) if age >= DERIVED_TEMP_MIN_AGE)
        {
            continue;
        }
        // 画布瓦片文件名里写着它属于哪张原图、用哪版算法：不认这个名字，看过一次图留下的
        // 几十个小文件就永远不会被回收。
        if let Some((version, sha)) = tile {
            // 不是当前版本的瓦片不会再被任何请求命中，留着只是占盘，不必等配额压力。
            if version != tile_pyramid::PYRAMID_VERSION {
                if std::fs::remove_file(&path).is_ok() {
                    stale_reclaimed += size;
                }
                continue;
            }
            if live_hashes.iter().any(|live| live == sha) {
                live_bytes += size;
                continue;
            }
            orphans.push((path, modified, size));
            continue;
        }
        if let Some(hash) = owned
            && live_hashes.iter().any(|live| live == hash)
        {
            live_bytes += size;
            continue;
        }
        orphans.push((path, modified, size));
    }
    // 配额约束总占用（引用 + 孤儿）：超限时按最旧优先删孤儿，直到回到限额内。
    let mut total = live_bytes;
    for (_, _, size) in &orphans {
        total += size;
    }
    if total <= limit_bytes {
        return Ok(stale_reclaimed);
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
    Ok(reclaimed + stale_reclaimed)
}

/// 画布瓦片文件名 `v{版本}.{sha256}.{层}.{x}.{y}.jpg`，构建中途留下的临时件是同一路径换成
/// `.{随机号}.tmp` 后缀。不符合这两种形状的一律返回 None，免得把用户自己丢进缓存目录的文件
/// 当成回收对象。
fn tile_parts(name: &str) -> Option<(u8, &str)> {
    let mut parts = name.split('.');
    let version: u8 = parts.next()?.strip_prefix('v')?.parse().ok()?;
    let sha = parts.next()?;
    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let coords: Vec<&str> = parts.by_ref().take(3).collect();
    if coords
        .iter()
        .any(|value| value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let tail = parts.collect::<Vec<_>>().join(".");
    (tail == "jpg" || tail.ends_with(".tmp")).then_some((version, sha))
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
    let mut scaled_decode = false;
    let mut scaled_source: Option<(u32, u32)> = None;
    let mut bitmap = match decode_scaled(&path, &extension) {
        Some((source, scaled)) => {
            scaled_source = Some(source);
            scaled_decode = true;
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
    // 缓存标注只声明这条路实际做到的事：缩放解码已把像素显式映射到 sRGB；标准解码保留源数值，
    // 于是照抄源自己的配置；平台解码（sips 转 PNG）说不清，就留一张没有声明的缓存。
    let cache_profile: Option<Vec<u8>> = if native || (thumbnail.is_file() && preview.is_file()) {
        None
    } else if scaled_decode {
        crate::color_engine::srgb_cache_profile_bytes()
    } else {
        crate::color_engine::source_profile_bytes_for_cache(&path)
    };
    // thumbnail() 会放大小图；只缩小，避免低分辨率图片产生无用的大缓存。
    if !thumbnail.is_file() {
        save_cache(
            &bitmap.thumbnail(width.min(320), height.min(320)),
            &thumbnail,
            cache_profile.as_deref(),
        )?;
    }
    if !preview.is_file() {
        save_cache(
            &bitmap.thumbnail(width.min(1400), height.min(1400)),
            &preview,
            cache_profile.as_deref(),
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

/// 解码像素前必须过的门禁：单次分配上限与单边像素上限。
/// 头尺寸检查与整图解码共用同一份，避免出现"一条路拦、另一条路放行"的第二个真相。
fn decode_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    limits.max_image_width = Some(30_000);
    limits.max_image_height = Some(30_000);
    limits
}

fn decode(path: &Path) -> Result<image::DynamicImage> {
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    reader.limits(decode_limits());
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

/// 写出派生缓存。`icc` 是这条解码路径**实际做到**的那次映射的身份：写进文件但不碰像素数值，
/// 于是"这份缓存说自己在哪个空间"变成可以核对的事实，而不是读侧的猜测。
fn save_cache(bitmap: &image::DynamicImage, path: &Path, icc: Option<&[u8]>) -> Result<()> {
    let mut bytes = Vec::new();
    bitmap
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg)
        .map_err(|error| AppError::new("CACHE_WRITE", error.to_string()))?;
    let bytes = match icc {
        Some(icc) => crate::color_engine::tag_jpeg_with_icc(&bytes, icc)?,
        None => bytes,
    };
    let temporary = path.with_extension(format!("{}.tmp", crate::domain::id()));
    std::fs::write(&temporary, bytes)
        .map_err(|error| AppError::new("CACHE_WRITE", format!("无法写入色彩缓存：{error}")))?;
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

/// FR3-015 瓦片金字塔：多级缩放瓦片与内容寻址缓存键。
/// 层级 > 0 且源图长边超过缩放门槛时交给 ImageIO 边解码边缩到目标档，只为那一档付像素；
/// 全分辨率档、小图与 HEIC/RAW 仍走标准解码。层级尺寸逐级向上取整折半，与前端算格子同口径。
pub mod tile_pyramid {
    use crate::domain::{AppError, Result};

    /// 瓦片边长（像素）；金字塔每级 2× 缩小，直到最长边 ≤ TILE。
    pub const TILE: u32 = 256;
    /// 降采样算法版本：纳入缓存键，算法变化时旧瓦片自然失效。
    /// v2 起瓦片按 EXIF 方向取图并把透明合成白底；v3 起层级尺寸改为向上取整（与前端算格子
    /// 的口径一致，边缘像素不再被丢掉），v2 留下的瓦片网格可能对不齐，不能继续命中。
    pub const PYRAMID_VERSION: u8 = 3;

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

    /// 某一层的宽高：每级取一半并向上进位（513→257→129）。前端按同样的规则算需要几列几行，
    /// 两边规则不一致时最外面那一列会要不存在的瓦片，画布只能退回糊掉的缩略图。
    pub fn level_dims(width: u32, height: u32, level: u8) -> (u32, u32) {
        let step = 1_u32 << level.min(31);
        (width.div_ceil(step).max(1), height.div_ceil(step).max(1))
    }

    /// 2× 盒式降采样：每 2×2 块取算术平均（含 alpha）；边缘只剩半个块时按实际存在的像素平均。
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
        let (out_w, out_h) = (width.div_ceil(2).max(1), height.div_ceil(2).max(1));
        if out_w == width && out_h == height {
            return Ok((width, height, rgba.to_vec()));
        }
        let stride = width as usize * 4;
        let mut out = Vec::with_capacity(out_w as usize * out_h as usize * 4);
        for y in 0..out_h {
            for x in 0..out_w {
                let mut sums = [0_u32; 4];
                let mut taken = 0_u32;
                for row in [y * 2, y * 2 + 1] {
                    if row >= height {
                        continue;
                    }
                    for column in [x * 2, x * 2 + 1] {
                        if column >= width {
                            continue;
                        }
                        let base = (row as usize) * stride + (column as usize) * 4;
                        taken += 1;
                        for component in 0..4 {
                            sums[component] += rgba[base + component] as u32;
                        }
                    }
                }
                out.extend(sums.iter().map(|total| (*total / taken) as u8));
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

    /// 只求到目标层级需要的那一段降采样链：逐级 2× 并立刻释放上一级。
    /// 不再为取一小块瓦片把整棵金字塔同时留在内存里；缩不下去时如实报错，不返回上层重复数据。
    pub fn level_pixels(
        width: u32,
        height: u32,
        rgba: &[u8],
        level: u8,
    ) -> Result<(u32, u32, Vec<u8>)> {
        if level >= level_count(width, height)? {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                format!("层级 {level} 超出金字塔"),
            ));
        }
        let mut current = (width, height, rgba.to_vec());
        for _ in 0..level {
            let (w, h, data) = downsample_2x(current.0, current.1, &current.2)?;
            if w == current.0 && h == current.1 {
                return Err(AppError::new(
                    "VALIDATION_ERROR",
                    format!("层级 {level} 超出金字塔"),
                ));
            }
            current = (w, h, data);
        }
        Ok(current)
    }

    /// 透明区域合成白底后保留四通道：与图库缓存同一条规矩。
    /// 直接丢掉 alpha 会让半透明像素变成黑色，画布上就是一块黑边。
    fn opaque_rgba(bitmap: &image::DynamicImage) -> image::RgbaImage {
        let rgba = bitmap.to_rgba8();
        image::RgbaImage::from_fn(rgba.width(), rgba.height(), |x, y| {
            let pixel = rgba.get_pixel(x, y);
            let alpha = pixel[3] as u32;
            image::Rgba(std::array::from_fn(|i| {
                ((pixel[i] as u32 * alpha + 255 * (255 - alpha)) / 255) as u8
            }))
        })
    }

    /// 只读文件头得到"看起来"的宽高：应用 EXIF 方向，但不解码任何像素。
    /// 门禁与整图解码同一条：超限或格式不认识就返回 None，让调用方退回原路径如实报错。
    #[cfg(target_os = "macos")]
    fn header_dimensions(original: &std::path::Path) -> Option<(u32, u32)> {
        use image::ImageDecoder;
        let mut reader = image::ImageReader::open(original)
            .ok()?
            .with_guessed_format()
            .ok()?;
        reader.limits(super::decode_limits());
        let mut decoder = reader.into_decoder().ok()?;
        let (width, height) = decoder.dimensions();
        let orientation = decoder.orientation().ok()?;
        let swapped = matches!(
            orientation,
            image::metadata::Orientation::Rotate90
                | image::metadata::Orientation::Rotate270
                | image::metadata::Orientation::Rotate90FlipH
                | image::metadata::Orientation::Rotate270FlipH
        );
        if width == 0 || height == 0 {
            return None;
        }
        Some(if swapped {
            (height, width)
        } else {
            (width, height)
        })
    }

    /// 大图的粗档位交给 ImageIO：边解码边缩到目标档位，只为这一档付像素，
    /// 不再把整幅全分辨率同时压在内存里。这条路已把像素显式映射到 sRGB，
    /// 于是声明也跟着写 sRGB，而不是照抄源配置——和图库缓存的缩放档同一条规矩。
    /// 尺寸与目标档位差一时按同一网格精确重采一次，保证瓦片坐标算得准。
    #[cfg(target_os = "macos")]
    fn scaled_level_image(
        original: &std::path::Path,
        width: u32,
        height: u32,
    ) -> Option<(image::RgbaImage, Option<Vec<u8>>)> {
        let extension = original.extension()?.to_str()?.to_ascii_lowercase();
        if !matches!(extension.as_str(), "jpg" | "jpeg" | "png" | "tif" | "tiff") {
            return None;
        }
        let bytes = std::fs::read(original).ok()?;
        let (w, h, rgba) =
            super::scaled_decode::decode_rgba_scaled(&bytes, width.max(height)).ok()?;
        let buffer = image::RgbaImage::from_raw(w, h, rgba)?;
        let bitmap = image::DynamicImage::ImageRgba8(super::unpremultiplied(buffer));
        let opaque = opaque_rgba(&bitmap);
        let pixels = if opaque.dimensions() == (width, height) {
            opaque
        } else {
            image::DynamicImage::ImageRgba8(opaque)
                .resize_exact(width, height, image::imageops::FilterType::Triangle)
                .into_rgba8()
        };
        Some((pixels, crate::color_engine::srgb_cache_profile_bytes()))
    }

    /// 取目标层级那一幅图：像素已按 EXIF 方向、透明已合成白底，附带这条路径真实的色彩声明。
    fn level_image(
        original: &std::path::Path,
        level: u8,
    ) -> Result<(image::RgbaImage, Option<Vec<u8>>)> {
        #[cfg(target_os = "macos")]
        if level > 0
            && let Some((source_w, source_h)) = header_dimensions(original)
            && source_w.max(source_h) > super::SCALED_DECODE_THRESHOLD
            && level < level_count(source_w, source_h)?
        {
            let (want_w, want_h) = level_dims(source_w, source_h, level);
            if let Some(image) = scaled_level_image(original, want_w, want_h) {
                return Ok(image);
            }
        }
        // 其余情形（小图、HEIC/RAW、非 macOS，以及缩放解码失败）走标准解码：
        // 应用方向、受同一套分配与像素上限约束，数值保留所以声明源自己的配置。
        let bitmap = super::decode(original)?;
        let rgba = opaque_rgba(&bitmap);
        let (width, height) = rgba.dimensions();
        let (level_w, level_h, pixels) = level_pixels(width, height, rgba.as_raw(), level)?;
        let image = image::RgbaImage::from_raw(level_w, level_h, pixels)
            .ok_or_else(|| AppError::new("DECODE", "瓦片缓冲尺寸不匹配"))?;
        Ok((
            image,
            crate::color_engine::source_profile_bytes_for_cache(original),
        ))
    }

    /// 解码原图、构建金字塔并提取 (level, x, y) 瓦片，编码为 JPEG 写入 target（tmp+rename 原子写）。
    /// 返回瓦片像素尺寸；瓦片文件本身即持久缓存。
    pub fn write_tile_jpeg(
        original: &std::path::Path,
        level: u8,
        x: u32,
        y: u32,
        target: &std::path::Path,
    ) -> Result<(u32, u32)> {
        let (level_image, profile) = level_image(original, level)?;
        let (level_w, level_h) = level_image.dimensions();
        let (tile_w, tile_h, tile) = tile_pixels(level_image.as_raw(), level_w, level_h, x, y)?;
        let buffer = image::RgbaImage::from_raw(tile_w, tile_h, tile)
            .ok_or_else(|| AppError::new("DECODE", "瓦片缓冲尺寸不匹配"))?;
        let rgb = image::DynamicImage::ImageRgba8(buffer).to_rgb8();
        // 声明按上面那条路实际做到的事写：缩放档写 sRGB，标准解码沿用源配置，没有依据就留白。
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(rgb)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Jpeg,
            )
            .map_err(|e| AppError::new("CACHE_WRITE", e.to_string()))?;
        if let Some(profile) = profile.as_deref() {
            bytes = crate::color_engine::tag_jpeg_with_icc(&bytes, profile)?;
        }
        let temporary = target.with_extension(format!("{}.tmp", crate::domain::id()));
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(&temporary, target)?;
        Ok((tile_w, tile_h))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::collections::HashSet;

        #[test]
        #[cfg(target_os = "macos")]
        fn tiles_declare_their_source_profile_instead_of_an_assumed_one() {
            let dir = tempfile::tempdir().unwrap();
            let icc = std::fs::read("/System/Library/ColorSync/Profiles/Display P3.icc")
                .unwrap_or_else(|error| panic!("缺少系统色彩配置 Display P3.icc: {error}"));
            let mut bytes = Vec::new();
            image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                1200,
                800,
                image::Rgb([200, 40, 60]),
            ))
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Jpeg,
            )
            .unwrap();

            let tagged = dir.path().join("tagged.jpg");
            std::fs::write(
                &tagged,
                crate::color_engine::tag_jpeg_with_icc(&bytes, &icc).unwrap(),
            )
            .unwrap();
            let tile = dir.path().join("tile.jpg");
            write_tile_jpeg(&tagged, 1, 0, 0, &tile).unwrap();
            assert_eq!(
                crate::color_engine::embedded_icc_bytes(&tile)
                    .unwrap()
                    .as_deref(),
                Some(icc.as_slice()),
                "瓦片像素沿用源数值，就该带上源自己的配置"
            );

            let plain = dir.path().join("plain.jpg");
            std::fs::write(&plain, &bytes).unwrap();
            let plain_tile = dir.path().join("plain-tile.jpg");
            write_tile_jpeg(&plain, 1, 0, 0, &plain_tile).unwrap();
            assert_eq!(
                crate::color_engine::embedded_icc_bytes(&plain_tile).unwrap(),
                None,
                "源没给依据时不替瓦片假定身份"
            );
        }

        /// 大图粗档位与全分辨率档走的是两条不同的解码路，声明必须跟着各自的路走。
        /// 这一条同时是"粗档不再整幅全分辨率解码"的唯一可核对证据：同一张源图，
        /// 层级 0 仍抄源配置、层级 1 换成系统 sRGB，只有真正走了缩放解码才会这样。
        #[test]
        #[cfg(target_os = "macos")]
        fn coarse_tiles_from_large_sources_declare_srgb_while_full_res_keeps_the_source() {
            let dir = tempfile::tempdir().unwrap();
            let icc = std::fs::read("/System/Library/ColorSync/Profiles/Display P3.icc")
                .unwrap_or_else(|error| panic!("缺少系统色彩配置 Display P3.icc: {error}"));
            let srgb =
                crate::color_engine::srgb_cache_profile_bytes().expect("系统 sRGB 配置应可用");
            let mut bytes = Vec::new();
            image::RgbImage::from_pixel(4000, 2500, image::Rgb([200, 40, 60]))
                .write_to(
                    &mut std::io::Cursor::new(&mut bytes),
                    image::ImageFormat::Jpeg,
                )
                .unwrap();
            let large = dir.path().join("large.jpg");
            std::fs::write(
                &large,
                crate::color_engine::tag_jpeg_with_icc(&bytes, &icc).unwrap(),
            )
            .unwrap();

            let full = dir.path().join("level0.jpg");
            let (w, h) = write_tile_jpeg(&large, 0, 0, 0, &full).unwrap();
            assert_eq!((w, h), (TILE, TILE));
            assert_eq!(
                crate::color_engine::embedded_icc_bytes(&full)
                    .unwrap()
                    .as_deref(),
                Some(icc.as_slice()),
                "层级 0 取的是原始像素，声明该跟着源走"
            );

            let coarse = dir.path().join("level1.jpg");
            let (w, h) = write_tile_jpeg(&large, 1, 5, 2, &coarse).unwrap();
            assert_eq!((w, h), (TILE, TILE));
            assert_eq!(
                crate::color_engine::embedded_icc_bytes(&coarse)
                    .unwrap()
                    .as_deref(),
                Some(srgb.as_slice()),
                "粗档位由 ImageIO 边解码边缩放到 sRGB，声明不能再写源配置"
            );
            let tile = image::open(&coarse).unwrap().to_rgb8();
            let pixel = tile.get_pixel(128, 128);
            assert!(
                pixel[0] > 120 && pixel[0] > pixel[1] && pixel[0] > pixel[2],
                "缩放过的那一档也得是同一块红色，实际 {pixel:?}"
            );

            // 小图不进这条路：源没超过缩放门槛，声明仍按源配置，与上一测试同一条口径。
            let small = dir.path().join("small.jpg");
            let mut small_bytes = Vec::new();
            image::RgbImage::from_pixel(2000, 1250, image::Rgb([200, 40, 60]))
                .write_to(
                    &mut std::io::Cursor::new(&mut small_bytes),
                    image::ImageFormat::Jpeg,
                )
                .unwrap();
            std::fs::write(
                &small,
                crate::color_engine::tag_jpeg_with_icc(&small_bytes, &icc).unwrap(),
            )
            .unwrap();
            let small_coarse = dir.path().join("small-level1.jpg");
            write_tile_jpeg(&small, 1, 3, 1, &small_coarse).unwrap();
            assert_eq!(
                crate::color_engine::embedded_icc_bytes(&small_coarse)
                    .unwrap()
                    .as_deref(),
                Some(icc.as_slice()),
                "没超过缩放门槛的源不能谎称自己被映射到了 sRGB"
            );
        }

        #[test]
        fn write_tile_jpeg_produces_cacheable_files() {
            let dir = tempfile::tempdir().unwrap();
            let original = dir.path().join("src.png");
            image::RgbImage::from_pixel(1200, 800, image::Rgb([120, 140, 160]))
                .save_with_format(&original, image::ImageFormat::Png)
                .unwrap();
            let target = dir.path().join("tile.jpg");
            // 1200×800 → level 1 = 600×400；瓦片 (2,1) 起点 (512,256)，尺寸 88×144。
            let (w, h) = write_tile_jpeg(&original, 1, 2, 1, &target).unwrap();
            assert_eq!((w, h), (88, 144));
            let decoded = image::open(&target).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (88, 144));
            // 幂等：同目标重复写入成功。
            write_tile_jpeg(&original, 1, 2, 1, &target).unwrap();
            // 层级与坐标越界拒绝。
            assert!(write_tile_jpeg(&original, 9, 0, 0, &dir.path().join("a.jpg")).is_err());
            assert!(write_tile_jpeg(&original, 1, 99, 0, &dir.path().join("b.jpg")).is_err());
        }

        fn solid(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
            pixel
                .iter()
                .copied()
                .cycle()
                .take(width as usize * height as usize * 4)
                .collect()
        }

        /// 把 EXIF 方向标签写进 JPEG 的 APP1 段（orientation 6 = 显示时顺时针转 90°）。
        fn with_orientation(jpeg: &[u8], code: u8) -> Vec<u8> {
            let mut entry = vec![0x12, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00];
            entry.extend_from_slice(&[code, 0, 0, 0]);
            let mut tiff = vec![b'I', b'I', 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00];
            tiff.extend_from_slice(&entry);
            tiff.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
            let mut exif = b"Exif\0\0".to_vec();
            exif.extend_from_slice(&tiff);
            let mut oriented = jpeg[..2].to_vec();
            oriented.extend_from_slice(&[0xff, 0xe1]);
            oriented.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
            oriented.extend_from_slice(&exif);
            oriented.extend_from_slice(&jpeg[2..]);
            oriented
        }

        #[test]
        fn tiles_follow_the_exif_orientation_instead_of_the_stored_dimensions() {
            let dir = tempfile::tempdir().unwrap();
            // 存储态 600×400：左半红、右半蓝。方向 6 意味着真实看到的是 400×600，红上蓝下。
            let mut bytes = Vec::new();
            image::RgbImage::from_fn(600, 400, |x, _| {
                if x < 300 {
                    image::Rgb([220, 30, 30])
                } else {
                    image::Rgb([30, 30, 220])
                }
            })
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
            let rotated = dir.path().join("rotated.jpg");
            std::fs::write(&rotated, with_orientation(&bytes, 6)).unwrap();

            // 层级 1 按显示方向是 200×300，边缘瓦片 200×256；没转方向会算成 300×200 的 256×200。
            let target = dir.path().join("tile.jpg");
            let (w, h) = write_tile_jpeg(&rotated, 1, 0, 0, &target).unwrap();
            assert_eq!((w, h), (200, 256));
            let tile = image::open(&target).unwrap().to_rgb8();
            let top = tile.get_pixel(100, 10);
            let bottom = tile.get_pixel(100, 250);
            assert!(
                top[0] > 180 && top[2] < 90,
                "顶部应是被转过来的红色，实际 {top:?}"
            );
            assert!(
                bottom[2] > 180 && bottom[0] < 90,
                "底部应是被转过来的蓝色，实际 {bottom:?}"
            );

            // 横向不再容得下第 3 列瓦片：越界如实报错，而不是画出一块空白。
            assert!(write_tile_jpeg(&rotated, 0, 2, 0, &dir.path().join("oob.jpg")).is_err());
            // 同一份像素不带方向时第 3 列存在，证明报错来自方向而不是尺寸门禁。
            let plain = dir.path().join("plain.jpg");
            std::fs::write(&plain, &bytes).unwrap();
            assert_eq!(
                write_tile_jpeg(&plain, 0, 2, 0, &dir.path().join("oob.jpg")).unwrap(),
                (88, 256)
            );
        }

        /// 缩放解码与 EXIF 方向是两条新路，交叉处最容易悄悄错：ImageIO 缩略图已按方向转正，
        /// 目标档位却是按显示方向算出来的格子。这一条把两者钉在一起。
        #[test]
        #[cfg(target_os = "macos")]
        fn coarse_tiles_of_large_rotated_sources_use_the_oriented_grid() {
            let dir = tempfile::tempdir().unwrap();
            // 存储态 2500×4000（竖幅）：左半红、右半蓝；方向 6 意味着看到的是 4000×2500，红上蓝下。
            let mut bytes = Vec::new();
            image::RgbImage::from_fn(2500, 4000, |x, _| {
                if x < 1250 {
                    image::Rgb([220, 30, 30])
                } else {
                    image::Rgb([30, 30, 220])
                }
            })
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
            let rotated = dir.path().join("rotated-large.jpg");
            std::fs::write(&rotated, with_orientation(&bytes, 6)).unwrap();

            // 层级 1 按显示方向是 2000×1250，有 8 列；没转方向会算成 1250×2000 的 5 列，第 7 列直接越界。
            let target = dir.path().join("tile.jpg");
            let (w, h) = write_tile_jpeg(&rotated, 1, 6, 2, &target).unwrap();
            assert_eq!((w, h), (TILE, TILE));
            let tile = image::open(&target).unwrap().to_rgb8();
            let top = tile.get_pixel(128, 20);
            let bottom = tile.get_pixel(128, 230);
            assert!(
                top[0] > 150 && top[2] < 90,
                "顶部应是转过来的红色，实际 {top:?}"
            );
            assert!(
                bottom[2] > 150 && bottom[0] < 90,
                "底部应是转过来的蓝色，实际 {bottom:?}"
            );
            // 这一档确实走了缩放解码，而不是退回整图：声明应落在 sRGB。
            let srgb =
                crate::color_engine::srgb_cache_profile_bytes().expect("系统 sRGB 配置应可用");
            assert_eq!(
                crate::color_engine::embedded_icc_bytes(&target)
                    .unwrap()
                    .as_deref(),
                Some(srgb.as_slice()),
                "大图粗档位的瓦片应声明自己已被映射到 sRGB"
            );
        }

        #[test]
        fn tiles_composite_transparency_on_white_instead_of_black() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("alpha.png");
            // 左半完全透明的红，右半 50% 透明的红：直接丢 alpha 会一块刺红、一块死黑。
            image::RgbaImage::from_fn(600, 400, |x, _| {
                if x < 300 {
                    image::Rgba([255, 0, 0, 0])
                } else {
                    image::Rgba([255, 0, 0, 128])
                }
            })
            .save(&path)
            .unwrap();
            // 瓦片 (1,0) 横跨 x=300 的分界：两块区域都取样在同一张 JPEG 里。
            let target = dir.path().join("tile.jpg");
            let (w, h) = write_tile_jpeg(&path, 0, 1, 0, &target).unwrap();
            assert_eq!((w, h), (256, 256));
            let tile = image::open(&target).unwrap().to_rgb8();
            let clear = tile.get_pixel(14, 10);
            assert!(
                clear[0] > 235 && clear[1] > 200 && clear[2] > 200,
                "透明像素应合成白底，实际 {clear:?}"
            );
            let half = tile.get_pixel(200, 10);
            assert!(
                half[0] > 235 && (100..175).contains(&half[1]) && (100..175).contains(&half[2]),
                "半透明像素应合成出淡红，实际 {half:?}"
            );
        }

        #[test]
        fn level_pixels_matches_the_full_pyramid_and_reports_the_end_of_it() {
            let (w, h) = (1000u32, 800u32);
            let rgba = solid(w, h, [9, 8, 7, 255]);
            let levels = build_levels(w, h, &rgba).unwrap();
            for (level, (bw, bh, data)) in levels.iter().enumerate() {
                let (lw, lh, pixels) = level_pixels(w, h, &rgba, level as u8).unwrap();
                assert_eq!((lw, lh), (*bw, *bh));
                assert_eq!(pixels, *data);
            }
            assert_eq!(
                level_pixels(w, h, &rgba, levels.len() as u8)
                    .unwrap_err()
                    .code,
                "VALIDATION_ERROR"
            );
            // 有一边已经缩到 1 时不返回上一层的重复数据，如实说这级不存在。
            let flat = solid(512, 1, [1, 2, 3, 255]);
            let (lw, lh, pixels) = level_pixels(512, 1, &flat, 1).unwrap();
            assert_eq!((lw, lh), (256, 1));
            assert_eq!(pixels, solid(256, 1, [1, 2, 3, 255]));
            assert_eq!(
                level_pixels(1, 1, &solid(1, 1, [1, 2, 3, 255]), 1)
                    .unwrap_err()
                    .code,
                "VALIDATION_ERROR"
            );
        }

        #[test]
        fn tile_file_names_are_recognized_only_for_this_apps_own_shapes() {
            let sha = "c".repeat(64);
            let built = format!("v2.{sha}.3.2.1.jpg");
            assert_eq!(
                crate::image_pipeline::tile_parts(&built),
                Some((2, sha.as_str()))
            );
            // 构建中途留下的临时件是同一路径换成 `.{随机号}.tmp`，也要认得才能回收。
            let temp = format!("v2.{sha}.3.2.1.0123456789abcdef.tmp");
            assert_eq!(
                crate::image_pipeline::tile_parts(&temp),
                Some((2, sha.as_str()))
            );
            for foreign in [
                "v2.not-a-sha.0.0.jpg",
                "notes.txt",
                "v2.0.0.jpg",
                "version.1.2.3.jpg",
            ] {
                assert_eq!(
                    crate::image_pipeline::tile_parts(foreign),
                    None,
                    "不该认领 {foreign}"
                );
            }
        }

        #[test]
        fn tile_files_are_counted_and_reclaimed_by_the_cache_quota_sweep() {
            let dir = tempfile::tempdir().unwrap();
            let cache = dir.path().join("cache");
            std::fs::create_dir(&cache).unwrap();
            let live = "a".repeat(64);
            let gone = "b".repeat(64);
            let name = |version: u8, sha: &str| format!("v{version}.{sha}.0.0.0.jpg");
            let current_live = cache.join(name(PYRAMID_VERSION, &live));
            let current_orphan = cache.join(name(PYRAMID_VERSION, &gone));
            let stale_version = cache.join(name(PYRAMID_VERSION - 1, &live));
            let stranger = cache.join("v2.nope.0.0.jpg");
            for path in [&current_live, &current_orphan, &stale_version, &stranger] {
                std::fs::write(path, vec![7u8; 100]).unwrap();
            }
            // 版本过期那份是确定再也命中不了的垃圾：额度宽松时也要清掉，其余三份都不动。
            assert_eq!(
                crate::image_pipeline::sweep_orphans(&cache, std::slice::from_ref(&live), 10_000)
                    .unwrap(),
                100
            );
            assert!(current_live.is_file(), "在库照片的当前版瓦片不能动");
            assert!(current_orphan.is_file(), "额度没满时孤儿也可以留着");
            assert!(stranger.is_file(), "不是本应用命名的文件一律不动");
            assert!(!stale_version.is_file(), "旧版算法留下的瓦片永远不会再命中");
            // 额度只够一张在库照片的瓦片：照片已删的那份这时才走。
            assert_eq!(
                crate::image_pipeline::sweep_orphans(&cache, std::slice::from_ref(&live), 100)
                    .unwrap(),
                100
            );
            assert!(!current_orphan.is_file(), "照片已删的瓦片要能回收");
            // 再次宽松时不再动任何东西：同一片瓦片不会被清两次。
            assert_eq!(
                crate::image_pipeline::sweep_orphans(&cache, &[live], 10_000).unwrap(),
                0
            );
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
            // 奇数尺寸：最外那一列/行只剩半个块，按实际存在的像素平均，不再被整条丢掉。
            let odd_row = vec![0, 0, 0, 255, 100, 100, 100, 255, 200, 200, 200, 255];
            let (w, h, out) = downsample_2x(3, 1, &odd_row).unwrap();
            assert_eq!((w, h), (2, 1));
            assert_eq!(out, vec![50, 50, 50, 255, 200, 200, 200, 255]);
            let padded = solid(3, 3, [10, 20, 30, 255]);
            let (w, h, out) = downsample_2x(3, 3, &padded).unwrap();
            assert_eq!((w, h), (2, 2));
            assert_eq!(out, solid(2, 2, [10, 20, 30, 255]));
            assert!(downsample_2x(2, 2, &[0u8; 7]).is_err());
        }

        #[test]
        fn level_grid_matches_the_rounding_the_canvas_uses() {
            // 前端算格子用 Math.ceil 逐级折半；两边口径不一致时最外那一列会要不存在的瓦片。
            assert_eq!(level_dims(999, 801, 0), (999, 801));
            assert_eq!(level_dims(999, 801, 1), (500, 401));
            assert_eq!(level_dims(999, 801, 2), (250, 201));
            assert_eq!(level_dims(513, 513, 1), (257, 257));
            // 闭式算法（除以 2^level 向上进位）与逐级降采样必须给出同一个尺寸。
            let rgba = solid(999, 801, [4, 5, 6, 255]);
            for (level, (w, h, data)) in build_levels(999, 801, &rgba).unwrap().iter().enumerate() {
                assert_eq!(*w, level_dims(999, 801, level as u8).0);
                assert_eq!(*h, level_dims(999, 801, level as u8).1);
                // 前端按 ceil(边长/256) 算行列数，最后一格必须非空，否则整节点退回糊图。
                let (tw, th, _) =
                    tile_pixels(data, *w, *h, w.div_ceil(TILE) - 1, h.div_ceil(TILE) - 1)
                        .expect("最外一格瓦片必须存在");
                assert_eq!(tw, w - (w.div_ceil(TILE) - 1) * TILE);
                assert_eq!(th, h - (h.div_ceil(TILE) - 1) * TILE);
            }
        }

        #[test]
        fn the_outermost_tile_holds_the_pixels_the_old_grid_dropped() {
            // 513×1 在这一级有 257 列：旧口径算成 256 列，多出来的那一列像素从来没上过屏。
            let rgba = solid(513, 513, [0, 0, 0, 255]);
            let mut level1 = level_pixels(513, 513, &rgba, 1).unwrap();
            assert_eq!((level1.0, level1.1), (257, 257));
            // 把这一级最后一列涂成红色，取最外那一格瓦片应当读到红，而不是报坐标越界。
            for y in 0..257 {
                let base = ((y * 257 + 256) as usize) * 4;
                level1.2[base] = 255;
                level1.2[base + 1] = 0;
                level1.2[base + 2] = 0;
            }
            let (tw, th, tile) = tile_pixels(&level1.2, level1.0, level1.1, 1, 0).unwrap();
            assert_eq!((tw, th), (1, 256));
            assert_eq!(tile[0], 255);
            assert_eq!(tile[(255 * tw as usize) * 4], 255);
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
            // 版本号写死在这里：每次上调都要改这一行，顺带提醒旧瓦片要靠它被回收掉。
            assert_eq!(key.cache_string(), format!("v3.{}.0.1.2", "a".repeat(64)));
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
/// 输出为预乘 alpha 的 RGBA8，像素已显式映射到 sRGB（不再用设备 RGB，避免缓存身份随显示器漂移）。
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
        // 目标空间用显式 sRGB，不用设备 RGB：设备 RGB 等于"当时接的那块主屏"，
        // 缓存像素会跟着显示器漂移却没有标注，换屏后重导入就连颜色都对不上。
        // 拿不到 sRGB 时直接失败，让调用方退回保留源数值的标准解码，不猜空间。
        let name = CFString::new("kCGColorSpaceSRGB");
        let Some(space) = CGColorSpace::create_with_name(name.as_concrete_TypeRef()) else {
            return Err(AppError::new(
                "COLOR_SRGB_UNAVAILABLE",
                "系统未提供 sRGB 色彩空间，缩放解码不降级到设备空间",
            ));
        };
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

/// 一张照片要修哪两份派生文件。路径与指纹都取自库里这条记录，
/// 调用方只能点名照片，不能指定要读哪个文件。
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewJob {
    pub photo_id: String,
    pub filename: String,
    pub original_path: String,
    pub content_sha256: String,
    pub thumbnail_path: String,
    pub preview_path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRepair {
    pub photo_id: String,
    pub filename: String,
    /// `repaired` 已重新算出；`healthy` 本来就是好的，一个字节都没动；
    /// 其余取值与视觉观察回执里的原因码同一套词表，界面无需再翻译一遍。
    pub outcome: String,
    /// 真正重新写出来的档位（`thumbnail` / `preview`），其余情形为空。
    pub rebuilt: Vec<String>,
    /// 一句实话：为什么是这个结局，下一步该做什么。
    pub detail: String,
}

/// 缓存里那份派生文件是不是真的还能用：只读文件头，不为判断把整张图解出来。
fn derived_usable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
        && image::image_dimensions(path).is_ok()
}

/// 一份体检结论：`outcome` 沿用视觉观察回执那套原因码，另加 `needs_repair` 表示"该重算，还没动手"。
struct Diagnosis {
    outcome: String,
    /// 该重算的档位与它们现在落在哪；`needs_repair` 之外为空。
    stale: Vec<(&'static str, PathBuf)>,
    detail: String,
}

/// 单张照片的两份派生文件到底还能不能用。原图只读：最多算一次原图指纹，从不写盘。
///
/// `check_original` 关掉时省掉那次指纹比对——整项目点名要把原图从头读一遍的开销去掉。
/// 所以清点只会少报、不会多报：外部改过原图的那几张要等到真动手重算时才被逐张拦下。
fn diagnose_preview_job(cache: &Path, job: &PreviewJob, check_original: bool) -> Diagnosis {
    let plain = |outcome: &str, detail: String| Diagnosis {
        outcome: outcome.to_owned(),
        stale: vec![],
        detail,
    };
    let original = PathBuf::from(&job.original_path);
    if !original.is_file() {
        return plain(
            "original_missing",
            "原图不在这台机器上，先重新定位这张照片才能重算".to_owned(),
        );
    }
    // 动手前的硬门：原图当前内容必须还是库里记的那一张。不是的话，
    // 重算出来的文件会用旧指纹命名，等于把另一张照片的画面挂到这张的名字上。
    if check_original {
        match fingerprint(&original) {
            Ok(hash) if hash != job.content_sha256 => {
                return plain(
                    "original_changed",
                    "原图内容已经和库里记录的不一致：这里不替它决定，要在照片上确认\"内容更新\"另走一步".to_owned(),
                );
            }
            Ok(_) => {}
            Err(error) => {
                return plain(
                    "original_unreadable",
                    format!("原图现在也读不出来：{}", error.message),
                );
            }
        }
    }
    // 只认本应用按内容指纹命名的那两份缓存，目录和文件名都对得上才动手。
    // 目录比对走规范路径：macOS 上 /var 与 /private/var 是同一个地方，
    // 按字符串比会把该修的每一张都误判成"位置可疑"。
    let cache_dir = match std::fs::canonicalize(cache) {
        Ok(dir) if dir.is_dir() => dir,
        _ => {
            return plain(
                "cache_path_suspicious",
                format!("缓存目录 {} 现在打不开，先不覆盖", cache.display()),
            );
        }
    };
    let mut stale: Vec<(&'static str, PathBuf)> = vec![];
    for (tier, suffix, recorded) in [
        ("thumbnail", "-320.jpg", job.thumbnail_path.as_str()),
        ("preview", "-1400.jpg", job.preview_path.as_str()),
    ] {
        let path = PathBuf::from(recorded);
        let expected = format!("{}{}", job.content_sha256, suffix);
        let in_cache = path.file_name().and_then(|name| name.to_str()) == Some(expected.as_str())
            && path
                .parent()
                .and_then(|parent| std::fs::canonicalize(parent).ok())
                .is_some_and(|parent| parent == cache_dir);
        if !in_cache {
            return plain(
                "cache_path_suspicious",
                "库里记的缓存位置不是本应用按这份指纹命名的文件，先不覆盖".to_owned(),
            );
        }
        if !derived_usable(&path) {
            stale.push((tier, path));
        }
    }
    if stale.is_empty() {
        return plain("healthy", "这两份派生文件本来就能读，没有改写".to_owned());
    }
    Diagnosis {
        outcome: "needs_repair".to_owned(),
        stale,
        detail: "有一档派生文件现在读不出来，只读原图就能重算".to_owned(),
    }
}

/// 图库里那两份中间文件坏掉时的修复动作（视觉观察只会说"这张没送出"，这里把它修好）。
///
/// 原图只读：这条路径只写图库自己生成的缓存，且只写按这张照片的内容指纹命名的那两份；
/// 本来就是好的文件不动，色彩标注也就不会被白白重写一遍。
fn repair_one_preview(cache: &Path, job: &PreviewJob) -> PreviewRepair {
    let report = |outcome: String, rebuilt: Vec<String>, detail: String| PreviewRepair {
        photo_id: job.photo_id.clone(),
        filename: job.filename.clone(),
        outcome,
        rebuilt,
        detail,
    };
    let diagnosis = diagnose_preview_job(cache, job, true);
    if diagnosis.outcome != "needs_repair" {
        return report(diagnosis.outcome, vec![], diagnosis.detail);
    }
    // 删文件之前再确认一次目录还打得开：点名时能过，动手时也可能已经被挪走。
    let cache_dir = match std::fs::canonicalize(cache) {
        Ok(dir) if dir.is_dir() => dir,
        _ => {
            return report(
                "cache_path_suspicious".to_owned(),
                vec![],
                "缓存目录现在打不开，先不覆盖".to_owned(),
            );
        }
    };
    // 删掉坏文件之后交给生成派生缓存的同一条工序：文件名由内容指纹决定，
    // 路径与库里记录的一致，所以数据库不用改一行。
    let rebuilt: Vec<String> = diagnosis
        .stale
        .iter()
        .map(|(tier, _)| (*tier).to_owned())
        .collect();
    for (_, path) in &diagnosis.stale {
        let _ = std::fs::remove_file(path);
    }
    match prepare(&PathBuf::from(&job.original_path), &cache_dir) {
        Ok(_) => report(
            "repaired".to_owned(),
            rebuilt,
            "已只读原图重新算出派生文件".to_owned(),
        ),
        Err(error) if error.code == "FILE_CHANGED" => report(
            "original_changed".to_owned(),
            vec![],
            "重算期间原图又变了，留下的文件不作数".to_owned(),
        ),
        Err(error) => report(
            "original_unreadable".to_owned(),
            vec![],
            format!("原图重算失败：{}", error.message),
        ),
    }
}

/// 批量修复派生预览。整批都在阻塞线程里跑：每张要解一次原图。
pub fn repair_preview_jobs(cache: &Path, jobs: &[PreviewJob]) -> Vec<PreviewRepair> {
    jobs.iter()
        .map(|job| repair_one_preview(cache, job))
        .collect()
}

/// 只读体检的结论转成回执：`rebuilt` 一律为空——这条路一个字节都没写，
/// 不能把"该重算哪一档"说成"已经重算了哪一档"。
fn diagnosis_report(job: &PreviewJob, diagnosis: Diagnosis) -> PreviewRepair {
    PreviewRepair {
        photo_id: job.photo_id.clone(),
        filename: job.filename.clone(),
        outcome: diagnosis.outcome,
        rebuilt: vec![],
        detail: diagnosis.detail,
    }
}

/// 单张只读体检：与真动手重算前那道门完全同一套判断（含原图内容比对），但从不写盘。
/// 检视器先用它说清"这张现在能不能读"，只有 `needs_repair` 才值得摆重算按钮。
pub fn inspect_preview_job(cache: &Path, job: &PreviewJob) -> PreviewRepair {
    diagnosis_report(job, diagnose_preview_job(cache, job, true))
}

/// 批量点名，一个字节都不改：`outcome` 为 `needs_repair` 的就是该重算的那几张。
///
/// 这里不比对原图内容指纹，所以外部改过原图的那几张会被算成"能用"；真去重算时那道门逐张把关，
/// 两边的分工写在同一处，不会互相冒充。
pub fn audit_preview_jobs(cache: &Path, jobs: &[PreviewJob]) -> Vec<PreviewRepair> {
    jobs.iter()
        .map(|job| diagnosis_report(job, diagnose_preview_job(cache, job, false)))
        .collect()
}

#[cfg(test)]
mod preview_repair_tests {
    use super::{PreviewJob, prepare, repair_preview_jobs};
    use std::path::{Path, PathBuf};

    fn prepared_job() -> (tempfile::TempDir, PreviewJob) {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("cache");
        let original = temp.path().join("a.jpg");
        image::RgbImage::from_pixel(64, 48, image::Rgb([12, 200, 40]))
            .save(&original)
            .unwrap();
        let prepared = prepare(&original, &cache).unwrap();
        let job = PreviewJob {
            photo_id: "p1".to_owned(),
            filename: prepared.filename.clone(),
            original_path: prepared.path.to_string_lossy().into_owned(),
            content_sha256: prepared.hash.clone(),
            thumbnail_path: prepared.thumbnail.to_string_lossy().into_owned(),
            preview_path: prepared.preview.to_string_lossy().into_owned(),
        };
        (temp, job)
    }

    fn cache_of(job: &PreviewJob) -> PathBuf {
        Path::new(&job.preview_path).parent().unwrap().to_path_buf()
    }

    #[test]
    fn repair_rebuilds_only_the_corrupt_derived_file_and_never_touches_the_original() {
        let (_temp, job) = prepared_job();
        let original = PathBuf::from(&job.original_path);
        let thumbnail = PathBuf::from(&job.thumbnail_path);
        let preview = PathBuf::from(&job.preview_path);
        let original_bytes = std::fs::read(&original).unwrap();
        let thumbnail_bytes = std::fs::read(&thumbnail).unwrap();
        std::fs::write(&preview, b"not a jpeg at all").unwrap();

        let done = repair_preview_jobs(&cache_of(&job), std::slice::from_ref(&job));
        assert_eq!(done[0].outcome, "repaired", "{:?}", done[0]);
        assert_eq!(
            done[0].rebuilt,
            vec!["preview".to_owned()],
            "只重算坏的那一档"
        );
        assert_eq!(
            std::fs::read(&original).unwrap(),
            original_bytes,
            "原图只读"
        );
        assert_eq!(
            std::fs::read(&thumbnail).unwrap(),
            thumbnail_bytes,
            "好的那份不动"
        );
        assert_eq!(
            image::image_dimensions(&preview).unwrap(),
            (64, 48),
            "修好的预览要真的解得开"
        );

        // 再点一次：本来就是好的，一个字节都不该重写（重写会连色彩标注一起再过一遍）。
        let settled = std::fs::read(&preview).unwrap();
        let again = repair_preview_jobs(&cache_of(&job), &[job]);
        assert_eq!(again[0].outcome, "healthy");
        assert!(again[0].rebuilt.is_empty());
        assert_eq!(std::fs::read(&preview).unwrap(), settled);
    }

    #[test]
    fn repair_refuses_to_write_when_the_original_is_not_the_recorded_photo() {
        let (_temp, job) = prepared_job();
        let preview = PathBuf::from(&job.preview_path);
        std::fs::write(&preview, b"not a jpeg at all").unwrap();

        // 原图内容变了：这份新画面不能挂到旧指纹的名字上，所以一张文件都不写。
        image::RgbImage::from_pixel(64, 48, image::Rgb([1, 2, 3]))
            .save(Path::new(&job.original_path))
            .unwrap();
        let changed = repair_preview_jobs(&cache_of(&job), std::slice::from_ref(&job));
        assert_eq!(changed[0].outcome, "original_changed", "{:?}", changed[0]);
        assert!(changed[0].rebuilt.is_empty());
        assert_eq!(std::fs::read(&preview).unwrap(), b"not a jpeg at all");

        // 原图不在这台机器上：这一步修不了，要说清得先重新定位。
        std::fs::remove_file(Path::new(&job.original_path)).unwrap();
        let gone = repair_preview_jobs(&cache_of(&job), std::slice::from_ref(&job));
        assert_eq!(gone[0].outcome, "original_missing", "{:?}", gone[0]);

        // 库里记的缓存位置不是本应用按这份指纹命名的文件：一律不覆盖。
        let (_other, mut astray) = prepared_job();
        astray.preview_path = Path::new(&astray.preview_path)
            .parent()
            .unwrap()
            .join("someone-else.jpg")
            .to_string_lossy()
            .into_owned();
        let refused = repair_preview_jobs(&cache_of(&astray), &[astray]);
        assert_eq!(
            refused[0].outcome, "cache_path_suspicious",
            "{:?}",
            refused[0]
        );
    }

    #[test]
    fn inspect_says_the_same_thing_as_repair_without_writing_a_byte() {
        use super::{audit_preview_jobs, inspect_preview_job};
        let (_temp, job) = prepared_job();
        let cache = cache_of(&job);
        let original = PathBuf::from(&job.original_path);
        let thumbnail = PathBuf::from(&job.thumbnail_path);
        let preview = PathBuf::from(&job.preview_path);
        let thumbnail_bytes = std::fs::read(&thumbnail).unwrap();
        let healthy = inspect_preview_job(&cache, &job);
        assert_eq!(healthy.outcome, "healthy", "{healthy:?}");
        assert!(healthy.rebuilt.is_empty(), "只读体检不能说成已经重算过");

        std::fs::write(&preview, b"not a jpeg at all").unwrap();
        let broken = inspect_preview_job(&cache, &job);
        assert_eq!(broken.outcome, "needs_repair", "{broken:?}");
        assert!(broken.rebuilt.is_empty());
        assert_eq!(
            std::fs::read(&preview).unwrap(),
            b"not a jpeg at all",
            "体检不许顺手把文件改掉"
        );
        // 整项目清点走的是同一份结论，只是不比对原图内容；两边都不许声称重算过。
        let audited = audit_preview_jobs(&cache, std::slice::from_ref(&job));
        assert_eq!(audited[0].outcome, "needs_repair");
        assert!(audited[0].rebuilt.is_empty());

        // 体检用的门和动手时是同一道：原图换过、原图不在，都按重算时的说法回答。
        image::RgbImage::from_pixel(64, 48, image::Rgb([1, 2, 3]))
            .save(&original)
            .unwrap();
        assert_eq!(
            inspect_preview_job(&cache, &job).outcome,
            "original_changed"
        );
        std::fs::remove_file(&original).unwrap();
        assert_eq!(
            inspect_preview_job(&cache, &job).outcome,
            "original_missing"
        );
        assert_eq!(std::fs::read(&thumbnail).unwrap(), thumbnail_bytes);
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

    fn system_profile(name: &str) -> Vec<u8> {
        std::fs::read(std::path::Path::new("/System/Library/ColorSync/Profiles").join(name))
            .unwrap_or_else(|error| panic!("缺少系统色彩配置 {name}: {error}"))
    }

    /// 把色彩配置写进夹具：模拟相机/编辑软件产出的带配置原图。
    fn tag_fixture(path: &Path, icc: &[u8]) {
        let bytes = std::fs::read(path).unwrap();
        let tagged = crate::color_engine::tag_jpeg_with_icc(&bytes, icc).unwrap();
        std::fs::write(path, tagged).unwrap();
    }

    fn cache_icc(path: &Path) -> Option<Vec<u8>> {
        crate::color_engine::embedded_icc_bytes(path).unwrap()
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn scaled_cache_declares_srgb_instead_of_whatever_screen_was_attached() {
        let dir = tempfile::tempdir().unwrap();
        let p3 = system_profile("Display P3.icc");
        let bitmap = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            3600,
            2400,
            image::Rgb([210, 40, 60]),
        ));
        let path = write_fixture(dir.path(), "large-p3.jpg", bitmap);
        tag_fixture(&path, &p3);
        let prepared = prepare(&path, &dir.path().join("cache")).unwrap();
        let declared = cache_icc(&prepared.preview).expect("缩放缓存必须写明自己的空间");
        assert_eq!(
            declared,
            crate::color_engine::srgb_cache_profile_bytes().expect("系统 sRGB 配置"),
            "缩放解码显式映射到 sRGB，标注就得是这份 sRGB 配置"
        );
        assert_ne!(declared, p3, "不能把源配置抄给已经被映射过的像素");
        assert_eq!(
            cache_icc(&prepared.thumbnail).as_deref(),
            Some(declared.as_slice())
        );
    }

    #[test]
    fn standard_decode_cache_carries_the_source_profile_without_touching_values() {
        let dir = tempfile::tempdir().unwrap();
        let p3 = system_profile("Display P3.icc");
        // 800×600 在缩放阈值之下：像素数值沿用原图，缓存因此可以如实声明源配置。
        let bitmap = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            800,
            600,
            image::Rgb([210, 40, 60]),
        ));
        let path = write_fixture(dir.path(), "small-p3.jpg", bitmap);
        tag_fixture(&path, &p3);
        let prepared = prepare(&path, &dir.path().join("cache")).unwrap();
        assert_eq!(cache_icc(&prepared.preview).as_deref(), Some(p3.as_slice()));
        assert_eq!(
            cache_icc(&prepared.thumbnail).as_deref(),
            Some(p3.as_slice())
        );
        let preview = image::open(&prepared.preview).unwrap().to_rgb8();
        let center = preview.get_pixel(preview.width() / 2, preview.height() / 2);
        assert!(
            (center[0] as i32 - 210).abs() <= 4
                && (center[1] as i32 - 40).abs() <= 4
                && (center[2] as i32 - 60).abs() <= 4,
            "标注不该改像素，实际 {center:?}"
        );
    }

    #[test]
    fn sources_without_a_profile_leave_the_cache_unclaimed() {
        let dir = tempfile::tempdir().unwrap();
        let bitmap = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            800,
            600,
            image::Rgb([30, 160, 90]),
        ));
        let path = write_fixture(dir.path(), "plain.jpg", bitmap);
        assert_eq!(cache_icc(&path), None, "夹具本身不带配置，才能测这条边界");
        let prepared = prepare(&path, &dir.path().join("cache")).unwrap();
        assert_eq!(cache_icc(&prepared.preview), None, "没有依据就不声明");
        assert_eq!(cache_icc(&prepared.thumbnail), None);
    }

    #[test]
    fn tagging_a_cache_rewrites_only_the_declaration() {
        let dir = tempfile::tempdir().unwrap();
        let bitmap = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            64,
            48,
            image::Rgb([12, 200, 190]),
        ));
        let plain = dir.path().join("plain.jpg");
        let tagged = dir.path().join("tagged.jpg");
        save_cache(&bitmap, &plain, None).unwrap();
        save_cache(&bitmap, &tagged, Some(&system_profile("Display P3.icc"))).unwrap();
        assert_eq!(
            image::open(&plain).unwrap().to_rgb8(),
            image::open(&tagged).unwrap().to_rgb8()
        );
        // 写出走临时文件改名：目录里不该留下半截缓存。
        let leftovers = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "tmp"))
            .count();
        assert_eq!(leftovers, 0);
    }
}
