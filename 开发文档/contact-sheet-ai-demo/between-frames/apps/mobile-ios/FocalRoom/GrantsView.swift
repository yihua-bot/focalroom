import SwiftUI
import FocalRoomCore

// 项目授权管理：列出被授权成员，调整角色与 EDITOR 专属权限，撤销访问；仅 OWNER/ADMIN 可见入口。
struct GrantsView: View {
    let model: AppModel
    let projectID: String
    @State private var editorTarget: ProjectGrant?
    @State private var editorRole: String = "EDITOR"
    @State private var editorApprove = false
    @State private var editorFormal = false
    @State private var revokeTarget: ProjectGrant?

    private var grants: [ProjectGrant] {
        guard let snapshot = model.snapshot, snapshot.project_id == projectID else { return [] }
        return (snapshot.grants ?? []).filter(\.active)
    }

    private var canManage: Bool {
        guard let snapshot = model.snapshot, snapshot.project_id == projectID else { return false }
        return snapshot.principal.role == "OWNER" || snapshot.principal.role == "ADMIN"
    }

    var body: some View {
        Group {
            if model.snapshot == nil {
                EmptyWorkspace(title: "选择一个项目", detail: "查看与管理项目成员的访问权限。", symbol: "person.crop.circle.badge.checkmark")
            } else if !canManage {
                EmptyWorkspace(title: "仅所有者或管理员可管理权限", detail: "当前角色为只读。", symbol: "lock.shield")
            } else if grants.isEmpty {
                EmptyWorkspace(title: "还没有被授权的成员", detail: "通过账户页的邀请功能添加协作者后，在这里调整他们的项目权限。", symbol: "person.badge.plus")
            } else {
                List {
                    Section {
                        ForEach(grants) { grant in
                            row(grant)
                        }
                    } footer: {
                        Text("撤销后成员立即失去项目访问；EDITOR 专属的审批与正式编辑权限只有编辑角色可持有。")
                    }
                }
            }
        }
        .navigationTitle("成员与权限").navigationBarTitleDisplayMode(.inline)
        .refreshable { await model.reloadSnapshot() }
        .sheet(item: $editorTarget, onDismiss: { editorTarget = nil }) { grant in
            roleEditorSheet(grant)
        }
        .confirmationDialog("撤销成员访问", isPresented: Binding(get: { revokeTarget != nil },
                                                               set: { if !$0 { revokeTarget = nil } }),
                           titleVisibility: .visible) {
            Button("撤销 \(revokeTarget?.email ?? "")", role: .destructive) {
                if let target = revokeTarget {
                    revokeTarget = nil
                    Task { await model.revokeGrant(userID: target.user_id) }
                }
            }
        } message: {
            Text("该成员将立即失去此项目的访问权限。")
        }
    }

    private func row(_ grant: ProjectGrant) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(grant.email).font(.subheadline.weight(.medium))
                    Text(roleTitle(grant.role) + privilegeSuffix(grant))
                        .font(.caption).foregroundStyle(.secondary)
                }
                Spacer()
                Button("调整") {
                    editorRole = grant.role
                    editorApprove = grant.can_approve
                    editorFormal = grant.can_edit_formal
                    editorTarget = grant
                }
                .font(.caption)
                .accessibilityIdentifier("grant.edit.\(grant.user_id)")
                Button("撤销", role: .destructive) { revokeTarget = grant }
                    .font(.caption)
                    .accessibilityIdentifier("grant.revoke.\(grant.user_id)")
            }
        }
        .padding(.vertical, 4)
        .accessibilityIdentifier("grant.row.\(grant.user_id)")
    }

    private func roleEditorSheet(_ grant: ProjectGrant) -> some View {
        NavigationStack {
            Form {
                Section("角色") {
                    Picker("角色", selection: $editorRole) {
                        Text("编辑").tag("EDITOR")
                        Text("审阅者").tag("REVIEWER")
                        Text("观察者").tag("VIEWER")
                    }
                    .pickerStyle(.segmented)
                }
                if editorRole == "EDITOR" {
                    Section("编辑专属权限") {
                        Toggle("可审批提案", isOn: $editorApprove)
                        Toggle("可正式编辑", isOn: $editorFormal)
                    }
                } else {
                    Section {
                        Text("审阅者与观察者不能持有审批或正式编辑权限。")
                            .font(.footnote).foregroundStyle(.secondary)
                    }
                }
            }
            .navigationTitle("调整 \(String(grant.email.prefix(18)))")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("取消") { editorTarget = nil } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("保存") {
                        let id = grant.user_id, role = editorRole, approve = editorApprove, formal = editorFormal
                        editorTarget = nil
                        Task { await model.updateGrant(userID: id, role: role, canApprove: approve, canEditFormal: formal) }
                    }
                    .accessibilityIdentifier("grant.save")
                }
            }
        }
        .presentationDetents([.medium])
    }

    private func roleTitle(_ role: String) -> String {
        switch role {
        case "EDITOR": return "编辑"
        case "REVIEWER": return "审阅者"
        case "VIEWER": return "观察者"
        default: return role
        }
    }

    private func privilegeSuffix(_ grant: ProjectGrant) -> String {
        guard grant.role == "EDITOR" else { return "" }
        var parts: [String] = []
        if grant.can_approve { parts.append("可审批") }
        if grant.can_edit_formal { parts.append("可正式编辑") }
        return parts.isEmpty ? "" : " · " + parts.joined(separator: "、")
    }
}
