use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub retriable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<i64>,
}
impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retriable: code == "REVISION_CONFLICT" || code == "DATABASE",
            current_revision: None,
        }
    }
}
impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::new("DATABASE", format!("本地数据库操作失败：{error}"))
    }
}
impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::new("FILE_IO", format!("文件操作失败：{error}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Photo {
    pub id: String,
    #[sqlx(rename = "import_index")]
    pub index: i64,
    pub name: String,
    pub original_path: String,
    pub thumbnail_path: String,
    pub preview_path: String,
    pub content_sha256: String,
    pub width: i64,
    pub height: i64,
    pub file_size: i64,
    pub status: String,
    pub note: String,
    pub rating: i64,
    #[sqlx(skip)]
    pub is_missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DecisionEvent {
    pub id: String,
    pub photo_id: Option<String>,
    pub actor: String,
    #[sqlx(rename = "event_type")]
    pub kind: String,
    pub previous_json: Option<String>,
    pub next_json: Option<String>,
    pub reason: Option<String>,
    pub context_json: String,
    #[sqlx(rename = "created_at")]
    pub at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub archived: bool,
    pub title: String,
    pub brief: String,
    pub revision: i64,
    pub updated_at: String,
    pub photos: Vec<Photo>,
    pub sequence: Vec<String>,
    pub events: Vec<DecisionEvent>,
    pub history: HistoryState,
    pub versions: Vec<SequenceVersion>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryState {
    pub undo_group_id: Option<String>,
    pub redo_group_id: Option<String>,
    pub undo_count: i64,
    pub redo_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PhotoDecisionState {
    pub photo_id: String,
    pub status: String,
    pub rating: i64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceVersion {
    pub id: String,
    pub label: String,
    pub based_on_revision: i64,
    pub created_at: String,
    pub photo_ids: Vec<String>,
    pub photos: Vec<PhotoDecisionState>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportFailure {
    pub path: String,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportJob {
    pub job_id: String,
    pub project_id: String,
    pub status: String,
    pub completed: usize,
    pub total: usize,
    pub imported: usize,
    pub skipped: usize,
    pub failures: Vec<ImportFailure>,
}

pub fn timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub fn json(value: &Value) -> String {
    value.to_string()
}
