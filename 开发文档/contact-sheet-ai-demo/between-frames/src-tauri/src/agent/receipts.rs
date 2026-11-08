//! 画面依据的核验单怎么读，只有这一处。
//!
//! 界面要说"这份方案的画面事实是按哪一档问法挑的"，重算要点名"哪几份还欠着一次重算"，
//! 两边读的都是 `tool_calls` 里那批 `evaluate_draft` 回执。规矩各写一遍就会出现两张嘴：
//! 一边说看得见，另一边说还欠着。
//!
//! 原来两边都拿"整条不超过 64KB"当门。那份核验单里"相邻相似"和"完全重复"两份清单是不计条数
//! 往里塞的，几百张的画布一眼就能顶破；顶破了界面就对那份方案什么都不说，重算那边还每轮把它
//! 算成欠着、白重算一遍（重算并不会让核验单变小）。所以这里把门改成两条，各管各的：
//!
//! - 判欠账只要"这份方案最新的口径记着哪个版本号"，那几个字在库里就能取出来，
//!   一条整回执也不用搬回内存，也就不看体积脸色；
//! - 界面要说计数和逐张名单，得整条读回来，所以按体积兜底：单条给一个比 64KB 宽得多的上限，
//!   一个任务再给一个总预算，装不下的整条不读——读不到就什么都不说，绝不拿半截数据编一段口径。
//!
//! 读回来的也只有"每份方案最新的那一条"：投影本来就只留每份方案最新的一份，
//! 以前却把 50 条整回执全搬回内存再扔掉四十九份。

use super::types::ToolResult;
use crate::{command_audit::encode, db::Store, domain::Result};
use serde_json::Value;
use sqlx::{Row, Sqlite, Transaction};
use std::collections::{BTreeMap, BTreeSet};

/// 单条核验单最多读多大：比原先那道 64KB 宽得多，几百张的方案才看得见自己的口径；
/// 再宽就不值得为一次界面加载搬回内存。
const MAX_RECEIPT_BYTES: i64 = 512 * 1024;
/// 一个任务一次读回内存的总量。原先这道门相当于"50 条 × 64KB"，最坏 3.2MB；
/// 现在单条放宽到 512KB、总量却收在 2MB——大方案看得见，投影也比改之前更省。
const TASK_BYTE_BUDGET: i64 = 2 * 1024 * 1024;
/// 一个任务往回扫多少条核验单。原先这道 50 行同时当两件事用：既挡住解析代价，又"顺便"挡住了
/// 份数（界面通道只收 50 份）。同一份方案反复重算几轮就把行号堆满，更早那份方案整条口径都被挤掉——
/// 现在份数由 `MAX_ENTRIES` 明确管着，这里只留代价上限，与就地取数那条同一个量级。
const TASK_SCAN_ROWS: i64 = 400;
/// 就地补计数时最多扫多少条：整条读回来的那些已经占掉了界面通道的位置，这里只给
/// 剩下的那几份方案补几个数字，一次投影为此付的 JSON 解析代价必须有个顶。
const SUMMARY_SCAN_ROWS: i64 = 400;
/// 界面通道一次最多投多少份方案的口径（前端对这条投影有同样的条数校验）。
pub(crate) const MAX_ENTRIES: usize = 50;

/// 从一条回执里就地取一个标量，不把整条搬回内存。
/// 外面套一层 `json_valid` 是因为库里存的是文本：早年的行、写一半的行都可能压根不是 JSON，
/// 而 `json_extract` 碰到这种行不是返回空，是直接报错——一条坏行不该把整个项目问倒。
fn peek(path: &str) -> String {
    format!("CASE WHEN json_valid(c.result_json) THEN json_extract(c.result_json,'{path}') END")
}

/// 同上，但要的是"这个字段写得像什么"，用来认出版本号被写成字符串的那些老行。
fn peek_kind(path: &str) -> String {
    format!("CASE WHEN json_valid(c.result_json) THEN json_type(c.result_json,'{path}') END")
}

/// 只有这两样形状的条目才是界面要说的那份口径，写在 SQL 里先把别的结果挡在外面。
fn evaluation_filter() -> String {
    format!(
        "{}=1 AND {}='structured_local_evaluation'",
        peek("$.ok"),
        peek("$.result.mode")
    )
}

/// 这份核验单是不是界面说的那份口径：与上面那段 SQL 的筛法一致，只多了"读得懂"这一层。
pub(crate) fn evaluation(raw: &str) -> Option<Value> {
    let Ok(ToolResult { ok, result, .. }) = serde_json::from_str::<ToolResult>(raw) else {
        return None;
    };
    ok.then_some(result?)
        .filter(|value| value["mode"] == "structured_local_evaluation")
}

/// 挑出要整条读回来的那几条：按行号从新往旧看，一份方案只认它最新的一条，
/// 单条太宽就不认这一条、往下看它旧的那份（跟原先那道 64KB 门同一个行为），总预算用完就不再收。
/// 返回行号升序，也就是时间先后。
async fn rowids_to_read(tx: &mut Transaction<'_, Sqlite>, task_id: &str) -> Result<Vec<i64>> {
    let sql = format!(
        "SELECT c.rowid AS rid, \
                length(CAST(c.result_json AS BLOB)) AS size, \
                {} AS draft \
         FROM tool_calls c \
         WHERE c.task_id=? AND c.tool_name='evaluate_draft' AND {} \
         ORDER BY c.rowid DESC LIMIT ?",
        peek("$.result.draft_id"),
        evaluation_filter()
    );
    let candidates = sqlx::query(&sql)
        .bind(task_id)
        .bind(TASK_SCAN_ROWS)
        .fetch_all(&mut **tx)
        .await?;
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut picked: Vec<i64> = vec![];
    let mut left = TASK_BYTE_BUDGET;
    for row in candidates {
        let size: i64 = row.get("size");
        // 太宽的这一条不算数，但也不许这份方案就此静音：旧一条还读得回来就用旧一条。
        if size > MAX_RECEIPT_BYTES || size > left {
            continue;
        }
        let Some(draft) = row
            .get::<Option<String>, _>("draft")
            .filter(|id| !id.is_empty())
        else {
            continue;
        };
        if !taken.insert(draft) {
            continue;
        }
        left -= size;
        picked.push(row.get::<i64, _>("rid"));
        // 份数上限在这儿收口，不靠扫描行数顶出来：前端对这条通道有同样的条数校验，
        // 多投一条会让整份投影被判成坏数据，整个面板一起看不见。
        if picked.len() >= MAX_ENTRIES {
            break;
        }
    }
    picked.reverse();
    Ok(picked)
}

/// 一个任务留下的核验单里，每份方案现在该说的那一条，按时间先后返回，值已是回执的结果正文。
/// 读不回来的那些（太大、超预算）在投影里就是没有口径，界面对它什么都不说。
pub(crate) async fn for_task(
    tx: &mut Transaction<'_, Sqlite>,
    task_id: &str,
) -> Result<Vec<Value>> {
    let picked = rowids_to_read(tx, task_id).await?;
    if picked.is_empty() {
        return Ok(vec![]);
    }
    let raw: Vec<String> = sqlx::query_scalar(
        "SELECT result_json FROM tool_calls \
         WHERE rowid IN (SELECT value FROM json_each(?)) ORDER BY rowid",
    )
    .bind(encode(&picked)?)
    .fetch_all(&mut **tx)
    .await?;
    // 一条坏掉的回执不能拖垮整个投影：读不懂就跳过。
    Ok(raw.iter().filter_map(|value| evaluation(value)).collect())
}

/// 每份方案"现在的口径记着哪一版"：只要这几个字，所以在库里就地取，整条核验单一次也不搬回来。
///
/// 名单之外的一条也不读：欠账判定原来跟着"全项目最新 5000 条"这道窗口，而点名能翻到的份数早就超过它，
/// 窗口外的方案哪怕刚刚算完也被报成还欠着——每一轮都把额度花在重算上，"还欠多少"永远不降。
/// 现在只看名单里这几份各自最新那一条，名单里没有的本来就是没算过。
/// 这仍然意味着欠账点名不看体积脸色——体积大的那份重算一百遍也不会变小，一直算成欠着只是每轮白跑。
pub(crate) async fn recorded_revisions(
    db: &Store,
    project_id: &str,
    draft_ids: &[String],
) -> Result<BTreeMap<String, i64>> {
    let sql = format!(
        "SELECT {} AS draft, \
                {} AS revision, \
                {} AS kind \
         FROM tool_calls c JOIN agent_tasks t ON t.id=c.task_id \
         WHERE t.project_id=? AND c.tool_name='evaluate_draft' AND {} \
         AND {} IN (SELECT value FROM json_each(?)) \
         ORDER BY c.rowid DESC",
        peek("$.result.draft_id"),
        peek("$.result.draft_revision"),
        peek_kind("$.result.draft_revision"),
        evaluation_filter(),
        peek("$.result.draft_id"),
    );
    // 名单走 json_each 而不是占位符：一份方案一个问号，几万个方案会把参数上限撑爆。
    let rows = sqlx::query(&sql)
        .bind(project_id)
        .bind(encode(&draft_ids)?)
        .fetch_all(&db.pool)
        .await?;
    let mut recorded: BTreeMap<String, i64> = BTreeMap::new();
    for row in rows {
        // 从新往旧扫，先落下的那条就是这份方案现在的口径；版本号写得不像整数就当没记过。
        let Some(draft) = row
            .get::<Option<String>, _>("draft")
            .filter(|id| !id.is_empty())
        else {
            continue;
        };
        let Some(revision) = row
            .get::<Option<i64>, _>("revision")
            .filter(|_| row.get::<Option<String>, _>("kind").as_deref() == Some("integer"))
            .filter(|n| *n > 0)
        else {
            continue;
        };
        recorded.entry(draft).or_insert(revision);
    }
    Ok(recorded)
}

/// `omitted` 这一格要的只是"排除了几张"这个数，不用把名单搬回来。
/// 先问一句它是不是数组：SQLite 的 JSON 函数在坏数据上是报错，不是返回空。
fn omitted_len() -> String {
    let path = "$.result.omitted_visual_evidence";
    format!(
        "CASE WHEN json_valid(c.result_json) AND json_type(c.result_json,'{path}')='array' \
         THEN json_array_length(c.result_json,'{path}') END"
    )
}

/// 算过但整条读不回来的那些方案：几个计数在库里就地取出来，整条核验单一次也不搬回内存。
///
/// 有了这几个字，界面才能把"这份算过，只是逐张档位没读回来"和"这份从来没算过"分开说；
/// 少了它，两种就只能混成同一句含糊话——那句含糊话本身不算撒谎，但把知道的说少了。
/// 一行取回一个 JSON 对象而不是十几个整数列：老库里同一个字段可能被写成字符串或浮点数，
/// 按列取值会直接把整条查询问倒，取回对象再自己判形状，最坏只是那一份不投。
pub(crate) async fn counts_for_unreadable(
    tx: &mut Transaction<'_, Sqlite>,
    task_id: &str,
    readable: &[String],
) -> Result<Vec<Value>> {
    let sql = format!(
        "SELECT json_object(\
            'draft',{draft},'revision',{rev},'revision_kind',{rev_kind},\
            'count',{count},'observed',{observed},'unobserved',{unobserved},\
            'page',{page},'detail',{detail},'other_version',{other_version},\
            'other_tool',{other_tool},'pairs',{pairs},'omitted',{omitted}) AS row_json \
         FROM tool_calls c \
         WHERE c.task_id=? AND c.tool_name='evaluate_draft' AND {filter} \
         AND {draft} NOT IN (SELECT value FROM json_each(?)) \
         ORDER BY c.rowid DESC LIMIT ?",
        draft = peek("$.result.draft_id"),
        rev = peek("$.result.draft_revision"),
        rev_kind = peek_kind("$.result.draft_revision"),
        count = peek("$.result.count"),
        observed = peek("$.result.metrics.observed"),
        unobserved = peek("$.result.metrics.unobserved"),
        page = peek("$.result.metrics.evidence_contracts.current_page_tier"),
        detail = peek("$.result.metrics.evidence_contracts.current_detail_tier"),
        other_version = peek("$.result.metrics.evidence_contracts.other_prompt_version"),
        other_tool = peek("$.result.metrics.evidence_contracts.other_tool"),
        pairs = peek("$.result.metrics.cross_contract_adjacent_pairs"),
        omitted = omitted_len(),
        filter = evaluation_filter(),
    );
    let rows = sqlx::query(&sql)
        .bind(task_id)
        .bind(encode(&readable)?)
        .bind(SUMMARY_SCAN_ROWS)
        .fetch_all(&mut **tx)
        .await?;
    // 从新往旧扫，一份方案先落下的那条就是它现在的口径；上面那道 NOT IN 已经把读回来的排掉了。
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut summaries = vec![];
    for row in rows {
        let raw: String = row.get("row_json");
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let Some(draft) = value["draft"].as_str().filter(|id| !id.is_empty()) else {
            continue;
        };
        if seen.insert(draft.to_string()) {
            summaries.push(value);
        }
    }
    Ok(summaries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent::types::{AgentRequest, Budget, TaskCommand},
        command_handler::CommandHandler,
        domain,
    };
    use serde_json::json;

    async fn fixture() -> (tempfile::TempDir, Store, String, String) {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("receipts.sqlite"))
            .await
            .unwrap();
        let project = db.create("核验单", "读取").await.unwrap().id;
        // 建任务要求项目里有照片（额度 1–500），所以夹具先放两张，和真实使用同一条路径。
        for n in 0..2 {
            let file = temp.path().join(format!("{n}.png"));
            image::RgbImage::from_pixel(8, 8, image::Rgb([(n * 60) as u8, 40, 90]))
                .save(&file)
                .unwrap();
            let image = crate::image_pipeline::prepare(&file, &temp.path().join("cache")).unwrap();
            db.insert_photo(&project, &image).await.unwrap();
        }
        let task = CommandHandler::new(&db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.clone(),
                command: TaskCommand::StartTask {
                    goal: "读核验单".into(),
                    selected_photo_ids: vec![],
                    target_count: 2,
                    budget: Budget::default(),
                    consent_id: None,
                },
            })
            .await
            .unwrap()
            .id;
        (temp, db, project, task)
    }

    /// 按真实 `ToolResult` 序列化一份核验单：键名怎么拼由它自己定，测试不能另编一套。
    /// 体积垫在界面不读的那份清单上，跟真实顶破那道门时的形状一样。
    fn receipt(draft: &str, mode: &str, revision: i64, bytes: usize, ok: bool) -> String {
        let payload = json!({
            "draft_id": draft, "mode": mode, "draft_revision": revision,
            "count": 2, "metrics": {"observed":2,"unobserved":0,"evidence_contracts":{
                "current_page_tier":2,"current_detail_tier":0,"other_prompt_version":0,"other_tool":0},
                "cross_contract_adjacent_pairs":0},
            "item_facts": [], "omitted_visual_evidence": [],
            "similar_adjacent": [{"pad": "x".repeat(bytes)}],
        });
        serde_json::to_string(&ToolResult {
            call_id: domain::id(),
            ok,
            result: Some(payload),
            error: None,
            observation_refs: vec![],
            duration_ms: 0,
            billing_estimate: 0,
        })
        .unwrap()
    }

    fn padded(draft: &str, revision: i64, bytes: usize) -> String {
        receipt(draft, "structured_local_evaluation", revision, bytes, true)
    }

    async fn write_as(db: &Store, task: &str, tool: &str, raw: &str) {
        sqlx::query(
            "INSERT INTO tool_calls(task_id,call_id,tool_name,fingerprint,result_json) VALUES(?,?,?,?,?)",
        )
        .bind(task)
        .bind(domain::id())
        .bind(tool)
        .bind("fixture")
        .bind(raw)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    async fn write(db: &Store, task: &str, raw: &str) {
        write_as(db, task, "evaluate_draft", raw).await;
    }

    fn draft_of(result: &Value) -> String {
        result["draft_id"].as_str().unwrap_or_default().to_string()
    }

    /// 走投影用的同一条路径：从池子里开一个事务，再把核验单读回来。
    async fn read(db: &Store, task: &str) -> Vec<Value> {
        let mut tx = db.pool.begin().await.unwrap();
        super::for_task(&mut tx, task).await.unwrap()
    }

    #[tokio::test]
    async fn reads_the_newest_readable_receipt_per_draft_in_time_order() {
        let (_temp, db, _project, task) = fixture().await;
        write(&db, &task, &padded("draft-old", 1, 0)).await;
        // 这份方案最新的一条大到不打算读：它前面那条小的还能说，界面就用小的那条，
        // 而不是因为最大的一条把整份方案的口径都堵住。
        write(&db, &task, &padded("draft-big", 9, 1024 * 1024)).await;
        write(&db, &task, &padded("draft-old", 2, 0)).await;
        write(&db, &task, &padded("draft-big", 3, 0)).await;
        let rows = read(&db, &task).await;
        assert_eq!(
            rows.iter().map(draft_of).collect::<Vec<_>>(),
            vec!["draft-old", "draft-big"],
            "每份方案只留它读得回来的最新一条，按时间先后给出：{rows:?}"
        );
        assert_eq!(rows[0]["draft_revision"], 2, "同一份方案留下后写的那一份");
        assert_eq!(
            rows[1]["draft_revision"], 3,
            "超体积的那一条是被跳过的读回，不是整份方案静音"
        );
    }

    #[tokio::test]
    async fn stops_reading_once_the_task_byte_budget_is_gone() {
        let (_temp, db, _project, task) = fixture().await;
        // 每条约 400KB：都在单条上限之内，但一个任务的总量只够装最新的那几条。
        for n in 0..8 {
            write(&db, &task, &padded(&format!("draft-{n}"), 1, 400 * 1024)).await;
        }
        let drafts = read(&db, &task)
            .await
            .iter()
            .map(draft_of)
            .collect::<Vec<_>>();
        assert_eq!(
            drafts,
            vec!["draft-3", "draft-4", "draft-5", "draft-6", "draft-7"],
            "总量兜住的必须是更早的那几条，最新算出来的不能被关闭口"
        );
    }

    #[tokio::test]
    async fn revisions_are_named_from_the_library_no_how_big_the_receipt_is() {
        let (_temp, db, project, task) = fixture().await;
        write(&db, &task, &padded("draft-tiny", 3, 0)).await;
        write(&db, &task, &padded("draft-huge", 7, 3 * 1024 * 1024)).await;
        let mut json_no_rev = serde_json::from_str::<Value>(&padded("draft-gone", 1, 0)).unwrap();
        json_no_rev["result"]
            .as_object_mut()
            .unwrap()
            .remove("draft_revision");
        write(&db, &task, &json_no_rev.to_string()).await;
        let named = ["draft-tiny", "draft-huge", "draft-gone"]
            .iter()
            .map(|id| (*id).to_owned())
            .collect::<Vec<String>>();
        let recorded = recorded_revisions(&db, &project, &named).await.unwrap();
        assert_eq!(recorded.get("draft-tiny"), Some(&3));
        assert_eq!(
            recorded.get("draft-huge"),
            Some(&7),
            "整条读不回来的体积也该报得出对着哪一版：欠账点名不能看体积脸色"
        );
        assert_eq!(
            recorded.get("draft-gone"),
            None,
            "没记版本号的老回执不能猜一个"
        );
        assert!(
            read(&db, &task)
                .await
                .iter()
                .all(|row| draft_of(row) != "draft-huge"),
            "整条读不回来的那条不能混进这份投影：它由 `counts_for_unreadable` 就地取的计数兜住"
        );
    }

    #[tokio::test]
    async fn a_draft_older_than_the_fiftieth_receipt_still_gets_its_full_basis() {
        let (_temp, db, _project, task) = fixture().await;
        write(&db, &task, &padded("draft-old", 1, 0)).await;
        // 同一份方案反复重算十几轮：行号全被它占掉。窗口按行数截的时候，更早那份方案的整条口径
        // 连读回来的机会都没有，界面只能靠就地取的计数兜底。
        for _ in 0..120 {
            write(&db, &task, &padded("draft-new", 2, 0)).await;
        }
        let drafts = read(&db, &task)
            .await
            .iter()
            .map(draft_of)
            .collect::<Vec<_>>();
        assert!(
            drafts.contains(&"draft-old".to_string()),
            "更早那份方案的口径不该被同任务的重复核验挤掉：{drafts:?}"
        );
        assert_eq!(
            drafts,
            vec!["draft-old", "draft-new"],
            "投影按时间先后给，一份方案只说它现在该说的那一条"
        );
    }

    #[tokio::test]
    async fn the_projection_never_ships_more_drafts_than_the_ui_channel_accepts() {
        let (_temp, db, _project, task) = fixture().await;
        for n in 0..60 {
            write(&db, &task, &padded(&format!("draft-{n}"), 1, 0)).await;
        }
        assert_eq!(
            read(&db, &task).await.len(),
            MAX_ENTRIES,
            "份数上限必须由代码说清楚：前端对这条通道有同样的条数校验，多投一条整份投影都会被判成坏数据"
        );
    }

    #[tokio::test]
    async fn ignores_receipts_that_are_not_the_picture_evidence_one() {
        let (_temp, db, project, task) = fixture().await;
        write(
            &db,
            &task,
            &receipt("draft-fail", "structured_local_evaluation", 1, 0, false),
        )
        .await;
        write(
            &db,
            &task,
            &receipt("draft-other", "vision_observed", 1, 0, true),
        )
        .await;
        write(&db, &task, &padded("draft-1", 1, 0)).await;
        // 形状对但记在别的工具名下：那是工具自己的结果，不是画面口径。
        write_as(
            &db,
            &task,
            "list_photos",
            &receipt("draft-2", "structured_local_evaluation", 1, 0, true),
        )
        .await;
        // 库里存的是文本，写了一半的坏行只能被跳过，不能把整条查询问倒。
        write(&db, &task, "这不是 JSON").await;
        let rows = read(&db, &task).await;
        assert_eq!(
            rows.iter().map(draft_of).collect::<Vec<_>>(),
            vec!["draft-1"],
            "失败的回执、别的问法、别种工具的结果都不该被读回来当口径"
        );
        assert_eq!(
            recorded_revisions(&db, &project, &["draft-1".to_owned()])
                .await
                .unwrap()
                .get("draft-1")
                .copied(),
            Some(1),
            "同一批坏行也不能让欠账点名报错"
        );
    }

    /// 名单点到的那份方案，回执哪怕被别人的几千条挤到最老，也还得认它"已经跟上"。
    /// 这条钉的是"欠账判定不再有一道全项目上限"：原来最新 5000 条之外的方案刚算完也被报成还欠着，
    /// 每一轮都把额度花在重算上，"还欠多少"永远不降。
    #[tokio::test]
    async fn a_named_drafts_receipt_is_found_behind_thousands_of_other_drafts() {
        let (_temp, db, project, task) = fixture().await;
        write(&db, &task, &padded("draft-old", 4, 0)).await;
        for n in 0..5_001 {
            write(&db, &task, &padded(&format!("draft-noise-{n}"), 1, 0)).await;
        }
        assert_eq!(
            recorded_revisions(&db, &project, &["draft-old".to_owned()])
                .await
                .unwrap()
                .get("draft-old"),
            Some(&4),
            "刚算完的那份不能被别人的回执数量挤成永远欠着"
        );
    }

    /// 手动基准：`cargo test --lib --features desktop projection_refresh_cost -- --ignored --nocapture`。
    /// 只打印耗时，不做时间断言——机器负载会让它随机红。用来给 `TASK_SCAN_ROWS` 定数：
    /// 一个 24 个任务、约 1450 条核验单的项目，整次投影刷新、单任务整条读回、就地补计数各花多少。
    #[tokio::test]
    #[ignore = "手动基准：打印大项目投影刷新耗时，不做时间断言"]
    async fn projection_refresh_cost_on_a_recomputed_project() {
        let (_temp, db, project, first) = fixture().await;
        let handler = CommandHandler::new(&db);
        let mut tasks = vec![];
        // 一个项目同时只许一个任务在跑：每建一个就取消掉腾出名额。取消的任务照样进投影，
        // 所以量到的还是"24 个任务一起刷新"的真实代价。
        for n in 0..24 {
            let id = if n == 0 {
                first.clone()
            } else {
                handler
                    .agent_request(AgentRequest {
                        request_id: domain::id(),
                        project_id: project.clone(),
                        command: TaskCommand::StartTask {
                            goal: "读核验单".into(),
                            selected_photo_ids: vec![],
                            target_count: 2,
                            budget: Budget::default(),
                            consent_id: None,
                        },
                    })
                    .await
                    .unwrap()
                    .id
            };
            handler
                .agent_request(AgentRequest {
                    request_id: domain::id(),
                    project_id: project.clone(),
                    command: TaskCommand::CancelTask {
                        task_id: id.clone(),
                    },
                })
                .await
                .unwrap();
            tasks.push(id);
        }
        // 第一个任务反复重算 80 轮，把 400 行那道窗口顶满；其余任务各 7 轮，是常见的形状。
        for round in 0..80i64 {
            for draft in 0..6 {
                write(
                    &db,
                    &tasks[0],
                    &padded(&format!("draft-{draft}"), round + 1, 0),
                )
                .await;
            }
        }
        for task in &tasks[1..] {
            for round in 0..7i64 {
                for draft in 0..6 {
                    write(&db, task, &padded(&format!("draft-{draft}"), round + 1, 0)).await;
                }
            }
        }
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        println!(
            "tasks={} tool_call_rows={rows} TASK_SCAN_ROWS={TASK_SCAN_ROWS} SUMMARY_SCAN_ROWS={SUMMARY_SCAN_ROWS} MAX_ENTRIES={MAX_ENTRIES}",
            tasks.len()
        );
        for run in 0..5 {
            let started = std::time::Instant::now();
            let view = crate::agent::store::projection(&db, &project)
                .await
                .unwrap();
            let ms = started.elapsed().as_millis();
            let entries: usize = view.tasks.iter().map(|t| t.draft_evidence.len()).sum();
            println!(
                "run={run} projection_ms={ms} tasks={} evidence_entries={entries}",
                view.tasks.len()
            );
            assert_eq!(view.tasks.len(), 24);
        }
        for run in 0..5 {
            let started = std::time::Instant::now();
            let rows = read(&db, &tasks[0]).await;
            let whole_ms = started.elapsed().as_millis();
            let started = std::time::Instant::now();
            let mut tx = db.pool.begin().await.unwrap();
            counts_for_unreadable(&mut tx, &tasks[0], &[])
                .await
                .unwrap();
            let summary_ms = started.elapsed().as_millis();
            println!(
                "run={run} window_bound_task_read_ms={whole_ms} entries={} summary_ms={summary_ms}",
                rows.len()
            );
            assert!(rows.len() <= MAX_ENTRIES);
        }
        // 第二段：扫描代价是按字节算的，小回执量出来的数不能替大回执背书。
        // 再建一个任务，往里塞 402 条"每条 64KB"的核验单——正好顶满 400 行那道窗口，
        // 也就是当年一眼就顶破 64KB 整条门的那种体积。
        let big = handler
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.clone(),
                command: TaskCommand::StartTask {
                    goal: "读大核验单".into(),
                    selected_photo_ids: vec![],
                    target_count: 2,
                    budget: Budget::default(),
                    consent_id: None,
                },
            })
            .await
            .unwrap()
            .id;
        handler
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.clone(),
                command: TaskCommand::CancelTask {
                    task_id: big.clone(),
                },
            })
            .await
            .unwrap();
        println!(
            "big_receipt_bytes={}",
            padded("draft-0", 1, 64 * 1024).len()
        );
        for round in 0..67i64 {
            for draft in 0..6 {
                write(
                    &db,
                    &big,
                    &padded(&format!("draft-{draft}"), round + 1, 64 * 1024),
                )
                .await;
            }
        }
        for run in 0..3 {
            let started = std::time::Instant::now();
            let rows = read(&db, &big).await;
            let read_ms = started.elapsed().as_millis();
            let started = std::time::Instant::now();
            let summary_ms = {
                let mut tx = db.pool.begin().await.unwrap();
                counts_for_unreadable(&mut tx, &big, &[]).await.unwrap();
                started.elapsed().as_millis()
            };
            let started = std::time::Instant::now();
            let view = crate::agent::store::projection(&db, &project)
                .await
                .unwrap();
            let projection_ms = started.elapsed().as_millis();
            println!(
                "big run={run} read_ms={read_ms} entries={} summary_ms={summary_ms} projection_ms={projection_ms} tasks={}",
                rows.len(),
                view.tasks.len()
            );
            assert_eq!(view.tasks.len(), 25);
            assert!(rows.len() <= MAX_ENTRIES);
        }
    }
}
