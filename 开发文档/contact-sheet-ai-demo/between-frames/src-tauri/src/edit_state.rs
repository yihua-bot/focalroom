use crate::{
    db,
    domain::{self, AppError, PhotoDecisionState, Result},
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::HashSet;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditState {
    pub title: String,
    pub brief: String,
    pub archived_at: Option<String>,
    pub photos: Vec<PhotoDecisionState>,
    pub sequence: Vec<String>,
    pub comparisons: Vec<String>,
}
pub(crate) async fn capture(tx: &mut Transaction<'_, Sqlite>, project: &str) -> Result<EditState> {
    let row = sqlx::query("SELECT title,brief,deleted_at FROM projects WHERE id=?")
        .bind(project)
        .fetch_one(&mut **tx)
        .await?;
    let photos=sqlx::query_as("SELECT photo_id,status,rating,user_note AS note FROM photo_decisions WHERE project_id=? ORDER BY photo_id").bind(project).fetch_all(&mut **tx).await?;
    let comparisons=sqlx::query_scalar("SELECT c.id FROM comparison_events c LEFT JOIN comparison_state s ON s.comparison_id=c.id WHERE c.project_id=? AND COALESCE(s.active,1)=1 ORDER BY c.id").bind(project).fetch_all(&mut **tx).await?;
    Ok(EditState {
        title: row.get("title"),
        brief: row.get("brief"),
        archived_at: row.get("deleted_at"),
        photos,
        sequence: db::sequence(tx, project).await?,
        comparisons,
    })
}
pub(crate) fn compatible(current: &EditState, expected: &EditState) -> bool {
    current.title == expected.title
        && current.brief == expected.brief
        && current.archived_at == expected.archived_at
        && current.sequence == expected.sequence
        && current.comparisons == expected.comparisons
        && expected
            .photos
            .iter()
            .all(|photo| current.photos.iter().any(|item| item == photo))
}
pub(crate) async fn apply_state(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    state: &EditState,
) -> Result<()> {
    for photo in &state.photos {
        if sqlx::query("UPDATE photo_decisions SET status=?,rating=?,user_note=?,updated_at=? WHERE project_id=? AND photo_id=?").bind(&photo.status).bind(photo.rating).bind(&photo.note).bind(domain::timestamp()).bind(project).bind(&photo.photo_id).execute(&mut **tx).await?.rows_affected()!=1 {return Err(AppError::new("NOT_FOUND","快照包含其他项目或失效照片"));}
    }
    sqlx::query("UPDATE projects SET title=?,brief=?,deleted_at=? WHERE id=?")
        .bind(&state.title)
        .bind(&state.brief)
        .bind(&state.archived_at)
        .bind(project)
        .execute(&mut **tx)
        .await?;
    db::write_sequence(tx, project, &state.sequence).await?;
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM comparison_events WHERE project_id=?")
            .bind(project)
            .fetch_all(&mut **tx)
            .await?;
    for id in ids {
        sqlx::query("INSERT INTO comparison_state VALUES(?,?) ON CONFLICT(comparison_id) DO UPDATE SET active=excluded.active").bind(&id).bind(state.comparisons.contains(&id)).execute(&mut **tx).await?;
    }
    Ok(())
}
pub(crate) async fn validate_selection(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    photos: &[String],
) -> Result<()> {
    if photos.is_empty()
        || photos.len() > 1000
        || photos.iter().collect::<HashSet<_>>().len() != photos.len()
    {
        return Err(AppError::new(
            "INVALID_INPUT",
            "选区需包含 1–1000 张不同照片",
        ));
    }
    for photo in photos {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM photos WHERE project_id=? AND id=?")
                .bind(project)
                .bind(photo)
                .fetch_one(&mut **tx)
                .await?;
        if count != 1 {
            return Err(AppError::new("NOT_FOUND", "选区包含不属于该项目的照片"));
        }
    }
    Ok(())
}
