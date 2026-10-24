//! AI 上传图片压缩：把预览图缩到 512px JPEG 再发送。
//! 目的：减小 AI 派生图；整份请求大小仍取决于图像数量、内容与文本。
//! 原图只读——压缩的是内存副本，磁盘缓存不动。

use crate::domain::{AppError, Result};
use std::io::Read;

const MAX_EDGE: u32 = 512;
const JPEG_QUALITY: u8 = 75;
const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

/// 读取预览文件并压缩为 ≤512px JPEG（内存操作，不写磁盘）。
/// 输入已经是小图（<512px）时原样返回 JPEG 重编码结果。
pub fn compress_for_upload(path: &std::path::Path) -> Result<Vec<u8>> {
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
    let mut reader = image::ImageReader::new(std::io::Cursor::new(data))
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
    let scale = if w.max(h) > MAX_EDGE {
        f64::from(MAX_EDGE) / f64::from(w.max(h))
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
    let resized = if (nw, nh) != (w, h) {
        image::imageops::resize(&rgb, nw, nh, image::imageops::FilterType::Lanczos3)
    } else {
        rgb
    };
    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
    image::DynamicImage::ImageRgb8(resized)
        .write_with_encoder(encoder)
        .map_err(|e| AppError::new("ENCODE", format!("JPEG 编码失败：{e}")))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
