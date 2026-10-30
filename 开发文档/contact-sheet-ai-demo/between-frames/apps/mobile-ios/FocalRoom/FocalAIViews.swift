import SwiftUI
import FocalRoomCore

struct FocalAIView: View {
    let model: AppModel
    @State private var draft = ""
    @State private var results: [AgentResult] = []
    @State private var jobs: [AgentJob] = []
    @State private var tasksError: String?
    @State private var choosingPhotos = false
    @State private var selectedPhotos: Set<String> = []
    @State private var preparingVision = false
    @State private var vision: PreparedVision?
    @State private var showingTasks = false
    @State private var followingConversation = true
    @State private var scrollRequest = 0
    @State private var streamStartedHere = false
    @State private var streamProject: ProjectReference?
    @State private var draftOwner: ProjectReference?
    @FocusState private var inputFocused: Bool

    private var hasPending: Bool {
        model.chatMessages.contains { $0.state == .unknown && $0.role == "user" }
    }

    var body: some View {
        VStack(spacing: 0) {
            ProjectContext(model: model)
            statusBar
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
        }
        .onChange(of: model.currentProject) { _, _ in resetContext() }
        .onChange(of: model.credentials?.user.id) { _, _ in resetContext() }
        .sheet(isPresented: $choosingPhotos) { photoPicker }
        .sheet(isPresented: $showingTasks) { tasksSheet }
        .sheet(item: $vision, onDismiss: { model.cancelVision() }) { prepared in
            VisionConsentView(draft: prepared) {
                // Sheet 收起会撤销尚未使用的授权；发送任务先消费授权，再主动收起。
                beginFollowingRequest()
                Task {
                    await model.aiSend(text: prepared.text, vision: prepared, onVisionConsumed: { vision = nil })
                }
                draft = ""
                selectedPhotos = []
            } cancel: { vision = nil }
        }
    }

    private var statusBar: some View {
        HStack(spacing: 8) {
            if let account = model.aiAccountState {
                if !account.enabled {
                    Label("AI 暂未开放", systemImage: "circle")
                } else if !account.verified {
                    Label("请先验证邮箱", systemImage: "envelope")
                } else if account.available == 0 {
                    Label("暂无可用额度", systemImage: "circle")
                } else if model.offline {
                    Label("当前离线", systemImage: "wifi.slash")
                } else {
                    Label("\(account.available.formatted()) \(account.unit) 可用", systemImage: "sparkle")
                }
            } else if let error = model.aiAccountError {
                Button { Task { await model.loadAIAccount() } } label: {
                    Label("服务状态读取失败，重试", systemImage: "arrow.clockwise")
                }
                .accessibilityHint(error)
                    .accessibilityIdentifier("ai.account.retry")
            } else {
                ProgressView().controlSize(.mini)
                Text("读取 AI 状态…")
            }
            Spacer(minLength: 8)
            Button { showingTasks = true } label: {
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
        .accessibilityIdentifier("ai.balance")
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
                    if model.chatMessages.isEmpty && !showingThinking && visibleStreamingText == nil {
                        welcome
                    }
                    ForEach(model.chatMessages) { message in
                        messageRow(message).id(message.id)
                    }
                    if let streaming = visibleStreamingText {
                        assistantRow(text: streaming.isEmpty ? "正在思考…" : streaming, streaming: true)
                            .id("ai.streaming")
                    } else if showingThinking {
                        assistantRow(text: "正在思考…", streaming: true)
                            .id("ai.streaming")
                    }
                    draftSection
                    if hasPending { pendingNotice }
                    Color.clear.frame(height: 1).id("ai.bottom")
                        .onAppear { followingConversation = true }
                        .onDisappear { followingConversation = false }
                }
                .padding(.horizontal, 20).padding(.top, 24).padding(.bottom, 18)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .scrollDismissesKeyboard(.interactively)
            .overlay(alignment: .bottomTrailing) {
                if !followingConversation && !model.chatMessages.isEmpty {
                    Button {
                        withAnimation(.easeOut(duration: 0.2)) { scroll.scrollTo("ai.bottom", anchor: .bottom) }
                    } label: {
                        Image(systemName: "arrow.down")
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(.primary)
                            .frame(width: 44, height: 44)
                            .background(Color(white: 0.18), in: Circle())
                    }
                    .accessibilityLabel("回到最新消息")
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
            Text("一起看照片、整理线索，或讨论一组作品的顺序。")
                .font(.subheadline).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            VStack(alignment: .leading, spacing: 0) {
                suggestion("帮我看看这组照片的共同气质")
                suggestion("这组照片可以怎样排列？")
            }.padding(.top, 6)
        }
        .padding(.vertical, 18)
        .accessibilityIdentifier("ai.welcome")
    }

    private func suggestion(_ text: String) -> some View {
        Button {
            draft = text
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
            Text("有消息结果待确认").font(.caption).foregroundStyle(.secondary)
            Spacer()
            if hasPending {
                Button("核对结果") { Task { await model.aiResolvePending() } }
                    .font(.caption.weight(.medium)).frame(minHeight: 44)
                    .accessibilityIdentifier("ai.resolve")
            }
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
                    if message.state == .unknown {
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
                if streaming && text == "正在思考…" {
                    HStack(spacing: 6) {
                        ProgressView().controlSize(.mini)
                        Text("正在思考…").font(.subheadline).foregroundStyle(.secondary)
                    }
                } else {
                    Text(text).font(.body).lineSpacing(4).textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    if streaming { Text("正在输入…").font(.caption2).foregroundStyle(.secondary) }
                }
            }.frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    // 草案卡：模型建议先展示为可读清单，用户显式确认后才创建权威提案（可部分采纳）。
    @ViewBuilder
    private var draftSection: some View {
        if let draft = model.lastDraft, let owner = draftOwner, owner == model.currentProject {
            VStack(alignment: .leading, spacing: 12) {
                Label("选片草案", systemImage: "sparkles")
                    .font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                Text(draft.title).font(.headline)
                ForEach(draft.operations) { op in
                    Text("\(opTitle(op.op)) · \(op.photoFilename)\(op.toIndex.map { " · 第 \($0 + 1) 位" } ?? "")")
                        .font(.subheadline).foregroundStyle(.secondary)
                }
                Text("提交后可在项目中逐项审阅。")
                    .font(.caption).foregroundStyle(.secondary)
                HStack(spacing: 18) {
                    Button { Task { await model.submitDraftAsProposal() } } label: {
                        Text("提交为提案").font(.subheadline.weight(.semibold))
                            .foregroundStyle(.black).padding(.horizontal, 16).frame(minHeight: 44)
                            .background(.white, in: Capsule())
                    }
                        .buttonStyle(.plain)
                        .disabled(model.busy || model.offline)
                        .opacity(model.busy || model.offline ? 0.4 : 1)
                        .accessibilityIdentifier("ai.draft.submit")
                    Button("放弃草案") { model.lastDraft = nil }
                        .font(.subheadline).foregroundStyle(.secondary).frame(minHeight: 44)
                        .disabled(model.busy)
                        .accessibilityIdentifier("ai.draft.discard")
                }
            }
            .padding(16).frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.white.opacity(0.055), in: RoundedRectangle(cornerRadius: 16))
            .accessibilityIdentifier("ai.draft.card")
        } else if model.selectedProjectID != nil, !model.offline {
            Button {
                draftOwner = model.currentProject
                model.lastDraft = nil
                beginFollowingRequest()
                Task { await model.requestDraft() }
            } label: {
                Label("生成选片草案", systemImage: "sparkles.rectangle.stack")
                    .font(.subheadline).foregroundStyle(.secondary).frame(minHeight: 44)
            }
                .buttonStyle(.plain)
                .disabled(model.aiSending || !model.canSendAI)
                .accessibilityIdentifier("ai.draft.request")
        }
    }

    private func opTitle(_ op: String) -> String {
        switch op {
        case "add": return "加入序列"
        case "remove": return "移出序列"
        case "move": return "调整顺序"
        default: return op
        }
    }

    private var tasksSection: some View {
        Section("项目分析任务") {
            if model.selectedLocalProjectID != nil {
                Text("可直接选择本机照片向 AI 提问，无需迁云。云端项目分析任务不会出现在本机项目中。")
                    .font(.footnote).foregroundStyle(.secondary)
            } else if let tasksError {
                Text(tasksError).font(.footnote).foregroundStyle(.secondary)
            } else if jobs.isEmpty && results.isEmpty {
                Text("暂无任务。项目内已有的分析任务与评论汇总会显示在这里。")
                    .font(.footnote).foregroundStyle(.secondary)
            }
            ForEach(jobs) { job in
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(job.kind).font(.subheadline.weight(.semibold))
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
                    Text(result.task_type == "summarize_disagreement" ? "评论分歧" : "评论汇总")
                        .font(.subheadline.weight(.semibold))
                    Text(result.summary).font(.footnote).textSelection(.enabled)
                    Text("\(result.model_id) · \(result.comment_count) 条评论 · \(result.created_by_name)")
                        .font(.caption2).foregroundStyle(.secondary)
                }
            }
        }
    }

    private var tasksSheet: some View {
        NavigationStack {
            List {
                tasksSection
                if let account = model.aiAccountState {
                    Section("AI 服务") {
                        LabeledContent("服务", value: "\(account.provider) · \(account.model)")
                        LabeledContent("可用", value: "\(account.available.formatted()) \(account.unit)")
                        LabeledContent("已用", value: "\(account.spent.formatted()) \(account.unit)")
                    }
                }
            }
                .navigationTitle("项目动态").navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .confirmationAction) {
                        Button("完成") { showingTasks = false }
                    }
                }
        }
    }

    private var inputBar: some View {
        VStack(alignment: .leading, spacing: 8) {
            if !selectedPhotos.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(model.observationPhotos.filter { selectedPhotos.contains($0.id) }) { photo in
                            Button { selectedPhotos.remove(photo.id) } label: {
                                HStack(spacing: 6) {
                                    Image(systemName: "photo")
                                    Text(photo.filename).lineLimit(1)
                                    Image(systemName: "xmark").font(.system(size: 9, weight: .semibold))
                                }
                                .font(.caption).foregroundStyle(.secondary)
                                .padding(.horizontal, 10).frame(minHeight: 36)
                                .background(Color.white.opacity(0.07), in: Capsule())
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel("移除照片 \(photo.filename)")
                            .accessibilityIdentifier("ai.attachment.remove.\(photo.id)")
                        }
                    }
                }
            }
            HStack(alignment: .bottom, spacing: 2) {
                Button { choosingPhotos = true } label: {
                    Image(systemName: "plus")
                        .font(.system(size: 19, weight: .regular))
                        .foregroundStyle(.white.opacity(0.8))
                        .frame(width: 44, height: 44)
                        .contentShape(Circle())
                }
                .buttonStyle(.plain)
                .disabled(model.observationPhotos.isEmpty || !model.canSendAI || model.aiSending || preparingVision)
                .opacity(model.observationPhotos.isEmpty || !model.canSendAI || model.aiSending || preparingVision ? 0.35 : 1)
                .accessibilityLabel("选择照片")
                .accessibilityIdentifier("ai.attach")

                TextField("向 Focal AI 提问…", text: $draft, axis: .vertical)
                    .font(.body).lineLimit(1...5)
                    .textFieldStyle(.plain)
                    .focused($inputFocused)
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
                .accessibilityLabel(preparingVision ? "准备照片" : "发送")
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
        model.canSendAI && !model.aiSending && !preparingVision && !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private func send() {
        guard canSendNow else { return }
        inputFocused = false
        let text = draft
        if selectedPhotos.isEmpty {
            draft = ""
            beginFollowingRequest()
            Task { await model.aiSend(text: text) }
        } else {
            preparingVision = true
            Task {
                defer { preparingVision = false }
                do { vision = try await model.prepareVision(text: text, photoIDs: selectedPhotos) }
                catch is CancellationError { }
                catch { model.notice = error.localizedDescription }
            }
        }
    }

    private func clearPhotos() {
        selectedPhotos = []; vision = nil; choosingPhotos = false
        model.cancelVision()
    }

    private func resetContext() {
        clearPhotos()
        streamStartedHere = false
        followingConversation = true
        scrollRequest += 1
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
                            .accessibilityValue(selectedPhotos.contains(photo.id) ? "已选择" : "未选择")
                            .accessibilityIdentifier("ai.attach.photo.\(photo.id)")
                        }
                    }
                }
                .padding(16)
            }
            .background(Color(white: 0.055))
            .navigationTitle("选择照片 · \(selectedPhotos.count)/4").navigationBarTitleDisplayMode(.inline)
            .toolbar { Button("完成") { choosingPhotos = false } }
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
        guard let projectID = model.selectedProjectID, !model.offline else { return }
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
            if !Task.isCancelled, model.selectedProjectID == projectID { tasksError = "任务读取失败，切换项目后重试。" }
        }
    }

    private func jobStatus(_ status: String) -> String {
        switch status {
        case "QUEUED": return "排队中"
        case "RUNNING": return "运行中"
        case "CANCELLED": return "已取消"
        case "DONE", "COMPLETED": return "已完成"
        default: return status
        }
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
