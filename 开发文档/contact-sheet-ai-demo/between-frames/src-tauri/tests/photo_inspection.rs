use between_frames_lib::{
    db::Store,
    image_pipeline,
    importer::ImportManager,
    photo_inspection::{compact_project, display_managed_inspection, original_path},
};

async fn setup(
    extension: &str,
) -> (
    tempfile::TempDir,
    Store,
    between_frames_lib::domain::Project,
) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join(format!("original.{extension}"));
    image::RgbImage::from_pixel(2400, 1600, image::Rgb([71, 91, 113]))
        .save(&source)
        .unwrap();
    let store = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = store.create("原图检视", "").await.unwrap();
    let imports = ImportManager::default();
    let job = imports
        .start(
            store.clone(),
            temp.path().join("cache"),
            project.id.clone(),
            vec![source],
            |_| {},
        )
        .await
        .unwrap();
    loop {
        let progress = imports.get(&job).await.unwrap();
        if progress.status != "running" {
            assert!(progress.failures.is_empty());
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let project = store.get(&project.id).await.unwrap();
    (temp, store, project)
}

#[tokio::test]
async fn inspect_original_is_owned_read_only_and_larger_than_preview() {
    let (_temp, store, project) = setup("png").await;
    let photo = &project.photos[0];
    let source = original_path(&store, &project.id, &photo.id).await.unwrap();
    assert_eq!(source.kind(), "original");
    let path = source.path().unwrap();
    assert_eq!(image::image_dimensions(path).unwrap(), (2400, 1600));
    assert_eq!(
        image::image_dimensions(&photo.preview_path).unwrap().0,
        1400
    );
    assert_eq!(
        image_pipeline::fingerprint(path).unwrap(),
        photo.content_sha256
    );
    assert_eq!(
        original_path(&store, "other-project", &photo.id)
            .await
            .unwrap_err()
            .code,
        "NOT_FOUND"
    );
    assert_eq!(
        original_path(&store, &project.id, "other-photo")
            .await
            .unwrap_err()
            .code,
        "NOT_FOUND"
    );
    store.archive(&project.id, project.revision).await.unwrap();
    assert_eq!(
        original_path(&store, &project.id, &photo.id)
            .await
            .unwrap_err()
            .code,
        "NOT_FOUND"
    );
}

#[tokio::test]
async fn changed_missing_and_unsupported_originals_do_not_masquerade_as_full_resolution() {
    let (_temp, store, project) = setup("jpg").await;
    let photo = &project.photos[0];
    std::fs::write(&photo.original_path, b"changed").unwrap();
    assert_eq!(
        original_path(&store, &project.id, &photo.id)
            .await
            .unwrap_err()
            .code,
        "FILE_CHANGED"
    );
    let moved = format!("{}.moved", photo.original_path);
    std::fs::rename(&photo.original_path, moved).unwrap();
    assert_eq!(
        original_path(&store, &project.id, &photo.id)
            .await
            .unwrap_err()
            .code,
        "ORIGINAL_MISSING"
    );
    // TIFF 自本轮起支持直通（B 项交付）；不再作为不支持样例。
    let (_temp2, store2, project2) = setup("tiff").await;
    let tiff_source = original_path(&store2, &project2.id, &project2.photos[0].id)
        .await
        .unwrap();
    assert_eq!(tiff_source.kind(), "original");
    sqlx::query("UPDATE photos SET width=40000,height=40000 WHERE id=?")
        .bind(&project2.photos[0].id)
        .execute(&store2.pool)
        .await
        .unwrap();
    assert_eq!(
        original_path(&store2, &project2.id, &project2.photos[0].id)
            .await
            .unwrap_err()
            .code,
        "INSPECTION_LIMIT"
    );
}

#[tokio::test]
async fn compact_gui_projection_preserves_audit_exports_and_idempotent_receipts() {
    use between_frames_lib::{
        command_handler::{CommandHandler, GuiCommandRequest},
        commands_generated::GuiCommand,
    };
    let (_temp, store, project) = setup("png").await;
    let request = GuiCommandRequest::new(
        &project.id,
        project.revision,
        GuiCommand::SetPhotoDecision {
            photo_ids: vec![project.photos[0].id.clone()],
            status: "keep".into(),
        },
    );
    let handler = CommandHandler::new(&store);
    let receipt = handler.execute_gui(request.clone()).await.unwrap();
    let full = receipt.project.clone().unwrap();
    let full_bytes = serde_json::to_vec(&full).unwrap();
    let compact = compact_project(full.clone());
    assert_eq!(compact.sequence, full.sequence);
    assert_eq!(compact.history.undo_group_id, full.history.undo_group_id);
    assert_eq!(compact.events.len(), full.events.len());
    assert!(
        compact
            .events
            .iter()
            .all(|event| event.previous_json.is_none()
                && event.next_json.is_none()
                && event.context_json == "{}")
    );
    assert!(serde_json::to_vec(&compact).unwrap().len() < full_bytes.len());
    assert_eq!(
        serde_json::to_vec(&store.get(&project.id).await.unwrap()).unwrap(),
        full_bytes
    );
    assert_eq!(
        serde_json::to_value(handler.execute_gui(request).await.unwrap()).unwrap(),
        serde_json::to_value(receipt).unwrap()
    );
}

#[tokio::test]
async fn tiff_original_is_direct_and_read_only() {
    let (temp, store, project) = setup("tiff").await;
    let photo = &project.photos[0];
    let source = original_path(&store, &project.id, &photo.id).await.unwrap();
    assert_eq!(source.kind(), "original");
    let before = image_pipeline::fingerprint(std::path::Path::new(&photo.original_path)).unwrap();
    assert_eq!(before, photo.content_sha256);
    let _ = temp;
}

#[tokio::test]
#[ignore = "requires explicit HEIC fixture at /tmp/between-frames-heic-fixture.heic"]
async fn heic_original_passes_brand_check_when_fixture_exists() {
    let fixture = std::path::Path::new("/tmp/between-frames-heic-fixture.heic");
    assert!(fixture.is_file(), "请准备 HEIC 样本后显式运行此项测试");
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("camera.heic");
    std::fs::copy(fixture, &source).unwrap();
    let store = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = store.create("HEIC", "").await.unwrap();
    let imports = ImportManager::default();
    let job = imports
        .start(
            store.clone(),
            temp.path().join("cache"),
            project.id.clone(),
            vec![source],
            |_| {},
        )
        .await
        .unwrap();
    while imports.get(&job).await.unwrap().status == "running" {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let project = store.get(&project.id).await.unwrap();
    let photo = &project.photos[0];
    let inspected = original_path(&store, &project.id, &photo.id).await.unwrap();
    assert_eq!(
        inspected.kind(),
        "original",
        "HEIC 应直通 WKWebView 原生渲染"
    );
    let before = image_pipeline::fingerprint(std::path::Path::new(&photo.original_path)).unwrap();
    assert_eq!(before, photo.content_sha256, "原 HEIC 不被修改");
}

#[tokio::test]
#[ignore = "requires explicit CR2 fixture at /tmp/between-frames-raw-fixture.CR2"]
async fn raw_cr2_returns_bounded_embedded_jpeg_not_full_raw() {
    let fixture = std::path::Path::new("/tmp/between-frames-raw-fixture.CR2");
    assert!(fixture.is_file(), "请准备 CR2 样本后显式运行此项测试");
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("shooting.CR2");
    std::fs::copy(fixture, &source).unwrap();
    let raw_hash = image_pipeline::fingerprint(&source).unwrap();
    let store = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = store.create("RAW", "").await.unwrap();
    let imports = ImportManager::default();
    let job = imports
        .start(
            store.clone(),
            temp.path().join("cache"),
            project.id.clone(),
            vec![source],
            |_| {},
        )
        .await
        .unwrap();
    while imports.get(&job).await.unwrap().status == "running" {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let project = store.get(&project.id).await.unwrap();
    let photo = &project.photos[0];
    let inspected = original_path(&store, &project.id, &photo.id).await.unwrap();
    assert_eq!(
        inspected.kind(),
        "embedded_preview",
        "CR2 IFD0 JPEG 应与派生缓存区分"
    );
    assert!(inspected.path().is_none(), "内嵌 JPEG 不授权整个 RAW 文件");
    let encoded = serde_json::to_value(&inspected).unwrap();
    use base64::Engine;
    let jpeg = base64::engine::general_purpose::STANDARD
        .decode(
            encoded["dataUrl"]
                .as_str()
                .unwrap()
                .strip_prefix("data:image/jpeg;base64,")
                .unwrap(),
        )
        .unwrap();
    let decoded = image::load_from_memory(&jpeg).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (1936, 1288));
    assert!(jpeg.len() <= 4 * 1024 * 1024 + 36);
    // 二次检视按需提取相同字节，不运行 RAW 转换或创建检视缓存。
    let again = original_path(&store, &project.id, &photo.id).await.unwrap();
    assert_eq!(serde_json::to_value(&again).unwrap(), encoded);
    // 原 RAW 不被修改。
    assert_eq!(
        raw_hash,
        image_pipeline::fingerprint(std::path::Path::new(&photo.original_path)).unwrap()
    );
}

#[tokio::test]
async fn fake_raw_and_unsupported_extensions_fail_with_clear_errors() {
    let temp = tempfile::tempdir().unwrap();
    let fake = temp.path().join("fake.CR2");
    std::fs::write(&fake, b"this is not a raw file at all").unwrap();
    let store = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = store.create("伪造", "").await.unwrap();
    let imports = ImportManager::default();
    let job = imports
        .start(
            store.clone(),
            temp.path().join("cache"),
            project.id.clone(),
            vec![fake],
            |_| {},
        )
        .await
        .unwrap();
    while imports.get(&job).await.unwrap().status == "running" {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    // 导入本身会失败（sips 解码不了）——伪造 RAW 不入库。
    let progress = imports.get(&job).await.unwrap();
    assert!(!progress.failures.is_empty(), "伪造 RAW 应在导入即失败");
    let project = store.get(&project.id).await.unwrap();
    assert!(project.photos.is_empty());
}

#[tokio::test]
async fn png_16bit_quantization_preserves_original_bytes() {
    // C 项色彩边界（ADR 0027）：不冒充通过，锁定已知行为供产品与用户理解。
    let temp = tempfile::tempdir().unwrap();
    // 1) 16 位 PNG：管线全流程可走（解码→缩略/预览→入库→检视），缓存降为 8 位。
    let png16 = temp.path().join("deep.png");
    make_png16(&png16, 400, 300);
    let store = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = store.create("色彩", "").await.unwrap();
    let imports = ImportManager::default();
    let job = imports
        .start(
            store.clone(),
            temp.path().join("cache"),
            project.id.clone(),
            vec![png16],
            |_| {},
        )
        .await
        .unwrap();
    while imports.get(&job).await.unwrap().status == "running" {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let progress = imports.get(&job).await.unwrap();
    assert!(
        progress.failures.is_empty(),
        "16 位 PNG 导入应成功：{:?}",
        progress.failures.first().map(|f| &f.message)
    );
    let project = store.get(&project.id).await.unwrap();
    assert_eq!(project.photos[0].width, 400, "16 位 PNG 元数据尺寸正确");
    // 缓存实际为 8 位 JPEG。
    let thumb = image::open(&project.photos[0].thumbnail_path).unwrap();
    assert_eq!(
        thumb.color(),
        image::ColorType::Rgb8,
        "缩略图缓存必须为 8 位 RGB（行为锁定）"
    );
    let photo = &project.photos[0];
    let inspected = original_path(&store, &project.id, &photo.id).await.unwrap();
    assert_eq!(
        image::open(inspected.path().unwrap()).unwrap().color(),
        image::ColorType::Rgb16
    );
    assert_eq!(
        image_pipeline::fingerprint(inspected.path().unwrap()).unwrap(),
        photo.content_sha256
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn icc_metadata_is_preserved_in_original_and_preview_declares_its_space() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("wide.jpg");
    image::RgbImage::from_pixel(320, 240, image::Rgb([200, 100, 50]))
        .save(&source)
        .unwrap();
    let profile = "/System/Library/ColorSync/Profiles/Display P3.icc";
    assert!(
        std::path::Path::new(profile).is_file(),
        "ICC 分支不能静默跳过"
    );
    embed_icc_app2(&source, profile);
    let bytes = std::fs::read(&source).unwrap();
    let store = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = store.create("ICC 字节边界", "").await.unwrap();
    let prepared = image_pipeline::prepare(&source, &temp.path().join("cache")).unwrap();
    store.insert_photo(&project.id, &prepared).await.unwrap();
    let project = store.get(&project.id).await.unwrap();
    let photo = &project.photos[0];
    // 320×240 在缩放阈值之下：像素沿用原图数值，缓存因此必须如实写明它在源配置里。
    // 旧规矩"缓存一律不带配置"已作废——不标注的那张会被读侧当成 sRGB 再管一次。
    let source_icc = std::fs::read(profile).unwrap();
    let declared = between_frames_lib::color_engine::embedded_icc_bytes(std::path::Path::new(
        &photo.preview_path,
    ))
    .unwrap();
    assert_eq!(
        declared.as_deref(),
        Some(source_icc.as_slice()),
        "标准解码生成的预览必须原样声明源配置"
    );
    assert_eq!(
        between_frames_lib::color_engine::embedded_icc_bytes(std::path::Path::new(
            &photo.thumbnail_path,
        ))
        .unwrap(),
        declared,
        "缩略图与预览来自同一次解码，声明必须是同一份"
    );
    let inspected = original_path(&store, &project.id, &photo.id).await.unwrap();
    assert_eq!(std::fs::read(inspected.path().unwrap()).unwrap(), bytes);
    assert!(bytes.windows(11).any(|w| w == b"ICC_PROFILE"));
    // 以上验证字节保留，不证明显示器色彩准确。
}

/// 全分辨率按显示器渲染：源用超过缓存预览上限的图，才能证明渲染取自原图而不是缓存预览。
#[cfg(target_os = "macos")]
async fn wide_import(
    dir: &std::path::Path,
    store: &Store,
    project_id: &str,
    extension: &str,
) -> between_frames_lib::domain::Photo {
    let source = dir.join(format!("wide.{extension}"));
    image::RgbImage::from_pixel(2400, 1600, image::Rgb([71, 91, 113]))
        .save(&source)
        .unwrap();
    if extension == "jpg" {
        embed_icc_app2(&source, "/System/Library/ColorSync/Profiles/Display P3.icc");
    }
    let prepared = image_pipeline::prepare(&source, &dir.join("cache")).unwrap();
    store.insert_photo(project_id, &prepared).await.unwrap();
    store.get(project_id).await.unwrap().photos[0].clone()
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn display_managed_inspection_renders_full_resolution_without_touching_original() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = store.create("显示器色彩检视", "").await.unwrap();
    let photo = wide_import(temp.path(), &store, &project.id, "jpg").await;
    let before = std::fs::read(&photo.original_path).unwrap();
    // 分支由"这台机器有没有可用的显示器配置"决定，而不是由被测代码的回答决定：
    // 无头环境确实取不到时要求它明说不可用；有显示器时任何"没变换"的回答都算失败。
    let display_usable =
        between_frames_lib::color_engine::native_transform::display_profile_bytes().is_ok();

    let outcome = display_managed_inspection(&store, &project.id, &photo.id)
        .await
        .unwrap();
    if !display_usable {
        assert_eq!(outcome.status, "display_unavailable");
        assert!(outcome.path.is_none(), "取不到显示器配置时不回传像素");
        assert!(outcome.detail.unwrap().contains("显示器"));
        assert_eq!(
            std::fs::read(&photo.original_path).unwrap(),
            before,
            "失败也不改原图"
        );
        return;
    }
    assert_eq!(
        outcome.status, "transformed",
        "已嵌入 Display P3 应报告真实来路"
    );
    assert!(outcome.source_profile.contains("Display P3"));
    let rendered = std::path::Path::new(
        outcome
            .path
            .as_ref()
            .expect("有显示器配置就必须给出渲染结果"),
    );
    // 全分辨率：缓存预览被压到 1400，原图尺寸只能来自原文件。
    assert_eq!(image::image_dimensions(rendered).unwrap(), (2400, 1600));
    assert_eq!(
        image::image_dimensions(&photo.preview_path).unwrap().0,
        1400
    );
    // 数值已是显示器色彩，文件必须标注本机那份配置；缺标注的话渲染端会按 sRGB 再管一次。
    assert_eq!(
        between_frames_lib::color_engine::embedded_icc_bytes(rendered).unwrap(),
        between_frames_lib::color_engine::native_transform::display_profile_bytes().ok(),
        "渲染结果要带上它所变换到的那份显示器配置"
    );
    assert_eq!(
        std::fs::read(&photo.original_path).unwrap(),
        before,
        "原图字节必须逐字节不变"
    );

    // 第二次命中同一缓存：不重渲染，也不换文件。
    let first = std::fs::metadata(rendered).unwrap().modified().unwrap();
    let again = display_managed_inspection(&store, &project.id, &photo.id)
        .await
        .unwrap();
    assert_eq!(again.path, outcome.path);
    assert_eq!(
        std::fs::metadata(rendered).unwrap().modified().unwrap(),
        first
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn display_managed_inspection_inherits_inspection_gates_and_refuses_derived_sources() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = store.create("显示器色彩门禁", "").await.unwrap();
    let photo = wide_import(temp.path(), &store, &project.id, "jpg").await;

    // 原图被改动：沿用检视门禁，报文件已变，不渲染一张冒充原图内容的显示器版本。
    std::fs::write(&photo.original_path, b"changed").unwrap();
    assert_eq!(
        display_managed_inspection(&store, &project.id, &photo.id)
            .await
            .unwrap_err()
            .code,
        "FILE_CHANGED"
    );

    // RAW 派生预览没有可用的全分辨率原图：明确拒绝而不是拿派生缓存换个说法。
    let dng = temp.path().join("shooting.dng");
    image::GrayImage::from_pixel(900, 700, image::Luma([128]))
        .save_with_format(&dng, image::ImageFormat::Tiff)
        .unwrap();
    let prepared = image_pipeline::prepare(&dng, &temp.path().join("cache")).unwrap();
    store.insert_photo(&project.id, &prepared).await.unwrap();
    let raw = store.get(&project.id).await.unwrap().photos[1].clone();
    let refused = display_managed_inspection(&store, &project.id, &raw.id)
        .await
        .unwrap();
    assert_eq!(refused.status, "source_not_original");
    assert!(refused.path.is_none(), "拒绝时不回传像素");
    assert!(refused.detail.unwrap().contains("全分辨率"));
}

fn make_png16(path: &std::path::Path, width: u32, height: u32) {
    let image = image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::from_fn(width, height, |x, y| {
        let v = ((x * 163 + y * 71) % 65536) as u16;
        image::Rgb([v, v / 2, v / 3])
    });
    image.save(path).unwrap();
}

/// 把 ICC profile 写入 JPEG APP2 段（SOI 后插入）。
#[cfg(target_os = "macos")]
fn embed_icc_app2(jpeg: &std::path::Path, profile: &str) {
    let mut bytes = std::fs::read(jpeg).unwrap();
    let icc = std::fs::read(profile).unwrap();
    let mut app2 = vec![0xFF, 0xE2];
    let segment_len = icc.len() + 2 + 12 + 2; // 长度字段自身 + ICC_PROFILE\0 + seq
    app2.extend_from_slice(&(segment_len as u16).to_be_bytes());
    app2.extend_from_slice(b"ICC_PROFILE\x00");
    app2.extend_from_slice(&[1, 1]); // seq 1/1
    app2.extend_from_slice(&icc);
    // 插到 SOI（前 2 字节）之后
    let mut out = Vec::with_capacity(bytes.len() + app2.len());
    out.extend_from_slice(&bytes[..2]);
    out.extend_from_slice(&app2);
    out.extend_from_slice(&bytes[2..]);
    std::fs::write(jpeg, out).unwrap();
    bytes.clear();
}

#[tokio::test]
async fn renamed_large_tiff_cannot_bypass_raw_inspection_pixel_bounds() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("large.dng");
    image::GrayImage::from_pixel(7000, 5000, image::Luma([128]))
        .save_with_format(&path, image::ImageFormat::Tiff)
        .unwrap();
    let store = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = store.create("RAW 边界", "").await.unwrap();
    let hash = image_pipeline::fingerprint(&path).unwrap();
    let prepared = image_pipeline::prepare(&path, &temp.path().join("cache")).unwrap();
    store.insert_photo(&project.id, &prepared).await.unwrap();
    let project = store.get(&project.id).await.unwrap();
    let source = original_path(&store, &project.id, &project.photos[0].id)
        .await
        .unwrap();
    assert_eq!(source.kind(), "derived_preview", "格式转换不能冒充嵌入预览");
    let (width, height) = image::image_dimensions(source.path().unwrap()).unwrap();
    assert!(
        width <= 1400 && height <= 1400,
        "RAW 检视必须维持有界输出，实际 {width}×{height}"
    );
    assert_eq!(image_pipeline::fingerprint(&path).unwrap(), hash);
    // 缓存被外部替换后也要重新验证产物尺寸，不能只检查文件存在。
    image::RgbImage::new(1500, 1)
        .save(source.path().unwrap())
        .unwrap();
    assert_eq!(
        original_path(&store, &project.id, &project.photos[0].id)
            .await
            .unwrap_err()
            .code,
        "INSPECTION_LIMIT"
    );
}
