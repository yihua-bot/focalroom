//! 首启动内置演示项目：从应用资源拷贝示例照片并经正式导入链路入库。
//! 仅在没有任何未删除项目时执行一次；所有照片走 image_pipeline + import_photo
//! （与用户手动导入完全同一条审计路径）。
use crate::{command_handler::CommandHandler, db::Store, domain::Result, image_pipeline};
use std::path::Path;

const DEMO_PROJECT_ID: &str = "demo-first-walk";
const DEMO_TITLE: &str = "示例 · 第一次散步";
const DEMO_BRIEF: &str = "内置示例照片，体验选片、比较与 AI 智能选片。可随时删除。";

/// 资源目录中的演示照片（有序）。
fn demo_photos(resource_demo: &Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(resource_demo)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("jpg")))
        .collect();
    files.sort();
    files
}

/// 首启动 seed：无项目时创建演示项目并导入全部示例照片。
pub async fn seed_if_empty(store: &Store, resource_demo: &Path, cache: &Path) -> Result<()> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM projects WHERE deleted_at IS NULL",
    )
    .fetch_one(&store.pool)
    .await
    .unwrap_or(0);
    if count > 0 {
        return Ok(());
    }
    let photos = demo_photos(resource_demo);
    if photos.is_empty() {
        return Ok(());
    }
    // 独立目录保存演示原图（用户删除项目时一并清理，不影响其他缓存）。
    let originals = cache.join("demo-originals");
    std::fs::create_dir_all(&originals)?;
    let now = crate::domain::timestamp();
    sqlx::query("INSERT INTO projects(id,title,brief,created_at,updated_at) VALUES(?,?,?,?,?)")
        .bind(DEMO_PROJECT_ID)
        .bind(DEMO_TITLE)
        .bind(DEMO_BRIEF)
        .bind(&now)
        .bind(&now)
        .execute(&store.pool)
        .await?;
    let handler = CommandHandler::new(store);
    let mut imported = 0usize;
    for source in photos {
        let target = originals.join(source.file_name().unwrap_or_default());
        if !target.exists() {
            std::fs::copy(&source, &target)?;
        }
        let prepared = match image_pipeline::prepare(&target, cache) {
            Ok(value) => value,
            Err(_) => continue, // 单张失败跳过，不阻塞演示项目
        };
        if handler.import_photo(DEMO_PROJECT_ID, &prepared).await.unwrap_or(false) {
            imported += 1;
        }
    }
    if imported == 0 {
        // 没有任何照片入库时回滚空项目，避免留下坏示例。
        let _ = sqlx::query("DELETE FROM projects WHERE id=?")
            .bind(DEMO_PROJECT_ID)
            .execute(&store.pool)
            .await;
    }
    Ok(())
}
