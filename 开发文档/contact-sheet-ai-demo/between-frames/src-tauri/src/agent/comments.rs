// v2.0 M5 评论（审稿协作数据层）：锚定照片的讨论线程，本地权威。
// 语义：人工写入（经 v2 桥的人类通道），Agent 只读（get_comments 工具）；
// 评论是协作元数据——不占人工撤销栈、不作为偏好证据、不改正式选片。
use crate::{
    db::Store,
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentThread {
    pub id: String,
    pub project_id: String,
    pub anchor: Value,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    pub comments: Vec<CommentEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentEntry {
    pub id: i64,
    pub body: String,
    pub author: String,
    pub created_at: String,
}

async fn validate_anchor(
    anchor: &Value,
    project: &str,
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<()> {
    let kind = anchor["type"].as_str().unwrap_or("");
    match kind {
        // 照片锚点：照片必须真实存在于该项目（防跨项目引用）。
        "photo" => {
            let photo = anchor["photo_id"].as_str().unwrap_or("");
            if photo.is_empty() {
                return Err(AppError::new("VALIDATION_ERROR", "照片锚点缺少 photo_id"));
            }
            let owned: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM photos WHERE id=? AND project_id=?")
                    .bind(photo)
                    .bind(project)
                    .fetch_one(&mut **tx)
                    .await
                    .unwrap_or(0);
            if owned != 1 {
                return Err(AppError::new("PERMISSION_DENIED", "锚点照片不属于此项目"));
            }
            Ok(())
        }
        "project" => Ok(()),
        // 出版物锚点：线程只属于确实存在于该项目的那一本， revision 记的是提意见时那一版。
        "publication" => {
            let publication = anchor["publication_id"].as_str().unwrap_or("");
            if publication.is_empty() {
                return Err(AppError::new(
                    "VALIDATION_ERROR",
                    "出版物锚点缺少 publication_id",
                ));
            }
            if !matches!(anchor["revision"].as_i64(), Some(value) if (0..=100_000).contains(&value))
            {
                return Err(AppError::new(
                    "VALIDATION_ERROR",
                    "出版物锚点缺少合法的版本号",
                ));
            }
            let owned: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM publications WHERE id=? AND project_id=?")
                    .bind(publication)
                    .bind(project)
                    .fetch_one(&mut **tx)
                    .await
                    .unwrap_or(0);
            if owned != 1 {
                return Err(AppError::new("PERMISSION_DENIED", "锚点出版物不属于此项目"));
            }
            Ok(())
        }
        _ => Err(AppError::new(
            "VALIDATION_ERROR",
            "锚点类型须为 photo、project 或 publication",
        )),
    }
}

/// 发起或追加讨论：同锚点未关闭线程追加；否则新建线程。
pub(crate) async fn add_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    author: &str,
    body: &str,
    anchor: &Value,
) -> Result<CommentThread> {
    let text = body.trim();
    if text.is_empty() || text.chars().count() > 5000 {
        return Err(AppError::new("VALIDATION_ERROR", "评论正文须为 1–5000 字"));
    }
    if author.trim().is_empty() || author.len() > 160 {
        return Err(AppError::new("VALIDATION_ERROR", "作者标识不合法"));
    }
    validate_anchor(anchor, project_id, tx).await?;
    let now = domain::timestamp();
    let anchor_json = anchor.to_string();
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM comment_threads WHERE project_id=? AND anchor_json=? AND state='open' ORDER BY created_at LIMIT 1",
    )
    .bind(project_id)
    .bind(&anchor_json)
    .fetch_optional(&mut **tx)
    .await?;
    let thread_id = match existing {
        Some(id) => id,
        None => {
            let id = domain::id();
            sqlx::query("INSERT INTO comment_threads(id,project_id,anchor_json,state,created_at,updated_at) VALUES(?,?,?,'open',?,?)")
                .bind(&id)
                .bind(project_id)
                .bind(&anchor_json)
                .bind(&now)
                .bind(&now)
                .execute(&mut **tx)
                .await?;
            id
        }
    };
    sqlx::query("INSERT INTO comments(thread_id,body,author,created_at) VALUES(?,?,?,?)")
        .bind(&thread_id)
        .bind(text)
        .bind(author.trim())
        .bind(&now)
        .execute(&mut **tx)
        .await?;
    sqlx::query("UPDATE comment_threads SET updated_at=? WHERE id=?")
        .bind(&now)
        .bind(&thread_id)
        .execute(&mut **tx)
        .await?;
    thread_view_tx(tx, &thread_id).await
}

/// 关闭/重开线程；已关闭线程可重开继续讨论。
pub(crate) async fn resolve_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    thread_id: &str,
    resolved: bool,
) -> Result<CommentThread> {
    let owned = sqlx::query("SELECT id,state FROM comment_threads WHERE id=? AND project_id=?")
        .bind(thread_id)
        .bind(project_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "讨论线程不属于此项目"))?;
    let _ = owned;
    let next = if resolved { "resolved" } else { "open" };
    sqlx::query("UPDATE comment_threads SET state=?,updated_at=? WHERE id=?")
        .bind(next)
        .bind(domain::timestamp())
        .bind(thread_id)
        .execute(&mut **tx)
        .await?;
    thread_view_tx(tx, thread_id).await
}

async fn thread_view_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    thread_id: &str,
) -> Result<CommentThread> {
    let row = sqlx::query("SELECT * FROM comment_threads WHERE id=?")
        .bind(thread_id)
        .fetch_one(&mut **tx)
        .await?;
    let entries =
        sqlx::query("SELECT id,body,author,created_at FROM comments WHERE thread_id=? ORDER BY id")
            .bind(thread_id)
            .fetch_all(&mut **tx)
            .await?
            .iter()
            .map(|r| CommentEntry {
                id: r.get("id"),
                body: r.get("body"),
                author: r.get("author"),
                created_at: r.get("created_at"),
            })
            .collect();
    Ok(CommentThread {
        id: row.get("id"),
        project_id: row.get("project_id"),
        anchor: serde_json::from_str(&row.get::<String, _>("anchor_json")).unwrap_or(Value::Null),
        state: row.get("state"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        comments: entries,
    })
}

/// Agent/界面读取：按项目（可过滤照片锚点），open 在前、新在前。
pub async fn list(
    store: &Store,
    project_id: &str,
    photo_id: Option<&str>,
    limit: usize,
) -> Result<Value> {
    let mut tx = store.pool.begin().await?;
    let value = list_tx(&mut tx, project_id, photo_id, limit).await?;
    tx.commit().await?;
    Ok(value)
}
/// 事务内版本（Agent 工具经 PolicyGate 同事务读取）。
pub async fn list_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    photo_id: Option<&str>,
    limit: usize,
) -> Result<Value> {
    let rows: Vec<(String, String)> = sqlx::query(
        "SELECT id,anchor_json FROM comment_threads WHERE project_id=? AND (? IS NULL OR anchor_json LIKE ?) ORDER BY CASE state WHEN 'open' THEN 0 ELSE 1 END, updated_at DESC LIMIT ?",
    )
    .bind(project_id)
    .bind(photo_id)
    .bind(photo_id.map(|id| format!("%{id}%")))
    .bind(limit as i64)
    .fetch_all(&mut **tx)
    .await?
    .iter()
    .map(|r| (r.get("id"), r.get("anchor_json")))
    .collect();
    let mut threads = vec![];
    for (id, _) in rows {
        let view = thread_view_tx(tx, &id).await?;
        threads.push(json!({
            "id": view.id, "anchor": view.anchor, "state": view.state,
            "comments": view.comments, "updated_at": view.updated_at,
        }));
    }
    Ok(json!({"threads": threads, "notice": "评论为人工审稿记录；Agent 只读，不构成偏好证据"}))
}

/// 未解决的出版物审阅意见条数：定稿要跨过这些意见，必须先有人明确处理掉。
pub(crate) async fn open_count_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    publication_id: &str,
) -> Result<usize> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM comment_threads
         WHERE project_id=? AND state='open'
           AND json_extract(anchor_json,'$.type')='publication'
           AND json_extract(anchor_json,'$.publication_id')=?",
    )
    .bind(project_id)
    .bind(publication_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count.max(0) as usize)
}

fn publication_of(anchor: &Value) -> Option<String> {
    (anchor["type"] == "publication")
        .then(|| anchor["publication_id"].as_str().unwrap_or("").to_owned())
}

async fn review_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    publication_id: &str,
) -> Result<Value> {
    let revision: Option<i64> =
        sqlx::query_scalar("SELECT revision FROM publications WHERE id=? AND project_id=?")
            .bind(publication_id)
            .bind(project_id)
            .fetch_optional(&mut **tx)
            .await?;
    let current =
        revision.ok_or_else(|| AppError::new("NOT_FOUND", "出版物不存在或不属于此项目"))?;
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM comment_threads
         WHERE project_id=?
           AND json_extract(anchor_json,'$.type')='publication'
           AND json_extract(anchor_json,'$.publication_id')=?
         ORDER BY CASE state WHEN 'open' THEN 0 ELSE 1 END, updated_at DESC, id DESC
         LIMIT 60",
    )
    .bind(project_id)
    .bind(publication_id)
    .fetch_all(&mut **tx)
    .await?;
    let mut threads = Vec::with_capacity(ids.len());
    for id in &ids {
        let view = thread_view_tx(tx, id).await?;
        threads.push(json!({
            "id": view.id,
            "revision": view.anchor["revision"].as_i64(),
            "state": view.state,
            "comments": view.comments,
        }));
    }
    let open = threads
        .iter()
        .filter(|thread| thread["state"] == "open")
        .count();
    Ok(
        json!({"publicationId": publication_id, "revision": current, "open": open, "threads": threads}),
    )
}

/// 审稿面板读侧：只读通道，不带修订号、不进幂等队列。
pub async fn publication_review(
    store: &Store,
    project_id: &str,
    publication_id: &str,
) -> Result<Value> {
    let mut tx = store.pool.begin().await?;
    let value = review_tx(&mut tx, project_id, publication_id).await?;
    tx.commit().await?;
    Ok(value)
}

/// 审稿面板写侧：`add` 挂在当前那一版上，`resolve`/`reopen` 明确表态这条意见处理过了。
#[derive(Debug, Clone, Deserialize)]
#[serde(
    rename_all = "camelCase",
    deny_unknown_fields,
    tag = "action",
    content = "data"
)]
pub enum PublicationReviewRequest {
    Add { body: String },
    Resolve { thread_id: String, resolved: bool },
}

pub async fn publication_review_write(
    store: &Store,
    project_id: &str,
    publication_id: &str,
    request: PublicationReviewRequest,
) -> Result<Value> {
    // 桌面本地身份：与 v2 通道同一个受信主体，审阅意见因此带上可核对的作者标签。
    let author = betweenframes_local_domain::actor_label(&crate::v2_bridge::local_owner_principal(
        "local-workspace",
        project_id,
    ));
    let mut tx = store.pool.begin().await?;
    let view = match &request {
        PublicationReviewRequest::Add { body } => {
            let revision: Option<i64> =
                sqlx::query_scalar("SELECT revision FROM publications WHERE id=? AND project_id=?")
                    .bind(publication_id)
                    .bind(project_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            let current =
                revision.ok_or_else(|| AppError::new("NOT_FOUND", "出版物不存在或不属于此项目"))?;
            add_tx(
                &mut tx,
                project_id,
                &author,
                body,
                &json!({"type": "publication", "publication_id": publication_id, "revision": current}),
            )
            .await?
        }
        PublicationReviewRequest::Resolve {
            thread_id,
            resolved,
        } => resolve_tx(&mut tx, project_id, thread_id, *resolved).await?,
    };
    // 这条通道只管本出版物的意见：拿照片线程或别本的线程号来关，整条拒掉且不落库。
    if publication_of(&view.anchor).as_deref() != Some(publication_id) {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "这条讨论不属于当前出版物",
        ));
    }
    let value = review_tx(&mut tx, project_id, publication_id).await?;
    tx.commit().await?;
    Ok(value)
}
