import Foundation
import Photos
import FocalRoomCore

/// 「保留」同步系统相册收藏：App 内保留 ↔ 系统相册星标。
/// 仅对从相册导入的照片生效（携带 PHAsset 身份）；文件导入的照片无相册身份，静默跳过。
/// 用户未授权相册时静默跳过——选片主流程不因同步失败受阻。
enum PhotoFavorites {
    /// 把照片的系统收藏状态同步为与 App 选片一致：KEEP ⇒ 收藏，其它 ⇒ 取消收藏。
    static func sync(photos: [LocalPhoto], to selection: Selection) async {
        let assetIDs = photos.compactMap(\.phAssetID)
        guard !assetIDs.isEmpty else { return }
        let favorite = selection == .KEEP
        let status = await PHPhotoLibrary.requestAuthorization(for: .readWrite)
        guard status == .authorized || status == .limited else { return }
        do {
            try await PHPhotoLibrary.shared().performChanges {
                let fetch = PHAsset.fetchAssets(withLocalIdentifiers: assetIDs, options: nil)
                fetch.enumerateObjects { asset, _, _ in
                    let request = PHAssetChangeRequest(for: asset)
                    request.isFavorite = favorite
                }
            }
        } catch {
            // 收藏同步是体验增强：失败不弹错、不回滚选片，下次选片重试。
        }
    }
}
