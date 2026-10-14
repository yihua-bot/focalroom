// M5 性能基线：记录可复现数字并对退化设宽松上界（防 CI 抖动误报）。
// 运行：cargo test --test perf_baseline -- --nocapture --ignored
use between_frames_lib::{
    command_handler::{CommandHandler, GuiCommandRequest},
    commands_generated::GuiCommand,
    db::Store,
    image_pipeline,
};
use std::time::Instant;

#[tokio::test]
#[ignore] // 基线测试手动运行，输出数字供报告引用
async fn perf_baseline_200_import_and_decision_p95() {
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = db.create("性能基线", "").await.unwrap();
    // 生成 200 张 1200x900 JPEG（接近真实体积量级）。
    let sources: Vec<_> = (0..200)
        .map(|i| {
            let path = temp.path().join(format!("perf{i}.jpg"));
            let image = image::RgbImage::from_fn(1200, 900, |x, y| {
                image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y + i) % 256) as u8])
            });
            image.save(&path).unwrap();
            path
        })
        .collect();
    // 导入吞吐（解码+缩略图+预览+落库）。
    let started = Instant::now();
    for path in &sources {
        let prepared = image_pipeline::prepare(path, &temp.path().join("cache")).unwrap();
        db.insert_photo(&project.id, &prepared).await.unwrap();
    }
    let import_seconds = started.elapsed().as_secs_f64();
    let project = db.get(&project.id).await.unwrap();
    assert_eq!(project.photos.len(), 200);
    // 决策命令延迟分布（顺序执行 120 次，含 keep/undo 混合）。
    let ids: Vec<String> = project.photos.iter().map(|p| p.id.clone()).collect();
    let mut revision = project.revision;
    let mut samples: Vec<f64> = vec![];
    for round in 0..60 {
        for (offset, status) in [(0usize, "keep"), (1, "reject")] {
            let started = Instant::now();
            let receipt = CommandHandler::new(&db)
                .execute_gui(GuiCommandRequest::new(
                    &project.id,
                    revision,
                    GuiCommand::SetPhotoDecision {
                        photo_ids: vec![ids[(round * 2 + offset) % ids.len()].clone()],
                        status: status.into(),
                    },
                ))
                .await
                .unwrap();
            samples.push(started.elapsed().as_secs_f64() * 1000.0);
            revision = receipt.revision;
        }
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = samples[samples.len() / 2];
    let p95 = samples[samples.len() * 95 / 100];
    // 投影查询延迟。
    let started = Instant::now();
    let _ = db.get(&project.id).await.unwrap();
    let projection_ms = started.elapsed().as_secs_f64() * 1000.0;
    println!("性能基线：");
    println!(
        "  200 张 1200x900 导入：{import_seconds:.1}s（{:.1} 张/秒）",
        200.0 / import_seconds
    );
    println!("  决策命令 p50/p95：{p50:.1}ms / {p95:.1}ms");
    println!("  项目投影读取：{projection_ms:.1}ms");
    // 宽松上界：防显著退化（目标机型 p95<150ms 是产品目标，此处为测试机守卫值）。
    assert!(p95 < 500.0, "决策 p95={p95:.1}ms 超过 500ms 守卫值");
    assert!(
        projection_ms < 300.0,
        "投影读取 {projection_ms:.1}ms 超过 300ms 守卫值"
    );
}
