import SwiftUI
import FocalRoomCore

struct NativePreferenceView: View {
    let model: AppModel
    let task: NativeAITask
    let preference: NativeAIPreference
    @State private var statement = ""
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("项目偏好建议").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
            if preference.state == "pending" {
                TextField("偏好说明", text: $statement, axis: .vertical).font(.subheadline)
                Text(preference.applicability).font(.footnote).foregroundStyle(.secondary)
                Text("确认后仅用于当前项目。").font(.caption).foregroundStyle(.secondary)
                HStack(spacing: 18) {
                    Button("保存偏好") { Task { await model.saveNativePreference(preference, task: task, statement: statement) } }
                        .disabled(statement.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || statement.count > 500)
                    Button("暂不记录") { Task { await model.saveNativePreference(preference, task: task, statement: nil) } }
                }.font(.subheadline).disabled(model.busy)
            } else {
                Text(preference.statement).font(.subheadline)
                Text(preference.state == "saved" ? String(localized: "已保存") : String(localized: "未用于当前偏好")).font(.caption).foregroundStyle(.secondary)
            }
        }.padding(14).background(Color.white.opacity(0.05), in: RoundedRectangle(cornerRadius: 12))
            .onAppear { statement = preference.statement }
    }
}

struct NativeDraftReviewView: View {
    let model: AppModel
    let draft: NativeAIDraft
    @State private var selected: Set<String> = []
    @State private var confirming = false
    @State private var reviewError: String?
    @Environment(\.dismiss) private var dismiss

    private var canApply: Bool {
        !draft.stale && !selected.isEmpty && !model.busy && draft.state == "pending" &&
            model.nativeProjection?.tasks.contains(where: { $0.id == draft.taskId && $0.needsReview }) == true
    }
    // 口径挂在任务上、按草案编号取：同一轮里可能同时有好几份方案，各说各的档位。
    private var evidence: NativeDraftEvidence? {
        model.nativeProjection?.tasks.first(where: { $0.id == draft.taskId })?
            .draftEvidence?.first(where: { $0.draftId == draft.id })
    }
    var body: some View {
        NavigationStack {
            List {
                Section {
                    Text(draft.title).font(.headline)
                    Text(draft.summary).font(.subheadline)
                    Text(draft.stale ? String(localized: "来源项目已更新。草案保留供参考，请重新委托后再采纳。") : String(localized: "勾选要采纳的建议。确认后才会改变本机选片，或创建待审批的云端提案。"))
                        .font(.footnote).foregroundStyle(.secondary)
                }
                NativeDraftEvidenceSection(evidence: evidence, draftRevision: draft.revision)
                    .accessibilityIdentifier("ai.review.evidence")
                if let reviewError { Section { Text(reviewError).font(.footnote).foregroundStyle(.red) } }
                Section("建议 \(draft.items.count) 项") {
                    ForEach(Array(draft.items.enumerated()), id: \.offset) { _, item in
                        Button {
                            if selected.contains(item.photo_id) { selected.remove(item.photo_id) } else { selected.insert(item.photo_id) }
                        } label: {
                            HStack(alignment: .top, spacing: 12) {
                                Image(systemName: selected.contains(item.photo_id) ? "checkmark.circle.fill" : "circle")
                                NativePhotoPreview(model: model, coreID: item.photo_id).frame(width: 64, height: 64)
                                VStack(alignment: .leading, spacing: 5) {
                                    Text(model.nativePhotoName(item.photo_id)).font(.subheadline.weight(.semibold))
                                    Text(actionTitle(item.proposed_action, position: item.position)).font(.caption)
                                    Text(item.rationale).font(.footnote).foregroundStyle(.secondary)
                                    if let tierNote = tierNote(for: item.photo_id) {
                                        Text(tierNote).font(.caption).foregroundStyle(.secondary)
                                    }
                                }
                            }.foregroundStyle(.primary)
                        }.buttonStyle(.plain)
                            .accessibilityIdentifier("ai.review.item.\(item.photo_id)")
                    }
                }
            }
            .navigationTitle("审阅选片草案").navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("关闭") { dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button(model.selectedLocalProjectID != nil ? String(localized: "采纳勾选项") : String(localized: "提交为提案")) { confirming = true }
                        .disabled(!canApply).accessibilityIdentifier("ai.review.apply")
                }
            }
            .onAppear { selected = Set(draft.items.map(\.photo_id)) }
            .confirmationDialog("确认采纳 \(selected.count) 项建议？", isPresented: $confirming, titleVisibility: .visible) {
                Button(model.selectedLocalProjectID != nil ? String(localized: "采纳到本机项目") : String(localized: "创建云端提案")) {
                    Task {
                        if await model.applyNativeDraft(draft, selectedIDs: selected) { dismiss() }
                        else { reviewError = model.notice ?? String(localized: "尚未采纳，请刷新项目后重试。"); model.notice = nil }
                    }
                }
                Button("取消", role: .cancel) {}
            }
        }
    }
    private func actionTitle(_ action: String, position: Int) -> String {
        switch action {
        case "keep": model.selectedLocalProjectID != nil ? String(localized: "保留 · 第 \(position + 1) 位") : String(localized: "加入作品序列 · 第 \(position + 1) 位")
        case "reject": model.selectedLocalProjectID != nil ? String(localized: "排除") : String(localized: "移出作品序列")
        case "undecided": model.selectedLocalProjectID != nil ? String(localized: "待选") : String(localized: "移出作品序列")
        default: String(localized: "需要核对的建议")
        }
    }
    // 逐张口径只在这张确实被记过时说话：没记过就什么都不加，不拿"没有记录"当作"没有观察"。
    private func tierNote(for photoID: String) -> String? {
        guard let entry = evidence?.perPhoto.first(where: { $0.photoId == photoID }) else { return nil }
        if entry.used {
            switch entry.contract {
            case "current_page_tier": return String(localized: "这张按当前页面档看的画面")
            case "current_detail_tier": return String(localized: "这张按大尺寸细节档看的画面，比页面档看得更细")
            case "other_prompt_version": return String(localized: "这张按更早的问法看的画面")
            case "other_tool": return String(localized: "这张用的是另一个工具问出的画面记录")
            default: return nil
            }
        }
        // 原图不在这台机器上、原图已经换过的，本地修不回来；这句话比"预览坏了"更该先说。
        switch entry.originalStatus {
        case "missing": return String(localized: "这张的画面记录没用上：原图不在这台机器上，本地修不回来")
        case "changed": return String(localized: "这张的画面记录没用上：原图已经换过，本地修不回来")
        default: break
        }
        switch entry.pixelVersion {
        case "mismatch": return String(localized: "这张的画面记录没用上：当前预览和当初看的那份对不上")
        case "unreadable": return String(localized: "这张的画面记录没用上：本地预览读不出来")
        case "unrecorded": return String(localized: "这张的画面记录没用上：没记下当初看的是哪份图像")
        default: return nil
        }
    }
}

/// 草案的"画面依据"口径一族：哪几档问法挑出来的、对着方案的哪一版算的、哪几张没采用。
/// 手机上只读这些数字，不重算——重算入口目前在桌面端。
struct NativeDraftEvidenceSection: View {
    let evidence: NativeDraftEvidence?
    let draftRevision: Int

    var body: some View {
        Section {
            if let evidence {
                Text(evidence.observed > 0
                    ? String(localized: "这份方案的画面依据：\(evidence.observed) 张有可核对的画面观察，\(evidence.unobserved) 张没有。")
                    : String(localized: "这份方案的照片都还没有可用的画面观察，下面的判断只靠评分、状态与顺序。"))
                    .font(.footnote)
                Text("同一张照片只取一条观察，按问法分档：当前页面档 \(evidence.pageTier) 张 · 大尺寸细节档 \(evidence.detailTier) 张 · 更早问法 \(evidence.otherVersion) 张 · 其它工具 \(evidence.otherTool) 张。")
                    .font(.caption).foregroundStyle(.secondary)
                if let then = evidence.draftRevision {
                    Text(then == draftRevision
                        ? String(localized: "上面的口径是对着方案第 \(then) 版算的，就是现在这一版。")
                        : String(localized: "上面的口径是对着方案第 \(then) 版算的，现在已是第 \(draftRevision) 版：中间改过，口径没重算。"))
                        .font(.caption).foregroundStyle(.secondary)
                }
                if evidence.crossContractPairs > 0 {
                    Text("有 \(evidence.crossContractPairs) 处相邻两张不是同一档问法问出来的，不能当成同一把尺子比。")
                        .font(.caption).foregroundStyle(.secondary)
                }
                if evidence.omitted > 0 {
                    Text("另有 \(evidence.omitted) 张的画面记录因为原图换过或本地预览读不出来，没有采用。")
                        .font(.caption).foregroundStyle(.secondary)
                }
                // 降级那一份的计数是就地取的，不是猜的；但逐张档位真没读回来，就得照着说。
                if evidence.basisSummary {
                    Text("这份方案算过，只是那份核验单没能整条读回来，逐张的档位这里就不写了。")
                        .font(.caption).foregroundStyle(.secondary)
                }
                if evidence.observed > evidence.scanned {
                    Text("逐张口径只点到下面列出的那几张，其余有画面观察的照片只算进总数。")
                        .font(.caption).foregroundStyle(.secondary)
                }
                // 口径可能对不上时才开口（版本号对不上，或有画面的记录被排除），本来就在说"手机上重算不了"。
                if evidence.omitted > 0 || evidence.draftRevision != draftRevision {
                    Text("要按现在这一版重算画面依据，得在桌面端打开这份草案；手机上只读这些数字，不重算，也不会因此改动草案。")
                        .font(.caption).foregroundStyle(.secondary)
                }
            } else {
                // 这一格说的是"没有能核对上的画面依据"：既包括从来没算过的，也包括算过但核验单整条读不回来的——
                // 手机这边分辨不了，所以话只能说到这里，不许猜成"没算过"。
                Text("这份方案还没有能核对的画面依据，下面的判断只靠评分、状态与顺序。")
                    .font(.footnote)
            }
        } header: {
            Text("画面依据").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
        }
    }
}

struct NativeBoardReviewView: View {
    let model: AppModel
    let board: NativeAIBoard
    @State private var nodes: [NativeAINode] = []
    @State private var error: String?
    @State private var confirming = false
    @Environment(\.dismiss) private var dismiss
    private var supported: Bool {
        !nodes.isEmpty && nodes.allSatisfy {
            ["photo", "text"].contains($0.nodeType) && $0.visible && $0.rotation == 0 &&
                (40...4000).contains($0.width) && (40...4000).contains($0.height) && $0.zOrder >= 0
        }
    }
    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    Text(board.title).font(.headline)
                    if let error { Text(error).foregroundStyle(.secondary) }
                    if !nodes.isEmpty {
                        NativeBoardLayout(model: model, nodes: nodes).frame(height: 320)
                            .background(Color.white.opacity(0.04), in: RoundedRectangle(cornerRadius: 12))
                        Text("\(nodes.count) 个节点 · 草案版本 \(board.revision)").font(.caption).foregroundStyle(.secondary)
                        if !supported {
                            Text("此草案包含手机画布尚不支持的节点或变换，当前只能预览，暂不能采纳。")
                                .font(.footnote).foregroundStyle(.secondary)
                        }
                        if model.selectedLocalProjectID == nil {
                            Text("此处仅预览当前设备保存的画布建议，尚未接入云端画布发布。")
                                .font(.footnote).foregroundStyle(.secondary)
                        }
                    } else if error == nil { ProgressView("读取草案…") }
                }.padding(20)
            }
            .navigationTitle("审阅画布草案").navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("关闭") { dismiss() } }
                if model.selectedLocalProjectID != nil {
                    ToolbarItem(placement: .confirmationAction) {
                        Button("添加到画布") { confirming = true }.disabled(!supported || model.busy)
                    }
                }
            }
            .task {
                do { nodes = try await model.nativeBoardNodes(board) }
                catch { self.error = String(localized: "无法读取这份草案，可能已经更新，请关闭后刷新。") }
            }
            .confirmationDialog("将草案作为新的一组内容添加到本机画布？", isPresented: $confirming, titleVisibility: .visible) {
                Button("确认添加") { Task {
                    if await model.applyNativeBoard(board, nodes: nodes) { dismiss() }
                    else { error = model.notice ?? String(localized: "尚未添加，请刷新草案后重试。"); model.notice = nil }
                } }
                Button("取消", role: .cancel) {}
            }
        }
    }
}

private struct NativePhotoPreview: View {
    let model: AppModel
    let coreID: String
    @State private var image: UIImage?
    var body: some View {
        Group {
            if let image { Image(uiImage: image).resizable().scaledToFit() }
            else { Image(systemName: "photo").foregroundStyle(.secondary) }
        }.task(id: coreID) { image = try? await model.nativePhotoImage(coreID) }
    }
}

private struct NativeBoardLayout: View {
    let model: AppModel
    let nodes: [NativeAINode]
    var body: some View {
        GeometryReader { geometry in
            let visible = nodes.filter(\.visible)
            let minX = visible.map(\.x).min() ?? 0, minY = visible.map(\.y).min() ?? 0
            let width = max(1, (visible.map { $0.x + $0.width }.max() ?? 1) - minX)
            let height = max(1, (visible.map { $0.y + $0.height }.max() ?? 1) - minY)
            let scale = min((geometry.size.width - 20) / width, (geometry.size.height - 20) / height)
            ZStack(alignment: .topLeading) {
                ForEach(visible.sorted { $0.zOrder < $1.zOrder }) { node in
                    Group {
                        if node.nodeType == "photo", let asset = node.assetId {
                            NativePhotoPreview(model: model, coreID: asset)
                        } else if node.nodeType == "text", let content = node.contentJson,
                                  let object = try? JSONSerialization.jsonObject(with: Data(content.utf8)) as? [String: Any] {
                            Text(object["text"] as? String ?? String(localized: "文字")).font(.system(size: max(7, 14 * scale)))
                        } else { Image(systemName: "square.dashed").foregroundStyle(.secondary) }
                    }
                    .frame(width: max(1, node.width * scale), height: max(1, node.height * scale))
                    .rotationEffect(.degrees(node.rotation))
                    .position(x: (node.x - minX + node.width / 2) * scale + 10,
                              y: (node.y - minY + node.height / 2) * scale + 10)
                }
            }.frame(width: geometry.size.width, height: geometry.size.height).clipped()
        }
    }
}
