// v1.2 页跨图：册页标记命令（可撤销）、快照恢复、印刷分页尊重标记。
use between_frames_lib::{
    command_handler::{CommandHandler, GuiCommandRequest},
    commands_generated::GuiCommand,
    db::Store,
    domain::Project,
    export, image_pipeline,
};
use std::path::Path;

async fn setup(path: &Path, count: usize) -> (Store, Project) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let project = db.create("册页", "").await.unwrap();
    for i in 0..count {
        let source = path.join(format!("b{i}.jpg"));
        image::RgbImage::from_pixel(64, 48, image::Rgb([(i * 67 % 256) as u8, 40, 70]))
            .save(&source)
            .unwrap();
        db.insert_photo(
            &project.id,
            &image_pipeline::prepare(&source, &path.join("cache")).unwrap(),
        )
        .await
        .unwrap();
    }
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
    let project = db.get(&loaded.id).await.unwrap();
    (db, project)
}

#[tokio::test]
async fn mark_command_roundtrip_undo_and_snapshot_restore() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 2).await;
    let photo = &project.photos[0];
    // 设为跨页大图。
    let after = CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &project.id,
            project.revision,
            GuiCommand::SetPhotoBookMark {
                photo_id: photo.id.clone(),
                mark: Some("spread".into()),
            },
        ))
        .await
        .unwrap()
        .project
        .unwrap();
    assert_eq!(after.photos[0].book_mark.as_deref(), Some("spread"));
    // 非法值拒绝。
    assert!(
        CommandHandler::new(&db)
            .execute_gui(GuiCommandRequest::new(
                &project.id,
                after.revision,
                GuiCommand::SetPhotoBookMark {
                    photo_id: photo.id.clone(),
                    mark: Some("big".into()),
                }
            ))
            .await
            .is_err()
    );
    // 撤销后标记清除（册页标记是可撤销编辑）。
    let head: (String,) = sqlx::query_as(
        "SELECT h.group_id FROM command_history h JOIN history_heads c ON c.project_id=h.project_id AND c.cursor=h.position WHERE h.project_id=? AND h.active=1",
    )
    .bind(&project.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let before_undo = db.get(&project.id).await.unwrap();
    let undone = CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &project.id,
            before_undo.revision,
            GuiCommand::UndoCommandGroup {
                transaction_group_id: head.0,
            },
        ))
        .await
        .unwrap()
        .project
        .unwrap();
    assert!(undone.photos[0].book_mark.is_none());
    // 重新标记并保存版本，恢复版本后标记保留（快照含 book_mark）。
    let after_undo = db.get(&project.id).await.unwrap();
    let _marked = CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &project.id,
            after_undo.revision,
            GuiCommand::SetPhotoBookMark {
                photo_id: photo.id.clone(),
                mark: Some("full".into()),
            },
        ))
        .await
        .unwrap()
        .project
        .unwrap();
    let authoritative = db.get(&project.id).await.unwrap();
    CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &project.id,
            authoritative.revision,
            GuiCommand::SaveSequenceVersion {
                label: "带册页标记".into(),
            },
        ))
        .await
        .unwrap();
    let with_mark = db.get(&project.id).await.unwrap();
    assert_eq!(with_mark.photos[0].book_mark.as_deref(), Some("full"));
}

#[tokio::test]
async fn print_paging_respects_spread_marks() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), 3).await;
    // 第三张设为跨页大图 → 分页为 [1,2] + [3 独占]。
    let target = project.photos[2].id.clone();
    let after = CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &project.id,
            project.revision,
            GuiCommand::SetPhotoBookMark {
                photo_id: target,
                mark: Some("spread".into()),
            },
        ))
        .await
        .unwrap()
        .project
        .unwrap();
    let html =
        String::from_utf8(export::render_with_options(&after, "print", false).unwrap()).unwrap();
    assert_eq!(html.matches("class=\"sheet\"").count(), 2);
    assert_eq!(html.matches("plate spread").count(), 1);
    assert_eq!(html.matches("plate half").count(), 2);
    // 标记在序列中间时也正确：第一张 spread → [1 独占] + [2,3]。
    let first = after.photos[0].id.clone();
    let rearranged = CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &project.id,
            after.revision,
            GuiCommand::SetPhotoBookMark {
                photo_id: first,
                mark: Some("spread".into()),
            },
        ))
        .await
        .unwrap()
        .project
        .unwrap();
    let html = String::from_utf8(export::render_with_options(&rearranged, "print", false).unwrap())
        .unwrap();
    assert_eq!(html.matches("plate spread").count(), 2);
    assert_eq!(html.matches("plate half").count(), 1);
}
