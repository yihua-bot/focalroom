import SwiftUI
import FocalRoomCore

// 提案审阅：SUBMITTED 提案逐项勾选部分采纳，或整体驳回；走项目命令权威与幂等事务。
struct ProposalsView: View {
    let model: AppModel
    @State private var detail: Proposal?

    private var submitted: [Proposal] {
        (model.snapshot?.proposals ?? []).filter(\.isSubmitted)
    }

    private var decided: [Proposal] {
        (model.snapshot?.proposals ?? []).filter(\.isDecided)
    }

    var body: some View {
        Group {
            if model.snapshot == nil {
                EmptyWorkspace(title: "选择一个项目", detail: "查看该项目等待审阅的提案。", symbol: "checklist")
            } else if model.offline {
                EmptyWorkspace(title: "提案需要联网", detail: "离线时可查看图库与本机资料。", symbol: "wifi.slash")
            } else if submitted.isEmpty && decided.isEmpty {
                EmptyWorkspace(title: "还没有提案", detail: "AI 或协作者提交的作品序列建议会显示在这里。", symbol: "checklist")
            } else {
                List {
                    if !submitted.isEmpty {
                        Section("待审阅") {
                            ForEach(submitted) { proposal in
                                row(proposal)
                            }
                        }
                    }
                    if !decided.isEmpty {
                        Section("已处理") {
                            ForEach(decided) { proposal in
                                row(proposal, dimmed: true)
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle("提案").navigationBarTitleDisplayMode(.inline)
        .refreshable { await model.reloadSnapshot() }
        .sheet(item: $detail) { proposal in
            ProposalDetailView(model: model, proposal: proposal)
        }
    }

    private func row(_ proposal: Proposal, dimmed: Bool = false) -> some View {
        Button {
            if proposal.isSubmitted { detail = proposal }
        } label: {
            VStack(alignment: .leading, spacing: 6) {
                HStack {
                    Text(proposal.title).font(.headline).foregroundStyle(.primary)
                    Spacer()
                    Text(proposal.isSubmitted ? "待审阅" : proposal.state == "APPLIED" ? "已采纳" : "已驳回")
                        .font(.caption).foregroundStyle(proposal.isSubmitted ? .orange : .secondary)
                }
                Text("\(proposal.author) · \(proposal.operations.count) 项操作 · 基线版本 \(proposal.base_revision)")
                    .font(.caption).foregroundStyle(.secondary)
            }.padding(.vertical, 6)
            .opacity(dimmed ? 0.6 : 1)
        }
        .accessibilityIdentifier("proposals.row.\(proposal.id)")
    }
}

private struct ProposalDetailView: View {
    let model: AppModel
    let proposal: Proposal
    @Environment(\.dismiss) private var dismiss
    @State private var selected: Set<String> = []
    @State private var busy = false
    @State private var notice: String?
    @State private var confirmAccept = false
    @State private var confirmReject = false
    @State private var projection: ProposalDiff?
    @State private var projectionFailed = false

    var body: some View {
        NavigationStack {
            Form {
                Section("提案") {
                    LabeledContent("标题", value: proposal.title)
                    LabeledContent("作者", value: proposal.author)
                    LabeledContent("基线版本", value: "\(proposal.base_revision)")
                }
                Section("操作（勾选要采纳的项）") {
                    ForEach(proposal.operations) { operation in
                        // 用按钮行而非系统开关：iOS 26 表单内 Switch 命中点不可靠，按钮行点击稳定且语义相同。
                        Button {
                            if selected.contains(operation.operation_id) {
                                selected.remove(operation.operation_id)
                            } else {
                                selected.insert(operation.operation_id)
                            }
                        } label: {
                            HStack {
                                Image(systemName: selected.contains(operation.operation_id) ? "checkmark.circle.fill" : "circle")
                                    .foregroundStyle(selected.contains(operation.operation_id) ? Color.accentColor : Color.secondary)
                                Text(operationPhotoTitle(operation))
                                Spacer()
                                Text(operation.title).font(.caption).foregroundStyle(.secondary)
                            }
                        }
                        .accessibilityIdentifier("proposals.op.\(operation.operation_id)")
                        .accessibilityValue(selected.contains(operation.operation_id) ? "已勾选" : "未勾选")
                    }
                }
                if model.snapshot?.principal.can_approve != true {
                    Section {
                        Text("当前权限为只读，不能处理提案。").font(.footnote).foregroundStyle(.secondary)
                    }
                } else if proposal.isSubmitted {
                    Section {
                        Button("采纳所选（\(selected.count) 项）") { confirmAccept = true }
                            .disabled(busy || selected.isEmpty)
                            .accessibilityIdentifier("proposals.accept")
                        Button("驳回整个提案", role: .destructive) { confirmReject = true }
                            .disabled(busy)
                            .accessibilityIdentifier("proposals.reject")
                    }
                }
                // 投影放最后：补充信息不得把采纳/驳回按钮挤出首屏。
                projectionSection
            }
            .navigationTitle("审阅提案").navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button("关闭") { dismiss() } } }
            .onAppear { selected = Set(proposal.operations.map(\.operation_id)) }
            .task { await loadProjection() }
            .confirmationDialog("采纳所选操作", isPresented: $confirmAccept, titleVisibility: .visible) {
                Button("采纳 \(selected.count) 项", role: .destructive) { Task { await apply(Array(selected)) } }
            } message: {
                Text("未勾选的操作将被忽略；采纳会推进作品正式版本。")
            }
            .confirmationDialog("驳回提案", isPresented: $confirmReject, titleVisibility: .visible) {
                Button("驳回", role: .destructive) { Task { await apply([]) } }
            } message: {
                Text("原提案保留供重建，不会修改作品序列。")
            }
            .alert("提示", isPresented: Binding(get: { notice != nil }, set: { if !$0 { notice = nil } })) {
                Button("知道了", role: .cancel) { notice = nil }
            } message: { Text(notice ?? "") }
        }
    }

    // 服务端投影：stale 表示提案基线与当前版本不一致，采纳会被拒绝、原提案保留。
    @ViewBuilder
    private var projectionSection: some View {
        Section("采纳后序列预览（服务端投影）") {
            if let projection {
                if projection.stale {
                    Label("提案基线已过期（基于版本 \(projection.baseRevision)，当前版本 \(projection.currentRevision)）；采纳会被服务端拒绝，原提案保留供重建。",
                          systemImage: "exclamationmark.triangle")
                        .font(.footnote).foregroundStyle(.orange)
                } else if let change = projection.sequence {
                    if !change.added.isEmpty {
                        LabeledContent("新增", value: change.added.map { photoFilename($0) }.joined(separator: "、"))
                            .font(.footnote)
                    }
                    if !change.removed.isEmpty {
                        LabeledContent("移除", value: change.removed.map { photoFilename($0) }.joined(separator: "、"))
                            .font(.footnote)
                    }
                    ForEach(change.moved, id: \.photo_id) { move in
                        Text("移动 \(photoFilename(move.photo_id))：第 \(move.fromIndex + 1) 位 → 第 \(move.toIndex + 1) 位")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                    ForEach(Array(change.after.enumerated()), id: \.offset) { index, photoID in
                        HStack {
                            Text(photoFilename(photoID)).font(.subheadline)
                            Spacer()
                            Text("第 \(index + 1) 位").font(.caption).foregroundStyle(.secondary)
                        }
                    }
                } else {
                    Text("暂无投影。").font(.footnote).foregroundStyle(.secondary)
                }
            } else if !projectionFailed {
                HStack(spacing: 8) {
                    ProgressView()
                    Text("读取投影…").foregroundStyle(.secondary)
                }
            }
        }
    }

    private func photoFilename(_ photoID: String) -> String {
        model.snapshot?.snapshot.photos.first { $0.id == photoID }?.filename ?? photoID
    }

    private func loadProjection() async {
        do { projection = try await model.proposalDiff(proposalID: proposal.id) }
        catch { projectionFailed = true }
    }

    private func operationPhotoTitle(_ operation: Proposal.Operation) -> String {
        let filename = model.snapshot?.snapshot.photos.first { $0.id == operation.photo_id }?.filename ?? operation.photo_id
        return "\(operation.title) · \(filename)"
    }

    private func apply(_ accepted: [String]) async {
        guard !busy else { return }
        busy = true
        defer { busy = false }
        do {
            try await model.applyProposal(proposal: proposal, acceptedOperationIDs: accepted)
            dismiss()
        } catch {
            notice = error.localizedDescription
        }
    }
}
