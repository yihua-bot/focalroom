//! G2 Color Engine：ICC profile 读取、色彩管线的四路分离与资产域绑定。
//!
//! v3 目标：不再共享「8-bit 1400px JPEG」作为唯一像素来源。四条管线各有身份：
//! - thumbnail: 320px 快速浏览
//! - display_proxy: 1400px 屏幕检视（当前实现）
//! - ai_preview: 模型观察用（成本敏感，尺寸/质量可配）
//! - print_master: 印刷母版（高位深、ICC 保留、不可降格）
//!
//! 当前阶段：profile 检测 + digest 落库 + 管线类型枚举。高位深渲染和 ColorSync
//! 集成需要原生 Metal/CoreImage 层，按 v3 蓝图 §10.2 的技术验证关口决定。

use crate::domain::{AppError, Result};
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
}

/// 从图像文件中提取嵌入的 ICC profile 并计算 digest。
/// 支持 JPEG APP2 ICC_PROFILE、PNG iCCP chunk、TIFF tag 34675。
pub fn read_source_profile(path: &Path) -> Result<Option<SourceProfile>> {
    let bytes =
        std::fs::read(path).map_err(|e| AppError::new("FILE_IO", format!("读取失败：{e}")))?;

    let icc_bytes = match extract_embedded_icc(&bytes) {
        Some(icc) => icc,
        None => return Ok(None), // 无嵌入 profile 不视为错误
    };

    let digest = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&icc_bytes);
        let result: [u8; 32] = hasher.finalize().into();
        result
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let description = parse_icc_description(&icc_bytes);
    let valid = icc_bytes.len() >= 128 && icc_bytes.len() <= 32 * 1024 * 1024;

    Ok(Some(SourceProfile {
        digest,
        description,
        valid,
    }))
}

/// 从 JPEG/PNG/TIFF 字节中提取嵌入的 ICC profile 原始字节。
fn extract_embedded_icc(bytes: &[u8]) -> Option<Vec<u8>> {
    // JPEG: APP2 marker (0xFFE2) with "ICC_PROFILE\0"
    if bytes.starts_with(&[0xFF, 0xD8]) {
        return extract_jpeg_icc(bytes);
    }
    // PNG: iCCP chunk
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return extract_png_icc(bytes);
    }
    // TIFF: tag 34675 (0x8773) — 简化：在前 64KB 内搜索 ICC signature
    if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        return extract_tiff_icc(bytes);
    }
    None
}

fn extract_jpeg_icc(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 2;
    let mut icc = Vec::new();
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            break;
        }
        let marker = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]);
        if marker == 0xFFE2 {
            // APP2
            let len = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
            let data_start = pos + 4;
            let data_end = (data_start + len - 2).min(bytes.len());
            if data_end > data_start + 12 && &bytes[data_start..data_start + 12] == b"ICC_PROFILE\0"
            {
                // 跳过 seq/total (2 bytes)
                let chunk = &bytes[data_start + 14..data_end];
                icc.extend_from_slice(chunk);
            }
            pos = data_end;
        } else if marker == 0xFFDA {
            break; // SOS — 后面是压缩数据
        } else if (0xFFD0..=0xFFD7).contains(&marker) {
            pos += 2; // 无长度的 standalone marker
        } else {
            let len = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
            pos += 2 + len;
        }
    }
    if icc.is_empty() { None } else { Some(icc) }
}

fn extract_png_icc(bytes: &[u8]) -> Option<Vec<u8>> {
    // 简化：搜索 iCCP chunk
    let search = b"iCCP";
    let mut pos = 8;
    while pos + 12 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
            as usize;
        let chunk_type = &bytes[pos + 4..pos + 8];
        if chunk_type == search {
            // iCCP: profile_name\0 compression_method compressed_data
            let data = &bytes[pos + 8..(pos + 8 + len).min(bytes.len())];
            // 找到 null 后跳过 1 字节压缩方式，剩余为 zlib 压缩的 ICC
            if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                let compressed = &data[null_pos + 2..];
                // 解压 zlib
                use std::io::Read;
                let mut decoder = flate2::read::ZlibDecoder::new(compressed);
                let mut icc = Vec::new();
                if decoder.read_to_end(&mut icc).is_ok() && !icc.is_empty() {
                    return Some(icc);
                }
            }
        }
        pos += 12 + len;
    }
    None
}

fn extract_tiff_icc(bytes: &[u8]) -> Option<Vec<u8>> {
    // 简化搜索：TIFF ICC tag 的值通常在文件内某处，搜索 ICC header signature "acsp"
    // 这不是精确解析——生产实现需要正确的 IFD 遍历
    if let Some(pos) = find_subslice(bytes, b"acsp").filter(|&p| p >= 128) {
        let start = pos - 36;
        let size = u32::from_be_bytes([
            bytes[start],
            bytes[start + 1],
            bytes[start + 2],
            bytes[start + 3],
        ]);
        if size > 0 && start + size as usize <= bytes.len() {
            return Some(bytes[start..start + size as usize].to_vec());
        }
    }
    None
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// 解析 ICC profile 的 description tag（tag signature 'desc' = 0x64657363）。
fn parse_icc_description(icc: &[u8]) -> String {
    if icc.len() < 128 {
        return "invalid".to_owned();
    }
    let tag_count = u32::from_be_bytes([icc[128], icc[129], icc[130], icc[131]]) as usize;
    for i in 0..tag_count.min(64) {
        let offset = 132 + i * 12;
        if offset + 12 > icc.len() {
            break;
        }
        let tag = &icc[offset..offset + 4];
        if tag == b"desc" {
            let pos = u32::from_be_bytes([
                icc[offset + 4],
                icc[offset + 5],
                icc[offset + 6],
                icc[offset + 7],
            ]) as usize;
            let len = u32::from_be_bytes([
                icc[offset + 8],
                icc[offset + 9],
                icc[offset + 10],
                icc[offset + 11],
            ]) as usize;
            if pos + len <= icc.len() && len > 4 {
                // 'desc' tag: type(4) reserved(4) then ASCII string
                let str_start = pos + 8;
                let str_len = (len - 12).min(128);
                if str_start + str_len <= icc.len() {
                    let desc = &icc[str_start..str_start + str_len];
                    let end = desc.iter().position(|&b| b == 0).unwrap_or(str_len);
                    return String::from_utf8_lossy(&desc[..end]).to_string();
                }
            }
        }
    }
    "unknown".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_without_icc_returns_none() {
        // 最小 JPEG header（无 APP2）
        let bytes = [0xFF, 0xD8, 0xFF, 0xDA, 0x00, 0x00];
        assert!(extract_embedded_icc(&bytes).is_none());
    }

    #[test]
    fn icc_description_parses() {
        // 构造最小 ICC 结构：128 byte header + tag table with 'desc'
        let mut icc = vec![0u8; 200];
        icc[36..40].copy_from_slice(b"acsp"); // signature
        // tag count = 1
        icc[128..132].copy_from_slice(&1u32.to_be_bytes());
        // tag 0: 'desc' at offset 144, size 32
        icc[132..136].copy_from_slice(b"desc");
        icc[136..140].copy_from_slice(&144u32.to_be_bytes());
        icc[140..144].copy_from_slice(&32u32.to_be_bytes());
        // at 144: type='desc' + reserved + string
        icc[144..148].copy_from_slice(b"desc");
        // string starts at 152: "Test Profile\0"
        let name = b"Test Profile\0";
        icc[152..152 + name.len()].copy_from_slice(name);
        assert_eq!(parse_icc_description(&icc), "Test Profile");
    }

    #[test]
    fn tiff_icc_finds_acsp() {
        // 构造简单 TIFF with embedded ICC signature
        let mut bytes = vec![0x49, 0x49, 0x2A, 0x00]; // TIFF LE header
        bytes.extend_from_slice(&[0u8; 100]); // padding
        // 模拟 ICC data at offset ~100
        let icc_offset = 100;
        let size = 200u32;
        bytes.resize(icc_offset + size as usize, 0);
        bytes[icc_offset..icc_offset + 4].copy_from_slice(&size.to_be_bytes());
        bytes[icc_offset + 36..icc_offset + 40].copy_from_slice(b"acsp");
        let result = extract_tiff_icc(&bytes);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), size as usize);
    }
}
