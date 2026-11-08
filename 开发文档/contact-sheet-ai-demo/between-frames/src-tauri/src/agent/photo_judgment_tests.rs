use super::*;
use crate::{
    agent::{execution, types::*, upload_compress},
    command_handler::CommandHandler,
    db::Store,
    domain,
};

async fn fixture() -> (tempfile::TempDir, Store, Task, String, String) {
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("judgment.sqlite"))
        .await
        .unwrap();
    let p = db.create("判断依据", "比较观看距离").await.unwrap();
    let source = temp.path().join("a.png");
    image::RgbImage::from_pixel(900, 600, image::Rgb([30, 60, 90]))
        .save(&source)
        .unwrap();
    db.insert_photo(
        &p.id,
        &crate::image_pipeline::prepare(&source, &temp.path().join("cache")).unwrap(),
    )
    .await
    .unwrap();
    let photo = db.get(&p.id).await.unwrap().photos[0].id.clone();
    let path: String = sqlx::query_scalar("SELECT preview_path FROM photos WHERE id=?")
        .bind(&photo)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: p.id,
            command: TaskCommand::StartConversation {
                allow_queued_recovery: false,
                expected_account_id: None,
                goal: "解释这张照片".into(),
                selected_photo_ids: vec![photo.clone()],
                focus_photo_ids: vec![],
                detail_photo_ids: vec![],
                target_count: 1,
                budget: Budget::default(),
                consent_id: None,
                source_board: None,
                continuation: None,
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
    let hash: String = sqlx::query_scalar("SELECT content_sha256 FROM photos WHERE id=?")
        .bind(&photo)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let preview = upload_compress::compress_and_fingerprint(std::path::Path::new(&path))
        .unwrap()
        .1;
    seed(
        &db,
        &task,
        "observation",
        "inspect_photos",
        seen(
            &photo,
            &hash,
            Some(preview.as_str()),
            &["人物位于画面中央", "背景明亮"],
        ),
    )
    .await;
    seed(&db, &task, "method", "search_photography_knowledge", json!({"results":[{"citationId":"FR-TEST","title":"原创方法","passage":"比较主体与背景的关系，而非机械套构图规则","author":"FocalRoom","license":"fixture","version":"1","source":"focalroom://fixture"}]})).await;
    (temp, db, task, photo, path)
}
/// 一份结构完整、像素来路也齐全的观察回执；每个用例只把要测的那一环改坏。
/// `preview=None` 对应旧缓存里从未记录预览字节指纹的情形。
fn seen(photo: &str, hash: &str, preview: Option<&str>, facts: &[&str]) -> Value {
    json!({"mode":"vision_observed","observations":[{"photo_id":photo,"visible_facts":facts,"interpretation":"可能成为停顿","uncertainty":"表情看不清"}],"provenance":[{"photo_id":photo,"photo_hash":hash,"preview_sha256":preview,"prompt_version":super::super::vision::VISION_PROMPT_VERSION}]})
}
async fn pixels(db: &Store, photo: &str) -> (String, String) {
    let hash: String = sqlx::query_scalar("SELECT content_sha256 FROM photos WHERE id=?")
        .bind(photo)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let path: String = sqlx::query_scalar("SELECT preview_path FROM photos WHERE id=?")
        .bind(photo)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    (
        hash,
        upload_compress::compress_and_fingerprint(std::path::Path::new(&path))
            .unwrap()
            .1,
    )
}
async fn row(db: &Store, task: &Task, id: &str, name: &str, receipt: Value) {
    sqlx::query("INSERT INTO tool_calls(task_id,call_id,tool_name,fingerprint,result_json) VALUES(?,?,?,?,?)")
        .bind(&task.id).bind(id).bind(name).bind("fixture").bind(receipt.to_string()).execute(&db.pool).await.unwrap();
}
async fn seed(db: &Store, task: &Task, id: &str, name: &str, result: Value) {
    row(db, task, id, name, json!({"callId":id,"ok":true,"result":result,"error":null,"observationRefs":[],"durationMs":0,"billingEstimate":0})).await;
}
async fn seed_failed(db: &Store, task: &Task, id: &str, name: &str) {
    row(db, task, id, name, json!({"callId":id,"ok":false,"result":null,"error":{"code":"TOOL_TIMEOUT","retryable":true,"message":"这次调用失败了"},"observationRefs":[],"durationMs":0,"billingEstimate":0})).await;
}
fn args(photo: &str) -> Value {
    json!({"photo_id":photo,"observation_call_id":"observation","fact_indices":[0],"judgment":"主体与背景的关系可能不够明确","reasoning":"中心位置与明亮背景可能竞争注意力；需与相邻照片比较","knowledge_refs":[{"call_id":"method","citation_id":"FR-TEST","claim_index":null}],"uncertainty":"仅为预览观察，不能判断拍摄意图"})
}
async fn apply(db: &Store, task: &Task, mut args: Value, id: &str) -> ToolResult {
    args["project_id"] = json!(task.project_id);
    execution::tool(
        db,
        &task.id,
        &ToolCall {
            call_id: id.into(),
            name: "explain_photo_judgment".into(),
            args,
        },
        None,
    )
    .await
    .unwrap()
}

fn reason(result: &ToolResult) -> String {
    result.error.as_ref().unwrap().code.clone()
}
/// 拒绝不仅要失败，还要指名断掉的那一环：模型和用户都靠这个编号决定下一步做什么。
async fn refused(db: &Store, task: &Task, input: Value, id: &str, code: &str) -> ToolResult {
    let result = apply(db, task, input, id).await;
    assert!(!result.ok, "{id} 本应被拒绝，却成功了：{:?}", result.result);
    assert_eq!(reason(&result), code, "回执里的说法：{:?}", result.error);
    result
}

#[tokio::test]
async fn judgment_copies_real_facts_and_sources_and_replays_without_changing_work() {
    let (_temp, db, task, photo, _path) = fixture().await;
    let before = serde_json::to_value(db.get(&task.project_id).await.unwrap()).unwrap();
    let first = apply(&db, &task, args(&photo), "judgment").await;
    assert!(first.ok, "{first:?}");
    let data = first.result.as_ref().unwrap();
    assert_eq!(
        data["facts"],
        json!([{"index":0,"text":"人物位于画面中央"}])
    );
    assert_eq!(
        data["methods"][0]["passage"],
        "比较主体与背景的关系，而非机械套构图规则"
    );
    assert_eq!(data["inference"], true);
    assert_eq!(
        serde_json::to_value(apply(&db, &task, args(&photo), "judgment").await).unwrap(),
        serde_json::to_value(first).unwrap()
    );
    let mut tx = db.pool.begin().await.unwrap();
    let view = projection(&mut tx, &task).await.unwrap();
    assert_eq!(view.len(), 1);
    assert_eq!(view[0]["imageStatus"], "verified");
    assert!(view[0].get("provenance").is_none());
    tx.rollback().await.unwrap();
    assert_eq!(
        serde_json::to_value(db.get(&task.project_id).await.unwrap()).unwrap(),
        before
    );
}

#[tokio::test]
async fn invented_facts_methods_foreign_receipts_and_metadata_are_refused() {
    let (_temp, db, task, photo, _path) = fixture().await;
    let base = args(&photo);
    let variants = [
        ("photo_id", json!("foreign"), "JUDGMENT_SCOPE"),
        (
            "observation_call_id",
            json!("missing"),
            "JUDGMENT_RECEIPT_MISSING",
        ),
        ("fact_indices", json!([2]), "JUDGMENT_FACT_INDEX"),
        ("fact_indices", json!([0, 0]), "VALIDATION_ERROR"),
        (
            "knowledge_refs",
            json!([{"call_id":"method","citation_id":"INVENTED","claim_index":null}]),
            "JUDGMENT_CITATION_UNKNOWN",
        ),
        (
            "knowledge_refs",
            json!([{"call_id":"missing","citation_id":"FR-TEST","claim_index":null}]),
            "JUDGMENT_RECEIPT_MISSING",
        ),
    ];
    for (i, (key, value, code)) in variants.into_iter().enumerate() {
        let mut input = base.clone();
        input[key] = value;
        refused(&db, &task, input, &format!("invalid-{i}"), code).await;
    }
    let foreign_project = db.create("其他项目", "不授予当前任务").await.unwrap();
    db.insert_photo(
        &foreign_project.id,
        &crate::image_pipeline::prepare(
            std::path::Path::new(&_path),
            &_temp.path().join("foreign-cache"),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let foreign_task = CommandHandler::new(&db)
        .agent_request(AgentRequest {
            request_id: domain::id(),
            project_id: foreign_project.id,
            command: TaskCommand::StartTask {
                goal: "其他任务".into(),
                selected_photo_ids: vec![],
                target_count: 1,
                budget: Budget::default(),
                consent_id: None,
            },
        })
        .await
        .unwrap();
    let raw: String = sqlx::query_scalar(
        "SELECT result_json FROM tool_calls WHERE task_id=? AND call_id='observation'",
    )
    .bind(&task.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let observation: ToolResult = serde_json::from_str(&raw).unwrap();
    seed(
        &db,
        &foreign_task,
        "foreign-observation",
        "inspect_photos",
        observation.result.unwrap(),
    )
    .await;
    let mut input = base.clone();
    input["observation_call_id"] = json!("foreign-observation");
    // 别的任务里确实有这条真回执，但在本任务查不到——按"本任务没有这个编号"报，不暗示回执造假。
    refused(
        &db,
        &task,
        input,
        "foreign-judgment",
        "JUDGMENT_RECEIPT_MISSING",
    )
    .await;
    seed(
        &db,
        &task,
        "metadata",
        "inspect_photos",
        json!({"mode":"metadata_only"}),
    )
    .await;
    let mut input = base;
    input["observation_call_id"] = json!("metadata");
    refused(
        &db,
        &task,
        input,
        "metadata-judgment",
        "JUDGMENT_METADATA_ONLY",
    )
    .await;
}

#[tokio::test]
async fn changed_pixels_mark_historical_judgments_and_block_new_judgments() {
    let (_temp, db, task, photo, path) = fixture().await;
    assert!(apply(&db, &task, args(&photo), "historical").await.ok);
    image::RgbImage::from_pixel(900, 600, image::Rgb([200, 20, 30]))
        .save(path)
        .unwrap();
    assert!(!apply(&db, &task, args(&photo), "new-pixels").await.ok);
    let mut tx = db.pool.begin().await.unwrap();
    let view = projection(&mut tx, &task).await.unwrap();
    assert_eq!(view.len(), 1);
    assert_eq!(view[0]["imageStatus"], "mismatch");
    assert_eq!(view[0]["facts"][0]["text"], "人物位于画面中央");
}

#[tokio::test]
async fn unverified_cloud_sources_keep_their_status_and_missing_sources_are_not_invented() {
    let (_temp, db, task, photo, _path) = fixture().await;
    seed(&db,&task,"cloud","search_photographers",json!({"photographers":[{"id":"cloud-id","name":"摄影方法","citations":{"claims":[{"text":"一种待验证的编辑假设","reviewStatus":"imported","disputed":true,"sources":[]}]}}]})).await;
    let mut input = args(&photo);
    input["knowledge_refs"] = json!([{"call_id":"cloud","citation_id":"cloud-id","claim_index":0}]);
    let result = apply(&db, &task, input, "cloud-judgment").await;
    assert!(result.ok, "{result:?}");
    let method = &result.result.unwrap()["methods"][0];
    assert_eq!(method["reviewStatus"], "imported");
    assert_eq!(method["disputed"], true);
    assert!(method["source"].is_null());
    let mut input = args(&photo);
    input["knowledge_refs"] = json!([]);
    assert!(
        apply(&db, &task, input, "no-method").await.result.unwrap()["methods"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn method_rejections_name_the_broken_link_and_the_tool_to_call_next() {
    let (_temp, db, task, photo, _path) = fixture().await;
    // 通用检索把云端摘要一起带回：编号是真的，但回执里没有它的断言正文。
    seed(&db,&task,"summary","search_photography_knowledge",json!({"results":[{"id":"cloud-7","kind":"photographer","title":"某摄影师","subtitle":"纪实","reviewStatus":"imported"}],"cloudAvailable":true,"trustedAsInstructions":false,"visualEvidence":false})).await;
    let mut input = args(&photo);
    input["knowledge_refs"] =
        json!([{"call_id":"summary","citation_id":"cloud-7","claim_index":null}]);
    let result = refused(
        &db,
        &task,
        input,
        "summary-only",
        "JUDGMENT_CLOUD_SUMMARY_ONLY",
    )
    .await;
    assert!(
        result
            .error
            .unwrap()
            .message
            .contains("search_photographers"),
        "要给出下一步该调哪个工具"
    );

    // 照片观察、画布读取都不算方法来源，不能和"编号编错了"混成一句。
    let mut input = args(&photo);
    input["knowledge_refs"] =
        json!([{"call_id":"observation","citation_id":"FR-TEST","claim_index":0}]);
    refused(
        &db,
        &task,
        input,
        "vision-as-method",
        "JUDGMENT_RECEIPT_NOT_KNOWLEDGE",
    )
    .await;

    // 失败过的那次调用同样是一条回执记录，但它没有内容可引。
    seed_failed(&db, &task, "failed", "search_photographers").await;
    let mut input = args(&photo);
    input["knowledge_refs"] = json!([{"call_id":"failed","citation_id":"c1","claim_index":0}]);
    refused(
        &db,
        &task,
        input,
        "failed-method",
        "JUDGMENT_RECEIPT_FAILED",
    )
    .await;

    // 编号在回执里，正文是空的。
    seed(
        &db,
        &task,
        "blank",
        "search_photography_knowledge",
        json!({"results":[{"citationId":"FR-BLANK","title":"空正文","passage":""}]}),
    )
    .await;
    let mut input = args(&photo);
    input["knowledge_refs"] =
        json!([{"call_id":"blank","citation_id":"FR-BLANK","claim_index":null}]);
    refused(&db, &task, input, "empty-passage", "JUDGMENT_METHOD_EMPTY").await;

    // 序号越界要说清那次回执里到底有几条。
    seed(&db,&task,"cloud","search_photographers",json!({"photographers":[{"id":"c1","name":"一条断言","citations":{"claims":[{"text":"正文","reviewStatus":"imported","disputed":false,"sources":[]}]},"detailError":null}]})).await;
    let mut input = args(&photo);
    input["knowledge_refs"] = json!([{"call_id":"cloud","citation_id":"c1","claim_index":4}]);
    let result = refused(
        &db,
        &task,
        input,
        "claim-out-of-range",
        "JUDGMENT_CLAIM_INDEX",
    )
    .await;
    assert!(result.error.unwrap().message.contains("只有 1 条断言"));

    // 详情没取得与排在补取范围之外是两件事：前者要重新检索，后者断言正文本来就没带回来。
    seed(&db,&task,"errs","search_photographers",json!({"photographers":[{"id":"e1","name":"详情失败","citations":null,"detailError":{"code":"KNOWLEDGE_PROTOCOL","message":"断言详情未取得"}}]})).await;
    let mut input = args(&photo);
    input["knowledge_refs"] = json!([{"call_id":"errs","citation_id":"e1","claim_index":0}]);
    refused(
        &db,
        &task,
        input,
        "detail-missing",
        "JUDGMENT_DETAIL_UNAVAILABLE",
    )
    .await;
    seed(&db,&task,"skip","search_photographers",json!({"photographers":[{"id":"s1","name":"没补取详情的第三条","citations":null,"detailError":null}]})).await;
    let mut input = args(&photo);
    input["knowledge_refs"] = json!([{"call_id":"skip","citation_id":"s1","claim_index":0}]);
    let result = refused(&db, &task, input, "no-claims", "JUDGMENT_CLAIM_INDEX").await;
    assert!(
        result
            .error
            .unwrap()
            .message
            .contains("没有可引用的断言正文")
    );

    // 编号根本不在那次回执里。
    let mut input = args(&photo);
    input["knowledge_refs"] =
        json!([{"call_id":"cloud","citation_id":"NOT-RETURNED","claim_index":0}]);
    refused(
        &db,
        &task,
        input,
        "unknown-citation",
        "JUDGMENT_CITATION_UNKNOWN",
    )
    .await;
}

#[tokio::test]
async fn observation_rejections_separate_absent_receipts_from_unusable_observations() {
    let (_temp, db, task, photo, _path) = fixture().await;
    let (hash, preview) = pixels(&db, &photo).await;

    // 知识检索回执不是照片观察。
    let mut input = args(&photo);
    input["observation_call_id"] = json!("method");
    refused(
        &db,
        &task,
        input,
        "knowledge-as-observation",
        "JUDGMENT_RECEIPT_NOT_VISION",
    )
    .await;

    seed_failed(&db, &task, "failed", "inspect_photos").await;
    let mut input = args(&photo);
    input["observation_call_id"] = json!("failed");
    refused(
        &db,
        &task,
        input,
        "failed-observation",
        "JUDGMENT_RECEIPT_FAILED",
    )
    .await;

    // 回执存在但超过单份读取上限：过去被报成"没有这条回执"，模型只能原样重试并继续消耗预算。
    seed(&db,&task,"huge","inspect_photos",json!({"mode":"vision_observed","observations":[{"photo_id":photo,"visible_facts":["人物位于画面中央"],"interpretation":"可能成为停顿","uncertainty":"表情看不清"}],"provenance":[],"bulk":"字".repeat(30_000)})).await;
    let mut input = args(&photo);
    input["observation_call_id"] = json!("huge");
    let result = refused(
        &db,
        &task,
        input,
        "oversized-observation",
        "JUDGMENT_RECEIPT_TOO_LARGE",
    )
    .await;
    assert!(result.error.unwrap().message.contains("上限"));

    // 观察确实做过，只是那一批里没有这张照片。
    seed(
        &db,
        &task,
        "other",
        "inspect_photos",
        seen("别的照片", &hash, Some(preview.as_str()), &["画面偏暗"]),
    )
    .await;
    let mut input = args(&photo);
    input["observation_call_id"] = json!("other");
    refused(
        &db,
        &task,
        input,
        "photo-not-observed",
        "JUDGMENT_NOT_OBSERVED",
    )
    .await;

    // 观察记录结构不合法（一条可见事实也没有）。
    seed(
        &db,
        &task,
        "nofacts",
        "inspect_photos",
        seen(&photo, &hash, Some(preview.as_str()), &[]),
    )
    .await;
    let mut input = args(&photo);
    input["observation_call_id"] = json!("nofacts");
    refused(
        &db,
        &task,
        input,
        "broken-observation",
        "JUDGMENT_OBSERVATION_INVALID",
    )
    .await;

    // 有观察但没有像素来路，无从核对是在哪一份画面上看出来的。
    let mut homeless = seen(&photo, &hash, Some(preview.as_str()), &["人物位于画面中央"]);
    homeless["provenance"] = json!([]);
    seed(&db, &task, "homeless", "inspect_photos", homeless).await;
    let mut input = args(&photo);
    input["observation_call_id"] = json!("homeless");
    refused(
        &db,
        &task,
        input,
        "no-provenance",
        "JUDGMENT_PROVENANCE_MISSING",
    )
    .await;

    // 旧缓存的观察从未记录预览字节指纹。
    seed(
        &db,
        &task,
        "oldcache",
        "inspect_photos",
        seen(&photo, &hash, None, &["人物位于画面中央"]),
    )
    .await;
    let mut input = args(&photo);
    input["observation_call_id"] = json!("oldcache");
    refused(
        &db,
        &task,
        input,
        "unrecorded-version",
        "JUDGMENT_PIXEL_UNRECORDED",
    )
    .await;

    // 界面两侧只认这几种像素状态，旧缓存的历史判断仍按"版本未记录"展示。
    // 回执里的 provenance 是 explain 从数组里挑出的那一条，形状与 explain 的产物一致。
    let mut past = seen(&photo, &hash, None, &["人物位于画面中央"]);
    past["photoId"] = json!(photo);
    past["observationCallId"] = json!("oldcache");
    past["judgment"] = json!("主体与背景的关系可能不够明确");
    past["reasoning"] = json!("中心位置与明亮背景可能竞争注意力");
    past["uncertainty"] = json!("仅为预览观察");
    past["observationUncertainty"] = json!("表情看不清");
    let single = past["provenance"][0].clone();
    past["provenance"] = single;
    past["facts"] = json!([{"index":0,"text":"人物位于画面中央"}]);
    past["methods"] = json!([]);
    past["inference"] = json!(true);
    seed(&db, &task, "past", "explain_photo_judgment", past).await;
    let mut tx = db.pool.begin().await.unwrap();
    let view = projection(&mut tx, &task).await.unwrap();
    assert_eq!(view[0]["imageStatus"], "unrecorded");
    assert!(view[0].get("provenance").is_none());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn pixel_rejections_tell_apart_changed_preview_changed_original_and_lost_cache() {
    let (_temp, db, task, photo, path) = fixture().await;
    assert!(apply(&db, &task, args(&photo), "historical").await.ok);

    // 只有派生预览的字节变了（缓存被重建），原图没动。
    image::RgbImage::from_pixel(900, 600, image::Rgb([200, 20, 30]))
        .save(&path)
        .unwrap();
    let result = refused(
        &db,
        &task,
        args(&photo),
        "preview-drift",
        "JUDGMENT_PREVIEW_CHANGED",
    )
    .await;
    assert!(
        !result.error.unwrap().message.contains("原图"),
        "预览变了不能说要重新导入原图"
    );

    // 预览文件读不出来时，不能说成"像素变了"。
    std::fs::remove_file(&path).unwrap();
    refused(
        &db,
        &task,
        args(&photo),
        "preview-gone",
        "JUDGMENT_PREVIEW_UNREADABLE",
    )
    .await;

    // 原图内容真的被改写，这才是要重新观察之外还得重新导入的那一档。
    sqlx::query("UPDATE photos SET content_sha256=? WHERE id=?")
        .bind("0000000000000000000000000000000000000000000000000000000000000000")
        .bind(&photo)
        .execute(&db.pool)
        .await
        .unwrap();
    refused(
        &db,
        &task,
        args(&photo),
        "original-drift",
        "JUDGMENT_ORIGINAL_CHANGED",
    )
    .await;
    let mut tx = db.pool.begin().await.unwrap();
    let view = projection(&mut tx, &task).await.unwrap();
    assert_eq!(view[0]["imageStatus"], "mismatch");
    tx.rollback().await.unwrap();
}
