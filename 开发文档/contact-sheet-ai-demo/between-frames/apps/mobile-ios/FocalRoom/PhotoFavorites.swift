import Foundation
import Photos
import FocalRoomCore

/// 将相册导入的原照片标记为系统收藏；不会取消用户已有的系统收藏。
enum PhotoFavorites {
    static func syncKeptPhoto(_ photo: LocalPhoto) async -> String? {
        guard let assetID = photo.phAssetID else {
            return "已在项目中保留。此照片来自文件或没有相册标识，无法自动同步到系统相册收藏。"
        }
        let status = await PHPhotoLibrary.requestAuthorization(for: .readWrite)
        guard status == .authorized || status == .limited else {
            return "已在项目中保留。请在 iPhone 设置中允许照片访问，才能同步到系统相册收藏。"
        }
        let fetch = PHAsset.fetchAssets(withLocalIdentifiers: [assetID], options: nil)
        guard let asset = fetch.firstObject else {
            return "已在项目中保留。系统相册中找不到这张照片，或当前授权不包含它，无法同步收藏。"
        }
        if asset.isFavorite { return nil }
        do {
            try await PHPhotoLibrary.shared().performChanges {
                PHAssetChangeRequest(for: asset).isFavorite = true
            }
            return nil
        } catch {
            return "已在项目中保留，但系统相册收藏同步失败。请检查照片权限后重试。"
        }
    }
}
