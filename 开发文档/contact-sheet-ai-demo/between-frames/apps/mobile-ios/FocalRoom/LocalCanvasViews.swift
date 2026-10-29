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

    let projectID: String
    @ObservationIgnored private let library: LocalLibrary
    // undoStack 存逆操作（提交即撤销）；redoStack 存正向操作（提交即重做）。
    @ObservationIgnored private var undoStack: [LocalCanvasOperation] = []
    @ObservationIgnored private var redoStack: [LocalCanvasOperation] = []
    @ObservationIgnored private var active = true

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
        } catch { notice = "画布读取失败，请重试。" }
    }

    func shutdown() {
        active = false
    }

    private func perform(_ operation: LocalCanvasOperation) async {
        guard active, !busy else { return }
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
            notice = "项目已迁云，本机画布已冻结。"
        } catch {
            notice = "画布操作未生效，请重试。"
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
        } catch { notice = "撤销失败，请重试。" }
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
        } catch { notice = "重做失败，请重试。" }
    }

    func addPhotoNode(photoID: String) async {
        let z = (nodes.map(\.zOrder).max() ?? 0) + 1
        let column = nodes.count % 4
        let row = nodes.count / 4
        let node = LocalCanvasNode(id: UUID().uuidString.lowercased(), kind: .photo, photoID: photoID,
                                   x: Double(40 + column * 260), y: Double(40 + row * 220),
                                   width: 240, height: 180, zOrder: z)
        await perform(.createNode(node))
        selectedNodeID = node.id
    }

    func addNote() async {
        let z = (nodes.map(\.zOrder).max() ?? 0) + 1
        let node = LocalCanvasNode(id: UUID().uuidString.lowercased(), kind: .note, text: "新便签",
                                   x: Double(60 + nodes.count * 24), y: Double(60 + nodes.count * 18),
                                   width: 200, height: 120, zOrder: z)
        await perform(.createNode(node))
        selectedNodeID = node.id
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
    @State private var addPhotoSheet = false
    @State private var noteEditor: LocalCanvasNode?
    @State private var noteDraft = ""
    @State private var annotationTarget: String?
    @State private var annotationDraft = ""
    @State private var confirmDelete = false

    var body: some View {
        GeometryReader { _ in
            ZStack {
                Color(white: 0.07)
                ForEach(model.nodes.sorted { ($0.zOrder, $0.id) < ($1.zOrder, $1.id) }) { node in
                    nodeView(node)
                        .position(x: CGFloat(node.x) * model.zoom + model.panOffset.width + 120,
                                  y: CGFloat(node.y) * model.zoom + model.panOffset.height + 120)
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
        }
        .navigationTitle("本机画布").navigationBarTitleDisplayMode(.inline)
        .toolbar { toolbarContent }
        .task { await model.load() }
        .onDisappear { model.shutdown() }
        .sheet(isPresented: $addPhotoSheet) { photoPicker }
        .sheet(item: $noteEditor, onDismiss: { noteDraft = "" }) { node in
            noteEditorSheet(node)
        }
        .sheet(item: Binding(get: { annotationTarget.map { AnnotationTarget(nodeID: $0) } },
                             set: { annotationTarget = $0?.nodeID }),
               onDismiss: { annotationDraft = "" }) { target in
            annotationSheet(target)
        }
        .confirmationDialog("删除所选节点", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button("删除", role: .destructive) { Task { await model.removeSelected() } }
        } message: {
            Text("节点及其批注一并删除，可撤销。")
        }
        .alert("提示", isPresented: Binding(get: { model.notice != nil }, set: { if !$0 { model.notice = nil } })) {
            Button("知道了", role: .cancel) { model.notice = nil }
        } message: { Text(model.notice ?? "") }
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
            Button("编辑文本") { noteDraft = node.text ?? ""; noteEditor = node }
            Button("添加批注") { annotationDraft = ""; annotationTarget = node.id }
            Button("移到最上层") { Task { await model.bringToFront(nodeID: node.id) } }
            Button("删除", role: .destructive) { model.selectedNodeID = node.id; confirmDelete = true }
        }
        .accessibilityIdentifier("local.canvas.node.\(node.id)")
        .accessibilityLabel(node.kind == .photo ? "画布照片" : "便签：\(node.text ?? "")")
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
            Button { addPhotoSheet = true } label: { Image(systemName: "photo.badge.plus") }
                .disabled(model.busy)
                .accessibilityLabel("添加照片").accessibilityIdentifier("canvas.add.photo")
            Button { Task { await model.addNote() } } label: { Image(systemName: "note.text.badge.plus") }
                .disabled(model.busy)
                .accessibilityLabel("添加便签").accessibilityIdentifier("canvas.add.note")
            Button { Task { await model.undo() } } label: { Image(systemName: "arrow.uturn.backward") }
                .disabled(!model.canUndo || model.busy)
                .accessibilityLabel("撤销").accessibilityIdentifier("canvas.undo")
            Button { Task { await model.redo() } } label: { Image(systemName: "arrow.uturn.forward") }
                .disabled(!model.canRedo || model.busy)
                .accessibilityLabel("重做").accessibilityIdentifier("canvas.redo")
            if model.selectedNode != nil {
                Button { confirmDelete = true } label: { Image(systemName: "trash") }
                    .disabled(model.busy)
                    .accessibilityLabel("删除所选").accessibilityIdentifier("canvas.delete")
            }
        }
    }

    private var photoPicker: some View {
        NavigationStack {
            List(library.projects.first { $0.id == model.projectID }?.photos ?? []) { photo in
                Button {
                    addPhotoSheet = false
                    Task { await model.addPhotoNode(photoID: photo.id) }
                } label: {
                    HStack {
                        Image(systemName: "photo").foregroundStyle(.secondary)
                        Text(photo.filename).lineLimit(1)
                        Spacer()
                        Text(photo.selection.title).font(.caption).foregroundStyle(.secondary)
                    }
                }
                .accessibilityIdentifier("canvas.pick.photo.\(photo.id)")
            }
            .navigationTitle("选择照片").navigationBarTitleDisplayMode(.inline)
            .toolbar { Button("取消") { addPhotoSheet = false } }
        }
        .presentationDetents([.medium, .large])
    }

    private func noteEditorSheet(_ node: LocalCanvasNode) -> some View {
        NavigationStack {
            TextEditor(text: $noteDraft)
                .padding()
                .navigationTitle("编辑便签").navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button("取消") { noteEditor = nil } }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("保存") {
                            let id = node.id, text = noteDraft
                            noteEditor = nil
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
                    ToolbarItem(placement: .cancellationAction) { Button("取消") { annotationTarget = nil } }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("添加") {
                            let nodeID = target.nodeID, body = annotationDraft
                            annotationTarget = nil
                            Task { await model.addAnnotation(to: nodeID, body: body) }
                        }
                        .disabled(annotationDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ||
                                  annotationDraft.count > LocalCanvas.textLimit)
                    }
                }
        }
        .presentationDetents([.medium])
    }

    private struct AnnotationTarget: Identifiable {
        let nodeID: String
        var id: String { nodeID }
    }
}
