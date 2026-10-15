// v2.0 M2 执行桥：v2 envelope → local-domain 鉴权 → v1 CommandHandler 真实执行。
// 覆盖：owner 写入/幂等重放、序列 move、Agent principal 只能提案（正式写入拒绝）、
// 版本冲突 409、M5 范围命令诚实 NOT_IMPLEMENTED、v1 历史与 Undo 语义继承。
use between_frames_lib::{
    db::Store,
    image_pipeline,
    v2_bridge::{self, V2Execution},
};
use betweenframes_local_domain::parse_command;
use serde_json::{Value, json};
use std::path::Path;

async fn setup(path: &Path, count: usize) -> Store {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let project = db.create("v2 桥", "").await.unwrap();
    for i in 0..count {
        let source = path.join(format!("p{i}.png"));
        image::RgbImage::from_pixel(32, 24, image::Rgb([(i * 37 % 256) as u8, 90, 130]))
            .save(&source)
            .unwrap();
        db.insert_photo(
            &project.id,
            &image_pipeline::prepare(&source, &path.join("cache")).unwrap(),
        )
        .await
        .unwrap();
    }
    db
}

fn envelope(kind: &str, payload: Value, revision: i64, key: &str) -> Value {
    json!({
        "version": "2.0",
        "command_id": format!("cmd-{key}"),
        "workspace_id": "local-workspace",
        "project_id": "",
        "expected_revision": revision,
        "idempotency_key": format!("idem-{key}"),
        "kind": kind,
        "payload": payload,
    })
}
async fn run(
    db: &Store,
    command: Value,
    principal: betweenframes_local_domain::Principal,
) -> Value {
    let validated = parse_command(command.clone())
        .unwrap_or_else(|code| panic!("契约不合法 {code}：{command}"));
    let authority = v2_bridge::authority_for(db, command["project_id"].as_str().unwrap())
        .await
        .unwrap();
    v2_bridge::execute(
        db,
        V2Execution {
            envelope: validated,
            principal,
            authority,
        },
        1_800_000_000_000,
    )
    .await
}

async fn prepare(db: &Store) -> (String, Vec<String>, i64) {
    let project = db.list(false).await.unwrap().into_iter().next().unwrap();
    let ids: Vec<String> = project.photos.iter().map(|p| p.id.clone()).collect();
    (project.id, ids, project.revision)
}

#[tokio::test]
async fn owner_selection_executes_with_idempotent_replay_and_v1_history() {
    let temp = tempfile::tempdir().unwrap();
    let db = setup(temp.path(), 3).await;
    let (project_id, ids, revision) = prepare(&db).await;
    let principal = v2_bridge::local_owner_principal("local-workspace", &project_id);
    let command = envelope(
        "photo.set_selection",
        json!({"photo_ids": [ids[0]], "selection_state": "KEEP"}),
        revision,
        "selection-0001",
    );
    // project_id 需要真实值：构造再改。
    let mut command = command;
    command["project_id"] = json!(project_id);
    let first = run(&db, command.clone(), principal.clone()).await;
    assert!(first["ok"].as_bool().unwrap(), "owner 写入应执行：{first}");
    assert!(first["revision"].as_i64().unwrap() > revision);
    // 幂等重放：同 command_id 返回原结果，不重复推进 revision。
    let replay = run(&db, command, principal).await;
    assert!(replay["ok"].as_bool().unwrap(), "重放结果：{replay}");
    assert_eq!(replay["revision"], first["revision"]);
    // v1 语义继承：命令进入历史，可撤销。
    let project = db.get(&project_id).await.unwrap();
    assert_eq!(project.sequence, vec![ids[0].clone()]);
    assert!(project.history.undo_count >= 1);
}

#[tokio::test]
async fn sequence_move_reorders_and_remove_leaves_keep_set() {
    let temp = tempfile::tempdir().unwrap();
    let db = setup(temp.path(), 3).await;
    let (project_id, ids, revision) = prepare(&db).await;
    let principal = v2_bridge::local_owner_principal("local-workspace", &project_id);
    // 先全部 keep。
    let mut all = envelope(
        "photo.set_selection",
        json!({"photo_ids": ids, "selection_state": "KEEP"}),
        revision,
        "keep-all-batch",
    );
    all["project_id"] = json!(project_id);
    let kept = run(&db, all, principal.clone()).await;
    assert!(kept["ok"].as_bool().unwrap());
    let revision = kept["revision"].as_i64().unwrap();
    // move 第 0 张到末尾。
    let mut move_cmd = envelope(
        "sequence.move_item",
        json!({"sequence_id": project_id, "item_id": ids[0], "to_index": 2}),
        revision,
        "move-item-01",
    );
    move_cmd["project_id"] = json!(project_id);
    let moved = run(&db, move_cmd, principal.clone()).await;
    assert!(moved["ok"].as_bool().unwrap(), "{moved}");
    let project = db.get(&project_id).await.unwrap();
    assert_eq!(
        project.sequence,
        vec![ids[1].clone(), ids[2].clone(), ids[0].clone()]
    );
    // remove 中间一张 → 移出序列、状态待定。
    let revision = moved["revision"].as_i64().unwrap();
    let mut remove_cmd = envelope(
        "sequence.remove_item",
        json!({"sequence_id": project_id, "item_id": ids[2]}),
        revision,
        "remove-item-01",
    );
    remove_cmd["project_id"] = json!(project_id);
    let removed = run(&db, remove_cmd, principal).await;
    assert!(removed["ok"].as_bool().unwrap(), "{removed}");
    let project = db.get(&project_id).await.unwrap();
    assert_eq!(project.sequence, vec![ids[1].clone(), ids[0].clone()]);
    assert_eq!(
        project
            .photos
            .iter()
            .find(|p| p.id == ids[2])
            .unwrap()
            .status,
        "undecided"
    );
}

#[tokio::test]
async fn agent_principal_cannot_write_formal_state() {
    let temp = tempfile::tempdir().unwrap();
    let db = setup(temp.path(), 2).await;
    let (project_id, ids, revision) = prepare(&db).await;
    let agent = v2_bridge::agent_principal("local-workspace", &project_id, 4_102_444_800_000);
    let mut command = envelope(
        "photo.set_selection",
        json!({"photo_ids": [ids[0]], "selection_state": "KEEP"}),
        revision,
        "agent-forbidden",
    );
    command["project_id"] = json!(project_id);
    let result = run(&db, command, agent).await;
    assert!(!result["ok"].as_bool().unwrap());
    assert_eq!(result["error"]["code"], "PERMISSION_DENIED");
    // v2 分支轨迹第 4 条：Agent 停止/越权后无正式写入。
    let project = db.get(&project_id).await.unwrap();
    assert!(project.sequence.is_empty());
}

#[tokio::test]
async fn revision_conflict_maps_to_409_and_m5_kinds_stay_not_implemented() {
    let temp = tempfile::tempdir().unwrap();
    let db = setup(temp.path(), 1).await;
    let (project_id, ids, revision) = prepare(&db).await;
    let owner = v2_bridge::local_owner_principal("local-workspace", &project_id);
    // 旧版本号 → 409。
    let mut stale = envelope(
        "photo.set_selection",
        json!({"photo_ids": [ids[0]], "selection_state": "KEEP"}),
        revision - 1,
        "stale-revision-1",
    );
    stale["project_id"] = json!(project_id);
    let result = run(&db, stale, owner.clone()).await;
    assert!(!result["ok"].as_bool().unwrap());
    assert_eq!(result["error"]["code"], "REVISION_CONFLICT");
    // M5 审批提案仍未实现（评论已在本切片交付）：诚实拒绝，不假装成功。
    let mut proposal = envelope(
        "proposal.create",
        json!({"base_revision": revision, "title": "t", "operations": [{"operation_id": "op-01", "photo_id": "photo-none", "op": "remove"}]}),
        revision,
        "proposal-will-fail",
    );
    proposal["project_id"] = json!(project_id);
    let result = run(&db, proposal, owner).await;
    assert!(!result["ok"].as_bool().unwrap());
    assert_eq!(result["error"]["code"], "NOT_IMPLEMENTED");
}
