use sqlx::SqlitePool;

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        email TEXT UNIQUE NOT NULL,
        salt TEXT NOT NULL,
        password_hash TEXT NOT NULL,
        display_name TEXT NOT NULL,
        created_at TEXT DEFAULT (datetime('now'))
    )"#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS sessions (
        token TEXT PRIMARY KEY,
        user_id TEXT NOT NULL REFERENCES users(id),
        created_at TEXT DEFAULT (datetime('now'))
    )"#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS cloud_projects (
        id TEXT PRIMARY KEY,
        owner_id TEXT NOT NULL REFERENCES users(id),
        title TEXT NOT NULL,
        created_at TEXT DEFAULT (datetime('now'))
    )"#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS project_grants (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL REFERENCES cloud_projects(id),
        grantee_id TEXT NOT NULL REFERENCES users(id),
        role TEXT NOT NULL CHECK(role IN('viewer','reviewer')),
        state TEXT NOT NULL DEFAULT('active') CHECK(state IN('active','revoked')),
        created_at TEXT DEFAULT (datetime('now')),
        revoked_at TEXT,
        UNIQUE(project_id, grantee_id)
    )"#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS preview_files (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL REFERENCES cloud_projects(id),
        sha256 TEXT NOT NULL,
        width INTEGER NOT NULL,
        height INTEGER NOT NULL,
        file_size INTEGER NOT NULL,
        created_at TEXT DEFAULT (datetime('now')),
        UNIQUE(project_id, sha256)
    )"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
