// 仅供 integration-harness：生成有来源声明、格式/尺寸可核对的临时文件。
use serde_json::{Value, json};
use std::path::Path;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn prepare(root: &Path, profile: &str) -> Result<()> {
    if !matches!(profile, "standard" | "m1-500") {
        return Err("未知测试素材配置".into());
    }
    if std::fs::read_dir(root.join("fixtures"))?.next().is_some() {
        return Ok(());
    }
    let count = if profile == "m1-500" { 500 } else { 200 };
    let samples: Vec<_> = (1..=28)
        .map(|index| {
            let source = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../assets/frame_{index:02}.jpg"));
            image::open(source).map(|image| {
                if profile == "m1-500" {
                    image
                        .resize_exact(1200, 800, image::imageops::FilterType::Triangle)
                        .to_rgb8()
                } else {
                    image.thumbnail(480, 480).to_rgb8()
                }
            })
        })
        .collect::<std::result::Result<_, _>>()?;
    for index in 0..count {
        let png = profile == "m1-500" && index % 2 == 0;
        let extension = if png { "png" } else { "jpg" };
        let path = root
            .join("fixtures")
            .join(format!("frame-{index:03}.{extension}"));
        let mut photo = samples[index % samples.len()].clone();
        photo.put_pixel(
            0,
            0,
            image::Rgb([
                (index % 256) as u8,
                (index / 256) as u8,
                (index * 13 % 256) as u8,
            ]),
        );
        photo.save(&path)?;
        if !png {
            // JPEG 会抹掉单像素变化；COM 段保证跨 256 个索引仍为唯一内容。
            let bytes = std::fs::read(&path)?;
            let comment = format!("Between Frames automated fixture {index:03}");
            let mut unique = bytes[..2].to_vec();
            unique.extend([0xff, 0xfe]);
            unique.extend(((comment.len() + 2) as u16).to_be_bytes());
            unique.extend(comment.as_bytes());
            unique.extend(&bytes[2..]);
            std::fs::write(path, unique)?;
        }
    }
    if profile == "m1-500" {
        let raw = root.join("fixture-raw");
        std::fs::create_dir_all(&raw)?;
        // 改名格式边界图，用于 GUI 派生预览标识；不冒充真实相机 RAW。
        std::fs::copy(root.join("fixtures/frame-000.png"), raw.join("derived.dng"))?;
        let inspection = root.join("fixture-inspection");
        std::fs::create_dir_all(&inspection)?;
        let large = image::RgbImage::from_fn(2400, 1600, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
        });
        large.save(inspection.join("detail.png"))?;
        large.save(inspection.join("detail.jpg"))?;
        let failures = root.join("fixture-failures");
        std::fs::create_dir_all(&failures)?;
        std::fs::write(failures.join("broken.jpg"), b"not a JPEG")?;
        std::fs::write(
            failures.join("encoding.png"),
            b"\x89PNG\r\n\x1a\ninvalid IHDR",
        )?;
        std::fs::File::create(failures.join("oversized.jpg"))?.set_len(600 * 1024 * 1024)?;
        std::fs::copy(
            root.join("fixtures/frame-000.png"),
            failures.join("duplicate.png"),
        )?;
    }
    let mut bytes = 0_u64;
    let mut jpeg = 0;
    let mut png = 0;
    for file in std::fs::read_dir(root.join("fixtures"))? {
        let file = file?;
        bytes += file.metadata()?.len();
        if file
            .path()
            .extension()
            .is_some_and(|extension| extension == "png")
        {
            png += 1;
        } else {
            jpeg += 1;
        }
    }
    std::fs::write(
        root.join("fixture-info.json"),
        serde_json::to_vec_pretty(&json!({
            "profile":profile,"count":count,"jpeg":jpeg,"png":png,"bytes":bytes,
            "dimensions":if profile == "m1-500" {"1200x800"} else {"within 480x480"},
            "provenance":"28 repository reference images resized with unique content markers; generated test corpus, not 500 independent photographer originals"
        }))?,
    )?;
    Ok(())
}
pub fn info(root: &Path) -> between_frames_lib::domain::Result<Value> {
    let mut value: Value = serde_json::from_slice(&std::fs::read(root.join("fixture-info.json"))?)
        .map_err(|error| {
            between_frames_lib::domain::AppError::new("TEST_FIXTURE", error.to_string())
        })?;
    value["harnessPid"] = json!(std::process::id());
    Ok(value)
}
