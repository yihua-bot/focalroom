//! 显式桌面 Promotion：冻结本地 → 受控预览 → 云提交 → 本地只读。
//! 不确定的网络结果保留冻结，重试使用相同迁移键；从不因超时重新启用本地写入。
use crate::{
    agent::promotion,
    db::Store,
    domain::{AppError, Result},
    shared_export,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShareRequest {
    pub project_id: String,
    pub expected_revision: i64,
    pub base_url: String,
    pub web_url: String,
    pub email: String,
    pub password: String,
    pub action: String,
    pub preview_upload_authorized: bool,
}
fn endpoint(value: &str) -> Result<String> {
    let parsed = reqwest::Url::parse(value)
        .map_err(|_| AppError::new("CONFIG_INVALID", "共享服务地址不合法"))?;
    let local = matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if !(parsed.scheme() == "https" || (local && parsed.scheme() == "http"))
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.path() != "/"
    {
        return Err(AppError::new(
            "CONFIG_INVALID",
            "服务地址须为 HTTPS 来源（本机开发允许 HTTP），不得含凭证或路径",
        ));
    }
    Ok(parsed.origin().ascii_serialization())
}
async fn response(mut response: reqwest::Response) -> Result<Value> {
    let status = response.status();
    if response.content_length().unwrap_or(0) > 2 * 1024 * 1024 {
        return Err(AppError::new("CLOUD_PROTOCOL", "云端回包超过上限"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err(AppError::new("CLOUD_PROTOCOL", "云端回包超过上限"));
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::new("CLOUD_PROTOCOL", "云端协议错误；迁移状态保留"))?;
    if !status.is_success() {
        let code = value["error"]["code"].as_str().unwrap_or("CLOUD_REJECTED");
        return Err(AppError::new(
            code,
            "共享服务拒绝请求；请检查账户、权限或网络后重试，迁移冻结不会自动解除",
        ));
    }
    Ok(value)
}
fn network(_: reqwest::Error) -> AppError {
    AppError::new(
        "CLOUD_UNAVAILABLE",
        "共享服务连接失败；已有迁移保留冻结，请恢复连接后继续或显式取消",
    )
}
async fn transition(store: &Store, id: &str, target: &str, receipt: Option<&str>) -> Result<()> {
    let state: String = sqlx::query_scalar("SELECT state FROM v2_promotions WHERE id=?")
        .bind(id)
        .fetch_one(&store.pool)
        .await?;
    if state != target {
        promotion::advance(store, id, target, receipt).await?;
    }
    Ok(())
}
pub async fn status(store: &Store, project: &str) -> Result<Value> {
    let mut authority = promotion::authority_of(store, project).await?;
    let row = sqlx::query("SELECT id,state,detail_json FROM v2_promotions WHERE project_id=? AND state IN('pending','uploading','verifying','completed') ORDER BY created_at DESC LIMIT 1").bind(project).fetch_optional(&store.pool).await?;
    if let Some(row) = row {
        let detail: Value = serde_json::from_str(row.get("detail_json")).unwrap_or(json!({}));
        authority["promotionId"] = json!(row.get::<String, _>("id"));
        authority["state"] = json!(row.get::<String, _>("state"));
        authority["remote"] = detail["remote"].clone();
    }
    Ok(authority)
}
pub async fn review_url(store: &Store, project: &str) -> Result<String> {
    review_link(&status(store, project).await?)
}
fn review_link(state: &Value) -> Result<String> {
    if state["authority"] != "shared" {
        return Err(AppError::new("AUTHORITY_MISMATCH", "项目尚未完成共享迁移"));
    }
    let invalid = || AppError::new("SHARE_BINDING_CHANGED", "共享评片地址与已确认绑定不符");
    let origin = endpoint(
        state["remote"]["binding"]["webUrl"]
            .as_str()
            .ok_or_else(invalid)?,
    )?;
    let project = state["remote"]["projectId"].as_str().ok_or_else(invalid)?;
    uuid::Uuid::parse_str(project).map_err(|_| invalid())?;
    let url = format!("{origin}/shared#project={project}");
    if state["remote"]["webUrl"].as_str() != Some(url.as_str()) {
        return Err(invalid());
    }
    Ok(url)
}

pub async fn share(store: &Store, request: ShareRequest) -> Result<Value> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _lock = LOCK.lock().await;
    if !request.preview_upload_authorized || !matches!(request.action.as_str(), "resume" | "abort")
    {
        return Err(AppError::new(
            "CONSENT_REQUIRED",
            "须明确授权派生预览共享，并选择继续或取消迁移",
        ));
    }
    let base = endpoint(&request.base_url)?;
    let web = endpoint(&request.web_url)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| AppError::new("CLOUD_CONFIG", "HTTP 客户端创建失败"))?;
    let auth = response(
        client
            .post(format!("{base}/api/v2/auth/login"))
            .json(&json!({"email":request.email,"password":request.password}))
            .send()
            .await
            .map_err(network)?,
    )
    .await?;
    let token = auth["token"]
        .as_str()
        .filter(|s| s.len() == 64)
        .ok_or_else(|| AppError::new("CLOUD_PROTOCOL", "登录回包不完整"))?;
    let user = auth["user"]["id"]
        .as_str()
        .ok_or_else(|| AppError::new("CLOUD_PROTOCOL", "登录回包缺少身份"))?;
    let workspaces = response(
        client
            .get(format!("{base}/api/v2/workspaces"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(network)?,
    )
    .await?;
    let workspace = workspaces
        .as_array()
        .and_then(|list| list.iter().find(|w| w["owner_id"] == user))
        .and_then(|w| w["id"].as_str())
        .ok_or_else(|| AppError::new("PERMISSION_DENIED", "当前账户没有自己的云工作区"))?;
    let active=sqlx::query("SELECT id,state,from_revision,detail_json FROM v2_promotions WHERE project_id=? AND state IN('pending','uploading','verifying','completed') ORDER BY created_at DESC LIMIT 1").bind(&request.project_id).fetch_optional(&store.pool).await?;
    let (id, state, revision, detail) = if let Some(row) = active {
        (
            row.get::<String, _>("id"),
            row.get::<String, _>("state"),
            row.get::<i64, _>("from_revision"),
            serde_json::from_str::<Value>(row.get("detail_json")).unwrap_or(json!({})),
        )
    } else {
        if request.action == "abort" {
            return Err(AppError::new("INVALID_TRANSITION", "没有可取消的共享迁移"));
        }
        let start = promotion::begin(store, &request.project_id, user).await?;
        (
            start["promotion_id"].as_str().unwrap().to_string(),
            "pending".to_string(),
            start["from_revision"].as_i64().unwrap(),
            json!({}),
        )
    };
    if revision != request.expected_revision {
        if state == "pending" && detail["remote"].is_null() {
            transition(store, &id, "aborted", None).await?;
        }
        return Err(AppError::new(
            "REVISION_CONFLICT",
            "本地版本已变化，请重新打开共享窗口",
        ));
    }
    let binding = json!({"baseUrl":base,"webUrl":web,"userId":user,"workspaceId":workspace});
    if !detail["remote"].is_null() && detail["remote"]["binding"] != binding {
        return Err(AppError::new(
            "SHARE_BINDING_CHANGED",
            "迁移已绑定其他服务、账户或工作区；须用原配置恢复，不能静默转移素材",
        ));
    }
    if state == "completed" {
        return status(store, &request.project_id).await;
    }
    let project = store.get(&request.project_id).await?;
    let manifest = shared_export::manifest(&project, &id, workspace)?;
    sqlx::query("UPDATE v2_promotions SET detail_json=json_set(detail_json,'$.remote',json(?)),updated_at=? WHERE id=?").bind(json!({"binding":binding}).to_string()).bind(crate::domain::timestamp()).bind(&id).execute(&store.pool).await?;
    let start = response(
        client
            .post(format!("{base}/api/v2/promotions"))
            .bearer_auth(token)
            .json(&manifest)
            .send()
            .await
            .map_err(network)?,
    )
    .await?;
    let cloud = start["project_id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| AppError::new("CLOUD_PROTOCOL", "共享回包缺少项目"))?;
    let hash = start["manifest_digest"]
        .as_str()
        .filter(|h| h.len() == 64)
        .ok_or_else(|| AppError::new("CLOUD_PROTOCOL", "共享回包缺少清单校验"))?;
    let remote = json!({"binding":binding,"projectId":cloud,"manifestDigest":hash,"webUrl":format!("{web}/shared#project={cloud}")});
    sqlx::query("UPDATE v2_promotions SET detail_json=json_set(detail_json,'$.remote',json(?)),updated_at=? WHERE id=?").bind(remote.to_string()).bind(crate::domain::timestamp()).bind(&id).execute(&store.pool).await?;
    if request.action == "abort" {
        response(
            client
                .post(format!("{base}/api/v2/projects/{cloud}/abort"))
                .bearer_auth(token)
                .json(&json!({"manifest_digest":hash}))
                .send()
                .await
                .map_err(network)?,
        )
        .await?;
        transition(store, &id, "aborted", None).await?;
        return status(store, &request.project_id).await;
    }
    if state == "pending" {
        transition(store, &id, "uploading", None).await?;
    }
    if start["state"] != "SHARED" {
        if start["state"] != "STAGING" {
            return Err(AppError::new(
                "INVALID_TRANSITION",
                "云端迁移已取消，请显式取消本地迁移后重新开始",
            ));
        }
        for photo in &project.photos {
            let path = std::path::PathBuf::from(&photo.preview_path);
            let bytes =
                tokio::task::spawn_blocking(move || shared_export::preview_bytes(Path::new(&path)))
                    .await
                    .map_err(|_| AppError::new("BACKGROUND", "预览处理失败"))??;
            response(
                client
                    .put(format!(
                        "{base}/api/v2/projects/{cloud}/previews/{}",
                        photo.id
                    ))
                    .bearer_auth(token)
                    .header("content-type", "image/jpeg")
                    .body(bytes)
                    .send()
                    .await
                    .map_err(network)?,
            )
            .await?;
        }
    }
    // 必须先持久化 verifying，再发送可能已经提交但丢失响应的 finalize。
    transition(store, &id, "verifying", None).await?;
    let receipt = response(
        client
            .post(format!("{base}/api/v2/projects/{cloud}/finalize"))
            .bearer_auth(token)
            .json(&json!({"manifest_digest":hash}))
            .send()
            .await
            .map_err(network)?,
    )
    .await?;
    if receipt["authority"] != "CLOUD"
        || receipt["project_id"] != cloud
        || receipt["workspace_id"] != workspace
        || receipt["promotion_id"] != id
        || receipt["manifest_digest"] != hash
        || receipt["from_revision"] != revision
    {
        return Err(AppError::new(
            "CLOUD_PROTOCOL",
            "云端确认与冻结迁移不匹配；继续保留本地冻结",
        ));
    }
    transition(store, &id, "completed", Some(&receipt.to_string())).await?;
    status(store, &request.project_id).await
}

#[cfg(test)]
mod review_link_tests {
    use super::*;
    #[test]
    fn browser_link_requires_completed_bound_project_and_safe_origin() {
        let id = uuid::Uuid::new_v4().to_string();
        let url = format!("https://review.example/shared#project={id}");
        let mut state = json!({"authority":"shared","remote":{"binding":{"webUrl":"https://review.example"},"projectId":id,"webUrl":url}});
        assert_eq!(review_link(&state).unwrap(), url);
        state["authority"] = json!("local-promoting");
        assert!(review_link(&state).is_err());
        state["authority"] = json!("shared");
        state["remote"]["webUrl"] = json!("https://other.example/shared");
        assert!(review_link(&state).is_err());
        state["remote"]["binding"]["webUrl"] = json!("file:///private/secret");
        assert!(review_link(&state).is_err());
        state["remote"]["binding"]["webUrl"] = json!("https://review.example");
        state["remote"]["projectId"] = json!("../foreign?token=secret");
        assert!(review_link(&state).is_err());
    }
}
