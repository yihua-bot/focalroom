import SwiftUI
import FocalRoomCore

struct ProjectSyncView: View {
    let model: AppModel
    let projectID: String
    @State private var state: ProjectSyncState?
    @State private var busy = false
    @State private var failure: String?
    @State private var confirming = false

    var body: some View {
        List {
            if let state {
                Section("同步范围") {
                    Label(state.mode == .PRIVATE_SYNCED ? "私有同步" : "协作共享", systemImage: state.mode == .PRIVATE_SYNCED ? "lock.icloud" : "person.2")
                        .accessibilityIdentifier("project.sync.mode")
                    Text(state.mode == .PRIVATE_SYNCED
                         ? "云端为正式版本，本机保留原片。只有你自己的已登录设备能访问，工作区管理员也不可见。"
                         : "云端为正式版本，工作区管理员及已授权协作者按当前权限访问。")
                        .font(.footnote).foregroundStyle(.secondary)
                    if model.snapshot?.principal.role == "OWNER" {
                        Button(state.mode == .PRIVATE_SYNCED ? "开启协作共享" : "收回为私有同步") { confirming = true }
                            .disabled(busy).accessibilityIdentifier("project.sync.change")
                    }
                }
            } else if failure == nil { ProgressView("核实同步范围…") }
            if let failure { Text(failure).font(.footnote).foregroundStyle(.orange) }
        }
        .navigationTitle("同步与共享").navigationBarTitleDisplayMode(.inline)
        .task { await reload() }
        .refreshable { await reload() }
        .alert(state?.mode == .PRIVATE_SYNCED ? "开启协作共享？" : "收回为私有同步？", isPresented: $confirming) {
            Button("取消", role: .cancel) {}
            Button("确认") { Task { await change() } }
        } message: {
            Text(state?.mode == .PRIVATE_SYNCED
                 ? "开启后当前工作区管理员将可访问此项目；其他协作者仍须获得邀请。"
                 : "所有项目协作者、未接受邀请与任务授权将被撤销。再次开启共享不会自动恢复旧授权。")
        }
    }

    private func reload() async {
        do { state = try await model.projectSyncState(projectID: projectID); failure = nil }
        catch { state = nil; failure = error.localizedDescription }
    }

    private func change() async {
        guard let state, !busy else { return }
        busy = true
        defer { busy = false }
        do {
            let command = try SyncModeCommand(state: state, shared: state.mode == .PRIVATE_SYNCED)
            try await model.changeProjectSyncState(projectID: projectID, command: command)
            await reload()
        } catch {
            // 不以网络错误推断未提交；只读权威状态，用户确认后才再次改变范围。
            await reload()
            failure = "上次结果需核对，当前显示服务端可确认的范围。\n" + error.localizedDescription
        }
    }
}
