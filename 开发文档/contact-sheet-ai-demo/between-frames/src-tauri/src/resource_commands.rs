use crate::{
    command_audit::internal_event,
    command_handler::CommandHandler,
    db,
    domain::{self, AppError, ImportOutcome, PhotoChange, Project, Result},
    image_pipeline::PreparedPhoto,
    sidecar::{self, PhotoJudgement},
};
use serde_json::json;
use sqlx::{Row, Sqlite, Transaction};

impl CommandHandler<'_> {
    pub async fn import_photo(
        &self,
        project: &str,
        prepared: &PreparedPhoto,
    ) -> Result<ImportOutcome> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let revision: i64 =
            sqlx::query_scalar("SELECT revision FROM projects WHERE id=? AND deleted_at IS NULL")
                .bind(project)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| AppError::new("NOT_FOUND", "项目不存在或已归档"))?;
        // 判重看的是"同一份来源文件重复导入"，不是"画面内容相同"：
        // 相同内容来自不同文件时各自保留独立记录，选片状态与序列位置互不影响。
        // 同一路径只认最新一条：内容一致才算重复；指纹不同说明这文件被就地改写过
        // （外部编辑器直接保存回原路径就是这个形状），此时报出来而不是再插第二条。
        let known: Option<(String, String, i64, i64, i64)> = sqlx::query_as(
            "SELECT id,content_sha256,width,height,file_size FROM photos WHERE project_id=? AND original_path=? ORDER BY (content_sha256=?) DESC,import_index DESC LIMIT 1",
        )
        .bind(project)
        .bind(prepared.path.to_string_lossy().as_ref())
        .bind(&prepared.hash)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((photo_id, previous_hash, width, height, size)) = known {
            if previous_hash == prepared.hash {
                tx.commit().await?;
                return Ok(ImportOutcome::Duplicate { photo_id });
            }
            tx.commit().await?;
            return Ok(ImportOutcome::Changed(PhotoChange {
                filename: prepared.filename.clone(),
                incoming_size: prepared.size as i64,
                incoming_width: prepared.width as i64,
                incoming_height: prepared.height as i64,
                incoming_sha256: prepared.hash.clone(),
                path: prepared.path.to_string_lossy().into_owned(),
                photo_id,
                previous_sha256: previous_hash,
                previous_width: width,
                previous_height: height,
                previous_size: size,
            }));
        }
        let id = domain::id();
        let now = domain::timestamp();
        let index: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(import_index),0)+1 FROM photos WHERE project_id=?",
        )
        .bind(project)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO photos(id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,thumbnail_path,preview_path,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)").bind(&id).bind(project).bind(index).bind(prepared.path.to_string_lossy().as_ref()).bind(&prepared.filename).bind(prepared.size as i64).bind(&prepared.hash).bind(prepared.width as i64).bind(prepared.height as i64).bind(prepared.thumbnail.to_string_lossy().as_ref()).bind(prepared.preview.to_string_lossy().as_ref()).bind(&now).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photo_decisions(project_id,photo_id,updated_at) VALUES(?,?,?)")
            .bind(project)
            .bind(&id)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        // G1 dual-write：新导入照片同步写入资产域（不影响旧路径）。
        let locator = format!(
            "[{{\"path\":{path},\"kind\":\"file\"}}]",
            path = serde_json::json!(prepared.path.to_string_lossy().as_ref())
        );
        let format = prepared
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let format = if format.len() > 10 {
            "unknown".to_owned()
        } else {
            format
        };
        // G2：读取源 ICC profile digest（无嵌入 profile 时为 NULL）
        let profile_result = crate::color_engine::read_source_profile(&prepared.path)
            .ok()
            .flatten();
        let profile_digest = match &profile_result {
            Some(p) if p.valid => Some(p.digest.clone()),
            // H0-08：invalid 也记录 digest（带前缀标记诊断），absent 保持 NULL
            Some(p) => Some(format!("invalid:{}", p.diagnostic.as_str())),
            None => None,
        };
        // 相同内容在本项目已有母版资产时复用同一资产，新记录只登记别名；资产表的项目内内容唯一性保持不变。
        let shared_asset: Option<String> = sqlx::query_scalar(
            "SELECT id FROM source_assets WHERE project_id=? AND content_sha256=? AND id<>?",
        )
        .bind(project)
        .bind(&prepared.hash)
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await?;
        let asset_id = match shared_asset {
            Some(existing) => existing,
            None => {
                sqlx::query(
                    "INSERT INTO source_assets(id,project_id,content_sha256,original_locator,file_size,width,height,format,source_profile_digest,source_hash_verified_at,created_at) VALUES(?,?,?,?,?,?,?,?,?,NULL,?)",
                )
                .bind(&id).bind(project).bind(&prepared.hash).bind(&locator)
                .bind(prepared.size as i64).bind(prepared.width as i64).bind(prepared.height as i64)
                .bind(&format).bind(&profile_digest).bind(&now)
                .execute(&mut *tx).await?;
                let variant_id = domain::id();
                sqlx::query(
                    "INSERT INTO edit_variants(id,asset_id,name,external_software,external_variant_id,is_current,created_at) VALUES(?,?,?,NULL,NULL,1,?)",
                )
                .bind(&variant_id).bind(&id).bind("original").bind(&now)
                .execute(&mut *tx).await?;
                // G1 修复：读缓存文件真实尺寸（缩略图按宽高比适配 320px，不是 width/4）
                for (kind, path) in [
                    ("thumbnail", &prepared.thumbnail),
                    ("display_proxy", &prepared.preview),
                ] {
                    let rep_size = std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);
                    let (w, h) = image::image_dimensions(path)
                        .map(|(dw, dh)| (dw as i64, dh as i64))
                        .unwrap_or((prepared.width as i64, prepared.height as i64));
                    sqlx::query(
                        "INSERT INTO asset_representations(id,asset_id,kind,algorithm,width,height,file_path,file_size,profile_digest,created_at) VALUES(?,?,?,?,?,?,?,?,NULL,?)",
                    )
                    .bind(domain::id()).bind(&id).bind(kind).bind("image_pipeline_v1")
                    .bind(w).bind(h).bind(path.to_string_lossy().as_ref()).bind(rep_size).bind(&now)
                    .execute(&mut *tx).await?;
                }
                id.clone()
            }
        };
        sqlx::query("INSERT INTO asset_aliases(photo_id,asset_id) VALUES(?,?)")
            .bind(&id)
            .bind(&asset_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE projects SET revision=revision+1,updated_at=? WHERE id=?")
            .bind(&now)
            .bind(project)
            .execute(&mut *tx)
            .await?;
        let after =
            json!({"photoId":id,"hash":prepared.hash,"originalPath":prepared.path,"index":index});
        internal_event(
            &mut tx,
            project,
            Some(&id),
            "import",
            revision,
            &json!(null),
            &after,
            "system_import",
        )
        .await?;
        tx.commit().await?;
        Ok(ImportOutcome::Inserted { photo_id: id })
    }
    pub async fn relink_photo(
        &self,
        project: &str,
        photo: &str,
        prepared: &PreparedPhoto,
        expected: i64,
    ) -> Result<Project> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let previous = sqlx::query(
            "SELECT original_path,content_sha256 FROM photos WHERE project_id=? AND id=?",
        )
        .bind(project)
        .bind(photo)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "照片不属于项目"))?;
        if previous.get::<String, _>("content_sha256") != prepared.hash {
            return Err(AppError::new("HASH_MISMATCH", "所选文件与原图指纹不同"));
        }
        if sqlx::query("UPDATE projects SET revision=revision+1,updated_at=? WHERE id=? AND revision=? AND deleted_at IS NULL").bind(domain::timestamp()).bind(project).bind(expected).execute(&mut *tx).await?.rows_affected()!=1 {return Err(AppError::new("REVISION_CONFLICT","项目已更新，请重新加载"));}
        sqlx::query("UPDATE photos SET original_path=?,thumbnail_path=?,preview_path=? WHERE project_id=? AND id=?").bind(prepared.path.to_string_lossy().as_ref()).bind(prepared.thumbnail.to_string_lossy().as_ref()).bind(prepared.preview.to_string_lossy().as_ref()).bind(project).bind(photo).execute(&mut *tx).await?;
        internal_event(
            &mut tx,
            project,
            Some(photo),
            "relink",
            expected,
            &json!({"originalPath":previous.get::<String,_>("original_path")}),
            &json!({"originalPath":prepared.path}),
            "gui_direct",
        )
        .await?;
        let result = db::snapshot_tx(&mut tx, project, Some(false)).await?;
        tx.commit().await?;
        Ok(result)
    }
    /// 原图被外部就地改写后，把这一条记录换成文件现在真正的内容。
    /// 照片 id、选片状态、序列位置、备注与评分都不动——换掉的只有指纹、尺寸和派生缓存；
    /// 之前对旧内容做过的 AI 分析因为观察行按内容指纹关联，会自动不再命中这张图。
    pub async fn apply_photo_change(
        &self,
        project: &str,
        photo: &str,
        prepared: &PreparedPhoto,
        expected: i64,
    ) -> Result<Project> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let previous = sqlx::query(
            "SELECT original_path,content_sha256,width,height,file_size FROM photos WHERE project_id=? AND id=?",
        )
        .bind(project)
        .bind(photo)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "照片不属于项目"))?;
        let previous_hash: String = previous.get("content_sha256");
        let previous_width: i64 = previous.get("width");
        let previous_height: i64 = previous.get("height");
        let previous_size: i64 = previous.get("file_size");
        if previous_hash == prepared.hash {
            return Err(AppError::new(
                "CONFLICT",
                "这条记录已经是文件现在的内容，无需更新",
            ));
        }
        // 更新的内容只能来自这张照片自己的原文件：换一条路径是"重新定位"，不是"内容更新"。
        let stored = std::path::PathBuf::from(previous.get::<String, _>("original_path"));
        let current = stored
            .canonicalize()
            .map_err(|_| AppError::new("ORIGINAL_MISSING", "原图已失联，请先重新定位这张照片"))?;
        if current != prepared.path {
            return Err(AppError::new("CONFLICT", "所选文件不是这张照片的原图"));
        }
        let now = domain::timestamp();
        if sqlx::query("UPDATE projects SET revision=revision+1,updated_at=? WHERE id=? AND revision=? AND deleted_at IS NULL").bind(&now).bind(project).bind(expected).execute(&mut *tx).await?.rows_affected()!=1 {return Err(AppError::new("REVISION_CONFLICT","项目已更新，请重新加载"));}
        sqlx::query("UPDATE photos SET content_sha256=?,file_size=?,width=?,height=?,thumbnail_path=?,preview_path=? WHERE project_id=? AND id=?")
            .bind(&prepared.hash)
            .bind(prepared.size as i64)
            .bind(prepared.width as i64)
            .bind(prepared.height as i64)
            .bind(prepared.thumbnail.to_string_lossy().as_ref())
            .bind(prepared.preview.to_string_lossy().as_ref())
            .bind(project)
            .bind(photo)
            .execute(&mut *tx)
            .await?;
        resync_asset(&mut tx, project, photo, prepared, &now).await?;
        internal_event(
            &mut tx,
            project,
            Some(photo),
            "content_apply",
            expected,
            &json!({"sha256":previous_hash,"width":previous_width,"height":previous_height,"fileSize":previous_size}),
            &json!({"sha256":prepared.hash,"width":prepared.width,"height":prepared.height,"fileSize":prepared.size,"path":prepared.path}),
            "gui_direct",
        )
        .await?;
        let result = db::snapshot_tx(&mut tx, project, Some(false)).await?;
        tx.commit().await?;
        Ok(result)
    }

    /// 台账里每条判断的当前值，加上这条记录在册的原图路径——
    /// 边车位置由这条路径推出来，界面递不进任何路径。
    pub async fn sidecar_rows(&self, project: &str) -> Result<Vec<PhotoJudgement>> {
        let rows = sqlx::query(
            "SELECT p.id,p.filename,p.original_path,d.status,d.rating FROM photos p JOIN photo_decisions d ON d.project_id=p.project_id AND d.photo_id=p.id JOIN projects pr ON pr.id=p.project_id AND pr.deleted_at IS NULL WHERE p.project_id=? ORDER BY p.import_index",
        )
        .bind(project)
        .fetch_all(&self.store.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|row| PhotoJudgement {
                photo_id: row.get("id"),
                filename: row.get("filename"),
                original_path: row.get("original_path"),
                status: row.get("status"),
                rating: row.get("rating"),
            })
            .collect())
    }

    /// 确认回填：把这条记录自己的 XMP 边车里的评分与选片落到本地判断上。
    /// 边车永远只读，落不落库由人点头；换评分不换照片身份，也不新增任何历史档案。
    pub async fn apply_sidecar_judgement(
        &self,
        project: &str,
        photo: &str,
        expected: i64,
    ) -> Result<Project> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let row = sqlx::query(
            "SELECT p.original_path,d.status,d.rating FROM photos p JOIN photo_decisions d ON d.project_id=p.project_id AND d.photo_id=p.id WHERE p.project_id=? AND p.id=?",
        )
        .bind(project)
        .bind(photo)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "照片不属于项目"))?;
        let original: String = row.get("original_path");
        let local_status: String = row.get("status");
        let local_rating: i64 = row.get("rating");
        let source = sidecar::read_sidecar(std::path::Path::new(&original))?
            .ok_or_else(|| AppError::new("SIDECAR_MISSING", "这张照片旁边没有 XMP 边车文件"))?;
        let judgement = &source.judgement;
        if judgement.ambiguous_rating {
            return Err(AppError::new(
                "CONFLICT",
                "边车里有两处不一致的评分，请先在外部软件里统一",
            ));
        }
        let next_rating = judgement.rating.filter(|rating| *rating != local_rating);
        // 没选中不等于淘汰：本地可能是人工标过拒绝的，边车那半边不足以推翻。
        let next_status: Option<&str> = if judgement.pick == Some(true) && local_status != "keep" {
            Some("keep")
        } else {
            None
        };
        if next_rating.is_none() && next_status.is_none() {
            return Err(AppError::new(
                "CONFLICT",
                "本地判断已经和这张照片的边车一致，无需回填",
            ));
        }
        let final_rating = next_rating.unwrap_or(local_rating);
        let final_status = next_status.unwrap_or(local_status.as_str());
        let now = domain::timestamp();
        if sqlx::query("UPDATE projects SET revision=revision+1,updated_at=? WHERE id=? AND revision=? AND deleted_at IS NULL").bind(&now).bind(project).bind(expected).execute(&mut *tx).await?.rows_affected()!=1 {return Err(AppError::new("REVISION_CONFLICT","项目已更新，请重新加载"));}
        sqlx::query("UPDATE photo_decisions SET rating=?,status=?,updated_at=? WHERE project_id=? AND photo_id=?")
            .bind(final_rating)
            .bind(final_status)
            .bind(&now)
            .bind(project)
            .bind(photo)
            .execute(&mut *tx)
            .await?;
        if next_status.is_some() {
            let mut sequence = db::sequence(&mut tx, project).await?;
            if !sequence.iter().any(|id| id == photo) {
                sequence.push(photo.to_owned());
                db::write_sequence(&mut tx, project, &sequence).await?;
            }
        }
        internal_event(
            &mut tx,
            project,
            Some(photo),
            "sidecar_apply",
            expected,
            &json!({"rating":local_rating,"status":local_status,"sidecarPath":source.path}),
            &json!({"rating":final_rating,"status":final_status,"colorLabel":judgement.color_label,"pick":judgement.pick}),
            "gui_direct",
        )
        .await?;
        let result = db::snapshot_tx(&mut tx, project, Some(false)).await?;
        tx.commit().await?;
        Ok(result)
    }

    /// 一键回填：整个项目里"本地还空着、外部软件已经写着"的判断一次落库。
    /// 读盘在开事务之前做完（逐张照片找边车是磁盘活），写入时把扫描看到的本地值当条件——
    /// 这中间人自己评过分、判过保留淘汰的，条件不成立就跳过，绝不拿刚读到的边车去盖他的判断。
    pub async fn backfill_sidecar_judgements(
        &self,
        project: &str,
        expected: i64,
    ) -> Result<sidecar::SidecarBatchOutcome> {
        let rows = self.sidecar_rows(project).await?;
        let plan: Vec<(String, i64, String, i64, String)> =
            tokio::task::spawn_blocking(move || {
                crate::sidecar::proposals_for(&rows).map(|found| {
                    found
                        .iter()
                        .filter_map(|item| {
                            item.batch_fill().map(|(rating, status)| {
                                (
                                    item.photo_id.clone(),
                                    rating,
                                    status.to_owned(),
                                    item.local_rating,
                                    item.local_status.clone(),
                                )
                            })
                        })
                        .collect::<Vec<_>>()
                })
            })
            .await
            .map_err(|error| AppError::new("BACKGROUND", error.to_string()))??;
        if plan.is_empty() {
            return Err(AppError::new(
                "CONFLICT",
                "没有可以一键回填的判断：外部软件写着的那些，本地要么已经有了，要么得逐条确认",
            ));
        }
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let now = domain::timestamp();
        if sqlx::query("UPDATE projects SET revision=revision+1,updated_at=? WHERE id=? AND revision=? AND deleted_at IS NULL").bind(&now).bind(project).bind(expected).execute(&mut *tx).await?.rows_affected()!=1 {return Err(AppError::new("REVISION_CONFLICT","项目已更新，请重新加载"));}
        let mut applied: Vec<String> = Vec::new();
        let mut newly_kept: Vec<String> = Vec::new();
        let mut skipped: i64 = 0;
        for (photo, rating, status, seen_rating, seen_status) in plan {
            let changed = sqlx::query("UPDATE photo_decisions SET rating=?,status=?,updated_at=? WHERE project_id=? AND photo_id=? AND rating=? AND status=?")
                .bind(rating)
                .bind(&status)
                .bind(&now)
                .bind(project)
                .bind(&photo)
                .bind(seen_rating)
                .bind(&seen_status)
                .execute(&mut *tx)
                .await?
                .rows_affected();
            if changed != 1 {
                skipped += 1;
                continue;
            }
            if status == "keep" && seen_status != "keep" {
                newly_kept.push(photo.clone());
            }
            applied.push(photo);
        }
        if applied.is_empty() {
            // 一条都没落成：不占修订号、不留空事件，事务随这次返回一起回滚。
            return Err(AppError::new(
                "CONFLICT",
                "这些判断在你点一键回填之前已经被你自己改过了，一条都没有动",
            ));
        }
        if !newly_kept.is_empty() {
            // 变成保留的那些要进序列，走的和人工点保留是同一条路。
            let mut sequence = db::sequence(&mut tx, project).await?;
            let before = sequence.len();
            for photo in &newly_kept {
                if !sequence.iter().any(|id| id == photo) {
                    sequence.push(photo.to_owned());
                }
            }
            if sequence.len() != before {
                db::write_sequence(&mut tx, project, &sequence).await?;
            }
        }
        let listed: Vec<&str> = applied.iter().take(50).map(String::as_str).collect();
        internal_event(
            &mut tx,
            project,
            None,
            "sidecar_apply",
            expected,
            &json!({"mode": "batch"}),
            &json!({"mode":"batch","applied":applied.len(),"skipped":skipped,"photoIds":listed,"photoIdsTruncated":applied.len()>50}),
            "gui_direct",
        )
        .await?;
        let result = db::snapshot_tx(&mut tx, project, Some(false)).await?;
        tx.commit().await?;
        Ok(sidecar::SidecarBatchOutcome {
            applied: applied.len() as i64,
            skipped,
            project: result,
        })
    }
}

/// 母版资产跟着内容走，但绝不替别人改历史：
/// 项目里已经有这份新内容的母版就挂过去；旧母版只有这一张照片在用就地换掉；
/// 旧母版还有别人在用（同内容不同来源）就新立一份，别的那条记录不受牵连。
async fn resync_asset(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    photo: &str,
    prepared: &PreparedPhoto,
    now: &str,
) -> Result<()> {
    let aliased: Option<String> =
        sqlx::query_scalar("SELECT asset_id FROM asset_aliases WHERE photo_id=?")
            .bind(photo)
            .fetch_optional(&mut **tx)
            .await?;
    let same_content: Option<String> =
        sqlx::query_scalar("SELECT id FROM source_assets WHERE project_id=? AND content_sha256=?")
            .bind(project)
            .bind(&prepared.hash)
            .fetch_optional(&mut **tx)
            .await?;
    if let Some(existing) = same_content {
        if aliased.as_deref() != Some(existing.as_str()) {
            sqlx::query("UPDATE asset_aliases SET asset_id=? WHERE photo_id=?")
                .bind(&existing)
                .bind(photo)
                .execute(&mut **tx)
                .await?;
        }
        return Ok(());
    }
    let shared = match &aliased {
        Some(asset) => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM asset_aliases WHERE asset_id=? AND photo_id<>?",
            )
            .bind(asset)
            .bind(photo)
            .fetch_one(&mut **tx)
            .await?
        }
        None => 0,
    };
    let locator = format!(
        "[{{\"path\":{path},\"kind\":\"file\"}}]",
        path = serde_json::json!(prepared.path.to_string_lossy().as_ref())
    );
    let format = prepared
        .path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let format = if format.len() > 10 {
        "unknown".to_owned()
    } else {
        format
    };
    let profile_digest = match crate::color_engine::read_source_profile(&prepared.path)
        .ok()
        .flatten()
    {
        Some(profile) if profile.valid => Some(profile.digest.clone()),
        Some(profile) => Some(format!("invalid:{}", profile.diagnostic.as_str())),
        None => None,
    };
    let asset_id = match aliased.clone().filter(|_| shared == 0) {
        Some(asset) => {
            sqlx::query("UPDATE source_assets SET content_sha256=?,original_locator=?,file_size=?,width=?,height=?,format=?,source_profile_digest=?,source_hash_verified_at=? WHERE id=? AND project_id=?")
                .bind(&prepared.hash)
                .bind(&locator)
                .bind(prepared.size as i64)
                .bind(prepared.width as i64)
                .bind(prepared.height as i64)
                .bind(&format)
                .bind(&profile_digest)
                .bind(now)
                .bind(&asset)
                .bind(project)
                .execute(&mut **tx)
                .await?;
            asset
        }
        None => {
            let fresh = domain::id();
            sqlx::query("INSERT INTO source_assets(id,project_id,content_sha256,original_locator,file_size,width,height,format,source_profile_digest,source_hash_verified_at,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)")
                .bind(&fresh)
                .bind(project)
                .bind(&prepared.hash)
                .bind(&locator)
                .bind(prepared.size as i64)
                .bind(prepared.width as i64)
                .bind(prepared.height as i64)
                .bind(&format)
                .bind(&profile_digest)
                .bind(now)
                .bind(now)
                .execute(&mut **tx)
                .await?;
            sqlx::query("INSERT INTO edit_variants(id,asset_id,name,external_software,external_variant_id,is_current,created_at) VALUES(?,?,?,NULL,NULL,1,?)")
                .bind(domain::id())
                .bind(&fresh)
                .bind("original")
                .bind(now)
                .execute(&mut **tx)
                .await?;
            fresh
        }
    };
    // 派生缓存按内容指纹命名，新内容必然落在新键上；这里只把资产表达指到真正存在的那两份。
    for (kind, path) in [
        ("thumbnail", &prepared.thumbnail),
        ("display_proxy", &prepared.preview),
    ] {
        let file_size = std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);
        let (width, height) = image::image_dimensions(path)
            .map(|(w, h)| (w as i64, h as i64))
            .unwrap_or((prepared.width as i64, prepared.height as i64));
        let updated = sqlx::query("UPDATE asset_representations SET width=?,height=?,file_path=?,file_size=?,created_at=? WHERE asset_id=? AND kind=?")
            .bind(width)
            .bind(height)
            .bind(path.to_string_lossy().as_ref())
            .bind(file_size)
            .bind(now)
            .bind(&asset_id)
            .bind(kind)
            .execute(&mut **tx)
            .await?
            .rows_affected();
        if updated == 0 {
            sqlx::query("INSERT INTO asset_representations(id,asset_id,kind,algorithm,width,height,file_path,file_size,profile_digest,created_at) VALUES(?,?,?,?,?,?,?,?,NULL,?)")
                .bind(domain::id())
                .bind(&asset_id)
                .bind(kind)
                .bind("image_pipeline_v1")
                .bind(width)
                .bind(height)
                .bind(path.to_string_lossy().as_ref())
                .bind(file_size)
                .bind(now)
                .execute(&mut **tx)
                .await?;
        }
    }
    // 这张照片的别名要落在真正描述新内容的那份母版上。
    let linked =
        sqlx::query("UPDATE asset_aliases SET asset_id=? WHERE photo_id=? AND asset_id<>?")
            .bind(&asset_id)
            .bind(photo)
            .bind(&asset_id)
            .execute(&mut **tx)
            .await?
            .rows_affected();
    if linked == 0 && aliased.is_none() {
        sqlx::query("INSERT INTO asset_aliases(photo_id,asset_id) VALUES(?,?)")
            .bind(photo)
            .bind(&asset_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
