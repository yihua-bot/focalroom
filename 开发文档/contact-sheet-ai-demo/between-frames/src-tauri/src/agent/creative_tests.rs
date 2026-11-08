use super::*;
use crate::{
    agent::{
        context::ContextBuilder,
        execution, lifecycle,
        memory_control::{MemoryCommand, MemoryRequest},
        model_binding::ModelIdentity,
    },
    db::Store,
};

struct Fixture {
    _temp: tempfile::TempDir,
    db: Store,
    p: String,
    photos: Vec<String>,
}
impl Fixture {
    async fn new(count: usize) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("creative.sqlite"))
            .await
            .unwrap();
        let p = db.create("创作上下文", "测试来源保留").await.unwrap().id;
        for n in 0..count {
            let file = temp.path().join(format!("{n}.png"));
            image::RgbImage::from_pixel(8, 8, image::Rgb([(n % 256) as u8, (n / 256) as u8, 40]))
                .save(&file)
                .unwrap();
            let image = crate::image_pipeline::prepare(&file, &temp.path().join("cache")).unwrap();
            db.insert_photo(&p, &image).await.unwrap();
        }
        let photos = db
            .get(&p)
            .await
            .unwrap()
            .photos
            .into_iter()
            .map(|p| p.id)
            .collect();
        Self {
            _temp: temp,
            db,
            p,
            photos,
        }
    }
    async fn start(
        &self,
        scope: Vec<String>,
        identity: Option<&ModelIdentity>,
        continuation: Option<DraftReference>,
    ) -> Result<Task> {
        lifecycle::request_bound(
            &self.db,
            AgentRequest {
                request_id: domain::id(),
                project_id: self.p.clone(),
                command: TaskCommand::StartConversation {
                    allow_queued_recovery: false,
                    expected_account_id: identity.and_then(|i| i.account_id.clone()),
                    goal: "我希望这个项目保留留白和观看距离的变化".into(),
                    selected_photo_ids: scope,
                    focus_photo_ids: vec![],
                    detail_photo_ids: vec![],
                    target_count: 20,
                    budget: Budget::default(),
                    consent_id: None,
                    source_board: None,
                    continuation,
                },
            },
            identity,
        )
        .await
    }
    async fn run(&self, t: &Task) {
        execution::state(&self.db, &t.id, "planning", "test")
            .await
            .unwrap();
        execution::state(&self.db, &t.id, "running", "test")
            .await
            .unwrap();
    }
    async fn finish(&self, t: &Task) {
        execution::final_outcome(
            &self.db,
            &t.id,
            ModelOutcome::NoAction {
                reason: "保留原版供比较".into(),
            },
        )
        .await
        .unwrap();
    }
    async fn tool(&self, t: &Task, name: &str, mut args: Value) -> ToolResult {
        args["project_id"] = json!(self.p);
        execution::tool(
            &self.db,
            &t.id,
            &ToolCall {
                call_id: domain::id(),
                name: name.into(),
                args,
            },
            None,
        )
        .await
        .unwrap()
    }
    async fn read(&self, t: &Task, section: &str, object: Option<&str>) -> Value {
        let result = self
            .tool(
                t,
                "read_creative_context",
                json!({"section":section,"object_id":object,"offset":0,"limit":30}),
            )
            .await;
        assert!(result.ok, "{:?}", result.error);
        result.result.unwrap()
    }
}
fn identity(user: &str, endpoint: &str) -> ModelIdentity {
    ModelIdentity {
        account_id: Some(user.into()),
        provider_id: crate::agent::PROVIDER_OPENAI.into(),
        model_id: "local-test".into(),
        endpoint_sha256: endpoint.repeat(64),
        capability_profile_sha256: None,
        observation: None,
    }
}

#[tokio::test]
async fn creative_reads_scope_versions_variants_publication_and_never_expose_file_paths() {
    let f = Fixture::new(2).await;
    let a = &f.photos[0];
    let b = &f.photos[1];
    let first = f.db.create_board(&f.p, "范围内").await.unwrap();
    let outside = f.db.create_board(&f.p, "范围外").await.unwrap();
    f.db.place_photo_node(
        &f.p,
        &crate::canvas::PlacePhotoNodeCommand {
            board_id: first.id.clone(),
            asset_id: a.clone(),
            x: 0.,
            y: 0.,
            width: 100.,
            height: 100.,
        },
    )
    .await
    .unwrap();
    f.db.place_photo_node(
        &f.p,
        &crate::canvas::PlacePhotoNodeCommand {
            board_id: outside.id.clone(),
            asset_id: b.clone(),
            x: 0.,
            y: 0.,
            width: 100.,
            height: 100.,
        },
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO edit_variants(id,asset_id,name,created_at) VALUES('variant',?,'v1','now')",
    )
    .bind(a)
    .execute(&f.db.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO renditions(id,variant_id,sha256,width,height,source_type,file_path,file_size,created_at) VALUES('render','variant','hash',1200,800,'external_lightroom','/private/secret.tiff',500,'now')").execute(&f.db.pool).await.unwrap();
    sqlx::query("INSERT INTO sequence_versions VALUES('saved',?,'旧版',0,?,'now')")
        .bind(&f.p)
        .bind(json!({"title":"saved","brief":"","archivedAt":null,"comparisons":[],"sequence":[a,b],"photos":[{"photoId":a,"status":"keep","rating":4,"note":"开篇","bookMark":null},{"photoId":b,"status":"keep","rating":3,"note":"结尾","bookMark":null}]}).to_string())
        .execute(&f.db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO publications(id,project_id,title,created_at,updated_at) VALUES('pub',?,'书','now','now')").bind(&f.p).execute(&f.db.pool).await.unwrap();
    sqlx::query("INSERT INTO publication_pages(id,publication_id,page_number,created_at,updated_at) VALUES('page','pub',1,'now','now')").execute(&f.db.pool).await.unwrap();
    for (i, asset) in [a, b].iter().enumerate() {
        sqlx::query("INSERT INTO publication_frames(id,page_id,asset_id,created_at,updated_at) VALUES(?,'page',?,'now','now')").bind(format!("frame{i}")).bind(asset).execute(&f.db.pool).await.unwrap();
    }
    let t = f.start(vec![a.clone()], None, None).await.unwrap();
    f.run(&t).await;
    let boards = f.read(&t, "boards", None).await;
    assert_eq!(boards["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(boards["data"]["items"][0]["id"], first.id);
    assert!(
        !f.tool(
            &t,
            "read_creative_context",
            json!({"section":"board","object_id":outside.id,"offset":0,"limit":20})
        )
        .await
        .ok
    );
    let variants = f.read(&t, "variants", None).await;
    assert!(
        variants["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["renditionId"] == "render")
    );
    assert!(!variants.to_string().contains("/private"));
    assert_eq!(variants["visualEvidence"], false);
    let version = f.read(&t, "version", Some("saved")).await;
    assert_eq!(version["data"]["items"][0]["photoIds"], json!([a]));
    assert_eq!(version["data"]["items"][0]["partial"], true);
    assert_eq!(version["data"]["items"][0]["photos"][0]["note"], "开篇");
    let layout = f.read(&t, "publication", Some("pub")).await;
    assert_eq!(
        layout["data"]["items"][0]["frames"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(!layout.to_string().contains(b));
    assert_eq!(f.db.get(&f.p).await.unwrap().revision, t.base_revision);
}

#[tokio::test]
async fn board_followup_is_an_independent_child_with_stable_root_and_deleted_baseline() {
    let f = Fixture::new(2).await;
    let first = f.start(vec![], None, None).await.unwrap();
    f.run(&first).await;
    let parent = f
        .tool(&first, "create_board_draft", json!({"title":"原方案"}))
        .await
        .result
        .unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_string();
    let result=f.tool(&first,"place_photos_on_board",json!({"board_id":parent,"placements":[{"photo_id":f.photos[0],"x":10,"y":20,"width":220,"height":160}]})).await;
    assert!(result.ok);
    f.finish(&first).await;
    let before = f.db.get_board_nodes(&parent).await.unwrap();
    let revision: i64 = sqlx::query_scalar("SELECT revision FROM boards WHERE id=?")
        .bind(&parent)
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    let reference = DraftReference {
        kind: "board".into(),
        id: parent.clone(),
        revision,
    };

    let second = f
        .start(vec![], None, Some(reference.clone()))
        .await
        .unwrap();
    let request = ContextBuilder::build(&f.db, &second.id).await.unwrap();
    assert_eq!(
        request.layers["creative"]["continuation"]["reference"]["id"],
        parent
    );
    f.run(&second).await;
    let result = f
        .tool(
            &second,
            "fork_creative_draft",
            json!({"parent":reference,"title":"第二版","reason":"调整节奏"}),
        )
        .await;
    assert!(result.ok, "{:?}", result.error);
    let child = result.result.unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_string();
    let nodes = f.db.get_board_nodes(&child).await.unwrap();
    assert_eq!(nodes.len(), 1);
    assert_ne!(nodes[0].id, before[0].id);
    let edit=f.tool(&second,"move_nodes",json!({"board_id":child,"node_ids":[nodes[0].id],"dx":80,"dy":0,"rationale":"按反馈增加留白"})).await;
    assert!(edit.ok);
    assert_eq!(
        f.db.get_board_nodes(&parent).await.unwrap()[0].x,
        before[0].x
    );
    assert!(!f.tool(&second,"move_nodes",json!({"board_id":parent,"node_ids":[before[0].id],"dx":1,"dy":0,"rationale":"不应改父方案"})).await.ok);
    f.finish(&second).await;
    let mut tx = f.db.pool.begin().await.unwrap();
    let records = lineage(&mut tx, &second.id).await.unwrap();
    assert_eq!(records[0]["parent"]["id"], parent);
    tx.commit().await.unwrap();
    let child_revision: i64 = sqlx::query_scalar("SELECT revision FROM boards WHERE id=?")
        .bind(&child)
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    let third = f
        .start(
            vec![],
            None,
            Some(DraftReference {
                kind: "board".into(),
                id: child.clone(),
                revision: child_revision,
            }),
        )
        .await
        .unwrap();
    f.run(&third).await;
    assert!(f.tool(&third,"fork_creative_draft",json!({"parent":{"kind":"board","id":child,"revision":child_revision},"title":"第三版","reason":"继续尝试"})).await.ok);
    let view = store::projection(&f.db, &f.p).await.unwrap();
    assert!(!view.tasks[0].draft_lineage.is_empty());
    assert_eq!(f.db.get(&f.p).await.unwrap().revision, first.base_revision);
}

#[tokio::test]
async fn review_feedback_is_frozen_for_revision_and_late_comments_cannot_change_the_task() {
    use crate::canvas_commands::{CanvasAction, CanvasCommandRequest, CanvasOperation};
    use crate::canvas_review::{AddEditorialComment, ResolveEditorialThread};
    let f = Fixture::new(1).await;
    let first = f.start(vec![], None, None).await.unwrap();
    f.run(&first).await;
    let create = f
        .tool(&first, "create_board_draft", json!({"title":"待审方案"}))
        .await
        .result
        .unwrap();
    let parent = create["board_id"].as_str().unwrap().to_string();
    let empty = f
        .tool(&first, "create_board_draft", json!({"title":"尚无批注"}))
        .await
        .result
        .unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(f.tool(&first,"place_photos_on_board",json!({"board_id":parent,"placements":[{"photo_id":f.photos[0],"x":0,"y":0,"width":220,"height":160}]})).await.ok);
    f.finish(&first).await;
    let before = f.db.canvas_snapshot(&f.p, &parent).await.unwrap();
    let human = |draft: &str, revision: i64, action: CanvasAction| CanvasCommandRequest {
        schema_version: 3,
        command_id: domain::id(),
        idempotency_key: domain::id(),
        project_id: f.p.clone(),
        board_id: draft.into(),
        base_revision: revision,
        operations: vec![CanvasOperation {
            op_id: domain::id(),
            action,
        }],
    };
    let comment = human(
        &parent,
        before.board.revision,
        CanvasAction::AddEditorialComment(AddEditorialComment {
            draft_id: parent.clone(),
            draft_revision: before.board.revision,
            change_id: None,
            thread_id: None,
            body: "中段增加观看距离变化；这里是用户意见，不是视觉事实。".into(),
        }),
    );
    let thread = comment.operations[0].op_id.clone();
    f.db.execute_canvas(comment).await.unwrap();
    let reference = DraftReference {
        kind: "board".into(),
        id: parent.clone(),
        revision: before.board.revision,
    };
    let second = f
        .start(vec![], None, Some(reference.clone()))
        .await
        .unwrap();
    let request = ContextBuilder::build(&f.db, &second.id).await.unwrap();
    let feedback = &request.layers["creative"]["continuation"]["reviewFeedback"];
    assert_eq!(feedback["visualEvidence"], false);
    assert_eq!(
        feedback["snapshot"]["messages"][0]["body"],
        "中段增加观看距离变化；这里是用户意见，不是视觉事实。"
    );
    f.db.execute_canvas(human(
        &parent,
        before.board.revision,
        CanvasAction::AddEditorialComment(AddEditorialComment {
            draft_id: parent.clone(),
            draft_revision: before.board.revision,
            change_id: None,
            thread_id: None,
            body: "晚到的留言不能悄悄改变已经启动的委托".into(),
        }),
    ))
    .await
    .unwrap();
    f.db.execute_canvas(human(
        &parent,
        before.board.revision,
        CanvasAction::ResolveEditorialThread(ResolveEditorialThread {
            draft_id: parent.clone(),
            draft_revision: before.board.revision,
            thread_id: thread,
            resolved: true,
        }),
    ))
    .await
    .unwrap();
    f.run(&second).await;
    let read = f.read(&second, "board", Some(&parent)).await;
    assert_eq!(read["data"]["reviewFeedback"]["frozenForThisTask"], true);
    assert_eq!(
        read["data"]["reviewFeedback"]["snapshot"]["totalMessages"],
        1
    );
    assert_eq!(
        read["data"]["reviewFeedback"]["snapshot"]["messages"][0]["resolved"],
        false
    );
    assert!(!read.to_string().contains("晚到"));
    assert!(
        f.tool(
            &second,
            "fork_creative_draft",
            json!({"parent":reference,"title":"依据意见的新方案","reason":"处理冻结的审稿意见"})
        )
        .await
        .ok
    );
    assert_eq!(
        encode(&before).unwrap(),
        encode(&f.db.canvas_snapshot(&f.p, &parent).await.unwrap()).unwrap()
    );
    f.finish(&second).await;
    let empty_snapshot = f.db.canvas_snapshot(&f.p, &empty).await.unwrap();
    let third = f
        .start(
            vec![],
            None,
            Some(DraftReference {
                kind: "board".into(),
                id: empty.clone(),
                revision: empty_snapshot.board.revision,
            }),
        )
        .await
        .unwrap();
    f.db.execute_canvas(human(
        &empty,
        empty_snapshot.board.revision,
        CanvasAction::AddEditorialComment(AddEditorialComment {
            draft_id: empty.clone(),
            draft_revision: empty_snapshot.board.revision,
            change_id: None,
            thread_id: None,
            body: "零批注快照之后的新意见".into(),
        }),
    ))
    .await
    .unwrap();
    f.run(&third).await;
    assert!(f.read(&third, "board", Some(&empty)).await["data"]["reviewFeedback"].is_null());
    assert_eq!(
        ContextBuilder::build(&f.db, &third.id)
            .await
            .unwrap()
            .layers["creative"]["continuation"]["reviewCursor"]["cursor"],
        0
    );
}

#[tokio::test]
async fn revision_context_preserves_frozen_decisions_discussion_and_hash_bound_observation_across_rounds()
 {
    use crate::canvas_commands::{CanvasAction, CanvasCommandRequest, CanvasOperation};
    use crate::canvas_review::{
        AddEditorialComment, RecordEditorialDecision, decisions::ReviewDecision,
    };
    let f = Fixture::new(1).await;
    let root = f.db.create_board(&f.p, "原作品").await.unwrap();
    let first = f
        .start(
            vec![],
            None,
            Some(DraftReference {
                kind: "board".into(),
                id: root.id.clone(),
                revision: root.revision,
            }),
        )
        .await
        .unwrap();
    f.run(&first).await;
    let parent=f.tool(&first,"fork_creative_draft",json!({"parent":{"kind":"board","id":root.id,"revision":root.revision},"title":"第一轮","reason":"协议夹具"})).await.result.unwrap()["board_id"].as_str().unwrap().to_string();
    let placements: Vec<_> = (0..35)
        .map(|i| json!({"photo_id":f.photos[0],"x":i*240,"y":0,"width":220,"height":160}))
        .collect();
    assert!(
        f.tool(
            &first,
            "place_photos_on_board",
            json!({"board_id":parent,"placements":placements})
        )
        .await
        .ok
    );
    let call = ToolCall {
        call_id: domain::id(),
        name: "inspect_photos".into(),
        args: json!({"project_id":f.p,"photo_ids":f.photos}),
    };
    let data = super::super::vision::VisionPrefetch {
        kind: "inspect_photos".into(),
        requested: f.photos.clone(),
        observations: vec![(
            f.photos[0].clone(),
            json!({"photo_id":f.photos[0],"visible_facts":["generated fixture"],"interpretation":"test interpretation","uncertainty":"not real model quality"}),
        )],
        image_versions: vec![],
        uploaded: 0,
        dropped: vec![],
        unreadable: vec![],
        rebuilt: vec![],
        unreadable_reasons: vec![],
        tokens: 0,
        consent_id: 0,
        cost_minor_usd: 0,
        model_id: "fixture-observer".into(),
        cache_version: "fixture-v1".into(),
    };
    assert!(
        execution::tool(&f.db, &first.id, &call, Some(&data))
            .await
            .unwrap()
            .ok
    );
    f.finish(&first).await;
    let comparison = f.db.canvas_comparison(&f.p, &parent).await.unwrap();
    let change = comparison.changes[0].id.clone();
    let revision = comparison.draft.revision;
    let human = |board: &str, base: i64, action| CanvasCommandRequest {
        schema_version: 3,
        command_id: domain::id(),
        idempotency_key: domain::id(),
        project_id: f.p.clone(),
        board_id: board.into(),
        base_revision: base,
        operations: vec![CanvasOperation {
            op_id: domain::id(),
            action,
        }],
    };
    let decide = |decision| {
        CanvasAction::RecordEditorialDecision(RecordEditorialDecision {
            draft_id: parent.clone(),
            draft_revision: revision,
            change_ids: vec![change.clone()],
            decision,
            fields_by_change: Default::default(),
        })
    };
    f.db.execute_canvas(human(
        &root.id,
        root.revision,
        decide(ReviewDecision::Rejected),
    ))
    .await
    .unwrap();
    let comment = |body: &str| {
        CanvasAction::AddEditorialComment(AddEditorialComment {
            draft_id: parent.clone(),
            draft_revision: revision,
            change_id: Some(change.clone()),
            thread_id: None,
            body: body.into(),
        })
    };
    f.db.execute_canvas(human(&parent, revision, comment("第一轮的原始意见")))
        .await
        .unwrap();
    let reference = DraftReference {
        kind: "board".into(),
        id: parent.clone(),
        revision,
    };
    let second = f
        .start(vec![], None, Some(reference.clone()))
        .await
        .unwrap();
    // 启动修订之后的决定和留言不得追写进既有修订的依据。
    f.db.execute_canvas(human(
        &root.id,
        root.revision,
        decide(ReviewDecision::Deferred),
    ))
    .await
    .unwrap();
    f.db.execute_canvas(human(&parent, revision, comment("晚到意见不属于这轮")))
        .await
        .unwrap();
    f.run(&second).await;
    let read = f.read(&second, "board", Some(&parent)).await;
    assert_eq!(read["data"]["parentReviewContext"]["totalDecisions"], 35);
    assert!(!read.to_string().contains("晚到意见"));
    let child = f
        .tool(
            &second,
            "fork_creative_draft",
            json!({"parent":reference,"title":"第二轮","reason":"按冻结意见修改"}),
        )
        .await
        .result
        .unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_string();
    let alternative=f.tool(&second,"fork_creative_draft",json!({"parent":{"kind":"board","id":root.id,"revision":root.revision},"title":"另一来源的方案","reason":"不能串用前版依据"})).await.result.unwrap()["board_id"].as_str().unwrap().to_string();
    f.finish(&second).await;
    assert!(
        f.db.editorial_revision_context(&f.p, &alternative, 0)
            .await
            .unwrap()
            .is_none()
    );
    let page =
        f.db.editorial_revision_context(&f.p, &child, 0)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(page.parent.id, parent);
    assert_eq!(page.decisions.len(), 30);
    assert_eq!(page.total_decisions, 35);
    assert_eq!(page.next_offset, Some(30));
    assert_eq!(
        page.decisions
            .iter()
            .find(|d| d.change_id == change)
            .unwrap()
            .decision,
        "rejected"
    );
    assert_eq!(page.discussion.total_messages, 1);
    assert_eq!(page.discussion.messages[0].body, "第一轮的原始意见");
    assert_eq!(page.observations[0]["photoId"], f.photos[0]);
    assert_eq!(page.observations[0]["callId"], call.call_id);
    assert_eq!(page.observations[0]["currentHashMatches"], true);
    let tail =
        f.db.editorial_revision_context(&f.p, &child, 30)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(tail.decisions.len(), 5);
    assert_eq!(tail.next_offset, None);
    assert!(
        f.db.editorial_revision_context("other-project", &child, 0)
            .await
            .is_err()
    );
    assert!(
        f.db.editorial_revision_context(&f.p, &parent, 0)
            .await
            .unwrap()
            .is_none()
    );
    let child_revision =
        f.db.canvas_snapshot(&f.p, &child)
            .await
            .unwrap()
            .board
            .revision;
    let third_ref = DraftReference {
        kind: "board".into(),
        id: child.clone(),
        revision: child_revision,
    };
    let third = f
        .start(vec![], None, Some(third_ref.clone()))
        .await
        .unwrap();
    f.run(&third).await;
    let grandchild = f
        .tool(
            &third,
            "fork_creative_draft",
            json!({"parent":third_ref,"title":"第三轮","reason":"保留前轮路径"}),
        )
        .await
        .result
        .unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_string();
    f.finish(&third).await;
    let third_context =
        f.db.editorial_revision_context(&f.p, &grandchild, 0)
            .await
            .unwrap()
            .unwrap();
    assert!(third_context.has_earlier);
    assert_eq!(third_context.observations[0]["taskId"], first.id);
    assert_eq!(third_context.observations[0]["matchedAtDelegation"], true);
    assert_eq!(f.db.get_board_nodes(&root.id).await.unwrap().len(), 0);
    sqlx::query("INSERT INTO tool_calls(task_id,call_id,tool_name,fingerprint,result_json) VALUES(?,?,'inspect_photos','oversize-fixture',?)")
        .bind(&first.id).bind(domain::id()).bind("x".repeat(2*1024*1024+1)).execute(&f.db.pool).await.unwrap();
    assert_eq!(
        f.start(
            vec![],
            None,
            Some(DraftReference {
                kind: "board".into(),
                id: parent.clone(),
                revision
            })
        )
        .await
        .unwrap_err()
        .code,
        "VALIDATION_ERROR"
    );
    let path = f._temp.path().join("creative.sqlite");
    f.db.pool.close().await;
    let db = Store::open(&path).await.unwrap();
    assert_eq!(
        db.editorial_revision_context(&f.p, &child, 0)
            .await
            .unwrap()
            .unwrap()
            .discussion
            .messages[0]
            .body,
        "第一轮的原始意见"
    );
    sqlx::query("UPDATE photos SET content_sha256=? WHERE id=?")
        .bind("0".repeat(64))
        .bind(&f.photos[0])
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        db.editorial_revision_context(&f.p, &child, 0)
            .await
            .unwrap()
            .unwrap()
            .observations[0]["currentHashMatches"],
        false
    );
    sqlx::query("UPDATE task_messages SET body=json_set(body,'$.parentTitle','tampered') WHERE task_id=? AND kind='creative_review_context'").bind(&second.id).execute(&db.pool).await.unwrap();
    assert_eq!(
        db.editorial_revision_context(&f.p, &child, 0)
            .await
            .unwrap_err()
            .code,
        "RECOVERY_ERROR"
    );
    assert_eq!(
        ContextBuilder::build(&db, &second.id)
            .await
            .unwrap_err()
            .code,
        "RECOVERY_ERROR"
    );
}

#[tokio::test]
async fn selection_revision_context_freezes_partial_approval_feedback_and_observations_across_rounds()
 {
    use crate::{
        command_handler::{CommandHandler, GuiCommandRequest},
        commands_generated::GuiCommand,
    };
    let f = Fixture::new(2).await;
    let first = f.start(vec![], None, None).await.unwrap();
    f.run(&first).await;
    assert!(
        f.tool(
            &first,
            "request_feedback",
            json!({"question":"希望保留哪种观看距离？","options":["先人物","先空间"]})
        )
        .await
        .ok
    );
    lifecycle::request_bound(
        &f.db,
        AgentRequest {
            request_id: domain::id(),
            project_id: f.p.clone(),
            command: TaskCommand::SubmitFeedback {
                task_id: first.id.clone(),
                answer: "先保留人物，但不要自动记为偏好".into(),
            },
        },
        None,
    )
    .await
    .unwrap();
    f.run(&first).await;
    let parent=f.tool(&first,"create_selection_draft",json!({"title":"第一轮选片","base_project_revision":first.base_revision,"summary":"原图和正式序列保持独立","items":f.photos.iter().enumerate().map(|(position,photo)|json!({"photo_id":photo,"proposed_action":"keep","position":position,"rationale":"仅用于协议验证"})).collect::<Vec<_>>()})).await.result.unwrap()["draft_id"].as_str().unwrap().to_string();
    let call = ToolCall {
        call_id: domain::id(),
        name: "inspect_photos".into(),
        args: json!({"project_id":f.p,"photo_ids":[f.photos[0]]}),
    };
    let observation = super::super::vision::VisionPrefetch {
        kind: "inspect_photos".into(),
        requested: vec![f.photos[0].clone()],
        observations: vec![(
            f.photos[0].clone(),
            json!({"photo_id":f.photos[0],"visible_facts":["generated fixture"],"interpretation":"test only","uncertainty":"not real model quality"}),
        )],
        image_versions: vec![],
        uploaded: 0,
        dropped: vec![],
        unreadable: vec![],
        rebuilt: vec![],
        unreadable_reasons: vec![],
        tokens: 0,
        consent_id: 0,
        cost_minor_usd: 0,
        model_id: "fixture-observer".into(),
        cache_version: "fixture-v1".into(),
    };
    assert!(
        execution::tool(&f.db, &first.id, &call, Some(&observation))
            .await
            .unwrap()
            .ok
    );
    f.finish(&first).await;
    let command = GuiCommandRequest::new(
        &f.p,
        f.db.get(&f.p).await.unwrap().revision,
        GuiCommand::ApplyDraft {
            draft_id: parent.clone(),
            expected_draft_revision: 1,
            approved_photo_ids: vec![f.photos[0].clone()],
            mode: "partial".into(),
        },
    );
    let accepted = CommandHandler::new(&f.db)
        .execute_gui(command)
        .await
        .unwrap();
    let original = f.db.get(&f.p).await.unwrap();
    assert_eq!(original.sequence, vec![f.photos[0].clone()]);
    let reference = DraftReference {
        kind: "selection".into(),
        id: parent.clone(),
        revision: 2,
    };
    let second = f
        .start(vec![], None, Some(reference.clone()))
        .await
        .unwrap();
    f.run(&second).await;
    let frozen = f.read(&second, "selection_draft", Some(&parent)).await;
    assert_eq!(frozen["data"]["frozenForThisTask"], true);
    assert_eq!(
        frozen["data"]["reviews"][0]["photoIds"],
        json!([f.photos[0]])
    );
    assert_eq!(frozen["data"]["totalItems"], 2);
    assert!(
        !frozen
            .to_string()
            .contains(f._temp.path().to_str().unwrap())
    );
    assert_eq!(frozen["data"]["visualEvidence"], false);
    let child = f
        .tool(
            &second,
            "fork_creative_draft",
            json!({"parent":reference,"title":"第二轮选片","reason":"结合人工反馈另存"}),
        )
        .await
        .result
        .unwrap()["draft_id"]
        .as_str()
        .unwrap()
        .to_string();
    let alternative=f.tool(&second,"create_selection_draft",json!({"title":"独立来源","base_project_revision":second.base_revision,"summary":"不能套用父版依据","items":[{"photo_id":f.photos[1],"proposed_action":"keep","position":0,"rationale":"独立"}]})).await.result.unwrap()["draft_id"].as_str().unwrap().to_string();
    f.finish(&second).await;
    let page =
        f.db.selection_revision_context(&f.p, &child, 0)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(page.parent.id, parent);
    assert_eq!(page.parent_state, "applied");
    assert_eq!(page.total_items, 2);
    assert_eq!(page.feedback.len(), 3);
    assert_eq!(page.feedback[1].role, "assistant");
    assert_eq!(page.feedback[2].role, "user");
    assert_eq!(page.feedback[2].content, "先保留人物，但不要自动记为偏好");
    assert_eq!(page.reviews[0].mode.as_deref(), Some("partial"));
    assert_eq!(page.reviews[0].photo_ids, vec![f.photos[0].clone()]);
    assert_eq!(page.reviews[0].undone_at_delegation, Some(false));
    assert_eq!(page.observations[0]["callId"], call.call_id);
    assert_eq!(page.observations[0]["currentHashMatches"], true);
    assert!(
        f.db.selection_revision_context(&f.p, &alternative, 0)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        f.db.selection_revision_context("foreign", &child, 0)
            .await
            .is_err()
    );
    assert_eq!(
        encode(&original).unwrap(),
        encode(&f.db.get(&f.p).await.unwrap()).unwrap()
    );
    CommandHandler::new(&f.db)
        .execute_gui(GuiCommandRequest::new(
            &f.p,
            original.revision,
            GuiCommand::UndoCommandGroup {
                transaction_group_id: accepted.transaction_group_id.unwrap(),
            },
        ))
        .await
        .unwrap();
    assert_eq!(
        f.db.selection_revision_context(&f.p, &child, 0)
            .await
            .unwrap()
            .unwrap()
            .reviews[0]
            .undone_at_delegation,
        Some(false)
    );
    let current_parent = f
        .start(vec![], None, Some(reference.clone()))
        .await
        .unwrap();
    assert_eq!(
        ContextBuilder::build(&f.db, &current_parent.id)
            .await
            .unwrap()
            .layers["creative"]["continuation"]["selectionContextPreview"]["reviews"][0]["undoneAtDelegation"],
        true
    );
    f.run(&current_parent).await;
    f.finish(&current_parent).await;
    let third_ref = DraftReference {
        kind: "selection".into(),
        id: child.clone(),
        revision: 1,
    };
    let third = f
        .start(vec![], None, Some(third_ref.clone()))
        .await
        .unwrap();
    f.run(&third).await;
    let grandchild = f
        .tool(
            &third,
            "fork_creative_draft",
            json!({"parent":third_ref,"title":"第三轮","reason":"保留来源"}),
        )
        .await
        .result
        .unwrap()["draft_id"]
        .as_str()
        .unwrap()
        .to_string();
    f.finish(&third).await;
    let history =
        f.db.selection_revision_context(&f.p, &grandchild, 0)
            .await
            .unwrap()
            .unwrap();
    assert!(history.has_earlier);
    assert_eq!(history.observations[0]["taskId"], first.id);
    let path = f._temp.path().join("creative.sqlite");
    f.db.pool.close().await;
    let db = Store::open(&path).await.unwrap();
    assert_eq!(
        db.selection_revision_context(&f.p, &child, 0)
            .await
            .unwrap()
            .unwrap()
            .feedback[2]
            .content,
        "先保留人物，但不要自动记为偏好"
    );
    sqlx::query("UPDATE photos SET content_sha256=? WHERE id=?")
        .bind("0".repeat(64))
        .bind(&f.photos[0])
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        db.selection_revision_context(&f.p, &child, 0)
            .await
            .unwrap()
            .unwrap()
            .items[0]["currentHashMatches"],
        false
    );
    sqlx::query("UPDATE task_messages SET body=json_set(body,'$.parentTitle','changed') WHERE task_id=? AND kind='selection_review_context'").bind(&second.id).execute(&db.pool).await.unwrap();
    assert_eq!(
        ContextBuilder::build(&db, &second.id)
            .await
            .unwrap_err()
            .code,
        "RECOVERY_ERROR"
    );
}

#[tokio::test]
async fn selection_context_pages_are_frozen_and_other_accounts_or_oversized_feedback_cannot_expand_them()
 {
    use crate::{
        command_handler::{CommandHandler, GuiCommandRequest},
        commands_generated::GuiCommand,
    };
    let f = Fixture::new(33).await;
    let identity = identity("creator", "a");
    let first = f.start(vec![], Some(&identity), None).await.unwrap();
    f.run(&first).await;
    let parent=f.tool(&first,"create_selection_draft",json!({"title":"分页父版","base_project_revision":first.base_revision,"summary":"保留全部身份","items":f.photos.iter().enumerate().map(|(position,photo)|json!({"photo_id":photo,"proposed_action":"keep","position":position,"rationale":"元数据夹具"})).collect::<Vec<_>>()})).await.result.unwrap()["draft_id"].as_str().unwrap().to_string();
    f.finish(&first).await;
    let reference = DraftReference {
        kind: "selection".into(),
        id: parent.clone(),
        revision: 1,
    };
    assert!(
        f.start(
            vec![],
            Some(&crate::agent::creative::tests::identity("other", "a")),
            Some(reference.clone())
        )
        .await
        .is_err()
    );
    let second = f
        .start(vec![], Some(&identity), Some(reference.clone()))
        .await
        .unwrap();
    f.run(&second).await;
    let child = f
        .tool(
            &second,
            "fork_creative_draft",
            json!({"parent":reference,"title":"分页子版","reason":"保留父版顺序"}),
        )
        .await
        .result
        .unwrap()["draft_id"]
        .as_str()
        .unwrap()
        .to_string();
    f.finish(&second).await;
    let first_page =
        f.db.selection_revision_context(&f.p, &child, 0)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(first_page.total_items, 33);
    assert_eq!(first_page.items.len(), 30);
    assert_eq!(first_page.next_offset, Some(30));
    let tail =
        f.db.selection_revision_context(&f.p, &child, 30)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(tail.items.len(), 3);
    assert_eq!(tail.items[0]["position"], 30);
    assert_eq!(tail.next_offset, None);
    let original = f.db.get(&f.p).await.unwrap();
    let mut reversed = f.photos.clone();
    reversed.reverse();
    CommandHandler::new(&f.db)
        .execute_gui(GuiCommandRequest::new(
            &f.p,
            original.revision,
            GuiCommand::UpdateDraft {
                draft_id: parent.clone(),
                expected_draft_revision: 1,
                ordered_photo_ids: reversed,
            },
        ))
        .await
        .unwrap();
    assert_eq!(
        f.db.selection_revision_context(&f.p, &child, 0)
            .await
            .unwrap()
            .unwrap()
            .items[0]["photoId"],
        f.photos[0]
    );
    assert!(
        f.start(vec![], Some(&identity), Some(reference))
            .await
            .is_err()
    );
    sqlx::query("INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'clarification_answer',?,'now')").bind(&first.id).bind("x".repeat(2*1024*1024+1)).execute(&f.db.pool).await.unwrap();
    assert_eq!(
        f.start(
            vec![],
            Some(&identity),
            Some(DraftReference {
                kind: "selection".into(),
                id: parent,
                revision: 2
            })
        )
        .await
        .unwrap_err()
        .code,
        "VALIDATION_ERROR"
    );
    assert!(f.db.get(&f.p).await.unwrap().sequence.is_empty());
}

#[tokio::test]
async fn formal_board_branch_retains_root_deletions_across_rounds_and_replays_one_receipt() {
    let f = Fixture::new(2).await;
    let root = f.db.create_board(&f.p, "原作").await.unwrap();
    for (i, id) in f.photos.iter().enumerate() {
        f.db.place_photo_node(
            &f.p,
            &crate::canvas::PlacePhotoNodeCommand {
                board_id: root.id.clone(),
                asset_id: id.clone(),
                x: i as f64 * 240.,
                y: 0.,
                width: 220.,
                height: 160.,
            },
        )
        .await
        .unwrap();
    }
    let revision: i64 = sqlx::query_scalar("SELECT revision FROM boards WHERE id=?")
        .bind(&root.id)
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    let parent = DraftReference {
        kind: "board".into(),
        id: root.id.clone(),
        revision,
    };
    let first = f.start(vec![], None, Some(parent.clone())).await.unwrap();
    f.run(&first).await;
    let call = ToolCall {
        call_id: domain::id(),
        name: "fork_creative_draft".into(),
        args: json!({"project_id":f.p,"parent":parent,"title":"试删一张","reason":"比较节奏"}),
    };
    let created = execution::tool(&f.db, &first.id, &call, None)
        .await
        .unwrap();
    assert!(created.ok, "{:?}", created.error);
    let child = created.result.as_ref().unwrap()["board_id"]
        .as_str()
        .unwrap();
    let nodes = f.db.get_board_nodes(child).await.unwrap();
    assert_eq!(nodes.len(), 2);
    assert!(
        f.tool(
            &first,
            "remove_nodes",
            json!({"board_id":child,"node_ids":[nodes[0].id],"rationale":"试验删除，原版保留"})
        )
        .await
        .ok
    );
    f.finish(&first).await;
    let replay = execution::tool(&f.db, &first.id, &call, None)
        .await
        .unwrap();
    assert_eq!(replay.result, created.result);
    let rev: i64 = sqlx::query_scalar("SELECT revision FROM boards WHERE id=?")
        .bind(child)
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    let parent = DraftReference {
        kind: "board".into(),
        id: child.into(),
        revision: rev,
    };
    let second = f.start(vec![], None, Some(parent.clone())).await.unwrap();
    f.run(&second).await;
    let new = f
        .tool(
            &second,
            "fork_creative_draft",
            json!({"parent":parent,"title":"继续修订","reason":"保留上一轮删除决定"}),
        )
        .await;
    assert!(new.ok, "{:?}", new.error);
    let next = new.result.unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_string();
    let comparison = f.db.canvas_comparison(&f.p, &next).await.unwrap();
    assert_eq!(comparison.source_board_id, Some(root.id.clone()));
    assert_eq!(
        comparison
            .changes
            .iter()
            .filter(|c| c.kind == "remove")
            .count(),
        1
    );
    assert_eq!(f.db.get_board_nodes(&root.id).await.unwrap().len(), 2);
    assert_eq!(f.db.get_board_nodes(&next).await.unwrap().len(), 1);
    f.finish(&second).await;
    f.db.adopt_board_draft(
        &f.p,
        &crate::canvas::AdoptBoardDraftCommand {
            board_id: child.into(),
        },
    )
    .await
    .unwrap();
    let revision: i64 = sqlx::query_scalar("SELECT revision FROM boards WHERE id=?")
        .bind(child)
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    let parent = DraftReference {
        kind: "board".into(),
        id: child.into(),
        revision,
    };
    let third = f.start(vec![], None, Some(parent.clone())).await.unwrap();
    f.run(&third).await;
    let adopted = f
        .tool(
            &third,
            "fork_creative_draft",
            json!({"parent":parent,"title":"独立作品的新方案","reason":"基于已另存的作品"}),
        )
        .await;
    assert!(adopted.ok, "{:?}", adopted.error);
    let id = adopted.result.unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_string();
    let comparison = f.db.canvas_comparison(&f.p, &id).await.unwrap();
    assert_eq!(comparison.source_board_id.as_deref(), Some(child));
    assert!(comparison.changes.is_empty());
}

#[tokio::test]
async fn parent_binding_rejects_changed_revision_foreign_account_recipient_and_narrow_scope() {
    let f = Fixture::new(2).await;
    let id = identity("a", "a");
    let t = f.start(vec![], Some(&id), None).await.unwrap();
    f.run(&t).await;
    let result=f.tool(&t,"create_selection_draft",json!({"title":"父方案","base_project_revision":t.base_revision,"summary":"保持范围","items":[{"photo_id":f.photos[1],"proposed_action":"keep","position":0,"rationale":"元数据夹具"}]})).await;
    assert!(result.ok);
    let parent = result.result.unwrap()["draft_id"]
        .as_str()
        .unwrap()
        .to_string();
    f.finish(&t).await;
    let r = DraftReference {
        kind: "selection".into(),
        id: parent.clone(),
        revision: 1,
    };
    for (who, scope) in [
        (identity("b", "a"), vec![]),
        (identity("a", "b"), vec![]),
        (id.clone(), vec![f.photos[0].clone()]),
    ] {
        assert!(f.start(scope, Some(&who), Some(r.clone())).await.is_err());
    }
    assert!(
        f.start(
            vec![],
            Some(&id),
            Some(DraftReference {
                revision: 99,
                ..r.clone()
            })
        )
        .await
        .is_err()
    );
    let next = f.start(vec![], Some(&id), Some(r.clone())).await.unwrap();
    f.run(&next).await;
    let fork = f
        .tool(
            &next,
            "fork_creative_draft",
            json!({"parent":r,"title":"新选片","reason":"保留父方案"}),
        )
        .await;
    assert!(fork.ok, "{:?}", fork.error);
    let child = fork.result.unwrap()["draft_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(child, parent);
    let parent_items: Vec<(String, i64)> =
        sqlx::query_as("SELECT photo_id,position FROM draft_items WHERE draft_id=?")
            .bind(&parent)
            .fetch_all(&f.db.pool)
            .await
            .unwrap();
    let child_items: Vec<(String, i64)> =
        sqlx::query_as("SELECT photo_id,position FROM draft_items WHERE draft_id=?")
            .bind(&child)
            .fetch_all(&f.db.pool)
            .await
            .unwrap();
    assert_eq!(parent_items, child_items);
}

#[tokio::test]
async fn creative_binding_tamper_is_not_recovered_and_old_tasks_get_no_new_tools() {
    let f = Fixture::new(1).await;
    let task = f.start(vec![], None, None).await.unwrap();
    let context = ContextBuilder::build(&f.db, &task.id).await.unwrap();
    assert!(
        context
            .tool_specs
            .to_string()
            .contains("read_creative_context")
    );
    sqlx::query("INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'creative_continuation','{}','now')").bind(&task.id).execute(&f.db.pool).await.unwrap();
    assert_eq!(
        ContextBuilder::build(&f.db, &task.id)
            .await
            .err()
            .unwrap()
            .code,
        "RECOVERY_ERROR"
    );
    execution::state(&f.db, &task.id, "cancelled", "test")
        .await
        .unwrap();
    let old = lifecycle::request(
        &f.db,
        AgentRequest {
            request_id: domain::id(),
            project_id: f.p.clone(),
            command: TaskCommand::StartTask {
                goal: "旧选片".into(),
                selected_photo_ids: vec![],
                target_count: 1,
                budget: Budget::default(),
                consent_id: None,
            },
        },
    )
    .await
    .unwrap();
    let context = ContextBuilder::build(&f.db, &old.id).await.unwrap();
    assert!(
        !context
            .tool_specs
            .to_string()
            .contains("read_creative_context")
    );
}

#[tokio::test]
async fn confirmed_preference_is_human_deduplicated_retractable_and_account_scoped() {
    let f = Fixture::new(1).await;
    let who = identity("a", "a");
    let t = f.start(vec![], Some(&who), None).await.unwrap();
    f.run(&t).await;
    let messages = crate::focal_chat::messages(&mut f.db.pool.begin().await.unwrap(), &t.id)
        .await
        .unwrap();
    let args = json!({"statement":"本项目偏好保留留白","applicability":"仅本项目的叙事编排","message_ids":[messages[0].id],"photo_ids":[f.photos[0]]});
    let mut fake = args.clone();
    fake["message_ids"] = json!([999999]);
    assert!(!f.tool(&t, "propose_preference", fake).await.ok);
    let result = f.tool(&t, "propose_preference", args).await;
    assert!(result.ok);
    let proposal = result.result.unwrap()["proposalId"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM preference_evidence")
            .fetch_one(&f.db.pool)
            .await
            .unwrap(),
        0
    );
    f.finish(&t).await;
    let command = |account: &str| MemoryCommand::RememberPreference {
        task_id: t.id.clone(),
        proposal_id: proposal.clone(),
        statement: "本项目偏好保留留白".into(),
        expected_account_id: Some(account.into()),
    };
    let request = |command| MemoryRequest {
        request_id: domain::id(),
        project_id: f.p.clone(),
        command,
    };
    let handler = crate::command_handler::CommandHandler::new(&f.db);
    assert!(handler.execute_memory(request(command("b"))).await.is_err());
    assert!(
        handler
            .execute_memory(request(command("a")))
            .await
            .unwrap()
            .changed
    );
    assert!(
        !handler
            .execute_memory(request(command("a")))
            .await
            .unwrap()
            .changed
    );
    let evidence: i64 = sqlx::query_scalar("SELECT id FROM preference_evidence")
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    let hypothesis: i64 = sqlx::query_scalar(
        "SELECT id FROM preference_hypotheses WHERE statement='本项目偏好保留留白'",
    )
    .fetch_one(&f.db.pool)
    .await
    .unwrap();
    let a = f.start(vec![], Some(&who), None).await.unwrap();
    let context = ContextBuilder::build(&f.db, &a.id).await.unwrap();
    assert!(
        context.layers["human_evidence"]["hypotheses"]
            .to_string()
            .contains("保留留白")
    );
    let mut trace_tx = f.db.pool.begin().await.unwrap();
    let captured = super::super::memory_trace::capture(
        &mut trace_tx,
        &context.layers["human_evidence"]["hypotheses"],
    )
    .await
    .unwrap();
    assert_eq!(captured[0]["confirmation"]["action"], "RememberPreference");
    let saved_request: String=sqlx::query_scalar("SELECT request_id FROM memory_commands WHERE project_id=? AND changed=1 AND json_extract(command_json,'$.type')='RememberPreference' AND json_extract(command_json,'$.proposalId')=?")
        .bind(&f.p).bind(&proposal).fetch_one(&mut *trace_tx).await.unwrap();
    assert_eq!(captured[0]["confirmation"]["requestId"], saved_request);
    assert_eq!(captured[0]["statusAtRead"], "confirmed");
    trace_tx.rollback().await.unwrap();
    f.run(&a).await;
    f.finish(&a).await;
    let b = f
        .start(vec![], Some(&identity("b", "a")), None)
        .await
        .unwrap();
    let context = ContextBuilder::build(&f.db, &b.id).await.unwrap();
    assert!(
        !context.layers["human_evidence"]["hypotheses"]
            .to_string()
            .contains("保留留白")
    );
    f.run(&b).await;
    f.finish(&b).await;
    handler
        .execute_memory(request(MemoryCommand::SetEvidenceExcluded {
            evidence_id: evidence,
            excluded: true,
        }))
        .await
        .unwrap();
    let memory = super::super::memory::projection(&f.db, &f.p).await.unwrap();
    assert!(
        memory
            .hypotheses
            .iter()
            .find(|h| h.id == hypothesis)
            .unwrap()
            .evidence_ids
            .is_empty()
    );
    handler
        .execute_memory(request(MemoryCommand::SetEvidenceExcluded {
            evidence_id: evidence,
            excluded: false,
        }))
        .await
        .unwrap();
    let memory = super::super::memory::projection(&f.db, &f.p).await.unwrap();
    assert_eq!(
        memory
            .hypotheses
            .iter()
            .find(|h| h.id == hypothesis)
            .unwrap()
            .evidence_ids,
        vec![evidence]
    );
    handler
        .execute_memory(request(MemoryCommand::CorrectHypothesis {
            hypothesis_id: hypothesis,
            correction: "只适用于开篇，其他位置不一定".into(),
        }))
        .await
        .unwrap();
    let c = f.start(vec![], Some(&who), None).await.unwrap();
    assert!(
        !ContextBuilder::build(&f.db, &c.id).await.unwrap().layers["human_evidence"]["hypotheses"]
            .to_string()
            .contains("保留留白")
    );
    assert_eq!(f.db.get(&f.p).await.unwrap().revision, t.base_revision);
}

#[test]
fn photography_knowledge_has_exact_licensed_passages_and_no_fabricated_sources() {
    let result =
        super::super::knowledge::search(&json!({"query":"摄影书 photobook","limit":3})).unwrap();
    let entries = result["results"].as_array().unwrap();
    assert!(!entries.is_empty());
    assert_eq!(entries[0]["license"], "CC0-1.0");
    assert!(entries[0]["passage"].as_str().unwrap().contains("读者"));
    assert_eq!(entries[0]["contentSha256"].as_str().unwrap().len(), 64);
    assert_eq!(result["visualEvidence"], false);
    assert!(super::super::knowledge::search(&json!({"query":"zzzzzzqwerty","limit":3})).unwrap()["results"].as_array().unwrap().is_empty());
}

struct ThreeThreads;
impl crate::agent::provider::ModelProvider for ThreeThreads {
    fn generate<'a>(
        &'a self,
        input: &'a crate::agent::context::ModelTurnRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ModelOutcome>> + Send + 'a>>
    {
        Box::pin(async move {
            let archive: Vec<ProtocolMessage> = input.layers["execution"]["archived_tool_receipts"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|body| ProtocolMessage {
                    kind: "tool".into(),
                    body: body.clone(),
                })
                .collect();
            let records: Vec<_> = archive
                .iter()
                .chain(input.protocol.iter())
                .filter(|p| p.kind == "tool")
                .collect();
            assert!(!records.iter().any(|r| r.body["result"]["ok"] == false));
            let call = |name: &str, mut args: Value| {
                assert!(
                    input
                        .tool_specs
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|s| s["name"] == name)
                );
                args["project_id"] = json!(input.task.project_id);
                Ok(ModelOutcome::ToolCalls {
                    calls: vec![ToolCall {
                        call_id: format!("{}-{}", input.task.id, input.task.usage.model_turns),
                        name: name.into(),
                        args,
                    }],
                })
            };
            if !records
                .iter()
                .any(|r| r.body["name"] == "search_photography_knowledge")
                && !input.layers["creative"]["portfolio_method"].is_object()
            {
                return call(
                    "search_photography_knowledge",
                    json!({"query":"portfolio-curation-v1","limit":1}),
                );
            }
            let pages: Vec<_> = records
                .iter()
                .filter(|r| r.body["name"] == "list_photos")
                .collect();
            if pages.len() < 3 {
                return call(
                    "list_photos",
                    json!({"status":null,"limit":100,"cursor":(pages.len()*100).to_string()}),
                );
            }
            let photos: Vec<_> = pages
                .iter()
                .flat_map(|r| {
                    r.body["result"]["result"]["photos"]
                        .as_array()
                        .map(|items| {
                            items
                                .iter()
                                .map(|p| p["photo_id"].clone())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_else(|| {
                            r.body["result"]["result"]["photo_ids"]
                                .as_array()
                                .unwrap()
                                .clone()
                        })
                        .into_iter()
                })
                .collect();
            assert_eq!(photos.len(), 300);
            let boards: Vec<_> = records
                .iter()
                .filter(|r| r.body["name"] == "create_board_draft")
                .map(|r| r.body["result"]["result"]["board_id"].clone())
                .collect();
            let placed = records
                .iter()
                .filter(|r| r.body["name"] == "place_photos_on_board")
                .count();
            if boards.len() > placed {
                let k = placed;
                return call(
                    "place_photos_on_board",
                    json!({"board_id":boards[k],"placements":photos.iter().skip(k*100).take(20).enumerate().map(|(i,p)|json!({"photo_id":p,"x":(i%5)*240,"y":(i/5)*180,"width":220,"height":160})).collect::<Vec<_>>()}),
                );
            }
            if boards.len() < 3 {
                return call(
                    "create_board_draft",
                    json!({"title":format!("SIMULATED · 线索 {}",boards.len()+1)}),
                );
            }
            call(
                "finish_canvas_task",
                json!({"task_id":input.task.id,"board_ids":boards,"summary":"夹具已创建三份各 20 张的方案；没有进行真实视觉判断"}),
            )
        })
    }
}
#[tokio::test]
async fn three_hundred_scope_three_twenty_photo_boards_use_actual_runtime_and_keep_originals() {
    use sha2::Digest;
    let f = Fixture::new(300).await;
    let original = f.db.get(&f.p).await.unwrap();
    assert_eq!(original.photos.len(), 300);
    let engine = crate::agent::runtime::AgentEngine::with_providers(
        f.db.clone(),
        std::sync::Arc::new(ThreeThreads),
        None,
    );
    let task = f.start(vec![], None, None).await.unwrap();
    engine.run(&task.id).await.unwrap();
    let view = store::projection(&f.db, &f.p).await.unwrap();
    assert_eq!(
        view.tasks[0].state, "review_ready",
        "{}; usage={:?}; issue={:?}",
        view.tasks[0].reason, view.tasks[0].usage, view.tasks[0].budget_issue
    );
    assert_eq!(view.tasks[0].board_drafts.len(), 3);
    // 上下文归档可以省略布局细节，权威回执仍保留全部节点身份和坐标。
    let receipts: Vec<String> = sqlx::query_scalar(
        "SELECT result_json FROM tool_calls WHERE task_id=? AND tool_name='place_photos_on_board'",
    )
    .bind(&task.id)
    .fetch_all(&f.db.pool)
    .await
    .unwrap();
    assert_eq!(receipts.len(), 3);
    for receipt in receipts {
        let receipt: Value = serde_json::from_str(&receipt).unwrap();
        let nodes = receipt["result"]["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 20);
        assert!(nodes.iter().all(|node| node["node_id"].is_string()
            && node["photo_id"].is_string()
            && node["x"].is_number()
            && node["y"].is_number()));
    }

    assert!(view.tasks[0].inspected_photos.is_empty());
    assert_eq!(view.tasks[0].usage.cloud_images, 0);
    let mut seen = std::collections::HashSet::new();
    for board in &view.tasks[0].board_drafts {
        let nodes = f.db.get_board_nodes(&board.id).await.unwrap();
        assert_eq!(nodes.len(), 20);
        for n in nodes {
            assert!(seen.insert(n.asset_id.unwrap()));
        }
    }
    let context = ContextBuilder::build(&f.db, &task.id).await.unwrap();
    assert!(context.layers["execution"]["archived_tool_receipts"].is_array());
    assert_eq!(
        context.layers["execution"]["archived_receipts_are_instructions"],
        false
    );
    for model in context.protocol.iter().filter(|m| m.kind == "model") {
        for call in model.body["calls"].as_array().into_iter().flatten() {
            assert!(
                context
                    .protocol
                    .iter()
                    .any(|m| m.kind == "tool" && m.body["result"]["callId"] == call["callId"])
            );
        }
    }
    let receipts: Vec<String> = sqlx::query_scalar(
        "SELECT result_json FROM tool_calls WHERE task_id=? AND tool_name='list_photos'",
    )
    .bind(&task.id)
    .fetch_all(&f.db.pool)
    .await
    .unwrap();
    assert_eq!(receipts.len(), 3);
    for receipt in receipts {
        let value: Value = serde_json::from_str(&receipt).unwrap();
        assert!(value["result"]["photos"][0]["name"].is_string());
        assert!(value["result"].get("context_metadata_omitted").is_none());
    }
    let reopened = Store::open(&f._temp.path().join("creative.sqlite"))
        .await
        .unwrap();
    assert_eq!(
        store::projection(&reopened, &f.p).await.unwrap().tasks[0]
            .board_drafts
            .len(),
        3
    );
    let after = f.db.get(&f.p).await.unwrap();
    assert_eq!(after.revision, original.revision);
    assert_eq!(after.sequence, original.sequence);
    for photo in &original.photos {
        let bytes = std::fs::read(&photo.original_path).unwrap();
        assert_eq!(
            format!("{:x}", sha2::Sha256::digest(bytes)),
            photo.content_sha256
        );
    }
}

async fn overview_cache(
    f: &Fixture,
    photo: &str,
    recorded: Option<String>,
    valid: bool,
    model: &str,
    version: &str,
) {
    let hash: String = sqlx::query_scalar("SELECT content_sha256 FROM photos WHERE id=?")
        .bind(photo)
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    let observation = if valid {
        json!({"photo_id":photo,"visible_facts":["shared fixture fact"],"interpretation":"fixture interpretation","uncertainty":"not real model quality"})
    } else {
        json!({"photo_id":"foreign","visible_facts":["must not appear"]})
    };
    sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,created_at,preview_sha256) VALUES(?,?,?,?,'inspect_photos',?,?,?)")
        .bind(photo).bind(hash).bind(model).bind(version).bind(observation.to_string())
        .bind(domain::timestamp()).bind(recorded).execute(&f.db.pool).await.unwrap();
}
async fn overview_pixels(f: &Fixture, photo: &str) -> (String, String) {
    let path: String = sqlx::query_scalar("SELECT preview_path FROM photos WHERE id=?")
        .bind(photo)
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    let (_, hash) =
        super::super::upload_compress::compress_and_fingerprint(std::path::Path::new(&path))
            .unwrap();
    (path, hash)
}
async fn overview_page(f: &Fixture, t: &Task, offset: usize, limit: usize) -> Value {
    let result = f
        .tool(
            t,
            "read_creative_context",
            json!({"section":"visual_overview","object_id":null,"offset":offset,"limit":limit}),
        )
        .await;
    assert!(result.ok, "{:?}", result.error);
    result.result.unwrap()["data"].clone()
}

#[tokio::test]
async fn visual_overview_paginates_scope_and_separates_unseen_stale_legacy_and_unreadable_pixels() {
    let f = Fixture::new(8).await;
    let ids = f.photos[..6].to_vec();
    let task = f.start(ids.clone(), None, None).await.unwrap();
    f.run(&task).await;
    let version = format!(
        "{}@{}",
        super::super::vision::VISION_PROMPT_VERSION,
        "a".repeat(64)
    );
    for (index, id) in ids.iter().enumerate().take(5) {
        let (path, hash) = overview_pixels(&f, id).await;
        overview_cache(
            &f,
            id,
            if index == 2 { None } else { Some(hash) },
            index != 4,
            "fixture",
            &version,
        )
        .await;
        if index == 1 {
            image::RgbImage::from_pixel(8, 8, image::Rgb([240, 1, 2]))
                .save(&path)
                .unwrap();
        }
        if index == 3 {
            std::fs::rename(&path, format!("{path}.saved")).unwrap();
        }
    }
    let first = overview_page(&f, &task, 0, 3).await;
    assert_eq!(first["scope"]["photoCount"], 6);
    assert_eq!(first["coverage"]["storedObservationRecords"], 5);
    assert_eq!(
        first["coverage"]["globalPixelVerification"],
        "not_performed"
    );
    assert_eq!(
        first["coverage"]["pagePixelVersions"],
        json!({"verified":1,"mismatch":1,"unrecorded":1})
    );
    assert_eq!(first["nextOffset"], 3);
    assert_eq!(first["nextSteps"]["inspectPhotoIds"], json!([ids[1]]));
    assert_eq!(
        first["nextSteps"]["historicalUnverifiedPhotoIds"],
        json!([ids[2]])
    );
    assert_eq!(
        first["items"][0]["visibleFacts"],
        json!(["shared fixture fact"])
    );
    assert!(first["items"][1]["visibleFacts"].is_null());
    assert!(first["items"][2]["visibleFacts"].is_null());
    assert!(
        !first.to_string().contains(f._temp.path().to_str().unwrap()),
        "不把本地路径交给模型"
    );
    for hidden in &f.photos[6..] {
        assert!(!first.to_string().contains(hidden));
    }
    let second = overview_page(&f, &task, 3, 3).await;
    assert_eq!(
        second["coverage"]["pagePixelVersions"],
        json!({"unreadable":1,"invalid":1,"missing":1})
    );
    assert_eq!(second["nextSteps"]["unavailablePhotoIds"], json!([ids[3]]));
    assert_eq!(
        second["nextSteps"]["inspectPhotoIds"],
        json!([ids[4], ids[5]])
    );
    assert!(second["nextOffset"].is_null());
    assert!(!second.to_string().contains("must not appear"));
    assert_eq!(overview_page(&f, &task, 6, 3).await["items"], json!([]));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photo_observations")
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    assert_eq!(count, 5, "概览不创建观察缓存或触发上传");
    assert_eq!(
        store::projection(&f.db, &f.p).await.unwrap().tasks[0]
            .usage
            .cloud_images,
        0
    );
}

#[tokio::test]
async fn visual_overview_rechecks_after_new_observations_and_compares_only_true_sequence_neighbors()
{
    let f = Fixture::new(3).await;
    let task = f.start(f.photos.clone(), None, None).await.unwrap();
    f.run(&task).await;
    let version = format!(
        "{}@{}",
        super::super::vision::VISION_PROMPT_VERSION,
        "a".repeat(64)
    );
    assert_eq!(
        overview_page(&f, &task, 0, 3).await["coverage"]["pagePixelVersions"]["missing"],
        3
    );
    for id in &f.photos {
        let (_, hash) = overview_pixels(&f, id).await;
        overview_cache(&f, id, Some(hash), true, "fixture", &version).await;
    }
    sqlx::query("UPDATE photo_decisions SET status='keep' WHERE project_id=?")
        .bind(&f.p)
        .execute(&f.db.pool)
        .await
        .unwrap();
    for (position, id) in f.photos.iter().enumerate() {
        sqlx::query(
            "INSERT INTO sequence_items(project_id,photo_id,position,updated_at) VALUES(?,?,?,?)",
        )
        .bind(&f.p)
        .bind(id)
        .bind(position as i64)
        .bind(domain::timestamp())
        .execute(&f.db.pool)
        .await
        .unwrap();
    }
    let checked = overview_page(&f, &task, 0, 3).await;
    assert_eq!(checked["coverage"]["pagePixelVersions"]["verified"], 3);
    assert_eq!(
        checked["nextSteps"]["comparePhotoIds"],
        json!(&f.photos[..2])
    );
    assert_eq!(checked["pageRepeatedFacts"][0]["photoIds"], json!(f.photos));
    assert_eq!(checked["sampling"]["isSelectionRecommendation"], false);
    let first_page = overview_page(&f, &task, 0, 1).await;
    assert_eq!(
        first_page["nextSteps"]["comparePhotoIds"],
        json!([]),
        "不把未核对页面内的图当作序列比较依据"
    );
}

#[tokio::test]
async fn visual_overview_respects_model_binding_and_empty_projects() {
    let f = Fixture::new(1).await;
    let binding = identity("fixture-user", "b");
    let task = f
        .start(f.photos.clone(), Some(&binding), None)
        .await
        .unwrap();
    f.run(&task).await;
    let (_, hash) = overview_pixels(&f, &f.photos[0]).await;
    overview_cache(
        &f,
        &f.photos[0],
        Some(hash.clone()),
        true,
        "other-model",
        &binding.cache_version(),
    )
    .await;
    assert_eq!(
        overview_page(&f, &task, 0, 30).await["coverage"]["noStoredObservation"],
        1
    );
    overview_cache(
        &f,
        &f.photos[0],
        Some(hash.clone()),
        true,
        binding.observation_model_id(),
        &format!(
            "{}@{}",
            super::super::vision::VISION_PROMPT_VERSION,
            "a".repeat(64)
        ),
    )
    .await;
    assert_eq!(
        overview_page(&f, &task, 0, 30).await["coverage"]["noStoredObservation"],
        1
    );
    overview_cache(
        &f,
        &f.photos[0],
        Some(hash),
        true,
        binding.observation_model_id(),
        &binding.cache_version(),
    )
    .await;
    assert_eq!(
        overview_page(&f, &task, 0, 30).await["coverage"]["pagePixelVersions"]["verified"],
        1
    );
    let empty = Fixture::new(0).await;
    let task = empty.start(vec![], None, None).await.unwrap();
    empty.run(&task).await;
    let page = overview_page(&empty, &task, 0, 30).await;
    assert_eq!(page["scope"]["photoCount"], 0);
    assert_eq!(page["items"], json!([]));
    assert_eq!(page["sampling"]["photoIds"], json!([]));
    assert!(page["nextOffset"].is_null());
}

struct OverviewWithoutVision;
impl crate::agent::provider::ModelProvider for OverviewWithoutVision {
    fn generate<'a>(
        &'a self,
        input: &'a crate::agent::context::ModelTurnRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ModelOutcome>> + Send + 'a>>
    {
        Box::pin(async move {
            let tools: Vec<_> = input.protocol.iter().filter(|p| p.kind == "tool").collect();
            let call = |name: &str, mut args: Value| {
                args["project_id"] = json!(input.task.project_id);
                ModelOutcome::ToolCalls {
                    calls: vec![ToolCall {
                        call_id: format!("overview-{}", input.task.usage.model_turns),
                        name: name.into(),
                        args,
                    }],
                }
            };
            if tools.is_empty() {
                assert!(input.tool_specs.as_array().unwrap().iter().any(|t| {
                    t["name"] == "read_creative_context"
                        && t["parameters"]["properties"]["section"]["enum"]
                            .as_array()
                            .unwrap()
                            .contains(&json!("visual_overview"))
                }));
                return Ok(call(
                    "read_creative_context",
                    json!({"section":"visual_overview","object_id":null,"offset":0,"limit":30}),
                ));
            }
            assert!(tools.iter().all(|t| t.body["result"]["ok"] == true));
            if tools.len() == 1 {
                let ids =
                    &tools[0].body["result"]["result"]["data"]["nextSteps"]["inspectPhotoIds"];
                assert!(!ids.as_array().unwrap().is_empty());
                return Ok(call("inspect_photos", json!({"photo_ids":ids})));
            }
            assert_eq!(
                tools[1].body["result"]["result"]["mode"],
                "mock_metadata_only"
            );
            Ok(ModelOutcome::NoAction {
                reason: "缺少视觉授权；概览只说明待观察范围，没有完成视觉分析。".into(),
            })
        })
    }
}
#[tokio::test]
async fn visual_overview_runs_in_agent_loop_without_turning_metadata_into_visual_analysis() {
    let f = Fixture::new(4).await;
    let task = f.start(f.photos[..2].to_vec(), None, None).await.unwrap();
    let engine = crate::agent::runtime::AgentEngine::with_providers(
        f.db.clone(),
        std::sync::Arc::new(OverviewWithoutVision),
        None,
    );
    engine.run(&task.id).await.unwrap();
    let projection = store::projection(&f.db, &f.p).await.unwrap();
    let result = &projection.tasks[0];
    assert_eq!(result.usage.tool_calls, 2);
    assert_eq!(result.usage.cloud_images, 0);
    assert!(result.board_drafts.is_empty());
    assert!(result.inspected_photos.is_empty());
    assert!(result.reason.contains("没有完成视觉分析"));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photo_observations")
        .fetch_one(&f.db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn visual_overview_does_not_group_different_facts_with_the_same_excerpt_or_count_one_photo_twice()
 {
    let f = Fixture::new(2).await;
    let task = f.start(f.photos.clone(), None, None).await.unwrap();
    f.run(&task).await;
    let version = format!(
        "{}@{}",
        super::super::vision::VISION_PROMPT_VERSION,
        "a".repeat(64)
    );
    for (index, id) in f.photos.iter().enumerate() {
        let (_, hash) = overview_pixels(&f, id).await;
        overview_cache(&f, id, Some(hash), true, "fixture", &version).await;
        let facts = if index == 0 {
            vec![
                format!("{}A", "a".repeat(160)),
                "one photo only".into(),
                "one photo only".into(),
            ]
        } else {
            vec![format!("{}B", "a".repeat(160))]
        };
        sqlx::query("UPDATE photo_observations SET observation_json=? WHERE photo_id=?")
            .bind(json!({"photo_id":id,"visible_facts":facts,"interpretation":"fixture","uncertainty":"fixture"}).to_string())
            .bind(id).execute(&f.db.pool).await.unwrap();
    }
    let page = overview_page(&f, &task, 0, 30).await;
    assert_eq!(page["pageRepeatedFacts"], json!([]));
    assert_eq!(page["items"][0]["factExcerptsTruncated"], true);
    assert_eq!(
        page["nextSteps"]["focusReview"]["higherResolutionAvailable"],
        false
    );
    assert_eq!(
        page["nextSteps"]["focusReview"]["photoIds"],
        json!(f.photos)
    );
}

#[tokio::test]
async fn knowledge_references_explain_the_match_and_keep_cloud_summaries_apart() {
    let f = Fixture::new(0).await;
    sqlx::query("INSERT INTO agent_tasks(id,project_id,goal,base_revision,allowed_ids_json,selected_ids_json,target_count,state,reason,budget_json,usage_json,created_at,updated_at) VALUES('t',?,'引用理由',1,'[]','[]',1,'completed','测试','{}','{}','2026-11-07','2026-11-07')")
        .bind(&f.p).execute(&f.db.pool).await.unwrap();
    let receipt = json!({"callId":"c1","ok":true,"error":null,"observationRefs":[],"durationMs":0,"billingEstimate":0,"result":{"results":[
        {"citationId":"FR-K004","title":"色彩与版本比较","passage":"正文","version":"v1","source":"focalroom://knowledge/original-method-notes","license":"CC0-1.0","contentSha256":"abc",
         "matchReason":{"kind":"local_terms","termCount":2,"matchedTerms":["色彩","调色"],"fields":["title","tags"],"matched":4,"returned":3}},
        {"id":"cloud-one","title":"Shore","kind":"photographer","reviewStatus":"verified","matchReason":{"kind":"cloud_search","relevance":0.5}},
        {"id":"cloud-two","title":"Araki","kind":"photographer","reviewStatus":"draft","matchReason":{"kind":"cloud_search","relevance":null}}
    ]}});
    sqlx::query("INSERT INTO tool_calls VALUES('t','c1','search_photography_knowledge','fp',?)")
        .bind(receipt.to_string())
        .execute(&f.db.pool)
        .await
        .unwrap();
    let mut tx = f.db.pool.begin().await.unwrap();
    let list = references(&mut tx, "t").await.unwrap();
    // 两条云端摘要早先会挤成同一条无名引用；现在各按自己的编号入表。
    assert_eq!(list.len(), 3, "{list:?}");
    let local = list
        .iter()
        .find(|row| row["citationId"] == "FR-K004")
        .expect("本机方法条");
    assert_eq!(local["matchReason"]["kind"], "local_terms");
    assert_eq!(local["matchReason"]["matched"], 4);
    assert_eq!(local["passage"], "正文");
    for (id, label, relevance) in [
        ("cloud-one", "Shore", json!(0.5)),
        ("cloud-two", "Araki", json!(null)),
    ] {
        let row = list
            .iter()
            .find(|row| row["citationId"] == id)
            .expect("云端摘要条");
        assert_eq!(row["label"], label);
        assert_eq!(row["summaryOnly"], true, "摘要不能被当成已取到断言的出处");
        assert!(row.get("passage").is_none(), "{row}");
        assert_eq!(
            row["matchReason"],
            json!({"kind":"cloud_search","relevance":relevance})
        );
        assert_eq!(row["visualEvidence"], false);
    }
}

#[tokio::test]
async fn search_ledger_keeps_empty_unanswered_and_failed_calls_apart() {
    let f = Fixture::new(0).await;
    sqlx::query("INSERT INTO agent_tasks(id,project_id,goal,base_revision,allowed_ids_json,selected_ids_json,target_count,state,reason,budget_json,usage_json,created_at,updated_at) VALUES('t',?,'检索台账',1,'[]','[]',1,'completed','测试','{}','{}','2026-11-07','2026-11-07')")
        .bind(&f.p).execute(&f.db.pool).await.unwrap();
    let ok = |id: &str, result: Value| json!({"callId":id,"ok":true,"error":null,"observationRefs":[],"durationMs":0,"billingEstimate":0,"result":result});
    let rows = [
        (
            "c1",
            "search_photography_knowledge",
            ok(
                "c1",
                json!({"query":"两边都没有","results":[],"localMatched":0,"localReturned":0,"cloudAvailable":true,"cloudError":null}),
            ),
        ),
        (
            "c2",
            "search_photography_knowledge",
            ok(
                "c2",
                json!({"query":"云端没答","results":[],"localMatched":0,"cloudAvailable":false,"cloudError":{"code":"AUTH_REQUIRED","message":"需要当前账户授权"}}),
            ),
        ),
        // 内嵌方法那次只查本地：回执里根本没有云端字段，不能替它说"云端也没有"。
        (
            "c3",
            "search_photography_knowledge",
            ok(
                "c3",
                json!({"query":"只查本地","results":[],"matched":0,"returned":0}),
            ),
        ),
        (
            "c4",
            "search_photographers",
            ok(
                "c4",
                json!({"query":"shore","photographers":[{"name":"Shore","id":"p1"},{"name":"Araki","id":"p2"}],"available":true,"error":null}),
            ),
        ),
        (
            "c5",
            "compare_photography_series",
            ok(
                "c5",
                json!({"query":"A / B","seriesA":{"name":"Americans","id":"s1"},"seriesB":null,"available":false,"errors":{"seriesA":null,"seriesB":{"code":"KNOWLEDGE_HTTP","message":"第二个系列没取到"}}}),
            ),
        ),
        (
            "c6",
            "get_photobook_structure",
            json!({"callId":"c6","ok":false,"result":null,"error":{"code":"KNOWLEDGE_PROTOCOL","retryable":false,"message":"响应不是检索列表"},"observationRefs":[],"durationMs":0,"billingEstimate":0}),
        ),
        // 不是知识检索的回执不进台账。
        ("c7", "inspect_photos", ok("c7", json!({"photos":[]}))),
    ];
    for (id, tool, receipt) in rows {
        sqlx::query("INSERT INTO tool_calls VALUES('t',?,?,'fp',?)")
            .bind(id)
            .bind(tool)
            .bind(receipt.to_string())
            .execute(&f.db.pool)
            .await
            .unwrap();
    }
    let mut tx = f.db.pool.begin().await.unwrap();
    let ledger = searches(&mut tx, "t").await.unwrap();
    assert_eq!(ledger.len(), 6, "{ledger:?}");
    let ids: Vec<_> = ledger
        .iter()
        .map(|row| row["callId"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["c1", "c2", "c3", "c4", "c5", "c6"], "按发生顺序留档");
    for (index, outcome, returned) in [
        (0, "no_match", 0),
        (1, "unavailable", 0),
        (2, "no_match", 0),
        (3, "found", 2),
        (4, "found", 1),
        (5, "failed", 0),
    ] {
        assert_eq!(ledger[index]["outcome"], outcome, "{:?}", ledger[index]);
        assert_eq!(ledger[index]["returned"], returned);
    }
    assert_eq!(ledger[0]["query"], "两边都没有");
    assert_eq!(ledger[0]["localMatched"], 0);
    assert_eq!(ledger[0]["cloudAvailable"], true);
    assert_eq!(ledger[1]["cloudError"]["code"], "AUTH_REQUIRED");
    assert_eq!(
        ledger[1]["error"],
        Value::Null,
        "云端没答上来不等于整次检索失败"
    );
    assert_eq!(ledger[2]["cloudAvailable"], Value::Null);
    assert_eq!(ledger[2]["localMatched"], 0);
    assert_eq!(ledger[3]["localMatched"], Value::Null);
    // 一次比较里只有一边没取到：条数照实报，出错的那一边单独说。
    assert_eq!(ledger[4]["cloudError"]["message"], "第二个系列没取到");
    assert_eq!(ledger[4]["cloudAvailable"], false);
    assert_eq!(ledger[5]["error"]["code"], "KNOWLEDGE_PROTOCOL");
    assert_eq!(ledger[5]["query"], "", "失败回执没留下问法，界面不能编一个");
    assert_eq!(ledger[5]["cloudAvailable"], Value::Null);

    // 三次空手而归在参考来源里一条都没有：台账是它们唯一能被看见的地方。
    let cited = references(&mut tx, "t").await.unwrap();
    assert_eq!(cited.len(), 3, "{cited:?}");
    let mut cited_ids: Vec<_> = cited
        .iter()
        .map(|row| row["citationId"].as_str().unwrap())
        .collect();
    cited_ids.sort();
    assert_eq!(cited_ids, ["p1", "p2", "s1"]);
}
