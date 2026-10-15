//! 只读提取 CR2 IFD0 的 JPEG；不遍历 RAW IFD、不运行解码进程或创建持久缓存。
use image::{ImageDecoder, ImageReader};
use std::{
    collections::HashMap,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    path::Path,
};

const MAX_JPEG_BYTES: u64 = 4 * 1024 * 1024;
const MAX_PIXELS: u64 = 16_000_000;

/// 仅接受已校验原图中可独立显示的 IFD0 JPEG；任何不支持/损坏布局回退派生预览。
pub(crate) fn extract(path: &Path) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    if length > 512 * 1024 * 1024 {
        return None;
    }
    let mut header = [0_u8; 16];
    file.read_exact(&mut header).ok()?;
    // CR2 v2 的小端 TIFF 头；不把改名 TIFF、DNG 或 CR3 当作 CR2。
    if &header[..4] != b"II\x2a\0" || &header[8..12] != b"CR\x02\0" {
        return None;
    }
    let ifd = u32::from_le_bytes(header[4..8].try_into().ok()?) as u64;
    if ifd < 16 || ifd.checked_add(2)? > length {
        return None;
    }
    file.seek(SeekFrom::Start(ifd)).ok()?;
    let mut count = [0_u8; 2];
    file.read_exact(&mut count).ok()?;
    let count = u16::from_le_bytes(count) as usize;
    if count == 0 || count > 128 || ifd.checked_add(2 + count as u64 * 12 + 4)? > length {
        return None;
    }
    let mut fields = HashMap::new();
    for _ in 0..count {
        let mut entry = [0_u8; 12];
        file.read_exact(&mut entry).ok()?;
        let tag = u16::from_le_bytes(entry[..2].try_into().ok()?);
        if !matches!(tag, 0x100 | 0x101 | 0x103 | 0x111 | 0x112 | 0x117) {
            continue;
        }
        let field_type = u16::from_le_bytes(entry[2..4].try_into().ok()?);
        let values = u32::from_le_bytes(entry[4..8].try_into().ok()?);
        // 只支持单条连续 JPEG，拒绝数组、重复字段、私有偏移和复杂条带布局。
        if values != 1 || !matches!(field_type, 3 | 4) || fields.contains_key(&tag) {
            return None;
        }
        let value = if field_type == 3 {
            u16::from_le_bytes(entry[8..10].try_into().ok()?) as u32
        } else {
            u32::from_le_bytes(entry[8..12].try_into().ok()?)
        };
        fields.insert(tag, value);
    }
    if fields.get(&0x103) != Some(&6) {
        return None;
    }
    let width = *fields.get(&0x100)?;
    let height = *fields.get(&0x101)?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return None;
    }
    let orientation = fields.get(&0x112).copied().unwrap_or(1);
    if !(1..=8).contains(&orientation) {
        return None;
    }
    let offset = u64::from(*fields.get(&0x111)?);
    let bytes = u64::from(*fields.get(&0x117)?);
    if !(4..=MAX_JPEG_BYTES).contains(&bytes)
        || offset < ifd + 2 + count as u64 * 12 + 4
        || offset.checked_add(bytes)? > length
    {
        return None;
    }
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut jpeg = vec![0_u8; bytes as usize];
    file.read_exact(&mut jpeg).ok()?;
    if !jpeg.starts_with(b"\xff\xd8") || !jpeg.ends_with(b"\xff\xd9") {
        return None;
    }
    let mut reader = ImageReader::with_format(Cursor::new(&jpeg), image::ImageFormat::Jpeg);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(64 * 1024 * 1024);
    limits.max_image_width = Some(16_000);
    limits.max_image_height = Some(16_000);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().ok()?;
    if decoder.dimensions() != (width, height) || decoder.total_bytes() > 64 * 1024 * 1024 {
        return None;
    }
    let jpeg_orientation = decoder.orientation().ok()?.to_exif() as u32;
    let has_exif = decoder.exif_metadata().ok()?.is_some();
    drop(decoder);
    if has_exif && jpeg_orientation != orientation {
        // 元数据冲突时不猜测方向，继续使用导入阶段已经校正的派生预览。
        return None;
    }
    if !has_exif && orientation != 1 {
        // 只增加最小 EXIF 方向，不重编码 JPEG 像素或丢弃原有 ICC 等段。
        let mut app1 =
            b"\xff\xe1\0\x22Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0".to_vec();
        app1.extend((orientation as u16).to_le_bytes());
        app1.extend([0; 6]);
        jpeg.splice(2..2, app1);
    }
    Some(jpeg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg() -> Vec<u8> {
        let bitmap = image::RgbImage::from_fn(1800, 1000, |x, y| {
            image::Rgb(match (x < 900, y < 500) {
                (true, true) => [230, 20, 20],
                (false, true) => [20, 230, 20],
                (true, false) => [20, 20, 230],
                (false, false) => [230, 230, 20],
            })
        });
        let mut bytes = Cursor::new(Vec::new());
        bitmap
            .write_to(&mut bytes, image::ImageFormat::Jpeg)
            .unwrap();
        bytes.into_inner()
    }

    fn cr2(jpeg: &[u8], orientation: u32) -> Vec<u8> {
        let mut bytes = b"II\x2a\0\x10\0\0\0CR\x02\0\0\0\0\0".to_vec();
        bytes.extend(6_u16.to_le_bytes());
        for (tag, value) in [
            (0x100_u16, 1800),
            (0x101, 1000),
            (0x103, 6),
            (0x111, 94),
            (0x112, orientation),
            (0x117, jpeg.len() as u32),
        ] {
            bytes.extend(tag.to_le_bytes());
            bytes.extend(4_u16.to_le_bytes());
            bytes.extend(1_u32.to_le_bytes());
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend(0_u32.to_le_bytes());
        bytes.extend(jpeg);
        bytes
    }

    #[test]
    fn extracts_only_declared_primary_jpeg_without_changing_source_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("sample.CR2");
        let jpeg = jpeg();
        let bytes = cr2(&jpeg, 1);
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(extract(&path).unwrap(), jpeg);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    fn rejects_invalid_ranges_counts_duplicate_tags_and_pixel_limits() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("malformed.CR2");
        let valid = cr2(&jpeg(), 1);
        // 字段值偏移：IFD0 起点 16 + count 2 + entry*12 + value 8。
        for (offset, value) in [
            (4, u32::MAX),
            (26, 0),
            (26, 16001),
            (38, 16000),
            (50, 1),
            (62, 10),
            (62, u32::MAX),
            (74, 9),
            (86, u32::MAX),
            (86, MAX_JPEG_BYTES as u32 + 1),
            (18 + 3 * 12 + 4, u32::MAX),
        ] {
            let mut bytes = valid.clone();
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            std::fs::write(&path, bytes).unwrap();
            assert!(
                extract(&path).is_none(),
                "未拒绝 offset={offset}, value={value}"
            );
        }
        let mut duplicate = valid.clone();
        duplicate[18 + 12..18 + 14].copy_from_slice(&0x100_u16.to_le_bytes());
        std::fs::write(&path, duplicate).unwrap();
        assert!(extract(&path).is_none());
        for end in [0, 15, 17, 93, valid.len() - 1] {
            std::fs::write(&path, &valid[..end]).unwrap();
            assert!(extract(&path).is_none(), "截断长度 {end} 应拒绝");
        }
        let mut wrong_header = valid;
        wrong_header[8] = b'X';
        std::fs::write(&path, wrong_header).unwrap();
        assert!(extract(&path).is_none());
    }

    #[test]
    fn rejects_non_jpeg_payload_and_metadata_dimension_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.CR2");
        std::fs::write(&path, cr2(b"\xff\xd8not-a-jpeg\xff\xd9", 1)).unwrap();
        assert!(extract(&path).is_none());
        let mut bytes = cr2(&jpeg(), 1);
        bytes[26..30].copy_from_slice(&1700_u32.to_le_bytes());
        std::fs::write(&path, bytes).unwrap();
        assert!(extract(&path).is_none());
    }

    #[test]
    fn carries_all_eight_orientations_and_preserves_pixel_and_icc_segments() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("rotated.CR2");
        let mut jpeg = jpeg();
        // 这是字节保留标记，不冒充有效 ICC profile 或显示器色彩验收。
        jpeg.splice(
            2..2,
            b"\xff\xe2\0\x12ICC_PROFILE\0\x01\x01ok".iter().copied(),
        );
        let expected = [
            [0, 1, 2, 3],
            [1, 0, 3, 2],
            [3, 2, 1, 0],
            [2, 3, 0, 1],
            [0, 2, 1, 3],
            [2, 0, 3, 1],
            [3, 1, 2, 0],
            [1, 3, 0, 2],
        ];
        let colors = [
            [230_i16, 20, 20],
            [20, 230, 20],
            [20, 20, 230],
            [230, 230, 20],
        ];
        for orientation in 1..=8 {
            std::fs::write(&path, cr2(&jpeg, orientation)).unwrap();
            let extracted = extract(&path).unwrap();
            assert!(extracted.windows(11).any(|window| window == b"ICC_PROFILE"));
            assert!(extracted.ends_with(&jpeg[2..]), "像素/ICC 段不能重编码");
            let source = temp.path().join(format!("orientation-{orientation}.jpg"));
            std::fs::write(&source, &extracted).unwrap();
            let prepared =
                crate::image_pipeline::prepare(&source, &temp.path().join("cache")).unwrap();
            let expected_size = if orientation <= 4 {
                (1800, 1000)
            } else {
                (1000, 1800)
            };
            assert_eq!((prepared.width, prepared.height), expected_size);
            let preview = image::open(prepared.preview).unwrap().to_rgb8();
            let corners = [
                (5, 5),
                (preview.width() - 6, 5),
                (5, preview.height() - 6),
                (preview.width() - 6, preview.height() - 6),
            ];
            for ((x, y), expected_color) in
                corners.into_iter().zip(expected[orientation as usize - 1])
            {
                let pixel = preview.get_pixel(x, y);
                for channel in 0..3 {
                    assert!(
                        (pixel[channel] as i16 - colors[expected_color][channel]).abs() < 12,
                        "方向 {orientation} 的角落颜色不正确"
                    );
                }
            }
        }
    }

    #[test]
    fn conflicting_existing_exif_falls_back_instead_of_guessing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("conflict.CR2");
        std::fs::write(&path, cr2(&jpeg(), 6)).unwrap();
        let oriented = extract(&path).unwrap();
        std::fs::write(&path, cr2(&oriented, 6)).unwrap();
        assert_eq!(extract(&path).unwrap(), oriented, "同方向不重复插入 EXIF");
        std::fs::write(&path, cr2(&oriented, 1)).unwrap();
        assert!(extract(&path).is_none());
    }
}
