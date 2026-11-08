//! G2 Color Engine：ICC profile 读取、色彩管线的四路分离与资产域绑定。
//!
//! v3 目标：不再共享「8-bit 1400px JPEG」作为唯一像素来源。四条管线各有身份：
//! - thumbnail: 320px 快速浏览
//! - display_proxy: 1400px 屏幕检视（当前实现）
//! - ai_preview: 模型观察用（成本敏感，尺寸/质量可配）
//! - print_master: 印刷母版（高位深、ICC 保留、不可降格）
//!
//! 当前已有 RGB 8/16/32f 原生 CMM。显式检视从原图经 16-bit 变换生成带目标 ICC 的
//! 8-bit JPEG；图库缓存在写出时就声明自己的空间（缩放解码标注 sRGB，标准解码沿用源配置，
//! 平台解码不声明），上传给模型的像素统一落到 sRGB 并如实标注。瓦片按 EXIF 方向取图、
//! 透明合成白底并带上身份声明，大图粗档位已改走缩放解码（声明也随之落 sRGB），但全分辨率
//! 档与小图仍整图解码、且未做块级局部取像；印刷母版仍是待接通的路径。

use crate::domain::Result;
use std::path::Path;

/// 色彩管线输出目标。v3 蓝图 §05.1：四条管线不得共享低质中间图。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineTarget {
    /// 缩略图库：320px，允许有损压缩
    Thumbnail,
    /// 屏幕检视代理：当前 1400px JPEG（v3 将升级为原生瓦片）
    DisplayProxy,
    /// AI 观察代理：成本敏感，可进一步压缩
    AiPreview,
    /// 印刷母版：16-bit/float，保留源 ICC，禁止从代理放大
    PrintMaster,
}

impl PipelineTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Thumbnail => "thumbnail",
            Self::DisplayProxy => "display_proxy",
            Self::AiPreview => "ai_preview",
            Self::PrintMaster => "print_master",
        }
    }
}

/// 检测到的源文件 ICC profile 信息。
#[derive(Debug, Clone)]
pub struct SourceProfile {
    /// ICC profile 字节的 SHA-256 digest（用于缓存键和资产域 source_profile_digest）
    pub digest: String,
    /// 色彩空间描述（如 "Display P3"、"sRGB IEC61966-2.1"）
    pub description: String,
    /// profile 是否有效可解析
    pub valid: bool,
    /// H0-08 明确诊断：
    /// Absent = 文件无嵌入 profile（多数 sRGB 相机默认）
    /// Unsupported = 容器格式暂不支持提取（HEIF/BigTIFF 等）
    /// Invalid = 有嵌入数据但结构校验失败
    /// Valid = 结构安全可读（不代表色彩转换已实现）
    pub diagnostic: ProfileDiagnostic,
}

/// ICC profile 诊断结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileDiagnostic {
    Absent,
    Unsupported(&'static str),
    Invalid(String),
    Valid,
}

impl ProfileDiagnostic {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Unsupported(_) => "unsupported",
            Self::Invalid(_) => "invalid",
            Self::Valid => "valid",
        }
    }
}

/// 源文件按段读取；不把大型 TIFF/RAW 像素整体载入内存。
pub fn read_source_profile(path: &Path) -> Result<Option<SourceProfile>> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let file_len = file.metadata()?.len();
    let Some(icc) = extract_embedded_icc(&mut file, file_len) else {
        return Ok(None);
    };
    let valid = valid_icc_structure(&icc);
    let diagnostic = if valid {
        ProfileDiagnostic::Valid
    } else {
        ProfileDiagnostic::Invalid(format!("header {} bytes", icc.len()))
    };
    Ok(Some(SourceProfile {
        digest: format!("{:x}", Sha256::digest(&icc)),
        description: if valid {
            parse_icc_description(&icc)
        } else {
            "invalid".to_owned()
        },
        valid,
        diagnostic,
    }))
}

/// 检视渲染需要 ICC 字节本身（诊断接口只返回摘要）；按容器格式分段读取原始 ICC。
pub fn embedded_icc_bytes(path: &Path) -> Result<Option<Vec<u8>>> {
    let mut file = std::fs::File::open(path)?;
    let file_len = file.metadata()?.len();
    Ok(extract_embedded_icc(&mut file, file_len))
}

/// 缓存像素沿用源数值（标准解码）时，缓存如实声明源自己的配置。
/// 判定不了、或配置不是 RGB 时返回 None：宁可留一张没有声明的缓存，也不替它猜一个身份。
pub fn source_profile_bytes_for_cache(original: &Path) -> Option<Vec<u8>> {
    rgb_only(embedded_icc_bytes(original).ok().flatten())
}

/// 只有结构可读、且色彩空间标记确实是 RGB 的配置，才允许写进 RGB 像素的声明里。
fn rgb_only(icc: Option<Vec<u8>>) -> Option<Vec<u8>> {
    let icc = icc?;
    (valid_icc_structure(&icc) && icc.get(16..20) == Some(b"RGB ")).then_some(icc)
}

/// 缩放解码显式把像素映射到 sRGB 后，缓存要标注的就是这份 sRGB 配置。
#[cfg(target_os = "macos")]
pub fn srgb_cache_profile_bytes() -> Option<Vec<u8>> {
    native_transform::srgb_profile_bytes().ok()
}

#[cfg(not(target_os = "macos"))]
pub fn srgb_cache_profile_bytes() -> Option<Vec<u8>> {
    None
}

/// 内存里的一份图片字节 → 可核对的 RGB 源配置；容器读不懂、结构不完整或不是 RGB 都返回 None。
pub fn declared_rgb_profile(bytes: &[u8]) -> Option<Vec<u8>> {
    rgb_only(extract_embedded_icc(
        &mut std::io::Cursor::new(bytes),
        bytes.len() as u64,
    ))
}

/// 交给模型的像素必须有确定身份：无标签 JPEG 会被接收方当成 sRGB 解读，
/// 所以这里只做一件事——把有明确 RGB 声明的像素真正落到 sRGB，并如实标上 sRGB。
/// 说不清身份的（旧缓存、平台解码）保留数值、不补标签，也不假装已经变换过。
/// 返回值是给上传 JPEG 补的标签，None 表示这份上传件没有可声明的身份。
pub fn normalize_upload_pixels(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    source_icc: Option<&[u8]>,
) -> Result<Option<Vec<u8>>> {
    let Some(source_icc) = source_icc else {
        return Ok(None);
    };
    #[cfg(target_os = "macos")]
    {
        let target = srgb_cache_profile_bytes().ok_or_else(|| {
            crate::domain::AppError::new(
                "COLOR_SRGB_UNAVAILABLE",
                "系统未提供 sRGB 配置，上传像素不猜测色彩身份",
            )
        })?;
        if source_icc != target {
            native_transform::transform_rgb8(source_icc, &target, pixels, width, height)?;
        }
        Ok(Some(target))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (pixels, width, height);
        Ok(Some(source_icc.to_vec()))
    }
}

use std::io::{Read, Seek, SeekFrom};
const MAX_ICC_BYTES: usize = 32 * 1024 * 1024;
const MAX_METADATA_RECORDS: usize = 4096;

/// 每次分配和偏移运算均有界；读取失败或不完整的容器不会被当作有效 profile。
fn read_at<R: Read + Seek>(
    reader: &mut R,
    file_len: u64,
    offset: u64,
    len: usize,
) -> Option<Vec<u8>> {
    if len > MAX_ICC_BYTES || offset.checked_add(len as u64)? > file_len {
        return None;
    }
    reader.seek(SeekFrom::Start(offset)).ok()?;
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

fn be32(bytes: &[u8], offset: usize) -> Option<usize> {
    Some(u32::from_be_bytes(bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?) as usize)
}

#[derive(Default)]
struct ProfileScan {
    present: bool,
    complete: bool,
}

fn extract_embedded_icc<R: Read + Seek>(reader: &mut R, file_len: u64) -> Option<Vec<u8>> {
    extract_profile_scan(reader, file_len, &mut ProfileScan::default())
}

fn extract_profile_scan<R: Read + Seek>(
    reader: &mut R,
    file_len: u64,
    scan: &mut ProfileScan,
) -> Option<Vec<u8>> {
    let header = read_at(reader, file_len, 0, 8)?;
    if header.starts_with(&[0xff, 0xd8]) {
        extract_jpeg_icc(reader, file_len, scan)
    } else if header == b"\x89PNG\r\n\x1a\n" {
        extract_png_icc(reader, file_len, scan)
    } else if header.starts_with(b"II*\0") || header.starts_with(b"MM\0*") {
        extract_tiff_icc(reader, file_len, &header, scan)
    } else {
        // BigTIFF、HEIF 和相机 RAW 的 profile 需由其专用解码器处理。
        None
    }
}

fn extract_jpeg_icc<R: Read + Seek>(
    reader: &mut R,
    file_len: u64,
    scan: &mut ProfileScan,
) -> Option<Vec<u8>> {
    let mut position = 2;
    let mut total = None;
    let mut chunks = std::collections::BTreeMap::new();
    let mut collected = 0usize;
    for _ in 0..MAX_METADATA_RECORDS {
        let marker = read_at(reader, file_len, position, 2)?;
        if marker[0] != 0xff {
            return None;
        }
        if marker[1] == 0xff {
            position += 1;
            continue;
        }
        if matches!(marker[1], 0xda | 0xd9) {
            scan.complete = true;
            break;
        }
        if matches!(marker[1], 0x01 | 0xd0..=0xd7) {
            position += 2;
            continue;
        }
        let length = read_at(reader, file_len, position + 2, 2)?;
        let length = u16::from_be_bytes(length.try_into().ok()?) as usize;
        if length < 2 {
            return None;
        }
        let end = position.checked_add(2 + length as u64)?;
        if end > file_len {
            return None;
        }
        if marker[1] == 0xe2 {
            let data = read_at(reader, file_len, position + 4, length - 2)?;
            if data.starts_with(b"ICC_PROFILE\0") {
                scan.present = true;
                if data.len() < 14 {
                    return None;
                }
                let (sequence, count) = (data[12], data[13]);
                if sequence == 0
                    || count == 0
                    || sequence > count
                    || total.is_some_and(|n| n != count)
                {
                    return None;
                }
                total = Some(count);
                collected = collected.checked_add(data.len() - 14)?;
                if collected > MAX_ICC_BYTES
                    || chunks.insert(sequence, data[14..].to_vec()).is_some()
                {
                    return None;
                }
            }
        }
        position = end;
    }
    if !scan.complete {
        return None;
    }
    let total = total?;
    if chunks.len() != total as usize || collected == 0 {
        return None;
    }
    let mut icc = Vec::with_capacity(collected);
    for index in 1..=total {
        icc.extend_from_slice(chunks.get(&index)?);
    }
    Some(icc)
}

fn extract_png_icc<R: Read + Seek>(
    reader: &mut R,
    file_len: u64,
    scan: &mut ProfileScan,
) -> Option<Vec<u8>> {
    let mut position = 8u64;
    for _ in 0..MAX_METADATA_RECORDS {
        let header = read_at(reader, file_len, position, 8)?;
        let length = be32(&header, 0)?;
        let end = position.checked_add(12 + length as u64)?;
        if end > file_len {
            return None;
        }
        if &header[4..8] == b"iCCP" {
            scan.present = true;
            let data = read_at(reader, file_len, position + 8, length)?;
            let name_end = data.iter().position(|byte| *byte == 0)?;
            if !(1..=79).contains(&name_end) || *data.get(name_end + 1)? != 0 {
                return None;
            }
            let compressed = data.get(name_end + 2..)?;
            let mut decoder =
                flate2::read::ZlibDecoder::new(compressed).take((MAX_ICC_BYTES + 1) as u64);
            let mut icc = Vec::new();
            decoder.read_to_end(&mut icc).ok()?;
            return (!icc.is_empty() && icc.len() <= MAX_ICC_BYTES).then_some(icc);
        }
        // iCCP 应在像素数据之前；不读取、解压 IDAT。
        if matches!(&header[4..8], b"IDAT" | b"IEND") {
            scan.complete = true;
            return None;
        }
        position = end;
    }
    None
}

fn extract_tiff_icc<R: Read + Seek>(
    reader: &mut R,
    file_len: u64,
    header: &[u8],
    scan: &mut ProfileScan,
) -> Option<Vec<u8>> {
    let little = header.starts_with(b"II");
    let u16_at = |data: &[u8], offset: usize| -> Option<u16> {
        let raw = data.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
        Some(if little {
            u16::from_le_bytes(raw)
        } else {
            u16::from_be_bytes(raw)
        })
    };
    let u32_at = |data: &[u8], offset: usize| -> Option<u32> {
        let raw = data.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(raw)
        } else {
            u32::from_be_bytes(raw)
        })
    };
    let ifd = u32_at(header, 4)? as u64;
    if ifd < 8 {
        return None;
    }
    let count_bytes = read_at(reader, file_len, ifd, 2)?;
    let count = u16_at(&count_bytes, 0)? as usize;
    if count > MAX_METADATA_RECORDS {
        return None;
    }
    let entries = read_at(
        reader,
        file_len,
        ifd + 2,
        count.checked_mul(12)?.checked_add(4)?,
    )?;
    for entry in entries[..count * 12].chunks_exact(12) {
        if u16_at(entry, 0)? == 34675 {
            scan.present = true;
            // TIFF/EP ICCProfile 为 UNDEFINED 字节数组，不在像素数据中搜索 acsp。
            if !matches!(u16_at(entry, 2)?, 1 | 7) {
                return None;
            }
            let size = u32_at(entry, 4)? as usize;
            if !(132..=MAX_ICC_BYTES).contains(&size) {
                return None;
            }
            return read_at(reader, file_len, u32_at(entry, 8)? as u64, size);
        }
    }
    scan.complete = true;
    None
}

/// 只证明 profile 结构可安全读取；不等同于原生 CMM 的色彩转换或印刷合规验证。
fn valid_icc_structure(icc: &[u8]) -> bool {
    if !(132..=MAX_ICC_BYTES).contains(&icc.len())
        || icc.get(36..40) != Some(b"acsp")
        || be32(icc, 0) != Some(icc.len())
    {
        return false;
    }
    let Some(count) = be32(icc, 128) else {
        return false;
    };
    let Some(table_end) = count.checked_mul(12).and_then(|n| n.checked_add(132)) else {
        return false;
    };
    if count > MAX_METADATA_RECORDS || table_end > icc.len() {
        return false;
    }
    for tag in icc[132..table_end].chunks_exact(12) {
        let (Some(offset), Some(size)) = (be32(tag, 4), be32(tag, 8)) else {
            return false;
        };
        if offset < table_end
            || size < 8
            || offset.checked_add(size).is_none_or(|end| end > icc.len())
        {
            return false;
        }
    }
    true
}

fn parse_icc_description(icc: &[u8]) -> String {
    if !valid_icc_structure(icc) {
        return "invalid".to_owned();
    }
    let count = be32(icc, 128).unwrap_or(0);
    for entry in icc[132..132 + count * 12].chunks_exact(12) {
        if &entry[..4] != b"desc" {
            continue;
        }
        let (Some(offset), Some(size)) = (be32(entry, 4), be32(entry, 8)) else {
            continue;
        };
        let Some(data) = icc.get(offset..offset + size) else {
            continue;
        };
        if data.starts_with(b"desc") {
            let Some(length) = be32(data, 8) else {
                continue;
            };
            if length == 0 || length > 4096 {
                continue;
            }
            let Some(text) = data.get(12..12 + length) else {
                continue;
            };
            let end = text
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(text.len());
            return String::from_utf8_lossy(&text[..end]).to_string();
        }
        if data.starts_with(b"mluc") {
            let (Some(records), Some(record_size)) = (be32(data, 8), be32(data, 12)) else {
                continue;
            };
            if records == 0 || records > 1024 || record_size != 12 {
                continue;
            }
            let Some(table_end) = records
                .checked_mul(record_size)
                .and_then(|n| n.checked_add(16))
            else {
                continue;
            };
            let Some(table) = data.get(16..table_end) else {
                continue;
            };
            for record in table.chunks_exact(12) {
                let (Some(length), Some(start)) = (be32(record, 4), be32(record, 8)) else {
                    continue;
                };
                if length == 0 || length > 8192 || length % 2 != 0 || start < table_end {
                    continue;
                }
                let Some(text) = start
                    .checked_add(length)
                    .and_then(|end| data.get(start..end))
                else {
                    continue;
                };
                let units: Vec<u16> = text
                    .chunks_exact(2)
                    .map(|raw| u16::from_be_bytes([raw[0], raw[1]]))
                    .collect();
                if let Ok(description) = String::from_utf16(&units) {
                    return description;
                }
            }
        }
    }
    "unknown".to_owned()
}

/// macOS ColorSync 原生变换原型（FR3-014）。
/// 经 CoreGraphics 位图上下文在源/目标 ICC 色彩空间之间转换 RGB8；
/// 16-bit/float 与预览管线接入在 G2 评审后另行推进，参考值见同模块测试。
#[cfg(target_os = "macos")]
pub mod native_transform {
    use crate::domain::{AppError, Result};
    use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
    use core_foundation::data::{CFData, CFDataRef};
    use core_graphics::color_space::CGColorSpace;
    use core_graphics::context::CGContext;
    use core_graphics::geometry::{CGPoint, CGRect, CGSize};
    use foreign_types::ForeignType;
    use std::ffi::c_void;

    unsafe extern "C" {
        fn CGColorSpaceCreateWithICCProfile(data: CFDataRef) -> *mut c_void;
        fn CGMainDisplayID() -> u32;
        fn CGDisplayCopyColorSpace(display: u32) -> *mut c_void;
        fn CGColorSpaceCopyICCProfile(space: *mut c_void) -> CFDataRef;
    }

    fn space_from_icc(icc: &[u8]) -> Result<CGColorSpace> {
        let data = CFData::from_buffer(icc);
        let raw = unsafe { CGColorSpaceCreateWithICCProfile(data.as_concrete_TypeRef()) };
        if raw.is_null() {
            return Err(AppError::new(
                "COLOR_PROFILE_INVALID",
                "ICC 配置无法创建色彩空间",
            ));
        }
        Ok(unsafe { CGColorSpace::from_ptr(raw.cast()) })
    }

    // macOS 位图上下文不支持 24bpp 紧凑 RGB，采用 32bpp RGBX（kCGImageAlphaNoneSkipLast=5）。
    const ALPHA_NONE_SKIP_LAST: u32 = 5;
    // 16bpc 同样需 64bpp RGBX；小端字节序（kCGBitmapByteOrder16Little=0x1000，字节序字段位于 12–14 位）让内存布局与 u16 原生序一致。
    const ALPHA_NONE_SKIP_LAST_16LE: u32 = 5 | 0x1000;

    fn bitmap_context(space: &CGColorSpace, width: usize, height: usize) -> CGContext {
        CGContext::create_bitmap_context(
            None,
            width,
            height,
            8,
            width * 4,
            space,
            ALPHA_NONE_SKIP_LAST,
        )
    }

    fn bitmap_context16(space: &CGColorSpace, width: usize, height: usize) -> CGContext {
        CGContext::create_bitmap_context(
            None,
            width,
            height,
            16,
            width * 8,
            space,
            ALPHA_NONE_SKIP_LAST_16LE,
        )
    }

    // 32bpc 浮点位图：kCGBitmapFloatComponents(1<<8=256) + SkipLast(5) + ByteOrder32Little(2<<12=0x2000)，
    // 128bpp RGBA float；小端序使内存布局与 f32::to_le_bytes 一致。
    const FLOAT_SKIP_LAST_32LE: u32 = 256 | 5 | 0x2000;

    fn bitmap_context32f(space: &CGColorSpace, width: usize, height: usize) -> CGContext {
        CGContext::create_bitmap_context(
            None,
            width,
            height,
            32,
            width * 16,
            space,
            FLOAT_SKIP_LAST_32LE,
        )
    }

    /// 就地按目标 ICC 变换 RGB8 像素（紧密三通道）。
    pub fn transform_rgb8(
        src_icc: &[u8],
        dst_icc: &[u8],
        pixels: &mut [u8],
        width: usize,
        height: usize,
    ) -> Result<()> {
        if width == 0 || height == 0 || width > 65535 || height > 65535 {
            return Err(AppError::new("VALIDATION_ERROR", "像素尺寸超出支持范围"));
        }
        let expected = width
            .checked_mul(height)
            .and_then(|n| n.checked_mul(3))
            .ok_or_else(|| AppError::new("VALIDATION_ERROR", "像素尺寸溢出"))?;
        if pixels.len() != expected {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "像素缓冲长度与尺寸不匹配",
            ));
        }
        let src_space = space_from_icc(src_icc)?;
        let dst_space = space_from_icc(dst_icc)?;
        let mut src_ctx = bitmap_context(&src_space, width, height);
        let mut staging = vec![0u8; expected / 3 * 4];
        for (index, chunk) in pixels.chunks_exact(3).enumerate() {
            staging[index * 4..index * 4 + 3].copy_from_slice(chunk);
            staging[index * 4 + 3] = 255;
        }
        src_ctx.data().copy_from_slice(&staging);
        let src_image = src_ctx
            .create_image()
            .ok_or_else(|| AppError::new("COLOR_TRANSFORM_FAILED", "无法从源像素创建图像"))?;
        let mut dst_ctx = bitmap_context(&dst_space, width, height);
        dst_ctx.draw_image(
            CGRect::new(
                &CGPoint::new(0.0, 0.0),
                &CGSize::new(width as f64, height as f64),
            ),
            &src_image,
        );
        let converted = dst_ctx.data();
        for (index, chunk) in converted.chunks_exact(4).enumerate() {
            pixels[index * 3..index * 3 + 3].copy_from_slice(&chunk[..3]);
        }
        Ok(())
    }

    /// 就地按目标 ICC 变换 RGB16 像素（u16 三通道，主机字节序）。
    pub fn transform_rgb16(
        src_icc: &[u8],
        dst_icc: &[u8],
        pixels: &mut [u16],
        width: usize,
        height: usize,
    ) -> Result<()> {
        if width == 0 || height == 0 || width > 65535 || height > 65535 {
            return Err(AppError::new("VALIDATION_ERROR", "像素尺寸超出支持范围"));
        }
        let expected = width
            .checked_mul(height)
            .and_then(|n| n.checked_mul(3))
            .ok_or_else(|| AppError::new("VALIDATION_ERROR", "像素尺寸溢出"))?;
        if pixels.len() != expected {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "像素缓冲长度与尺寸不匹配",
            ));
        }
        let src_space = space_from_icc(src_icc)?;
        let dst_space = space_from_icc(dst_icc)?;
        let mut src_ctx = bitmap_context16(&src_space, width, height);
        let mut staging = Vec::with_capacity(expected / 3 * 8);
        for pixel in pixels.chunks_exact(3) {
            for value in pixel {
                staging.extend_from_slice(&value.to_le_bytes());
            }
            staging.extend_from_slice(&65535u16.to_le_bytes());
        }
        src_ctx.data().copy_from_slice(&staging);
        let src_image = src_ctx
            .create_image()
            .ok_or_else(|| AppError::new("COLOR_TRANSFORM_FAILED", "无法从源像素创建图像"))?;
        let mut dst_ctx = bitmap_context16(&dst_space, width, height);
        dst_ctx.draw_image(
            CGRect::new(
                &CGPoint::new(0.0, 0.0),
                &CGSize::new(width as f64, height as f64),
            ),
            &src_image,
        );
        let converted = dst_ctx.data();
        for (index, chunk) in converted.chunks_exact(8).enumerate() {
            for component in 0..3 {
                let bytes = [chunk[component * 2], chunk[component * 2 + 1]];
                pixels[index * 3 + component] = u16::from_le_bytes(bytes);
            }
        }
        Ok(())
    }

    /// 就地按目标 ICC 变换 RGB 浮点像素（f32 三通道，主机字节序）。
    pub fn transform_rgb32f(
        src_icc: &[u8],
        dst_icc: &[u8],
        pixels: &mut [f32],
        width: usize,
        height: usize,
    ) -> Result<()> {
        if width == 0 || height == 0 || width > 65535 || height > 65535 {
            return Err(AppError::new("VALIDATION_ERROR", "像素尺寸超出支持范围"));
        }
        let expected = width
            .checked_mul(height)
            .and_then(|n| n.checked_mul(3))
            .ok_or_else(|| AppError::new("VALIDATION_ERROR", "像素尺寸溢出"))?;
        if pixels.len() != expected {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "像素缓冲长度与尺寸不匹配",
            ));
        }
        let src_space = space_from_icc(src_icc)?;
        let dst_space = space_from_icc(dst_icc)?;
        let mut src_ctx = bitmap_context32f(&src_space, width, height);
        let mut staging = Vec::with_capacity(expected / 3 * 16);
        for pixel in pixels.chunks_exact(3) {
            for value in pixel {
                staging.extend_from_slice(&value.to_le_bytes());
            }
            staging.extend_from_slice(&1.0f32.to_le_bytes());
        }
        src_ctx.data().copy_from_slice(&staging);
        let src_image = src_ctx
            .create_image()
            .ok_or_else(|| AppError::new("COLOR_TRANSFORM_FAILED", "无法从源像素创建图像"))?;
        let mut dst_ctx = bitmap_context32f(&dst_space, width, height);
        dst_ctx.draw_image(
            CGRect::new(
                &CGPoint::new(0.0, 0.0),
                &CGSize::new(width as f64, height as f64),
            ),
            &src_image,
        );
        let converted = dst_ctx.data();
        for (index, chunk) in converted.chunks_exact(16).enumerate() {
            for component in 0..3 {
                let start = component * 4;
                let bytes = [
                    chunk[start],
                    chunk[start + 1],
                    chunk[start + 2],
                    chunk[start + 3],
                ];
                pixels[index * 3 + component] = f32::from_le_bytes(bytes);
            }
        }
        Ok(())
    }

    /// 主显示器 ICC 配置字节（FR3-014 目标色彩空间采集原型）。
    /// 可直接作为 transform_* 的 dst_icc；随显示器/校准变化，勿并入内容寻址缓存键。
    /// 无内嵌 ICC 的源按 sRGB 处理：与显示器 ICC 同源机制取得系统 sRGB 配置字节。
    /// ColorSync 命名常量即字符串本身（kCGColorSpaceSRGB）。
    pub fn srgb_profile_bytes() -> Result<Vec<u8>> {
        let name = core_foundation::string::CFString::new("kCGColorSpaceSRGB");
        let Some(space) =
            core_graphics::color_space::CGColorSpace::create_with_name(name.as_concrete_TypeRef())
        else {
            return Err(AppError::new(
                "COLOR_SRGB_UNAVAILABLE",
                "无法创建系统 sRGB 色彩空间",
            ));
        };
        unsafe {
            let data = CGColorSpaceCopyICCProfile(space.as_ptr() as *mut std::ffi::c_void);
            if data.is_null() {
                return Err(AppError::new(
                    "COLOR_SRGB_UNAVAILABLE",
                    "系统 sRGB 未提供 ICC 配置",
                ));
            }
            let profile = CFData::wrap_under_create_rule(data);
            Ok(profile.bytes().to_vec())
        }
    }

    pub fn display_profile_bytes() -> Result<Vec<u8>> {
        unsafe {
            let space = CGDisplayCopyColorSpace(CGMainDisplayID());
            if space.is_null() {
                return Err(AppError::new(
                    "COLOR_DISPLAY_UNAVAILABLE",
                    "无法获取主显示器色彩空间",
                ));
            }
            let data = CGColorSpaceCopyICCProfile(space);
            CFRelease(space as CFTypeRef);
            if data.is_null() {
                return Err(AppError::new(
                    "COLOR_DISPLAY_UNAVAILABLE",
                    "显示器色彩空间未提供 ICC 配置",
                ));
            }
            let profile = CFData::wrap_under_create_rule(data);
            Ok(profile.bytes().to_vec())
        }
    }
}

// ── G2-③ 色彩管理检视：源空间判定 → 显示器变换 ──

/// 缓存预览像素所在的源色彩空间结论。判定只看容器魔数与 ICC 结构，不认文件扩展名。
#[derive(Debug, PartialEq, Eq)]
pub enum SourceSpace {
    /// 原文件嵌入且结构合法的 ICC，字节可直接交给原生 CMM。
    Profile { icc: Vec<u8>, description: String },
    /// 容器可解析且确认没有嵌入 ICC：按 sRGB 处理，但结论要说明这是假定。
    AssumedSrgb,
    /// 读不出可靠源空间：不猜，把原因交给界面说明。
    Rejected {
        status: &'static str,
        detail: String,
    },
}

/// 只判定原文件的源色彩空间，不据此推断旧缓存的像素空间。
pub fn source_space_for_preview(original: &Path) -> SourceSpace {
    use std::io::Read;
    let mut file = match std::fs::File::open(original) {
        Ok(file) => file,
        Err(error) => {
            return SourceSpace::Rejected {
                status: "source_missing",
                detail: format!("原文件无法打开：{error}"),
            };
        }
    };
    let mut magic = [0_u8; 8];
    let read = match file.read(&mut magic) {
        Ok(read) => read,
        Err(error) => {
            return SourceSpace::Rejected {
                status: "source_missing",
                detail: format!("原文件读取失败：{error}"),
            };
        }
    };
    // 短文件也按已有字节判断：JPEG 只占 2 字节，PNG 占 8 字节。
    let head = &magic[..read];
    let parseable = head.starts_with(&[0xff, 0xd8])
        || head.starts_with(b"\x89PNG\r\n\x1a\n")
        || head.starts_with(b"II*\0")
        || head.starts_with(b"MM\0*");
    if !parseable {
        return SourceSpace::Rejected {
            status: "container_unsupported",
            detail: "该容器暂不支持读取嵌入色彩配置（HEIF、RAW、BigTIFF 等）".to_owned(),
        };
    }
    let Ok(metadata) = file.metadata() else {
        return SourceSpace::Rejected {
            status: "source_missing",
            detail: "原文件信息读取失败".to_owned(),
        };
    };
    let mut scan = ProfileScan::default();
    match extract_profile_scan(&mut file, metadata.len(), &mut scan) {
        None if scan.present || !scan.complete => SourceSpace::Rejected {
            status: "profile_invalid",
            detail: "色彩配置分段或容器元数据不完整，不能按无配置处理".into(),
        },
        None => SourceSpace::AssumedSrgb,
        Some(icc) if valid_icc_structure(&icc) => SourceSpace::Profile {
            description: parse_icc_description(&icc),
            icc,
        },
        Some(icc) => SourceSpace::Rejected {
            status: "profile_invalid",
            detail: format!("嵌入色彩配置 {} 字节未通过结构校验", icc.len()),
        },
    }
}

/// 色彩管理渲染结论：状态自解释；没有真正变换时不回传与源相同的像素。
#[derive(Debug)]
pub struct ManagedRender {
    /// transformed / assumed_srgb = 已变换；其余为未变换的原因代码。
    pub status: &'static str,
    pub detail: Option<String>,
    pub source_profile: String,
    pub display_profile: String,
    pub jpeg: Option<Vec<u8>>,
}

impl ManagedRender {
    fn stopped(
        status: &'static str,
        detail: String,
        source_profile: String,
        display_profile: String,
    ) -> Self {
        Self {
            status,
            detail: Some(detail),
            source_profile,
            display_profile,
            jpeg: None,
        }
    }
}

/// 判定好的两侧色彩配置与状态。缓存预览重渲染与全分辨率原图渲染共用同一次判定和同一套
/// 状态码，否则两条链路会各说各话地解释"为什么没变换"。Err 里已带回可直接上屏的说明。
#[cfg(target_os = "macos")]
pub struct RenderPlan {
    pub source_icc: Vec<u8>,
    pub display_icc: Vec<u8>,
    pub source_profile: String,
    pub display_profile: String,
    pub status: &'static str,
}

#[cfg(target_os = "macos")]
pub fn plan_managed_render(
    original: &Path,
    display_icc: crate::domain::Result<Vec<u8>>,
) -> std::result::Result<RenderPlan, ManagedRender> {
    let display_icc = match display_icc {
        Ok(icc) if valid_icc_structure(&icc) && icc.get(16..20) == Some(b"RGB ") => icc,
        Ok(icc) => {
            return Err(ManagedRender::stopped(
                "display_unavailable",
                format!("显示器色彩配置 {} 字节未通过结构校验", icc.len()),
                "无法确定".to_owned(),
                "不可用".to_owned(),
            ));
        }
        Err(error) => {
            return Err(ManagedRender::stopped(
                "display_unavailable",
                error.message,
                "无法确定".to_owned(),
                "不可用".to_owned(),
            ));
        }
    };
    let display_desc = parse_icc_description(&display_icc);
    let assumed = "未嵌入色彩配置（按 sRGB 假定）".to_owned();
    let (source_icc, source_desc, status) = match source_space_for_preview(original) {
        SourceSpace::Profile { icc, description } => (icc, description, "transformed"),
        SourceSpace::AssumedSrgb => match native_transform::srgb_profile_bytes() {
            Ok(icc) => (icc, assumed, "assumed_srgb"),
            Err(error) => {
                return Err(ManagedRender::stopped(
                    "transform_failed",
                    error.message,
                    assumed,
                    display_desc,
                ));
            }
        },
        SourceSpace::Rejected { status, detail } => {
            return Err(ManagedRender::stopped(
                status,
                detail,
                "无法确定".to_owned(),
                display_desc,
            ));
        }
    };
    if source_icc.get(16..20) != Some(b"RGB ") {
        return Err(ManagedRender::stopped(
            "profile_unsupported",
            "当前显式色彩检视只支持 RGB 源配置".to_owned(),
            source_desc,
            display_desc,
        ));
    }
    Ok(RenderPlan {
        source_icc,
        display_icc,
        source_profile: source_desc,
        display_profile: display_desc,
        status,
    })
}

/// 变换后的像素已经是显示器色彩空间的数值，但不带配置的 JPEG 会被渲染端当成 sRGB **再管一次**，
/// 同一次转换做两遍，颜色反而错得更远。把作为目标的那份显示器 ICC 写成 APP2 段，渲染端再管理
/// 时使用实际输出的色彩空间；窗口所在显示器不同时，系统仍需按该标记进行映射。
pub fn tag_jpeg_with_icc(jpeg: &[u8], icc: &[u8]) -> crate::domain::Result<Vec<u8>> {
    if !jpeg.starts_with(&[0xFF, 0xD8]) {
        return Err(crate::domain::AppError::new(
            "NOT_JPEG",
            "变换结果不是有效 JPEG，无法写入色彩配置",
        ));
    }
    // ICC JPEG 约定：每段最多 65519 字节配置数据，序号与总数各占一个字节。
    const CHUNK: usize = 65519;
    if icc.is_empty() || icc.len() > CHUNK * 255 {
        return Err(crate::domain::AppError::new(
            "ICC_TOO_LARGE",
            "色彩配置无法装入有界 JPEG ICC 段",
        ));
    }
    let count = icc.len().div_ceil(CHUNK);
    let mut out = Vec::with_capacity(jpeg.len() + icc.len() + count * 18);
    out.extend_from_slice(&jpeg[..2]);
    for (index, chunk) in icc.chunks(CHUNK).enumerate() {
        out.extend_from_slice(&[0xFF, 0xE2]);
        out.extend_from_slice(&((chunk.len() + 16) as u16).to_be_bytes());
        out.extend_from_slice(b"ICC_PROFILE\0");
        out.extend_from_slice(&[(index + 1) as u8, count as u8]);
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&jpeg[2..]);
    Ok(out)
}

/// 从原图解码并应用方向，保持 16-bit 变换精度，到显示用 JPEG 编码时才降到 8-bit。
/// 适合屏幕与全分辨率共用此路径，不解释来源不明的旧缓存像素。
#[cfg(target_os = "macos")]
pub(crate) fn render_source_jpeg(
    original: &std::path::Path,
    source_icc: &[u8],
    display_icc: &[u8],
    max_edge: u32,
    expected_sha256: Option<&str>,
) -> Result<Vec<u8>> {
    use crate::domain::AppError;
    use image::ImageDecoder;
    use std::io::Read;
    if max_edge == 0 || max_edge > 30_000 {
        return Err(AppError::new("INSPECTION_LIMIT", "渲染尺寸无效"));
    }
    let metadata = std::fs::metadata(original)?;
    if !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 {
        return Err(AppError::new(
            "INSPECTION_LIMIT",
            "原图超过色彩检视文件上限",
        ));
    }
    let mut bytes = vec![];
    std::fs::File::open(original)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(AppError::new(
            "INSPECTION_LIMIT",
            "原图超过色彩检视文件上限",
        ));
    }
    if let Some(expected) = expected_sha256 {
        use sha2::{Digest, Sha256};
        if format!("{:x}", Sha256::digest(&bytes)) != expected {
            return Err(AppError::new(
                "FILE_CHANGED",
                "实际解码的原图字节与登记版本不一致",
            ));
        }
    }
    if let Some(icc) = extract_embedded_icc(&mut std::io::Cursor::new(&bytes), bytes.len() as u64)
        && icc != source_icc
    {
        return Err(AppError::new("FILE_CHANGED", "解码快照与源色彩配置不一致"));
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    limits.max_image_width = Some(30_000);
    limits.max_image_height = Some(30_000);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| AppError::new("DECODE", e.to_string()))?;
    let (width, height) = decoder.dimensions();
    if matches!(
        decoder.color_type(),
        image::ColorType::Rgb32F | image::ColorType::Rgba32F
    ) {
        return Err(AppError::new(
            "COLOR_PRECISION_UNSUPPORTED",
            "浮点或 HDR 源尚未接入显示映射，未生成色彩检视",
        ));
    }
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 32_000_000 {
        return Err(AppError::new(
            "INSPECTION_LIMIT",
            "原图超过色彩检视像素上限",
        ));
    }
    let orientation = decoder
        .orientation()
        .map_err(|e| AppError::new("DECODE", e.to_string()))?;
    let mut bitmap = image::DynamicImage::from_decoder(decoder)
        .map_err(|e| AppError::new("DECODE", e.to_string()))?;
    bitmap.apply_orientation(orientation);
    if bitmap.width().max(bitmap.height()) > max_edge {
        bitmap = bitmap.thumbnail(max_edge, max_edge);
    }
    let alpha = bitmap.color().has_alpha().then(|| {
        bitmap
            .to_rgba16()
            .pixels()
            .map(|p| p[3])
            .collect::<Vec<_>>()
    });
    let rgb = bitmap.into_rgb16();
    let (width, height) = rgb.dimensions();
    let mut pixels = rgb.into_raw();
    crate::color_engine::native_transform::transform_rgb16(
        source_icc,
        display_icc,
        &mut pixels,
        width as usize,
        height as usize,
    )?;
    // 在目标显示色彩空间合成白底；透明区域不因丢 alpha 变黑。
    if let Some(alpha) = alpha {
        for (rgb, alpha) in pixels.chunks_exact_mut(3).zip(alpha) {
            let alpha = u64::from(alpha);
            for channel in rgb {
                *channel = ((u64::from(*channel) * alpha + 65535 * (65535 - alpha) + 32767) / 65535)
                    as u16;
            }
        }
    }
    let managed = image::RgbImage::from_raw(
        width,
        height,
        pixels
            .into_iter()
            .map(|v| ((u32::from(v) + 128) / 257) as u8)
            .collect(),
    )
    .ok_or_else(|| AppError::new("DECODE", "变换后缓冲尺寸不匹配"))?;
    let mut jpeg = vec![];
    image::DynamicImage::ImageRgb8(managed)
        .write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut jpeg, 92,
        ))
        .map_err(|e| AppError::new("ENCODE", e.to_string()))?;
    crate::color_engine::tag_jpeg_with_icc(&jpeg, display_icc)
}

/// 显式色彩预览只从原图生成，拒绝推断旧缓存的像素色彩身份。
#[cfg(target_os = "macos")]
pub fn render_managed_preview(
    original: &Path,
    display_icc: crate::domain::Result<Vec<u8>>,
) -> ManagedRender {
    let plan = match plan_managed_render(original, display_icc) {
        Ok(plan) => plan,
        Err(stopped) => return stopped,
    };
    match render_source_jpeg(original, &plan.source_icc, &plan.display_icc, 1400, None) {
        Ok(jpeg) => ManagedRender {
            status: plan.status,
            detail: None,
            source_profile: plan.source_profile,
            display_profile: plan.display_profile,
            jpeg: Some(jpeg),
        },
        Err(error) => ManagedRender::stopped(
            "transform_failed",
            error.message,
            plan.source_profile,
            plan.display_profile,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn profile() -> Vec<u8> {
        let name = b"Test Profile\0";
        let mut icc = vec![0u8; 156 + name.len()];
        let size = icc.len() as u32;
        icc[..4].copy_from_slice(&size.to_be_bytes());
        icc[36..40].copy_from_slice(b"acsp");
        icc[16..20].copy_from_slice(b"RGB ");
        icc[128..132].copy_from_slice(&1u32.to_be_bytes());
        icc[132..136].copy_from_slice(b"desc");
        icc[136..140].copy_from_slice(&144u32.to_be_bytes());
        icc[140..144].copy_from_slice(&(12 + name.len() as u32).to_be_bytes());
        icc[144..148].copy_from_slice(b"desc");
        icc[152..156].copy_from_slice(&(name.len() as u32).to_be_bytes());
        icc[156..].copy_from_slice(name);
        icc
    }
    fn extract(bytes: &[u8]) -> Option<Vec<u8>> {
        extract_embedded_icc(&mut Cursor::new(bytes), bytes.len() as u64)
    }
    fn jpeg_segment(sequence: u8, count: u8, data: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xe2];
        bytes.extend_from_slice(&((data.len() + 16) as u16).to_be_bytes());
        bytes.extend_from_slice(b"ICC_PROFILE\0");
        bytes.extend_from_slice(&[sequence, count]);
        bytes.extend_from_slice(data);
        bytes
    }
    fn png(data: &[u8]) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(b"iCCP");
        bytes.extend_from_slice(data);
        // 本测试只测试元数据定位；完整 PNG 解码/CRC 由图像解码器负责。
        bytes.extend_from_slice(&[0; 4]);
        bytes
    }

    #[test]
    fn jpeg_reassembles_sequence_and_rejects_missing_duplicate_or_inconsistent_chunks() {
        let icc = profile();
        let mut bytes = vec![0xff, 0xd8];
        bytes.extend(jpeg_segment(2, 2, &icc[80..]));
        bytes.extend(jpeg_segment(1, 2, &icc[..80]));
        bytes.extend([0xff, 0xda]);
        assert_eq!(extract(&bytes), Some(icc.clone()));
        for segments in [
            vec![jpeg_segment(2, 2, &icc)],
            vec![jpeg_segment(1, 2, &icc), jpeg_segment(1, 2, &icc)],
            vec![jpeg_segment(1, 2, &icc), jpeg_segment(2, 3, &icc)],
            vec![jpeg_segment(0, 1, &icc)],
        ] {
            let mut data = vec![0xff, 0xd8];
            data.extend(segments.concat());
            data.extend([0xff, 0xd9]);
            assert!(extract(&data).is_none());
        }
    }

    #[test]
    fn png_iccp_checks_name_compression_and_output_limit() {
        let icc = profile();
        let mut compressor =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        compressor.write_all(&icc).unwrap();
        let mut data = b"profile\0\0".to_vec();
        data.extend(compressor.finish().unwrap());
        assert_eq!(extract(&png(&data)), Some(icc));
        for invalid in [
            b"x\0".to_vec(),
            b"x\0\x01broken".to_vec(),
            b"\0\0broken".to_vec(),
        ] {
            assert!(extract(&png(&invalid)).is_none());
        }
        let mut compressor =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::copy(
            &mut std::io::repeat(0).take((MAX_ICC_BYTES + 1) as u64),
            &mut compressor,
        )
        .unwrap();
        let mut bomb = b"profile\0\0".to_vec();
        bomb.extend(compressor.finish().unwrap());
        assert!(extract(&png(&bomb)).is_none());
    }

    #[test]
    fn tiff_requires_ifd_tag_in_both_byte_orders_instead_of_signature_search() {
        let icc = profile();
        for little in [true, false] {
            let mut bytes = if little {
                b"II*\0".to_vec()
            } else {
                b"MM\0*".to_vec()
            };
            let number = |n: u32| {
                if little {
                    n.to_le_bytes()
                } else {
                    n.to_be_bytes()
                }
            };
            bytes.extend(number(8));
            bytes.extend(if little {
                1u16.to_le_bytes()
            } else {
                1u16.to_be_bytes()
            });
            bytes.extend(if little {
                34675u16.to_le_bytes()
            } else {
                34675u16.to_be_bytes()
            });
            bytes.extend(if little {
                7u16.to_le_bytes()
            } else {
                7u16.to_be_bytes()
            });
            bytes.extend(number(icc.len() as u32));
            bytes.extend(number(26));
            bytes.extend(number(0));
            bytes.extend(&icc);
            assert_eq!(extract(&bytes), Some(icc.clone()));
            bytes[10..12].fill(0);
            assert!(
                extract(&bytes).is_none(),
                "像素中出现 acsp 不能证明嵌入 profile"
            );
        }
    }

    #[test]
    fn description_accepts_ascii_and_mluc_and_rejects_invalid_tag_ranges() {
        let mut icc = profile();
        assert_eq!(parse_icc_description(&icc), "Test Profile");
        icc[140..144].copy_from_slice(&9u32.to_be_bytes());
        assert_eq!(parse_icc_description(&icc), "unknown");
        icc[136..140].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(parse_icc_description(&icc), "invalid");
        let text: Vec<u8> = "显示 P3"
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect();
        let mut icc = profile();
        icc.resize(144 + 28 + text.len(), 0);
        let len = icc.len() as u32;
        icc[..4].copy_from_slice(&len.to_be_bytes());
        icc[140..144].copy_from_slice(&(28 + text.len() as u32).to_be_bytes());
        icc[144..148].copy_from_slice(b"mluc");
        icc[152..156].copy_from_slice(&1u32.to_be_bytes());
        icc[156..160].copy_from_slice(&12u32.to_be_bytes());
        icc[160..164].copy_from_slice(b"zhCN");
        icc[164..168].copy_from_slice(&(text.len() as u32).to_be_bytes());
        icc[168..172].copy_from_slice(&28u32.to_be_bytes());
        icc[172..].copy_from_slice(&text);
        assert_eq!(parse_icc_description(&icc), "显示 P3");
    }

    #[test]
    fn all_truncations_and_hostile_lengths_are_safe() {
        let icc = profile();
        for length in 0..icc.len() {
            assert!(!valid_icc_structure(&icc[..length]));
            assert_eq!(parse_icc_description(&icc[..length]), "invalid");
        }
        for header in [
            b"\xff\xd8\xff\xe2\0\0\0\0".as_slice(),
            b"\xff\xd8\xff\xe2\0\x0fICC_PROFILE\0",
            b"\x89PNG\r\n\x1a\n\xff\xff\xff\xffiCCP",
        ] {
            for length in 0..=header.len() {
                assert!(extract(&header[..length]).is_none());
            }
        }
        for seed in 0..256usize {
            let mut malformed = vec![0; seed + 140];
            for (i, byte) in malformed.iter_mut().enumerate() {
                *byte = (i * 31 + seed) as u8;
            }
            let _ = extract(&malformed);
            assert!(!valid_icc_structure(&malformed));
        }
    }

    #[test]
    fn sparse_large_tiff_reads_profile_without_reading_pixels() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let mut file = temp.reopen().unwrap();
        let icc = profile();
        file.write_all(b"II*\0\x08\0\0\0\x01\0\x73\x87\x07\0")
            .unwrap();
        file.write_all(&(icc.len() as u32).to_le_bytes()).unwrap();
        file.write_all(&26u32.to_le_bytes()).unwrap();
        file.write_all(&0u32.to_le_bytes()).unwrap();
        file.write_all(&icc).unwrap();
        file.set_len(2 * 1024 * 1024 * 1024).unwrap();
        let result = read_source_profile(temp.path()).unwrap().unwrap();
        assert!(result.valid);
        assert_eq!(result.description, "Test Profile");
    }

    // ── G2-③ 色彩管理预览：源空间判定 ──

    fn jpeg_with(icc: Option<&[u8]>) -> Vec<u8> {
        let mut bytes = vec![0xff, 0xd8];
        if let Some(icc) = icc {
            bytes.extend(jpeg_segment(1, 1, icc));
        }
        bytes.extend([0xff, 0xd9]);
        bytes
    }

    fn written(name: &str, bytes: &[u8]) -> tempfile::NamedTempFile {
        let file = tempfile::Builder::new().prefix(name).tempfile().unwrap();
        std::fs::write(file.path(), bytes).unwrap();
        file
    }

    fn solid_jpeg(color: [u8; 3]) -> Vec<u8> {
        let image = image::RgbImage::from_pixel(8, 8, image::Rgb(color));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg)
            .unwrap();
        bytes
    }

    #[cfg(target_os = "macos")]
    fn system_icc(name: &str) -> Vec<u8> {
        let path = std::path::Path::new("/System/Library/ColorSync/Profiles").join(name);
        std::fs::read(&path).unwrap_or_else(|error| panic!("缺少系统色彩配置 {name}: {error}"))
    }

    #[test]
    fn tagged_jpeg_declares_the_display_profile_so_pixels_are_not_managed_twice() {
        let icc = profile();
        let original = solid_jpeg([220, 40, 30]);
        let tagged = tag_jpeg_with_icc(&original, &icc).unwrap();
        assert_eq!(extract(&tagged), Some(icc.clone()));
        assert_eq!(
            embedded_icc_bytes(written("tagged.jpg", &tagged).path()).unwrap(),
            Some(icc.clone())
        );
        // 标注只加头部，不改像素：解码出来仍是变换后的那一份数值。
        let decoded = image::load_from_memory(&tagged).unwrap().to_rgb8();
        assert_eq!(
            decoded.get_pixel(4, 4).0.to_vec(),
            image::load_from_memory(&original)
                .unwrap()
                .to_rgb8()
                .get_pixel(4, 4)
                .0
                .to_vec()
        );
        // 不是 JPEG、或配置超出 JPEG ICC 分段上限时明确失败，绝不交出一张"没标注却自称已管理"的图。
        assert_eq!(
            tag_jpeg_with_icc(b"not a jpeg", &icc).unwrap_err().code,
            "NOT_JPEG"
        );
        assert_eq!(
            tag_jpeg_with_icc(&original, &vec![0u8; 65519 * 255 + 1])
                .unwrap_err()
                .code,
            "ICC_TOO_LARGE"
        );
    }

    #[test]
    fn cache_declaration_refuses_to_guess_a_space() {
        let plain = written("plain.jpg", &solid_jpeg([10, 20, 30]));
        assert_eq!(
            source_profile_bytes_for_cache(plain.path()),
            None,
            "源没有嵌入配置时不替缓存编一个身份"
        );

        let rgb = written(
            "rgb.jpg",
            &tag_jpeg_with_icc(&solid_jpeg([10, 20, 30]), &profile()).unwrap(),
        );
        assert_eq!(
            source_profile_bytes_for_cache(rgb.path()).as_deref(),
            Some(profile().as_slice()),
            "合法 RGB 配置原样交给缓存"
        );

        let mut gray = profile();
        gray[16..20].copy_from_slice(b"GRAY");
        let gray_file = written(
            "gray.jpg",
            &tag_jpeg_with_icc(&solid_jpeg([10, 20, 30]), &gray).unwrap(),
        );
        assert_eq!(
            source_profile_bytes_for_cache(gray_file.path()),
            None,
            "结构完整但不是 RGB 的配置，不写进 RGB 缓存"
        );
    }

    #[test]
    fn upload_pixels_without_a_known_space_stay_unclaimed() {
        let mut pixels = vec![210, 40, 60, 1, 2, 3];
        let before = pixels.clone();
        assert_eq!(
            normalize_upload_pixels(&mut pixels, 2, 1, None).unwrap(),
            None,
            "核对不出身份时不交出标签"
        );
        assert_eq!(pixels, before, "身份不明时一个像素都不动");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn upload_pixels_already_in_srgb_are_not_managed_twice() {
        let srgb = srgb_cache_profile_bytes().expect("系统 sRGB 配置缺失时本用例不成立");
        let mut pixels = vec![200, 100, 50, 12, 200, 9];
        let before = pixels.clone();
        let declared = normalize_upload_pixels(&mut pixels, 2, 1, Some(&srgb)).unwrap();
        assert_eq!(declared.as_deref(), Some(srgb.as_slice()));
        assert_eq!(pixels, before, "已经是 sRGB 的数值不再变换一次");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn upload_pixels_from_a_wide_gamut_source_are_transformed_once_and_declare_srgb() {
        let path = std::path::Path::new("/System/Library/ColorSync/Profiles/Display P3.icc");
        assert!(path.is_file(), "广色源配置缺失时本用例不成立");
        let p3 = std::fs::read(path).unwrap();
        let srgb = srgb_cache_profile_bytes().expect("系统 sRGB 配置缺失时本用例不成立");
        let mut pixels = vec![200, 100, 50, 12, 200, 9];
        let before = pixels.clone();
        let declared = normalize_upload_pixels(&mut pixels, 2, 1, Some(&p3))
            .unwrap()
            .expect("可核对的广色域源必须落到确定身份");
        assert_eq!(
            declared, srgb,
            "上传件声明的是真正落到的 sRGB，而不是源配置"
        );
        assert_ne!(pixels, before, "P3 数值确实被变换过，不是只换个标签");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_profile_the_engine_cannot_load_fails_instead_of_faking_a_label() {
        // 结构可读、标记也是 RGB，但引擎认不出的配置不能换来一份"看起来已管理"的上传件。
        let fake = profile();
        let mut pixels = vec![200, 100, 50];
        let before = pixels.clone();
        let error = normalize_upload_pixels(&mut pixels, 1, 1, Some(&fake)).unwrap_err();
        assert_eq!(
            error.code, "COLOR_PROFILE_INVALID",
            "变换失败要如实报错：{error:?}"
        );
        assert_eq!(pixels, before, "报错时不留下半成品像素");
    }

    #[test]
    fn declared_rgb_profile_accepts_only_a_loadable_rgb_declaration() {
        let tagged = tag_jpeg_with_icc(&solid_jpeg([10, 20, 30]), &profile()).unwrap();
        assert_eq!(
            declared_rgb_profile(&tagged).as_deref(),
            Some(profile().as_slice())
        );
        assert_eq!(declared_rgb_profile(&solid_jpeg([10, 20, 30])), None);
        let mut cmyk = profile();
        cmyk[16..20].copy_from_slice(b"CMYK");
        let cmyk_file = tag_jpeg_with_icc(&solid_jpeg([10, 20, 30]), &cmyk).unwrap();
        assert_eq!(
            declared_rgb_profile(&cmyk_file),
            None,
            "非 RGB 声明不做任何 RGB 承诺"
        );
    }

    #[test]
    fn large_icc_roundtrips_across_multiple_jpeg_segments_without_changing_pixels() {
        let mut icc = profile();
        icc.resize(150_000, 0);
        let len = icc.len() as u32;
        icc[..4].copy_from_slice(&len.to_be_bytes());
        let original = solid_jpeg([20, 100, 180]);
        let tagged = tag_jpeg_with_icc(&original, &icc).unwrap();
        assert_eq!(extract(&tagged), Some(icc));
        assert_eq!(
            image::load_from_memory(&tagged).unwrap().to_rgb8(),
            image::load_from_memory(&original).unwrap().to_rgb8()
        );
        assert_eq!(
            tagged
                .windows(12)
                .filter(|s| *s == b"ICC_PROFILE\0")
                .count(),
            3
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn explicit_view_rejects_non_rgb_source_profile_without_publishing_pixels() {
        let mut icc = system_icc("sRGB Profile.icc");
        icc[16..20].copy_from_slice(b"CMYK");
        let bytes = tag_jpeg_with_icc(&solid_jpeg([20, 100, 180]), &icc).unwrap();
        let file = written("unsupported.jpg", &bytes);
        let rendered = render_managed_preview(file.path(), Ok(system_icc("sRGB Profile.icc")));
        assert_eq!(rendered.status, "profile_unsupported");
        assert!(rendered.jpeg.is_none());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn profiled_original_is_converted_once_and_float_sources_are_not_silently_clipped() {
        let p3 = system_icc("Display P3.icc");
        let srgb = system_icc("sRGB Profile.icc");
        let bytes = tag_jpeg_with_icc(&solid_jpeg([64, 128, 128]), &p3).unwrap();
        let original = written("p3.jpg", &bytes);
        let rendered = render_managed_preview(original.path(), Ok(srgb.clone()));
        assert_eq!(rendered.status, "transformed");
        let pixel = image::load_from_memory(rendered.jpeg.as_ref().unwrap())
            .unwrap()
            .to_rgb8()
            .get_pixel(4, 4)
            .0;
        // P3→sRGB 此样本红通道约32：不转换约64，重复转换会压到接近0。
        assert!(
            (20..=45).contains(&pixel[0]) && pixel[1] > 118 && pixel[2] > 118,
            "{pixel:?}"
        );
        let dir = tempfile::tempdir().unwrap();
        let float = dir.path().join("float.tiff");
        image::Rgb32FImage::from_pixel(4, 4, image::Rgb([0.5, 1.2, 0.3]))
            .save(&float)
            .unwrap();
        assert_eq!(
            render_source_jpeg(&float, &srgb, &srgb, 1400, None)
                .unwrap_err()
                .code,
            "COLOR_PRECISION_UNSUPPORTED"
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn incomplete_icc_chunks_are_not_assumed_to_be_an_untagged_srgb_source() {
        let srgb = system_icc("sRGB Profile.icc");
        let mut bytes = tag_jpeg_with_icc(&solid_jpeg([20, 100, 180]), &srgb).unwrap();
        let marker = bytes
            .windows(12)
            .position(|w| w == b"ICC_PROFILE\0")
            .unwrap();
        bytes[marker + 13] = 2; // 声明两段，实际仅有第一段。
        let file = written("incomplete.jpg", &bytes);
        assert!(matches!(
            source_space_for_preview(file.path()),
            SourceSpace::Rejected {
                status: "profile_invalid",
                ..
            }
        ));
        let rendered = render_managed_preview(file.path(), Ok(srgb));
        assert_eq!(rendered.status, "profile_invalid");
        assert!(rendered.jpeg.is_none());
    }

    #[test]
    fn source_space_assumes_srgb_only_for_parseable_container_without_profile() {
        // 容器可解析且确认没有嵌入 ICC 才允许假定；扩展名不参与判断。
        let file = written("plain.jpg", &solid_jpeg([10, 20, 30]));
        assert_eq!(
            source_space_for_preview(file.path()),
            SourceSpace::AssumedSrgb
        );
    }

    #[test]
    fn source_space_uses_the_embedded_profile_bytes_verbatim() {
        let icc = profile();
        let file = written("embedded.jpg", &jpeg_with(Some(&icc)));
        match source_space_for_preview(file.path()) {
            SourceSpace::Profile {
                icc: found,
                description,
            } => {
                assert_eq!(found, icc);
                assert_eq!(description, "Test Profile");
            }
            other => panic!("应使用嵌入配置，实际 {other:?}"),
        }
    }

    #[test]
    fn source_space_refuses_containers_it_cannot_read_instead_of_guessing() {
        // HEIF 魔数与 BigTIFF 都读不出源空间：必须拒绝，不能按 sRGB 变换。
        for (label, bytes) in [
            ("heic", b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00".to_vec()),
            ("bigtiff", b"II+\0\x08\0\x00\0".to_vec()),
            ("unrecognised", vec![0x00, 0x01, 0x02]),
        ] {
            let file = written(label, &bytes);
            let SourceSpace::Rejected { status, detail } = source_space_for_preview(file.path())
            else {
                panic!("{label} 应被拒绝");
            };
            assert_eq!(status, "container_unsupported", "{label}");
            assert!(!detail.is_empty(), "{label}");
        }
    }

    #[test]
    fn source_space_separates_missing_original_from_invalid_profile() {
        // 原文件失联与"有嵌入数据但结构损坏"是两回事，界面说的话不一样。
        let missing = std::path::Path::new("/tmp/between-frames-definitely-absistent-20261104.bin");
        assert!(matches!(
            source_space_for_preview(missing),
            SourceSpace::Rejected {
                status: "source_missing",
                ..
            }
        ));
        let broken = written("broken.jpg", &jpeg_with(Some(&[0_u8; 50])));
        let SourceSpace::Rejected { status, detail } = source_space_for_preview(broken.path())
        else {
            panic!("结构损坏的嵌入配置应被拒绝");
        };
        assert_eq!(status, "profile_invalid");
        assert!(detail.contains("50"), "{detail}");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn managed_preview_transforms_with_the_embedded_profile_and_leaves_original_untouched() {
        let srgb = system_icc("sRGB Profile.icc");
        let original_bytes = tag_jpeg_with_icc(&solid_jpeg([220, 40, 30]), &srgb).unwrap();
        let file = written("source.jpg", &original_bytes);
        let outcome = render_managed_preview(file.path(), Ok(srgb.clone()));
        assert_eq!(outcome.status, "transformed", "{:?}", outcome.detail);
        let jpeg = outcome.jpeg.clone().unwrap();
        assert!(
            outcome.source_profile.to_ascii_lowercase().contains("srgb"),
            "{}",
            outcome.source_profile
        );
        // 输出的数值已是显示器色彩空间，必须标注成那份显示器配置；不标注就会被渲染端
        // 当作 sRGB 再管一次，同一次转换做两遍，颜色反而更错。
        assert_eq!(
            embedded_icc_bytes(written("managed.jpg", &jpeg).path()).unwrap(),
            Some(srgb.clone())
        );
        let pixels = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        let red = pixels.get_pixel(4, 4);
        // sRGB → sRGB 近似恒等：变换不应把颜色搬走，只证明像素确实经过原生 CMM 输出。
        assert!(
            (red[0] as i16 - 220).abs() <= 12 && red[1] < 70 && red[2] < 70,
            "{red:?}"
        );
        // 检视只读：原文件字节一个都不该变。
        assert_eq!(std::fs::read(file.path()).unwrap(), original_bytes);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn managed_preview_says_the_source_space_was_assumed_when_nothing_is_embedded() {
        let srgb = system_icc("sRGB Profile.icc");
        let file = written("plain.jpg", &solid_jpeg([30, 200, 40]));
        let outcome = render_managed_preview(file.path(), Ok(srgb));
        assert_eq!(outcome.status, "assumed_srgb", "{:?}", outcome.detail);
        assert!(outcome.jpeg.is_some());
        assert!(
            outcome.source_profile.contains("假定"),
            "{}",
            outcome.source_profile
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn managed_preview_returns_no_pixels_when_nothing_was_transformed() {
        // 未变换却回传同一份缓存字节，就是"看起来成功、其实没变换"的老毛病。
        let srgb = system_icc("sRGB Profile.icc");
        let heic = written("photo.heic", b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00");
        let outcome = render_managed_preview(heic.path(), Ok(srgb.clone()));
        assert_eq!(outcome.status, "container_unsupported");
        assert!(outcome.jpeg.is_none());
        assert!(outcome.detail.unwrap().contains("HEIF"));

        let display = written("plain.jpg", &jpeg_with(None));
        let error = crate::domain::AppError::new("COLOR_DISPLAY_UNAVAILABLE", "拿不到显示器配置");
        let outcome = render_managed_preview(display.path(), Err(error));
        assert_eq!(outcome.status, "display_unavailable");
        assert!(outcome.jpeg.is_none());
        assert_eq!(outcome.detail.as_deref(), Some("拿不到显示器配置"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn managed_preview_reports_the_transformer_own_failure() {
        // 结构自检通过、原生 CMM 却建不出色彩空间：失败原因必须原样上屏。
        let file = written("fake.jpg", &jpeg_with(Some(&profile())));
        let outcome = render_managed_preview(file.path(), Ok(system_icc("sRGB Profile.icc")));
        assert_eq!(outcome.status, "transform_failed", "{:?}", outcome);
        assert!(outcome.jpeg.is_none());
        assert!(!outcome.detail.unwrap().is_empty());
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    #[test]
    fn absent_profile_returns_none() {
        // 无嵌入 profile → None（Absent 路径）
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no-icc.jpg");
        std::fs::write(&path, [0xFF, 0xD8, 0xFF, 0xD9]).unwrap(); // 最小 JPEG
        assert!(read_source_profile(&path).unwrap().is_none());
    }

    #[test]
    fn valid_profile_gets_valid_diagnostic() {
        // 构造结构合法的最小 ICC → Valid 诊断
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("valid.png");
        // PNG with iCCP（用测试辅助构造）
        let icc = valid_test_icc();
        let png = png_with_iccp(&icc);
        std::fs::write(&path, png).unwrap();
        let profile = read_source_profile(&path).unwrap().unwrap();
        assert_eq!(profile.diagnostic, ProfileDiagnostic::Valid);
        assert!(profile.valid);
    }

    #[test]
    fn invalid_profile_gets_invalid_diagnostic() {
        // 有嵌入数据但结构损坏 → Invalid 诊断（不是静默丢弃）
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.png");
        let broken_icc = [0u8; 50]; // 太短，结构校验必失败
        let png = png_with_iccp(&broken_icc);
        std::fs::write(&path, png).unwrap();
        let profile = read_source_profile(&path).unwrap();
        // 提取可能失败返回 None，或提取成功但校验失败 → 两种都是明确路径
        match profile {
            None => { /* Absent 路径：PNG 解析器可能拒绝损坏 chunk */ }
            Some(p) => {
                assert!(!p.valid);
                assert_eq!(p.diagnostic.as_str(), "invalid");
            }
        }
    }

    fn valid_test_icc() -> Vec<u8> {
        // 最小合法 ICC：128 header + tag table + desc tag
        let mut icc = vec![0u8; 200];
        let total = icc.len() as u32;
        icc[0..4].copy_from_slice(&total.to_be_bytes()); // 声明总长
        icc[36..40].copy_from_slice(b"acsp");
        icc[128..132].copy_from_slice(&1u32.to_be_bytes());
        icc[132..136].copy_from_slice(b"desc");
        icc[136..140].copy_from_slice(&144u32.to_be_bytes());
        icc[140..144].copy_from_slice(&40u32.to_be_bytes());
        icc[144..148].copy_from_slice(b"desc");
        icc
    }

    fn png_with_iccp(icc: &[u8]) -> Vec<u8> {
        // 构造最小 PNG + iCCP chunk（zlib 压缩 ICC）
        use std::io::Write;
        let mut compressed = Vec::new();
        let mut encoder =
            flate2::write::ZlibEncoder::new(&mut compressed, flate2::Compression::default());
        encoder.write_all(icc).unwrap();
        drop(encoder);
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        // IHDR
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]);
        let crc = crc32(&png[12..29]);
        png.extend_from_slice(&crc.to_be_bytes());
        // iCCP
        let mut chunk_data = Vec::new();
        chunk_data.extend_from_slice(b"test\0"); // name + null
        chunk_data.push(0); // compression method
        chunk_data.extend_from_slice(&compressed);
        png.extend_from_slice(&(chunk_data.len() as u32).to_be_bytes());
        png.extend_from_slice(b"iCCP");
        let crc_start = png.len();
        png.extend_from_slice(&chunk_data);
        let crc = crc32(&png[crc_start..]);
        png.extend_from_slice(&crc.to_be_bytes());
        png
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFFFFFF;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }

    #[cfg(target_os = "macos")]
    fn system_profile(name: &str) -> Vec<u8> {
        let path = std::path::Path::new("/System/Library/ColorSync/Profiles").join(name);
        std::fs::read(&path).unwrap_or_else(|error| panic!("缺少系统色彩配置 {name}: {error}"))
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_transform_identity_keeps_srgb_pixels() {
        let srgb = system_profile("sRGB Profile.icc");
        let mut pixels = vec![
            255u8, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 18, 99, 200,
        ];
        let expected = pixels.clone();
        super::native_transform::transform_rgb8(&srgb, &srgb, &mut pixels, 5, 1).unwrap();
        for (actual, want) in pixels.iter().zip(expected.iter()) {
            assert!(
                ((*actual as i32) - (*want as i32)).abs() <= 1,
                "identity 差异过大: {pixels:?}"
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_transform_srgb_to_display_p3_matches_reference() {
        let srgb = system_profile("sRGB Profile.icc");
        let p3 = system_profile("Display P3.icc");
        let mut pixels = vec![255u8, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        super::native_transform::transform_rgb8(&srgb, &p3, &mut pixels, 4, 1).unwrap();
        let channel = |pixel: usize, component: usize| pixels[pixel * 3 + component] as i32;
        // 预登记参考值（P3 原色 + sRGB 传递函数，D65）：
        assert!((channel(0, 0) - 234).abs() <= 3, "红R {}", channel(0, 0));
        assert!((channel(0, 1) - 51).abs() <= 3, "红G {}", channel(0, 1));
        assert!((channel(0, 2) - 35).abs() <= 3, "红B {}", channel(0, 2));
        assert!((channel(1, 0) - 117).abs() <= 3, "绿R {}", channel(1, 0));
        assert!((channel(1, 1) - 251).abs() <= 3, "绿G {}", channel(1, 1));
        assert!((channel(1, 2) - 76).abs() <= 3, "绿B {}", channel(1, 2));
        // sRGB 蓝原色与 P3 同色度但最大亮度更低（Y 0.0722 对 0.0793），
        // 线性比≈0.9106，经 sRGB 传递函数编码≈245，而非 255：
        assert!((channel(2, 2) - 245).abs() <= 3, "蓝B {}", channel(2, 2));
        assert!(channel(2, 0) <= 3 && channel(2, 1) <= 3, "蓝通道纯净");
        for component in 0..3 {
            assert!(
                (channel(3, component) - 255).abs() <= 1,
                "白通道 {}",
                channel(3, component)
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_transform_rejects_invalid_profiles_and_buffers() {
        let srgb = system_profile("sRGB Profile.icc");
        let mut pixels = vec![0u8; 3];
        assert!(
            super::native_transform::transform_rgb8(b"not an icc", &srgb, &mut pixels, 1, 1)
                .is_err()
        );
        assert!(super::native_transform::transform_rgb8(&srgb, &srgb, &mut pixels, 2, 1).is_err());
        let mut wrong = vec![0u8; 4];
        assert!(super::native_transform::transform_rgb8(&srgb, &srgb, &mut wrong, 1, 1).is_err())
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_transform16_identity_keeps_srgb_pixels() {
        let srgb = system_profile("sRGB Profile.icc");
        let mut pixels: Vec<u16> = vec![
            65535, 0, 0, 0, 65535, 0, 0, 0, 65535, 65535, 65535, 65535, 4626, 25443, 51400,
        ];
        let expected = pixels.clone();
        super::native_transform::transform_rgb16(&srgb, &srgb, &mut pixels, 5, 1).unwrap();
        for (actual, want) in pixels.iter().zip(expected.iter()) {
            assert!(
                ((*actual as i32) - (*want as i32)).abs() <= 257,
                "16 位 identity 差异过大: {pixels:?}"
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_transform16_srgb_to_display_p3_matches_reference() {
        let srgb = system_profile("sRGB Profile.icc");
        let p3 = system_profile("Display P3.icc");
        let mut pixels: Vec<u16> = vec![65535, 0, 0, 0, 65535, 0, 0, 0, 65535, 65535, 65535, 65535];
        super::native_transform::transform_rgb16(&srgb, &p3, &mut pixels, 4, 1).unwrap();
        let scaled = |pixel: usize, component: usize| {
            (pixels[pixel * 3 + component] as f64 / 257.0).round() as i32
        };
        // 16 位结果按 8 位尺度（÷257）须与 RGB8 预登记参考一致：
        // 红 234/51/35、绿 117/251/76、蓝 245、白 255。
        for (component, want) in [(0usize, 234), (1, 51), (2, 35)] {
            assert!(
                (scaled(0, component) - want).abs() <= 2,
                "红{component} {}",
                scaled(0, component)
            );
        }
        for (component, want) in [(0usize, 117), (1, 251), (2, 76)] {
            assert!(
                (scaled(1, component) - want).abs() <= 2,
                "绿{component} {}",
                scaled(1, component)
            );
        }
        assert!((scaled(2, 2) - 245).abs() <= 2, "蓝B {}", scaled(2, 2));
        for component in 0..3 {
            assert!(
                (scaled(3, component) - 255).abs() <= 1,
                "白{component} {}",
                scaled(3, component)
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_transform16_rejects_invalid_buffers() {
        let srgb = system_profile("sRGB Profile.icc");
        let mut pixels: Vec<u16> = vec![0; 3];
        assert!(super::native_transform::transform_rgb16(&srgb, &srgb, &mut pixels, 2, 1).is_err());
        let mut wrong: Vec<u16> = vec![0; 4];
        assert!(super::native_transform::transform_rgb16(&srgb, &srgb, &mut wrong, 1, 1).is_err());
        assert!(
            super::native_transform::transform_rgb16(b"bad", &srgb, &mut pixels, 1, 1).is_err()
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_transform32f_identity_keeps_srgb_pixels() {
        let srgb = system_profile("sRGB Profile.icc");
        let mut pixels: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.07, 0.4, 0.8];
        let expected = pixels.clone();
        super::native_transform::transform_rgb32f(&srgb, &srgb, &mut pixels, 3, 1).unwrap();
        for (actual, want) in pixels.iter().zip(expected.iter()) {
            assert!(
                (actual - want).abs() <= 0.002,
                "float identity 差异过大: {pixels:?}"
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_transform32f_srgb_to_display_p3_matches_reference() {
        let srgb = system_profile("sRGB Profile.icc");
        let p3 = system_profile("Display P3.icc");
        let mut pixels: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
        super::native_transform::transform_rgb32f(&srgb, &p3, &mut pixels, 4, 1).unwrap();
        let scaled = |pixel: usize, component: usize| pixels[pixel * 3 + component] * 255.0;
        // 浮点结果 ×255 须与 RGB8 预登记参考一致：红 234/51/35、绿 117/251/76、蓝 245、白 255。
        for (component, want) in [(0usize, 234.0), (1, 51.0), (2, 35.0)] {
            assert!(
                (scaled(0, component) - want).abs() <= 2.0,
                "红{component} {}",
                scaled(0, component)
            );
        }
        for (component, want) in [(0usize, 117.0), (1, 251.0), (2, 76.0)] {
            assert!(
                (scaled(1, component) - want).abs() <= 2.0,
                "绿{component} {}",
                scaled(1, component)
            );
        }
        assert!((scaled(2, 2) - 245.0).abs() <= 2.0, "蓝B {}", scaled(2, 2));
        for component in 0..3 {
            assert!(
                (pixels[3 * 3 + component] - 1.0).abs() <= 0.008,
                "白{component} {}",
                pixels[3 * 3 + component]
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_transform32f_rejects_invalid_buffers() {
        let srgb = system_profile("sRGB Profile.icc");
        let mut pixels: Vec<f32> = vec![0.0; 3];
        assert!(
            super::native_transform::transform_rgb32f(&srgb, &srgb, &mut pixels, 2, 1).is_err()
        );
        let mut wrong: Vec<f32> = vec![0.0; 4];
        assert!(super::native_transform::transform_rgb32f(&srgb, &srgb, &mut wrong, 1, 1).is_err());
        assert!(
            super::native_transform::transform_rgb32f(b"bad", &srgb, &mut pixels, 1, 1).is_err()
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn display_profile_is_valid_icc_and_transform_target() {
        let profile = super::native_transform::display_profile_bytes().unwrap();
        // ICC 头 128 字节、签名 "acsp" 位于偏移 36；不断言具体机型/校准身份（runner 显示配置各异）。
        assert!(profile.len() >= 128, "ICC 过短: {}", profile.len());
        assert_eq!(profile[36..40], *b"acsp");
        let srgb = system_profile("sRGB Profile.icc");
        let mut pixels = vec![255u8, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        super::native_transform::transform_rgb8(&srgb, &profile, &mut pixels, 4, 1).unwrap();
        // 白色在 D65 基准的常见显示配置下应保持接近白色；宽容量适配不同默认配置。
        assert!(
            pixels[9] >= 250 && pixels[10] >= 250 && pixels[11] >= 250,
            "白色漂移: {pixels:?}"
        );
    }
}
