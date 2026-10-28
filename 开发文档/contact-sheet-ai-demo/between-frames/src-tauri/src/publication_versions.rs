//! 出版版面快照：引用固定资产/图像版本，不含文件路径，不冒充已渲染的印前交付。
use crate::{
    command_audit::{encode, fingerprint, serialization},
    db::Store,
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};

pub(crate) const MAX_SNAPSHOT: usize = 8 * 1024 * 1024;
fn invalid(message: &str) -> AppError {
    AppError::new("VALIDATION_ERROR", message)
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutPublication {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub pub_type: String,
    pub spec_json: String,
    pub state: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutPage {
    pub id: String,
    pub page_number: i64,
    pub side: String,
    pub width_mm: f64,
    pub height_mm: f64,
    pub margin_mm: f64,
    pub bleed_mm: f64,
    pub layout_json: Option<String>,
    #[sqlx(skip)]
    pub frames: Vec<LayoutFrame>,
    #[sqlx(skip)]
    pub texts: Vec<LayoutText>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutFrame {
    pub id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub fit_mode: String,
    pub asset_id: Option<String>,
    pub rendition_id: Option<String>,
    pub crop_x: f64,
    pub crop_y: f64,
    pub crop_zoom: f64,
    #[sqlx(skip)]
    pub source: Option<LayoutSource>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutText {
    pub id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub content_json: String,
    pub style_json: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutSource {
    pub asset_id: String,
    pub source_sha256: String,
    pub width: i64,
    pub height: i64,
    pub source_profile_digest: Option<String>,
    pub rendition_id: Option<String>,
    pub variant_id: Option<String>,
    pub rendition_sha256: Option<String>,
    pub rendition_width: Option<i64>,
    pub rendition_height: Option<i64>,
    pub bit_depth: Option<i64>,
    pub icc_digest: Option<String>,
    #[sqlx(skip)]
    pub preview_sha256: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationLayout {
    pub schema_version: i64,
    pub publication: LayoutPublication,
    pub pages: Vec<LayoutPage>,
}
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PublicationVersionInfo {
    pub id: String,
    pub publication_id: String,
    pub revision: i64,
    pub label: String,
    pub kind: String,
    pub snapshot_sha256: String,
    pub created_at: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationVersion {
    pub version: PublicationVersionInfo,
    pub layout: PublicationLayout,
    pub previews: Vec<crate::publication_previews::FramePreview>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationDocument {
    pub revision: i64,
    pub layout: PublicationLayout,
    pub previews: Vec<crate::publication_previews::FramePreview>,
    pub master_states: Vec<crate::publication_commands::masters::MasterState>,
}

pub(crate) async fn source_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    asset: Option<&str>,
    rendition: Option<&str>,
) -> Result<Option<LayoutSource>> {
    let Some(asset) = asset else {
        if rendition.is_some() {
            return Err(invalid("请先为图像版本选择照片"));
        }
        return Ok(None);
    };
    let source:Option<LayoutSource>=sqlx::query_as("SELECT a.id AS asset_id,a.content_sha256 AS source_sha256,a.width,a.height,a.source_profile_digest,r.id AS rendition_id,r.variant_id,r.sha256 AS rendition_sha256,r.width AS rendition_width,r.height AS rendition_height,r.bit_depth,r.icc_digest FROM source_assets a LEFT JOIN edit_variants v ON v.asset_id=a.id LEFT JOIN renditions r ON r.variant_id=v.id AND r.id=? WHERE a.id=? AND a.project_id=? AND (? IS NULL OR r.id=?) ORDER BY r.id DESC LIMIT 1")
        .bind(rendition).bind(asset).bind(project).bind(rendition).bind(rendition).fetch_optional(&mut **tx).await?;
    source
        .map(Some)
        .ok_or_else(|| AppError::new("ASSET_NOT_FOUND", "照片或图像版本不属于当前项目"))
}
pub(crate) async fn layout_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    id: &str,
) -> Result<PublicationLayout> {
    let publication:LayoutPublication=sqlx::query_as("SELECT b.id,b.project_id,b.title,b.pub_type,b.spec_json,b.state FROM publications b JOIN projects p ON p.id=b.project_id WHERE b.id=? AND b.project_id=? AND p.deleted_at IS NULL")
        .bind(id).bind(project).fetch_optional(&mut **tx).await?.ok_or_else(||AppError::new("NOT_FOUND","出版物不存在或项目已归档"))?;
    let mut pages:Vec<LayoutPage>=sqlx::query_as("SELECT id,page_number,side,width_mm,height_mm,margin_mm,bleed_mm,layout_json FROM publication_pages WHERE publication_id=? ORDER BY CASE WHEN page_number=-1 THEN 10001 ELSE page_number END,id LIMIT 513").bind(id).fetch_all(&mut **tx).await?;
    if pages.len() > 512 {
        return Err(invalid("出版物最多支持512页，请拆分为多个分册"));
    }
    let count:i64=sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM publication_frames f JOIN publication_pages p ON p.id=f.page_id WHERE p.publication_id=?)+(SELECT COUNT(*) FROM publication_texts t JOIN publication_pages p ON p.id=t.page_id WHERE p.publication_id=?)").bind(id).bind(id).fetch_one(&mut **tx).await?;
    if count > 4096 {
        return Err(invalid("版面内容过多，请拆分为多个分册"));
    }
    for page in &mut pages {
        page.frames=sqlx::query_as("SELECT id,x_mm,y_mm,width_mm,height_mm,fit_mode,asset_id,rendition_id,crop_x,crop_y,crop_zoom FROM publication_frames WHERE page_id=? ORDER BY id").bind(&page.id).fetch_all(&mut **tx).await?;
        page.texts=sqlx::query_as("SELECT id,x_mm,y_mm,width_mm,height_mm,content_json,style_json FROM publication_texts WHERE page_id=? ORDER BY id").bind(&page.id).fetch_all(&mut **tx).await?;
        for frame in &mut page.frames {
            frame.source = source_tx(
                tx,
                project,
                frame.asset_id.as_deref(),
                frame.rendition_id.as_deref(),
            )
            .await?;
        }
    }
    let layout = PublicationLayout {
        schema_version: if crate::publication_commands::layers::has(&publication.spec_json)? {
            3
        } else if pages.iter().try_fold(false, |found, page| {
            crate::publication_commands::spreads::links(page)
                .map(|links| found || !links.is_empty())
        })? {
            2
        } else {
            1
        },
        publication,
        pages,
    };
    let mut layout = layout;
    crate::publication_previews::capture(tx, &mut layout).await?;
    validate_layout(&layout)?;
    if encode(&layout)?.len() > MAX_SNAPSHOT {
        return Err(invalid("版面记录过大，请拆分为多个分册"));
    }
    Ok(layout)
}
pub(crate) fn validate_layout(layout: &PublicationLayout) -> Result<()> {
    use std::collections::BTreeSet;
    if !matches!(layout.schema_version, 1..=3) || layout.pages.len() > 512 {
        return Err(invalid("此版面版本暂不支持打开"));
    }
    crate::publication_commands::layers::spec(&layout.publication.spec_json)?;
    crate::publication_commands::masters::masters(layout)?;
    let spread_frames = crate::publication_commands::spreads::validate(layout)?;
    let mut ids = BTreeSet::new();
    let mut numbers = BTreeSet::new();
    let mut count = 0;
    for page in &layout.pages {
        crate::publication_commands::masters::link(page)?;
        crate::publication_commands::guides::read(page)?;
        if !ids.insert(&page.id)
            || !numbers.insert(page.page_number)
            || !matches!(
                page.side.as_str(),
                "left" | "right" | "spread" | "cover" | "back"
            )
            || !(-1..=10000).contains(&page.page_number)
            || ![page.width_mm, page.height_mm]
                .iter()
                .all(|v| v.is_finite() && (50.0..=2000.0).contains(v))
            || !page.margin_mm.is_finite()
            || page.margin_mm < 0.
            || page.margin_mm * 2. >= page.width_mm.min(page.height_mm)
            || !page.bleed_mm.is_finite()
            || !(0.0..=25.0).contains(&page.bleed_mm)
        {
            return Err(invalid("页面尺寸、边距或页码无效"));
        }
        for frame in &page.frames {
            if !ids.insert(&frame.id)
                || !matches!(frame.fit_mode.as_str(), "fit" | "fill" | "stretch")
                || ![frame.crop_x, frame.crop_y]
                    .iter()
                    .all(|n| n.is_finite() && (0.0..=1.0).contains(n))
                || !frame.crop_zoom.is_finite()
                || !(1.0..=20.0).contains(&frame.crop_zoom)
            {
                return Err(invalid("照片框或裁切设置无效"));
            }
            if !spread_frames.contains(&frame.id) {
                rectangle(
                    page,
                    frame.x_mm,
                    frame.y_mm,
                    frame.width_mm,
                    frame.height_mm,
                )?;
            }
        }
        for text in &page.texts {
            if !ids.insert(&text.id)
                || text.content_json.len() > 32768
                || text.style_json.as_ref().is_some_and(|v| v.len() > 8192)
            {
                return Err(invalid("文字内容过大或重复"));
            }
            let content: serde_json::Value =
                serde_json::from_str(&text.content_json).map_err(serialization)?;
            if !content.is_object() {
                return Err(invalid("文字内容无效"));
            }
            if let Some(style) = &text.style_json
                && !serde_json::from_str::<serde_json::Value>(style)
                    .map_err(serialization)?
                    .is_object()
            {
                return Err(invalid("文字样式无效"));
            }
            crate::publication_commands::typography::stored(text.style_json.as_deref())?;
            rectangle(page, text.x_mm, text.y_mm, text.width_mm, text.height_mm)?;
        }
        count += page.frames.len() + page.texts.len();
    }
    if count > 4096 {
        return Err(invalid("版面内容过多，请拆分为多个分册"));
    }
    crate::publication_commands::layers::read(layout)?;
    crate::publication_commands::chapters::read(layout)?;
    crate::publication_commands::cover::validate(layout)?;
    Ok(())
}
pub(crate) fn rectangle(page: &LayoutPage, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
    if ![x, y, w, h].iter().all(|v| v.is_finite())
        || w <= 0.
        || h <= 0.
        || x < -page.bleed_mm
        || y < -page.bleed_mm
        || x + w > page.width_mm + page.bleed_mm
        || y + h > page.height_mm + page.bleed_mm
    {
        return Err(invalid("内容超出页面及出血范围，请调整位置和尺寸"));
    }
    Ok(())
}
pub(crate) async fn version_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    id: &str,
) -> Result<PublicationVersion> {
    let row:Option<(String,String,String)>=sqlx::query_as("SELECT snapshot_json,snapshot_sha256,id FROM publication_versions WHERE id=? AND publication_id=? AND project_id=? AND length(CAST(snapshot_json AS BLOB))<=?").bind(id).bind(publication).bind(project).bind(MAX_SNAPSHOT as i64).fetch_optional(&mut **tx).await?;
    let (raw, digest, _) = row.ok_or_else(|| AppError::new("NOT_FOUND", "找不到这份出版版本"))?;
    let layout: PublicationLayout = serde_json::from_str(&raw).map_err(serialization)?;
    if layout.publication.id != publication
        || layout.publication.project_id != project
        || fingerprint(&serde_json::to_value(&layout).map_err(serialization)?) != digest
    {
        return Err(AppError::new(
            "RECOVERY_ERROR",
            "出版版本记录不完整，请从备份恢复",
        ));
    }
    validate_layout(&layout)?;
    let version:PublicationVersionInfo=sqlx::query_as("SELECT id,publication_id,revision,label,kind,snapshot_sha256,created_at FROM publication_versions WHERE id=?").bind(id).fetch_one(&mut **tx).await?;
    Ok(PublicationVersion {
        version,
        layout,
        previews: Vec::new(),
    })
}
pub(crate) async fn restore_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    layout: &PublicationLayout,
) -> Result<()> {
    validate_layout(layout)?;
    if layout.publication.id != publication || layout.publication.project_id != project {
        return Err(invalid("这份版面不属于当前出版物"));
    }
    for page in &layout.pages {
        for frame in &page.frames {
            let current = source_tx(
                tx,
                project,
                frame.asset_id.as_deref(),
                frame.rendition_id.as_deref(),
            )
            .await?;
            if !same_source(current.as_ref(), frame.source.as_ref()) {
                return Err(AppError::new(
                    "CONFLICT",
                    "历史版面的照片或图像版本已变化，请重新关联后再恢复",
                ));
            }
        }
    }
    sqlx::query("DELETE FROM publication_pages WHERE publication_id=?")
        .bind(publication)
        .execute(&mut **tx)
        .await?;
    let now = crate::domain::timestamp();
    for page in &layout.pages {
        sqlx::query("INSERT INTO publication_pages(id,publication_id,page_number,side,width_mm,height_mm,margin_mm,bleed_mm,layout_json,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&page.id).bind(publication).bind(page.page_number).bind(&page.side).bind(page.width_mm).bind(page.height_mm).bind(page.margin_mm).bind(page.bleed_mm).bind(&page.layout_json).bind(&now).bind(&now).execute(&mut **tx).await?;
        for f in &page.frames {
            sqlx::query("INSERT INTO publication_frames(id,page_id,x_mm,y_mm,width_mm,height_mm,fit_mode,asset_id,rendition_id,crop_x,crop_y,crop_zoom,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)").bind(&f.id).bind(&page.id).bind(f.x_mm).bind(f.y_mm).bind(f.width_mm).bind(f.height_mm).bind(&f.fit_mode).bind(&f.asset_id).bind(&f.rendition_id).bind(f.crop_x).bind(f.crop_y).bind(f.crop_zoom).bind(&now).bind(&now).execute(&mut **tx).await?;
        }
        for t in &page.texts {
            sqlx::query("INSERT INTO publication_texts(id,page_id,x_mm,y_mm,width_mm,height_mm,content_json,style_json,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?)").bind(&t.id).bind(&page.id).bind(t.x_mm).bind(t.y_mm).bind(t.width_mm).bind(t.height_mm).bind(&t.content_json).bind(&t.style_json).bind(&now).bind(&now).execute(&mut **tx).await?;
        }
    }
    sqlx::query(
        "UPDATE publications SET title=?,pub_type=?,spec_json=? WHERE id=? AND project_id=?",
    )
    .bind(&layout.publication.title)
    .bind(&layout.publication.pub_type)
    .bind(&layout.publication.spec_json)
    .bind(publication)
    .bind(project)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
impl Store {
    pub async fn publication_document(
        &self,
        project: &str,
        publication: &str,
    ) -> Result<PublicationDocument> {
        let mut tx = self.pool.begin().await?;
        let layout = layout_tx(&mut tx, project, publication).await?;
        let revision =
            sqlx::query_scalar("SELECT revision FROM publications WHERE id=? AND project_id=?")
                .bind(publication)
                .bind(project)
                .fetch_one(&mut *tx)
                .await?;
        let previews = crate::publication_previews::project(&mut tx, &layout).await?;
        let master_states = crate::publication_commands::masters::states(&layout)?;
        tx.commit().await?;
        Ok(PublicationDocument {
            revision,
            layout,
            previews,
            master_states,
        })
    }
    pub async fn publication_layout(
        &self,
        project: &str,
        publication: &str,
    ) -> Result<PublicationLayout> {
        let mut tx = self.pool.begin().await?;
        let layout = layout_tx(&mut tx, project, publication).await?;
        tx.commit().await?;
        Ok(layout)
    }
    pub async fn publication_version(
        &self,
        project: &str,
        publication: &str,
        version: &str,
    ) -> Result<PublicationVersion> {
        let mut tx = self.pool.begin().await?;
        ensure_tx(&mut tx, project, publication).await?;
        let mut result = version_tx(&mut tx, project, publication, version).await?;
        result.previews = crate::publication_previews::project(&mut tx, &result.layout).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn publication_versions(
        &self,
        project: &str,
        publication: &str,
        before: Option<i64>,
    ) -> Result<Vec<PublicationVersionInfo>> {
        if before.is_some_and(|n| n <= 0) {
            return Err(invalid("版本页码无效"));
        }
        let mut tx = self.pool.begin().await?;
        ensure_tx(&mut tx, project, publication).await?;
        let rows=sqlx::query_as("SELECT id,publication_id,revision,label,kind,snapshot_sha256,created_at FROM publication_versions WHERE project_id=? AND publication_id=? AND (? IS NULL OR revision<?) ORDER BY revision DESC LIMIT 30")
            .bind(project).bind(publication).bind(before).bind(before).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(rows)
    }
}

pub(crate) fn same_source(left: Option<&LayoutSource>, right: Option<&LayoutSource>) -> bool {
    let normalize = |source: Option<&LayoutSource>| {
        source.cloned().map(|mut source| {
            source.preview_sha256 = None;
            source
        })
    };
    normalize(left) == normalize(right)
}

async fn ensure_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
) -> Result<()> {
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM publications b JOIN projects p ON p.id=b.project_id WHERE b.id=? AND b.project_id=? AND p.deleted_at IS NULL)").bind(publication).bind(project).fetch_one(&mut **tx).await?;
    if exists {
        Ok(())
    } else {
        Err(AppError::new("NOT_FOUND", "出版物不存在或项目已归档"))
    }
}
