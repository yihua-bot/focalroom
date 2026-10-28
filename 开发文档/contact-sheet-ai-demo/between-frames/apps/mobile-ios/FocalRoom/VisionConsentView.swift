import SwiftUI
import FocalRoomCore

struct PreparedVision: Identifiable {
    struct Photo {
        let descriptor: PreviewDescriptor
        let filename: String
        let preview: VisionPreview
    }
    let id: UUID
    let generation: UUID
    let accountID: String
    let projectID: String
    let text: String
    let createdAt: Date
    let photos: [Photo]
}

struct VisionConsentView: View {
    let draft: PreparedVision
    let confirm: () -> Void
    let cancel: () -> Void

    var body: some View {
        NavigationStack {
            List {
                Section("本次照片观察") {
                    Text(draft.text)
                    Text("仅将下面 \(draft.photos.count) 张派生预览发送给 FocalRoom 与 DeepSeek，用于本次问题。每张最长边 1400 像素，已去除 GPS 和来源备注；原片不发送。")
                    Text("按账户实际额度计量；本次同意仅使用一次。取消不会发送照片。结果未确认时只查询原请求，不自动重发图片。")
                        .font(.footnote).foregroundStyle(.secondary)
                }
                Section("即将发送的照片") {
                    ForEach(draft.photos, id: \.descriptor.photo_id) { photo in
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
                    Button("同意并发送", action: confirm).accessibilityIdentifier("ai.vision.confirm")
                }
            }
        }
    }
}
