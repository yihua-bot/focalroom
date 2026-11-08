//! 用户点「按现在这一版重算口径」时走的独立通道。
//!
//! 重算复用任务里那条本地评估工具：不经过模型、不上传照片，只是把库里存着的画面事实和
//! 现在的预览文件再比一遍——所以修好坏预览之后不必重新问模型，口径就能跟上。
//! 新回执仍然只写 `tool_calls`：界面上的口径本来就是从这张表读的，另存一份只会造出两处真相。

use super::{
    store, tools,
    types::{ToolCall, ToolResult},
};
use crate::{
    command_audit::{encode, fingerprint, serialization},
    db::Store,
    domain::{self, AppError, Result},
};
use serde_json::{Value, json};

/// 一轮最多重算多少份方案：每份都要把它那些照片的本地事实再读一遍，所以和修图一样按轮走。
pub const ROUND_LIMIT: usize = 200;
/// 欠账点名每页取多少份方案：翻过头就在结果里说清点了多少，不假装点完了。
const DRAFT_SCAN_LIMIT: i64 = 2000;
/// 按照片点名方案时每页的规模上限：超出就在结果里说清点到多少，不假装点完了。
const TARGET_SCAN_LIMIT: i64 = 2000;
/// 一轮最多翻几页。这是**上限**而不是"一定翻这么多"：翻到头仍旧按已点到的份数报，
/// 剩下的那份差额由 `recheckRemaining` 继续认成没跟上，不拿页数冒充点完了整个项目。
const SCAN_MAX_PAGES: usize = 8;

/// 只回计数和版本号：观察正文一律不往外投，界面重新取投影才算数。
pub async fn run(db: &Store, project_id: &str, draft_id: &str) -> Result<Value> {
    let owner =
        sqlx::query_as::<_, (String, String)>("SELECT project_id, task_id FROM drafts WHERE id=?")
            .bind(draft_id)
            .fetch_optional(&db.pool)
            .await?;
    let (draft_project, task_id) = match owner {
        None => return Err(AppError::new("NOT_FOUND", "方案已不在库里，请刷新后重试")),
        Some(pair) => pair,
    };
    if draft_project != project_id {
        return Err(AppError::new("PERMISSION_DENIED", "方案不属于这个项目"));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut task = store::load(&mut tx, &task_id).await?;
    let summary = record(&mut tx, &mut task, draft_id).await?;
    tx.commit().await?;
    Ok(summary)
}

/// 在**已经开好的那条事务**里跑一次本地评估并把回执写进 `tool_calls`，不提交。
///
/// 画布工具生成序列草案时占着的就是这条事务的写锁：这里要是图省事另开一条连接去写回执
/// （`run` 那种走法），WAL 下会一直等外层释放写锁，等于把画布工具卡死。
/// 所以取数与写回执都顺着传入的 tx 走，只有一份实现。
pub(crate) async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    task: &mut super::types::Task,
    draft_id: &str,
) -> Result<Value> {
    let call = ToolCall {
        call_id: format!("recheck-{}", domain::id()),
        name: "evaluate_draft".into(),
        args: json!({"project_id": task.project_id, "draft_id": draft_id}),
    };
    // 画布工具会经由 execute 再走回这里，async 自递归必须靠一层 Box::pin 断开，
    // 否则 futures 的尺寸算不出来，编译直接拒绝。
    let evaluation = Box::pin(tools::execute(tx, task, &call, None, None)).await?;
    let count = |value: &Value| value.as_i64().filter(|n| *n >= 0).unwrap_or_default();
    let summary = json!({
        "draftId": draft_id,
        "draftRevision": evaluation["draft_revision"],
        "count": count(&evaluation["count"]),
        "observed": count(&evaluation["metrics"]["observed"]),
        "unobserved": count(&evaluation["metrics"]["unobserved"]),
        "omitted": evaluation["omitted_visual_evidence"]
            .as_array()
            .map_or(0, Vec::len),
    });
    let result = ToolResult::success(&call.call_id, evaluation);
    sqlx::query("INSERT INTO tool_calls VALUES(?,?,?,?,?)")
        .bind(&task.id)
        .bind(&call.call_id)
        .bind(&call.name)
        .bind(fingerprint(
            &serde_json::to_value(&call).map_err(serialization)?,
        ))
        .bind(encode(&result)?)
        .execute(&mut **tx)
        .await?;
    Ok(summary)
}

/// 两种点名共用的一份翻页取数，按"改动时间新→旧、同一时刻按编号小→大"往回翻。
///
/// 排序键必须和投影那边读的是同一套顺序，否则同一份方案可能在两页之间被跳过或者被取两次。
/// 游标那道条件也不能图省事写成行值比较 `(updated_at,id) < (游标,游标)`：行值里两个字段同向，
/// 而这里的排序是一降一升，同一时刻里编号更大的那份会被整页漏掉。
async fn scan(
    db: &Store,
    project_id: &str,
    photo_ids: Option<&[String]>,
    page: i64,
) -> Result<Vec<(String, i64)>> {
    let mut rows: Vec<(String, i64)> = Vec::new();
    let mut cursor: Option<(String, String)> = None;
    for _ in 0..SCAN_MAX_PAGES {
        let statement = match photo_ids {
            None => "SELECT id,revision,updated_at FROM drafts WHERE project_id=? \
                    AND (? IS NULL OR updated_at<? OR (updated_at=? AND id>?)) \
                    ORDER BY updated_at DESC,id LIMIT ?"
                .to_owned(),
            Some(ids) => format!(
                "SELECT DISTINCT d.id,d.revision,d.updated_at FROM drafts d \
                 JOIN draft_items i ON i.draft_id=d.id \
                 WHERE d.project_id=? AND i.photo_id IN ({}) \
                 AND (? IS NULL OR d.updated_at<? OR (d.updated_at=? AND d.id>?)) \
                 ORDER BY d.updated_at DESC,d.id LIMIT ?",
                vec!["?"; ids.len()].join(",")
            ),
        };
        let mut query = sqlx::query_as::<_, (String, i64, String)>(&statement).bind(project_id);
        for photo_id in photo_ids.unwrap_or_default() {
            query = query.bind(photo_id);
        }
        let at = cursor.as_ref().map(|(at, _)| at.clone());
        let after = cursor.as_ref().map(|(_, id)| id.clone());
        let page_rows = query
            .bind(at.clone())
            .bind(at.clone())
            .bind(at)
            .bind(after)
            .bind(page)
            .fetch_all(&db.pool)
            .await?;
        let short = (page_rows.len() as i64) < page;
        for (id, revision, updated_at) in page_rows {
            cursor = Some((updated_at, id.clone()));
            rows.push((id, revision));
        }
        if short {
            break;
        }
    }
    Ok(rows)
}

/// 这个项目里哪些方案的口径还欠一次重算，并按轮把欠的补上。
///
/// 欠账只按库里的事实判：投影读不到这份方案的口径，或者读到的那条记着另一个方案版本号。
/// 原来按"最近改过的 20 份"截断，更早的方案修完预览后口径仍旧过期，而回执里看不出还欠着多少，
/// 界面上那句"已重算"就会被当成全都跟上了。这里把欠账数与这一轮之后还差多少一并报出去。
pub async fn project(db: &Store, project_id: &str, limit: usize) -> Result<Value> {
    project_page(db, project_id, limit, DRAFT_SCAN_LIMIT).await
}

/// `page` 是每页取多少份：翻页这件事得能在一个小项目里被验证，所以留成参数——
/// 生产入口固定用 `DRAFT_SCAN_LIMIT`，用例把页取小，才能证明第一页之外欠着的方案也真的被点到名。
async fn project_page(db: &Store, project_id: &str, limit: usize, page: i64) -> Result<Value> {
    db.get(project_id).await?;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM drafts WHERE project_id=?")
        .bind(project_id)
        .fetch_one(&db.pool)
        .await?;
    let drafts = scan(db, project_id, None, page).await?;
    // 欠账只要"这份方案最新的口径记着哪一版"，那几个人家的字段在库里就地取，整条核验单一次也不搬回来。
    // 原来这里也跟投影一样拿"整条不超过 64KB"当门：几百张的画布一眼就能顶破，顶破的那份每轮都被
    // 算成欠着，可重算又不会让核验单变小——于是不管修多少预览，这一轮都白跑一遍还报"仍欠着"。
    // 判定只按刚点到的那份名单问，不再拿"全项目最新几千条"当窗口：名单外的回执一条也不必读回来。
    let ids: Vec<String> = drafts.iter().map(|(id, _)| id.clone()).collect();
    let recorded = super::receipts::recorded_revisions(db, project_id, &ids).await?;
    let owed: Vec<&String> = drafts
        .iter()
        .filter(|(id, revision)| recorded.get(id) != Some(revision))
        .map(|(id, _)| id)
        .collect();
    sweep(
        db,
        project_id,
        &owed,
        drafts.len() as i64,
        total,
        owed.len() as i64,
        limit,
    )
    .await
}

/// 补过像素版本之后，收着这些照片的方案要按名单各重算一遍。
///
/// 欠账点名（[`project`]）只看"回执记的版本号和现在的版本号对不对得上"：补一个指纹不改方案版本号，
/// 那些回执里"这一张的版本没记过"的说法就会一直挂着，数字看着是跟上了、画面依据仍旧少几张。
/// 所以这里按照片点名，不看欠不欠，算的就是"这几张的事实变了"。
pub async fn drafts_covering(
    db: &Store,
    project_id: &str,
    photo_ids: &[String],
    limit: usize,
) -> Result<Value> {
    drafts_covering_page(db, project_id, photo_ids, limit, TARGET_SCAN_LIMIT).await
}

/// 按照片点名的翻页版，`page` 的含义与 [`project_page`] 一样。
async fn drafts_covering_page(
    db: &Store,
    project_id: &str,
    photo_ids: &[String],
    limit: usize,
    page: i64,
) -> Result<Value> {
    db.get(project_id).await?;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM drafts WHERE project_id=?")
        .bind(project_id)
        .fetch_one(&db.pool)
        .await?;
    if photo_ids.is_empty() {
        return Ok(json!({
            "drafts": total, "draftsScanned": 0, "recheckOwed": 0,
            "recheckedDrafts": 0, "recheckFailed": 0, "recheckRemaining": 0,
        }));
    }
    let ids: Vec<String> = scan(db, project_id, Some(photo_ids), page)
        .await?
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let targets: Vec<&String> = ids.iter().collect();
    let scanned = ids.len() as i64;
    sweep(db, project_id, &targets, scanned, total, scanned, limit).await
}

/// 把名单里的方案挨个走一遍本地评估，按同一套六个数字报出去。
///
/// `owed` 是这一轮该算的份数（欠账点名给"还欠多少"，按名单点名给"这一轮覆盖多少"），
/// `remaining` 一律按"该算的减去算成的"算：没算成的仍旧算没跟上，不拿轮次上限冒充算完。
async fn sweep(
    db: &Store,
    project_id: &str,
    targets: &[&String],
    scanned: i64,
    total: i64,
    owed: i64,
    limit: usize,
) -> Result<Value> {
    let mut rechecked = 0i64;
    let mut failed = 0i64;
    for draft_id in targets.iter().take(limit) {
        match run(db, project_id, draft_id).await {
            Ok(_) => rechecked += 1,
            // 一份方案被别处删掉不该让整轮算失败：已经算好的那些回执都在库里了。
            Err(_) => failed += 1,
        }
    }
    Ok(json!({
        "drafts": total,
        "draftsScanned": scanned,
        "recheckOwed": owed,
        "recheckedDrafts": rechecked,
        "recheckFailed": failed,
        // 没算成的仍旧欠着：这里报的是这一轮之后还差多少，不是这一轮没轮到多少。
        "recheckRemaining": owed - rechecked,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent::types::{AgentRequest, Budget, TaskCommand},
        command_handler::CommandHandler,
        image_pipeline,
    };

    struct Draft {
        db: Store,
        project: String,
        draft: String,
    }

    /// 三张照片 + 一个真实任务 + 一份带条目的方案：重算走的是任务里那条工具，夹具不能只造回执字符串。
    async fn draft() -> (tempfile::TempDir, Draft) {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("recheck.sqlite"))
            .await
            .unwrap();
        let project = db.create("重算口径", "仅本地合成素材").await.unwrap().id;
        for index in 0..3 {
            let file = temp.path().join(format!("photo-{index}.png"));
            image::RgbImage::from_pixel(8, 8, image::Rgb([(index * 40) as u8, 60, 100]))
                .save(&file)
                .unwrap();
            db.insert_photo(
                &project,
                &image_pipeline::prepare(&file, &temp.path().join("cache")).unwrap(),
            )
            .await
            .unwrap();
        }
        let task = CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.clone(),
                command: TaskCommand::StartTask {
                    goal: "挑有叙事的一组".into(),
                    selected_photo_ids: vec![],
                    target_count: 3,
                    budget: Budget::default(),
                    consent_id: None,
                },
            })
            .await
            .unwrap()
            .id;
        let photos: Vec<String> =
            sqlx::query_scalar("SELECT id FROM photos WHERE project_id=? ORDER BY filename")
                .bind(&project)
                .fetch_all(&db.pool)
                .await
                .unwrap();
        let draft = domain::id();
        let now = domain::timestamp();
        sqlx::query("INSERT INTO drafts(id,project_id,task_id,title,base_revision,revision,state,summary,created_at,updated_at) VALUES(?,?,?,?,1,1,'pending',?,?,?)")
            .bind(&draft)
            .bind(&project)
            .bind(&task)
            .bind("重算夹具")
            .bind("按本地事实挑出的一组")
            .bind(&now)
            .bind(&now)
            .execute(&db.pool)
            .await
            .unwrap();
        for (position, photo) in photos.iter().enumerate() {
            sqlx::query("INSERT INTO draft_items(draft_id,photo_id,proposed_action,position,rationale) VALUES(?,?,?,?,?)")
                .bind(&draft)
                .bind(photo)
                .bind("keep")
                .bind(position as i64)
                .bind("开篇")
                .execute(&db.pool)
                .await
                .unwrap();
        }
        (temp, Draft { db, project, draft })
    }

    async fn evidence(fixture: &Draft) -> Vec<Value> {
        crate::agent::projection(&fixture.db, &fixture.project)
            .await
            .unwrap()
            .tasks[0]
            .draft_evidence
            .clone()
    }

    #[tokio::test]
    async fn recheck_moves_the_recorded_revision_to_the_current_one() {
        let (_temp, fixture) = draft().await;
        let revision_before: i64 = sqlx::query_scalar("SELECT revision FROM projects WHERE id=?")
            .bind(&fixture.project)
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap();
        assert!(
            evidence(&fixture).await.is_empty(),
            "没核验过的方案不该凭空有口径"
        );
        let summary = run(&fixture.db, &fixture.project, &fixture.draft)
            .await
            .unwrap();
        assert_eq!(summary["draftRevision"], json!(1));
        assert_eq!(summary["count"], 3, "计数按方案里真实的条目算");
        assert_eq!(summary["observed"], 0);
        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM tool_calls WHERE tool_name='evaluate_draft'")
                .fetch_one(&fixture.db.pool)
                .await
                .unwrap();
        assert_eq!(rows, 1, "重算留下的就是那条本地评估回执");
        let first = evidence(&fixture).await;
        assert_eq!(first.len(), 1);
        assert_eq!(first[0]["draftRevision"], json!(1));
        // 本地改动会抬高方案版本号，旧口径不能继续冒充现在的：这正是界面上"口径没重算"的那一句。
        sqlx::query("UPDATE drafts SET revision=2 WHERE id=?")
            .bind(&fixture.draft)
            .execute(&fixture.db.pool)
            .await
            .unwrap();
        assert_eq!(evidence(&fixture).await[0]["draftRevision"], json!(1));
        let again = run(&fixture.db, &fixture.project, &fixture.draft)
            .await
            .unwrap();
        assert_eq!(again["draftRevision"], json!(2));
        assert_eq!(evidence(&fixture).await[0]["draftRevision"], json!(2));
        // 重算只是把本地事实再比一遍：不新建任务，也不动项目版本号，免得看一趟就被记成改过作品。
        let tasks: i64 = sqlx::query_scalar("SELECT count(*) FROM agent_tasks WHERE project_id=?")
            .bind(&fixture.project)
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap();
        assert_eq!(tasks, 1);
        let revision: i64 = sqlx::query_scalar("SELECT revision FROM projects WHERE id=?")
            .bind(&fixture.project)
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap();
        assert_eq!(revision, revision_before, "看一趟口径不能把项目记成改过");
    }

    #[tokio::test]
    async fn refuses_a_draft_that_is_not_in_the_named_project() {
        let (_temp, fixture) = draft().await;
        let other = fixture
            .db
            .create("另一个项目", "只验证拦截")
            .await
            .unwrap()
            .id;
        let error = run(&fixture.db, &other, &fixture.draft)
            .await
            .expect_err("方案不在点名的项目里时必须拒绝");
        assert_eq!(error.code, "PERMISSION_DENIED");
        let missing = run(&fixture.db, &fixture.project, "没有这个方案")
            .await
            .expect_err("方案查不到时必须拒绝");
        assert_eq!(missing.code, "NOT_FOUND");
        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM tool_calls")
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap();
        assert_eq!(rows, 0, "被拒绝的重算不该留下任何回执");
    }

    /// 同一个任务下再复制出若干份方案：老方案与新方案共用一条任务，投影那边才看得出覆没覆盖到。
    async fn copies(fixture: &Draft, count: usize) {
        for _ in 0..count {
            let copy = domain::id();
            sqlx::query("INSERT INTO drafts(id,project_id,task_id,title,base_revision,revision,state,summary,created_at,updated_at) SELECT ?,project_id,task_id,title,base_revision,revision,state,summary,created_at,updated_at FROM drafts WHERE id=?")
                .bind(&copy)
                .bind(&fixture.draft)
                .execute(&fixture.db.pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO draft_items(draft_id,photo_id,proposed_action,position,rationale) SELECT ?,photo_id,proposed_action,position,rationale FROM draft_items WHERE draft_id=?")
                .bind(&copy)
                .bind(&fixture.draft)
                .execute(&fixture.db.pool)
                .await
                .unwrap();
        }
    }

    async fn receipts(fixture: &Draft) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls WHERE tool_name='evaluate_draft'")
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn one_pass_reaches_drafts_far_older_than_the_recent_twenty() {
        let (_temp, fixture) = draft().await;
        copies(&fixture, 24).await;
        let report = project(&fixture.db, &fixture.project, ROUND_LIMIT)
            .await
            .unwrap();
        assert_eq!(report["drafts"], json!(25));
        assert_eq!(report["draftsScanned"], json!(25));
        assert_eq!(report["recheckOwed"], json!(25));
        assert_eq!(
            report["recheckedDrafts"],
            json!(25),
            "二十份之外的老方案同样要跟上"
        );
        assert_eq!(report["recheckFailed"], json!(0));
        assert_eq!(report["recheckRemaining"], json!(0));
        assert_eq!(receipts(&fixture).await, 25);
        // 算完还要真能在界面上读到：口径是从回执投影出来的，不是这里报个数就算数。
        assert_eq!(evidence(&fixture).await.len(), 25);
        let again = project(&fixture.db, &fixture.project, ROUND_LIMIT)
            .await
            .unwrap();
        assert_eq!(again["recheckOwed"], json!(0), "都跟上了就不该再算一遍");
        assert_eq!(again["recheckedDrafts"], json!(0));
        assert_eq!(receipts(&fixture).await, 25, "没欠账的一轮不留新回执");
    }

    #[tokio::test]
    async fn a_round_reports_what_it_did_not_reach_and_the_next_round_finishes_it() {
        let (_temp, fixture) = draft().await;
        copies(&fixture, 24).await;
        let first = project(&fixture.db, &fixture.project, 5).await.unwrap();
        assert_eq!(first["recheckOwed"], json!(25));
        assert_eq!(first["recheckedDrafts"], json!(5));
        assert_eq!(
            first["recheckRemaining"],
            json!(20),
            "一轮算不完就说还欠多少"
        );
        assert_eq!(receipts(&fixture).await, 5);
        let second = project(&fixture.db, &fixture.project, 5).await.unwrap();
        assert_eq!(
            second["recheckOwed"],
            json!(20),
            "已经跟上的那五份不再算成欠账"
        );
        assert_eq!(second["recheckedDrafts"], json!(5));
        assert_eq!(second["recheckRemaining"], json!(15));
        let rest = project(&fixture.db, &fixture.project, ROUND_LIMIT)
            .await
            .unwrap();
        assert_eq!(rest["recheckOwed"], json!(15));
        assert_eq!(rest["recheckedDrafts"], json!(15));
        assert_eq!(rest["recheckRemaining"], json!(0));
        assert_eq!(receipts(&fixture).await, 25);
    }

    /// 页取 2、页数上限 8：一次取数的老写法只看得到最前 2 份，后面 23 份连"欠着"都算不上名。
    /// 这条同时钉两件事——翻页确实往后翻了，以及翻到页数头里时按上限老实报，不冒充点完了整个项目。
    #[tokio::test]
    async fn owed_drafts_beyond_the_first_page_still_get_named() {
        let (_temp, fixture) = draft().await;
        copies(&fixture, 24).await;
        let window = 2 * SCAN_MAX_PAGES as i64;
        let report = project_page(&fixture.db, &fixture.project, ROUND_LIMIT, 2)
            .await
            .unwrap();
        assert_eq!(report["drafts"], json!(25));
        assert_eq!(
            report["draftsScanned"],
            json!(window),
            "翻到页数上限就照实说只点了这么多，剩下的是没轮到，不是没欠"
        );
        assert_eq!(report["recheckOwed"], json!(window));
        assert_eq!(report["recheckedDrafts"], json!(window));
        assert_eq!(report["recheckRemaining"], json!(0));
        assert_eq!(
            receipts(&fixture).await,
            window,
            "点到的每一份都真留了一条回执"
        );
    }

    /// 按照片点名用的是同一份翻页实现，但那边多了一次连接和去重，所以单独再钉一条。
    #[tokio::test]
    async fn drafts_covering_a_photo_beyond_the_first_page_get_named_too() {
        let (_temp, fixture) = draft().await;
        copies(&fixture, 24).await;
        let photos: Vec<String> =
            sqlx::query_scalar("SELECT id FROM photos WHERE project_id=? ORDER BY filename")
                .bind(&fixture.project)
                .fetch_all(&fixture.db.pool)
                .await
                .unwrap();
        let window = 2 * SCAN_MAX_PAGES as i64;
        let report = drafts_covering_page(&fixture.db, &fixture.project, &photos, ROUND_LIMIT, 2)
            .await
            .unwrap();
        assert_eq!(report["draftsScanned"], json!(window));
        assert_eq!(report["recheckOwed"], json!(window));
        assert_eq!(report["recheckFailed"], json!(0));
        assert_eq!(report["recheckRemaining"], json!(0));
    }

    #[tokio::test]
    async fn a_draft_edited_after_it_was_rechecked_owes_another_pass() {
        let (_temp, fixture) = draft().await;
        let first = project(&fixture.db, &fixture.project, ROUND_LIMIT)
            .await
            .unwrap();
        assert_eq!(first["recheckedDrafts"], json!(1));
        assert_eq!(first["recheckRemaining"], json!(0));
        assert_eq!(evidence(&fixture).await[0]["draftRevision"], json!(1));
        sqlx::query("UPDATE drafts SET revision=2 WHERE id=?")
            .bind(&fixture.draft)
            .execute(&fixture.db.pool)
            .await
            .unwrap();
        let second = project(&fixture.db, &fixture.project, ROUND_LIMIT)
            .await
            .unwrap();
        assert_eq!(
            second["recheckOwed"],
            json!(1),
            "本地改过就重新欠一次，不看上轮算过没有"
        );
        assert_eq!(second["recheckedDrafts"], json!(1));
        assert_eq!(evidence(&fixture).await[0]["draftRevision"], json!(2));
    }

    /// 顶破原先那道长度门的就是"相邻相似"这份不计条数的清单，夹具照同一种形状垫体积。
    async fn oversized(fixture: &Draft, revision: i64) {
        let task: String = sqlx::query_scalar("SELECT task_id FROM drafts WHERE id=?")
            .bind(&fixture.draft)
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap();
        let mut payload = json!({
            "draft_id": fixture.draft, "mode": "structured_local_evaluation",
            "draft_revision": revision, "count": 3,
            "metrics": {"observed":0,"unobserved":3,"evidence_contracts":{
                "current_page_tier":0,"current_detail_tier":0,"other_prompt_version":0,"other_tool":0},
                "cross_contract_adjacent_pairs":0},
            "item_facts": [], "omitted_visual_evidence": [],
        });
        payload["similar_adjacent"] = json!([{"pad": "x".repeat(600 * 1024)}]);
        let result = ToolResult::success(&domain::id(), payload);
        sqlx::query("INSERT INTO tool_calls VALUES(?,?,?,?,?)")
            .bind(&task)
            .bind(&result.call_id)
            .bind("evaluate_draft")
            .bind("fixture")
            .bind(encode(&result).unwrap())
            .execute(&fixture.db.pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_receipt_too_large_to_read_back_does_not_owe_a_recheck_forever() {
        let (_temp, fixture) = draft().await;
        let first = project(&fixture.db, &fixture.project, ROUND_LIMIT)
            .await
            .unwrap();
        assert_eq!(first["recheckedDrafts"], json!(1));
        sqlx::query("UPDATE drafts SET revision=2 WHERE id=?")
            .bind(&fixture.draft)
            .execute(&fixture.db.pool)
            .await
            .unwrap();
        // 这份回执确实对着第 2 版算过，只是大到界面读不回来。
        oversized(&fixture, 2).await;
        let second = project(&fixture.db, &fixture.project, ROUND_LIMIT)
            .await
            .unwrap();
        assert_eq!(
            second["recheckOwed"],
            json!(0),
            "大到读不回来不等于没算过：每轮重算一遍也不会让它变小"
        );
        assert_eq!(second["recheckedDrafts"], json!(0));
        assert_eq!(
            receipts(&fixture).await,
            2,
            "不该为一份读不回来的回执再留第三条：{second:?}"
        );
        // 界面这边不能借它冒充"跟上第 2 版"：投出来的仍旧是读得回来的那一条，对着哪一版照实说。
        assert_eq!(
            evidence(&fixture).await[0]["draftRevision"],
            json!(1),
            "读不回来那条不能替它作保，界面上那句\"口径没重算\"还得说得出"
        );
    }

    #[tokio::test]
    async fn the_pass_only_names_drafts_in_the_project_it_was_given() {
        let (_temp, fixture) = draft().await;
        let other = fixture
            .db
            .create("另一个项目", "只验证点名范围")
            .await
            .unwrap()
            .id;
        let empty = project(&fixture.db, &other, ROUND_LIMIT).await.unwrap();
        assert_eq!(empty["drafts"], json!(0));
        assert_eq!(empty["recheckOwed"], json!(0));
        assert_eq!(
            receipts(&fixture).await,
            0,
            "别的项目的方案不该被顺手算一遍"
        );
        assert!(
            project(&fixture.db, "不在这个库里的项目", ROUND_LIMIT)
                .await
                .is_err()
        );
        assert_eq!(receipts(&fixture).await, 0);
    }

    #[tokio::test]
    async fn the_newest_receipt_still_shows_after_a_task_piles_up_more_than_fifty() {
        let (_temp, fixture) = draft().await;
        // 每修一轮预览、每点一次重算都往同一个任务追加一条回执：攒过五十条之后，
        // 界面要看的仍旧是最新那条，不能停在最早的那一批上。
        for _ in 0..50 {
            run(&fixture.db, &fixture.project, &fixture.draft)
                .await
                .unwrap();
        }
        assert_eq!(evidence(&fixture).await[0]["draftRevision"], json!(1));
        sqlx::query("UPDATE drafts SET revision=2 WHERE id=?")
            .bind(&fixture.draft)
            .execute(&fixture.db.pool)
            .await
            .unwrap();
        run(&fixture.db, &fixture.project, &fixture.draft)
            .await
            .unwrap();
        assert_eq!(receipts(&fixture).await, 51);
        let shown = evidence(&fixture).await;
        assert_eq!(shown.len(), 1);
        assert_eq!(
            shown[0]["draftRevision"],
            json!(2),
            "第五十一条回执被挡在窗口外的话，点了重算界面也一动不动"
        );
    }
}
