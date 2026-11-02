import SwiftUI
import FocalRoomCore

struct ProjectsView: View {
    let model: AppModel
    @State private var query = ""
    @State private var sheet: ProjectSheet?
    private var filtered: [ProjectSummary] {
        model.projects.filter { query.isEmpty || $0.title.localizedCaseInsensitiveContains(query) }
    }

    // 单一枚举驱动的浮层：同级两个 .sheet 在 iOS 上只有最后一个生效，新建本机项目会被同步中心顶掉。
    private enum ProjectSheet: String, Identifiable {
        case createLocal, syncCenter
        var id: String { rawValue }
    }

    private var active: [ProjectSummary] { filtered.filter { $0.state != .ARCHIVED } }
    private var archived: [ProjectSummary] { filtered.filter { $0.state == .ARCHIVED } }

    private func projectLabel(_ project: ProjectSummary) -> String {
        if project.sync_mode == .PRIVATE_SYNCED { return "私有同步" }
        return project.state == .SHARED ? "可访问" : "准备中"
    }

    private func projectCard(_ project: ProjectSummary, note: String) -> some View {
        VStack(alignment: .leading, spacing: 22) {
            HStack {
                Image(systemName: "rectangle.stack").font(.title2).foregroundStyle(.secondary)
                Spacer()
                Label(note, systemImage: project.state == .SHARED ? "checkmark.icloud" : "clock")
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
                if active.isEmpty {
                    EmptyWorkspace(title: query.isEmpty ? "还没有可访问的项目" : "未找到项目",
                                   detail: "可在 Mac 开启项目共享，或使用已有邀请加入项目。")
                }
                ForEach(active) { project in
                    NavigationLink {
                        ProjectLibraryView(model: model, projectID: project.id)
                    } label: {
                        projectCard(project, note: projectLabel(project))
                    }
                    .buttonStyle(.plain).disabled(project.state != .SHARED || model.busy)
                    .accessibilityIdentifier("project.\(project.id)")
                }
                // 归档项目只在服务端列表里默认隐藏；这里必须显式列出并可恢复，否则归档即失联。
                if !archived.isEmpty {
                    Text("已归档").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                    ForEach(archived) { project in
                        HStack(spacing: 12) {
                            projectCard(project, note: "已归档")
                            Button("恢复") {
                                Task {
                                    guard let text = await model.archiveCloudProject(project.id, archive: false) else { return }
                                    await model.reloadProjects()
                                    model.notice = text
                                }
                            }
                                .disabled(model.busy)
                                .accessibilityIdentifier("project.unarchive.\(project.id)")
                        }
                    }
                }
            }.padding(20)
        }
        .navigationTitle("项目").navigationBarTitleDisplayMode(.inline)
        .searchable(text: $query, prompt: "搜索项目")
        .refreshable { await model.reloadProjects(); await model.localLibrary?.reload() }
        .toolbar {
            ToolbarItem(placement: .topBarLeading) {
                Button { sheet = .createLocal } label: { Image(systemName: "plus") }
                    .accessibilityLabel("新建本机项目").accessibilityIdentifier("local.create")
                    .disabled(model.busy || model.localLibrary == nil)
            }
            ToolbarItem(placement: .topBarTrailing) {
                Button { sheet = .syncCenter } label: {
                    Image(systemName: model.offline ? "wifi.slash" : "arrow.triangle.2.circlepath")
                }
                .accessibilityLabel("同步中心").accessibilityIdentifier("sync.center")
            }
        }
        .sheet(item: $sheet) { value in
            switch value {
            case .createLocal:
                if let library = model.localLibrary { CreateLocalProjectView(library: library) }
            case .syncCenter:
                SyncCenterView(model: model)
            }
        }
        .task(id: model.credentials?.user.id) { await model.localLibrary?.reload() }
        .background(Color(white: 0.055))
    }
}

struct ProjectLibraryView: View {
    let model: AppModel
    let projectID: String
    private func cloudPhotoLink(_ photo: Photo) -> some View {
        NavigationLink {
            PhotoViewer(model: model, projectID: projectID, photoID: photo.id)
        } label: {
            cloudPhotoCell(photo)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(photo.filename)，\(photo.selection.title)")
        .accessibilityIdentifier("photo.\(photo.id)")
    }

    private func cloudPhotoCell(_ photo: Photo) -> some View {
        ZStack(alignment: .bottomLeading) {
            PhotoPreview(model: model, photoID: photo.id)
                .frame(height: 210).clipped()
            Text(LocalizedStringKey(photo.selection.title))
                .font(.caption2.weight(.medium))
                .padding(.horizontal, 9).padding(.vertical, 5)
                .background(.black.opacity(0.75), in: Capsule())
                .padding(8)
        }
    }

    @State private var filter: Selection?
    @State private var toolDestination: ProjectTool?
    @Environment(\.dismiss) private var dismiss

    private enum ProjectTool: String, Identifiable {
        case proposals, history, grants, research
        var id: String { rawValue }
    }
    private let columns = [GridItem(.flexible(), spacing: 4), GridItem(.flexible(), spacing: 4)]

    // 项目级危险操作走一个枚举驱动的对话框：同级两个 .confirmationDialog 只有一个能弹出。
    private enum ProjectDanger {
        case archive, unarchive, delete
    }
    @State private var danger: ProjectDanger?
    private var dangerPresented: Binding<Bool> {
        Binding(get: { danger != nil }, set: { if !$0 { danger = nil } })
    }
    private var dangerTitle: String {
        switch danger {
        case .delete: return "删除云端项目"
        case .unarchive: return "取消归档"
        default: return "归档项目"
        }
    }
    private var dangerMessage: String {
        switch danger {
        case .delete: return "仅项目所有者可删除；含注销审计或共享协作的项目会被拒绝并说明原因。本机原片不受影响。"
        case .unarchive: return "恢复后项目重新出现在默认列表，内容与协作记录不变。"
        default: return "归档不影响项目内容与协作记录，只是从默认列表收起到「已归档」分组，可随时恢复。"
        }
    }

    @ViewBuilder
    private var dangerActions: some View {
        switch danger {
        case .archive:
            Button("归档（列表默认隐藏，可随时恢复）") {
                Task { await applyProjectChange(await model.archiveCloudProject(projectID, archive: true)) }
            }.accessibilityIdentifier("project.archive.confirm")
        case .unarchive:
            Button("恢复为正常项目") {
                Task { await applyProjectChange(await model.archiveCloudProject(projectID, archive: false)) }
            }.accessibilityIdentifier("project.unarchive.confirm")
        case .delete:
            Button("永久删除云端项目", role: .destructive) {
                Task { await applyProjectChange(await model.deleteCloudProject(projectID)) }
            }.accessibilityIdentifier("project.delete.confirm")
        case nil:
            EmptyView()
        }
    }

    /// 详情内的项目级变更：先退出本页，再刷新列表并弹提示。顺序不能反——列表一重建，
    /// 这一格对应的导航条目就没了，之后的 dismiss 会被吞掉，用户停在「暂时无法打开项目」的死页。
    @MainActor
    private func applyProjectChange(_ noticeText: String?) async {
        guard let noticeText else { return }
        dismiss()
        await model.reloadProjects()
        model.notice = noticeText
    }

    private var photos: [Photo] {
        guard model.snapshot?.project_id == projectID else { return [] }
        return model.snapshot?.snapshot.photos.filter { filter == nil || $0.selection == filter } ?? []
    }

    private var loadingState: some View {
        ProgressView("读取项目…").frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var unavailableState: some View {
        EmptyWorkspace(title: "暂时无法打开项目", detail: "检查网络和项目权限后重试。")
            .safeAreaInset(edge: .bottom) {
                Button("重新读取") { Task { await model.openProject(projectID) } }.padding()
            }
    }

    private var galleryHeader: some View {
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
                ForEach(Selection.allCases, id: \.self) { Text(LocalizedStringKey($0.title)).tag(Optional($0)) }
            }.pickerStyle(.segmented)
        }.padding(.horizontal, 16)
    }

    private var gallery: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                galleryHeader
                PendingBanner(model: model)
                if photos.isEmpty { EmptyWorkspace(title: "没有符合条件的照片", detail: "选择其他选片状态查看。", symbol: "photo") }
                LazyVGrid(columns: columns, spacing: 4) {
                    ForEach(photos) { photo in
                        cloudPhotoLink(photo)
                    }
                }
            }.padding(.top, 16)
        }
        .refreshable { await model.reloadSnapshot() }
    }

    // 工具菜单项单独成属性：内联在 ToolbarItemGroup 里会超出编译器类型推断时限。
    @ViewBuilder
    private var toolsMenuContent: some View {
        let ready = !model.offline && model.snapshot?.project_id == projectID
        Button { toolDestination = .proposals } label: {
            Label("提案", systemImage: "checklist")
        }.disabled(!ready).accessibilityIdentifier("proposals.entry")
        Button { toolDestination = .history } label: {
            Label("版本历史", systemImage: "clock.arrow.circlepath")
        }.accessibilityIdentifier("history.entry")
        Button { danger = model.snapshot?.state == .ARCHIVED ? .unarchive : .archive } label: {
            Label(model.snapshot?.state == .ARCHIVED ? "取消归档…" : "归档项目…", systemImage: "archivebox")
        }.disabled(!ready).accessibilityIdentifier("project.archive")
        Button(role: .destructive) { danger = .delete } label: {
            Label("删除云端项目…", systemImage: "trash")
        }.disabled(!ready).accessibilityIdentifier("project.delete.cloud")
        Button { toolDestination = .grants } label: {
            Label("成员与权限", systemImage: "person.crop.circle.badge.checkmark")
        }.disabled(!ready).accessibilityIdentifier("grants.entry")
    }

    var body: some View {
        // 三个分支各自成子视图：整段内联时类型推断会超出编译器时限（v1.4.7 归档入口加入后 Debug 编译失败）。
        Group {
            if model.loadingProject && model.snapshot == nil {
                loadingState
            } else if model.snapshot?.project_id == projectID {
                gallery
            } else {
                unavailableState
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
                Button { toolDestination = .research } label: {
                    Image(systemName: "text.book.closed")
                }
                .disabled(model.offline || model.snapshot?.project_id != projectID)
                .accessibilityLabel("研究资料").accessibilityIdentifier("research.entry")
                // 提案/历史/授权收入菜单：五个直排在 iPhone 上溢出到"更多"导致入口不可达。
                Menu { toolsMenuContent } label: { Image(systemName: "ellipsis.circle") }
                .accessibilityLabel("更多工具").accessibilityIdentifier("project.tools.menu")
            }
        }
        .confirmationDialog(dangerTitle, isPresented: dangerPresented, titleVisibility: .visible) {
            dangerActions
            Button("取消", role: .cancel) {}
        } message: { Text(dangerMessage) }
        .sheet(item: $toolDestination) { destination in
            NavigationStack {
                switch destination {
                case .proposals: ProposalsView(model: model)
                case .history: VersionHistoryView(model: model, projectID: projectID)
                case .grants: GrantsView(model: model, projectID: projectID)
                case .research: ResearchView(model: model, projectID: projectID)
                }
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
    @State private var currentID: String
    @GestureState private var dragOffset = CGSize.zero

    init(model: AppModel, projectID: String, photoID: String) {
        self.model = model
        self.projectID = projectID
        self.photoID = photoID
        _currentID = State(initialValue: photoID)
    }

    private var photos: [Photo] { model.snapshot?.snapshot.photos ?? [] }
    private var currentIndex: Int { photos.firstIndex { $0.id == currentID } ?? 0 }
    @State private var info = false
    @State private var compare = false
    private var photo: Photo? {
        guard model.snapshot?.project_id == projectID else { return nil }
        return model.snapshot?.snapshot.photos.first { $0.id == currentID }
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
            // 左右滑动只做翻页（页式容器，与系统相册一致）；选片走底部按钮。
            TabView(selection: $currentID) {
                ForEach(photos) { item in
                    PhotoViewport(transform: $transform) {
                        PhotoPreview(model: model, photoID: item.id, fit: true)
                    }
                    .clipped()
                    .tag(item.id)
                }
            }
            .tabViewStyle(.page(indexDisplayMode: .never))
            .onChange(of: currentID) { _, _ in transform = PhotoTransform() }
            .accessibilityLabel("照片查看区。左滑排除，右滑保留，上滑待选；放大后可拖动查看细节")
            .accessibilityIdentifier("cloud.viewer.photo")
            .frame(maxWidth: .infinity)
            .aspectRatio(3 / 4, contentMode: .fit)
            .frame(maxHeight: UIScreen.main.bounds.height * 0.62)
            .frame(maxWidth: .infinity)

            // 底部信息与选片区布局优先：防止上方页式容器膨胀把按钮挤出屏幕。
            VStack(spacing: 14) {
                HStack {
                    Text(photo?.filename ?? "照片").font(.caption).lineLimit(1)
                    Spacer()
                    Text("授权派生预览").font(.caption2).foregroundStyle(.secondary)
                }
                HStack {
                    Button("上一张") {
                        if currentIndex > 0 { currentID = photos[currentIndex - 1].id; transform = PhotoTransform() }
                    }.disabled(currentIndex == 0 || model.busy)
                    Spacer()
                    Text("← 排除   ↑ 待选   保留 →").foregroundStyle(.secondary)
                }.font(.caption)
                HStack(spacing: 12) {
                    ForEach(Selection.allCases, id: \.self) { selection in
                        Button {
                            choose(selection)
                        } label: {
                            Label(LocalizedStringKey(selection.title), systemImage: selection == .KEEP ? "checkmark" : selection == .REJECT ? "xmark" : "minus")
                                .font(.subheadline).frame(maxWidth: .infinity, minHeight: 48)
                                .background(photo?.selection == selection ? .white.opacity(0.2) : .white.opacity(0.06),
                                            in: RoundedRectangle(cornerRadius: 12))
                        }
                        .disabled(!model.canSelect)
                        .accessibilityIdentifier("selection.\(selection.rawValue)")
                        .accessibilityValue(photo?.selection == selection ? "已选" : "未选")
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
                                    initialPhotoID: currentID, sourceLabel: "授权派生预览 · 非原片 100% 像素") { id in
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
                    if let descriptor = model.snapshot?.previews.first(where: { $0.photo_id == currentID }) {
                        LabeledContent("预览尺寸", value: "\(descriptor.width) × \(descriptor.height)")
                    }
                    Text("此预览不代表原片 100% 像素或印前签样。专业色彩与原片检视请使用 Mac。")
                        .font(.footnote).foregroundStyle(.secondary)
                }.navigationTitle("照片信息")
                    .toolbar { Button("完成") { info = false } }
            }.presentationDetents([.medium, .large])
        }
    }
    private func choose(_ selection: Selection) {
        guard let photo, model.canSelect else { return }
        let chosenID = currentID
        let nextID = photos.indices.contains(currentIndex + 1) ? photos[currentIndex + 1].id : nil
        Task {
            await model.select(photo: photo, selection: selection)
            if model.selectedProjectID == projectID, model.pending == nil,
               model.snapshot?.snapshot.photos.first(where: { $0.id == chosenID })?.selection == selection,
               currentID == chosenID, let nextID {
                currentID = nextID
                transform = PhotoTransform()
            }
        }
    }

}
