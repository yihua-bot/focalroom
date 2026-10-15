// v2.0 S02：签名链接——有效访问、过期拒绝、撤销拒绝（区别于过期）、伪造 token 拒绝、项目级批量撤销。
use between_frames_lib::{agent::media_links, db::Store, image_pipeline};
use std::path::Path;

async fn setup(path: &Path) -> (Store, String, String) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let project = db.create("链接", "").await.unwrap();
    let source = path.join("link.png");
    image::RgbImage::from_pixel(32, 24, image::Rgb([120, 180, 90]))
        .save(&source)
        .unwrap();
    let prepared = image_pipeline::prepare(&source, &path.join("cache")).unwrap();
    db.insert_photo(&project.id, &prepared).await.unwrap();
    let project = db.get(&project.id).await.unwrap();
    (db, project.id.clone(), project.photos[0].id.clone())
}

#[tokio::test]
async fn s02_revoked_link_denied_even_with_valid_signature() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project, photo) = setup(temp.path()).await;
    unsafe { std::env::set_var(media_links::MEDIA_KEY_ENV, "media-test-key") };
    let now = 1_900_000_000_000_i64;
    let grant = media_links::issue(&db, &project, &photo, now + 3_600_000)
        .await
        .unwrap();
    let token = grant["token"].as_str().unwrap().to_owned();
    let grant_id = grant["grant_id"].as_str().unwrap().to_owned();
    // 有效期内正常访问。
    assert!(
        media_links::verify(&db, &project, &photo, &token, now)
            .await
            .unwrap()["allowed"]
            .as_bool()
            .unwrap()
    );
    // 过期拒绝。
    let expired = media_links::verify(&db, &project, &photo, &token, now + 3_600_001)
        .await
        .unwrap_err();
    assert!(expired.message.contains("过期"));
    // 撤销后拒绝（S02 核心：签名仍有效也被拒，且区别于过期文案）。
    media_links::revoke(&db, &grant_id).await.unwrap();
    let revoked = media_links::verify(&db, &project, &photo, &token, now)
        .await
        .unwrap_err();
    assert_eq!(revoked.code, "PERMISSION_DENIED");
    assert!(
        revoked.message.contains("撤销"),
        "撤销拒绝须区别于过期：{}",
        revoked.message
    );
    // 伪造 token 拒绝。
    let forged = media_links::verify(&db, &project, &photo, "forged-token-value", now)
        .await
        .unwrap_err();
    assert_eq!(forged.code, "PERMISSION_DENIED");
    // 跨项目照片发链接拒绝。
    let cross = media_links::issue(&db, &project, "photo-not-mine", now + 60_000)
        .await
        .unwrap_err();
    assert_eq!(cross.code, "PERMISSION_DENIED");
    unsafe { std::env::remove_var(media_links::MEDIA_KEY_ENV) };
}

#[tokio::test]
async fn revoke_project_kills_all_active_links() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project, photo) = setup(temp.path()).await;
    unsafe { std::env::set_var(media_links::MEDIA_KEY_ENV, "media-test-key") };
    let now = 1_900_000_000_000_i64;
    media_links::issue(&db, &project, &photo, now + 60_000)
        .await
        .unwrap();
    media_links::issue(&db, &project, &photo, now + 120_000)
        .await
        .unwrap();
    let killed = media_links::revoke_project(&db, &project).await.unwrap();
    assert_eq!(killed, 2, "两条有效授权全部撤销");
    let grants: Vec<(String,)> =
        sqlx::query_as("SELECT state FROM v2_media_grants WHERE project_id=?")
            .bind(&project)
            .fetch_all(&db.pool)
            .await
            .unwrap();
    assert!(grants.iter().all(|(state,)| state == "revoked"));
    // 再次批量撤销为幂等零。
    assert_eq!(media_links::revoke_project(&db, &project).await.unwrap(), 0);
    unsafe { std::env::remove_var(media_links::MEDIA_KEY_ENV) };
}
