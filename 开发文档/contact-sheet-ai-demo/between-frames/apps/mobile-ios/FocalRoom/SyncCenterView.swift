import SwiftUI
import FocalRoomCore

// 同步中心：集中展示各项目的本机缓存与待提交意图；核对/放弃操作复用权威回执路径。
struct SyncCenterView: View {
    let model: AppModel
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Label(model.offline ? "当前离线：以下缓存可继续浏览，待提交操作联网后核对"
                                        : "在线：以下为本机缓存状态，打开项目时自动更新",
                          systemImage: model.offline ? "wifi.slash" : "checkmark.icloud")
                        .font(.subheadline).foregroundStyle(.secondary)
                        .accessibilityIdentifier("sync.status")
                }
                Section("已缓存项目") {
                    if model.syncOverview.isEmpty {
                        Text("尚无缓存项目；打开过的云端项目会自动缓存资料与已查看的预览。")
                            .foregroundStyle(.secondary)
                    } else {
                        ForEach(model.syncOverview) { row in
                            rowView(row)
                        }
                    }
                }
            }
            .navigationTitle("同步中心").navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) { Button("完成") { dismiss() } }
            }
            .task { await model.loadSyncOverview() }
            .refreshable { await model.loadSyncOverview() }
        }
    }

    private func rowView(_ row: CachedProjectOverview) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(row.title).font(.headline)
                Spacer()
                if let pending = row.pending {
                    Label(pendingBadge(pending.state), systemImage: "clock.arrow.circlepath")
                        .font(.caption).foregroundStyle(pending.state == .rejected ? .red : .orange)
                }
            }
            Text(row.revision >= 0
                 ? "缓存于 \(row.savedAt.formatted(date: .abbreviated, time: .shortened)) · 版本 \(row.revision)"
                 : "有待提交操作，项目资料未缓存")
                .font(.caption).foregroundStyle(.secondary)
            if let pending = row.pending {
                HStack(spacing: 12) {
                    if pending.state == .rejected {
                        Button("放弃这次未生效的选片") {
                            Task { await model.discardRejected(projectID: row.projectID); await model.loadSyncOverview() }
                        }
                        .font(.subheadline)
                        .accessibilityIdentifier("sync.discard.\(row.projectID)")
                    } else {
                        Button("核对并重试原操作") {
                            Task { await model.retryPending(projectID: row.projectID); await model.loadSyncOverview() }
                        }
                        .font(.subheadline)
                        .accessibilityIdentifier("sync.retry.\(row.projectID)")
                    }
                }
            }
        }
        .padding(.vertical, 4)
        .accessibilityIdentifier("sync.row.\(row.projectID)")
    }

    private func pendingBadge(_ state: PendingSelection.State) -> String {
        switch state {
        case .queued: return "选片待提交"
        case .unknown: return "结果待确认"
        case .rejected: return "选片未提交成功"
        }
    }
}
