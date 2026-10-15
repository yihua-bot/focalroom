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

#[tokio::test]
#[ignore] // 长期增长基准：手动运行，输出增长速率供 M1 报告引用
async fn long_edit_history_growth_baseline() {
    use between_frames_lib::command_handler::{CommandHandler, GuiCommandRequest};
    use between_frames_lib::commands_generated::GuiCommand;
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = db.create("增长基准", "").await.unwrap();
    let sources: Vec<_> = (0..40)
        .map(|i| {
            let path = temp.path().join(format!("g{i:02}.jpg"));
            image::RgbImage::from_pixel(600, 400, image::Rgb([(i * 6 % 256) as u8, 80, 120]))
                .save(&path)
                .unwrap();
            path
        })
        .collect();
    for path in &sources {
        let prepared = image_pipeline::prepare(path, &temp.path().join("cache")).unwrap();
        db.insert_photo(&project.id, &prepared).await.unwrap();
    }
    let current = db.get(&project.id).await.unwrap();
    let ids: Vec<String> = current.photos.iter().map(|p| p.id.clone()).collect();
    let db_path = temp.path().join("projects.sqlite");
    let size_after_import = logical_database_bytes(&db).await;
    let initial_events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_events")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let mut executed_commands = 0_u64;
    // 每组 1 决策 + 两轮 Undo/Redo，共 125×2×5=1250 条 GUI 命令。
    let started = Instant::now();
    for round in 0..125 {
        for (offset, status) in [(0usize, "keep"), (1, "reject")] {
            let target = ids[(round * 2 + offset) % ids.len()].clone();
            let authoritative = db.get(&project.id).await.unwrap();
            let _current = CommandHandler::new(&db)
                .execute_gui(GuiCommandRequest::new(
                    &project.id,
                    authoritative.revision,
                    GuiCommand::SetPhotoDecision {
                        photo_ids: vec![target],
                        status: status.into(),
                    },
                ))
                .await
                .unwrap()
                .project
                .unwrap();
            executed_commands += 1;
            for _ in 0..2 {
                let authoritative = db.get(&project.id).await.unwrap();
                let head: (String,) = sqlx::query_as(
                    "SELECT h.group_id FROM command_history h JOIN history_heads c ON c.project_id=h.project_id AND c.cursor=h.position WHERE h.project_id=? AND h.active=1",
                ).bind(&project.id).fetch_one(&db.pool).await.unwrap();
                let _current = CommandHandler::new(&db)
                    .execute_gui(GuiCommandRequest::new(
                        &project.id,
                        authoritative.revision,
                        GuiCommand::UndoCommandGroup {
                            transaction_group_id: head.0,
                        },
                    ))
                    .await
                    .unwrap()
                    .project
                    .unwrap();
                executed_commands += 1;
                let authoritative = db.get(&project.id).await.unwrap();
                let redo_target = authoritative.history.redo_group_id.clone().unwrap();
                let _current = CommandHandler::new(&db)
                    .execute_gui(GuiCommandRequest::new(
                        &project.id,
                        authoritative.revision,
                        GuiCommand::RedoCommandGroup {
                            transaction_group_id: redo_target,
                        },
                    ))
                    .await
                    .unwrap()
                    .project
                    .unwrap();
                executed_commands += 1;
            }
        }
    }
    let elapsed = started.elapsed().as_secs_f64();
    let size_final = logical_database_bytes(&db).await;
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_events")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let receipts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_receipt_aliases")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let history: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_history")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(executed_commands, 1250);
    assert_eq!(
        events - initial_events,
        executed_commands as i64,
        "成功调用次数与持久化回执增量必须一致"
    );
    let growth = size_final.saturating_sub(size_after_import);
    // 防止把全部历史再次写回每个回执；不作为低端设备性能门槛。
    assert!(
        growth < executed_commands * 128 * 1024,
        "回执增长超过 128 KiB/命令守卫"
    );
    let result = serde_json::json!({
        "commands": executed_commands, "receiptDelta": events - initial_events,
        "elapsedSeconds": elapsed, "commandsPerSecond": executed_commands as f64 / elapsed,
        "logicalBytesAfterImport": size_after_import, "logicalBytesFinal": size_final,
        "logicalGrowthBytesPer100Commands": growth as f64 * 100.0 / executed_commands as f64,
        "physicalMainBytes": std::fs::metadata(&db_path).unwrap().len(),
        "physicalWalBytes": std::fs::metadata(db_path.with_file_name("projects.sqlite-wal")).map(|m|m.len()).unwrap_or(0),
        "physicalShmBytes": std::fs::metadata(db_path.with_file_name("projects.sqlite-shm")).map(|m|m.len()).unwrap_or(0),
        "events": events, "historyGroups": history, "receiptAliases": receipts,
        "measurement": "logical bytes = PRAGMA page_count * page_size, including committed WAL pages; physical files sampled before closing; throughput includes preparation reads"
    });
    println!("growth-baseline: {result}");
}

async fn logical_database_bytes(db: &Store) -> u64 {
    let pages: i64 = sqlx::query_scalar("PRAGMA page_count")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let page_size: i64 = sqlx::query_scalar("PRAGMA page_size")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    (pages * page_size) as u64
}
