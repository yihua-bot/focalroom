// 印前检查的真实库验收：判定要落在真文件与真版面上，而不只是内存里的几何数字。
use between_frames_lib::{
    command_handler::{CommandHandler, GuiCommandRequest},
    commands_generated::GuiCommand,
    db::Store,
    domain::Project,
    image_pipeline,
    publication_preflight::PreflightReport,
    publishing::{CreatePublicationCommand, GenerateFromSequenceCommand},
};
use std::path::Path;

fn codes(report: &PreflightReport) -> Vec<&'static str> {
    report.issues.iter().map(|issue| issue.code).collect()
}

/// 按导入顺序把选中的照片送进作品序列；序列是生成书稿的唯一输入。
async fn keep(db: &Store, project: &Project, count: usize) -> Project {
    let ids: Vec<String> = project
        .photos
        .iter()
        .take(count)
        .map(|p| p.id.clone())
        .collect();
    CommandHandler::new(db)
        .execute_gui(GuiCommandRequest::new(
            &project.id,
            project.revision,
            GuiCommand::SetPhotoDecision {
                photo_ids: ids,
                status: "keep".into(),
            },
        ))
        .await
        .unwrap();
    db.get(&project.id).await.unwrap()
}

async fn setup(path: &Path, sizes: &[(u32, u32)]) -> (Store, Project) {
    let db = Store::open(&path.join("projects.sqlite")).await.unwrap();
    let project = db.create("印前", "").await.unwrap();
    for (i, (width, height)) in sizes.iter().enumerate() {
        let source = path.join(format!("p{i}.jpg"));
        image::RgbImage::from_pixel(*width, *height, image::Rgb([(i * 61 % 256) as u8, 40, 70]))
            .save(&source)
            .unwrap();
        db.insert_photo(
            &project.id,
            &image_pipeline::prepare(&source, &path.join("cache")).unwrap(),
        )
        .await
        .unwrap();
    }
    let project = db.get(&project.id).await.unwrap();
    (db, project)
}

async fn book(db: &Store, project: &Project) -> String {
    let publication = db
        .create_publication(
            &project.id,
            &CreatePublicationCommand {
                title: "初稿".into(),
                pub_type: None,
                width_mm: None,
                height_mm: None,
            },
        )
        .await
        .unwrap();
    db.generate_from_sequence(
        &project.id,
        &GenerateFromSequenceCommand {
            publication_id: publication.id.clone(),
            photos_per_page: Some(1),
        },
    )
    .await
    .unwrap();
    publication.id
}

/// 按创建时选的类型与开本生成一本已排好序列的册，用来比对不同出版物拿到的是哪把尺子。
async fn placed(
    db: &Store,
    project: &Project,
    pub_type: &str,
    width_mm: f64,
    height_mm: f64,
) -> String {
    let publication = db
        .create_publication(
            &project.id,
            &CreatePublicationCommand {
                title: "初稿".into(),
                pub_type: Some(pub_type.into()),
                width_mm: Some(width_mm),
                height_mm: Some(height_mm),
            },
        )
        .await
        .unwrap();
    db.generate_from_sequence(
        &project.id,
        &GenerateFromSequenceCommand {
            publication_id: publication.id.clone(),
            photos_per_page: Some(1),
        },
    )
    .await
    .unwrap();
    publication.id
}

#[tokio::test]
async fn a_generated_book_is_printable_until_one_original_goes_missing() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), &[(3000, 4000), (3000, 4000)]).await;
    let project = keep(&db, &project, 2).await;
    let publication = book(&db, &project).await;

    let report = db
        .publication_preflight(&project.id, &publication, None)
        .await
        .unwrap();
    assert_eq!(report.min_ppi, 300, "未指定时按印业常规 300 PPI");
    assert_eq!(report.pages, 3, "封面加两张内页");
    assert_eq!(report.frames, 3);
    assert_eq!(
        report.blocking, 0,
        "高清原图整本可送印：{:?}",
        report.issues
    );
    assert!(!report.truncated);
    for code in [
        "original_offline",
        "low_resolution",
        "frame_unbound",
        "source_missing",
        "bleed_shortfall",
        "sequence_not_placed",
    ] {
        assert!(!codes(&report).contains(&code), "不该出现 {code}");
    }

    // 原图从磁盘消失：只报失联，不再拿算不出的 PPI 叠加问题。
    std::fs::remove_file(&project.photos[0].original_path).unwrap();
    let report = db
        .publication_preflight(&project.id, &publication, None)
        .await
        .unwrap();
    let pages: Vec<i64> = report
        .issues
        .iter()
        .filter(|issue| issue.code == "original_offline")
        .map(|issue| issue.page_number.expect("失联必须有页码"))
        .collect();
    // 序列第一张同时是封面和第一页，所以要改的是两处，而不是一处。
    assert_eq!(pages, vec![0, 1], "{:?}", report.issues);
    assert_eq!(report.blocking, 2, "{:?}", report.issues);
    let offline = report
        .issues
        .iter()
        .find(|issue| issue.code == "original_offline")
        .expect("失联原图必须阻断");
    assert_eq!(offline.severity, "blocking");
    assert!(offline.frame_id.is_some() && offline.page_number.is_some());
    assert_eq!(offline.measured, None);
}

#[tokio::test]
async fn undersized_pixels_follow_the_requested_resolution() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), &[(600, 800)]).await;
    let project = keep(&db, &project, 1).await;
    let publication = book(&db, &project).await;

    let report = db
        .publication_preflight(&project.id, &publication, None)
        .await
        .unwrap();
    assert_eq!(
        codes(&report)
            .iter()
            .filter(|c| **c == "low_resolution")
            .count(),
        2,
        "封面加一页"
    );
    assert_eq!(report.blocking, 2);
    for issue in report.issues.iter().filter(|i| i.code == "low_resolution") {
        assert_eq!(issue.required, Some(300.0));
        let measured = issue.measured.expect("低分辨率必须给出实测 PPI");
        assert!(
            (1.0..300.0).contains(&measured),
            "实测 {measured} 应低于要求"
        );
    }

    // 换成要求 72 PPI 的打印件：同一版面应当放行。
    let report = db
        .publication_preflight(&project.id, &publication, Some(72))
        .await
        .unwrap();
    assert_eq!(report.min_ppi, 72);
    assert_eq!(report.blocking, 0, "{:?}", report.issues);
}

#[tokio::test]
async fn a_type_and_trim_size_chosen_at_creation_survives_to_the_pages_and_the_check() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), &[(3000, 4000), (3000, 4000)]).await;
    let project = keep(&db, &project, 1).await;

    // 同一段序列，两种出版物各自按创建时选的类型与开本生成。
    let photobook = placed(&db, &project, "photobook", 210.0, 297.0).await;
    let exhibition = placed(&db, &project, "exhibition", 297.0, 420.0).await;

    for (id, width, height) in [(&photobook, 210.0, 297.0), (&exhibition, 297.0, 420.0)] {
        let pages = db.publication_pages(&project.id, id).await.unwrap();
        assert_eq!(pages.len(), 2, "封面加一张内页");
        for page in &pages {
            assert_eq!(
                (page.width_mm, page.height_mm),
                (width, height),
                "第 {} 页没按创建时的开本生成",
                page.page_number
            );
        }
    }

    let book_report = db
        .publication_preflight(&project.id, &photobook, None)
        .await
        .unwrap();
    assert_eq!(book_report.blocking, 0, "{:?}", book_report.issues);

    // 同样 3000×4000 的像素放大到 A3，分辨率按同一条尺子掉下来：展览册不是被当成 A4 摄影书算的。
    let report = db
        .publication_preflight(&project.id, &exhibition, None)
        .await
        .unwrap();
    assert_eq!(report.pages, 2);
    assert_eq!(report.frames, 2);
    assert_eq!(report.blocking, 2, "{:?}", report.issues);
    let measured: Vec<f64> = report
        .issues
        .iter()
        .filter(|issue| issue.code == "low_resolution")
        .map(|issue| issue.measured.expect("低分辨率必须给出实测 PPI"))
        .collect();
    assert_eq!(measured.len(), 2, "{:?}", report.issues);
    assert!(
        measured.iter().all(|ppi| (200.0..300.0).contains(ppi)),
        "A3 上 3000px 宽约 285 PPI、满页裁切约 260 PPI：{measured:?}"
    );

    // 册生成后又多选了一张：摄影书要提醒漏排，展览册不按"整本选片"这条尺子判。
    let project = keep(&db, &project, 2).await;
    let book_report = db
        .publication_preflight(&project.id, &photobook, None)
        .await
        .unwrap();
    let unplaced = book_report
        .issues
        .iter()
        .find(|issue| issue.code == "sequence_not_placed")
        .expect("摄影书漏排一张要提醒");
    assert_eq!(unplaced.severity, "warning");
    assert_eq!(unplaced.required, Some(1.0));

    let report = db
        .publication_preflight(&project.id, &exhibition, None)
        .await
        .unwrap();
    assert!(
        !codes(&report).contains(&"sequence_not_placed"),
        "展览册不该拿摄影书的选片尺子判：{:?}",
        report.issues
    );
    assert_eq!(report.blocking, 2, "{:?}", report.issues);
}

#[tokio::test]
async fn selected_photos_left_out_of_the_book_and_a_bad_threshold() {
    let temp = tempfile::tempdir().unwrap();
    let (db, project) = setup(temp.path(), &[(3000, 4000), (3000, 4000), (3000, 4000)]).await;
    let project = keep(&db, &project, 2).await;
    let publication = book(&db, &project).await;
    let report = db
        .publication_preflight(&project.id, &publication, None)
        .await
        .unwrap();
    assert!(!codes(&report).contains(&"sequence_not_placed"));

    // 册生成后又选了一张：书稿里没有它的位置，要提醒而不是静默漏印。
    let project = keep(&db, &project, 3).await;
    let report = db
        .publication_preflight(&project.id, &publication, None)
        .await
        .unwrap();
    let unplaced = report
        .issues
        .iter()
        .find(|issue| issue.code == "sequence_not_placed")
        .expect("序列未入册必须提醒");
    assert_eq!(unplaced.severity, "warning");
    assert_eq!(unplaced.page_number, None, "整本级问题不绑页面");
    assert_eq!(unplaced.required, Some(1.0));
    assert_eq!(report.blocking, 0);

    for rejected in [Some(0), Some(71), Some(1201)] {
        let error = db
            .publication_preflight(&project.id, &publication, rejected)
            .await
            .unwrap_err();
        assert_eq!(error.code, "VALIDATION_ERROR");
        assert!(error.message.contains("72–1200"), "{}", error.message);
    }
}
