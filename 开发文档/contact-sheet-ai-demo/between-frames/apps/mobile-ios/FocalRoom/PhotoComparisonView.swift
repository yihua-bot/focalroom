import SwiftUI
import FocalRoomCore

struct AsyncPhotoPreview: View {
    let photoID: String
    let identity: String
    var fit = false
    var failureText = "点按重试"
    let load: () async throws -> UIImage
    @State private var image: UIImage?
    @State private var failed = false
    @State private var attempt = 0

    var body: some View {
        GeometryReader { geometry in
            ZStack {
                fit ? Color.black : Color(white: 0.09)
                if let image {
                    Image(uiImage: image).resizable().aspectRatio(contentMode: fit ? .fit : .fill)
                        .frame(width: geometry.size.width, height: geometry.size.height).clipped()
                        .accessibilityIdentifier("preview.loaded.\(photoID)")
                } else if failed {
                    Button { attempt += 1 } label: {
                        VStack(spacing: 8) { Image(systemName: "photo"); Text(failureText).font(.caption) }
                            .foregroundStyle(.secondary).frame(maxWidth: .infinity, maxHeight: .infinity)
                    }.buttonStyle(.plain).accessibilityLabel("重新读取照片预览")
                } else { ProgressView() }
            }.frame(width: geometry.size.width, height: geometry.size.height).clipped()
        }.contentShape(Rectangle())
        .task(id: "\(identity):\(attempt)") {
            image = nil; failed = false
            do {
                let value = try await load()
                try Task.checkCancellation()
                image = value
            } catch is CancellationError { }
            catch { if !Task.isCancelled { failed = true } }
        }
    }
}

struct PhotoViewport<Content: View>: View {
    @Binding var transform: PhotoTransform
    @ViewBuilder let content: () -> Content
    @State private var gestureOrigin: PhotoTransform?

    var body: some View {
        GeometryReader { geometry in
            content().frame(width: geometry.size.width, height: geometry.size.height)
                .scaleEffect(transform.zoom)
                .offset(x: transform.x * geometry.size.width, y: transform.y * geometry.size.height)
                // 双指缩放始终可用；单指平移仅在放大后启用——
                // 原大小时不挂拖动手势，把单指左右滑动完整让给页式翻页（TabView pager）。
                .gesture(MagnifyGesture().onChanged { value in
                    let origin = gestureOrigin ?? transform
                    gestureOrigin = origin
                    transform = origin.magnified(by: Double(value.magnification))
                }.onEnded { _ in gestureOrigin = nil })
                .modifier(PanGestureWhenZoomed(transform: $transform, gestureOrigin: $gestureOrigin))
                .onTapGesture(count: 2) { transform = transform.zoom == 1 ? PhotoTransform().magnified(by: 2) : PhotoTransform() }
        }.clipped().contentShape(Rectangle())
    }
}

/// 放大状态（zoom>1）下的单指平移；未放大时不挂手势，避免拦截翻页滑动。
private struct PanGestureWhenZoomed: ViewModifier {
    @Binding var transform: PhotoTransform
    @Binding var gestureOrigin: PhotoTransform?

    func body(content: Content) -> some View {
        if transform.zoom > 1.001 {
            content.gesture(DragGesture().onChanged { value in
                let origin = gestureOrigin ?? transform
                gestureOrigin = origin
                let size = UIScreen.main.bounds.size
                transform = origin.translated(x: Double(value.translation.width / max(1, size.width)),
                                              y: Double(value.translation.height / max(1, size.height)))
            }.onEnded { _ in gestureOrigin = nil })
        } else {
            content
        }
    }
}

struct ComparisonPhoto: Identifiable {
    let id: String
    let filename: String
}

struct PhotoComparisonView: View {
    let photos: [ComparisonPhoto]
    let initialPhotoID: String
    let sourceLabel: String
    let load: (String) async throws -> UIImage
    @Environment(\.dismiss) private var dismiss
    @State private var leftID = ""
    @State private var rightID = ""
    @State private var state = ComparisonState()

    var body: some View {
        NavigationStack {
            VStack(spacing: 8) {
                GeometryReader { geometry in
                    if geometry.size.width > geometry.size.height {
                        HStack(spacing: 6) { pane(.left); pane(.right) }
                    } else {
                        VStack(spacing: 6) { pane(.left); pane(.right) }
                    }
                }
                HStack {
                    Button { state.setLocked(!state.locked) } label: {
                        Label(state.locked ? "已锁定缩放与位置" : "独立调整缩放与位置", systemImage: state.locked ? "lock.fill" : "lock.open")
                            .font(.subheadline).frame(minHeight: 44)
                    }.accessibilityIdentifier("compare.lock").accessibilityValue(state.locked ? "已锁定" : "独立调整")
                    Spacer()
                    Button("复位") { state.reset() }.frame(minWidth: 44, minHeight: 44)
                        .accessibilityIdentifier("compare.reset")
                }.padding(.horizontal, 18)
                Text(sourceLabel).font(.caption).foregroundStyle(.secondary).padding(.bottom, 8)
            }
            .navigationTitle("双图比较").navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("完成") { dismiss() } }
                ToolbarItem(placement: .primaryAction) {
                    Button { swap(&leftID, &rightID); state.reset() } label: { Image(systemName: "arrow.left.arrow.right") }
                        .accessibilityLabel("交换照片").accessibilityIdentifier("compare.swap")
                }
            }
            .onAppear {
                leftID = photos.contains(where: { $0.id == initialPhotoID }) ? initialPhotoID : photos.first?.id ?? ""
                rightID = photos.first(where: { $0.id != leftID })?.id ?? ""
            }
        }
    }

    private func transform(_ side: ComparisonState.Side) -> Binding<PhotoTransform> {
        Binding(get: { side == .left ? state.left : state.right }, set: { state.update(side, transform: $0) })
    }

    private func pane(_ side: ComparisonState.Side) -> some View {
        let id = side == .left ? leftID : rightID
        let prefix = side == .left ? "compare.left" : "compare.right"
        return VStack(spacing: 4) {
            HStack {
                Menu {
                    ForEach(photos.filter { $0.id != (side == .left ? rightID : leftID) }) { photo in
                        Button(photo.filename) {
                            if side == .left { leftID = photo.id } else { rightID = photo.id }
                            state.reset()
                        }
                    }
                } label: {
                    Text(photos.first(where: { $0.id == id })?.filename ?? "选择照片").lineLimit(1)
                    Image(systemName: "chevron.down").font(.caption2)
                }.frame(minHeight: 44).contentShape(Rectangle()).accessibilityIdentifier(prefix + ".choose")
                Spacer()
                Text(String(format: "%.1f×", transform(side).wrappedValue.zoom))
                    .font(.caption.monospacedDigit()).accessibilityIdentifier(prefix + ".zoom")
                Button {
                    let value = transform(side).wrappedValue.magnified(by: 2)
                    state.update(side, transform: value)
                } label: { Image(systemName: "plus.magnifyingglass").frame(minWidth: 44, minHeight: 44) }
                    .accessibilityLabel("放大\(side == .left ? "第一张" : "第二张")")
                    .accessibilityIdentifier(prefix + ".zoomIn")
            }.font(.caption).padding(.horizontal, 12)
            PhotoViewport(transform: transform(side)) {
                AsyncPhotoPreview(photoID: id, identity: id, fit: true) { try await load(id) }
            }
        }.frame(maxWidth: .infinity, maxHeight: .infinity).background(Color(white: 0.055))
    }
}
