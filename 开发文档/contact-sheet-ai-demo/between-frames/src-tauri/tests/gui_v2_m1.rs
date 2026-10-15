// v2.0 M1 真实摄影 GUI 复验：500 张 PNG/JPEG 混合集、Undo/Redo 位置
// 唯一性、超大文件拒绝；原始 hash 不变贯穿全部场景。
use between_frames_lib::{
    command_handler::{CommandHandler, GuiCommandRequest},
    commands_generated::GuiCommand,
    db::Store,
    importer::ImportManager,
};
use std::path::{Path, PathBuf};

fn mixed_fixture(directory: &Path, index: u32) -> PathBuf {
    let png = index.is_multiple_of(2);
    let path = directory.join(format!("m{index:03}.{}", if png { "png" } else { "jpg" }));
    let image = image::RgbImage::from_fn(48, 36, |x, y| {
        image::Rgb([
            ((x + index) % 256) as u8,
            ((y + index * 5) % 256) as u8,
            ((x * y + index * 7 + index / 256) % 256) as u8,
        ])
    });
    image.save(&path).unwrap();
    path
}

#[tokio::test]
async fn five_hundred_mixed_photos_survive_restart_with_hashes_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let sources: Vec<_> = (0..500).map(|index| mixed_fixture(root, index)).collect();
    let original_hashes: Vec<_> = sources
        .iter()
        .map(|source| between_frames_lib::image_pipeline::fingerprint(source).unwrap())
        .collect();
    let db = Store::open(&root.join("projects.sqlite")).await.unwrap();
    let project = db.create("M1 混合集", "").await.unwrap();
    let imports = ImportManager::default();
    // 一次性导入 500 张（混合格式），全部成功。
    let job_id = imports
        .start(
            db.clone(),
            root.join("cache"),
            project.id.clone(),
            sources.clone(),
            |_| {},
        )
        .await
        .unwrap();
    let mut job = imports.get(&job_id).await.unwrap();
    while job.status == "running" {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        job = imports.get(&job_id).await.unwrap();
    }
    assert_eq!(job.failures.len(), 0);
    assert_eq!(job.completed, 500);
    let loaded = db.get(&project.id).await.unwrap();
    assert_eq!(loaded.photos.len(), 500);
    // 选片：偶数索引（PNG）批量保留 100 张 + 个别评分备注。
    let keep: Vec<String> = loaded
        .photos
        .iter()
        .step_by(5)
        .take(100)
        .map(|p| p.id.clone())
        .collect();
    let after = CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &project.id,
            loaded.revision,
            GuiCommand::SetPhotoDecision {
                photo_ids: keep.clone(),
                status: "keep".into(),
            },
        ))
        .await
        .unwrap()
        .project
        .unwrap();
    assert_eq!(after.sequence.len(), 100);
    // 重启：重开数据库，全部状态一致；500 个源 hash 不变。
    db.pool.close().await;
    let reopened = Store::open(&root.join("projects.sqlite")).await.unwrap();
    let revived = reopened.get(&project.id).await.unwrap();
    assert_eq!(revived.photos.len(), 500);
    assert_eq!(revived.sequence.len(), 100);
    assert_eq!(revived.sequence, after.sequence);
    for (source, original_hash) in sources.iter().zip(&original_hashes) {
        assert_eq!(
            &between_frames_lib::image_pipeline::fingerprint(source).unwrap(),
            original_hash,
            "原图内容被修改：{}",
            source.display()
        );
        let imported = revived
            .photos
            .iter()
            .find(|photo| Path::new(&photo.original_path) == source.canonicalize().unwrap())
            .unwrap();
        assert_eq!(
            &imported.content_sha256, original_hash,
            "入库指纹必须对应原文件"
        );
    }
}

#[tokio::test]
async fn undo_redo_never_duplicates_sequence_positions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let sources: Vec<_> = (0..24).map(|index| mixed_fixture(root, index)).collect();
    let db = Store::open(&root.join("projects.sqlite")).await.unwrap();
    let project = db.create("撤销重排", "").await.unwrap();
    let imports = ImportManager::default();
    let job_id = imports
        .start(
            db.clone(),
            root.join("cache"),
            project.id.clone(),
            sources,
            |_| {},
        )
        .await
        .unwrap();
    while imports.get(&job_id).await.unwrap().status == "running" {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let mut current = db.get(&project.id).await.unwrap();
    // 分三批保留不同照片，制造多组历史。
    for round in 0..3 {
        let ids: Vec<String> = current.photos[round * 6..round * 6 + 6]
            .iter()
            .map(|p| p.id.clone())
            .collect();
        current = CommandHandler::new(&db)
            .execute_gui(GuiCommandRequest::new(
                &project.id,
                current.revision,
                GuiCommand::SetPhotoDecision {
                    photo_ids: ids,
                    status: "keep".into(),
                },
            ))
            .await
            .unwrap()
            .project
            .unwrap();
    }
    assert_eq!(current.sequence.len(), 18);
    // 依次撤销三组，再全部重做——每步后序列不得出现重复照片或重复位置。
    let mut step = current.clone();
    for _ in 0..3 {
        let head: (String,) = sqlx::query_as(
            "SELECT h.group_id FROM command_history h JOIN history_heads c ON c.project_id=h.project_id AND c.cursor=h.position WHERE h.project_id=? AND h.active=1",
        ).bind(&project.id).fetch_one(&db.pool).await.unwrap();
        let authoritative = db.get(&project.id).await.unwrap();
        step = CommandHandler::new(&db)
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
        assert_unique_positions(&step.sequence);
    }
    for _ in 0..3 {
        let authoritative = db.get(&project.id).await.unwrap();
        let redo_group = authoritative.history.redo_group_id.clone().unwrap();
        step = CommandHandler::new(&db)
            .execute_gui(GuiCommandRequest::new(
                &project.id,
                authoritative.revision,
                GuiCommand::RedoCommandGroup {
                    transaction_group_id: redo_group,
                },
            ))
            .await
            .unwrap()
            .project
            .unwrap();
        assert_unique_positions(&step.sequence);
    }
    assert_eq!(
        step.sequence, current.sequence,
        "三次撤销三次重做后回到同一序列"
    );
}

fn assert_unique_positions(sequence: &[String]) {
    let deduped: std::collections::HashSet<&String> = sequence.iter().collect();
    assert_eq!(
        deduped.len(),
        sequence.len(),
        "序列出现重复照片：{sequence:?}"
    );
}

#[tokio::test]
async fn oversized_file_is_rejected_without_touching_others() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    // 正常图 + 超大文件（稀疏构造，超过 512MiB 文件上限）。
    let good = mixed_fixture(root, 0);
    let huge = root.join("huge.jpg");
    let file = std::fs::File::create(&huge).unwrap();
    file.set_len(600 * 1024 * 1024).unwrap();
    drop(file);
    let db = Store::open(&root.join("projects.sqlite")).await.unwrap();
    let project = db.create("超大文件", "").await.unwrap();
    let imports = ImportManager::default();
    let job_id = imports
        .start(
            db.clone(),
            root.join("cache"),
            project.id.clone(),
            vec![good.clone(), huge],
            |_| {},
        )
        .await
        .unwrap();
    let mut job = imports.get(&job_id).await.unwrap();
    while job.status == "running" {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        job = imports.get(&job_id).await.unwrap();
    }
    // 正常图成功入库；超大文件被逐张拒绝并说明原因（completed 为处理总数，imported 才是入库数）。
    assert_eq!(job.imported, 1);
    assert_eq!(job.failures.len(), 1);
    assert!(!job.failures[0].message.is_empty());
    let loaded = db.get(&project.id).await.unwrap();
    assert_eq!(loaded.photos.len(), 1);
}
