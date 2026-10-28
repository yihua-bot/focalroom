import SwiftUI
import FocalRoomCore

// 版本历史：选择两个版本对比照片/序列差异；只读权威数据，不在客户端重放合并。
struct VersionHistoryView: View {
    let model: AppModel
    let projectID: String
    @State private var versions: [VersionSummary] = []
    @State private var selected: [Int] = []
    @State private var diff: VersionDiff?
    @State private var loading = true
    @State private var error: String?
    @State private var diffError: String?

    var body: some View {
        Group {
            if loading {
                ProgressView("读取版本历史…").frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let error {
                EmptyWorkspace(title: "暂时无法读取", detail: error, symbol: "clock.arrow.circlepath")
                Button("重试") { Task { await load() } }.padding()
            } else if versions.isEmpty {
                EmptyWorkspace(title: "还没有版本记录", detail: "正式修改会推进项目版本。", symbol: "clock.arrow.circlepath")
            } else {
                List {
                    Section("选择两个版本对比") {
                        ForEach(versions) { version in
                            Button { toggle(version.revision) } label: {
                                HStack {
                                    VStack(alignment: .leading, spacing: 4) {
                                        Text("版本 \(version.revision)").font(.headline).foregroundStyle(.primary)
                                        Text("\(version.created_at) · 命令 \(version.command_id.prefix(8))")
                                            .font(.caption).foregroundStyle(.secondary)
                                    }
                                    Spacer()
                                    if selected.contains(version.revision) {
                                        Image(systemName: "checkmark.circle.fill").foregroundStyle(Color.accentColor)
                                    }
                                }
                            }
                            .accessibilityIdentifier("version.row.\(version.revision)")
                        }
                    }
                    if let diffError {
                        Section { Text(diffError).font(.footnote).foregroundStyle(.secondary) }
                    }
                    if let diff {
                        diffSection(diff)
                    }
                }
            }
        }
        .navigationTitle("版本历史").navigationBarTitleDisplayMode(.inline)
        .task { await load() }
    }

    private func diffSection(_ diff: VersionDiff) -> some View {
        Section("版本 \(diff.from.revision) → \(diff.to.revision) 差异") {
            entityRows("照片", diff.photos)
            entityRows("序列", diff.sequences)
        }
    }

    @ViewBuilder
    private func entityRows(_ title: String, _ entity: VersionDiff.EntityDiff) -> some View {
        if entity.added.isEmpty && entity.removed.isEmpty && entity.changed.isEmpty {
            Text("\(title)：无变化").font(.footnote).foregroundStyle(.secondary)
        } else {
            if !entity.added.isEmpty {
                LabeledContent("\(title) 新增", value: entity.added.joined(separator: "、"))
                    .font(.footnote)
            }
            if !entity.removed.isEmpty {
                LabeledContent("\(title) 移除", value: entity.removed.joined(separator: "、"))
                    .font(.footnote)
            }
            ForEach(entity.changed) { entity_ in
                VStack(alignment: .leading, spacing: 4) {
                    Text("\(title) \(entity_.id.prefix(12))").font(.footnote.weight(.semibold))
                    ForEach(entity_.fields.keys.sorted(), id: \.self) { field in
                        if let change = entity_.fields[field] {
                            Text("\(field)：\(change.before?.displayText ?? "空") → \(change.after?.displayText ?? "空")")
                                .font(.caption).foregroundStyle(.secondary)
                        }
                    }
                }
            }
        }
    }

    private func toggle(_ revision: Int) {
        if let index = selected.firstIndex(of: revision) {
            selected.remove(at: index)
        } else {
            selected.append(revision)
            if selected.count > 2 { selected.removeFirst() }
        }
        diff = nil
        diffError = nil
        if selected.count == 2, let api = model.credentials, !model.offline {
            let from = selected.min() ?? 0
            let to = selected.max() ?? 0
            Task {
                do {
                    diff = try await model.versionDiff(projectID: projectID, from: from, to: to)
                } catch {
                    diffError = "差异读取失败，请重试。"
                }
            }
        }
    }

    private func load() async {
        loading = true
        defer { loading = false }
        do {
            versions = try await model.projectHistory(projectID: projectID)
            selected = []
            diff = nil
        } catch {
            self.error = error.localizedDescription
        }
    }
}
