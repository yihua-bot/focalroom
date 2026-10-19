//! 首启动内置演示项目：从应用资源拷贝示例照片并经正式导入链路入库。
//! 内置项目仅创建一次，归档后不再创建；所有照片走 image_pipeline + import_photo
//! （与用户手动导入完全同一条审计路径）。
use crate::{command_handler::CommandHandler, db::Store, domain::Result, image_pipeline};
use std::path::Path;

const DEMO_PROJECT_ID: &str = "demo-first-walk";
const DEMO_TITLE: &str = "第一次散步 · 演示项目";
const DEMO_BRIEF: &str = "从这些冬日影像中选出 8 张照片，突出雪地的光线变化与动物的自然状态，编排一组节奏舒缓的漫游影像。";

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

/// 内置演示独立于用户项目：只补齐一次，尊重用户之后的编辑和归档。
pub async fn ensure_demo(store: &Store, resource_demo: &Path, cache: &Path) -> Result<()> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id=?)")
        .bind(DEMO_PROJECT_ID)
        .fetch_one(&store.pool)
        .await?;
    if exists {
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
        if handler
            .import_photo(DEMO_PROJECT_ID, &prepared)
            .await
            .unwrap_or(false)
        {
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
