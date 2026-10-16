//! M4 云 API 入口。
#[tokio::main]
async fn main() {
    let db_path = std::env::var("BF_CLOUD_DB").unwrap_or_else(|_| "bf-cloud.sqlite".into());
    let preview_dir = std::env::var("BF_PREVIEW_DIR").unwrap_or_else(|_| "./previews".into());
    let pool = sqlx::SqlitePool::connect(&format!("sqlite:{db_path}"))
        .await
        .expect("数据库连接失败");
    std::fs::create_dir_all(&preview_dir).expect("预览目录创建失败");
    let app = bf_cloud::build_router(pool, preview_dir.into()).await;
    let port: u16 = std::env::var("BF_CLOUD_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("端口绑定失败");
    println!("bf-cloud 垂直切片监听 :{port}");
    axum::serve(listener, app).await.expect("服务器启动失败");
}
