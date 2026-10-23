//! AI 上传图片压缩：把预览图缩到 512px JPEG 再发送。
//! 目的：减小请求体（多图场景 <500KB），降低模型处理时间和传输成本。
//! 原图只读——压缩的是内存副本，磁盘缓存不动。

use crate::domain::{AppError, Result};

const MAX_EDGE: u32 = 512;
const JPEG_QUALITY: u8 = 75;

/// 读取预览文件并压缩为 ≤512px JPEG（内存操作，不写磁盘）。
/// 输入已经是小图（<512px）时原样返回 JPEG 重编码结果。
pub fn compress_for_upload(path: &std::path::Path) -> Result<Vec<u8>> {
    let data = std::fs::read(path)
        .map_err(|e| AppError::new("RESOURCE_MISSING", format!("预览缓存缺失：{e}")))?;
    let img = image::load_from_memory(&data)
        .map_err(|e| AppError::new("DECODE", format!("预览解码失败：{e}")))?;
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let scale = if w.max(h) > MAX_EDGE {
        f64::from(MAX_EDGE) / f64::from(w.max(h))
    } else {
        1.0
    };
    let (nw, nh) = if scale < 1.0 {
        (
            (f64::from(w) * scale).round() as u32,
            (f64::from(h) * scale).round() as u32,
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
}
