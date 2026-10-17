//! SQLite 实验服务不提供 v2 共享权威；实际实现位于 apps/api（NestJS/PostgreSQL）。
use crate::error::AppError;
use axum::response::Json;
use serde_json::Value;

pub(crate) async fn disabled() -> Result<Json<Value>, AppError> {
    Err(AppError::unsupported(
        "此 SQLite 实验服务不提供 v2 共享权威；请配置 FocalRoom PostgreSQL API",
    ))
}

// 原草案保留于 Git 2faa817 与 docs/evidence/v2-m4-m5-postgres，不编译或注册。
