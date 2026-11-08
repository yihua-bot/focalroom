import SwiftUI
import FocalRoomCore

struct PreparedVision: Identifiable {
    struct Photo {
        let photoID: String
        let contentSHA: String
        let filename: String
        let preview: VisionPreview
    }
    let id: UUID
    let generation: UUID
    let accountID: String
    let project: ProjectReference
    let text: String
    let createdAt: Date
    let photos: [Photo]
    var detailInspection = false
}

struct VisionConsentView: View {
    let draft: PreparedVision
    var detailAvailable = false
    @State private var allowDetail = false
    let confirm: (PreparedVision) -> Void
    let cancel: () -> Void

    var body: some View {
        NavigationStack {
            List {
                Section("本次照片观察") {
                    Text(draft.text)
                    if allowDetail {
                        Text("仅将下面 \(draft.photos.count) 张照片的 JPEG 预览发送给 FocalRoom 与 DeepSeek，用于本次精查。最长边1400像素；原片保留本机。")
                    } else {
                        Text("仅将下面 \(draft.photos.count) 张照片的 JPEG 预览发送给 FocalRoom 与 DeepSeek，用于本次问题。发送前压缩至最长边 512 像素，并去除位置信息和来源备注；原片保留本机。")
                    }
                    if detailAvailable {
                        Toggle("允许所选照片精查（最多1400px）", isOn: $allowDetail)
                            .accessibilityIdentifier("ai.vision.detail")
                        if allowDetail {
                            Text("精查会请求更高细节，仍使用本次图像额度；原片不发送，预览不足1400px时不会放大补细节。")
                                .font(.footnote).foregroundStyle(.secondary)
                        }
                    }
                    Text("按账户实际额度计量；本次同意仅使用一次。取消不会发送照片。结果未确认时只查询原请求，不自动重发图片。")
                        .font(.footnote).foregroundStyle(.secondary)
                }
                Section("即将发送的照片") {
                    ForEach(draft.photos, id: \.photoID) { photo in
                        HStack {
                            if let image = UIImage(data: photo.preview.jpeg) {
                                Image(uiImage: image).resizable().scaledToFit().frame(width: 80, height: 80)
                            }
                            VStack(alignment: .leading) {
                                Text(photo.filename).font(.subheadline)
                                Text("\(photo.preview.width) × \(photo.preview.height) · sRGB JPEG")
                                    .font(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                }
            }
            .navigationTitle("照片发送授权").navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("取消", action: cancel) }
                ToolbarItem(placement: .confirmationAction) {
                    Button("同意并发送") { var approved = draft; approved.detailInspection = allowDetail && detailAvailable; confirm(approved) }.accessibilityIdentifier("ai.vision.confirm")
                }
            }
        }
    }
}
