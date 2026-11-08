//! 历史观察"当年没记像素版本"的点名与回填。
//!
//! 加那一列之前留下的观察没有记像素指纹，口径只能把它们报成"版本未记录"，一份方案的画面
//! 依据里就用不上它们。补一个值不能靠猜：写进去的指纹要能被当成"当初发出去的那一份字节"，
//! 否则就是把一次没核对过的观察洗白成核对过。
//!
//! 证明只有三步，全程本地只读：这条观察记着问法用的是哪一档（档位换了，交给模型的字节就不是
//! 同一份）；它记着的原图指纹与库里现在在册的仍是同一张；磁盘上那份预览压出来的字节与从原图
//! 重算出来的逐字节相同（缓存没被别的问法或别的内容写过）。三条同时成立才写回去，少一条就按
//! 原因码留在拒绝里。
//!
//! 第一步靠的是"档位字符串在流程真的换过时跟着换过"这个约定，不是证据：当年若改过缩放长边或
//! 质量却没换版本号，本地无从察觉，会被当成同一档字节补上指纹。界面那句"能证明"就建立在这上面。
//!
//! 回填之后收着这些照片的方案要跟着重算口径：那几张之前被排除在画面依据之外，不重算的话界面
//! 上的数字还停在"版本没记过"的那一晚。重算走的是同一条本地通道，不经过模型、不上传照片、不花额度。

use crate::{
    agent::{
        recheck,
        upload_compress::compress_profile,
        vision::{DETAIL_PROMPT_VERSION, VISION_PROMPT_VERSION, detail_cache},
    },
    db::Store,
    domain::Result,
    image_pipeline,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    result::Result as StdResult,
};

/// 一轮最多补多少条：每条都要把原图整张重算一遍像素，比点名贵得多。
pub const APPLY_LIMIT: i64 = 500;
/// 点名的规模上限，超出就在结果里说清点到了多少，不假装数完了。
const SCAN_LIMIT: i64 = 5000;
/// 点名单上最多列几条，其余只按原因码计入总数。
const ITEM_LIMIT: usize = 50;

/// 一条没有像素版本的历史观察，加上判断它能不能补上所需的全部本地事实。
struct Candidate {
    id: i64,
    photo_id: String,
    filename: String,
    /// 这条观察当年是按哪一档问法做出来的：档位决定交给模型的字节。
    detail: bool,
    /// 记录里的问法版本号，档位部分要今天还认得才算同一份字节。
    prompt_version: String,
    /// 观察做出时记录的原图指纹。
    observed_hash: String,
    /// 库里现在在册的原图指纹。
    current_hash: String,
    preview_path: String,
    original_path: String,
}

/// 一条观察的点名结果：`Ok` 是能补，给的就是要写回去的字节指纹；`Err` 是补不了的原因码。
type Proof = StdResult<String, &'static str>;

/// 能不能证明"当初发出去的就是现在这一份字节"。
///
/// 每一步都只读在册文件；重算只写系统临时目录里一个随机命名的目录，用完就删——
/// 图库缓存、原图、数据库都不动。
fn prove(row: &Candidate) -> Proof {
    let want = if row.detail {
        DETAIL_PROMPT_VERSION
    } else {
        VISION_PROMPT_VERSION
    };
    // 问法版本写成"<档位>@<端点>"：只看档位前缀，换个端点仍旧是同一档像素；
    // 连端点部分都没有的那条形状就不全，说不出是哪一档字节，照样拦下。
    if !row.prompt_version.starts_with(&format!("{want}@")) {
        return Err("pipeline_changed");
    }
    if row.observed_hash != row.current_hash {
        return Err("original_changed");
    }
    let original = Path::new(&row.original_path);
    if !original.is_file() {
        return Err("original_missing");
    }
    let stored = match compress_profile(Path::new(&row.preview_path), row.detail) {
        Ok((_, hash)) => hash,
        Err(_) => return Err("preview_unreadable"),
    };
    let staging = std::env::temp_dir().join(format!(
        "between-frames-observation-backfill-{}",
        crate::domain::id()
    ));
    let rebuilt = match image_pipeline::prepare(original, &staging) {
        // 重算时再比一次原图指纹：文件在这趟里被换过，那份字节就不是这一张的。
        Ok(built) if built.hash != row.current_hash => Err("original_changed"),
        Ok(built) => match compress_profile(&built.preview, row.detail) {
            Ok((_, hash)) => Ok(hash),
            Err(_) => Err("rebuild_failed"),
        },
        Err(_) => Err("original_unreadable"),
    };
    let _ = std::fs::remove_dir_all(&staging);
    match rebuilt {
        // 磁盘上那份缓存与从原图重算出来的不一致：它不是这一档问法算出来的，说不清当初发的是哪份。
        Ok(hash) if hash == stored => Ok(stored),
        Ok(_) => Err("cache_differs"),
        Err(reason) => Err(reason),
    }
}

async fn candidates(db: &Store, project_id: &str) -> Result<Vec<Candidate>> {
    let rows = sqlx::query_as::<_, (i64, String, String, String, String, String, String, String)>(
        "SELECT o.id,o.photo_id,p.filename,o.prompt_version,o.photo_hash,p.content_sha256,p.preview_path,p.original_path \
         FROM photo_observations o JOIN photos p ON p.id=o.photo_id \
         WHERE p.project_id=? AND o.preview_sha256 IS NULL ORDER BY o.id LIMIT ?",
    )
    .bind(project_id)
    .bind(SCAN_LIMIT)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(
                id,
                photo_id,
                filename,
                prompt_version,
                observed_hash,
                current_hash,
                preview_path,
                original_path,
            )| Candidate {
                id,
                photo_id,
                filename,
                detail: detail_cache(&prompt_version),
                prompt_version,
                observed_hash,
                current_hash,
                preview_path,
                original_path,
            },
        )
        .collect())
}

/// 这个项目里还有多少条观察没记像素版本。
async fn unrecorded(db: &Store, project_id: &str) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM photo_observations o JOIN photos p ON p.id=o.photo_id \
         WHERE p.project_id=? AND o.preview_sha256 IS NULL",
    )
    .bind(project_id)
    .fetch_one(&db.pool)
    .await?;
    Ok(count)
}

fn scan(rows: &[Candidate]) -> Vec<(&Candidate, Proof)> {
    rows.iter().map(|row| (row, prove(row))).collect()
}

/// 一份点名单：能补的、补不了的、各因为什么分开报，界面无需自己再归类。
///
/// 数字口径只少不多报：没点到的那批不算进"补不了"，另有 `scanned` 与 `rows` 对照。
fn ledger(rows: &[(&Candidate, Proof)], total: i64, applied: Option<i64>) -> Value {
    let ready = rows.iter().filter(|(_, proof)| proof.is_ok()).count() as i64;
    let mut reasons: BTreeMap<String, i64> = BTreeMap::new();
    for code in rows.iter().filter_map(|(_, proof)| proof.as_ref().err()) {
        // 只有"补不了"的才进原因码，能补的一条都不往里塞。
        *reasons.entry((*code).to_owned()).or_default() += 1;
    }
    let photos: BTreeSet<&str> = rows.iter().map(|(row, _)| row.photo_id.as_str()).collect();
    let items: Vec<Value> = rows
        .iter()
        .take(ITEM_LIMIT)
        .map(|(row, proof)| {
            json!({
                "observationId": row.id,
                "photoId": row.photo_id,
                "filename": row.filename,
                "outcome": match proof {
                    Ok(_) => "ready".to_owned(),
                    Err(code) => code.to_string(),
                },
            })
        })
        .collect();
    let mut value = json!({
        "rows": total,
        "scanned": rows.len() as i64,
        "photos": photos.len() as i64,
        "ready": ready,
        "refused": rows.len() as i64 - ready,
        "reasons": reasons,
        "items": items,
    });
    if let Some(done) = applied {
        value["backfilled"] = json!(done);
        // 这一轮之后还欠多少：能补而没轮到补的，这一轮仍旧欠着。
        value["remaining"] = json!((ready - done).max(0));
    }
    value
}

/// 只读点名：一个字节都不改，也不写回任何一条观察。
pub async fn audit(db: &Store, project_id: &str) -> Result<Value> {
    db.get(project_id).await?;
    let total = unrecorded(db, project_id).await?;
    let rows = candidates(db, project_id).await?;
    Ok(ledger(&scan(&rows), total, None))
}

/// 把能证明的那部分补上像素指纹，并让收着这些照片的方案口径跟着重算。
///
/// 写入带上 `AND preview_sha256 IS NULL`：点名之后这一条要是已经被别处补上（比如那次真的重新
/// 观察过），这里就不覆盖它——补的是"没记过"，不是"记的内容我不喜欢"。
pub async fn apply(db: &Store, project_id: &str, limit: i64) -> Result<Value> {
    db.get(project_id).await?;
    let total = unrecorded(db, project_id).await?;
    let rows = candidates(db, project_id).await?;
    let checked = scan(&rows);
    let mut backfilled = 0i64;
    let mut touched: Vec<String> = Vec::new();
    for (row, hash) in checked
        .iter()
        .filter_map(|(row, proof)| proof.as_ref().ok().map(|hash| (*row, hash)))
        .take(limit.max(0) as usize)
    {
        let written = sqlx::query(
            "UPDATE photo_observations SET preview_sha256=? WHERE id=? AND preview_sha256 IS NULL",
        )
        .bind(hash)
        .bind(row.id)
        .execute(&db.pool)
        .await?;
        if written.rows_affected() == 1 {
            backfilled += 1;
            if !touched.contains(&row.photo_id) {
                touched.push(row.photo_id.clone());
            }
        }
    }
    let mut value = ledger(&checked, total, Some(backfilled));
    if backfilled > 0 {
        // 补指纹不改方案版本号，欠账点名（`recheck::project`）看不出这些方案已经过期：按照片点名重算。
        match recheck::drafts_covering(db, project_id, &touched, recheck::ROUND_LIMIT).await {
            Ok(report) => value["recheck"] = report,
            // 补好的指纹已经在库里了：口径这一趟没跑成不该把整次回填报成失败。
            Err(error) => {
                value["recheckError"] = json!({"code": error.code, "message": error.message});
            }
        }
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent::{
            types::{AgentRequest, Budget, TaskCommand},
            vision::current_preview_status_for_version,
        },
        command_handler::CommandHandler,
        domain,
    };

    struct Fixture {
        db: Store,
        project: String,
        photos: Vec<String>,
        page_version: String,
    }

    async fn add_photo(
        temp: &tempfile::TempDir,
        db: &Store,
        project: &str,
        index: usize,
    ) -> String {
        let file = temp.path().join(format!("frame-{index}.png"));
        image::RgbImage::from_pixel(
            24,
            16,
            image::Rgb([(index * 55) as u8, 90, (index * 30) as u8]),
        )
        .save(&file)
        .unwrap();
        let prepared = image_pipeline::prepare(&file, &temp.path().join("cache")).unwrap();
        db.insert_photo(project, &prepared).await.unwrap();
        sqlx::query_scalar("SELECT id FROM photos WHERE project_id=? AND filename=?")
            .bind(project)
            .bind(format!("frame-{index}.png"))
            .fetch_one(&db.pool)
            .await
            .unwrap()
    }

    /// 一条真的做过、但当年没记像素版本的历史观察：问法档位用现在认得的那个，端点部分是夹具。
    async fn unrecorded_observation(db: &Store, photo: &str, version: &str) {
        let hash: String = sqlx::query_scalar("SELECT content_sha256 FROM photos WHERE id=?")
            .bind(photo)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,created_at) VALUES(?,?,'fixture',?,'inspect_photos',?,?)")
            .bind(photo).bind(hash).bind(version)
            .bind(json!({"photo_id":photo,"visible_facts":["画面里有一个站立的人影"],"interpretation":"可能的停顿","uncertainty":"夹具，不代表模型水平"}).to_string())
            .bind(domain::timestamp())
            .execute(&db.pool).await.unwrap();
    }

    /// 四张真图各配一条没记版本的历史观察：能证的、原图换过的、档位不认的、预览文件没的。
    async fn fixture() -> (tempfile::TempDir, Fixture) {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("backfill.sqlite"))
            .await
            .unwrap();
        let project = db
            .create("回填像素版本", "仅本地合成素材")
            .await
            .unwrap()
            .id;
        let hex = "b".repeat(64);
        let page_version = format!("{VISION_PROMPT_VERSION}@{hex}");
        let mut photos = Vec::new();
        for index in 0..4 {
            photos.push(add_photo(&temp, &db, &project, index).await);
        }
        for (index, photo) in photos.iter().enumerate() {
            let version = if index == 2 {
                "m3-vision-legacy-unread@fixture"
            } else {
                &page_version
            };
            unrecorded_observation(&db, photo, version).await;
        }
        // 第二条：观察之后原图记录被换过，库里的在册指纹与记录里的对不上。
        sqlx::query("UPDATE photos SET content_sha256=? WHERE id=?")
            .bind("c".repeat(64))
            .bind(&photos[1])
            .execute(&db.pool)
            .await
            .unwrap();
        // 第四条：本地预览文件不在了。
        let path: String = sqlx::query_scalar("SELECT preview_path FROM photos WHERE id=?")
            .bind(&photos[3])
            .fetch_one(&db.pool)
            .await
            .unwrap();
        std::fs::remove_file(&path).unwrap();
        (
            temp,
            Fixture {
                db,
                project,
                photos,
                page_version,
            },
        )
    }

    async fn outcomes(fixture: &Fixture) -> Value {
        audit(&fixture.db, &fixture.project).await.unwrap()
    }

    async fn recorded(fixture: &Fixture, photo: &str) -> Option<String> {
        sqlx::query_scalar(
            "SELECT o.preview_sha256 FROM photo_observations o JOIN photos p ON p.id=o.photo_id WHERE p.id=?",
        )
        .bind(photo)
        .fetch_one(&fixture.db.pool)
        .await
        .unwrap()
    }

    async fn preview_path(fixture: &Fixture, photo: &str) -> String {
        sqlx::query_scalar("SELECT preview_path FROM photos WHERE id=?")
            .bind(photo)
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn the_ledger_separates_what_can_be_proved_from_what_stands_in_the_way() {
        let (_temp, fixture) = fixture().await;
        let ledger = outcomes(&fixture).await;
        assert_eq!(ledger["rows"], json!(4));
        assert_eq!(ledger["scanned"], json!(4));
        assert_eq!(ledger["photos"], json!(4));
        assert_eq!(ledger["ready"], json!(1), "只有三条都成立的那条能补");
        assert_eq!(ledger["refused"], json!(3));
        assert_eq!(ledger["reasons"]["pipeline_changed"], json!(1));
        assert_eq!(ledger["reasons"]["original_changed"], json!(1));
        assert_eq!(ledger["reasons"]["preview_unreadable"], json!(1));
        assert_eq!(ledger["items"].as_array().unwrap().len(), 4);
        assert!(
            ledger["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["photoId"] == json!(&fixture.photos[0])
                    && row["outcome"] == json!("ready")),
            "能补的那条要在名单上点名"
        );
        assert!(ledger.get("backfilled").is_none());
        // 点名一个字节都不改：那四条仍旧没记版本。
        assert_eq!(unrecorded(&fixture.db, &fixture.project).await.unwrap(), 4);
    }

    #[tokio::test]
    async fn backfilling_writes_the_bytes_this_tier_still_produces() {
        let (_temp, fixture) = fixture().await;
        let report = apply(&fixture.db, &fixture.project, APPLY_LIMIT)
            .await
            .unwrap();
        assert_eq!(report["backfilled"], json!(1));
        assert_eq!(report["ready"], json!(1));
        assert_eq!(report["remaining"], json!(0));
        let path = preview_path(&fixture, &fixture.photos[0]).await;
        let written = recorded(&fixture, &fixture.photos[0])
            .await
            .expect("补过的那条要真的写进库里");
        let (_, current) = compress_profile(Path::new(&path), false).unwrap();
        assert_eq!(written, current, "写进去的就是下一次要发的那份字节");
        assert_eq!(
            current_preview_status_for_version(Some(&written), &path, &fixture.page_version).await,
            "verified"
        );
        // 补不了的三条一个字都不动，尤其不拿"看起来差不多"洗白成核对过。
        assert_eq!(unrecorded(&fixture.db, &fixture.project).await.unwrap(), 3);
        for photo in [&fixture.photos[1], &fixture.photos[2], &fixture.photos[3]] {
            assert_eq!(
                recorded(&fixture, photo).await,
                None,
                "挡路的那条不该被顺手补平"
            );
        }
    }

    #[tokio::test]
    async fn a_cache_file_this_tier_did_not_produce_is_refused_rather_than_guessed() {
        let (temp, fixture) = fixture().await;
        // 把在册预览换成另一张图的字节：这一档压出来的东西与原图重算的对不上。
        let decoy = temp.path().join("decoy.png");
        image::RgbImage::from_pixel(300, 300, image::Rgb([250, 12, 240]))
            .save(&decoy)
            .unwrap();
        let path = preview_path(&fixture, &fixture.photos[0]).await;
        std::fs::copy(&decoy, &path).unwrap();
        let ledger = outcomes(&fixture).await;
        assert_eq!(ledger["reasons"]["cache_differs"], json!(1));
        assert_eq!(ledger["ready"], json!(0));
        let report = apply(&fixture.db, &fixture.project, APPLY_LIMIT)
            .await
            .unwrap();
        assert_eq!(report["backfilled"], json!(0));
        assert!(report.get("recheck").is_none());
        assert_eq!(unrecorded(&fixture.db, &fixture.project).await.unwrap(), 4);
    }

    #[tokio::test]
    async fn a_round_reports_what_it_did_not_reach_and_the_next_round_finishes_it() {
        let (temp, mut fixture) = fixture().await;
        // 再造两条同样能证的：一轮只补一条时欠账要说清还剩几条。
        for index in 4..6 {
            let photo = add_photo(&temp, &fixture.db, &fixture.project, index).await;
            unrecorded_observation(&fixture.db, &photo, &fixture.page_version).await;
            fixture.photos.push(photo);
        }
        let first = apply(&fixture.db, &fixture.project, 1).await.unwrap();
        assert_eq!(first["ready"], json!(3));
        assert_eq!(first["backfilled"], json!(1));
        assert_eq!(first["remaining"], json!(2), "一轮补不完就说还欠几条");
        assert_eq!(unrecorded(&fixture.db, &fixture.project).await.unwrap(), 5);
        let second = apply(&fixture.db, &fixture.project, 1).await.unwrap();
        assert_eq!(second["ready"], json!(2));
        assert_eq!(second["backfilled"], json!(1));
        assert_eq!(second["remaining"], json!(1));
        let rest = apply(&fixture.db, &fixture.project, APPLY_LIMIT)
            .await
            .unwrap();
        assert_eq!(rest["backfilled"], json!(1));
        assert_eq!(rest["remaining"], json!(0));
        // 剩下三条各自挡在原图换过、档位不认、预览不在上，多轮也不该被补平。
        assert_eq!(unrecorded(&fixture.db, &fixture.project).await.unwrap(), 3);
        assert_eq!(outcomes(&fixture).await["ready"], json!(0));
    }

    #[tokio::test]
    async fn backfilling_lets_the_draft_that_holds_the_photo_use_the_observation() {
        let (_temp, fixture) = fixture().await;
        let task = CommandHandler::new(&fixture.db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: fixture.project.clone(),
                command: TaskCommand::StartTask {
                    goal: "挑有叙事的一组".into(),
                    selected_photo_ids: vec![],
                    target_count: 1,
                    budget: Budget::default(),
                    consent_id: None,
                },
            })
            .await
            .unwrap()
            .id;
        let now = domain::timestamp();
        let draft = domain::id();
        sqlx::query("INSERT INTO drafts(id,project_id,task_id,title,base_revision,revision,state,summary,created_at,updated_at) VALUES(?,?,?,?,1,1,'pending',?,?,?)")
            .bind(&draft).bind(&fixture.project).bind(&task).bind("回填夹具")
            .bind("按本地事实挑出的一组").bind(&now).bind(&now)
            .execute(&fixture.db.pool).await.unwrap();
        sqlx::query("INSERT INTO draft_items(draft_id,photo_id,proposed_action,position,rationale) VALUES(?,?,?,?,?)")
            .bind(&draft).bind(&fixture.photos[0]).bind("keep").bind(0).bind("开篇")
            .execute(&fixture.db.pool).await.unwrap();
        // 先按欠账点名算一遍口径：这条观察没记版本，那份画面依据里用不上它。
        let before = recheck::project(&fixture.db, &fixture.project, recheck::ROUND_LIMIT)
            .await
            .unwrap();
        assert_eq!(before["recheckedDrafts"], json!(1));
        let evidence = receipt(&fixture, &draft).await;
        assert_eq!(evidence["metrics"]["observed"], json!(0));
        assert_eq!(
            evidence["omitted_visual_evidence"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            evidence["omitted_visual_evidence"][0]["pixel_version"],
            json!("unrecorded")
        );
        let usage_before = usage(&fixture).await;
        let revision_before = revision(&fixture).await;
        // 补指纹不改方案版本号：光看欠账会以为一切都跟上了，这几张仍旧被排除在画面依据外。
        assert_eq!(
            recheck::project(&fixture.db, &fixture.project, recheck::ROUND_LIMIT)
                .await
                .unwrap()["recheckOwed"],
            json!(0)
        );
        let report = apply(&fixture.db, &fixture.project, APPLY_LIMIT)
            .await
            .unwrap();
        assert_eq!(report["recheck"]["recheckedDrafts"], json!(1));
        assert_eq!(report["recheck"]["recheckRemaining"], json!(0));
        let after = receipt(&fixture, &draft).await;
        assert_eq!(
            after["metrics"]["observed"],
            json!(1),
            "补上指纹的那条观察要真的被当成画面依据"
        );
        assert!(
            after["omitted_visual_evidence"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        // 全程本地：不新建任务、不动项目版本号、不花一次模型额度。
        assert_eq!(usage(&fixture).await, usage_before, "回填一轮不该花额度");
        assert_eq!(revision(&fixture).await, revision_before);
        let tasks: i64 = sqlx::query_scalar("SELECT count(*) FROM agent_tasks WHERE project_id=?")
            .bind(&fixture.project)
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap();
        assert_eq!(tasks, 1);
    }

    #[tokio::test]
    async fn local_recheck_records_whether_each_omitted_original_can_still_be_repaired() {
        let (temp, fixture) = fixture().await;
        apply(&fixture.db, &fixture.project, APPLY_LIMIT)
            .await
            .unwrap();
        // 只用能补的照片：需要查原图指纹的恰好 8 张，等于每轮上限，谁先谁后都查得到。
        let mut photos = vec![fixture.photos[0].clone()];
        for index in 4..15 {
            let photo = add_photo(&temp, &fixture.db, &fixture.project, index).await;
            unrecorded_observation(&fixture.db, &photo, &fixture.page_version).await;
            photos.push(photo);
        }
        let backfilled = apply(&fixture.db, &fixture.project, APPLY_LIMIT)
            .await
            .unwrap();
        assert_eq!(backfilled["backfilled"], json!(11));
        for photo in &photos {
            let preview = preview_path(&fixture, photo).await;
            std::fs::write(preview, b"no longer an image").unwrap();
        }
        let changed_path: String =
            sqlx::query_scalar("SELECT original_path FROM photos WHERE id=?")
                .bind(&photos[1])
                .fetch_one(&fixture.db.pool)
                .await
                .unwrap();
        std::fs::write(changed_path, b"different original bytes").unwrap();
        let missing_path: String =
            sqlx::query_scalar("SELECT original_path FROM photos WHERE id=?")
                .bind(&photos[2])
                .fetch_one(&fixture.db.pool)
                .await
                .unwrap();
        std::fs::remove_file(missing_path).unwrap();
        for photo in &photos[9..] {
            sqlx::query("UPDATE photo_observations SET preview_sha256=NULL WHERE photo_id=?")
                .bind(photo)
                .execute(&fixture.db.pool)
                .await
                .unwrap();
        }
        let task = CommandHandler::new(&fixture.db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: fixture.project.clone(),
                command: TaskCommand::StartTask {
                    goal: "核对每张被排除的原图状态".into(),
                    selected_photo_ids: photos.clone(),
                    target_count: photos.len() as u32,
                    budget: Budget::default(),
                    consent_id: None,
                },
            })
            .await
            .unwrap()
            .id;
        let now = domain::timestamp();
        let draft = domain::id();
        sqlx::query("INSERT INTO drafts(id,project_id,task_id,title,base_revision,revision,state,summary,created_at,updated_at) VALUES(?,?,?,?,1,1,'pending',?,?,?)")
            .bind(&draft).bind(&fixture.project).bind(&task).bind("原图状态夹具")
            .bind("只走本地重算").bind(&now).bind(&now)
            .execute(&fixture.db.pool).await.unwrap();
        for (position, photo) in photos.iter().enumerate() {
            sqlx::query("INSERT INTO draft_items(draft_id,photo_id,proposed_action,position,rationale) VALUES(?,?,?,?,?)")
                .bind(&draft).bind(photo).bind("keep").bind(position as i64).bind("本地核对")
                .execute(&fixture.db.pool).await.unwrap();
        }
        let usage_before = usage(&fixture).await;
        let revision_before = revision(&fixture).await;
        let summary = recheck::run(&fixture.db, &fixture.project, &draft)
            .await
            .unwrap();
        assert_eq!(summary["observed"], json!(0));
        assert_eq!(summary["omitted"], json!(12));
        let evidence = receipt(&fixture, &draft).await;
        let omitted = evidence["omitted_visual_evidence"].as_array().unwrap();
        let status = |photo: &str| {
            omitted
                .iter()
                .find(|row| row["photo_id"] == json!(photo))
                .unwrap()
        };
        assert_eq!(status(&photos[0])["original_status"], json!("matches"));
        assert_eq!(status(&photos[1])["original_status"], json!("changed"));
        assert_eq!(status(&photos[2])["original_status"], json!("missing"));
        for photo in &photos[3..9] {
            assert_eq!(status(photo)["original_status"], json!("matches"));
        }
        for photo in &photos[9..] {
            assert_eq!(status(photo)["pixel_version"], json!("unrecorded"));
            assert!(status(photo).get("original_status").is_none());
        }
        assert_eq!(usage(&fixture).await, usage_before);
        assert_eq!(revision(&fixture).await, revision_before);
    }

    #[tokio::test]
    async fn the_pass_only_touches_the_project_it_was_named() {
        let (_temp, fixture) = fixture().await;
        let other = fixture
            .db
            .create("另一个项目", "只验证点名范围")
            .await
            .unwrap()
            .id;
        let ledger = audit(&fixture.db, &other).await.unwrap();
        assert_eq!(ledger["rows"], json!(0));
        assert_eq!(ledger["scanned"], json!(0));
        let report = apply(&fixture.db, &other, APPLY_LIMIT).await.unwrap();
        assert_eq!(report["backfilled"], json!(0));
        assert!(report.get("recheck").is_none());
        assert_eq!(unrecorded(&fixture.db, &fixture.project).await.unwrap(), 4);
        assert!(audit(&fixture.db, "不在这个库里的项目").await.is_err());
        assert_eq!(unrecorded(&fixture.db, &fixture.project).await.unwrap(), 4);
    }

    #[tokio::test]
    async fn prove_checks_the_tier_before_it_checks_the_files() {
        // 档位比对只看前缀不看端点：认得档位但两张不是同一张，先按原图换过挡下；指纹一致才轮到查文件。
        let row = Candidate {
            id: 1,
            photo_id: "p".into(),
            filename: "p.png".into(),
            detail: false,
            prompt_version: format!("{VISION_PROMPT_VERSION}@{}", "d".repeat(64)),
            observed_hash: "e".repeat(64),
            current_hash: "f".repeat(64),
            preview_path: "/nonexistent/preview.jpg".into(),
            original_path: "/nonexistent/original.png".into(),
        };
        assert_eq!(
            prove(&row).unwrap_err(),
            "original_changed",
            "档位认得但两张不是同一张，该按原图换过挡下"
        );
        let same_content = Candidate {
            observed_hash: "f".repeat(64),
            ..row
        };
        assert_eq!(
            prove(&same_content).unwrap_err(),
            "original_missing",
            "指纹一致而原图不在，就该说原图不在"
        );
        assert_eq!(
            prove(&Candidate {
                prompt_version: VISION_PROMPT_VERSION.into(),
                ..same_content
            })
            .unwrap_err(),
            "pipeline_changed",
            "版本号没有端点部分也不算这一档字节"
        );
    }

    async fn receipt(fixture: &Fixture, draft: &str) -> Value {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT result_json FROM tool_calls WHERE tool_name='evaluate_draft' ORDER BY rowid DESC",
        )
        .fetch_all(&fixture.db.pool)
        .await
        .unwrap();
        for raw in rows {
            let value: Value = serde_json::from_str(&raw).unwrap();
            if value["result"]["draft_id"].as_str() == Some(draft) {
                return value["result"].clone();
            }
        }
        panic!("这份方案还没留下口径");
    }

    async fn usage(fixture: &Fixture) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM model_usage")
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap()
    }

    async fn revision(fixture: &Fixture) -> i64 {
        sqlx::query_scalar("SELECT revision FROM projects WHERE id=?")
            .bind(&fixture.project)
            .fetch_one(&fixture.db.pool)
            .await
            .unwrap()
    }
}
