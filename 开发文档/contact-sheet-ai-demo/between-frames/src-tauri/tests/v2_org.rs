// v2.0 M6 组织骨架：席位上限、角色撤回不可自愈、OWNER 保护、审计留痕。
use between_frames_lib::{agent::org, db::Store};
use std::path::Path;

async fn fresh(path: &Path) -> Store {
    Store::open(&path.join("projects.sqlite")).await.unwrap()
}

#[tokio::test]
async fn seats_limit_revocation_and_owner_protection() {
    let temp = tempfile::tempdir().unwrap();
    let db = fresh(temp.path()).await;
    // 本地默认行由迁移插入。
    let members = org::members(&db, "local-workspace").await.unwrap();
    let list = members["members"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["role"], "OWNER");
    // 新工作区：2 席上限。
    org::ensure_workspace(&db, "ws-edu", "teacher-1", "教学工坊", 2)
        .await
        .unwrap();
    org::add_member(&db, "ws-edu", "student-1", "VIEWER")
        .await
        .unwrap();
    // 第 3 席超限拒绝。
    let full = org::add_member(&db, "ws-edu", "student-2", "VIEWER")
        .await
        .unwrap_err();
    assert_eq!(full.code, "BUDGET_EXCEEDED");
    assert!(full.message.contains("2/2"));
    // 撤回 student-1 后席位释放，student-2 可加入；被撤回者不可自行恢复。
    org::revoke_member(&db, "ws-edu", "student-1")
        .await
        .unwrap();
    org::add_member(&db, "ws-edu", "student-2", "EDITOR")
        .await
        .unwrap();
    let back = org::add_member(&db, "ws-edu", "student-1", "VIEWER")
        .await
        .unwrap_err();
    assert_eq!(back.code, "PERMISSION_DENIED");
    // OWNER 不可被撤回。
    let owner = org::revoke_member(&db, "ws-edu", "teacher-1")
        .await
        .unwrap_err();
    assert_eq!(owner.code, "PERMISSION_DENIED");
    // 非法角色拒绝。
    let bad = org::add_member(&db, "ws-edu", "student-9", "SUPERADMIN")
        .await
        .unwrap_err();
    assert_eq!(bad.code, "VALIDATION_ERROR");
    // 审计留痕：ensure + add + revoke + add 至少 4 条。
    let audit: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM v2_workspace_audit WHERE workspace_id='ws-edu'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert!(audit >= 4);
}
