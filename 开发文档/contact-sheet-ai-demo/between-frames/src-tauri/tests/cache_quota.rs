// M1-09/M5 缓存配额清理：孤儿（无 photo 引用）按最旧回收；引用缓存与非缓存文件不动。
use between_frames_lib::image_pipeline;
use std::time::{Duration, SystemTime};

fn touch(path: &std::path::Path, bytes: usize, age_seconds: u64) {
    std::fs::write(path, vec![0u8; bytes]).unwrap();
    let stamp = SystemTime::now() - Duration::from_secs(age_seconds);
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(stamp)
        .unwrap();
}

#[test]
fn sweep_keeps_live_and_oldest_first_orphans_only() {
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    // 两个被引用 hash（各 100B）+ 三个孤儿（最旧 1000B、次旧 100B、最新 100B）。
    touch(&cache.join("aaaa-320.jpg"), 100, 60);
    touch(&cache.join("aaaa-1400.jpg"), 100, 60);
    touch(&cache.join("bbbb-320.jpg"), 100, 60);
    touch(&cache.join("old1-320.jpg"), 1000, 5000);
    touch(&cache.join("mid1-1400.jpg"), 100, 3000);
    touch(&cache.join("new1-320.jpg"), 100, 10);
    // 非缓存命名文件必须保留。
    touch(&cache.join("notes.txt"), 50, 9000);
    let live = vec!["aaaa".to_owned(), "bbbb".to_owned()];
    // 限额内：不删任何东西。
    assert_eq!(
        image_pipeline::sweep_orphans(&cache, &live, 10_000).unwrap(),
        0
    );
    assert!(cache.join("old1-320.jpg").is_file());
    // 限额 350B：live 300B + 孤儿共 1500B 超限；逐个删最旧孤儿直到 total ≤ 350。
    // 每个孤儿 ≥100B，删到只剩 live（300B）才达标，三个孤儿全部回收。
    let reclaimed = image_pipeline::sweep_orphans(&cache, &live, 350).unwrap();
    assert_eq!(reclaimed, 1200);
    assert!(!cache.join("old1-320.jpg").exists());
    assert!(!cache.join("mid1-1400.jpg").exists());
    assert!(!cache.join("new1-320.jpg").exists());
    assert!(cache.join("aaaa-320.jpg").is_file());
    assert!(cache.join("bbbb-320.jpg").is_file());
    assert!(cache.join("notes.txt").is_file());
    // 宽松限额下重新出现的孤儿不会被动；再一次超限清理按最旧优先。
    touch(&cache.join("fresh-320.jpg"), 100, 5);
    let reclaimed = image_pipeline::sweep_orphans(&cache, &live, 10_000).unwrap();
    assert_eq!(reclaimed, 0);
    assert!(cache.join("fresh-320.jpg").is_file());
    let reclaimed = image_pipeline::sweep_orphans(&cache, &live, 399).unwrap();
    assert_eq!(reclaimed, 100);
    assert!(!cache.join("fresh-320.jpg").exists());
}

#[test]
fn sweep_missing_cache_dir_is_noop() {
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(
        image_pipeline::sweep_orphans(&temp.path().join("none"), &[], 1000).unwrap(),
        0
    );
}
