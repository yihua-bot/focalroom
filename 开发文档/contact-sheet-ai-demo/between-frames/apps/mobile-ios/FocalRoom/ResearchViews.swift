import SwiftUI
import FocalRoomCore

enum ResearchEditorTarget: Identifiable {
    case create
    case edit(ResearchEntry)

    var id: String {
        switch self {
        case .create: return "create"
        case .edit(let entry): return entry.id
        }
    }
}

struct ResearchView: View {
    let model: AppModel
    let projectID: String
    @State private var entries: [ResearchEntry] = []
    @State private var loading = true
    @State private var busy = false
    @State private var notice: String?
    @State private var editorTarget: ResearchEditorTarget?

    private var canEdit: Bool {
        guard let snapshot = model.snapshot, snapshot.project_id == projectID else { return false }
        return snapshot.principal.role != "VIEWER" && !model.offline
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                if entries.isEmpty {
                    EmptyWorkspace(title: "还没有研究资料", detail: "记录参考笔记与链接，辅助项目研究与整理。", symbol: "text.book.closed")
                        .padding(.top, 60)
                } else {
                    ForEach(entries) { entry in
                        Button {
                            if canEdit { editorTarget = .edit(entry) }
                        } label: {
                            row(entry)
                        }
                        .buttonStyle(.plain)
                        .accessibilityIdentifier("research.row.\(entry.id)")
                    }
                }
            }
            .padding(.horizontal, 16).padding(.vertical, 12)
        }
        .navigationTitle("研究资料").navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    editorTarget = .create
                } label: { Image(systemName: "plus") }
                    .disabled(!canEdit || busy || loading)
                    .accessibilityLabel("新建研究资料").accessibilityIdentifier("research.create")
            }
        }
        .task { await load() }
        .sheet(item: $editorTarget) { target in
            ResearchEditorView(model: model, projectID: projectID, target: target) {
                Task { await load() }
            }
        }
        .alert("提示", isPresented: Binding(get: { notice != nil }, set: { if !$0 { notice = nil } })) {
            Button("知道了", role: .cancel) { notice = nil }
        } message: { Text(notice ?? "") }
    }

    private func row(_ entry: ResearchEntry) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: entry.isNote ? "note.text" : "link")
                .font(.title3).foregroundStyle(.secondary).padding(.top, 2)
            VStack(alignment: .leading, spacing: 5) {
                Text(entry.title).font(.headline).foregroundStyle(.primary)
                if let body = entry.body {
                    Text(body).font(.subheadline).foregroundStyle(.secondary).lineLimit(3)
                }
                if let url = entry.url {
                    Text(url).font(.caption.monospaced()).foregroundStyle(.secondary).lineLimit(1)
                }
                Text(metadataCaption(entry)).font(.caption2).foregroundStyle(.tertiary)
            }
            Spacer(minLength: 0)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.white.opacity(0.06), in: RoundedRectangle(cornerRadius: 14))
    }

    private func metadataCaption(_ entry: ResearchEntry) -> String {
        var parts = [entry.created_by_name]
        if let author = entry.author { parts.append(author) }
        if let source = entry.source { parts.append(source) }
        return parts.joined(separator: " · ")
    }

    private func load() async {
        loading = true
        defer { loading = false }
        do { entries = try await model.research(projectID: projectID) }
        catch {
            if !entries.isEmpty { notice = message(error) }
        }
    }

    private func message(_ error: Error) -> String {
        if let value = error as? LocalizedError, let description = value.errorDescription { return description }
        return "暂时无法读取研究资料，请稍后重试。"
    }
}

struct ResearchEditorView: View {
    let model: AppModel
    let projectID: String
    let target: ResearchEditorTarget
    let onSaved: () -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var kind: ResearchDraft.Kind
    @State private var title: String
    @State private var bodyText: String
    @State private var urlText: String
    @State private var author: String
    @State private var sourceText: String
    @State private var license: String
    @State private var busy = false
    @State private var notice: String?
    @State private var confirmDelete = false
    @State private var voice = VoiceDictationModel()

    init(model: AppModel, projectID: String, target: ResearchEditorTarget, onSaved: @escaping () -> Void) {
        self.model = model
        self.projectID = projectID
        self.target = target
        self.onSaved = onSaved
        let existing: ResearchEntry?
        if case .edit(let entry) = target { existing = entry } else { existing = nil }
        _kind = State(initialValue: existing?.isLink == true ? .link : .note)
        _title = State(initialValue: existing?.title ?? "")
        _bodyText = State(initialValue: existing?.body ?? "")
        _urlText = State(initialValue: existing?.url ?? "")
        _author = State(initialValue: existing?.author ?? "")
        _sourceText = State(initialValue: existing?.source ?? "")
        _license = State(initialValue: existing?.license ?? "")
    }

    private var editingEntry: ResearchEntry? {
        if case .edit(let entry) = target { return entry }
        return nil
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("类型") {
                    Picker("类型", selection: $kind) {
                        Text("笔记").tag(ResearchDraft.Kind.note).accessibilityIdentifier("research.kind.note")
                        Text("链接").tag(ResearchDraft.Kind.link).accessibilityIdentifier("research.kind.link")
                    }
                    .pickerStyle(.segmented)
                    .disabled(editingEntry != nil)
                }
                Section("内容") {
                    TextField("标题", text: $title).accessibilityIdentifier("research.title")
                    if kind == .note {
                        TextEditor(text: $bodyText)
                            .frame(minHeight: 120)
                            .accessibilityIdentifier("research.body")
                        VoiceEntrySection(voice: voice) { transcribed in
                            bodyText = bodyText.isEmpty ? transcribed : bodyText + "\n" + transcribed
                        }
                    } else {
                        TextField("https://…", text: $urlText)
                            .keyboardType(.URL).textInputAutocapitalization(.never).autocorrectionDisabled()
                            .accessibilityIdentifier("research.url")
                    }
                }
                Section("来源（可选）") {
                    TextField("作者", text: $author).accessibilityIdentifier("research.author")
                    TextField("来源", text: $sourceText)
                    TextField("许可", text: $license)
                }
                if let entry = editingEntry {
                    Section {
                        Button("删除这条研究资料", role: .destructive) { confirmDelete = true }
                            .accessibilityIdentifier("research.delete")
                    }
                }
            }
            .navigationTitle(editingEntry == nil ? "新建研究资料" : "编辑研究资料").navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("取消") { voice.stop(); dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("保存") { Task { await save() } }
                        .disabled(busy || title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                        .accessibilityIdentifier("research.save")
                }
            }
            .confirmationDialog("删除研究资料", isPresented: $confirmDelete, titleVisibility: .visible) {
                Button("删除", role: .destructive) { Task { await remove() } }
            } message: {
                Text("删除后无法恢复。")
            }
            .alert("提示", isPresented: Binding(get: { notice != nil }, set: { if !$0 { notice = nil } })) {
                Button("知道了", role: .cancel) { notice = nil }
            } message: { Text(notice ?? "") }
        }
    }

    private func save() async {
        guard !busy else { return }
        busy = true
        voice.stop()
        defer { busy = false }
        do {
            switch target {
            case .create:
                let draft = try ResearchDraft(kind: kind, title: title,
                                              body: kind == .note ? bodyText : nil,
                                              url: kind == .link ? urlText : nil,
                                              author: author, source: sourceText, license: license)
                try await model.createResearch(projectID: projectID, draft: draft)
            case .edit(let entry):
                let patch = try ResearchPatch(title: title,
                                              body: entry.isNote ? bodyText : nil,
                                              url: entry.isLink ? urlText : nil,
                                              author: author, source: sourceText, license: license)
                try await model.updateResearch(projectID: projectID, referenceID: entry.id, patch: patch)
            }
            dismiss()
            onSaved()
        } catch {
            notice = message(error)
        }
    }

    private func remove() async {
        guard let entry = editingEntry, !busy else { return }
        busy = true
        defer { busy = false }
        do {
            try await model.deleteResearch(projectID: projectID, referenceID: entry.id)
            dismiss()
            onSaved()
        } catch {
            notice = message(error)
        }
    }

    private func message(_ error: Error) -> String {
        if let value = error as? LocalizedError, let description = value.errorDescription { return description }
        return "暂时无法保存，请稍后重试。"
    }
}
