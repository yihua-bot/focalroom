//! 章节以稳定起始页引用定义范围；排版内容仍归页面和原出版历史管理。
use super::{AddPage, PublicationAction, TextUpdate, editing, invalid, layers, versions};
use crate::{
    command_audit::{encode, serialization},
    domain::{AppError, Result},
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sqlx::{Sqlite, Transaction};
use std::collections::{BTreeMap, BTreeSet};

pub const KEY: &str = "focalroomChaptersV1";
pub(super) fn valid_id(raw: &str) -> bool {
    if raw.len() != 36 {
        return false;
    }
    uuid::Uuid::parse_str(raw).is_ok_and(|id| {
        let bytes = id.as_bytes();
        id.is_nil()
            || id.as_u128() == u128::MAX
            || ((1..=8).contains(&(bytes[6] >> 4)) && bytes[8] & 0xc0 == 0x80)
    })
}

pub(super) fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub start_page_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetChapters {
    pub chapters: Vec<Chapter>,
    pub page_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateChapter {
    pub chapter: Chapter,
    pub pages: Vec<AddPage>,
    pub title: TextUpdate,
    pub start_on_right: bool,
}
pub fn read(layout: &versions::PublicationLayout) -> Result<Vec<Chapter>> {
    let spec = layers::spec(&layout.publication.spec_json)?;
    let Some(value) = spec.get(KEY) else {
        return Ok(Vec::new());
    };
    if layout.schema_version != 3 {
        return Err(invalid("章节版面格式无效，请恢复之前的版本"));
    }
    let chapters: Vec<Chapter> = serde_json::from_value(value.clone())
        .map_err(|_| invalid("章节信息不完整，请恢复之前的版本"))?;
    if chapters.len() > 64 || encode(&chapters)?.len() > 32768 {
        return Err(invalid("章节过多，请拆分为多个分册"));
    }
    let mut ids: BTreeSet<_> = layout
        .pages
        .iter()
        .flat_map(|p| {
            std::iter::once(p.id.clone())
                .chain(p.frames.iter().map(|f| f.id.clone()))
                .chain(p.texts.iter().map(|t| t.id.clone()))
        })
        .collect();
    let tree = layers::read(layout)?;
    ids.extend(
        tree.nodes
            .iter()
            .filter(|n| n.kind == "group")
            .map(|n| n.id.clone()),
    );
    let mut starts = BTreeSet::new();
    let mut ordered = Vec::new();
    let mut continuous_right = BTreeSet::new();
    for page in &layout.pages {
        for link in super::spreads::links(page)? {
            continuous_right.insert(link.right_page_id);
        }
    }
    for chapter in chapters {
        if !valid_id(&chapter.id)
            || !valid_id(&chapter.start_page_id)
            || !ids.insert(chapter.id.clone())
        {
            return Err(invalid("章节编号重复或无效，请重新打开出版物"));
        }
        if chapter.title.is_empty()
            || chapter.title.trim() != chapter.title
            || chapter.title.encode_utf16().count() > 120
            || chapter
                .title
                .chars()
                .any(|c| c.is_control() || matches!(c, '\u{feff}' | '\u{2028}' | '\u{2029}'))
        {
            return Err(invalid("章节名称须为1–120个字符，且不能包含换行"));
        }
        let index = layout
            .pages
            .iter()
            .position(|p| p.id == chapter.start_page_id && p.page_number > 0)
            .ok_or_else(|| invalid("请选择当前出版物的内页作为章节起始页"))?;
        if !starts.insert(chapter.start_page_id.clone())
            || continuous_right.contains(&chapter.start_page_id)
        {
            return Err(invalid("章节起始页不能重复，也不能拆开一张跨页照片"));
        }
        ordered.push((index, chapter));
    }
    ordered.sort_by_key(|entry| entry.0);
    Ok(ordered.into_iter().map(|entry| entry.1).collect())
}
fn owners(layout: &versions::PublicationLayout) -> Result<BTreeMap<String, Option<String>>> {
    let chapters = read(layout)?;
    let starts: BTreeMap<_, _> = chapters
        .iter()
        .map(|c| (c.start_page_id.as_str(), c.id.clone()))
        .collect();
    let mut current = None;
    Ok(layout
        .pages
        .iter()
        .filter(|p| p.page_number > 0)
        .map(|p| {
            if let Some(id) = starts.get(p.id.as_str()) {
                current = Some(id.clone());
            }
            (p.id.clone(), current.clone())
        })
        .collect())
}
pub(super) fn ordered_layout(
    layout: &versions::PublicationLayout,
    ids: &[String],
) -> Result<versions::PublicationLayout> {
    let rows: Vec<_> = layout
        .pages
        .iter()
        .map(|p| (p.id.clone(), p.page_number, p.side.clone()))
        .collect();
    let rows = editing::reordered_rows(&rows, ids)?;
    let mut next = layout.clone();
    next.pages = rows
        .into_iter()
        .map(|(id, number, side)| {
            let mut page = layout.pages.iter().find(|p| p.id == id).unwrap().clone();
            page.page_number = number;
            page.side = side;
            page
        })
        .collect();
    Ok(next)
}
pub(super) fn guard(
    layout: &versions::PublicationLayout,
    action: &PublicationAction,
) -> Result<()> {
    match action {
        PublicationAction::RemovePage(data)
            if read(layout)?
                .iter()
                .any(|c| c.start_page_id == data.page_id) =>
        {
            return Err(AppError::new(
                "CONFLICT",
                "这页是章节起始页，请先在章节管理中更换起始页或移除章节标记",
            ));
        }
        PublicationAction::ReorderPages(data) => {
            let after = ordered_layout(layout, &data.page_ids)?;
            if owners(layout)? != owners(&after)? && data.chapter_changes_confirmed != Some(true) {
                return Err(AppError::new(
                    "CONFLICT",
                    "移动这些页面会改变章节归属，请查看影响并确认",
                ));
            }
        }
        _ => {}
    }
    Ok(())
}
async fn write(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    value: Value,
) -> Result<()> {
    let raw = encode(&value)?;
    layers::spec(&raw)?;
    sqlx::query("UPDATE publications SET spec_json=? WHERE id=? AND project_id=?")
        .bind(raw)
        .bind(publication)
        .bind(project)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub(super) async fn clear(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
) -> Result<()> {
    let raw: String =
        sqlx::query_scalar("SELECT spec_json FROM publications WHERE id=? AND project_id=?")
            .bind(publication)
            .bind(project)
            .fetch_one(&mut **tx)
            .await?;
    let mut value = layers::spec(&raw)?;
    if value.as_object_mut().unwrap().remove(KEY).is_some() {
        write(tx, project, publication, value).await?;
    }
    Ok(())
}
pub(super) async fn set(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &SetChapters,
) -> Result<()> {
    if data.page_ids.is_empty() {
        return Err(invalid("请先添加页面，再管理章节"));
    }
    let original = versions::layout_tx(tx, project, publication).await?;
    let mut next = ordered_layout(&original, &data.page_ids)?;
    layers::put(&mut next, &layers::read(&original)?)?;
    let mut value = layers::spec(&next.publication.spec_json)?;
    value[KEY] = serde_json::to_value(&data.chapters).map_err(serialization)?;
    next.publication.spec_json = encode(&value)?;
    versions::validate_layout(&next)?;
    if original.pages != next.pages {
        editing::reorder(tx, publication, &data.page_ids).await?;
    }
    write(tx, project, publication, value).await
}
pub(super) async fn create(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &CreateChapter,
) -> Result<()> {
    let original = versions::layout_tx(tx, project, publication).await?;
    if original.pages.is_empty()
        || !(1..=2).contains(&data.pages.len())
        || data.pages.iter().any(|p| p.page_id == data.title.text_id)
        || data.title.kind != "text"
        || data.title.text != data.chapter.title
        || data.title.page_id != data.chapter.start_page_id
        || !data
            .pages
            .iter()
            .any(|p| p.page_id == data.chapter.start_page_id)
        || data
            .pages
            .get(1)
            .is_some_and(|p| p.after_page_id.as_deref() != Some(data.pages[0].page_id.as_str()))
    {
        return Err(invalid("章节起始页内容无效，请重新预览后创建"));
    }
    let mut chapters = read(&original)?;
    if chapters.iter().any(|c| c.id == data.chapter.id) {
        return Err(AppError::new("CONFLICT", "这章已存在，请重新打开出版物"));
    }
    for page in &data.pages {
        editing::apply(
            tx,
            project,
            publication,
            &PublicationAction::AddPage(page.clone()),
        )
        .await?;
    }
    editing::apply(
        tx,
        project,
        publication,
        &PublicationAction::AddText(data.title.clone()),
    )
    .await?;
    layers::reconcile(tx, project, publication).await?;
    let mut next = versions::layout_tx(tx, project, publication).await?;
    if data.start_on_right
        && !next
            .pages
            .iter()
            .any(|p| p.id == data.chapter.start_page_id && p.side == "right")
    {
        return Err(invalid("章节需从右页开始，请调整插入位置或加入一张空白页"));
    }
    let tree = layers::read(&next)?;
    layers::put(&mut next, &tree)?;
    chapters.push(data.chapter.clone());
    let mut value = layers::spec(&next.publication.spec_json)?;
    value[KEY] = serde_json::to_value(chapters).map_err(serialization)?;
    next.publication.spec_json = encode(&value)?;
    versions::validate_layout(&next)?;
    write(tx, project, publication, value).await
}
