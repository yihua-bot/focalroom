// v1.1 项目标签：独立通道（不占撤销栈、不改 revision）、校验与幂等。
use between_frames_lib::db::Store;

#[tokio::test]
async fn tags_roundtrip_validation_and_no_revision_drift() {
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("projects.sqlite"))
        .await
        .unwrap();
    let project = db.create("标签项目", "").await.unwrap();
    let revision_before = project.revision;
    // 设置 + 去重 + 顺序保留。
    let tags = db
        .set_project_tags(&project.id, &["家庭".into(), "2026".into(), "家庭".into()])
        .await
        .unwrap();
    assert_eq!(tags, vec!["家庭", "2026"]);
    let reloaded = db.get(&project.id).await.unwrap();
    assert_eq!(reloaded.tags, vec!["家庭".to_owned(), "2026".to_owned()]);
    // 组织元数据不推动 revision、不影响草案基线。
    assert_eq!(reloaded.revision, revision_before);
    // 幂等重放同值无副作用。
    db.set_project_tags(&project.id, &["家庭".into(), "2026".into()])
        .await
        .unwrap();
    assert_eq!(db.get(&project.id).await.unwrap().tags.len(), 2);
    // 校验：超长、空串、超过 12 个。
    assert!(
        db.set_project_tags(&project.id, &["x".repeat(25)])
            .await
            .is_err()
    );
    assert!(
        db.set_project_tags(&project.id, &["  ".to_string()])
            .await
            .is_err()
    );
    let many: Vec<String> = (0..13).map(|i| i.to_string()).collect();
    assert!(db.set_project_tags(&project.id, &many).await.is_err());
    // 不存在的项目。
    assert!(db.set_project_tags("no-such", &["a".into()]).await.is_err());
    // 归档项目不可改标签。
    use between_frames_lib::command_handler::{CommandHandler, GuiCommandRequest};
    use between_frames_lib::commands_generated::GuiCommand;
    let archived = CommandHandler::new(&db)
        .execute_gui(GuiCommandRequest::new(
            &project.id,
            reloaded.revision,
            GuiCommand::ArchiveProject {},
        ))
        .await
        .unwrap();
    let _ = archived;
    assert!(
        db.set_project_tags(&project.id, &["后期".into()])
            .await
            .is_err()
    );
}
