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
    @FocusState private var inputFocused: Bool

    private var hasPending: Bool {
        model.chatMessages.contains { $0.state == .unknown && $0.role == "user" }
    }

    var body: some View {
        VStack(spacing: 0) {
            ProjectContext(model: model)
            ScrollViewReader { scroll in
                List {
                    balanceSection
                    conversationSection
                    tasksSection
                }
                .onChange(of: model.chatMessages.last?.id) { _, id in
                    if let id { withAnimation { scroll.scrollTo(id, anchor: .bottom) } }
                }
            }
            inputBar
        }
        .navigationTitle("Focal AI").navigationBarTitleDisplayMode(.inline)
        .task(id: model.credentials?.user.id) {
            await model.loadAIAccount()
            await model.loadChat()
            await model.aiResolvePending()
        }
        .task(id: model.selectedProjectID) { await loadTasks() }
        .onChange(of: model.selectedProjectID) { _, _ in clearPhotos() }
        .onChange(of: model.credentials?.user.id) { _, _ in clearPhotos() }
        .sheet(isPresented: $choosingPhotos) { photoPicker }
        .sheet(item: $vision, onDismiss: { model.cancelVision() }) { prepared in
            VisionConsentView(draft: prepared) {
                // Sheet 收起会撤销尚未使用的授权；发送任务先消费授权，再主动收起。
                Task {
                    await model.aiSend(text: prepared.text, vision: prepared, onVisionConsumed: { vision = nil })
                }
                draft = ""
                selectedPhotos = []
            } cancel: { vision = nil }
        }
    }

    private var balanceSection: some View {
        Section {
            if let account = model.aiAccountState {
                HStack {
                    Label("\(account.available) \(account.unit) 可用 · 已用 \(account.spent)", systemImage: "sparkle")
                        .font(.subheadline)
                    Spacer()
                    Text("\(account.provider) · \(account.model)").font(.caption).foregroundStyle(.secondary)
                }
                .accessibilityIdentifier("ai.balance")
                if !account.enabled {
                    Text("AI 服务暂未开放。").font(.footnote).foregroundStyle(.secondary)
                } else if !account.verified {
                    Text("请先验证账户邮箱，再使用 AI。").font(.footnote).foregroundStyle(.secondary)
                } else if account.available == 0 {
                    Text("账户暂无可用 AI 额度。").font(.footnote).foregroundStyle(.secondary)
                }
            } else if let error = model.aiAccountError {
                Text(error).font(.footnote).foregroundStyle(.secondary)
                Button("重新读取 AI 服务状态") { Task { await model.loadAIAccount() } }
                    .accessibilityIdentifier("ai.account.retry")
            } else {
                HStack(spacing: 8) {
                    ProgressView()
                    Text("读取额度…").foregroundStyle(.secondary)
                }
            }
        }
    }

    private var conversationSection: some View {
        Section("对话") {
            if hasPending {
                Button("查询待确认结果") {
                    Task { await model.aiResolvePending() }
                }
                .font(.subheadline)
                .accessibilityIdentifier("ai.resolve")
            }
            if model.chatMessages.isEmpty {
                Text("向 Focal AI 提问选片与编排问题。可选择当前项目的照片；每次发送前须确认收件方与预览，不会自动发送照片。")
                    .font(.footnote).foregroundStyle(.secondary)
            }
            ForEach(model.chatMessages) { message in
                bubble(message).id(message.id)
            }
        }
    }

    private func bubble(_ message: AIChatMessage) -> some View {
        VStack(alignment: message.role == "user" ? .trailing : .leading, spacing: 4) {
            Text(message.text)
                .textSelection(.enabled)
                .font(.subheadline)
            if message.state == .unknown && message.role == "user" {
                Label("结果待确认", systemImage: "clock.arrow.circlepath")
                    .font(.caption2).foregroundStyle(.orange)
            }
            if message.state == .failed, let failure = message.failure {
                Label(failure, systemImage: "exclamationmark.circle")
                    .font(.caption2).foregroundStyle(.red)
            }
            if message.role == "assistant" {
                Text("Focal AI").font(.caption2).foregroundStyle(.tertiary)
            }
        }
        .frame(maxWidth: .infinity, alignment: message.role == "user" ? .trailing : .leading)
        .padding(12)
        .background(message.role == "user" ? Color.white.opacity(0.12) : Color.white.opacity(0.05),
                    in: RoundedRectangle(cornerRadius: 14))
        .accessibilityIdentifier("ai.message.\(message.id)")
    }

    private var tasksSection: some View {
        Section("项目分析任务") {
            if let tasksError {
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

    private var inputBar: some View {
        HStack(spacing: 10) {
            Button { choosingPhotos = true } label: {
                Label(selectedPhotos.isEmpty ? "照片" : "\(selectedPhotos.count)", systemImage: "photo.badge.plus")
            }.disabled(model.snapshot == nil || !model.canSendAI || model.aiSending || preparingVision)
                .accessibilityIdentifier("ai.attach")
            TextField("输入消息", text: $draft)
                .textFieldStyle(.roundedBorder)
                .focused($inputFocused)
                .accessibilityIdentifier("ai.input")
                .onSubmit { send() }
            Button(model.aiSending ? "发送中…" : "发送") { send() }
                .buttonStyle(.borderedProminent)
                .disabled(!model.canSendAI || model.aiSending || preparingVision || draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .accessibilityIdentifier("ai.send")
        }
        .padding(12)
        .background(.bar)
    }

    private func send() {
        inputFocused = false
        let text = draft
        if selectedPhotos.isEmpty {
            draft = ""
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

    private var photoPicker: some View {
        NavigationStack {
            List {
                Text("选择当前云端项目的照片，最多 4 张。发送前还会核实权限，并逐次确认。")
                    .font(.footnote).foregroundStyle(.secondary)
                ForEach(model.snapshot?.snapshot.photos ?? []) { photo in
                    Button {
                        if selectedPhotos.contains(photo.id) { selectedPhotos.remove(photo.id) }
                        else if selectedPhotos.count < 4 { selectedPhotos.insert(photo.id) }
                    } label: {
                        HStack {
                            Text(photo.filename)
                            Spacer()
                            if selectedPhotos.contains(photo.id) { Image(systemName: "checkmark.circle.fill") }
                        }
                    }.accessibilityIdentifier("ai.attach.photo.\(photo.id)")
                }
            }.navigationTitle("选择观察照片").navigationBarTitleDisplayMode(.inline)
                .toolbar { Button("完成") { choosingPhotos = false } }
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
