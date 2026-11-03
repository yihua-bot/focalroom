import SwiftUI
import FocalRoomCore

// 本机画布：平移/缩放视口、拖动/缩放节点、照片与便签、批注、撤销/重做。
// 编辑经 LocalLibrary.applyCanvasOperation 原子提交；返回的逆操作驱动撤销栈。
@MainActor @Observable
final class LocalCanvasModel {
    private(set) var nodes: [LocalCanvasNode] = []
    private(set) var annotations: [LocalCanvasAnnotation] = []
    var selectedNodeID: String?
    private(set) var canUndo = false
    private(set) var canRedo = false
    private(set) var busy = false
    var notice: String?
    var panOffset: CGSize = .zero
    var zoom: CGFloat = 1
    // 由视图写入、仅在放置新节点时读取的可视尺寸；界面不依赖它，故不进观察范围。
    @ObservationIgnored var viewport: CGSize = .zero

    let projectID: String
    // 视图把节点坐标映射到屏幕中心时的固定边距；放置新节点须与视图用同一个值，否则落点会算偏。
    static let canvasEdgeInset: Double = 120
    @ObservationIgnored private let library: LocalLibrary
    // undoStack 存逆操作（提交即撤销）；redoStack 存正向操作（提交即重做）。
    @ObservationIgnored private var undoStack: [LocalCanvasOperation] = []
    @ObservationIgnored private var redoStack: [LocalCanvasOperation] = []

    init(library: LocalLibrary, projectID: String) {
        self.library = library
        self.projectID = projectID
    }

    var selectedNode: LocalCanvasNode? {
        nodes.first { $0.id == selectedNodeID }
    }

    func load() async {
        do {
            let canvas = try await library.canvas(projectID: projectID)
            nodes = canvas?.nodes ?? []
            annotations = canvas?.annotations ?? []
        } catch { notice = String(localized: "画布读取失败，请重试。") }
    }

    func recordAIAddition(_ nodeIDs: [String]) async {
        undoStack.append(.removeNodes(ids: nodeIDs))
        redoStack.removeAll()
        canUndo = true
        canRedo = false
        await load()
    }

    private func perform(_ operation: LocalCanvasOperation) async {
        guard !busy else { return }
        busy = true
        defer { busy = false }
        do {
            let inverse = try await library.applyCanvasOperation(projectID: projectID, operation)
            undoStack.append(inverse)
            redoStack.removeAll()
            canUndo = true
            canRedo = false
            await load()
        } catch LocalLibraryError.projectFrozen {
            notice = String(localized: "项目已迁云，本机画布已冻结。")
        } catch {
            notice = String(localized: "画布操作未生效，请重试。")
        }
    }

    func undo() async {
        guard let inverse = undoStack.last, !busy else { return }
        busy = true
        defer { busy = false }
        do {
            let redoOperation = try await library.applyCanvasOperation(projectID: projectID, inverse)
            undoStack.removeLast()
            redoStack.append(redoOperation)
            canUndo = !undoStack.isEmpty
            canRedo = true
            await load()
        } catch { notice = String(localized: "撤销失败，请重试。") }
    }

    func redo() async {
        guard let forward = redoStack.last, !busy else { return }
        busy = true
        defer { busy = false }
        do {
            let inverse = try await library.applyCanvasOperation(projectID: projectID, forward)
            redoStack.removeLast()
            undoStack.append(inverse)
            canUndo = true
            canRedo = !redoStack.isEmpty
            await load()
        } catch { notice = String(localized: "重做失败，请重试。") }
    }

    func addPhotoNode(photoID: String) async {
        let origin = visibleOrigin(width: 240, height: 180)
        let node = LocalCanvasNode(id: UUID().uuidString.lowercased(), kind: .photo, photoID: photoID,
                                   x: origin.x, y: origin.y, width: 240, height: 180, zOrder: nextZOrder)
        await perform(.createNode(node))
        selectedNodeID = node.id
    }

    func addNote() async {
        let origin = visibleOrigin(width: 200, height: 120)
        let node = LocalCanvasNode(id: UUID().uuidString.lowercased(), kind: .note, text: String(localized: "新便签"),
                                   x: origin.x, y: origin.y, width: 200, height: 120, zOrder: nextZOrder)
        await perform(.createNode(node))
        selectedNodeID = node.id
    }

    private var nextZOrder: Int { (nodes.map(\.zOrder).max() ?? 0) + 1 }

    /// 画布坐标 → 视图中心点；新节点落点的反向计算以它为准。
    func center(of node: LocalCanvasNode) -> CGPoint {
        CGPoint(x: CGFloat(node.x) * zoom + panOffset.width + CGFloat(Self.canvasEdgeInset),
                y: CGFloat(node.y) * zoom + panOffset.height + CGFloat(Self.canvasEdgeInset))
    }

    /// 新节点必须落在当前可视矩形内：画布按 `坐标 × zoom + panOffset + 120` 映射到屏幕中心，
    /// 固定网格在 iPhone 宽度上第二列就整格出界，用户点了照片却什么都看不到。
    private func visibleOrigin(width: Double, height: Double) -> (x: Double, y: Double) {
        let scale = Double(zoom)
        let marginX = width * scale / 2 + 12
        let marginY = height * scale / 2 + 12
        let viewWidth = max(Double(viewport.width), marginX * 2 + 24)
        let viewHeight = max(Double(viewport.height), marginY * 2 + 24)
        let panX = Double(panOffset.width) + Self.canvasEdgeInset
        let panY = Double(panOffset.height) + Self.canvasEdgeInset
        let minX = (marginX - panX) / scale
        let maxX = (viewWidth - marginX - panX) / scale
        let minY = (marginY - panY) / scale
        let maxY = (viewHeight - marginY - panY) / scale
        let step = nodes.count % 5
        let tier = nodes.count / 5
        let fx = step == 0 ? 0 : Double(step) / 5
        let fy = tier == 0 ? 0 : 0.35 + Double(tier - 1) / 5
        return (minX + (maxX - minX) * fx, minY + (maxY - minY) * min(fy, 1))
    }

    func move(nodeID: String, x: Double, y: Double) async {
        await perform(.moveNode(id: nodeID, x: x, y: y))
    }

    func updateNoteText(nodeID: String, text: String) async {
        await perform(.updateNoteText(id: nodeID, text: text))
    }

    func removeSelected() async {
        guard let id = selectedNodeID else { return }
        await perform(.removeNode(id: id))
        selectedNodeID = nil
    }

    func bringToFront(nodeID: String) async {
        await perform(.bringToFront(id: nodeID))
    }

    func addAnnotation(to nodeID: String?, body: String) async {
        await perform(.createAnnotation(LocalCanvasAnnotation(id: UUID().uuidString.lowercased(),
                                                              nodeID: nodeID, body: body)))
    }

    func annotations(for nodeID: String) -> [LocalCanvasAnnotation] {
        annotations.filter { $0.nodeID == nodeID }
    }
}

struct LocalCanvasView: View {
    @Bindable var model: LocalCanvasModel
    let library: LocalLibraryModel
    @State private var dragOffsets: [String: CGSize] = [:]
    @State private var basePan: CGSize = .zero
    @State private var activeSheet: CanvasSheet?
    @State private var noteDraft = ""
    @State private var annotationDraft = ""
    @State private var confirmDelete = false

    var body: some View {
        GeometryReader { proxy in
            ZStack {
                Color(white: 0.07)
                ForEach(model.nodes.sorted { ($0.zOrder, $0.id) < ($1.zOrder, $1.id) }) { node in
                    nodeView(node).position(model.center(of: node))
                }
                if model.nodes.isEmpty {
                    VStack(spacing: 10) {
                        Image(systemName: "rectangle.dashed").font(.largeTitle).foregroundStyle(.secondary)
                        Text("从右上角添加照片或便签开始布局").font(.footnote).foregroundStyle(.secondary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .allowsHitTesting(false)
                }
            }
            .clipped()
            .contentShape(Rectangle())
            .onTapGesture { model.selectedNodeID = nil }
            .gesture(panGesture)
            .onAppear { model.viewport = proxy.size }
            .onChange(of: proxy.size) { _, size in model.viewport = size }
        }
        .navigationTitle("本机画布").navigationBarTitleDisplayMode(.inline)
        .toolbar { toolbarContent }
        .task(id: model.projectID) { await model.load() }
        .sheet(item: $activeSheet, onDismiss: { noteDraft = ""; annotationDraft = "" }) { sheet in
            switch sheet {
            case .addPhoto: photoPicker
            case .editNote(let node): noteEditorSheet(node)
            case .annotate(let nodeID): annotationSheet(AnnotationTarget(nodeID: nodeID))
            }
        }
        .confirmationDialog("删除所选节点", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button("删除", role: .destructive) { Task { await model.removeSelected() } }
        } message: {
            Text("节点及其批注一并删除，可撤销。")
        }
        .noticeAlert(model.notice, onDismiss: { model.notice = nil })
    }

    private var panGesture: some Gesture {
        DragGesture(minimumDistance: 12)
            .onChanged { value in
                model.panOffset = CGSize(width: basePan.width + value.translation.width,
                                         height: basePan.height + value.translation.height)
            }
            .onEnded { _ in basePan = model.panOffset }
    }

    @ViewBuilder
    private func nodeView(_ node: LocalCanvasNode) -> some View {
        let selected = model.selectedNodeID == node.id
        Group {
            if node.kind == .photo, let photoID = node.photoID {
                AsyncPhotoPreview(photoID: photoID, identity: photoID, fit: true) {
                    try await library.image(projectID: model.projectID, photoID: photoID)
                }
            } else {
                RoundedRectangle(cornerRadius: 8).fill(Color.yellow.opacity(0.28))
                    .overlay {
                        Text(node.text ?? "").font(.callout).padding(8).minimumScaleFactor(0.4)
                            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    }
            }
        }
        .frame(width: CGFloat(node.width) * model.zoom, height: CGFloat(node.height) * model.zoom)
        .overlay(RoundedRectangle(cornerRadius: 8).stroke(selected ? Color.accentColor : .clear, lineWidth: 3))
        .overlay(alignment: .bottom) {
            let notes = model.annotations(for: node.id)
            if !notes.isEmpty {
                Text(notes.map(\.body).joined(separator: "；"))
                    .font(.caption2).lineLimit(2)
                    .padding(4).background(.black.opacity(0.6), in: RoundedRectangle(cornerRadius: 4))
                    .padding(4)
            }
        }
        .contentShape(Rectangle())
        .onTapGesture { model.selectedNodeID = node.id }
        .highPriorityGesture(nodeDrag(node))
        .contextMenu {
            Button("编辑文本") { noteDraft = node.text ?? ""; activeSheet = .editNote(node) }
            Button("添加批注") { annotationDraft = ""; activeSheet = .annotate(node.id) }
            Button("移到最上层") { Task { await model.bringToFront(nodeID: node.id) } }
            Button("删除", role: .destructive) { model.selectedNodeID = node.id; confirmDelete = true }
        }
        // 合并为一个可访问元素：不加这一层时容器本身不是无障碍元素，
        // 下面的标识会被丢弃（只有标签落到内部文本上），VoiceOver 与自动化都点不到"节点"。
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("local.canvas.node.\(node.id)")
        .accessibilityLabel(node.kind == .photo ? String(localized: "画布照片") : String(localized: "便签：\(node.text ?? "")"))
    }

    private func nodeDrag(_ node: LocalCanvasNode) -> some Gesture {
        DragGesture(minimumDistance: 8)
            .onChanged { value in dragOffsets[node.id] = value.translation }
            .onEnded { value in
                let x = node.x + Double(value.translation.width) / Double(model.zoom)
                let y = node.y + Double(value.translation.height) / Double(model.zoom)
                Task { await model.move(nodeID: node.id, x: x, y: y) }
            }
    }

    @ToolbarContentBuilder
    private var toolbarContent: some ToolbarContent {
        ToolbarItemGroup(placement: .primaryAction) {
            Button { activeSheet = .addPhoto } label: { Image(systemName: "photo.badge.plus") }
                .disabled(model.busy)
                .accessibilityLabel(String(localized: "添加照片")).accessibilityIdentifier("canvas.add.photo")
            Button { Task { await model.addNote() } } label: { Image(systemName: "note.text.badge.plus") }
                .disabled(model.busy)
                .accessibilityLabel(String(localized: "添加便签")).accessibilityIdentifier("canvas.add.note")
            Button { Task { await model.undo() } } label: { Image(systemName: "arrow.uturn.backward") }
                .disabled(!model.canUndo || model.busy)
                .accessibilityLabel(String(localized: "撤销")).accessibilityIdentifier("canvas.undo")
            Button { Task { await model.redo() } } label: { Image(systemName: "arrow.uturn.forward") }
                .disabled(!model.canRedo || model.busy)
                .accessibilityLabel(String(localized: "重做")).accessibilityIdentifier("canvas.redo")
            if model.selectedNode != nil {
                Button { confirmDelete = true } label: { Image(systemName: "trash") }
                    .disabled(model.busy)
                    .accessibilityLabel(String(localized: "删除所选")).accessibilityIdentifier("canvas.delete")
            }
        }
    }

    private var photoPicker: some View {
        NavigationStack {
            // 本机图库尚未刷新时这里会是空表：空表必须说明原因，否则像按钮失灵。
            let photos = library.projects.first { $0.id == model.projectID }?.photos ?? []
            Group {
                if photos.isEmpty {
                    ContentUnavailableView("这个项目还没有照片", systemImage: "photo",
                                           description: Text("请先在相册导入照片，再添加到画布。"))
                        .accessibilityIdentifier("canvas.pick.empty")
                } else {
                    List(photos) { photo in
                        Button {
                            activeSheet = nil
                            Task { await model.addPhotoNode(photoID: photo.id) }
                        } label: {
                            HStack {
                                Image(systemName: "photo").foregroundStyle(.secondary)
                                Text(photo.filename).lineLimit(1)
                                Spacer()
                                Text(LocalizedStringKey(photo.selection.title)).font(.caption).foregroundStyle(.secondary)
                            }
                        }
                        .accessibilityIdentifier("canvas.pick.photo.\(photo.id)")
                    }
                }
            }
            .navigationTitle("选择照片").navigationBarTitleDisplayMode(.inline)
            .toolbar { Button("取消") { activeSheet = nil } }
        }
        .presentationDetents([.medium, .large])
    }

    private func noteEditorSheet(_ node: LocalCanvasNode) -> some View {
        NavigationStack {
            TextEditor(text: $noteDraft)
                .padding()
                .navigationTitle("编辑便签").navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button("取消") { activeSheet = nil } }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("保存") {
                            let id = node.id, text = noteDraft
                            activeSheet = nil
                            Task { await model.updateNoteText(nodeID: id, text: text) }
                        }
                        .disabled(noteDraft.count > LocalCanvas.textLimit)
                    }
                }
        }
        .presentationDetents([.medium])
    }

    private func annotationSheet(_ target: AnnotationTarget) -> some View {
        NavigationStack {
            TextEditor(text: $annotationDraft)
                .padding()
                .navigationTitle("批注").navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button("取消") { activeSheet = nil } }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("添加") {
                            let nodeID = target.nodeID, body = annotationDraft
                            activeSheet = nil
                            Task { await model.addAnnotation(to: nodeID, body: body) }
                        }
                        .disabled(annotationDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ||
                                  annotationDraft.count > LocalCanvas.textLimit)
                    }
                }
        }
        .presentationDetents([.medium])
    }

    private struct AnnotationTarget {
        let nodeID: String
    }

    // 单一枚举驱动的表单：同级多个 .sheet 修饰符在 iOS 上只有最后一个生效，选照片表单曾被顶掉。
    private enum CanvasSheet: Identifiable {
        case addPhoto
        case editNote(LocalCanvasNode)
        case annotate(String)

        var id: String {
            switch self {
            case .addPhoto: return "add-photo"
            case .editNote(let node): return "note-\(node.id)"
            case .annotate(let nodeID): return "annotate-\(nodeID)"
            }
        }
    }
}
