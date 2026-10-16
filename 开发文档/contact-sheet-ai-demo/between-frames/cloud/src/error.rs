//! 对外错误不包含数据库语句、磁盘路径或凭证。
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub(crate) struct AppError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl AppError {
    pub(crate) fn auth(message: &str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", message)
    }
    pub(crate) fn forbidden(message: &str) -> Self {
        Self::new(StatusCode::FORBIDDEN, "PERMISSION_DENIED", message)
    }
    pub(crate) fn conflict(message: &str) -> Self {
        Self::new(
            axum::http::StatusCode::CONFLICT,
            "REVISION_CONFLICT",
            message,
        )
    }
    pub(crate) fn not_found(message: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", message)
    }
    pub(crate) fn validation(message: &str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", message)
    }
    pub(crate) fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL",
            "服务暂时不可用",
        )
    }
    pub(crate) fn busy() -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "AUTH_BUSY",
            "认证繁忙，请稍后重试",
        )
    }
    fn new(status: StatusCode, code: &'static str, message: &str) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({"error": {"code": self.code, "message": self.message}})),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        if error
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation())
        {
            return Self::new(StatusCode::CONFLICT, "CONFLICT", "资源已存在");
        }
        Self::internal()
    }
}
