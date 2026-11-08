use super::*;
use crate::{command_handler::CommandHandler, db::Store};

async fn fixture() -> (tempfile::TempDir, Store, Task) {
    fixture_with_photos(1).await
}

/// 若干张照片的最小画布夹具。照片内容两两不同，观察事实才不会被内容哈希混为一谈。
async fn fixture_with_photos(count: usize) -> (tempfile::TempDir, Store, Task) {
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("canvas-tools.sqlite"))
        .await
        .unwrap();
    let project = db.create("画布权限回归", "仅本地合成素材").await.unwrap();
    for index in 0..count.max(1) {
        let path = temp.path().join(format!("photo-{index}.png"));
        image::RgbImage::from_pixel(8, 8, image::Rgb([1 + index as u8, 2, 3]))
            .save(&path)
            .unwrap();
        db.insert_photo(
            &project.id,
            &crate::image_pipeline::prepare(&path, &temp.path().join("cache")).unwrap(),
        )
        .await
        .unwrap();
    }
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

/// 造一条能被像素核对认下来的观察记录：口径（提示词版本前缀）与压缩尺寸要配套，
/// 否则测的就不是"选了哪一条"，而是"这条根本没法用"。
async fn cache_contract_row(db: &Store, photo: &str, version: &str, facts: &[&str], detail: bool) {
    let (hash, path): (String, String) =
        sqlx::query_as("SELECT content_sha256,preview_path FROM photos WHERE id=?")
            .bind(photo)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let (_, pixel) =
        super::super::upload_compress::compress_profile(std::path::Path::new(&path), detail)
            .unwrap();
    sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,created_at,preview_sha256) VALUES(?,?,'fixture',?,'inspect_photos',?,?,?)")
        .bind(photo).bind(hash).bind(version)
        .bind(json!({"photo_id":photo,"visible_facts":facts.iter().map(|f| f.to_string()).collect::<Vec<_>>(),"interpretation":"fixture","uncertainty":"not real model quality"}).to_string())
        .bind(domain::timestamp()).bind(pixel)
        .execute(&db.pool).await.unwrap();
}

#[tokio::test]
async fn evaluation_keeps_page_and_detail_observations_as_separate_contracts() {
    let (_temp, db, mut task) = fixture_with_photos(2).await;
    let ids = task.allowed_photo_ids.clone();
    assert_eq!(ids.len(), 2);
    let hex = "a".repeat(64);
    let page_version = format!("{}@{}", super::super::vision::VISION_PROMPT_VERSION, hex);
    let detail_version = format!("{}@{}", super::super::vision::DETAIL_PROMPT_VERSION, hex);
    cache_contract_row(
        &db,
        &ids[0],
        &page_version,
        &["事实甲", "事实乙", "事实丙"],
        false,
    )
    .await;
    cache_contract_row(&db, &ids[0], &detail_version, &["事实甲"], true).await;
    cache_contract_row(
        &db,
        &ids[1],
        &detail_version,
        &["事实甲", "事实乙", "事实丙"],
        true,
    )
    .await;

    let project_id = task.project_id.clone();
    let base_revision = task.base_revision;
    let mut tx = db.pool.begin().await.unwrap();
    let created = execute(
        &mut tx,
        &mut task,
        &ToolCall {
            call_id: "create".into(),
            name: "create_selection_draft".into(),
            args: json!({"project_id":project_id,"base_project_revision":base_revision,"title":"fixture","summary":"fixture",
                "items":ids.iter().enumerate().map(|(position,id)| json!({"photo_id":id,"proposed_action":"keep","position":position,"rationale":"fixture"})).collect::<Vec<_>>()}),
        },
        None,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let evaluate = ToolCall {
        call_id: "evaluate".into(),
        name: "evaluate_draft".into(),
        args: json!({"project_id":task.project_id,"draft_id":created["draft_id"]}),
    };
    let result = execute(&mut tx, &mut task, &evaluate, None, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let stored: i64 = sqlx::query_scalar("SELECT revision FROM drafts WHERE id=?")
        .bind(created["draft_id"].as_str().unwrap())
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        result["draft_revision"],
        json!(stored),
        "回执要能说这次核验对着方案的哪一版，报的必须是库里那一版"
    );
    let facts = result["item_facts"].as_array().unwrap();
    assert_eq!(facts.len(), 2);
    assert_eq!(
        facts[0]["evidence_contract"], "current_page_tier",
        "细节档做得更晚，也不该把同照片的页面档顶掉"
    );
    assert_eq!(
        facts[0]["top_facts"],
        json!(["事实甲", "事实乙", "事实丙"]),
        "用的是哪一条，就得报那一条的事实"
    );
    assert_eq!(facts[0]["evidence_prompt_version"], page_version);
    assert_eq!(facts[1]["evidence_contract"], "current_detail_tier");

    let pair = &result["similar_adjacent"][0];
    assert_eq!(pair["photo_ids"], json!([ids[0], ids[1]]));
    assert_eq!(
        pair["same_contract"], false,
        "一张按页面口径问、一张按细节口径问，相同程度的分子分母不是同一把尺子"
    );
    assert_eq!(
        pair["contracts"],
        json!(["current_page_tier", "current_detail_tier"])
    );
    assert_eq!(
        result["metrics"]["evidence_contracts"],
        json!({"current_page_tier":1,"current_detail_tier":1,"other_prompt_version":0,"other_tool":0})
    );
    assert_eq!(result["metrics"]["cross_contract_adjacent_pairs"], 1);
    assert_eq!(result["metrics"]["observed"], 2);
}

/// 最坏情况的一整本：相邻两张张张都像，于是每一对都要上榜。
/// 这份清单既不能少列（本地兜底去重读的就是这里的照片对），也不能把体积顶回界面那道门里。
#[tokio::test]
async fn a_draft_of_all_lookalike_pairs_keeps_every_pair_and_stays_small() {
    let (_temp, db, mut task) = fixture_with_photos(120).await;
    let ids = task.allowed_photo_ids.clone();
    assert_eq!(ids.len(), 120);
    let hex = "a".repeat(64);
    let page_version = format!("{}@{}", super::super::vision::VISION_PROMPT_VERSION, hex);
    for (index, photo) in ids.iter().enumerate() {
        cache_contract_row(
            &db,
            photo,
            &page_version,
            &[
                "公共甲",
                "公共乙",
                "公共丙",
                "公共丁",
                &format!("第{index}张独有的事"),
            ],
            false,
        )
        .await;
    }

    let project_id = task.project_id.clone();
    let base_revision = task.base_revision;
    let mut tx = db.pool.begin().await.unwrap();
    let created = execute(
        &mut tx,
        &mut task,
        &ToolCall {
            call_id: "create".into(),
            name: "create_selection_draft".into(),
            args: json!({"project_id":project_id,"base_project_revision":base_revision,"title":"fixture","summary":"fixture",
                "items":ids.iter().enumerate().map(|(position,id)| json!({"photo_id":id,"proposed_action":"keep","position":position,"rationale":"fixture"})).collect::<Vec<_>>()}),
        },
        None,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let result = execute(
        &mut tx,
        &mut task,
        &ToolCall {
            call_id: "evaluate".into(),
            name: "evaluate_draft".into(),
            args: json!({"project_id":project_id,"draft_id":created["draft_id"]}),
        },
        None,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let pairs = result["similar_adjacent"].as_array().unwrap();
    assert_eq!(
        pairs.len(),
        119,
        "相邻对一对都不能少：少列一对就少排除一张，本地兜底去重选出来的片子会跟着变"
    );
    for (index, pair) in pairs.iter().enumerate() {
        assert_eq!(
            pair["photo_ids"],
            json!([ids[index], ids[index + 1]]),
            "名单要按方案顺序完整列出，不能挑着报"
        );
        assert_eq!(pair["shared_fact_count"], 4);
        assert_eq!(
            pair["same_contract"], true,
            "同一把尺子量出来的对子，不该把同一个口径名再写两遍占地方"
        );
        assert!(
            pair["contracts"].is_null(),
            "口径相同就不点名，只有跨口径才说清两边各按哪档问的"
        );
    }
    assert!(
        result["uncertainty"].as_str().unwrap().contains("前 12 张"),
        "清单比逐张正文长时，就要在话里说清哪些对子只有条数没有文字：{result}"
    );
    let bytes = serde_json::to_string(&result).unwrap().len();
    assert!(
        bytes < 64 * 1024,
        "一百二十张全对上的最坏情况也要整条读得回来：{bytes} 字节"
    );
    assert!(
        bytes / pairs.len() <= 260,
        "每一对只留一句要点，体积才跟着照片数线性走：平均每对 {} 字节",
        bytes / pairs.len()
    );
}

#[tokio::test]
async fn evaluation_uses_only_observations_of_current_preview_pixels() {
    let (_temp, db, mut task) = fixture().await;
    let photo = task.allowed_photo_ids[0].clone();
    let (hash, path): (String, String) =
        sqlx::query_as("SELECT content_sha256,preview_path FROM photos WHERE id=?")
            .bind(&photo)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let pixel_hash = super::super::vision::current_preview_status(None, &path).await;
    assert_eq!(pixel_hash, "unrecorded");
    let (_, fingerprint) =
        super::super::upload_compress::compress_and_fingerprint(std::path::Path::new(&path))
            .unwrap();
    sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,created_at,preview_sha256) VALUES(?,?,'fixture','fixture','inspect_photos',?,?,?)")
        .bind(&photo).bind(&hash)
        .bind(json!({"photo_id":photo,"visible_facts":["generated fixture"],"interpretation":"fixture","uncertainty":"not real model quality"}).to_string())
        .bind(domain::timestamp()).bind(&fingerprint).execute(&db.pool).await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    let project_id = task.project_id.clone();
    let base_revision = task.base_revision;
    let created=execute(&mut tx,&mut task,&ToolCall {call_id:"create".into(),name:"create_selection_draft".into(),
        args:json!({"project_id":project_id,"base_project_revision":base_revision,"title":"fixture","summary":"fixture","items":[{"photo_id":photo,"proposed_action":"keep","position":0,"rationale":"fixture"}]})},None,None).await.unwrap();
    tx.commit().await.unwrap();
    let call = ToolCall {
        call_id: "evaluate".into(),
        name: "evaluate_draft".into(),
        args: json!({"project_id":task.project_id,"draft_id":created["draft_id"]}),
    };
    for (expected, status) in [
        (1, "verified"),
        (0, "mismatch"),
        (0, "unrecorded"),
        (0, "unreadable"),
    ] {
        if status == "mismatch" {
            image::RgbImage::from_pixel(8, 8, image::Rgb([200, 5, 6]))
                .save(&path)
                .unwrap();
        } else if status == "unrecorded" {
            sqlx::query("UPDATE photo_observations SET preview_sha256=NULL WHERE photo_id=?")
                .bind(&photo)
                .execute(&db.pool)
                .await
                .unwrap();
        } else if status == "unreadable" {
            sqlx::query("UPDATE photo_observations SET preview_sha256=? WHERE photo_id=?")
                .bind(&fingerprint)
                .bind(&photo)
                .execute(&db.pool)
                .await
                .unwrap();
            std::fs::rename(&path, format!("{path}.saved")).unwrap();
        }
        let mut tx = db.pool.begin().await.unwrap();
        let result = execute(&mut tx, &mut task, &call, None, None)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(result["metrics"]["observed"], expected);
        assert_eq!(
            result["item_facts"].as_array().unwrap().len(),
            expected as usize
        );
        if expected == 0 {
            assert_eq!(
                result["omitted_visual_evidence"][0]["pixel_version"],
                status
            );
        }
    }
    let unchanged: String = sqlx::query_scalar("SELECT content_sha256 FROM photos WHERE id=?")
        .bind(&photo)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(unchanged, hash);
}

/// 同一个名字只能有一张嘴。`evaluate_sequence` 是 `evaluate_draft` 的旧别名，
/// 它走的那条路必须真的对着像素核对过；扩展分发口不该另备一份只看文件名的"评估"，
/// 否则哪一份被读到，取决于从哪个门进来，而两份都说自己是结构化评估。
#[tokio::test]
async fn the_old_sequence_alias_only_reaches_the_one_picture_verified_evaluator() {
    assert_eq!(
        super::super::policy::ToolRegistry::canonical("evaluate_sequence").unwrap(),
        "evaluate_draft",
        "旧别名要落到那一份真核验单上，不能另开一条"
    );
    let (_temp, db, task) = fixture().await;
    let mut tx = db.pool.begin().await.unwrap();
    let error = execute_canvas_tool_extended(
        &mut tx,
        "evaluate_sequence",
        &json!({"project_id":task.project_id}),
        &task,
    )
    .await
    .unwrap_err();
    tx.rollback().await.unwrap();
    assert_eq!(error.code, "PERMISSION_DENIED");
    assert!(
        error.message.contains("未知扩展工具"),
        "扩展口宁可不认这个名字，也不能给出一份没看过像素的评估：{error:?}"
    );
}

/// 画布那一份序列草案到底能不能在界面上说清"凭几张画面事实说话"。四个环节全走真实通道：
/// 画布任务自己建沙盒画布、画布工具生成草案并就地写回执、界面读的是投影而不是回执原文。
/// 关键是"不用点重算就有"：这条用例里没有任何一次主动重算，回执只能来自生成草案那一步。
/// 哪一环其实是空的（比如草案根本没进投影要读的那张表），这条用例就会红。
#[tokio::test]
async fn a_canvas_sequence_draft_reaches_the_same_picture_basis_on_screen() {
    let (_temp, db, task) = fixture_with_photos(3).await;
    let project = task.project_id.clone();
    let ids = task.allowed_photo_ids.clone();
    assert_eq!(ids.len(), 3);
    let page_version = format!(
        "{}@{}",
        super::super::vision::VISION_PROMPT_VERSION,
        "c".repeat(64)
    );
    for (index, photo) in ids.iter().enumerate() {
        cache_contract_row(
            &db,
            photo,
            &page_version,
            &[
                "公共甲",
                "公共乙",
                "公共丙",
                "公共丁",
                &format!("第{index}张独有的事"),
            ],
            false,
        )
        .await;
    }
    own_board(&db, &task, "画布里的序列").await;
    let mut tx = db.pool.begin().await.unwrap();
    let created = execute_canvas_tool_extended(
        &mut tx,
        "create_sequence_draft",
        &json!({"photo_ids":ids,"title":"画布序列草案"}),
        &task,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let draft = created["draft_id"].as_str().unwrap().to_owned();
    assert_eq!(
        created["visual_basis"],
        json!("recorded"),
        "画布生成草案这一刻就该把本地评估做完，回执不能等用户去点重算：{created:?}"
    );

    // 这里刻意不再调一次"按现在这一版重算口径"：上屏的六个数字必须是生成时就有的那份。

    let view = super::super::store::projection(&db, &project)
        .await
        .unwrap();
    let evidence = &view
        .tasks
        .iter()
        .find(|row| row.id == task.id)
        .expect("画布任务也要出现在同一份投影里")
        .draft_evidence;
    assert_eq!(
        evidence.len(),
        1,
        "画布草案的口径要投出来，一份不多一份不少：{evidence:?}"
    );
    let row = &evidence[0];
    assert_eq!(row["draftId"], json!(draft));
    assert_eq!(row["count"], 3, "三张都算在这份草案里：{row:?}");
    assert_eq!(row["observed"], 3, "三张都有核对过像素的画面观察：{row:?}");
    assert_eq!(row["unobserved"], 0);
    assert_eq!(row["pageTier"], 3, "档位计数要按画布那三张问法说：{row:?}");
    assert_eq!(row["crossContractPairs"], 0);
    assert_eq!(row["omitted"], 0);
    assert_eq!(
        row["perPhoto"].as_array().unwrap().len(),
        3,
        "逐张口径要把画布那三张都点到：{row:?}"
    );
    assert!(
        row["perPhoto"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["used"] == json!(true)
                && item["contract"] == json!("current_page_tier")),
        "画布这几张的说法要和图库侧同一套口径名：{row:?}"
    );
    assert_eq!(
        row["draftRevision"], 1,
        "画布草案的版本号也要一起上屏：{row:?}"
    );
}
