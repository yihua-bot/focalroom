import SwiftUI
import FocalRoomCore

struct CommentsView: View {
    let model: AppModel

    var body: some View {
        VStack(spacing: 0) {
            ProjectContext(model: model)
            if model.snapshot == nil {
                EmptyWorkspace(title: "选择一个项目", detail: "查看项目中的评论与回复。", symbol: "bubble.left.and.bubble.right")
            } else if model.snapshot?.comments.isEmpty == true {
                EmptyWorkspace(title: "还没有评论", detail: "这里会显示当前项目已同步的评论。", symbol: "bubble.left")
            } else {
                List(model.snapshot?.comments ?? []) { comment in
                    VStack(alignment: .leading, spacing: 10) {
                        HStack {
                            Text(comment.reviewer).font(.subheadline.weight(.semibold))
                            Spacer()
                            Text(comment.state == "RESOLVED" ? "已解决" : "待处理").font(.caption).foregroundStyle(.secondary)
                        }
                        Text(comment.body).font(.body).textSelection(.enabled)
                        if let photoID = comment.anchor.photo_id,
                           let photo = model.snapshot?.snapshot.photos.first(where: { $0.id == photoID }) {
                            NavigationLink { PhotoViewer(model: model, projectID: model.snapshot?.project_id ?? "", photoID: photoID) } label: {
                                Label(photo.filename, systemImage: "photo").font(.caption)
                            }
                        }
                        if comment.parent_id != nil { Text("线程回复").font(.caption2).foregroundStyle(.secondary) }
                    }.padding(.vertical, 8)
                }.listStyle(.plain).refreshable { await model.reloadSnapshot() }
            }
        }.navigationTitle("消息").navigationBarTitleDisplayMode(.inline)
    }
}

struct BoardsView: View {
    let model: AppModel
    @State private var boards: [Board] = []
    @State private var loading = false
    @State private var error: String?

    var body: some View {
        VStack(spacing: 0) {
            ProjectContext(model: model)
            if model.selectedProjectID == nil {
                EmptyWorkspace(title: "选择一个项目", detail: "阅读已同步的项目画布。", symbol: "rectangle.3.group")
            } else if model.offline {
                EmptyWorkspace(title: "画布需要联网读取", detail: "当前可在项目中查看已缓存的图库资料。", symbol: "wifi.slash")
            } else if loading { ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity) }
            else if let error {
                EmptyWorkspace(title: "暂时无法读取", detail: error)
                Button("重试") { Task { await load() } }.padding()
            } else {
                List {
                    if boards.isEmpty {
                        EmptyWorkspace(title: "还没有共享画布", detail: "此处显示云端已保存的画布，Mac 本地画布需先同步。", symbol: "rectangle.3.group")
                    }
                    ForEach(boards) { board in
                        NavigationLink { BoardReader(model: model, board: board) } label: {
                            VStack(alignment: .leading, spacing: 6) {
                                Text(board.title).font(.headline)
                                Text("\(board.state == "official" ? "正式画布" : "草案") · 阅读模式")
                                    .font(.caption).foregroundStyle(.secondary)
                            }.padding(.vertical, 12)
                        }
                        .accessibilityIdentifier("board.\(board.id)")
                    }
                }.listStyle(.plain).refreshable { await load() }
            }
        }
        .navigationTitle("画布").navigationBarTitleDisplayMode(.inline)
        .task(id: model.selectedProjectID) { await load() }
    }

    private func load() async {
        boards = []; error = nil; loading = true
        defer { loading = false }
        do { boards = try await model.boards() }
        catch is CancellationError { }
        catch { if !Task.isCancelled { self.error = error.localizedDescription } }
    }
}

private struct BoardReader: View {
    let model: AppModel
    let board: Board
    @State private var nodes: [BoardNode] = []
    @State private var boardRevision: Int = 0
    @State private var dragOffsets: [String: CGSize] = [:]
    @State private var error: String?
    @State private var notice: String?
    @State private var loading = true
    @Environment(\.dismiss) private var dismiss

    private var editable: Bool {
        board.isDraft && model.snapshot?.project_id == board.project_id &&
        model.snapshot?.principal.canPropose == true && !model.offline &&
        !model.pendingBoards.contains(board.id) && !model.canvasSending
    }

    var body: some View {
        Group {
            if model.selectedProjectID != board.project_id { EmptyWorkspace(title: "当前项目已切换", detail: "返回项目后继续查看画布。") }
            else if loading { ProgressView("读取画布…") }
            else if let error { EmptyWorkspace(title: "暂时无法读取", detail: error) }
            else if nodes.isEmpty { EmptyWorkspace(title: "画布为空", detail: "画布中还没有可显示的内容。", symbol: "rectangle.dashed") }
            else {
                GeometryReader { geometry in
                    let minX = nodes.map(\.x).min() ?? 0
                    let minY = nodes.map(\.y).min() ?? 0
                    let width = max(1, (nodes.map { $0.x + $0.width }.max() ?? 1) - minX)
                    let height = max(1, (nodes.map { $0.y + $0.height }.max() ?? 1) - minY)
                    let scale = min((geometry.size.width - 32) / width, (geometry.size.height - 32) / height)
                    ScrollView([.horizontal, .vertical]) {
                        ZStack(alignment: .topLeading) {
                            Color(white: 0.065)
                            ForEach(nodes) { node in
                                nodeView(node, scale: scale, minX: minX, minY: minY)
                            }
                        }.frame(width: max(geometry.size.width, width * scale + 32),
                                height: max(geometry.size.height, height * scale + 32))
                    }
                }
            }
        }
        .navigationTitle(board.title).navigationBarTitleDisplayMode(.inline)
        .safeAreaInset(edge: .bottom) {
            VStack(spacing: 4) {
                if model.pendingBoards.contains(board.id) {
                    Text("上次移动结果待确认，核对前暂停编辑。")
                        .font(.caption).foregroundStyle(.orange)
                    Button("核对并继续原操作") {
                        Task {
                            do {
                                _ = try await model.resolveCanvas(boardID: board.id)
                                notice = nil
                                await refreshAfterConflict()
                            } catch { notice = error.localizedDescription }
                        }
                    }.disabled(model.canvasSending || model.offline).accessibilityIdentifier("canvas.resolve")
                }
                if editable {
                    Text("编辑模式 · 拖动照片调整布局，修改经画布权威命令提交")
                        .font(.caption).foregroundStyle(.secondary)
                        .accessibilityIdentifier("canvas.edit.hint")
                } else {
                    Text("阅读模式 · 正式/冻结画布只读；文字、笔迹等复杂节点暂以类型标记显示")
                        .font(.caption).foregroundStyle(.secondary)
                }
                if let notice { Text(notice).font(.caption).foregroundStyle(.orange) }
            }.padding(12)
        }
        .task {
            boardRevision = board.revision
            await model.loadCanvasPending(boardID: board.id)
            await reloadNodes()
        }
        .onChange(of: model.selectedProjectID) { _, id in if id != board.project_id { dismiss() } }
    }

    @ViewBuilder
    private func nodeView(_ node: BoardNode, scale: Double, minX: Double, minY: Double) -> some View {
        let offset = dragOffsets[node.id] ?? .zero
        Group {
            if let photoID = node.photo_id {
                PhotoPreview(model: model, photoID: photoID, fit: true)
            } else {
                RoundedRectangle(cornerRadius: 3).fill(.white.opacity(0.09))
                    .overlay { Text(node.node_type).font(.caption2).foregroundStyle(.secondary).lineLimit(1) }
            }
        }
        .frame(width: max(1, node.width * scale), height: max(1, node.height * scale))
        .rotationEffect(.degrees(node.rotation))
        .position(x: (node.x - minX + node.width / 2) * scale + 16 + offset.width,
                  y: (node.y - minY + node.height / 2) * scale + 16 + offset.height)
        .accessibilityLabel(node.node_type == "photo" ? "画布照片" : "\(node.node_type) 节点")
        .accessibilityIdentifier("canvas.node.\(node.id)")
        .highPriorityGesture(editable && node.node_type == "photo" ? dragGesture(node, scale: scale) : nil)
    }

    private func dragGesture(_ node: BoardNode, scale: Double) -> some Gesture {
        DragGesture(minimumDistance: 6)
            .onChanged { value in dragOffsets[node.id] = value.translation }
            .onEnded { value in
                dragOffsets[node.id] = nil
                let targetX = node.x + value.translation.width / scale
                let targetY = node.y + value.translation.height / scale
                let current = board
                let revised = Board(id: current.id, project_id: current.project_id, title: current.title,
                                    state: current.state, revision: boardRevision)
                Task { await commitMove(node: node, board: revised, x: targetX, y: targetY) }
            }
    }

    private func commitMove(node: BoardNode, board: Board, x: Double, y: Double) async {
        do {
            let receipt = try await model.moveBoardNode(boardID: board.id, board: board,
                                                        nodeID: node.id, x: x, y: y)
            boardRevision = receipt.newRevision
            if let index = nodes.firstIndex(where: { $0.id == node.id }) {
                nodes[index].x = x
                nodes[index].y = y
            }
            notice = nil
        } catch APIError.server(409, "REVISION_CONFLICT") {
            notice = "画布已被其他设备修改，已刷新到最新。"
            await refreshAfterConflict()
        } catch {
            notice = error.localizedDescription
        }
    }

    private func refreshAfterConflict() async {
        do {
            let boards = try await model.boards()
            if let fresh = boards.first(where: { $0.id == board.id }) { boardRevision = fresh.revision }
            await reloadNodes()
        } catch { notice = "刷新失败，请重试。" }
    }

    private func reloadNodes() async {
        do {
            nodes = try await model.nodes(board: board)
            loading = false
        } catch {
            self.error = error.localizedDescription
            loading = false
        }
    }
}
