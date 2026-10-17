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

pub async fn migrate_v2(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for ddl in [
        "CREATE TABLE IF NOT EXISTS workspaces (id TEXT PRIMARY KEY, owner_id TEXT NOT NULL, name TEXT NOT NULL DEFAULT '默认工作区', created_at TEXT DEFAULT (datetime('now')))",
        "CREATE TABLE IF NOT EXISTS v2_cloud_promotions (id TEXT PRIMARY KEY, cloud_project_id TEXT NOT NULL, workspace_id TEXT NOT NULL, local_project_id TEXT, manifest_digest TEXT NOT NULL, from_revision INTEGER NOT NULL DEFAULT 0, state TEXT NOT NULL DEFAULT 'SHARED' CHECK(state IN('SHARED','FINALIZED','ABORTED')), manifest_json TEXT NOT NULL, created_at TEXT DEFAULT (datetime('now')))",
    ] {
        sqlx::query(ddl).execute(pool).await?;
    }
    Ok(())
}

pub async fn migrate_collab(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for ddl in [
        "CREATE TABLE IF NOT EXISTS project_comments (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, author_id TEXT NOT NULL, photo_sha256 TEXT, anchor_x REAL, anchor_y REAL, body TEXT NOT NULL, parent_id TEXT, state TEXT DEFAULT 'open', created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now')))",
        "CREATE TABLE IF NOT EXISTS project_proposals (id TEXT PRIMARY KEY, project_id TEXT NOT NULL, author_id TEXT NOT NULL, title TEXT NOT NULL, base_revision INTEGER NOT NULL, state TEXT DEFAULT 'submitted', operations_json TEXT DEFAULT '[]', approval_nonce TEXT NOT NULL, decided_by TEXT, decided_at TEXT, created_at TEXT DEFAULT (datetime('now')))",
        "CREATE TABLE IF NOT EXISTS project_revisions (project_id TEXT NOT NULL, revision INTEGER NOT NULL, snapshot_json TEXT NOT NULL, author_id TEXT NOT NULL, cause TEXT NOT NULL, created_at TEXT DEFAULT (datetime('now')), PRIMARY KEY(project_id, revision))",
        "CREATE TABLE IF NOT EXISTS project_presence (project_id TEXT NOT NULL, user_id TEXT NOT NULL, session_id TEXT NOT NULL, last_seen TEXT DEFAULT (datetime('now')), viewing_sha256 TEXT, PRIMARY KEY(project_id, user_id, session_id))",
        "CREATE TABLE IF NOT EXISTS acl_audit (id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL, user_id TEXT NOT NULL, action TEXT NOT NULL, resource TEXT NOT NULL, allowed INTEGER NOT NULL, created_at TEXT DEFAULT (datetime('now')))",
    ] {
        sqlx::query(ddl).execute(pool).await?;
    }
    Ok(())
}
