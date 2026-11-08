//! AI 上传图片压缩：普通观察使用 512px JPEG；明确授权的精查保留最多 1400px。
//! 目的：减小 AI 派生图；整份请求大小仍取决于图像数量、内容与文本。
//! 原图只读——压缩的是内存副本，磁盘缓存不动。
//! 交出去的像素身份由 color_engine 统一判定：能核对的落到 sRGB 并标注，核对不了的留白。

use crate::domain::{AppError, Result};
use std::io::Read;

const MAX_EDGE: u32 = 512;
const JPEG_QUALITY: u8 = 75;
const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

/// 读取预览文件并压缩为 ≤512px JPEG（内存操作，不写磁盘）。
/// 输入已经是小图（<512px）时原样返回 JPEG 重编码结果。
pub fn compress_for_upload(path: &std::path::Path) -> Result<Vec<u8>> {
    Ok(compress_and_fingerprint(path)?.0)
}

/// 同上，但一并交出这份字节的 sha256。观察缓存要绑定"发出去的就是这几个字节"，
/// 才能在预览重新生成过之后发现旧观察描述的是另一份像素。
pub fn compress_and_fingerprint(path: &std::path::Path) -> Result<(Vec<u8>, String)> {
    compress_profile(path, false)
}

pub(crate) fn compress_profile(path: &std::path::Path, detail: bool) -> Result<(Vec<u8>, String)> {
    let max_edge = if detail { 1400 } else { MAX_EDGE };
    let quality = if detail { 90 } else { JPEG_QUALITY };
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| AppError::new("RESOURCE_MISSING", "照片预览不可读"))?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err(AppError::new(
            "AI_IMAGE_INVALID",
            "照片预览格式或大小不符合要求",
        ));
    }
    let mut data = Vec::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(MAX_INPUT_BYTES + 1).read_to_end(&mut data))
        .map_err(|_| AppError::new("RESOURCE_MISSING", "照片预览不可读"))?;
    if data.len() as u64 > MAX_INPUT_BYTES {
        return Err(AppError::new("AI_IMAGE_INVALID", "照片预览超过大小上限"));
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(&data))
        .with_guessed_format()
        .map_err(|_| AppError::new("DECODE", "照片预览格式无法识别"))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(1400);
    limits.max_image_height = Some(1400);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let img = reader
        .decode()
        .map_err(|_| AppError::new("DECODE", "照片预览无法读取，请重新生成预览"))?;
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let scale = if w.max(h) > max_edge {
        f64::from(max_edge) / f64::from(w.max(h))
    } else {
        1.0
    };
    let (nw, nh) = if scale < 1.0 {
        (
            (f64::from(w) * scale).round().max(1.0) as u32,
            (f64::from(h) * scale).round().max(1.0) as u32,
        )
    } else {
        (w, h)
    };
    let mut resized = if (nw, nh) != (w, h) {
        image::imageops::resize(&rgb, nw, nh, image::imageops::FilterType::Lanczos3)
    } else {
        rgb
    };
    // 上传件的色彩身份在编码前定死：预览带得出可核对的 RGB 声明时，真正做一次变换落到
    // sRGB 并标上 sRGB；带不出时数值原样保留、不补标签。无标签 JPEG 会被接收方当成
    // sRGB 解读，所以宁可留白也不能让广色域数值冒充 sRGB。
    let source_icc = crate::color_engine::declared_rgb_profile(&data);
    let declaration = crate::color_engine::normalize_upload_pixels(
        resized.as_mut(),
        nw as usize,
        nh as usize,
        source_icc.as_deref(),
    )?;
    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
    image::DynamicImage::ImageRgb8(resized)
        .write_with_encoder(encoder)
        .map_err(|e| AppError::new("ENCODE", format!("JPEG 编码失败：{e}")))?;
    // 标签先写完再量体积、算指纹：观察绑定的是真正交出去的那份字节。
    let out = match declaration.as_deref() {
        Some(icc) => crate::color_engine::tag_jpeg_with_icc(&out, icc)?,
        None => out,
    };
    if out.len() > 2 * 1024 * 1024 {
        return Err(AppError::new(
            "AI_IMAGE_INVALID",
            "精查预览超过发送上限，请缩小范围",
        ));
    }
    let hash = hex_sha256(&out);
    Ok((out, hash))
}

fn hex_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_keeps_bounded_preview_pixels_without_upscaling_or_touching_source() {
        let dir = tempfile::tempdir().unwrap();
        for (width, height) in [(1400, 933), (200, 150), (1, 1400)] {
            let path = dir.path().join(format!("detail-{width}-{height}.png"));
            image::RgbImage::from_pixel(width, height, image::Rgb([120, 150, 200]))
                .save(&path)
                .unwrap();
            let source = std::fs::read(&path).unwrap();
            let (detail, hash) = compress_profile(&path, true).unwrap();
            let decoded = image::load_from_memory(&detail).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (width, height));
            assert_eq!(hash, hex_sha256(&detail));
            assert!(detail.len() <= 2 * 1024 * 1024);
            assert_ne!(hash, compress_and_fingerprint(&path).unwrap().1);
            assert_eq!(std::fs::read(&path).unwrap(), source);
        }
        let path = dir.path().join("oversized.png");
        image::RgbImage::new(1401, 2).save(&path).unwrap();
        assert_eq!(compress_profile(&path, true).unwrap_err().code, "DECODE");
    }

    #[tokio::test]
    async fn detail_pixel_evidence_is_verified_with_its_recorded_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("preview.jpg");
        image::RgbImage::from_pixel(1400, 933, image::Rgb([120, 150, 200]))
            .save(&path)
            .unwrap();
        let hash = compress_profile(&path, true).unwrap().1;
        let version = format!("{}@fixture", super::super::vision::DETAIL_PROMPT_VERSION);
        assert_eq!(
            super::super::vision::current_preview_status_for_version(
                Some(&hash),
                path.to_str().unwrap(),
                &version
            )
            .await,
            "verified"
        );
        assert_eq!(
            super::super::vision::current_preview_status(Some(&hash), path.to_str().unwrap()).await,
            "mismatch"
        );
        image::RgbImage::from_pixel(1400, 933, image::Rgb([20, 30, 40]))
            .save(&path)
            .unwrap();
        assert_eq!(
            super::super::vision::current_preview_status_for_version(
                Some(&hash),
                path.to_str().unwrap(),
                &version
            )
            .await,
            "mismatch"
        );
    }

    #[test]
    fn compress_reduces_large_image() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.jpg");
        let img = image::RgbImage::from_pixel(1400, 933, image::Rgb([120, 150, 200]));
        img.save(&path).unwrap();
        let compressed = compress_for_upload(&path).unwrap();
        // 1400px → 512px，体积应显著小于原始
        let orig_len = std::fs::metadata(&path).unwrap().len() as usize;
        assert!(
            compressed.len() < orig_len,
            "压缩后 {} 应小于原始 {}",
            compressed.len(),
            orig_len
        );
        // 验证尺寸
        let decoded = image::load_from_memory(&compressed).unwrap();
        assert!(decoded.width().max(decoded.height()) <= MAX_EDGE);
    }

    #[test]
    fn small_image_passes_through_as_jpeg() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("small.png");
        image::RgbImage::from_pixel(200, 150, image::Rgb([255, 0, 0]))
            .save(&path)
            .unwrap();
        let out = compress_for_upload(&path).unwrap();
        // PNG 200px → 保持 200px 但转 JPEG
        let decoded = image::load_from_memory(&out).unwrap();
        assert_eq!(decoded.width(), 200);
        assert_eq!(decoded.height(), 150);
    }

    #[test]
    fn extreme_aspect_ratio_stays_nonzero_and_source_bytes_are_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        for (width, height) in [(1400, 1), (1, 1400)] {
            let path = dir.path().join(format!("{width}-{height}.png"));
            image::RgbImage::from_pixel(width, height, image::Rgb([10, 20, 30]))
                .save(&path)
                .unwrap();
            let before = std::fs::read(&path).unwrap();
            let out = compress_for_upload(&path).unwrap();
            let decoded = image::load_from_memory(&out).unwrap();
            assert_eq!(decoded.width().max(decoded.height()), 512);
            assert_eq!(decoded.width().min(decoded.height()), 1);
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }

    #[test]
    fn fingerprint_describes_the_bytes_actually_sent_and_moves_with_pixels() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.jpg");
        image::RgbImage::from_pixel(900, 600, image::Rgb([10, 200, 30]))
            .save(&path)
            .unwrap();
        let (bytes, hash) = compress_and_fingerprint(&path).unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.bytes().all(|c| c.is_ascii_hexdigit()));
        // 指纹必须是交出去的那份字节本身，而不是原图或磁盘上的预览。
        assert_eq!(hash, hex_sha256(&bytes));
        assert_eq!(compress_for_upload(&path).unwrap(), bytes);
        assert_eq!(
            compress_and_fingerprint(&path).unwrap(),
            (bytes, hash.clone())
        );
        // 同尺寸不同画面：原图指纹之外的像素变化要能在这份指纹上看见。
        let other = dir.path().join("b.jpg");
        image::RgbImage::from_pixel(900, 600, image::Rgb([200, 20, 30]))
            .save(&other)
            .unwrap();
        assert_ne!(compress_and_fingerprint(&other).unwrap().1, hash);
    }

    #[test]
    fn rejects_oversized_files_and_pixels_before_unbounded_decoding() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.bin");
        std::fs::File::create(&path)
            .unwrap()
            .set_len(MAX_INPUT_BYTES + 1)
            .unwrap();
        assert_eq!(
            compress_for_upload(&path).unwrap_err().code,
            "AI_IMAGE_INVALID"
        );
        let path = dir.path().join("wide.png");
        image::RgbImage::new(1401, 2).save(&path).unwrap();
        assert_eq!(compress_for_upload(&path).unwrap_err().code, "DECODE");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn uploads_landing_on_srgb_declare_it_and_the_fingerprint_covers_the_label() {
        use crate::color_engine::{
            declared_rgb_profile, srgb_cache_profile_bytes, tag_jpeg_with_icc,
        };
        let dir = tempfile::tempdir().unwrap();
        let srgb = srgb_cache_profile_bytes().expect("系统 sRGB 配置缺失时本用例不成立");
        let p3 = std::fs::read("/System/Library/ColorSync/Profiles/Display P3.icc").unwrap();
        let plain_jpeg = {
            let path = dir.path().join("base.jpg");
            image::RgbImage::from_pixel(600, 400, image::Rgb([200, 90, 40]))
                .save(&path)
                .unwrap();
            std::fs::read(&path).unwrap()
        };
        let write = |name: &str, icc: &[u8]| {
            let path = dir.path().join(name);
            std::fs::write(&path, tag_jpeg_with_icc(&plain_jpeg, icc).unwrap()).unwrap();
            path
        };
        let tagged_p3 = write("p3.jpg", &p3);
        let tagged_srgb = write("srgb.jpg", &srgb);
        let untagged = dir.path().join("legacy.jpg");
        std::fs::write(&untagged, &plain_jpeg).unwrap();
        let center = |bytes: &[u8]| -> [u8; 3] {
            let img = image::load_from_memory(bytes).unwrap().to_rgb8();
            let index = (200usize * img.dimensions().0 as usize + 300usize) * 3;
            img.as_raw()[index..index + 3].try_into().unwrap()
        };

        let (bytes, hash) = compress_profile(&tagged_p3, false).unwrap();
        assert_eq!(
            declared_rgb_profile(&bytes).as_deref(),
            Some(srgb.as_slice()),
            "带广色域声明的预览交出去时声明的是真正落到的 sRGB"
        );
        assert_eq!(hash, hex_sha256(&bytes), "观察指纹必须含标签在内的最终字节");
        let legacy = compress_for_upload(&untagged).unwrap();
        assert_eq!(
            declared_rgb_profile(&legacy),
            None,
            "身份核对不出的旧预览不补标签，也不假装管过"
        );
        assert_ne!(
            center(&bytes),
            center(&legacy),
            "同一份数值两种声明，落到模型的画面必须不同"
        );
        // 已经是 sRGB 的那份只做声明，不再管第二次：与无标签那份的像素差应停留在编码噪声级。
        let managed_once = compress_for_upload(&tagged_srgb).unwrap();
        for channel in 0..3 {
            let drift = center(&managed_once)[channel] as i32 - center(&legacy)[channel] as i32;
            assert!(drift.abs() <= 2, "sRGB 声明被二次管理的漂移：{drift}");
        }
        assert_eq!(
            std::fs::read(&tagged_p3).unwrap(),
            tag_jpeg_with_icc(&plain_jpeg, &p3).unwrap(),
            "上传压缩不得改写磁盘上的预览"
        );
    }
}
