//! 整章和关联页面共用一次纯预览与原子移除；不删除项目资产。
use super::{chapters, editing, invalid, layers, spreads, versions};
use crate::{
    command_audit::{encode, serialization},
    domain::{AppError, Result},
};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{Sqlite, Transaction};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemovePages {
    pub page_ids: Vec<String>,
    #[serde(deserialize_with = "nullable_id")]
    pub retain_blank_page_id: Option<String>,
    #[serde(deserialize_with = "nullable_id")]
    pub chapter_id: Option<String>,
}
fn nullable_id<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}
pub struct Plan {
    pub layout: versions::PublicationLayout,
    pub removed_page_ids: Vec<String>,
}
pub fn chapter_pages(layout: &versions::PublicationLayout, id: &str) -> Result<Vec<String>> {
    let chapters = chapters::read(layout)?;
    let index = chapters
        .iter()
        .position(|c| c.id == id)
        .ok_or_else(|| invalid("这章已变化，请重新查看章节"))?;
    let start = layout
        .pages
        .iter()
        .position(|p| p.id == chapters[index].start_page_id)
        .unwrap();
    let end = chapters
        .get(index + 1)
        .map(|c| {
            layout
                .pages
                .iter()
                .position(|p| p.id == c.start_page_id)
                .unwrap()
        })
        .unwrap_or(layout.pages.len());
    Ok(layout.pages[start..end]
        .iter()
        .filter(|p| p.page_number > 0)
        .map(|p| p.id.clone())
        .collect())
}
pub fn plan(layout: &versions::PublicationLayout, data: &RemovePages) -> Result<Plan> {
    let ids: BTreeSet<_> = data.page_ids.iter().collect();
    if data.page_ids.iter().any(|id| !chapters::valid_id(id))
        || data
            .chapter_id
            .as_ref()
            .is_some_and(|id| !chapters::valid_id(id))
        || ids.is_empty()
        || ids.len() != data.page_ids.len()
        || ids.len() > 512
        || data
            .retain_blank_page_id
            .as_ref()
            .is_some_and(|id| Some(id) != data.page_ids.first())
    {
        return Err(invalid("页面移除范围无效，请重新选择"));
    }
    let selected: Vec<_> = layout
        .pages
        .iter()
        .filter(|p| ids.contains(&p.id))
        .collect();
    if selected.len() != ids.len()
        || selected.iter().any(|p| p.page_number <= 0)
        || selected.iter().map(|p| &p.id).ne(data.page_ids.iter())
    {
        return Err(invalid("请选择当前出版物中按顺序排列的内页"));
    }
    let first = layout
        .pages
        .iter()
        .position(|p| p.id == data.page_ids[0])
        .unwrap();
    if layout.pages[first..first + selected.len()]
        .iter()
        .map(|p| &p.id)
        .ne(data.page_ids.iter())
    {
        return Err(invalid("只能一次移除一组连续页面"));
    }
    if let Some(chapter) = &data.chapter_id
        && chapter_pages(layout, chapter)? != data.page_ids
    {
        return Err(AppError::new("CONFLICT", "章节页面已变化，请重新预览整章"));
    }
    for page in &layout.pages {
        for link in spreads::links(page)? {
            if ids.contains(&page.id) != ids.contains(&link.right_page_id) {
                return Err(AppError::new(
                    "CONFLICT",
                    "请同时移除跨页照片所在的左右两页",
                ));
            }
        }
    }
    let chapters = chapters::read(layout)?;
    if chapters.iter().any(|c| {
        ids.contains(&c.start_page_id)
            && data.chapter_id.as_ref() != Some(&c.id)
            && data.retain_blank_page_id.as_ref() != Some(&c.start_page_id)
    }) {
        return Err(AppError::new(
            "CONFLICT",
            "所选页面包含其他章节的起始页，请先调整章节",
        ));
    }
    layers::guard_pages(layout, &layers::read(layout)?, &data.page_ids)?;
    if data.chapter_id.is_none()
        && selected.len() == 1
        && data.retain_blank_page_id.as_ref() == Some(&selected[0].id)
        && selected[0].frames.is_empty()
        && selected[0].texts.is_empty()
        && selected[0].layout_json.is_none()
    {
        return Err(invalid("所选页面已是空白，没有需要移除的内容"));
    }
    let mut next = layout.clone();
    next.pages
        .retain(|p| !ids.contains(&p.id) || data.retain_blank_page_id.as_ref() == Some(&p.id));
    if let Some(id) = &data.retain_blank_page_id {
        let page = next.pages.iter_mut().find(|p| &p.id == id).unwrap();
        page.frames.clear();
        page.texts.clear();
        page.layout_json = None;
    }
    if let Some(chapter) = &data.chapter_id {
        let mut spec = layers::spec(&next.publication.spec_json)?;
        spec[chapters::KEY] = serde_json::to_value(
            chapters
                .into_iter()
                .filter(|c| &c.id != chapter)
                .collect::<Vec<_>>(),
        )
        .map_err(serialization)?;
        next.publication.spec_json = encode(&spec)?;
    }
    let rows: Vec<_> = next
        .pages
        .iter()
        .map(|p| (p.id.clone(), p.page_number, p.side.clone()))
        .collect();
    let order: Vec<_> = next.pages.iter().map(|p| p.id.clone()).collect();
    let rows = editing::reordered_rows(&rows, &order)?;
    for (page, (_, number, side)) in next.pages.iter_mut().zip(rows) {
        page.page_number = number;
        page.side = side;
    }
    layers::reconcile_layout(&mut next)?;
    next.schema_version = if layers::has(&next.publication.spec_json)? {
        3
    } else if next.pages.iter().try_fold(false, |found, p| {
        spreads::links(p).map(|links| found || !links.is_empty())
    })? {
        2
    } else {
        1
    };
    versions::validate_layout(&next)?;
    if &next == layout {
        return Err(invalid("所选页面已是空白，没有需要移除的内容"));
    }
    Ok(Plan {
        removed_page_ids: data
            .page_ids
            .iter()
            .filter(|id| data.retain_blank_page_id.as_ref() != Some(id))
            .cloned()
            .collect(),
        layout: next,
    })
}
pub(super) async fn apply(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &RemovePages,
) -> Result<()> {
    let before = versions::layout_tx(tx, project, publication).await?;
    let plan = plan(&before, data)?;
    if let Some(id) = &data.retain_blank_page_id {
        sqlx::query("DELETE FROM publication_frames WHERE page_id=?")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        sqlx::query("DELETE FROM publication_texts WHERE page_id=?")
            .bind(id)
            .execute(&mut **tx)
            .await?;
        sqlx::query(
            "UPDATE publication_pages SET layout_json=NULL WHERE id=? AND publication_id=?",
        )
        .bind(id)
        .bind(publication)
        .execute(&mut **tx)
        .await?;
    }
    for id in &plan.removed_page_ids {
        sqlx::query("DELETE FROM publication_pages WHERE id=? AND publication_id=?")
            .bind(id)
            .bind(publication)
            .execute(&mut **tx)
            .await?;
    }
    let order = plan
        .layout
        .pages
        .iter()
        .map(|p| p.id.clone())
        .collect::<Vec<_>>();
    editing::reorder(tx, publication, &order).await?;
    sqlx::query("UPDATE publications SET spec_json=? WHERE id=? AND project_id=?")
        .bind(&plan.layout.publication.spec_json)
        .bind(publication)
        .bind(project)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
