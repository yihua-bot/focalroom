import SwiftUI
import FocalRoomCore

struct ProjectsView: View {
    let model: AppModel
    @State private var query = ""
    @State private var createLocalProject = false
    @State private var syncCenter = false
    private var filtered: [ProjectSummary] {
        model.projects.filter { query.isEmpty || $0.title.localizedCaseInsensitiveContains(query) }
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 22) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("继续创作").font(.system(.largeTitle, design: .serif).weight(.medium))
                    Text(model.offline ? "离线缓存 · 联网后核实最新权限与版本" : "从一条线索，回到你的作品。")
                        .font(.subheadline).foregroundStyle(.secondary)
                }.padding(.top, 20)
                if let localLibrary = model.localLibrary {
                    LocalProjectsSection(library: localLibrary, model: model, query: query)
                }
                Text("已同步项目").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                if filtered.isEmpty {
                    EmptyWorkspace(title: query.isEmpty ? "还没有可访问的项目" : "未找到项目",
                                   detail: "可在 Mac 开启项目共享，或使用已有邀请加入项目。")
                }
                ForEach(filtered) { project in
                    NavigationLink {
                        ProjectLibraryView(model: model, projectID: project.id)
                    } label: {
                        VStack(alignment: .leading, spacing: 22) {
                            HStack {
                                Image(systemName: "rectangle.stack").font(.title2).foregroundStyle(.secondary)
                                Spacer()
                                Label(project.sync_mode == .PRIVATE_SYNCED ? "私有同步" : project.state == .SHARED ? "可访问" : "准备中",
                                      systemImage: project.state == .SHARED ? "checkmark.icloud" : "clock")
                                    .font(.caption).foregroundStyle(.secondary)
                            }
                            Text(project.title).font(.title2.weight(.medium)).multilineTextAlignment(.leading)
                            HStack {
                                Text("版本 \(project.revision)").font(.caption.monospacedDigit())
                                Spacer()
                                Image(systemName: "arrow.up.right")
                            }.foregroundStyle(.secondary)
                        }
                        .padding(22).frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color(white: 0.11), in: RoundedRectangle(cornerRadius: 18))
                    }
                    .buttonStyle(.plain).disabled(project.state != .SHARED || model.busy)
                    .accessibilityIdentifier("project.\(project.id)")
                }
            }.padding(20)
        }
        .navigationTitle("项目").navigationBarTitleDisplayMode(.inline)
        .searchable(text: $query, prompt: "搜索项目")
        .refreshable { await model.reloadProjects(); await model.localLibrary?.reload() }
        .toolbar {
            ToolbarItem(placement: .topBarLeading) {
                Button { createLocalProject = true } label: { Image(systemName: "plus") }
                    .accessibilityLabel("新建本机项目").accessibilityIdentifier("local.create")
                    .disabled(model.busy || model.localLibrary == nil)
            }
            ToolbarItem(placement: .topBarTrailing) {
                Button { syncCenter = true } label: {
                    Image(systemName: model.offline ? "wifi.slash" : "arrow.triangle.2.circlepath")
                }
                .accessibilityLabel("同步中心").accessibilityIdentifier("sync.center")
            }
        }
        .sheet(isPresented: $createLocalProject) {
            if let library = model.localLibrary { CreateLocalProjectView(library: library) }
        }
        .sheet(isPresented: $syncCenter) {
            SyncCenterView(model: model)
        }
        .task(id: model.credentials?.user.id) { await model.localLibrary?.reload() }
        .background(Color(white: 0.055))
    }
}

struct ProjectLibraryView: View {
    let model: AppModel
    let projectID: String
    @State private var filter: Selection?
    private let columns = [GridItem(.flexible(), spacing: 4), GridItem(.flexible(), spacing: 4)]

    private var photos: [Photo] {
        guard model.snapshot?.project_id == projectID else { return [] }
        return model.snapshot?.snapshot.photos.filter { filter == nil || $0.selection == filter } ?? []
    }

    var body: some View {
        Group {
            if model.loadingProject && model.snapshot == nil {
                ProgressView("读取项目…").frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if model.snapshot?.project_id == projectID {
                ScrollView {
                    VStack(alignment: .leading, spacing: 18) {
                        VStack(alignment: .leading, spacing: 10) {
                            Text(model.snapshot?.title ?? "图库").font(.title2.weight(.medium))
                            HStack {
                                Text("\(model.snapshot?.snapshot.photos.count ?? 0) 张照片")
                                Spacer()
                                Label(model.offline ? "离线缓存" : "已同步",
                                      systemImage: model.offline ? "wifi.slash" : "checkmark.icloud")
                            }.font(.caption).foregroundStyle(.secondary)
                            if let date = model.cachedAt, model.offline {
                                Text("缓存于 \(date.formatted(date: .abbreviated, time: .shortened))，权限待联网核实")
                                    .font(.caption).foregroundStyle(.secondary)
                            }
                            Picker("选片筛选", selection: $filter) {
                                Text("全部").tag(Optional<Selection>.none)
                                ForEach(Selection.allCases, id: \.self) { Text($0.title).tag(Optional($0)) }
                            }.pickerStyle(.segmented)
                        }.padding(.horizontal, 16)
                        PendingBanner(model: model)
                        if photos.isEmpty { EmptyWorkspace(title: "没有符合条件的照片", detail: "选择其他选片状态查看。", symbol: "photo") }
                        LazyVGrid(columns: columns, spacing: 4) {
                            ForEach(photos) { photo in
                                NavigationLink {
                                    PhotoViewer(model: model, projectID: projectID, photoID: photo.id)
                                } label: {
                                    ZStack(alignment: .bottomLeading) {
                                        PhotoPreview(model: model, photoID: photo.id)
                                            .frame(height: 210).clipped()
                                        Text(photo.selection.title).font(.caption2.weight(.medium))
                                            .padding(.horizontal, 9).padding(.vertical, 5)
                                            .background(.black.opacity(0.75), in: Capsule()).padding(8)
                                    }
                                }
                                .buttonStyle(.plain)
                                .accessibilityLabel("\(photo.filename)，\(photo.selection.title)")
                                .accessibilityIdentifier("photo.\(photo.id)")
                            }
                        }
                    }.padding(.top, 16)
                }
                .refreshable { await model.reloadSnapshot() }
            } else {
                EmptyWorkspace(title: "暂时无法打开项目", detail: "检查网络和项目权限后重试。")
                    .safeAreaInset(edge: .bottom) {
                        Button("重新读取") { Task { await model.openProject(projectID) } }.padding()
                    }
            }
        }
        .navigationTitle("图库").navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                NavigationLink {
                    ProjectSyncView(model: model, projectID: projectID)
                } label: { Image(systemName: "lock.icloud") }
                .disabled(model.offline || model.snapshot?.project_id != projectID)
                .accessibilityLabel("同步与共享").accessibilityIdentifier("project.sync.entry")
                NavigationLink {
                    ResearchView(model: model, projectID: projectID)
                } label: {
                    Image(systemName: "text.book.closed")
                }
                .disabled(model.offline || model.snapshot?.project_id != projectID)
                .accessibilityLabel("研究资料").accessibilityIdentifier("research.entry")
                NavigationLink {
                    ProposalsView(model: model)
                } label: {
                    Image(systemName: "checklist")
                }
                .disabled(model.offline || model.snapshot?.project_id != projectID)
                .accessibilityLabel("提案").accessibilityIdentifier("proposals.entry")
                NavigationLink {
                    VersionHistoryView(model: model, projectID: projectID)
                } label: {
                    Image(systemName: "clock.arrow.circlepath")
                }
                .disabled(model.offline || model.snapshot?.project_id != projectID)
                .accessibilityLabel("版本历史").accessibilityIdentifier("history.entry")
            }
        }
        .background(Color(white: 0.055))
        .task(id: projectID) { if model.selectedProjectID != projectID { await model.openProject(projectID) } }
    }
}

struct PendingBanner: View {
    let model: AppModel

    var body: some View {
        if let pending = model.pending {
            VStack(alignment: .leading, spacing: 10) {
                Label(pending.state == .queued ? "选片待提交" : pending.state == .rejected ? "选片未提交成功" : "选片结果待确认",
                      systemImage: "clock.arrow.circlepath").font(.subheadline.weight(.semibold))
                Text("本机意图：\(pending.command.payload.selection_state.title)。当前显示仍为上次确认的正式版本。")
                    .font(.caption).foregroundStyle(.secondary)
                if pending.state == .rejected {
                    Button("放弃这次未生效的选片并刷新") { Task { await model.discardRejected() } }
                } else {
                    Button("核对并重试原操作") { Task { await model.retryPending() } }
                }
            }
            .disabled(model.busy || model.loadingProject)
            .padding(16).frame(maxWidth: .infinity, alignment: .leading)
            .background(.white.opacity(0.06))
        }
    }
}

struct PhotoPreview: View {
    let model: AppModel
    let photoID: String
    var fit = false

    private var identity: String {
        "\(model.credentials?.user.id ?? ""): \(model.selectedProjectID ?? ""): \(photoID):\(model.snapshot?.revision ?? -1):\(model.offline)"
    }

    var body: some View {
        AsyncPhotoPreview(photoID: photoID, identity: identity, fit: fit,
                          failureText: model.offline ? "联网后读取预览" : "点按重试") {
            try await model.image(photoID: photoID)
        }
    }
}

struct PhotoViewer: View {
    let model: AppModel
    let projectID: String
    let photoID: String
    @Environment(\.dismiss) private var dismiss
    @State private var transform = PhotoTransform()
    @State private var info = false
    @State private var compare = false
    private var photo: Photo? {
        guard model.snapshot?.project_id == projectID else { return nil }
        return model.snapshot?.snapshot.photos.first { $0.id == photoID }
    }

    var body: some View {
        Group {
            if model.selectedProjectID != projectID {
                EmptyWorkspace(title: "当前项目已切换", detail: "返回项目后继续查看照片。", symbol: "photo")
            } else {
                viewerContent
            }
        }
        .onChange(of: model.selectedProjectID) { _, id in if id != projectID { dismiss() } }
    }

    private var viewerContent: some View {
        VStack(spacing: 0) {
            PendingBanner(model: model)
            PhotoViewport(transform: $transform) {
                PhotoPreview(model: model, photoID: photoID, fit: true)
            }

            VStack(spacing: 14) {
                HStack {
                    Text(photo?.filename ?? "照片").font(.caption).lineLimit(1)
                    Spacer()
                    Text("授权派生预览").font(.caption2).foregroundStyle(.secondary)
                }
                HStack(spacing: 12) {
                    ForEach(Selection.allCases, id: \.self) { selection in
                        Button {
                            if let photo { Task { await model.select(photo: photo, selection: selection) } }
                        } label: {
                            Label(selection.title, systemImage: selection == .KEEP ? "checkmark" : selection == .REJECT ? "xmark" : "minus")
                                .font(.subheadline).frame(maxWidth: .infinity, minHeight: 48)
                                .background(photo?.selection == selection ? .white.opacity(0.2) : .white.opacity(0.06),
                                            in: RoundedRectangle(cornerRadius: 12))
                        }
                        .disabled(!model.canSelect)
                        .accessibilityIdentifier("selection.\(selection.rawValue)")
                        .accessibilityValue(photo?.selection == selection ? "已选" : "未选")
                    }
                }
                if model.snapshot?.principal.permitsSelection != true {
                    Text("当前权限为只读").font(.caption).foregroundStyle(.secondary)
                }
            }.padding(18)
        }
        .background(.black).navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                Button { compare = true } label: { Image(systemName: "rectangle.split.2x1") }
                    .disabled((model.snapshot?.snapshot.photos.count ?? 0) < 2)
                    .accessibilityLabel("双图比较").accessibilityIdentifier("photo.compare")
                Button { info = true } label: { Image(systemName: "info.circle") }.accessibilityLabel("照片信息")
            }
        }
        .sheet(isPresented: $compare) {
            if model.snapshot?.project_id == projectID {
                PhotoComparisonView(photos: model.snapshot?.snapshot.photos.map { ComparisonPhoto(id: $0.id, filename: $0.filename) } ?? [],
                                    initialPhotoID: photoID, sourceLabel: "授权派生预览 · 非原片 100% 像素") { id in
                    guard model.selectedProjectID == projectID else { throw APIError.permissionDenied }
                    return try await model.image(photoID: id)
                }
            } else { EmptyWorkspace(title: "项目访问已变化", detail: "关闭比较后重新打开项目。") }
        }
        .sheet(isPresented: $info) {
            NavigationStack {
                List {
                    LabeledContent("文件", value: photo?.filename ?? "")
                    LabeledContent("选片", value: photo?.selection.title ?? "")
                    LabeledContent("显示来源", value: "云端授权 JPEG 派生预览")
                    if let descriptor = model.snapshot?.previews.first(where: { $0.photo_id == photoID }) {
                        LabeledContent("预览尺寸", value: "\(descriptor.width) × \(descriptor.height)")
                    }
                    Text("此预览不代表原片 100% 像素或印前签样。专业色彩与原片检视请使用 Mac。")
                        .font(.footnote).foregroundStyle(.secondary)
                }.navigationTitle("照片信息")
                    .toolbar { Button("完成") { info = false } }
            }.presentationDetents([.medium, .large])
        }
    }
}
