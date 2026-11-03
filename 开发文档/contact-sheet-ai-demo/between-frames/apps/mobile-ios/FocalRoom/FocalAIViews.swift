import SwiftUI
import FocalRoomCore

struct FocalAIView: View {
    let model: AppModel
    @State private var draft = ""
    @State private var results: [AgentResult] = []
    @State private var jobs: [AgentJob] = []
    @State private var tasksError: String?
    @State private var selectedPhotos: Set<String> = []
    @State private var preparingVision = false
    @State private var activeSheet: ActiveSheet?
    @State private var followingConversation = true
    @State private var followSettledAt = Date.distantPast
    @State private var scrollRequest = 0
    @State private var streamStartedHere = false
    @State private var streamProject: ProjectReference?
    @State private var projectDrafts: [ProjectReference?: String] = [:]
    @State private var legacyMessages: [AIChatMessage] = []
    @FocusState private var inputFocused: Bool

    private enum ActiveSheet: Identifiable {
        case photos, tasks, legacy, vision(PreparedVision), draft(NativeAIDraft), board(NativeAIBoard)
        var id: String {
            switch self {
            case .photos: "photos"
            case .tasks: "tasks"
            case .legacy: "legacy"
            case .draft(let draft): "draft-" + draft.id
            case .board(let board): "board-" + board.id
            case .vision(let prepared): "vision-\(prepared.id)"
            }
        }
    }

    private var hasPending: Bool {
        model.chatMessages.contains { $0.state == .unknown && $0.role == "user" && !isSending($0) }
    }

    private func isSending(_ message: AIChatMessage) -> Bool {
        model.aiSending && streamStartedHere && streamProject == model.currentProject &&
            model.chatMessages.last(where: { $0.role == "user" })?.id == message.id
    }

    var body: some View {
        VStack(spacing: 0) {
            ProjectContext(model: model)
            Picker("AI 功能", selection: Binding(get: { model.aiMode }, set: { model.aiMode = $0 })) {
                ForEach(AppModel.AIMode.allCases, id: \.self) { mode in Text(LocalizedStringKey(mode.rawValue)).tag(mode) }
            }.pickerStyle(.segmented).padding(.horizontal, 18).padding(.vertical, 8)
                .disabled(model.aiSending || preparingVision)
                .accessibilityIdentifier("ai.mode")
            statusBar
            if let receipt = model.nativeReviewReceipt {
                Text(receipt).font(.footnote).foregroundStyle(.green)
                    .frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 18).padding(.vertical, 8)
                    .accessibilityIdentifier("ai.review.receipt")
            }
            conversation
            inputBar
        }
        .background(Color(white: 0.055))
        .navigationTitle("Focal AI").navigationBarTitleDisplayMode(.inline)
        .task(id: model.credentials?.user.id) {
            await model.loadAIAccount()
        }
        .task(id: model.currentProject) {
            await model.loadChat()
            await model.aiResolvePending()
            await loadTasks()
            while !Task.isCancelled {
                await model.refreshNativeTasks()
                try? await Task.sleep(for: .seconds(2))
            }
        }
        .onChange(of: model.aiMode) { _, _ in
            clearPhotos()
        }
        .onChange(of: model.currentProject) { old, new in
            projectDrafts[old] = draft
            draft = projectDrafts[new] ?? ""
            resetContext()
        }
        .onChange(of: model.credentials?.user.id) { _, _ in
            projectDrafts.removeAll()
            draft = ""
            resetContext()
        }
        .sheet(item: $activeSheet) { sheet in
            switch sheet {
            case .photos:
                photoPicker
            case .tasks:
                tasksSheet
            case .legacy:
                legacySheet
            case .draft(let draft):
                NativeDraftReviewView(model: model, draft: draft)
            case .board(let board):
                NativeBoardReviewView(model: model, board: board)
            case .vision(let prepared):
                VisionConsentView(draft: prepared) {
                    // 授权只消费一次；发送任务先消费授权，再主动收起确认页。
                    beginFollowingRequest()
                    let origin = model.currentProject
                    Task {
                        let sent = await model.aiSend(text: prepared.text, vision: prepared,
                                                      onVisionConsumed: { activeSheet = nil })
                        if sent {
                            clearSentDraft(for: origin)
                        } else {
                            // 被拒时收起确认页：浮层还开着会把提示 alert 挡在后面，看起来像点了没反应。
                            activeSheet = nil
                        }
                    }
                } cancel: { activeSheet = nil }
                .onDisappear { model.cancelVision() }
            }
        }
    }

    private var statusBar: some View {
        HStack(spacing: 8) {
            if model.aiSending && !(streamStartedHere && streamProject == model.currentProject) {
                Label("其他请求处理中", systemImage: "hourglass")
                    .accessibilityIdentifier("ai.balance")
            } else if let account = model.aiAccountState {
                Group {
                    if !account.enabled {
                        Label("AI 暂未开放", systemImage: "circle")
                    } else if !account.verified {
                        Label("请先验证邮箱", systemImage: "envelope")
                    } else if account.available == 0 {
                        Label("暂无可用额度", systemImage: "circle")
                    } else if model.offline {
                        Label("当前离线", systemImage: "wifi.slash")
                    } else {
                        Label("可用额度 \(account.available.formatted())", systemImage: "sparkle")
                    }
                }
                .accessibilityIdentifier("ai.balance")
            } else if let error = model.aiAccountError {
                Button { Task { await model.loadAIAccount() } } label: {
                    Label("服务状态读取失败，重试", systemImage: "arrow.clockwise")
                }
                .accessibilityHint(error)
                    .accessibilityIdentifier("ai.account.retry")
            } else {
                ProgressView().controlSize(.mini)
                Text("读取 AI 状态…")
                    .accessibilityIdentifier("ai.balance")
            }
            Spacer(minLength: 8)
            Button { activeSheet = .tasks } label: {
                HStack(spacing: 5) {
                    Text("项目动态")
                    if !jobs.isEmpty || !results.isEmpty { Text("\(jobs.count + results.count)").monospacedDigit() }
                    Image(systemName: "chevron.right").font(.system(size: 9, weight: .semibold))
                }
                .frame(minHeight: 44)
                .contentShape(Rectangle())
            }
            .accessibilityIdentifier("ai.tasks.open")
        }
        .font(.caption)
        .foregroundStyle(.secondary)
        .padding(.horizontal, 18)
        .overlay(alignment: .bottom) { Color.white.opacity(0.08).frame(height: 0.5) }
    }

    private var visibleStreamingText: String? {
        guard streamStartedHere, streamProject == model.currentProject else { return nil }
        return model.streamingText
    }

    private var showingThinking: Bool {
        streamStartedHere && streamProject == model.currentProject && model.aiSending && visibleStreamingText == nil
    }

    private var conversation: some View {
        ScrollViewReader { scroll in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 24) {
                    if (model.aiMode == .chat ? model.chatMessages.isEmpty : model.nativeProjection?.tasks.isEmpty != false) && !showingThinking && visibleStreamingText == nil {
                        welcome
                    }
                    if model.aiMode == .chat {
                        ForEach(model.chatMessages) { message in
                            messageRow(message).id(message.id)
                        }
                    } else {
                        nativeTaskRows
                    }
                    if let streaming = visibleStreamingText {
                        // 内容长度进入标识：增量到达时让 LazyVStack 用新身份重建该行，
                        // 否则与「正在思考…」分支共用同一 id 会让行停在思考态不刷新。
                        assistantRow(text: streaming.isEmpty ? String(localized: "正在思考…") : streaming, streaming: true)
                            .id(streaming.isEmpty ? "ai.streaming" : "ai.streaming-\(streaming.count)")
                    } else if showingThinking {
                        assistantRow(text: String(localized: "正在思考…"), streaming: true)
                            .id("ai.streaming")
                    }
                    draftSection
                    if model.aiMode == .chat && hasPending { pendingNotice }
                    Color.clear.frame(height: 1).id("ai.bottom")
                        .onAppear { follow(bottomVisible: true) }
                        .onDisappear { follow(bottomVisible: false) }
                }
                .padding(.horizontal, 20).padding(.top, 24).padding(.bottom, 18)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .scrollDismissesKeyboard(.interactively)
            .overlay(alignment: .bottomTrailing) {
                if !followingConversation && (model.aiMode == .chat ? !model.chatMessages.isEmpty : model.nativeProjection?.tasks.isEmpty == false) {
                    Button {
                        followSettledAt = Date()
                        followingConversation = true
                        withAnimation(.easeOut(duration: 0.2)) { scroll.scrollTo("ai.bottom", anchor: .bottom) }
                    } label: {
                        Image(systemName: "arrow.down")
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(.primary)
                            .frame(width: 44, height: 44)
                            .background(Color(white: 0.18), in: Circle())
                    }
                    .accessibilityLabel(String(localized: "回到最新消息"))
                    .accessibilityIdentifier("ai.scroll.latest")
                    .padding(.trailing, 18).padding(.bottom, 12)
                }
            }
            .onChange(of: model.chatMessages.last?.id) { _, _ in
                if followingConversation { withAnimation(.easeOut(duration: 0.2)) { scroll.scrollTo("ai.bottom", anchor: .bottom) } }
            }
            .onChange(of: visibleStreamingText) { _, _ in
                if followingConversation { scroll.scrollTo("ai.bottom", anchor: .bottom) }
            }
            .onChange(of: model.nativeProjection?.tasks.first?.messages.last?.id) { _, _ in
                if followingConversation { withAnimation(.easeOut(duration: 0.2)) { scroll.scrollTo("ai.bottom", anchor: .bottom) } }
            }
            .onChange(of: scrollRequest) { _, _ in
                withAnimation(.easeOut(duration: 0.2)) { scroll.scrollTo("ai.bottom", anchor: .bottom) }
            }
        }
    }

    private var welcome: some View {
        VStack(alignment: .leading, spacing: 16) {
            Image(systemName: "sparkle")
                .font(.system(size: 22, weight: .light))
                .foregroundStyle(.white.opacity(0.8))
                .frame(width: 44, height: 44, alignment: .leading)
            Text("从一张照片，\n开始对话。")
                .font(.system(.title2, design: .serif).weight(.medium))
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("ai.welcome")
            Text("一起整理线索、讨论选片和顺序。需要看图时，点下方 + 添加照片。")
                .font(.subheadline).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            VStack(alignment: .leading, spacing: 0) {
                suggestion(String(localized: "怎样制定一组照片的选片标准？"))
                suggestion(String(localized: "给我一些排列照片的思路"))
            }.padding(.top, 6)
        }
        .padding(.vertical, 18)
    }

    private func suggestion(_ text: String) -> some View {
        Button {
            if draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                draft = text
            } else {
                draft += (draft.hasSuffix("\n") ? "" : "\n") + text
            }
            inputFocused = true
        } label: {
            HStack(spacing: 10) {
                Text(text).multilineTextAlignment(.leading)
                Spacer(minLength: 4)
                Image(systemName: "arrow.up.left").font(.caption)
            }
            .font(.subheadline).foregroundStyle(.white.opacity(0.75))
            .frame(minHeight: 48).contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .overlay(alignment: .bottom) { Color.white.opacity(0.08).frame(height: 0.5) }
    }

    private var pendingNotice: some View {
        HStack(alignment: .center, spacing: 10) {
            Image(systemName: "clock.arrow.circlepath").foregroundStyle(.orange)
            Text("正在确认结果，完成后自动显示").font(.caption).foregroundStyle(.secondary)
            Spacer()
            Button("核对结果") { Task { await model.aiResolvePending(manual: true) } }
                .font(.caption.weight(.medium)).frame(minHeight: 44)
                .accessibilityIdentifier("ai.resolve")
        }
    }

    @ViewBuilder
    private func messageRow(_ message: AIChatMessage) -> some View {
        if message.role == "user" {
            HStack(alignment: .top) {
                Spacer(minLength: 60)
                VStack(alignment: .trailing, spacing: 6) {
                    Text(message.text).textSelection(.enabled)
                        .font(.body).lineSpacing(3)
                        .padding(.horizontal, 15).padding(.vertical, 11)
                        .background(Color.white.opacity(0.11), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
                    if message.state == .unknown && isSending(message) {
                        Text("发送中…").font(.caption2).foregroundStyle(.secondary)
                    } else if message.state == .unknown {
                        Label("结果待确认", systemImage: "clock.arrow.circlepath")
                            .font(.caption2).foregroundStyle(.orange)
                    } else if message.state == .failed, let failure = message.failure {
                        Label(failure, systemImage: "exclamationmark.circle")
                            .font(.caption2).foregroundStyle(.red)
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .trailing)
            .accessibilityIdentifier("ai.message.\(message.id)")
        } else {
            assistantRow(text: message.text, streaming: false)
                .accessibilityIdentifier("ai.message.\(message.id)")
        }
    }

    private func assistantRow(text: String, streaming: Bool) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: "sparkle")
                .font(.system(size: 14, weight: .medium))
                .foregroundStyle(.white.opacity(0.8))
                .frame(width: 28, height: 28)
                .background(Color.white.opacity(0.08), in: Circle())
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 8) {
                Text("Focal AI").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                if streaming && text == String(localized: "正在思考…") {
                    HStack(spacing: 6) {
                        ProgressView().controlSize(.mini)
                        Text("正在思考…").font(.subheadline).foregroundStyle(.secondary)
                    }
                } else {
                    AIResponseText(text: text)
                    if streaming { Text("正在输入…").font(.caption2).foregroundStyle(.secondary) }
                }
            }.frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    @ViewBuilder private var draftSection: some View {
        if model.aiMode == .task {
            if let error = model.nativeTaskError {
                Text(error).font(.footnote).foregroundStyle(.secondary)
            }
            ForEach(model.nativeProjection?.drafts ?? []) { draft in
                let applied = model.selectedLocalProject?.appliedAIDraftIDs?.contains(where: { $0.hasPrefix(draft.id + ":") }) == true
                Button { activeSheet = .draft(draft) } label: {
                    VStack(alignment: .leading, spacing: 8) {
                        Label("选片草案 · " + (applied ? String(localized: "已采纳建议") : draft.stale ? String(localized: "来源已更新") : String(localized: "等待审阅")), systemImage: "sparkles")
                            .font(.caption).foregroundStyle(.secondary)
                        Text(draft.title).font(.headline)
                        Text(draft.summary).font(.subheadline).foregroundStyle(.secondary)
                        Text("查看 \(draft.items.count) 项建议").font(.subheadline.weight(.semibold))
                    }.frame(maxWidth: .infinity, alignment: .leading).padding(16)
                        .background(Color.white.opacity(0.055), in: RoundedRectangle(cornerRadius: 16))
                }.buttonStyle(.plain).accessibilityIdentifier("ai.shared.draft.\(draft.id)")
            }
            ForEach((model.nativeProjection?.tasks ?? []).flatMap(\.boardDrafts)) { board in
                Button { activeSheet = .board(board) } label: {
                    Label("查看画布草案 · " + board.title, systemImage: "rectangle.3.group")
                        .font(.subheadline).frame(minHeight: 44)
                }.buttonStyle(.plain)
            }
        }
    }

    @ViewBuilder private var nativeTaskRows: some View {
        ForEach(Array((model.nativeProjection?.tasks ?? []).reversed())) { task in
            VStack(alignment: .leading, spacing: 14) {
                ForEach(task.messages) { message in
                    if message.role == "user" {
                        Text(message.content).font(.body).padding(12)
                            .background(Color.white.opacity(0.08), in: RoundedRectangle(cornerRadius: 12))
                    } else { assistantRow(text: message.content, streaming: false) }
                }
                HStack {
                    if task.isActive { ProgressView().controlSize(.mini) }
                    Text(taskStatus(task.state)).font(.caption.weight(.semibold))
                }.foregroundStyle(.secondary)
                if !task.reason.isEmpty && !task.isActive {
                    Text(task.reason.contains("AI_RESULT_UNKNOWN") ? String(localized: "结果尚未确认，请先核对，避免重复提交。") : task.reason)
                        .font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
                }
                if let question = task.question, task.state == "needs_input" {
                    Text(question).font(.subheadline)
                    Text("在下方输入你的答复，继续这次任务。").font(.caption).foregroundStyle(.secondary)
                }
                if !task.inspectedPhotos.isEmpty {
                    DisclosureGroup(String(localized: "已实际观察 \(task.inspectedPhotos.count) 张照片")) {
                        ForEach(task.inspectedPhotos) { observation in
                            VStack(alignment: .leading, spacing: 6) {
                                Text(model.nativePhotoName(observation.photo_id)).font(.subheadline.weight(.semibold))
                                Text(observation.visible_facts.joined(separator: "；")).font(.footnote)
                                Text("编辑解读：" + observation.interpretation).font(.footnote).foregroundStyle(.secondary)
                                Text("不确定性：" + observation.uncertainty).font(.caption).foregroundStyle(.secondary)
                            }.padding(.vertical, 8)
                        }
                    }.font(.subheadline)
                }
                ForEach(task.preferenceProposals) { preference in
                    NativePreferenceView(model: model, task: task, preference: preference)
                }
                HStack(spacing: 18) {
                    if task.isActive {
                        Button("暂停") { Task { await model.controlNativeTask(task, action: "PauseTask") } }
                        Button("取消任务") { Task { await model.controlNativeTask(task, action: "CancelTask") } }
                    } else if task.state == "paused" {
                        Button("核对结果") { Task { await model.controlNativeTask(task, action: "query") } }
                        Button("继续") { Task { await model.controlNativeTask(task, action: "ResumeTask") } }
                        Button("取消任务") { Task { await model.controlNativeTask(task, action: "CancelTask") } }
                    }
                }.font(.subheadline).buttonStyle(.borderless).disabled(model.aiSending)
            }.accessibilityIdentifier("ai.shared.task.\(task.id)")
        }
    }

    private func taskStatus(_ state: String) -> String {
        switch state {
        case "queued": String(localized: "排队中")
        case "planning", "running": String(localized: "正在处理")
        case "paused": String(localized: "已暂停")
        case "needs_input": String(localized: "等待你的答复")
        case "review_ready": String(localized: "草案等待审阅")
        case "completed": String(localized: "已完成")
        case "cancelled": String(localized: "已取消")
        case "failed": String(localized: "任务已停止")
        case "stale": String(localized: "来源已更新")
        default: String(localized: "状态待核对")
        }
    }

    private var tasksSection: some View {
        Section("项目分析任务") {
            if model.selectedLocalProjectID != nil {
                Text("可直接选择本机照片向 AI 提问，无需迁云。云端项目分析任务不会出现在本机项目中。")
                    .font(.footnote).foregroundStyle(.secondary)
            } else if let tasksError {
                VStack(alignment: .leading, spacing: 8) {
                    Text(tasksError).font(.footnote).foregroundStyle(.secondary)
                    Button("重试") { Task { await loadTasks() } }
                        .font(.footnote.weight(.semibold)).frame(minHeight: 36)
                        .disabled(model.offline)
                        .accessibilityIdentifier("ai.tasks.retry")
                }
            } else if jobs.isEmpty && results.isEmpty {
                Text("暂无任务。项目内已有的分析任务与评论汇总会显示在这里。")
                    .font(.footnote).foregroundStyle(.secondary)
            }
            ForEach(jobs) { job in
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(job.kind == "summarize_disagreement" ? String(localized: "评论分歧分析") : String(localized: "项目评论分析")).font(.subheadline.weight(.semibold))
                        Spacer()
                        Text(jobStatus(job.status)).font(.caption)
                            .foregroundStyle(job.isActive ? Color.orange : Color.secondary)
                    }
                    Text(job.created_by_name).font(.caption2).foregroundStyle(.tertiary)
                }
                .accessibilityIdentifier("ai.job.\(job.id)")
            }
            ForEach(results) { result in
                VStack(alignment: .leading, spacing: 8) {
                    Text(result.task_type == "summarize_disagreement" ? String(localized: "评论分歧") : String(localized: "评论汇总"))
                        .font(.subheadline.weight(.semibold))
                    Text(result.summary).font(.footnote).textSelection(.enabled)
                    Text("\(result.comment_count) 条评论 · \(result.created_by_name)")
                        .font(.caption2).foregroundStyle(.secondary)
                }
            }
        }
    }

    private var tasksSheet: some View {
        NavigationStack {
            List {
                tasksSection
                Section {
                    Button("查看旧版对话存档") { activeSheet = .legacy }
                }
                if let account = model.aiAccountState {
                    Section("AI 服务") {
                        LabeledContent(LocalizedStringKey("服务")) { Text("FocalRoom AI") }
                        LabeledContent(LocalizedStringKey("可用")) { Text(verbatim: "\(account.available.formatted()) \(account.unit)") }
                        LabeledContent(LocalizedStringKey("已用")) { Text(verbatim: "\(account.spent.formatted()) \(account.unit)") }
                    }
                }
            }
                .navigationTitle("项目动态").navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .confirmationAction) {
                        Button("完成") { activeSheet = nil }
                    }
                }
        }
    }

    private var legacySheet: some View {
        NavigationStack {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 20) {
                    Text("这些旧消息没有可核对的项目归属，仅保留阅读，不会自动转为任务或发送给模型。")
                        .font(.footnote).foregroundStyle(.secondary)
                    ForEach(legacyMessages) { message in
                        VStack(alignment: .leading, spacing: 6) {
                            Text(message.role == "user" ? String(localized: "你") : "Focal AI").font(.caption).foregroundStyle(.secondary)
                            AIResponseText(text: message.text)
                        }
                    }
                    if legacyMessages.isEmpty { Text("还没有旧版对话存档。").foregroundStyle(.secondary) }
                }.padding(20)
            }.navigationTitle("旧版对话存档").navigationBarTitleDisplayMode(.inline)
                .toolbar { ToolbarItem(placement: .confirmationAction) { Button("完成") { activeSheet = nil } } }
                .task { legacyMessages = await model.legacyAIChat() }
        }
    }

    private var inputBar: some View {
        VStack(alignment: .leading, spacing: 8) {
            if model.aiMode == .task {
                Stepper("参考张数：\(model.aiTargetCount)", value: Binding(get: { model.aiTargetCount }, set: { model.aiTargetCount = $0 }), in: 1...500)
                    .font(.caption).foregroundStyle(.secondary).disabled(model.aiSending)
            }
            if !selectedPhotos.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(model.observationPhotos.filter { selectedPhotos.contains($0.id) }) { photo in
                            Button { selectedPhotos.remove(photo.id) } label: {
                                HStack(spacing: 6) {
                                    Image(systemName: "photo")
                                    Text(photo.filename).lineLimit(1).frame(maxWidth: 160, alignment: .leading)
                                    Image(systemName: "xmark").font(.system(size: 9, weight: .semibold))
                                }
                                .font(.caption).foregroundStyle(.secondary)
                                .padding(.horizontal, 10).frame(minHeight: 36)
                                .background(Color.white.opacity(0.07), in: Capsule())
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel(String(localized: "移除照片 \(photo.filename)"))
                            .accessibilityIdentifier("ai.attachment.remove.\(photo.id)")
                        }
                    }
                }
            }
            HStack(alignment: .bottom, spacing: 2) {
                Button { activeSheet = .photos } label: {
                    Image(systemName: "plus")
                        .font(.system(size: 19, weight: .regular))
                        .foregroundStyle(.white.opacity(0.8))
                        .frame(width: 44, height: 44)
                        .contentShape(Circle())
                }
                .buttonStyle(.plain)
                .disabled(model.aiMode == .chat || model.observationPhotos.isEmpty || !model.canSendAI || model.aiSending || preparingVision)
                .opacity(model.observationPhotos.isEmpty || !model.canSendAI || model.aiSending || preparingVision ? 0.35 : 1)
                .accessibilityLabel(String(localized: "选择照片"))
                .accessibilityIdentifier("ai.attach")

                TextField("向 Focal AI 提问…", text: $draft, axis: .vertical)
                    .font(.body).lineLimit(1...5)
                    .textFieldStyle(.plain)
                    .focused($inputFocused)
                    // 任务模式多了一行「参考张数」后，输入框会被压到 44 点的最小高度，
                    // 长消息只能在框内向上滚动、前面的字看不见；按内容自身高度撑开。
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(minHeight: 44)
                    .submitLabel(.send)
                    .accessibilityIdentifier("ai.input")
                    .onSubmit { send() }

                Button(action: send) {
                    Group {
                        if preparingVision {
                            ProgressView().controlSize(.small).tint(.black)
                        } else {
                            Image(systemName: "arrow.up").font(.system(size: 17, weight: .semibold))
                        }
                    }
                    .foregroundStyle(canSendNow ? Color.black : Color.white.opacity(0.4))
                    .frame(width: 36, height: 36)
                    .background(canSendNow ? Color.white : Color.white.opacity(0.10), in: Circle())
                    .frame(width: 44, height: 44)
                    .contentShape(Circle())
                }
                .buttonStyle(.plain)
                .disabled(!canSendNow)
                .accessibilityLabel(preparingVision ? String(localized: "准备照片") : String(localized: "发送"))
                .accessibilityIdentifier("ai.send")
            }
            .padding(4)
            .background(Color(white: 0.12), in: RoundedRectangle(cornerRadius: 25, style: .continuous))
            .overlay { RoundedRectangle(cornerRadius: 25, style: .continuous).strokeBorder(Color.white.opacity(0.09)) }
        }
        .padding(.horizontal, 16).padding(.top, 10).padding(.bottom, 10)
        .background(Color(white: 0.055))
    }

    private var canSendNow: Bool {
        model.canSendAI && !model.aiSending && !preparingVision &&
            !(model.aiMode == .task && model.nativeProjection?.tasks.contains(where: \.isActive) == true) &&
            !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    // 键盘「发送」不经按钮，会绕过 disabled；被挡住时必须说明原因，否则像点了没反应。
    private var sendBlockReason: String? {
        if preparingVision { return String(localized: "正在准备照片授权，请稍候。") }
        if draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { return String(localized: "请输入消息内容。") }
        if model.aiSending { return String(localized: "上一条请求仍在处理，稍后再发送。") }
        if model.aiMode == .task && model.nativeProjection?.tasks.contains(where: \.isActive) == true { return String(localized: "这个项目已有任务执行中，请先暂停或取消。") }
        if model.offline { return String(localized: "当前离线，无法发送消息。") }
        guard let account = model.aiAccountState else {
            return model.aiAccountError != nil ? String(localized: "AI 服务状态读取失败，请点按状态栏重试。") : String(localized: "正在读取 AI 服务状态，请稍候。")
        }
        if !account.enabled { return String(localized: "AI 服务当前未开放。") }
        if !account.verified { return String(localized: "请先验证邮箱后再使用 AI。") }
        if account.available == 0 { return String(localized: "账户暂无可用额度，请先续费或等待额度重置。") }
        return nil
    }

    private func send() {
        guard canSendNow else {
            if let reason = sendBlockReason { model.notice = reason }
            return
        }
        inputFocused = false
        let text = draft
        if selectedPhotos.isEmpty {
            beginFollowingRequest()
            let origin = model.currentProject
            // 只有请求真正入库后才清空草稿；被拒时保留正文，避免用户重新输入。
            Task { if await model.aiSend(text: text) { clearSentDraft(for: origin) } }
        } else {
            preparingVision = true
            Task {
                defer { preparingVision = false }
                do {
                    let prepared = try await model.prepareVision(text: text, photoIDs: selectedPhotos)
                    guard prepared.project == model.currentProject else { model.cancelVision(); return }
                    activeSheet = .vision(prepared)
                }
                catch is CancellationError { }
                catch { model.notice = visionPrepareFailure(error) }
            }
        }
    }

    // prepareVision 用同一个 permissionDenied 覆盖多种前置条件，直接展示其文案会把离线说成权限问题。
    private func visionPrepareFailure(_ error: Error) -> String {
        if model.offline { return String(localized: "当前离线，无法准备照片观察。") }
        if model.aiSending { return String(localized: "上一条请求仍在处理，稍后再发送。") }
        if selectedPhotos.isEmpty { return String(localized: "请先选择要观察的照片。") }
        if model.currentProject == nil { return String(localized: "请先选择项目，再向 AI 提问。") }
        if let value = error as? LocalizedError, let description = value.errorDescription,
           description != APIError.permissionDenied.errorDescription { return description }
        return String(localized: "照片准备失败，请重新选择照片后再试。")
    }

    private func clearPhotos() {
        selectedPhotos = []; activeSheet = nil
        model.cancelVision()
    }

    // 回执落地时用户可能已切到别的项目并在输入框里打了新字：只清发起项目的草稿，不动当前输入区。
    private func clearSentDraft(for origin: ProjectReference?) {
        projectDrafts[origin] = ""
        guard model.currentProject == origin else { return }
        draft = ""
        selectedPhotos = []
    }

    private func resetContext() {
        clearPhotos()
        streamStartedHere = false
        followingConversation = true
        scrollRequest += 1
    }

    // 键盘弹起时底边 1pt 锚点会被 LazyVStack 反复挂接与卸载；逐次写状态会自激成
    // 每秒数百轮整树重算（实测主线程 100%、内存涨到 6.8GB）。按时间窗吞掉抖动，跟随语义不变。
    private func follow(bottomVisible: Bool) {
        guard Date().timeIntervalSince(followSettledAt) > 0.25 else { return }
        followSettledAt = Date()
        followingConversation = bottomVisible
    }

    private func beginFollowingRequest() {
        streamProject = model.currentProject
        streamStartedHere = true
        followingConversation = true
        scrollRequest += 1
    }

    private var photoPicker: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    Text("最多选择 4 张。发送前会逐次确认照片和接收方。")
                        .font(.subheadline).foregroundStyle(.secondary)
                    if selectedPhotos.count == 4 {
                        Text("已选满 4 张，取消一张后可更换。")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                    LazyVGrid(columns: [GridItem(.flexible(), spacing: 8), GridItem(.flexible(), spacing: 8), GridItem(.flexible(), spacing: 8)], spacing: 16) {
                        ForEach(model.observationPhotos) { photo in
                            Button {
                                if selectedPhotos.contains(photo.id) { selectedPhotos.remove(photo.id) }
                                else if selectedPhotos.count < 4 { selectedPhotos.insert(photo.id) }
                            } label: {
                                VStack(alignment: .leading, spacing: 6) {
                                    observationThumbnail(photo.id)
                                        .aspectRatio(1, contentMode: .fit)
                                        .clipped()
                                        .overlay(alignment: .topTrailing) {
                                            Image(systemName: selectedPhotos.contains(photo.id) ? "checkmark.circle.fill" : "circle")
                                                .font(.system(size: 20))
                                                .foregroundStyle(.white)
                                                .shadow(color: .black.opacity(0.7), radius: 3)
                                                .padding(7)
                                        }
                                    Text(photo.filename).font(.caption2).foregroundStyle(.secondary)
                                        .lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
                                }
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel(photo.filename)
                            .accessibilityValue(selectedPhotos.contains(photo.id) ? String(localized: "已选择") : String(localized: "未选择"))
                            .accessibilityIdentifier("ai.attach.photo.\(photo.id)")
                        }
                    }
                }
                .padding(16)
            }
            .background(Color(white: 0.055))
            .navigationTitle("选择照片 · \(selectedPhotos.count)/4").navigationBarTitleDisplayMode(.inline)
            .toolbar { Button("完成") { activeSheet = nil } }
        }
    }

    @ViewBuilder
    private func observationThumbnail(_ photoID: String) -> some View {
        if let project = model.selectedLocalProject, let library = model.localLibrary {
            AIObservationThumbnail(identity: "local:\(project.id):\(photoID)") {
                try await library.image(projectID: project.id, photoID: photoID)
            }
        } else {
            AIObservationThumbnail(identity: "cloud:\(model.selectedProjectID ?? ""):\(photoID)") {
                try await model.image(photoID: photoID)
            }
        }
    }

    private func loadTasks() async {
        results = []; jobs = []; tasksError = nil
        let projectID = model.selectedProjectID
        guard projectID != nil else { return }
        guard !model.offline else {
            tasksError = String(localized: "当前离线，无法读取项目任务。联网后可重试。")
            return
        }
        do {
            let loadedResults = try await model.agentResults()
            guard !Task.isCancelled, model.selectedProjectID == projectID else { return }
            let loadedJobs = try await model.agentJobs()
            guard !Task.isCancelled, model.selectedProjectID == projectID else { return }
            results = loadedResults
            jobs = loadedJobs
            tasksError = nil
        } catch is CancellationError {
        } catch {
            if !Task.isCancelled, model.selectedProjectID == projectID { tasksError = String(localized: "任务读取失败，可在此重试。") }
        }
    }

    private func jobStatus(_ status: String) -> String {
        switch status {
        case "QUEUED": return String(localized: "排队中")
        case "RUNNING": return String(localized: "运行中")
        case "CANCELLED": return String(localized: "已取消")
        case "DONE", "COMPLETED": return String(localized: "已完成")
        default: return String(localized: "状态待核对")
        }
    }
}

// 按行处理常见 Markdown 块，正文和流式回复共用同一呈现方式。
private struct AIResponseText: View {
    let text: String

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(text.components(separatedBy: "\n").enumerated()), id: \.offset) { _, line in
                responseLine(line)
            }
        }
        .textSelection(.enabled)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder private func responseLine(_ line: String) -> some View {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        if trimmed.isEmpty {
            Color.clear.frame(height: 10)
        } else if let heading = headingText(trimmed) {
            formatted(heading).font(.headline).padding(.top, 8).padding(.bottom, 4)
        } else if trimmed.hasPrefix("- ") || trimmed.hasPrefix("* ") {
            HStack(alignment: .top, spacing: 8) {
                Text("•")
                formatted(String(trimmed.dropFirst(2)))
            }.font(.body).padding(.vertical, 2)
        } else {
            formatted(line).font(.body).padding(.vertical, 2)
        }
    }

    private func headingText(_ line: String) -> String? {
        let marks = line.prefix(while: { $0 == "#" }).count
        guard (1...6).contains(marks), line.dropFirst(marks).first == " " else { return nil }
        return String(line.dropFirst(marks + 1))
    }

    private func formatted(_ value: String) -> some View {
        let options = AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
        return Text((try? AttributedString(markdown: value, options: options)) ?? AttributedString(value))
            .lineSpacing(4)
    }
}

private struct AIObservationThumbnail: View {
    let identity: String
    let load: () async throws -> UIImage
    @State private var image: UIImage?
    @State private var failed = false

    var body: some View {
        GeometryReader { geometry in
            ZStack {
                Color.white.opacity(0.06)
                if let image {
                    Image(uiImage: image).resizable().scaledToFill()
                        .frame(width: geometry.size.width, height: geometry.size.height).clipped()
                } else if failed {
                    Image(systemName: "photo").foregroundStyle(.secondary)
                } else {
                    ProgressView().controlSize(.small)
                }
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
        .accessibilityHidden(true)
        .task(id: identity) {
            image = nil; failed = false
            do {
                let loaded = try await load()
                guard !Task.isCancelled else { return }
                image = loaded
            } catch {
                if !Task.isCancelled { failed = true }
            }
        }
    }
}
