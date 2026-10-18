// M3 质量记录存储：写入校验（五维/样本量/理由）与读取换算；迁移随 Store 打开生效。
use between_frames_lib::agent::{quality::record_quality_score, router::TaskType};
use between_frames_lib::db::Store;

#[tokio::test]
async fn quality_scores_round_trip_with_validation() {
    let temp = tempfile::tempdir().unwrap();
    let db = Store::open(&temp.path().join("q.sqlite")).await.unwrap();
    // 非法输入被拒。
    assert!(
        record_quality_score(
            &db.pool,
            "openai",
            "m1",
            TaskType::Curation,
            "v1",
            [3, 3, 3, 3, 6],
            2,
            false,
            "理由"
        )
        .await
        .is_err(),
        "五维分越界必须拒绝"
    );
    assert!(
        record_quality_score(
            &db.pool,
            "openai",
            "m1",
            TaskType::Curation,
            "v1",
            [3, 3, 3, 3, 3],
            0,
            false,
            "理由"
        )
        .await
        .is_err(),
        "样本量为零必须拒绝"
    );
    assert!(
        record_quality_score(
            &db.pool,
            "openai",
            "m1",
            TaskType::Curation,
            "v1",
            [3, 3, 3, 3, 3],
            2,
            false,
            "  "
        )
        .await
        .is_err(),
        "空理由必须拒绝（可核验性硬要求）"
    );
    // 合法写入两条（同一模型先后版本），读取按时间升序；最新记录供路由消费。
    record_quality_score(
        &db.pool,
        "openai",
        "m1",
        TaskType::Curation,
        "v1",
        [3, 4, 3, 4, 3],
        6,
        false,
        "首版：节奏稳定",
    )
    .await
    .unwrap();
    record_quality_score(
        &db.pool,
        "openai",
        "m1",
        TaskType::Curation,
        "v2",
        [5, 4, 4, 5, 4],
        12,
        false,
        "二版：事实更准",
    )
    .await
    .unwrap();
    record_quality_score(
        &db.pool,
        "openai",
        "m2",
        TaskType::Curation,
        "v1",
        [1, 2, 2, 1, 2],
        3,
        true,
        "越权写入：硬门槛违规",
    )
    .await
    .unwrap();
    let records = between_frames_lib::agent::quality::load_quality_records(&db.pool, "openai")
        .await
        .unwrap();
    assert_eq!(records.len(), 3, "全部记录按时间升序返回");
    assert_eq!(records[0].score_version, "v1", "同模型旧版本在前");
    let m1_latest = records.iter().rev().find(|r| r.model_id == "m1").unwrap();
    assert_eq!(m1_latest.score_version, "v2");
    assert_eq!(m1_latest.sample_count, 12);
    // 五维均分换算：(5+4+4+5+4)/25*5 = 4.4。
    assert!((m1_latest.mean_score - 4.4).abs() < 1e-6);
    let m2 = records.iter().find(|r| r.model_id == "m2").unwrap();
    assert!(m2.hard_fail, "硬门槛标记必须保留");
    // 其他 provider 无记录。
    assert!(
        between_frames_lib::agent::quality::load_quality_records(&db.pool, "mock")
            .await
            .unwrap()
            .is_empty()
    );
}
