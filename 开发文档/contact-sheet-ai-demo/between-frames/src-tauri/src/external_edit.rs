//! G5 Round-trip：外部编辑器回流检测与匹配。
//!
//! Watcher 监控指定目录，发现 `.focalroom-manifest.json` 后：
//! 1. 解析 manifest，校验 SHA-256
//! 2. 按 assetId / externalPhotoId 匹配到已有资产
//! 3. 在已有 EditVariant 下创建不可变 Rendition
//! 4. 更新 transfer_jobs 状态机

use crate::{
    db::Store,
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalManifest {
    pub schema_version: i32,
    pub transfer_batch_id: String,
    pub provider: String,
    pub catalog_fingerprint: String,
    pub issued_at: Option<String>,
    pub items: Vec<ManifestItem>,
}

#[derive(Debug, Clone)]
pub struct RenditionInput {
    pub variant_id: String,
    pub sha256: String,
    pub file_path: String,
    pub width: i64,
    pub height: i64,
    pub bit_depth: i64,
    pub icc_digest: Option<String>,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub file_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestItem {
    pub asset_id: Option<String>,
    pub source_sha256: Option<String>,
    pub external_photo_id: String,
    pub external_variant_id: Option<String>,
    pub render_filename: String,
    pub render_sha256: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub bit_depth: Option<i64>,
    pub render_icc_sha256: Option<String>,
    pub media_format: Option<String>,
}

/// 解析 manifest 文件。
pub fn parse_manifest(path: &Path) -> Result<ExternalManifest> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AppError::new("FILE_IO", format!("读取 manifest 失败：{e}")))?;
    let manifest: ExternalManifest = serde_json::from_str(&content)
        .map_err(|e| AppError::new("VALIDATION_ERROR", format!("manifest 格式错误：{e}")))?;
    if manifest.schema_version != 3 {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            format!("manifest 版本 {} 不受支持", manifest.schema_version),
        ));
    }
    if manifest.items.is_empty() {
        return Err(AppError::new("VALIDATION_ERROR", "manifest 不含任何条目"));
    }
    Ok(manifest)
}

/// 匹配策略：按 assetId → external_edit_links → 手动。
pub enum MatchResult {
    /// 精确匹配到已有资产
    Matched {
        asset_id: String,
        variant_id: String,
    },
    /// 多个候选，需要人工确认
    Ambiguous { candidates: Vec<String> },
    /// 无法匹配
    Unmatched,
}

impl Store {
    /// 按 manifest item 匹配资产。
    pub async fn match_manifest_item(&self, item: &ManifestItem) -> MatchResult {
        // 策略 1：manifest 携带 assetId
        if let Some(asset_id) = &item.asset_id {
            let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM source_assets WHERE id=?")
                .bind(asset_id)
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0);
            if exists > 0 {
                // 找到或创建 variant
                let variant: Option<String> = sqlx::query_scalar(
                    "SELECT id FROM edit_variants WHERE asset_id=? AND is_current=1",
                )
                .bind(asset_id)
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);
                if let Some(vid) = variant {
                    return MatchResult::Matched {
                        asset_id: asset_id.clone(),
                        variant_id: vid,
                    };
                }
            }
        }

        // 策略 2：按 external_photo_id 查 external_edit_links
        let links: Vec<String> = sqlx::query_scalar(
            "SELECT asset_id FROM external_edit_links WHERE external_photo_id=? AND state='active'",
        )
        .bind(&item.external_photo_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        match links.len() {
            0 => MatchResult::Unmatched,
            1 => {
                let asset_id = links[0].clone();
                let variant: Option<String> = sqlx::query_scalar(
                    "SELECT id FROM edit_variants WHERE asset_id=? AND is_current=1",
                )
                .bind(&asset_id)
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);
                if let Some(vid) = variant {
                    MatchResult::Matched {
                        asset_id,
                        variant_id: vid,
                    }
                } else {
                    MatchResult::Unmatched
                }
            }
            _ => MatchResult::Ambiguous { candidates: links },
        }
    }

    /// 在匹配到的 variant 下创建 Rendition（不可变渲染记录）。
    pub async fn record_rendition(&self, rendition: &RenditionInput) -> Result<String> {
        let id = domain::id();
        let now = domain::timestamp();
        sqlx::query(
            "INSERT OR IGNORE INTO renditions(id,variant_id,sha256,width,height,bit_depth,icc_digest,source_type,source_ref,file_path,file_size,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(&id).bind(&rendition.variant_id).bind(&rendition.sha256)
        .bind(rendition.width).bind(rendition.height).bind(rendition.bit_depth)
        .bind(&rendition.icc_digest).bind(&rendition.source_type).bind(&rendition.source_ref)
        .bind(&rendition.file_path).bind(rendition.file_size).bind(&now)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(id)
    }

    /// 创建外发批次（FocalRoom → Lightroom/C1）。
    pub async fn create_outgoing_transfer(
        &self,
        project_id: &str,
        provider: &str,
        asset_ids: &[String],
    ) -> Result<String> {
        let id = domain::id();
        let now = domain::timestamp();
        let items = serde_json::to_string(asset_ids)
            .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
        sqlx::query(
            "INSERT INTO transfer_jobs(id,project_id,direction,provider,state,items_json,created_at,updated_at) VALUES(?,?,'outgoing',?,'prepared',?,?,?)",
        )
        .bind(&id).bind(project_id).bind(provider).bind(&items).bind(&now).bind(&now)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parses_valid_json() {
        let json = r#"{
            "schemaVersion": 3,
            "transferBatchId": "test-batch",
            "provider": "LIGHTROOM_CLASSIC",
            "catalogFingerprint": "test-catalog",
            "items": [{
                "externalPhotoId": "lr-123",
                "renderFilename": "photo.tif"
            }]
        }"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        std::fs::write(&path, json).unwrap();
        let manifest = parse_manifest(&path).unwrap();
        assert_eq!(manifest.provider, "LIGHTROOM_CLASSIC");
        assert_eq!(manifest.items.len(), 1);
        assert_eq!(manifest.items[0].external_photo_id, "lr-123");
    }

    #[test]
    fn manifest_rejects_wrong_version() {
        let json = r#"{
            "schemaVersion": 2,
            "transferBatchId": "x",
            "provider": "LIGHTROOM_CLASSIC",
            "catalogFingerprint": "x",
            "items": [{"externalPhotoId": "x", "renderFilename": "x"}]
        }"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        std::fs::write(&path, json).unwrap();
        assert!(parse_manifest(&path).is_err());
    }

    #[test]
    fn manifest_rejects_empty_items() {
        let json = r#"{
            "schemaVersion": 3,
            "transferBatchId": "x",
            "provider": "CAPTURE_ONE",
            "catalogFingerprint": "x",
            "items": []
        }"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        std::fs::write(&path, json).unwrap();
        assert!(parse_manifest(&path).is_err());
    }
}
