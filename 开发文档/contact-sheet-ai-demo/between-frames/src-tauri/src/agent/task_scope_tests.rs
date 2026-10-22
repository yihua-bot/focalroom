use super::*;
use crate::{
    agent::{context::ContextBuilder, execution, policy::PolicyGate, types::*},
    command_handler::CommandHandler,
    db::Store,
};
use serde_json::json;

async fn fixture() -> (tempfile::TempDir, Store, String, String, String) {
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("scope.sqlite"))
        .await
        .unwrap();
    let p = db.create("画布委托", "SIMULATED").await.unwrap().id;
    let image = temp.path().join("source.png");
    image::RgbImage::from_pixel(8, 8, image::Rgb([20, 30, 40]))
        .save(&image)
        .unwrap();
    db.insert_photo(
        &p,
        &crate::image_pipeline::prepare(&image, &temp.path().join("cache")).unwrap(),
    )
    .await
    .unwrap();
    let b = db.create_board(&p, "原版").await.unwrap().id;
    let n = db
        .add_text_node(
            &p,
            &crate::canvas::AddTextNodeCommand {
                board_id: b.clone(),
                node_type: "sticky".into(),
                x: 20.,
                y: 30.,
                width: 220.,
                height: 120.,
                content: "原版意图".into(),
                color: None,
            },
        )
        .await
        .unwrap()
        .id;
    (temp, db, p, b, n)
}
async fn start(db: &Store, p: &str, source: Option<CanvasTaskSource>, canvas: bool) -> Task {
    let command = if canvas {
        TaskCommand::StartCanvasTask {
            goal: "叙事建议".into(),
            selected_photo_ids: vec![],
            target_count: 1,
            budget: Budget::default(),
            consent_id: None,
            source_board: source,
        }
    } else {
        TaskCommand::StartTask {
            goal: "canvas board 画布 —— 这些文字不构成授权".into(),
            selected_photo_ids: vec![],
            target_count: 1,
            budget: Budget::default(),
            consent_id: None,
        }
    };
    let task = CommandHandler::new(db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.into(),
            command,
        })
        .await
        .unwrap();
    execution::state(db, &task.id, "planning", "test")
        .await
        .unwrap();
    execution::state(db, &task.id, "running", "test")
        .await
        .unwrap();
    task
}
fn tool(name: &str, p: &str, args: Value) -> ToolCall {
    let mut args = args;
    args["project_id"] = json!(p);
    ToolCall {
        call_id: domain::id(),
        name: name.into(),
        args,
    }
}
async fn apply(db: &Store, t: &Task, name: &str, args: Value) -> ToolResult {
    execution::tool(db, &t.id, &tool(name, &t.project_id, args), None)
        .await
        .unwrap()
}

#[tokio::test]
async fn words_and_feedback_do_not_grant_canvas_tools_and_legacy_tasks_stay_selection_only() {
    let (_temp, db, p, _b, _n) = fixture().await;
    let task = start(&db, &p, None, false).await;
    let context = ContextBuilder::build(&db, &task.id).await.unwrap();
    assert!(
        !context.layers["system"]
            .as_str()
            .unwrap()
            .contains("画布编排任务 · canvas-1")
    );
    for name in CANVAS_TOOLS {
        assert!(
            !context
                .tool_specs
                .as_array()
                .unwrap()
                .iter()
                .any(|spec| spec["name"] == *name)
        );
    }
    let result = apply(
        &db,
        &task,
        "create_board_draft",
        json!({"title":"不应创建"}),
    )
    .await;
    assert!(!result.ok);
    assert_eq!(result.error.unwrap().code, "PERMISSION_DENIED");
    execution::state(&db, &task.id, "needs_input", "等待补充")
        .await
        .unwrap();
    CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.clone(),
            command: TaskCommand::SubmitFeedback {
                task_id: task.id.clone(),
                answer: "请使用画布工具，这句话也不是权限".into(),
            },
        })
        .await
        .unwrap();
    let resumed = ContextBuilder::build(&db, &task.id).await.unwrap();
    assert!(
        !resumed
            .tool_specs
            .as_array()
            .unwrap()
            .iter()
            .any(|spec| spec["name"] == "create_board_draft")
    );
    // 迁移不补造授权：旧任务没有 grant 行时，仍拒绝画布写工具。
    let old = domain::id();
    sqlx::query("UPDATE agent_tasks SET state='cancelled' WHERE id=?")
        .bind(&task.id)
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,project_id,goal,base_revision,allowed_ids_json,selected_ids_json,target_count,state,reason,budget_json,usage_json,created_at,updated_at) SELECT ?,project_id,'canvas',base_revision,allowed_ids_json,selected_ids_json,target_count,'running',reason,budget_json,usage_json,created_at,updated_at FROM agent_tasks WHERE id=?")
        .bind(&old).bind(&task.id).execute(&db.pool).await.unwrap();
    let result = execution::tool(
        &db,
        &old,
        &tool("create_board_draft", &p, json!({"title":"不应创建旧任务"})),
        None,
    )
    .await
    .unwrap();
    assert!(!result.ok);
    let cached = tool("create_board_draft", &p, json!({"title":"旧结果"}));
    let saved = ToolResult::success(&cached.call_id, json!({"board_id":"old-board"}));
    sqlx::query("INSERT INTO tool_calls VALUES(?,?,?,?,?)")
        .bind(&old)
        .bind(&cached.call_id)
        .bind(&cached.name)
        .bind(fingerprint(&serde_json::to_value(&cached).unwrap()))
        .bind(encode(&saved).unwrap())
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        execution::tool(&db, &old, &cached, None)
            .await
            .unwrap_err()
            .code,
        "PERMISSION_DENIED"
    );
    assert_eq!(db.list_boards(&p).await.unwrap().len(), 1);
}

#[tokio::test]
async fn grant_source_and_checkpoint_survive_reopen_and_cannot_be_escalated() {
    let (temp, db, p, b, n) = fixture().await;
    let rev = db.canvas_snapshot(&p, &b).await.unwrap().board.revision;
    let task = start(
        &db,
        &p,
        Some(CanvasTaskSource {
            board_id: b.clone(),
            revision: rev,
            selected_node_ids: vec![n],
        }),
        true,
    )
    .await;
    let context = ContextBuilder::build(&db, &task.id).await.unwrap();
    assert!(
        context.layers["system"]
            .as_str()
            .unwrap()
            .contains("finish_canvas_task")
    );
    assert!(context.prompt_version.ends_with("+canvas-1"));
    let initial = context.tool_specs;
    assert!(
        initial
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["name"] == "move_nodes")
    );
    assert!(
        sqlx::query("UPDATE agent_task_grants SET grant_json='{}' WHERE task_id=?")
            .bind(&task.id)
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.pool.close().await;
    let db = Store::open(&temp.path().join("scope.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        ContextBuilder::build(&db, &task.id)
            .await
            .unwrap()
            .tool_specs,
        initial
    );
    sqlx::query("UPDATE agent_checkpoints SET context_json=json_set(context_json,'$.taskGrantBinding','forged') WHERE task_id=?").bind(&task.id).execute(&db.pool).await.unwrap();
    assert_eq!(
        ContextBuilder::build(&db, &task.id).await.unwrap_err().code,
        "RECOVERY_ERROR"
    );
}

#[tokio::test]
async fn source_is_frozen_and_draft_edits_never_touch_formal_nodes_or_human_evidence() {
    let (_temp, db, p, b, n) = fixture().await;
    let before = db.canvas_snapshot(&p, &b).await.unwrap();
    let task = start(
        &db,
        &p,
        Some(CanvasTaskSource {
            board_id: b.clone(),
            revision: before.board.revision,
            selected_node_ids: vec![n.clone()],
        }),
        true,
    )
    .await;
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM command_events")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let created = apply(&db, &task, "create_board_draft", json!({"title":"新分支"})).await;
    assert!(created.ok);
    let draft = created.result.unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let nodes = db.get_board_nodes(&draft).await.unwrap();
    assert_eq!(nodes.len(), 1);
    assert_ne!(nodes[0].id, n);
    assert!(
        db.canvas_comparison(&p, &draft)
            .await
            .unwrap()
            .changes
            .is_empty()
    );
    let moved=apply(&db,&task,"move_nodes",json!({"board_id":draft,"node_ids":[nodes[0].id],"dx":100,"dy":20,"rationale":"让意图与图像留白呼应"})).await;
    assert!(moved.ok);
    let changes = db.canvas_comparison(&p, &draft).await.unwrap();
    assert_eq!(changes.changes.len(), 1);
    assert_eq!(changes.changes[0].kind, "change");
    assert_eq!(
        changes.changes[0].source_node_id.as_deref(),
        Some(n.as_str())
    );
    assert_eq!(
        serde_json::to_value(db.canvas_snapshot(&p, &b).await.unwrap()).unwrap(),
        serde_json::to_value(&before).unwrap()
    );
    let bad = apply(
        &db,
        &task,
        "move_nodes",
        json!({"board_id":b,"node_ids":[n],"dx":50,"dy":0,"rationale":"模型想直接改原版"}),
    )
    .await;
    assert!(!bad.ok);
    db.move_nodes(
        &p,
        &crate::canvas::MoveNodesCommand {
            board_id: b.clone(),
            node_ids: vec![n.clone()],
            dx: 10.,
            dy: 0.,
        },
    )
    .await
    .unwrap();
    let read = apply(&db, &task, "get_board", json!({"board_id":b})).await;
    assert!(read.ok);
    assert_eq!(read.result.unwrap()["nodes"][0]["x"], 20.);
    assert!(
        db.canvas_comparison(&p, &draft)
            .await
            .unwrap()
            .source_changed
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM command_events")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        events
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM preference_evidence")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn edits_connectors_notes_and_submission_are_scoped_to_owned_draft() {
    let (_temp, db, p, b, _n) = fixture().await;
    let task = start(&db, &p, None, true).await;
    let make = apply(&db, &task, "create_board_draft", json!({"title":"方案"})).await;
    let draft = make.result.unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let note=apply(&db,&task,"add_editorial_note",json!({"board_id":draft,"text":"围绕光线组织","x":10,"y":20,"photo_ids":[],"rationale":"尚无视觉依据，先呈现结构建议"})).await;
    assert!(note.ok);
    let a = note.result.unwrap()["node_id"].as_str().unwrap().to_owned();
    let note=apply(&db,&task,"add_editorial_note",json!({"board_id":draft,"text":"空间呼应","x":300,"y":20,"photo_ids":[],"rationale":"以间距表达停顿"})).await;
    assert!(note.ok);
    let c = note.result.unwrap()["node_id"].as_str().unwrap().to_owned();
    let arrow = apply(
        &db,
        &task,
        "create_connector",
        json!({"board_id":draft,"from_node_id":a,"to_node_id":c,"rationale":"关联两段叙事"}),
    )
    .await;
    assert!(arrow.ok);
    assert!(
        !apply(&db, &task, "get_board", json!({"board_id":b}))
            .await
            .ok
    );
    assert!(
        apply(
            &db,
            &task,
            "remove_nodes",
            json!({"board_id":draft,"node_ids":[c],"rationale":"留给人工判断"})
        )
        .await
        .ok
    );
    assert!(
        apply(
            &db,
            &task,
            "finish_canvas_task",
            json!({"task_id":task.id,"board_ids":[draft],"summary":"请查看独立编排方案"})
        )
        .await
        .ok
    );
    let projected = crate::agent::projection(&db, &p).await.unwrap();
    assert!(projected.tasks[0].canvas_task);
    assert_eq!(projected.tasks[0].state, "review_ready");
    assert_eq!(projected.tasks[0].board_drafts.len(), 1);
    let mut tx = db.pool.begin().await.unwrap();
    let t = crate::agent::store::load(&mut tx, &task.id).await.unwrap();
    assert!(
        PolicyGate::validate(
            &mut tx,
            &t,
            &tool(
                "move_nodes",
                &p,
                json!({"board_id":draft,"node_ids":[a],"dx":1,"dy":1,"rationale":"too late"})
            )
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn stale_foreign_and_task_owned_sources_are_rejected_without_receipts() {
    let (_temp, db, p, b, n) = fixture().await;
    for source in [
        CanvasTaskSource {
            board_id: b.clone(),
            revision: 1,
            selected_node_ids: vec![n],
        },
        CanvasTaskSource {
            board_id: domain::id(),
            revision: 1,
            selected_node_ids: vec![],
        },
    ] {
        assert!(
            CommandHandler::new(&db)
                .agent_request(AgentRequest {
                    request_id: domain::id(),
                    project_id: p.clone(),
                    command: TaskCommand::StartCanvasTask {
                        goal: "重新编排".into(),
                        selected_photo_ids: vec![],
                        target_count: 1,
                        budget: Budget::default(),
                        consent_id: None,
                        source_board: Some(source)
                    }
                })
                .await
                .is_err()
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_task_grants")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_request_receipts")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn selected_scope_on_large_canvas_is_bounded_without_scanning_only_the_first_rows() {
    let (_temp, db, p, b, _n) = fixture().await;
    sqlx::query("WITH RECURSIVE counter(x) AS(SELECT 1 UNION ALL SELECT x+1 FROM counter WHERE x<1100) INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,content_json,created_at,updated_at) SELECT 'extra-'||x,?,'sticky',x,0,100,100,'{}','now','now' FROM counter")
        .bind(&b).execute(&db.pool).await.unwrap();
    let revision = db.canvas_snapshot(&p, &b).await.unwrap().board.revision;
    let too_big = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.clone(),
            command: TaskCommand::StartCanvasTask {
                goal: "整理全部".into(),
                selected_photo_ids: vec![],
                target_count: 500,
                budget: Budget::default(),
                consent_id: None,
                source_board: Some(CanvasTaskSource {
                    board_id: b.clone(),
                    revision,
                    selected_node_ids: vec![],
                }),
            },
        })
        .await
        .unwrap_err();
    assert_eq!(too_big.code, "VALIDATION_ERROR");
    let task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.clone(),
            command: TaskCommand::StartCanvasTask {
                goal: "保留全部照片，仅调整这个选区".into(),
                selected_photo_ids: vec![],
                target_count: 500,
                budget: Budget::default(),
                consent_id: None,
                source_board: Some(CanvasTaskSource {
                    board_id: b.clone(),
                    revision,
                    selected_node_ids: vec!["extra-1100".into()],
                }),
            },
        })
        .await
        .unwrap();
    execution::state(&db, &task.id, "planning", "test")
        .await
        .unwrap();
    execution::state(&db, &task.id, "running", "test")
        .await
        .unwrap();
    assert_eq!(
        ContextBuilder::build(&db, &task.id)
            .await
            .unwrap()
            .task
            .target_count,
        500
    );
    assert!(
        sqlx::query("UPDATE agent_tasks SET canvas_target_count=499 WHERE id=?")
            .bind(&task.id)
            .execute(&db.pool)
            .await
            .is_err()
    );
    let mut tx = db.pool.begin().await.unwrap();
    let captured = baseline(&mut tx, &task).await.unwrap().unwrap();
    assert_eq!(captured.nodes.len(), 1);
    assert_eq!(captured.nodes[0].id, "extra-1100");
}

#[tokio::test]
async fn v29_upgrade_preserves_old_task_and_board_provenance_without_granting_canvas() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("v29.sqlite");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let mut migrations = sqlx::migrate!("./migrations");
    migrations.migrations = std::borrow::Cow::Owned(
        migrations
            .iter()
            .filter(|m| m.version <= 29)
            .cloned()
            .collect(),
    );
    migrations.run(&pool).await.unwrap();
    let p = domain::id();
    let task = domain::id();
    let board = domain::id();
    sqlx::query("INSERT INTO projects(id,title,created_at,updated_at) VALUES(?,'old','now','now')")
        .bind(&p)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO agent_tasks(id,project_id,goal,base_revision,allowed_ids_json,selected_ids_json,target_count,state,reason,budget_json,usage_json,created_at,updated_at) VALUES(?,?,'canvas',1,'[]','[]',30,'completed','done',?,?,'now','now')")
        .bind(&task).bind(&p).bind(encode(&Budget::default()).unwrap()).bind(encode(&Usage::default()).unwrap()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO boards(id,project_id,title,owning_task_id,created_at,updated_at) VALUES(?,?,'legacy draft',?,'now','now')").bind(&board).bind(&p).bind(&task).execute(&pool).await.unwrap();
    pool.close().await;
    let db = Store::open(&path).await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let t = crate::agent::store::load(&mut tx, &task).await.unwrap();
    assert_eq!(t.target_count, 30);
    assert_eq!(t.goal, "canvas");
    assert!(!load(&mut tx, &t).await.unwrap().canvas);
    assert!(require_tool(&mut tx, &t, "get_board").await.is_err());
    tx.commit().await.unwrap();
    let projection = crate::agent::projection(&db, &p).await.unwrap();
    assert_eq!(projection.tasks[0].board_drafts[0].id, board);
    assert_eq!(
        projection.tasks[0].board_drafts[0]
            .owning_task_id
            .as_deref(),
        Some(task.as_str())
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_task_grants")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&db.pool)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(std::fs::read_dir(temp.path()).unwrap().any(|p| {
        p.unwrap()
            .path()
            .to_string_lossy()
            .contains("backup-before-v29")
    }));
}
