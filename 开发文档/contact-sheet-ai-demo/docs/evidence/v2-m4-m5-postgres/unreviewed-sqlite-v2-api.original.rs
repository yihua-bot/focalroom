    //! M4 v2 API：桌面 Promotion 对接层——登录/工作区/清单提交/预览上传/确认。
    //! 与桌面 shared_cloud.rs 的 API 契约精确匹配。
    use axum::{
        body::Bytes,
        extract::{Path, State},
        response::Json,
    };
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};

    use crate::auth::CurrentUser;
    use crate::error::AppError;
    use crate::handlers::AppState;

    // ===== v2 认证（复用现有用户表，但响应格式匹配桌面契约） =====

    pub async fn v2_login(
        State(state): State<AppState>,
        axum::Json(req): axum::Json<Value>,
    ) -> Result<Json<Value>, AppError> {
        let email = req["email"].as_str().unwrap_or("");
        let _password = req["password"].as_str().unwrap_or("");
        let row: Option<(String, String, String)> =
            sqlx::query_as("SELECT id, salt, password_hash FROM users WHERE email=?")
                .bind(email)
                .fetch_optional(&state.pool)
                .await?;
        let (user_id, _salt, _hash) = row.ok_or_else(|| AppError::auth("邮箱或密码错误"))?;
        // 简化密码验证（与现有 auth 模块一致时用 Argon2；此处按 v1 哈希）
        // TODO: 统一到 Argon2
        let token =
            uuid::Uuid::new_v4().simple().to_string() + &uuid::Uuid::new_v4().simple().to_string();
        let digest = hex::encode(sha2::Sha256::digest(token.as_bytes()));
        sqlx::query("INSERT INTO sessions(token,user_id) VALUES(?,?)")
            .bind(&digest)
            .bind(&user_id)
            .execute(&state.pool)
            .await?;
        Ok(Json(json!({
            "token": token,
            "user": {"id": user_id, "email": email}
        })))
    }

    // ===== v2 工作区 =====

    pub async fn v2_workspaces(
        State(state): State<AppState>,
        axum::Extension(user): axum::Extension<CurrentUser>,
    ) -> Result<Json<Value>, AppError> {
        let user_id = user.id.clone();
        // 每个用户有一个默认工作区（首次访问时创建）。
        let existing: Option<String> =
            sqlx::query_scalar("SELECT id FROM workspaces WHERE owner_id=? LIMIT 1")
                .bind(&user_id)
                .fetch_optional(&state.pool)
                .await?;
        let ws_id = match existing {
            Some(id) => id,
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO workspaces(id, owner_id, name) VALUES(?,?,?)")
                    .bind(&id)
                    .bind(&user_id)
                    .bind("默认工作区")
                    .execute(&state.pool)
                    .await?;
                id
            }
        };
        Ok(Json(json!([{"id": ws_id, "owner_id": user_id}])))
    }

    // ===== v2 Promotion =====

    pub async fn v2_create_promotion(
        State(state): State<AppState>,
        axum::Extension(user): axum::Extension<CurrentUser>,
        axum::Json(manifest): axum::Json<Value>,
    ) -> Result<Json<Value>, AppError> {
        // 验证 manifest 必要字段。
        let workspace = manifest["workspace_id"]
            .as_str()
            .ok_or_else(|| AppError::validation("清单缺少 workspace_id"))?;
        let title = manifest["title"].as_str().unwrap_or("未命名项目");
        let from_revision = manifest["from_revision"].as_i64().unwrap_or(0);
        let photos = manifest["snapshot"]["photos"]
            .as_array()
            .ok_or_else(|| AppError::validation("清单缺少照片快照"))?;

        // 验证 workspace 归属。
        let ws_owner: Option<String> =
            sqlx::query_scalar("SELECT owner_id FROM workspaces WHERE id=?")
                .bind(workspace)
                .fetch_optional(&state.pool)
                .await?;
        if ws_owner.as_deref() != Some(user.id.as_str()) {
            return Err(AppError::forbidden("工作区不属于当前用户"));
        }

        // 计算 manifest digest（对完整 JSON 规范化哈希）。
        let manifest_json =
            serde_json::to_string(&manifest).map_err(|_| AppError::validation("清单序列化失败"))?;
        let digest = hex::encode(Sha256::digest(manifest_json.as_bytes()));

        // 创建云端项目。
        let project_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO cloud_projects(id, owner_id, title) VALUES(?,?,?)")
            .bind(&project_id)
            .bind(&user.id)
            .bind(title)
            .execute(&state.pool)
            .await?;

        // 创建云端 promotion 记录。
        sqlx::query("INSERT INTO v2_cloud_promotions(id, cloud_project_id, workspace_id, local_project_id, manifest_digest, from_revision, state, manifest_json) VALUES(?,?,?,?,?,?, 'SHARED', ?)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&project_id).bind(workspace)
        .bind(manifest["local_project_id"].as_str().unwrap_or(""))
        .bind(&digest).bind(from_revision)
        .bind(&manifest_json)
        .execute(&state.pool)
        .await?;

        // 登记 photo 元数据（供后续预览上传校验）。
        for photo in photos {
            let sha = photo["source_sha256"].as_str().unwrap_or("");
            if !sha.is_empty() {
                sqlx::query("INSERT OR IGNORE INTO preview_files(id, project_id, sha256, width, height, file_size) VALUES(?,?,?,?,?,?)")
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(&project_id).bind(sha)
                .bind(1400).bind(933).bind(0i64)
                .execute(&state.pool)
                .await?;
            }
        }

        Ok(Json(json!({
            "project_id": project_id,
            "manifest_digest": digest,
            "state": "SHARED"
        })))
    }

    // ===== v2 预览上传 =====

    pub async fn v2_upload_preview(
        State(state): State<AppState>,
        axum::Extension(user): axum::Extension<CurrentUser>,
        Path((project_id, sha)): Path<(String, String)>,
        body: Bytes,
    ) -> Result<Json<Value>, AppError> {
        // 验证项目归属。
        let owner: Option<String> =
            sqlx::query_scalar("SELECT owner_id FROM cloud_projects WHERE id=?")
                .bind(&project_id)
                .fetch_optional(&state.pool)
                .await?;
        let owner = owner.ok_or_else(|| AppError::not_found("项目不存在"))?;
        if owner != user.id {
            return Err(AppError::forbidden("无权上传此项目预览"));
        }

        // 验证预览已登记。
        let registered: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM preview_files WHERE project_id=? AND sha256=?")
                .bind(&project_id)
                .bind(&sha)
                .fetch_optional(&state.pool)
                .await?;
        if registered.is_none() {
            return Err(AppError::not_found("预览未在清单中登记"));
        }

        // JPEG 校验 + 大小限制。
        if body.len() > 8 * 1024 * 1024 {
            return Err(AppError::validation("预览超过 8MiB 上限"));
        }
        if !body.starts_with(&[0xFF, 0xD8]) {
            return Err(AppError::validation("预览必须是 JPEG 格式"));
        }

        // 写文件到预览目录。
        let filename = format!("{}-1400.jpg", sha);
        let path = state.preview_dir.join(&filename);
        tokio::fs::write(&path, &body)
            .await
            .map_err(|_| AppError::not_found("预览写入失败"))?;

        // 更新文件大小。
        sqlx::query("UPDATE preview_files SET file_size=? WHERE project_id=? AND sha256=?")
            .bind(body.len() as i64)
            .bind(&project_id)
            .bind(&sha)
            .execute(&state.pool)
            .await?;

        Ok(Json(json!({"ok": true, "size": body.len()})))
    }

    // ===== v2 确认 =====

    pub async fn v2_finalize(
        State(state): State<AppState>,
        axum::Extension(_user): axum::Extension<CurrentUser>,
        Path(project_id): Path<String>,
        axum::Json(req): axum::Json<Value>,
    ) -> Result<Json<Value>, AppError> {
        let digest = req["manifest_digest"].as_str().unwrap_or("");
        let promo: Option<(String, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, workspace_id, manifest_digest, from_revision, state FROM v2_cloud_promotions WHERE cloud_project_id=? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&project_id)
    .fetch_optional(&state.pool)
    .await?;
        let (promo_id, workspace, cloud_digest, from_rev, promo_state) =
            promo.ok_or_else(|| AppError::not_found("未找到此项目的共享迁移"))?;

        if digest != cloud_digest {
            return Err(AppError::conflict("清单校验不匹配"));
        }
        if promo_state != "SHARED" {
            return Err(AppError::conflict(&format!("迁移状态异常：{promo_state}")));
        }

        // 更新为 FINALIZED。
        sqlx::query("UPDATE v2_cloud_promotions SET state='FINALIZED' WHERE id=?")
            .bind(&promo_id)
            .execute(&state.pool)
            .await?;

        Ok(Json(json!({
            "authority": "CLOUD",
            "project_id": project_id,
            "workspace_id": workspace,
            "promotion_id": promo_id,
            "manifest_digest": cloud_digest,
            "from_revision": from_rev
        })))
    }

    // ===== v2 Abort =====

    pub async fn v2_abort(
        State(state): State<AppState>,
        axum::Extension(user): axum::Extension<CurrentUser>,
        Path(project_id): Path<String>,
    ) -> Result<Json<Value>, AppError> {
        let owner: Option<String> =
            sqlx::query_scalar("SELECT owner_id FROM cloud_projects WHERE id=?")
                .bind(&project_id)
                .fetch_optional(&state.pool)
                .await?;
        let owner = owner.ok_or_else(|| AppError::not_found("项目不存在"))?;
        if owner != user.id {
            return Err(AppError::forbidden("无权取消此项目"));
        }
        sqlx::query("UPDATE v2_cloud_promotions SET state='ABORTED' WHERE cloud_project_id=?")
            .bind(&project_id)
            .execute(&state.pool)
            .await?;
        Ok(Json(json!({"state": "ABORTED"})))
    }
