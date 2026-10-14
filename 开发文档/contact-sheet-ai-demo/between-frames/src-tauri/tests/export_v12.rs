// v1.2 导出：印刷参考 HTML 与元数据侧车目录。
use between_frames_lib::{db::Store, export, image_pipeline};
use std::path::Path;

async fn setup(path: &Path, count: usize) -> between_frames_lib::domain::Project {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let project = db.create("印刷参考", "克制的日常").await.unwrap();
    for i in 0..count {
        let source = path.join(format!("f{i}.jpg"));
        // 像素差异要足够大：小尺寸纯色图在 JPEG 量化后可能同字节，触发内容去重。
        image::RgbImage::from_pixel(
            64,
            48,
            image::Rgb([
                (i * 67 % 256) as u8,
                (60 + i * 31 % 128) as u8,
                (90 + i * 13 % 128) as u8,
            ]),
        )
        .save(&source)
        .unwrap();
        db.insert_photo(
            &project.id,
            &image_pipeline::prepare(&source, &path.join("cache")).unwrap(),
        )
        .await
        .unwrap();
    }
    {
        use between_frames_lib::command_handler::{CommandHandler, GuiCommandRequest};
        use between_frames_lib::commands_generated::GuiCommand;
        let loaded = db.get(&project.id).await.unwrap();
        let all: Vec<String> = loaded.photos.iter().map(|photo| photo.id.clone()).collect();
        CommandHandler::new(&db)
            .execute_gui(GuiCommandRequest::new(
                &loaded.id,
                loaded.revision,
                GuiCommand::SetPhotoDecision {
                    photo_ids: all,
                    status: "keep".into(),
                },
            ))
            .await
            .unwrap();
    }
    db.get(&project.id).await.unwrap()
}

#[tokio::test]
async fn print_reference_html_has_cover_pages_and_plates() {
    let temp = tempfile::tempdir().unwrap();
    let project = setup(temp.path(), 3).await;
    // 序列放两张：一页两图 + 封面。
    let bytes = export::render_with_options(&project, "print", false).unwrap();
    let html = String::from_utf8(bytes).unwrap();
    assert!(html.contains("印刷参考"));
    assert!(html.contains("克制的日常"));
    assert!(html.contains("@page"));
    assert!(html.contains("data:image/jpeg;base64,"));
    // 三张作品 → 2 个 sheet（首页两图 + 次页一图），共三个 plate。
    assert_eq!(html.matches("class=\"sheet\"").count(), 2);
    assert_eq!(html.matches("class=\"plate\"").count(), 3);
    assert!(html.contains("第 1 页"));
    assert!(html.contains("第 2 页"));
}

#[tokio::test]
async fn sidecar_export_writes_per_photo_json_and_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let project = setup(temp.path(), 3).await;
    let target = temp.path().join("my-sidecar");
    let receipt = export::write_with_options(&project, "sidecar", &target, false).unwrap();
    assert!(target.is_dir());
    let entries: Vec<_> = std::fs::read_dir(&target)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    // 3 张照片 + manifest。
    assert_eq!(entries.len(), 4);
    let manifest_raw = std::fs::read_to_string(target.join("manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_raw).unwrap();
    assert_eq!(manifest["photos"], 3);
    assert_eq!(manifest["files"].as_array().unwrap().len(), 3);
    // 侧车内容：默认无私有路径，含状态与指纹。
    let sidecar_file = manifest["files"][0]["file"].as_str().unwrap();
    let sidecar: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(target.join(sidecar_file)).unwrap()).unwrap();
    assert!(sidecar["sha256"].is_string());
    assert!(sidecar["status"].is_string());
    assert!(sidecar.get("originalPath").is_none());
    // macOS 符号链接（/var → /private/var）：canonicalize 后前缀不同，比较文件名部分。
    assert!(receipt.path.ends_with("my-sidecar"));
    // 重复导出到已存在目录被拒绝；不覆盖用户文件。
    assert!(export::write_with_options(&project, "sidecar", &target, false).is_err());
    // opt-in 路径时包含 originalPath。
    let target2 = temp.path().join("sidecar-paths");
    export::write_with_options(&project, "sidecar", &target2, true).unwrap();
    let with_path: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            target2.join(
                serde_json::from_str::<serde_json::Value>(
                    &std::fs::read_to_string(target2.join("manifest.json")).unwrap(),
                )
                .unwrap()["files"][0]["file"]
                    .as_str()
                    .unwrap(),
            ),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(with_path["originalPath"].is_string());
}
