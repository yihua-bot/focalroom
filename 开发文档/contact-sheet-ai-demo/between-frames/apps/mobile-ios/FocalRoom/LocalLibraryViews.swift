import SwiftUI
import PhotosUI
import UniformTypeIdentifiers
import FocalRoomCore

struct LocalProjectsSection: View {
    let library: LocalLibraryModel
    let model: AppModel
    let query: String

    private func caption(_ project: LocalProject) -> String {
        switch project.migration?.phase {
        case .migrated: return "\(project.photos.count) 张照片 · 已迁云 · 原片保留本机"
        case .uploading: return "\(project.photos.count) 张照片 · 迁云进行中"
        case .enrolling: return "\(project.photos.count) 张照片 · 迁云进行中"
        case nil: return "\(project.photos.count) 张照片 · 仅此设备"
        }
    }

    var body: some View {
        let projects = library.projects.filter { query.isEmpty || $0.title.localizedCaseInsensitiveContains(query) }
        if !projects.isEmpty {
            Text("仅本机").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
            ForEach(projects) { project in
                NavigationLink {
                    LocalProjectView(library: library, migration: model.migration, model: model, projectID: project.id)
                        .onAppear { model.openLocalContext() }
                } label: {
                    HStack(spacing: 16) {
                        Image(systemName: project.migration?.phase == .migrated ? "icloud" : "iphone").font(.title2)
                        VStack(alignment: .leading, spacing: 6) {
                            Text(project.title).font(.headline)
                            Text(caption(project)).font(.caption).foregroundStyle(.secondary)
                        }
                        Spacer()
                        Image(systemName: "chevron.right").font(.caption).foregroundStyle(.secondary)
                    }.padding(20).background(.white.opacity(0.06), in: RoundedRectangle(cornerRadius: 16))
                }.buttonStyle(.plain).accessibilityIdentifier("local.project.\(project.id)")
            }
        }
    }
}

struct CreateLocalProjectView: View {
    @Bindable var library: LocalLibraryModel
    @Environment(\.dismiss) private var dismiss
    @State private var title = ""

    var body: some View {
        NavigationStack {
            Form {
                Section("项目名称") {
                    TextField("例如：城市散步", text: $title).accessibilityIdentifier("local.title")
                }
                Section {
                    Text("照片和项目保存在这台 iPhone，不会自动上传或同步到 Mac。请保留来源文件。")
                        .font(.footnote).foregroundStyle(.secondary)
                }
            }.navigationTitle("新建本机项目").navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button("取消") { dismiss() } }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("创建") { Task { if await library.createProject(title: title) != nil { dismiss() } } }
                            .disabled(title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || title.count > 120 || library.isSaving)
                            .accessibilityIdentifier("local.create.confirm")
                    }
                }
                .alert("提示", isPresented: Binding(get: { library.notice != nil }, set: { if !$0 { library.notice = nil } })) {
                    Button("知道了", role: .cancel) { library.notice = nil }
                } message: { Text(library.notice ?? "") }
        }
    }
}

struct LocalProjectView: View {
    @Bindable var library: LocalLibraryModel
    let migration: LocalMigrationModel?
    let model: AppModel
    let projectID: String
    @State private var picker = false
    @State private var files = false
    @State private var selected: [PhotosPickerItem] = []
    @State private var filter: Selection?
    @State private var confirmMigrate = false
    private var project: LocalProject? { library.projects.first { $0.id == projectID } }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                header
                if let migration, let project {
                    MigrationCard(migration: migration, model: model, project: project)
                        .padding(.horizontal, 16)
                }
                if library.total > 0 && library.importProjectID == projectID { ImportProgressView(library: library) }
                photoGrid
            }.padding(.top, 16)
        }
        .navigationTitle("本机图库").navigationBarTitleDisplayMode(.inline)
        .toolbar { toolbarContent }
        .confirmationDialog("迁云到工作区", isPresented: $confirmMigrate, titleVisibility: .visible) {
            Button("开始迁云", role: .destructive) {
                migration?.migrate(projectID: projectID)
            }.accessibilityIdentifier("local.migrate.confirm")
        } message: {
            Text("将上传每张照片的 2048 像素浏览预览（不含 GPS 与私密备注）到你的工作区，原片保留在本机。迁云期间本机写入冻结，完成后项目以云端为权威，可在 Mac 与其他设备续接。")
        }
        .onChange(of: migration?.finishedCloudProjectID) { _, _ in
            Task { await model.reloadProjects() }
        }
        .photosPicker(isPresented: $picker, selection: $selected, maxSelectionCount: 20, matching: .images, preferredItemEncoding: .current)
        .onChange(of: selected) { _, items in
            guard !items.isEmpty else { return }
            library.importPhotos(items, projectID: projectID)
            selected = []
        }
        .fileImporter(isPresented: $files, allowedContentTypes: [.jpeg, .png, .heic, .tiff], allowsMultipleSelection: true) { result in
            switch result {
            case .success(let urls): library.importFiles(urls, projectID: projectID)
            case .failure(let error): library.reportPickerFailure(error)
            }
        }
        .alert("提示", isPresented: Binding(get: { library.notice != nil }, set: { if !$0 { library.notice = nil } })) {
            Button("知道了", role: .cancel) { library.notice = nil }
        } message: { Text(library.notice ?? "") }
        .sheet(isPresented: Binding(get: { library.exportTarget != nil }, set: { if !$0 { library.cancelExport() } })) {
            FolderPicker { library.pickExportDestination($0) }
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(project?.title ?? "本机项目").font(.title2.weight(.medium))
            Label(statusLabel, systemImage: statusSymbol).font(.caption).foregroundStyle(.secondary)
                .accessibilityIdentifier("local.project.status")
            Picker("选片筛选", selection: $filter) {
                Text("全部").tag(Optional<Selection>.none)
                ForEach(Selection.allCases, id: \.self) { Text($0.title).tag(Optional($0)) }
            }.pickerStyle(.segmented)
        }.padding(.horizontal, 16)
    }

    private var photoGrid: some View {
        Group {
            if project?.photos.isEmpty != false {
                EmptyWorkspace(title: "导入第一组照片", detail: "从相册或文件选择照片，原文件保持不变。", symbol: "photo.on.rectangle.angled")
            } else {
                LazyVGrid(columns: [GridItem(.flexible(), spacing: 4), GridItem(.flexible(), spacing: 4)], spacing: 4) {
                    ForEach(project?.photos.filter { filter == nil || $0.selection == filter } ?? []) { photo in
                        photoCell(photo)
                    }
                }
            }
        }
    }

    private func photoCell(_ photo: LocalPhoto) -> some View {
        NavigationLink { LocalPhotoViewer(library: library, projectID: projectID, photoID: photo.id) } label: {
            ZStack(alignment: .bottomLeading) {
                localPreview(photo).frame(height: 210)
                Text(photo.selection.title).font(.caption2).padding(8)
                    .background(.black.opacity(0.75), in: Capsule()).padding(8)
            }.frame(maxWidth: .infinity).clipped().contentShape(Rectangle())
        }.buttonStyle(.plain).accessibilityLabel("\(photo.filename)，\(photo.selection.title)")
            .accessibilityIdentifier("local.photo.\(photo.id)")
    }

    @ToolbarContentBuilder
    private var toolbarContent: some ToolbarContent {
        ToolbarItem(placement: .primaryAction) {
            Menu {
                if !library.readOnly {
                    Button("从相册导入", systemImage: "photo") { picker = true }
                        .disabled(project?.migration != nil)
                    Button("从文件导入", systemImage: "folder") { files = true }
                        .disabled(project?.migration != nil)
                    if canStartMigration {
                        Button("迁云到工作区…", systemImage: "icloud.and.arrow.up") { confirmMigrate = true }
                            .accessibilityIdentifier("local.migrate")
                    }
                }
                Button("导出项目…", systemImage: "square.and.arrow.up") {
                    Task { await library.prepareExport(projectID: projectID) }
                }
                .disabled(library.isExporting || project?.photos.isEmpty != false)
                .accessibilityIdentifier("local.export")
            } label: { Image(systemName: "plus") }
                .disabled(library.isImporting || library.isExporting)
                .accessibilityLabel("导入照片").accessibilityIdentifier("local.import")
        }
    }

    private var canStartMigration: Bool {
        model.credentials != nil && migration != nil && project?.migration == nil &&
            project?.photos.isEmpty == false && !migration!.isWorking
    }

    private var statusLabel: String {
        switch project?.migration?.phase {
        case .migrated: return "已迁云 · 原片保留本机 · 云端为权威"
        case .uploading, .enrolling: return "迁云进行中 · 本机写入冻结"
        case nil: return "仅本机 · 原片不会自动上传"
        }
    }

    private var statusSymbol: String {
        project?.migration?.phase == .migrated ? "icloud" : "iphone"
    }

    private func localPreview(_ photo: LocalPhoto) -> some View {
        AsyncPhotoPreview(photoID: photo.id, identity: photo.previewSHA256) { try await library.image(projectID: projectID, photoID: photo.id) }
    }
}

struct MigrationCard: View {
    @Bindable var migration: LocalMigrationModel
    let model: AppModel
    let project: LocalProject
    @State private var confirmAbandon = false

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            switch project.migration?.phase {
            case .migrated:
                Label("已迁云", systemImage: "checkmark.icloud").font(.subheadline.weight(.semibold))
                Text("云端项目已是权威版本；本机保留原片与预览作为离线存档，选片请在云端项目进行。")
                    .font(.caption).foregroundStyle(.secondary)
                if let cloudID = project.migration?.cloudProjectID {
                    NavigationLink {
                        ProjectLibraryView(model: model, projectID: cloudID)
                    } label: {
                        Label("打开云端项目", systemImage: "arrow.up.right").frame(maxWidth: .infinity, minHeight: 44)
                    }.buttonStyle(.bordered).accessibilityIdentifier("local.migrate.open")
                }
            case .enrolling, .uploading:
                if migration.isWorking && migration.activeProjectID == project.id {
                    Label("正在迁云", systemImage: "icloud.and.arrow.up").font(.subheadline.weight(.semibold))
                    ProgressView("上传预览 \(migration.uploaded)/\(migration.total)",
                                 value: Double(migration.uploaded), total: Double(max(migration.total, 1)))
                        .accessibilityIdentifier("local.migrate.progress")
                    Button("暂停迁云") { migration.pause() }
                        .accessibilityIdentifier("local.migrate.pause")
                } else {
                    Label("迁云未完成", systemImage: "clock.arrow.circlepath").font(.subheadline.weight(.semibold))
                    Text("已上传 \(project.migration?.uploadedPhotoIDs.count ?? 0)/\(project.photos.count)。继续会沿用同一迁移，不会创建重复的云端项目。")
                        .font(.caption).foregroundStyle(.secondary)
                    HStack(spacing: 12) {
                        Button("继续迁云") { migration.migrate(projectID: project.id) }
                            .buttonStyle(.borderedProminent).accessibilityIdentifier("local.migrate.continue")
                        Button("放弃迁云", role: .destructive) { confirmAbandon = true }
                            .buttonStyle(.bordered).accessibilityIdentifier("local.migrate.abandon")
                    }
                }
            case nil:
                EmptyView()
            }
            if let error = migration.lastError, migration.activeProjectID == project.id {
                Text(error).font(.caption).foregroundStyle(.red)
            }
        }
        .padding(16).frame(maxWidth: .infinity, alignment: .leading)
        .background(.white.opacity(0.06), in: RoundedRectangle(cornerRadius: 16))
        .confirmationDialog("放弃迁云", isPresented: $confirmAbandon, titleVisibility: .visible) {
            Button("放弃并撤销云端副本", role: .destructive) { migration.abandon(projectID: project.id) }
        } message: {
            Text("将撤销尚未提交的云端副本并解冻本机项目。若云端已提交完成，则无法放弃。")
        }
    }
}

private struct ImportProgressView: View {
    let library: LocalLibraryModel

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if library.isImporting {
                ProgressView("正在导入 \(library.completed)/\(library.total)", value: Double(library.completed), total: Double(library.total))
                Button("取消剩余导入") { library.cancelImport() }
            }
            Text("\(library.wasCancelled ? "已取消，完成项已保留。" : "")已导入 \(library.imported) 张，跳过重复 \(library.duplicates) 张，失败 \(library.failures.count) 张。")
                .font(.caption).foregroundStyle(.secondary).accessibilityIdentifier("local.import.summary")
            if !library.failures.isEmpty {
                DisclosureGroup("查看失败原因") { ForEach(library.failures, id: \.self) { Text($0).font(.caption) } }
            }
        }.padding(16)
    }
}

private struct LocalPhotoViewer: View {
    let library: LocalLibraryModel
    let projectID: String
    let photoID: String
    @State private var transform = PhotoTransform()
    @State private var info = false
    @State private var compare = false
    private var project: LocalProject? { library.projects.first { $0.id == projectID } }
    private var photo: LocalPhoto? { project?.photos.first { $0.id == photoID } }

    var body: some View {
        VStack(spacing: 0) {
            PhotoViewport(transform: $transform) {
                AsyncPhotoPreview(photoID: photoID, identity: photoID, fit: true) { try await library.image(projectID: projectID, photoID: photoID) }
            }
            VStack(spacing: 12) {
                HStack {
                    Text(photo?.filename ?? "照片").font(.caption).lineLimit(1)
                    Spacer()
                    Text("本机 sRGB 预览").font(.caption2).foregroundStyle(.secondary)
                }
                HStack {
                    ForEach(Selection.allCases, id: \.self) { selection in
                        Button(selection.title) { Task { await library.select(projectID: projectID, photoID: photoID, selection: selection) } }
                            .frame(maxWidth: .infinity, minHeight: 48)
                            .background(photo?.selection == selection ? .white.opacity(0.2) : .white.opacity(0.06), in: RoundedRectangle(cornerRadius: 12))
                            .disabled(library.isSaving || project?.migration != nil || library.readOnly).accessibilityIdentifier("local.selection.\(selection.rawValue)")
                            .accessibilityValue(photo?.selection == selection ? "已选" : "未选")
                    }
                }
                if library.readOnly {
                    Text("离线只读：原片与记录可浏览、可导出；导入与选片请登录后操作")
                        .font(.caption).foregroundStyle(.secondary)
                } else if project?.migration != nil {
                    Text(project?.migration?.phase == .migrated ? "已迁云：选片请在云端项目进行" : "迁云进行中：本机选片已冻结")
                        .font(.caption).foregroundStyle(.secondary)
                }
            }.padding(18)
        }
        .background(.black).navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                Button { compare = true } label: { Image(systemName: "rectangle.split.2x1") }
                    .disabled((project?.photos.count ?? 0) < 2).accessibilityLabel("双图比较").accessibilityIdentifier("local.compare")
                Button { info = true } label: { Image(systemName: "info.circle") }.accessibilityLabel("照片信息")
            }
        }
        .sheet(isPresented: $compare) {
            PhotoComparisonView(photos: project?.photos.map { ComparisonPhoto(id: $0.id, filename: $0.filename) } ?? [],
                                initialPhotoID: photoID, sourceLabel: "本机 sRGB 派生预览 · 原片未改动") {
                try await library.image(projectID: projectID, photoID: $0)
            }
        }
        .sheet(isPresented: $info) {
            NavigationStack {
                List {
                    if let photo {
                        LabeledContent("文件", value: photo.filename)
                        LabeledContent("原始尺寸", value: "\(photo.width) × \(photo.height)")
                        LabeledContent("原始方向", value: "\(photo.orientation)")
                        LabeledContent("格式", value: photo.format.uppercased())
                        LabeledContent("源色彩配置", value: photo.profileName ?? "未标注")
                        LabeledContent("保存位置", value: "仅此 iPhone")
                        Text("原始 SHA256\n\(photo.sourceSHA256)").font(.caption.monospaced()).textSelection(.enabled)
                        Text("浏览预览为 sRGB JPEG，不代表原片像素或专业印前签样。")
                            .font(.footnote).foregroundStyle(.secondary)
                    }
                }.navigationTitle("本机照片信息").toolbar { Button("完成") { info = false } }
            }.presentationDetents([.medium, .large])
        }
    }
}
