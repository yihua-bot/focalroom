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
                    expected_account_id: identity.and_then(|i| i.account_id.clone()),
                    goal: "我希望这个项目保留留白和观看距离的变化".into(),
                    selected_photo_ids: scope,
                    focus_photo_ids: vec![],
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
        "{}; usage={:?}",
        view.tasks[0].reason, view.tasks[0].usage
    );
    assert_eq!(view.tasks[0].board_drafts.len(), 3);
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
