use between_frames_lib::{db::Store, seed_demo};

#[tokio::test]
async fn demo_is_created_once_alongside_user_projects_and_respects_archive() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(&root.path().join("projects.sqlite"))
        .await
        .unwrap();
    let user = store.create("我的旅行", "观察光线").await.unwrap();
    let resources = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/demo");
    let cache = root.path().join("cache");
    seed_demo::ensure_demo(&store, &resources, &cache)
        .await
        .unwrap();
    let projects = store.list(false).await.unwrap();
    assert_eq!(projects.len(), 2);
    let demo = projects.iter().find(|p| p.id == "demo-first-walk").unwrap();
    assert_eq!(demo.title, "第一次散步 · 演示项目");
    assert_eq!(demo.photos.len(), 12);
    assert!(demo.sequence.is_empty());
    assert_eq!(store.get(&user.id).await.unwrap().brief, "观察光线");
    seed_demo::ensure_demo(&store, &resources, &cache)
        .await
        .unwrap();
    assert_eq!(store.get(&demo.id).await.unwrap().revision, demo.revision);
    let renamed = store
        .edit(&demo.id, "我的演示作品", "新的创作目标", demo.revision)
        .await
        .unwrap();
    seed_demo::ensure_demo(&store, &resources, &cache)
        .await
        .unwrap();
    assert_eq!(store.get(&demo.id).await.unwrap().title, "我的演示作品");
    store.archive(&demo.id, renamed.revision).await.unwrap();
    seed_demo::ensure_demo(&store, &resources, &cache)
        .await
        .unwrap();
    assert_eq!(store.list(false).await.unwrap().len(), 1);
    assert_eq!(store.list(true).await.unwrap().len(), 1);
}

#[tokio::test]
async fn missing_demo_resources_do_not_leave_an_empty_project() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(&root.path().join("projects.sqlite"))
        .await
        .unwrap();
    seed_demo::ensure_demo(
        &store,
        &root.path().join("missing"),
        &root.path().join("cache"),
    )
    .await
    .unwrap();
    assert!(store.list(false).await.unwrap().is_empty());
}
