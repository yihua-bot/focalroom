//! 整项目的派生预览点名与一键修好。
//!
//! 已有的修复只能按页点名，用户看得见这一页坏了几张，看不见整个项目还欠着多少。这里补的是
//! "一次数清 + 一次修完"这两步，判断本身仍旧交给 `image_pipeline` 里同一条体检，不另写一套。
//!
//! 清点刻意不比对原图内容指纹：那道门要把原图整张读一遍，按页点名可以，按项目点名会变成
//! 几分钟的磁盘扫描。代价是外部改过原图的那几张在清点里算"能用"，真要重算时仍会被逐张拦下——
//! 两边各有分工，谁也不冒充谁。

use crate::{
    agent::recheck,
    db::Store,
    domain::{AppError, Result},
    image_pipeline,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

/// 一次点全不等于一次修完：每张重算都要解一次原图，所以一轮仍按原有的 200 张上限走。
const REPAIR_LIMIT: usize = 200;
/// 清点的扫描规模也设上限，超出就在结果里说还有多少没数到，不假装数完了。
const SCAN_LIMIT: i64 = 5000;

async fn total_photos(db: &Store, project_id: &str) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM photos WHERE project_id=?")
            .bind(project_id)
            .fetch_one(&db.pool)
            .await?,
    )
}

/// 体检任务只能由库里的记录拼出来：调用方点名的只有项目，路径一律界面不能指定。
async fn project_jobs(db: &Store, project_id: &str) -> Result<Vec<image_pipeline::PreviewJob>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id,filename,original_path,content_sha256,thumbnail_path,preview_path \
         FROM photos WHERE project_id=? ORDER BY filename,id LIMIT ?",
    )
    .bind(project_id)
    .bind(SCAN_LIMIT)
    .fetch_all(&db.pool)
    .await?;
    let mut jobs = Vec::with_capacity(rows.len());
    for (photo_id, filename, original_path, content_sha256, thumbnail_path, preview_path) in rows {
        jobs.push(image_pipeline::PreviewJob {
            photo_id,
            filename,
            original_path,
            content_sha256,
            thumbnail_path,
            preview_path,
        });
    }
    Ok(jobs)
}

/// 一份点名单：能用的、该重算的、各原因码挡住的分开放，界面无需自己再归类。
fn ledger(rows: &[image_pipeline::PreviewRepair], total: i64) -> Value {
    let count = |name: &str| rows.iter().filter(|row| row.outcome == name).count() as i64;
    let mut reasons: BTreeMap<String, i64> = BTreeMap::new();
    for row in rows {
        if row.outcome != "healthy" && row.outcome != "needs_repair" {
            *reasons.entry(row.outcome.clone()).or_default() += 1;
        }
    }
    let actionable: Vec<&image_pipeline::PreviewRepair> =
        rows.iter().filter(|row| row.outcome != "healthy").collect();
    let shown = actionable.len().min(50);
    json!({
        "total": total,
        "scanned": rows.len() as i64,
        "healthy": count("healthy"),
        "needsRepair": count("needs_repair"),
        "blocked": reasons.values().sum::<i64>(),
        "reasons": reasons,
        "items": serde_json::to_value(&actionable[..shown]).unwrap_or_default(),
    })
}

/// 只读点名：一个字节都不改。
pub async fn audit(db: &Store, cache: &Path, project_id: &str) -> Result<Value> {
    db.get(project_id).await?;
    let total = total_photos(db, project_id).await?;
    let jobs = project_jobs(db, project_id).await?;
    let cache = cache.to_path_buf();
    let rows =
        tokio::task::spawn_blocking(move || image_pipeline::audit_preview_jobs(&cache, &jobs))
            .await
            .map_err(|error| AppError::new("BACKGROUND", error.to_string()))?;
    Ok(ledger(&rows, total))
}

/// 一键修完这个项目里读不出来的派生文件，并把各份方案的口径顺手跟上。
///
/// 修好之后必须重算口径：那些照片的画面记录之前被排除在外，不重算的话界面上的数字还停在
/// "坏着"的那一晚。重算走的是同一条本地通道，不经过模型、不上传照片；一轮算不完就把还欠
/// 多少份一并报出去，界面接着点下一轮，不必重新修图。
pub async fn repair_all(db: &Store, cache: &Path, project_id: &str) -> Result<Value> {
    db.get(project_id).await?;
    let total = total_photos(db, project_id).await?;
    let jobs = project_jobs(db, project_id).await?;
    let cache_dir = cache.to_path_buf();
    let scan_jobs = jobs.clone();
    let scanned = tokio::task::spawn_blocking(move || {
        image_pipeline::audit_preview_jobs(&cache_dir, &scan_jobs)
    })
    .await
    .map_err(|error| AppError::new("BACKGROUND", error.to_string()))?;
    let batch: Vec<image_pipeline::PreviewJob> = jobs
        .iter()
        .zip(scanned.iter())
        .filter(|(_, row)| row.outcome == "needs_repair")
        .take(REPAIR_LIMIT)
        .map(|(job, _)| job.clone())
        .collect();
    let targets = scanned
        .iter()
        .filter(|row| row.outcome == "needs_repair")
        .count();
    let remaining = targets.saturating_sub(REPAIR_LIMIT) as i64;
    let repairs = if batch.is_empty() {
        vec![]
    } else {
        let cache_dir = cache.to_path_buf();
        tokio::task::spawn_blocking(move || image_pipeline::repair_preview_jobs(&cache_dir, &batch))
            .await
            .map_err(|error| AppError::new("BACKGROUND", error.to_string()))?
    };
    let repaired = repairs
        .iter()
        .filter(|row| row.outcome == "repaired")
        .count() as i64;
    let refused = repairs
        .iter()
        .filter(|row| row.outcome != "repaired" && row.outcome != "healthy")
        .count() as i64;
    let shown = repairs.len().min(50);
    let mut result = ledger(&scanned, total);
    result["repaired"] = json!(repaired);
    result["refused"] = json!(refused);
    result["remaining"] = json!(remaining);
    result["items"] = serde_json::to_value(&repairs[..shown]).unwrap_or_default();
    if repaired > 0 {
        // 口径欠账按库里的事实点名，不再截断成"最近 20 份"：更早的方案同样会因为预览修好而过期。
        match recheck::project(db, project_id, recheck::ROUND_LIMIT).await {
            Ok(report) => result["recheck"] = report,
            // 修好的文件已经在磁盘上了：重算这一趟没跑成，不该把"修好几张"也一起吞掉。
            Err(error) => {
                result["recheckError"] = json!({"code": error.code, "message": error.message});
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent::types::{AgentRequest, Budget, TaskCommand},
        command_handler::CommandHandler,
        domain,
    };
    use std::path::PathBuf;

    struct Fixture {
        db: Store,
        cache: PathBuf,
        project: String,
        /// 每张照片的（原图，预览）路径，顺序与库里按文件名排好的一致。
        files: Vec<(PathBuf, PathBuf)>,
    }

    async fn fixture(broken: usize) -> (tempfile::TempDir, Fixture) {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("cache");
        let db = Store::open(&temp.path().join("audit.sqlite"))
            .await
            .unwrap();
        let project = db.create("整项目点名", "仅本地合成素材").await.unwrap().id;
        let mut files = vec![];
        for index in 0..4 {
            let file = temp.path().join(format!("photo-{index}.png"));
            image::RgbImage::from_pixel(8, 8, image::Rgb([(index * 40) as u8, 60, 100]))
                .save(&file)
                .unwrap();
            let prepared = image_pipeline::prepare(&file, &cache).unwrap();
            db.insert_photo(&project, &prepared).await.unwrap();
            files.push((prepared.path.clone(), prepared.preview.clone()));
        }
        // 前 broken 张只毁掉缓存里那份派生文件：原图一张都不碰。
        for entry in files.iter().take(broken) {
            std::fs::write(&entry.1, []).unwrap();
        }
        (
            temp,
            Fixture {
                db,
                cache,
                project,
                files,
            },
        )
    }

    /// 真实任务 + 带条目的方案：口径重算走的是任务里那条工具，夹具不能只造一行方案。
    async fn draft(fixture: &Fixture) -> String {
        let task = CommandHandler::new(&fixture.db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: fixture.project.clone(),
                command: TaskCommand::StartTask {
                    goal: "挑有叙事的一组".into(),
                    selected_photo_ids: vec![],
                    target_count: 4,
                    budget: Budget::default(),
                    consent_id: None,
                },
            })
            .await
            .unwrap()
            .id;
        let photos: Vec<String> =
            sqlx::query_scalar("SELECT id FROM photos WHERE project_id=? ORDER BY filename,id")
                .bind(&fixture.project)
                .fetch_all(&fixture.db.pool)
                .await
                .unwrap();
        let draft = domain::id();
        let now = domain::timestamp();
        sqlx::query("INSERT INTO drafts(id,project_id,task_id,title,base_revision,revision,state,summary,created_at,updated_at) VALUES(?,?,?,?,1,1,'pending',?,?,?)")
            .bind(&draft)
            .bind(&fixture.project)
            .bind(&task)
            .bind("点名方案")
            .bind("按本地事实挑出的一组")
            .bind(&now)
            .bind(&now)
            .execute(&fixture.db.pool)
            .await
            .unwrap();
        for (position, photo) in photos.iter().enumerate() {
            sqlx::query("INSERT INTO draft_items(draft_id,photo_id,proposed_action,position,rationale) VALUES(?,?,?,?,?)")
                .bind(&draft)
                .bind(photo)
                .bind("keep")
                .bind(position as i64)
                .bind("开篇")
                .execute(&fixture.db.pool)
                .await
                .unwrap();
        }
        draft
    }

    #[tokio::test]
    async fn audit_counts_the_whole_project_without_touching_a_byte() {
        let (temp, fixture) = fixture(2).await;
        let ledger = audit(&fixture.db, &fixture.cache, &fixture.project)
            .await
            .unwrap();
        assert_eq!(ledger["total"], 4);
        assert_eq!(ledger["scanned"], 4);
        assert_eq!(ledger["needsRepair"], 2);
        assert_eq!(ledger["healthy"], 2);
        assert_eq!(ledger["blocked"], 0);
        // 点名单要能点名到具体哪几张，否则用户仍然只能一页一页翻。
        assert_eq!(ledger["items"].as_array().unwrap().len(), 2);
        // 图库那条状态靠文件名和编号点名：名字与库里这条记录对不上，界面就会指错照片。
        for (index, item) in ledger["items"].as_array().unwrap().iter().enumerate() {
            let filename = format!("photo-{index}.png");
            assert_eq!(item["filename"], filename);
            assert_eq!(item["outcome"], "needs_repair");
            let recorded: String =
                sqlx::query_scalar("SELECT id FROM photos WHERE project_id=? AND filename=?")
                    .bind(&fixture.project)
                    .bind(&filename)
                    .fetch_one(&fixture.db.pool)
                    .await
                    .unwrap();
            assert_eq!(item["photoId"], recorded);
        }
        for entry in fixture.files.iter().take(2) {
            assert_eq!(
                std::fs::metadata(&entry.1).unwrap().len(),
                0,
                "清点不该顺手把文件修好"
            );
        }
        drop(temp);
    }

    #[tokio::test]
    async fn blocked_photos_are_counted_by_reason_and_never_repaired() {
        let (temp, fixture) = fixture(0).await;
        // 原图被挪走：这张既不该被修，也不该被算成"能修"。
        std::fs::remove_file(&fixture.files[1].0).unwrap();
        std::fs::write(&fixture.files[1].1, []).unwrap();
        let ledger = audit(&fixture.db, &fixture.cache, &fixture.project)
            .await
            .unwrap();
        assert_eq!(ledger["needsRepair"], 0, "原图没了的不能算能修");
        assert_eq!(ledger["blocked"], 1);
        assert_eq!(ledger["reasons"]["original_missing"], 1);
        assert_eq!(std::fs::metadata(&fixture.files[1].1).unwrap().len(), 0);
        // 一键修也不能替它决定：原图不在，谁都动不了这张。
        let report = repair_all(&fixture.db, &fixture.cache, &fixture.project)
            .await
            .unwrap();
        assert_eq!(report["repaired"], 0);
        assert_eq!(report["remaining"], 0);
        assert_eq!(std::fs::metadata(&fixture.files[1].1).unwrap().len(), 0);
        drop(temp);
    }

    #[tokio::test]
    async fn repair_all_fixes_every_photo_it_named_and_rechecks_the_draft() {
        let (temp, fixture) = fixture(3).await;
        let draft = draft(&fixture).await;
        let before = audit(&fixture.db, &fixture.cache, &fixture.project)
            .await
            .unwrap();
        assert_eq!(before["needsRepair"], 3);
        let report = repair_all(&fixture.db, &fixture.cache, &fixture.project)
            .await
            .unwrap();
        assert_eq!(report["repaired"], 3);
        assert_eq!(report["refused"], 0);
        assert_eq!(report["remaining"], 0);
        assert_eq!(report["recheck"]["recheckedDrafts"], 1);
        assert_eq!(report["recheck"]["recheckFailed"], 0);
        assert_eq!(report["recheck"]["recheckOwed"], 1);
        assert_eq!(report["recheck"]["recheckRemaining"], 0);
        assert!(!draft.is_empty());
        for entry in &fixture.files {
            assert!(
                image::image_dimensions(&entry.1).is_ok(),
                "修完每张都该还能读"
            );
        }
        let after = audit(&fixture.db, &fixture.cache, &fixture.project)
            .await
            .unwrap();
        assert_eq!(after["needsRepair"], 0);
        assert_eq!(after["healthy"], 4);
        // 口径重算走的是回执表，不是另存一份结果。
        let receipts: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls WHERE tool_name='evaluate_draft'")
                .fetch_one(&fixture.db.pool)
                .await
                .unwrap();
        assert_eq!(receipts, 1);
        drop(temp);
    }

    #[tokio::test]
    async fn one_call_reaches_photos_that_a_single_page_never_shows_together() {
        let (temp, fixture) = fixture(0).await;
        // 再来 12 张，坏的那几张分散在"界面分页"永远凑不到一起的位置上。
        for index in 0..12 {
            let file = temp.path().join(format!("bulk-{index}.png"));
            image::RgbImage::from_pixel(8, 8, image::Rgb([9, 90 + index as u8, 200]))
                .save(&file)
                .unwrap();
            let prepared = image_pipeline::prepare(&file, &fixture.cache).unwrap();
            fixture
                .db
                .insert_photo(&fixture.project, &prepared)
                .await
                .unwrap();
            if index % 2 == 0 {
                std::fs::write(&prepared.preview, []).unwrap();
            }
        }
        let before = audit(&fixture.db, &fixture.cache, &fixture.project)
            .await
            .unwrap();
        assert_eq!(before["total"], 16);
        assert_eq!(before["needsRepair"], 6);
        let report = repair_all(&fixture.db, &fixture.cache, &fixture.project)
            .await
            .unwrap();
        assert_eq!(report["repaired"], 6, "一次点全不该只修看得见的某一页");
        assert_eq!(report["remaining"], 0);
        assert_eq!(
            audit(&fixture.db, &fixture.cache, &fixture.project)
                .await
                .unwrap()["needsRepair"],
            0
        );
        drop(temp);
    }

    #[tokio::test]
    async fn unknown_project_is_refused_before_any_file_is_read() {
        let (temp, fixture) = fixture(1).await;
        assert!(
            audit(&fixture.db, &fixture.cache, "不在这个库里的项目")
                .await
                .is_err()
        );
        assert!(
            repair_all(&fixture.db, &fixture.cache, "不在这个库里的项目")
                .await
                .is_err()
        );
        // 失败那趟不该留下任何回执。
        let receipts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls")
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap();
        assert_eq!(receipts, 0);
        drop(temp);
    }

    #[tokio::test]
    async fn repairing_reaches_drafts_older_than_the_twenty_most_recent() {
        let (temp, fixture) = fixture(1).await;
        let first = draft(&fixture).await;
        // 同一个任务下再复制 24 份方案：原来只重算"最近 20 份"，更早的这些修完仍旧口径过期。
        for _ in 0..24 {
            let copy = domain::id();
            sqlx::query("INSERT INTO drafts(id,project_id,task_id,title,base_revision,revision,state,summary,created_at,updated_at) SELECT ?,project_id,task_id,title,base_revision,revision,state,summary,created_at,updated_at FROM drafts WHERE id=?")
                .bind(&copy)
                .bind(&first)
                .execute(&fixture.db.pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO draft_items(draft_id,photo_id,proposed_action,position,rationale) SELECT ?,photo_id,proposed_action,position,rationale FROM draft_items WHERE draft_id=?")
                .bind(&copy)
                .bind(&first)
                .execute(&fixture.db.pool)
                .await
                .unwrap();
        }
        let report = repair_all(&fixture.db, &fixture.cache, &fixture.project)
            .await
            .unwrap();
        assert_eq!(report["repaired"], 1);
        assert_eq!(report["recheck"]["drafts"], 25);
        assert_eq!(report["recheck"]["recheckOwed"], 25);
        assert_eq!(report["recheck"]["recheckedDrafts"], 25);
        assert_eq!(report["recheck"]["recheckRemaining"], 0);
        let receipts: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tool_calls WHERE tool_name='evaluate_draft'")
                .fetch_one(&fixture.db.pool)
                .await
                .unwrap();
        assert_eq!(receipts, 25);
        // 文件都好了的一轮不该再算一遍口径：欠账按库里的事实点名，不按"上次修过"。
        let again = repair_all(&fixture.db, &fixture.cache, &fixture.project)
            .await
            .unwrap();
        assert_eq!(again["repaired"], 0);
        assert!(again.get("recheck").is_none(), "没修任何文件就不顺手重算");
        drop(temp);
    }
}
