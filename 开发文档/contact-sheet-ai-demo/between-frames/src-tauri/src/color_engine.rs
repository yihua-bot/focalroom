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
    Ok(Some(SourceProfile {
        digest: format!("{:x}", Sha256::digest(&icc)),
        description: if valid {
            parse_icc_description(&icc)
        } else {
            "invalid".to_owned()
        },
        valid,
    }))
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

fn extract_embedded_icc<R: Read + Seek>(reader: &mut R, file_len: u64) -> Option<Vec<u8>> {
    let header = read_at(reader, file_len, 0, 8)?;
    if header.starts_with(&[0xff, 0xd8]) {
        extract_jpeg_icc(reader, file_len)
    } else if header == b"\x89PNG\r\n\x1a\n" {
        extract_png_icc(reader, file_len)
    } else if header.starts_with(b"II*\0") || header.starts_with(b"MM\0*") {
        extract_tiff_icc(reader, file_len, &header)
    } else {
        // BigTIFF、HEIF 和相机 RAW 的 profile 需由其专用解码器处理。
        None
    }
}

fn extract_jpeg_icc<R: Read + Seek>(reader: &mut R, file_len: u64) -> Option<Vec<u8>> {
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

fn extract_png_icc<R: Read + Seek>(reader: &mut R, file_len: u64) -> Option<Vec<u8>> {
    let mut position = 8u64;
    for _ in 0..MAX_METADATA_RECORDS {
        let header = read_at(reader, file_len, position, 8)?;
        let length = be32(&header, 0)?;
        let end = position.checked_add(12 + length as u64)?;
        if end > file_len {
            return None;
        }
        if &header[4..8] == b"iCCP" {
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
}
