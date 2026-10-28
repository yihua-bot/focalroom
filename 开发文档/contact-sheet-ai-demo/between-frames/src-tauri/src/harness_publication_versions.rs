//! 仅供隔离浏览器旅程：生成图片并走真实插件回流流程，不伪造生产版本目录。
use between_frames_lib::{
    db::Store,
    domain::{self, AppError, Result},
    image_pipeline,
};
use serde_json::{Value, json};
use std::path::Path;
pub async fn prepare(
    store: &Store,
    root: &Path,
    project: &str,
    publication: &str,
    frame: &str,
) -> Result<Value> {
    let layout = store.publication_layout(project, publication).await?;
    let source = layout
        .pages
        .iter()
        .flat_map(|p| &p.frames)
        .find(|f| f.id == frame)
        .and_then(|f| f.source.as_ref())
        .ok_or_else(|| AppError::new("TEST_FIXTURE", "照片框没有来源"))?;
    for (index, color) in [[30, 60, 190], [200, 60, 30]].into_iter().enumerate() {
        let incoming = root.join(format!("publication-return-{index}"));
        std::fs::create_dir_all(&incoming)?;
        let file = incoming.join("edited.jpg");
        image::RgbImage::from_pixel(120, 80, image::Rgb(color))
            .save(&file)
            .map_err(|e| AppError::new("TEST_FIXTURE", e.to_string()))?;
        let hash = image_pipeline::fingerprint(&file)?;
        let manifest = json!({"schemaVersion":3,"transferBatchId":domain::id(),"provider":if index==0{"LIGHTROOM_CLASSIC"}else{"CAPTURE_ONE"},"catalogFingerprint":"e".repeat(64),"issuedAt":chrono::Utc::now().to_rfc3339(),"items":[{"sourceSha256":source.source_sha256,"externalPhotoId":"generated-original","externalVariantId":format!("generated-edit-{index}"),"variantId":domain::id(),"renderId":domain::id(),"renderFilename":"edited.jpg","renderSha256":hash,"width":120,"height":80,"bitDepth":8,"mediaFormat":"JPEG"}]});
        std::fs::write(
            incoming.join(".focalroom-manifest.json"),
            serde_json::to_vec(&manifest)
                .map_err(|e| AppError::new("TEST_FIXTURE", e.to_string()))?,
        )?;
        between_frames_lib::editor_bridge::import_batch(
            store,
            project,
            &incoming,
            &root.join("editor-renders"),
        )
        .await?;
    }
    Ok(json!(true))
}
