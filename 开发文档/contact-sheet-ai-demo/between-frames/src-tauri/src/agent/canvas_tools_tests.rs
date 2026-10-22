use super::*;
use crate::{command_handler::CommandHandler, db::Store};

async fn fixture() -> (tempfile::TempDir, Store, Task) {
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("canvas-tools.sqlite"))
        .await
        .unwrap();
    let project = db.create("画布权限回归", "仅本地合成素材").await.unwrap();
    let path = temp.path().join("photo.png");
    image::RgbImage::from_pixel(8, 8, image::Rgb([1, 2, 3]))
        .save(&path)
        .unwrap();
    db.insert_photo(
        &project.id,
        &crate::image_pipeline::prepare(&path, &temp.path().join("cache")).unwrap(),
    )
    .await
    .unwrap();
    let task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: project.id,
            command: TaskCommand::StartCanvasTask {
                source_board: None,
                goal: "canvas ownership regression".into(),
                selected_photo_ids: vec![],
                target_count: 1,
                budget: Budget::default(),
                consent_id: None,
            },
        })
        .await
        .unwrap();
    (temp, db, task)
}

async fn own_board(db: &Store, task: &Task, title: &str) -> String {
    let mut tx = db.pool.begin().await.unwrap();
    let board = execute_canvas_tool(&mut tx, "create_board_draft", &json!({"title":title}), task)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    board["board_id"].as_str().unwrap().to_owned()
}

async fn note(db: &Store, project: &str, board: &str) -> String {
    db.add_text_node(
        project,
        &crate::canvas::AddTextNodeCommand {
            board_id: board.into(),
            node_type: "sticky".into(),
            x: 10.0,
            y: 20.0,
            width: 200.0,
            height: 100.0,
            content: "test".into(),
            color: None,
        },
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn group_rejects_human_foreign_task_and_non_draft_boards_without_writes() {
    let (_temp, db, task) = fixture().await;
    let board = own_board(&db, &task, "任务草案").await;
    let node = note(&db, &task.project_id, &board).await;
    for (state, owner) in [
        ("draft", None),
        ("official", Some(task.id.as_str())),
        ("frozen", Some(task.id.as_str())),
    ] {
        sqlx::query("UPDATE boards SET state=?, owning_task_id=? WHERE id=?")
            .bind(state)
            .bind(owner)
            .bind(&board)
            .execute(&db.pool)
            .await
            .unwrap();
        let mut tx = db.pool.begin().await.unwrap();
        let error = execute_canvas_tool_extended(
            &mut tx,
            "group_nodes",
            &json!({"board_id":board,"node_ids":[node],"label":"分组"}),
            &task,
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "PERMISSION_DENIED");
        tx.rollback().await.unwrap();
    }
    sqlx::query("UPDATE boards SET state='draft', owning_task_id=? WHERE id=?")
        .bind(&task.id)
        .bind(&board)
        .execute(&db.pool)
        .await
        .unwrap();
    let mut other = task.clone();
    other.id = "another-task".into();
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        execute_canvas_tool_extended(
            &mut tx,
            "group_nodes",
            &json!({"board_id":board,"node_ids":[node],"label":"分组"}),
            &other
        )
        .await
        .unwrap_err()
        .code,
        "PERMISSION_DENIED"
    );
    tx.rollback().await.unwrap();
    let groups: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM canvas_nodes WHERE node_type='group'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(groups, 0);
}

#[tokio::test]
async fn group_checks_board_project_and_every_child_then_records_real_bounds() {
    let (_temp, db, task) = fixture().await;
    let board = own_board(&db, &task, "本任务").await;
    let node = note(&db, &task.project_id, &board).await;
    let second = own_board(&db, &task, "另一画布").await;
    let foreign_child = note(&db, &task.project_id, &second).await;
    let mut tx = db.pool.begin().await.unwrap();
    assert!(
        execute_canvas_tool_extended(
            &mut tx,
            "group_nodes",
            &json!({"board_id":board,"node_ids":[node,foreign_child],"label":"混入"}),
            &task
        )
        .await
        .is_err()
    );
    tx.rollback().await.unwrap();
    let mut wrong_project = task.clone();
    wrong_project.project_id = "different-project".into();
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        execute_canvas_tool_extended(
            &mut tx,
            "group_nodes",
            &json!({"board_id":board,"node_ids":[node],"label":"越界"}),
            &wrong_project
        )
        .await
        .unwrap_err()
        .code,
        "PERMISSION_DENIED"
    );
    tx.rollback().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let result = execute_canvas_tool_extended(
        &mut tx,
        "group_nodes",
        &json!({"board_id":board,"node_ids":[node],"label":"有效"}),
        &task,
    )
    .await
    .unwrap();
    let bounds: (f64, f64, f64, f64) =
        sqlx::query_as("SELECT x,y,width,height FROM canvas_nodes WHERE id=?")
            .bind(result["group_id"].as_str().unwrap())
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(bounds, (10.0, 20.0, 200.0, 100.0));
    tx.commit().await.unwrap();
}

#[test]
fn ordinary_tasks_do_not_receive_canvas_mutators() {
    let tools = super::super::policy::ToolRegistry::model_specs_for(false);
    assert!(
        !tools
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "group_nodes")
    );
    let tools = super::super::policy::ToolRegistry::model_specs_for(true);
    assert!(
        tools
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "group_nodes")
    );
}
