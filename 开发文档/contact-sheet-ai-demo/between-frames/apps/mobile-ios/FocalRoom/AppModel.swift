import Foundation
import SwiftUI
import Observation
import ImageIO
import UIKit
import FocalRoomCore
import CryptoKit

// 本机与云端编号属于不同身份域，不能把本机编号传入云端项目接口。
enum ProjectReference: Equatable, Hashable {
    case local(String)
    case cloud(String)
}

// 设备身份：每次安装生成一次的稳定 UUID，存于 UserDefaults（非机密），配合服务端幂等注册。
@MainActor enum DeviceIdentity {
    private static let key = "focalroom.device-key"

    static func value() -> String {
        if let existing = UserDefaults.standard.string(forKey: key), UUID(uuidString: existing) != nil {
            return existing
        }
        let fresh = UUID().uuidString.lowercased()
        UserDefaults.standard.set(fresh, forKey: key)
        return fresh
    }

    static func name() -> String {
        String(UIDevice.current.name.prefix(120))
    }
}

@MainActor @Observable
final class AppModel {
    enum AIMode: String, CaseIterable { case task = "选片任务", chat = "聊天" }
    var aiMode: AIMode = .task
    var aiTargetCount = 12
    private(set) var nativeProjection: NativeAIProjection?
    private(set) var nativeContext: NativeAIContext?
    private(set) var nativeTaskError: String?
    private(set) var nativeReviewReceipt: String?
    private var nativeOwner: ProjectReference?
    private var nativeSyncedStamp: String?
    @ObservationIgnored private var nativeAI: NativeAIClient?
    private(set) var credentials: Credentials?
    private(set) var projects: [ProjectSummary] = []
    private(set) var snapshot: ProjectSnapshot?
    private(set) var currentProject: ProjectReference?
    var selectedProjectID: String? {
        if case .cloud(let id) = currentProject { return id }
        return nil
    }
    var selectedLocalProjectID: String? {
        if case .local(let id) = currentProject { return id }
        return nil
    }
    var selectedLocalProject: LocalProject? {
        localLibrary?.projects.first { $0.id == selectedLocalProjectID }
    }
    var currentProjectTitle: String? {
        if let local = selectedLocalProject { return local.title }
        return projects.first { $0.id == selectedProjectID }?.title
    }
    struct ObservationPhoto: Identifiable {
        let id: String
        let filename: String
    }
    var observationPhotos: [ObservationPhoto] {
        if let local = selectedLocalProject { return local.photos.map { .init(id: $0.id, filename: $0.filename) } }
        return snapshot?.snapshot.photos.map { .init(id: $0.id, filename: $0.filename) } ?? []
    }
    private(set) var pending: PendingSelection?
    private(set) var cachedAt: Date?
    private(set) var offline = false
    private(set) var busy = false
    private(set) var loadingProject = false
    private(set) var starting = true
    private(set) var localLibrary: LocalLibraryModel?
    private(set) var migration: LocalMigrationModel?
    private(set) var devices: [DeviceRecord] = []
    private(set) var loadingDevices = false
    private(set) var aiAccountState: AIAccount?
    private(set) var aiAccountError: String?
    private(set) var chatMessages: [AIChatMessage] = []
    private(set) var aiSending = false
    /// 流式回复的实时预览文本：仅在流式进行中非空；完成后正式落盘并清空。
    private(set) var streamingText: String?
    private(set) var canvasSending = false
    private(set) var pendingBoards: Set<String> = []
    var notice: String?
    var address = "https://team.focroom.cc"
    /// 验证码已发送到邮箱的提示状态；登录成功或切换模式时清除。
    var codeSent = false

    @ObservationIgnored private var api: APIClient?
    @ObservationIgnored private var store: ScopedStore?
    @ObservationIgnored private var previewCache: PreviewCache?
    @ObservationIgnored private var chat: AIChatStore?
    @ObservationIgnored private var epoch = UUID()
    @ObservationIgnored private var projectRequest = UUID()
    @ObservationIgnored private var visionDraftID: UUID?
    @ObservationIgnored private let vault = SessionVault()
    @ObservationIgnored private let images = NSCache<NSString, UIImage>()
    @ObservationIgnored let root = URL.applicationSupportDirectory.appendingPathComponent("FocalRoom-iPhone")

    init() {
        images.totalCostLimit = 64 * 1024 * 1024
        #if DEBUG
        address = ProcessInfo.processInfo.environment["FOCALROOM_IOS_REVIEW_URL"] ?? address
        #endif
    }

    private func configuration(_ address: String) throws -> APIConfiguration {
        #if DEBUG
        return try APIConfiguration(address, allowLoopbackHTTP: true)
        #else
        return try APIConfiguration(address)
        #endif
    }

    func restore() async {
        defer { starting = false }
        do {
            guard let saved = try vault.read() else { return }
            let configuration = try configuration(saved.address)
            #if DEBUG
            if let review = ProcessInfo.processInfo.environment["FOCALROOM_IOS_REVIEW_URL"],
               try self.configuration(review).identity != configuration.identity { return }
            #endif
            address = saved.address
            activate(saved.credentials, configuration: configuration)
            // 缓存优先：先渲染上次的项目列表秒开界面，随后立即放行主界面。
            if let store, let cached = try? await store.projects() {
                projects = cached.projects
            }
            // 网络刷新与设备登记都放后台：主界面立刻可见，数据到了自动更新。
            starting = false
            Task {
                await reloadProjects()
                await registerCurrentDevice()
            }
        } catch { notice = message(error) }
    }

    func login(email: String, password: String) async {
        guard !busy else { return }
        busy = true
        notice = nil
        let generation = epoch
        defer { if generation == epoch { busy = false } }
        do {
            let configuration = try configuration(address)
            let value = try await APIClient(configuration: configuration).login(
                email: email.trimmingCharacters(in: .whitespacesAndNewlines).lowercased(), password: password)
            guard generation == epoch else { return }
            try vault.save(SavedSession(address: configuration.identity, credentials: value))
            activate(value, configuration: configuration)
            busy = false
            await reloadProjects()
            await registerCurrentDevice()
        } catch { if generation == epoch { notice = message(error) } }
    }

    /// 请求邮箱验证码；成功后提示查收，旧验证码在服务端作废。
    func requestLoginCode(email: String) async {
        guard !busy else { return }
        busy = true
        notice = nil
        let generation = epoch
        defer { if generation == epoch { busy = false } }
        do {
            let configuration = try configuration(address)
            try await APIClient(configuration: configuration).magicRequest(
                email: email.trimmingCharacters(in: .whitespacesAndNewlines).lowercased())
            guard generation == epoch else { return }
            codeSent = true
        } catch { if generation == epoch { notice = message(error) } }
    }

    func magicLogin(email: String, code: String) async {
        guard !busy else { return }
        busy = true
        notice = nil
        let generation = epoch
        defer { if generation == epoch { busy = false } }
        do {
            let configuration = try configuration(address)
            let value = try await APIClient(configuration: configuration).magicVerify(
                email: email.trimmingCharacters(in: .whitespacesAndNewlines).lowercased(), code: code)
            guard generation == epoch else { return }
            try vault.save(SavedSession(address: configuration.identity, credentials: value))
            activate(value, configuration: configuration)
            codeSent = false
            busy = false
            await reloadProjects()
            await registerCurrentDevice()
        } catch { if generation == epoch { notice = message(error) } }
    }

    /// 登录态设置/修改密码；验证码新建的账号可不带当前密码。返回 false 表示未提交。
    @discardableResult
    func setPassword(_ password: String, current: String?) async -> Bool {
        guard !busy else {
            notice = "上一个请求仍在处理，请稍候再试。"
            return false
        }
        guard let api else {
            notice = "登录状态已失效，请重新登录后再设置密码。"
            return false
        }
        busy = true
        notice = nil
        let generation = epoch
        defer { if generation == epoch { busy = false } }
        do {
            try await api.setPassword(token: credentials?.token ?? "", password: password, current: current)
            guard generation == epoch else { return false }
            notice = "密码已更新。下次登录可使用邮箱与新密码。"
            return true
        } catch {
            if generation == epoch { notice = message(error) }
            return false
        }
    }

    private func activate(_ value: Credentials, configuration: APIConfiguration) {
        let previousAI = nativeAI
        nativeAI = NativeAIClient()
        Task { await previousAI?.close() }
        stopAutoResolve()
        epoch = UUID()
        credentials = value
        api = APIClient(configuration: configuration)
        store = ScopedStore(root: root, scope: AccountScope(configuration: configuration, userID: value.user.id))
        LocalScopeIndex.record(root: root, server: configuration.identity, userID: value.user.id,
                               email: value.user.email, name: value.user.name)
        let scope = AccountScope(configuration: configuration, userID: value.user.id)
        previewCache = PreviewCache(directory: root.appendingPathComponent("PreviewCache").appendingPathComponent(scope.key))
        chat = AIChatStore(root: root.appendingPathComponent("AIChat"), scope: scope)
        let library = LocalLibrary(root: root.appendingPathComponent("LocalLibrary"), scope: scope)
        let libraryModel = LocalLibraryModel(library: library)
        // 删除的正是当前打开的本机项目时，必须清空项目上下文；否则各 Tab 仍持有已消失项目的引用。
        libraryModel.onProjectDeleted = { [weak self] deletedID in
            guard let self, self.currentProject == .local(deletedID) else { return }
            self.resetProject()
            self.notice = "本机项目已删除，已退出其画布与对话上下文。"
        }
        localLibrary = libraryModel
        let migrationModel = LocalMigrationModel(library: library, api: APIClient(configuration: configuration), credentials: value)
        migrationModel.onRecordChanged = { [weak libraryModel] in await libraryModel?.reload() }
        migration = migrationModel
    }

    /// 云端项目归档/取消归档：成功返回给用户看的文案，失败返回 nil（原因已写进 notice）。
    /// 列表刷新交给调用方，且必须排在退出项目详情之后：列表一重建，正在显示的详情页
    /// 对应的那条导航就没了，之后再 dismiss 会被吞掉，用户被留在「暂时无法打开项目」的死页。
    func archiveCloudProject(_ projectID: String, archive: Bool) async -> String? {
        guard let api, let credentials else {
            notice = "当前离线，无法修改项目归档状态。"
            return nil
        }
        let generation = epoch
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            try await api.archiveProject(projectID: projectID, archive: archive, token: credentials.token)
            guard generation == epoch else { return nil }
            if selectedProjectID == projectID { resetProject() }
            return archive ? "项目已归档，可在项目列表的「已归档」分组中恢复。" : "项目已恢复为正常项目。"
        } catch {
            if generation == epoch { notice = message(error) }
            return nil
        }
    }

    /// 云端项目删除：仅所有者；含审计记录或共享协作时服务端按规则拒绝并说明原因。
    /// 与归档一样，成功只返回文案，列表刷新留给调用方在退出详情之后执行。
    func deleteCloudProject(_ projectID: String) async -> String? {
        guard let api, let credentials else {
            notice = "当前离线，无法删除云端项目。"
            return nil
        }
        let generation = epoch
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            try await api.deleteCloudProject(projectID: projectID, token: credentials.token)
            guard generation == epoch else { return nil }
            if selectedProjectID == projectID { resetProject() }
            return "云端项目已删除。"
        } catch {
            if generation == epoch { notice = message(error) }
            return nil
        }
    }

    func refreshOnResume() async {
        guard !starting, !busy, !loadingProject, credentials != nil else { return }
        // 回前台：项目与本机库刷新并行；设备登记后台静默，不互相等待。
        async let projects: Void = reloadProjects()
        async let library: Void = localLibrary?.reload() ?? ()
        await projects
        await library
        if selectedProjectID != nil { await syncSelectedProject() }
        Task { await registerCurrentDevice() }
    }

    // 登录/恢复后登记本机设备并绑定当前会话；失败不阻断主流程，下次前台重试。
    func registerCurrentDevice() async {
        guard let api, let credentials else { return }
        let generation = epoch
        do {
            let device = try await api.registerDevice(key: DeviceIdentity.value(), name: DeviceIdentity.name(),
                                                      platform: "ios", token: credentials.token)
            guard generation == epoch else { return }
            try await api.bindDevice(id: device.id, token: credentials.token)
        } catch {
            guard generation == epoch, !(error is CancellationError) else { return }
            // 设备登记是安全配套，失败时保持静默，避免干扰正常使用。
        }
    }

    func loadDevices() async {
        guard let api, let credentials, !loadingDevices else { return }
        loadingDevices = true
        defer { loadingDevices = false }
        let generation = epoch
        do {
            let values = try await api.devices(token: credentials.token)
            guard generation == epoch else { return }
            devices = values
        } catch { if generation == epoch { notice = message(error) } }
    }

    func revokeDevice(_ id: String) async {
        guard let api, let credentials, !busy else { return }
        let generation = epoch
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            try await api.revokeDevice(id: id, token: credentials.token)
        } catch {
            guard generation == epoch else { return }
            // 撤销当前设备会让本会话随之失效，下一次请求按登录失效处理。
            if (error as? APIError)?.revokesSession != true { notice = message(error) }
        }
        if generation == epoch { await loadDevices() }
    }

    func openLocalContext(_ id: String) {
        guard !busy, localLibrary?.projects.contains(where: { $0.id == id }) == true else { return }
        if currentProject == .local(id) { return }
        resetProject()
        currentProject = .local(id)
        images.removeAllObjects()
    }

    func reloadProjects() async {
        guard let api, let credentials, let store else { return }
        let generation = epoch
        do {
            let values = try await api.projects(token: credentials.token)
            guard generation == epoch else { return }
            try await store.saveProjects(values)
            guard generation == epoch else { return }
            projects = values
            offline = false
            if let selectedProjectID, !values.contains(where: { $0.id == selectedProjectID }) {
                resetProject()
                images.removeAllObjects()
            }
        } catch {
            guard generation == epoch else { return }
            if await handleSession(error) { return }
            if isConnectivityError(error) {
                do {
                    let cached = try await store.projects()
                    guard generation == epoch else { return }
                    projects = cached?.projects ?? []
                    offline = true
                    if cached == nil { notice = "当前离线，尚无已缓存项目。" }
                } catch { notice = "无法读取本机项目缓存，请联网后重试。" }
            } else { notice = message(error) }
        }
    }

    func openProject(_ id: String) async {
        guard !busy else { return }
        if !projects.contains(where: { $0.id == id && $0.state == .SHARED }) {
            // 迁云刚完成时项目列表可能尚未刷新；先补一次权威列表再判定。
            await reloadProjects()
        }
        guard projects.contains(where: { $0.id == id && $0.state == .SHARED }) else {
            // 静默返回会让「打开项目」看起来完全没反应；说明原因并给出下一步。
            notice = "这个项目当前不可打开：可能仍在准备、已归档或权限已变化。请返回列表刷新后重试。"
            return
        }
        resetProject()
        currentProject = .cloud(id)
        await reloadSnapshot()
    }

    // 持久事件游标：以缓存快照的 cursor 拉增量事件，只有出现新事件才重新获取权威快照。
    func syncSelectedProject() async {
        guard let api, let credentials, let store, let id = selectedProjectID, pending == nil else {
            if selectedProjectID != nil { await reloadSnapshot() }
            return
        }
        do {
            let cached = try await store.snapshot(projectID: id)
            guard let cursor = Int(cached?.snapshot.cursor ?? "") else { return await reloadSnapshot() }
            var after = cursor
            var changed = false
            while true {
                let page = try await api.projectEvents(projectID: id, after: after, token: credentials.token)
                if !page.events.isEmpty { changed = true }
                if let next = page.nextCursor { after = next } else { break }
            }
            if changed { await reloadSnapshot() }
        } catch {
            await reloadSnapshot()
        }
    }

    func reloadSnapshot() async {
        guard let api, let credentials, let store, let id = selectedProjectID else { return }
        let generation = epoch
        let request = UUID()
        projectRequest = request
        loadingProject = true
        defer { if projectRequest == request { loadingProject = false } }
        do {
            let savedPending = try await store.pending(projectID: id)
            guard generation == epoch, projectRequest == request else { return }
            pending = savedPending
            let value = try await api.snapshot(projectID: id, credentials: credentials)
            guard generation == epoch, projectRequest == request else { return }
            try await store.saveSnapshot(value)
            guard generation == epoch, projectRequest == request else { return }
            snapshot = value
            cachedAt = Date()
            offline = false
        } catch {
            guard generation == epoch, projectRequest == request else { return }
            if await handleProject(error, id: id) { return }
            if isConnectivityError(error) {
                do {
                    let cached = try await store.snapshot(projectID: id)
                    guard generation == epoch, projectRequest == request else { return }
                    if let cached { try cached.snapshot.validate(projectID: id, userID: credentials.user.id) }
                    snapshot = cached?.snapshot
                    cachedAt = cached?.savedAt
                    offline = true
                    if cached == nil { notice = "这个项目尚未缓存，请联网后打开。" }
                } catch { notice = "无法读取本机项目缓存，请联网后重试。" }
            } else { notice = message(error) }
        }
    }

    var canSelect: Bool {
        !busy && !loadingProject && pending == nil && snapshot?.state == .SHARED &&
        snapshot?.principal.permitsSelection == true
    }

    func select(photo: Photo, selection: Selection) async {
        guard canSelect, let snapshot, let store else { return }
        let generation = epoch
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            let value = PendingSelection(command: try SelectionCommand(snapshot: snapshot, photoID: photo.id, selection: selection))
            try await store.savePending(value)
            guard generation == epoch else { return }
            pending = value
            if offline {
                notice = "已保存为待提交选片，联网后可核对并提交。"
            } else { await transmit(value, projectID: value.command.project_id, generation: generation) }
        } catch { if generation == epoch { notice = message(error) } }
    }

    func retryPending() async {
        guard let id = selectedProjectID else { return }
        await retryPending(projectID: id)
    }

    func retryPending(projectID: String) async {
        guard !busy, !loadingProject, let store else { return }
        let stored = try? await store.pending(projectID: projectID)
        guard let value = stored ?? nil else { return }
        let generation = epoch
        busy = true
        defer { if generation == epoch { busy = false } }
        if selectedProjectID == projectID { pending = value }
        await transmit(value, projectID: projectID, generation: generation)
    }

    private func transmit(_ initialValue: PendingSelection, projectID: String, generation: UUID) async {
        guard let api, let credentials, let store else { return }
        var value = initialValue
        do {
            // UNKNOWN 状态先按原编号查询权威回执；命中即已提交，不再重发。
            if value.state == .unknown,
               let receipt = try await api.commandReceipt(projectID: value.command.project_id,
                                                          commandID: value.command.command_id,
                                                          token: credentials.token) {
                try receipt.validate(for: value.command)
                try await store.acknowledge(receipt, command: value.command)
                guard generation == epoch else { return }
                if selectedProjectID == projectID {
                    pending = nil
                    snapshot = nil
                    await reloadSnapshot()
                }
                return
            }
            // 先记 UNKNOWN，进程即使在 HTTP 返回前终止也不会把它当成未发送。
            value.state = .unknown
            try await store.savePending(value)
            guard generation == epoch else { return }
            if selectedProjectID == projectID { pending = value }
            let receipt = try await api.select(value.command, token: credentials.token)
            guard generation == epoch else { return }
            try await store.acknowledge(receipt, command: value.command)
            guard generation == epoch else { return }
            if selectedProjectID == projectID {
                pending = nil
                snapshot = nil
                await reloadSnapshot()
            }
        } catch {
            guard generation == epoch else { return }
            if await handleProject(error, id: value.command.project_id) { return }
            value.state = (error as? APIError)?.rejectsCommand == true ? .rejected : .unknown
            do { try await store.savePending(value) }
            catch { notice = "待确认记录无法保存，请保持当前项目并重试。"; return }
            guard generation == epoch else { return }
            if selectedProjectID == projectID { pending = value }
            notice = value.state == .rejected ? message(error) : "结果尚未确认。再次核对会沿用原操作编号，不会创建第二次选片。"
        }
    }

    func discardRejected() async {
        guard let id = selectedProjectID else { return }
        await discardRejected(projectID: id)
    }

    func discardRejected(projectID: String) async {
        guard !busy, let store else { return }
        do {
            try await store.discardRejected(projectID: projectID)
            if selectedProjectID == projectID {
                pending = nil
                await reloadSnapshot()
            }
        } catch { notice = message(error) }
    }

    private(set) var syncOverview: [CachedProjectOverview] = []

    func loadSyncOverview() async {
        guard let store else { return }
        do { syncOverview = try await store.overview() }
        catch { notice = "无法读取本机同步状态，请稍后重试。" }
    }

    // MARK: - Focal AI

    func loadAIAccount() async {
        guard let api, let credentials else { return }
        let generation = epoch
        aiAccountError = nil
        do {
            let account = try await api.aiAccount(token: credentials.token)
            guard generation == epoch else { return }
            aiAccountState = account
        } catch {
            guard generation == epoch else { return }
            aiAccountState = nil
            aiAccountError = message(error)
        }
    }

    var canSendAI: Bool {
        guard let account = aiAccountState else { return false }
        return account.enabled && account.verified && account.available > 0 && !offline
    }

    func loadChat() async {
        scheduleAutoResolve()
        guard let chat else { return }
        let generation = epoch, project = currentProject
        let messages = (try? await chat.messages(localProjectID: selectedLocalProjectID, cloudProjectID: selectedProjectID)) ?? []
        guard generation == epoch, project == currentProject else { return }
        chatMessages = messages
    }

    func prepareVision(text: String, photoIDs: Set<String>) async throws -> PreparedVision {
        guard aiMode == .task, let api, let credentials, let project = currentProject, !offline, !aiSending,
              !photoIDs.isEmpty, photoIDs.count <= 4 else { throw APIError.permissionDenied }
        let generation = epoch
        visionDraftID = nil
        var photos: [PreparedVision.Photo] = []
        switch project {
        case .local(let id):
            guard let library = localLibrary?.library,
                  let local = try await library.projects().first(where: { $0.id == id }) else { throw APIError.permissionDenied }
            for photoID in photoIDs.sorted() {
                guard let photo = local.photos.first(where: { $0.id == photoID }) else { throw APIError.permissionDenied }
                let bytes = try await library.preview(projectID: id, photoID: photoID)
                let preview = try await Task.detached(priority: .userInitiated) { try VisionPreview.make(jpeg: bytes) }.value
                guard generation == epoch, project == currentProject else { throw CancellationError() }
                photos.append(.init(photoID: photoID, contentSHA: photo.previewSHA256, filename: photo.filename, preview: preview))
            }
        case .cloud(let id):
            let snapshot = try await projectRead(id) { try await api.snapshot(projectID: id, credentials: credentials) }
            for photoID in photoIDs.sorted() {
                guard let photo = snapshot.snapshot.photos.first(where: { $0.id == photoID }),
                      let descriptor = snapshot.previews.first(where: { $0.photo_id == photoID }) else { throw APIError.permissionDenied }
                let bytes = try await api.preview(projectID: id, descriptor: descriptor, token: credentials.token)
                let preview = try await Task.detached(priority: .userInitiated) { try VisionPreview.make(jpeg: bytes) }.value
                guard generation == epoch, project == currentProject else { throw CancellationError() }
                photos.append(.init(photoID: photoID, contentSHA: descriptor.content_sha256, filename: photo.filename, preview: preview))
            }
        }
        let draft = PreparedVision(id: UUID(), generation: generation, accountID: credentials.user.id,
                                   project: project, text: text, createdAt: Date(), photos: photos)
        visionDraftID = draft.id
        return draft
    }

    func cancelVision() { visionDraftID = nil }

    /// 返回 false 表示请求未被受理（调用方需保留用户已输入的正文，不能静默丢弃草稿）。
    @discardableResult
    func aiSend(text: String, vision: PreparedVision? = nil, onVisionConsumed: () -> Void = {}) async -> Bool {
        guard !aiSending, let api, let credentials, let chat else {
            if aiSending { notice = "上一条请求仍在处理，稍后再发送。" }
            else { notice = offline ? "当前离线，无法发送消息。" : "会话存储不可用，请重新登录后重试。" }
            return false
        }
        guard canSendAI else {
            notice = "请先确认 AI 服务已开放、邮箱已验证且账户额度可用，再发送消息。"
            return false
        }
        if aiMode == .task, nativeProjection?.tasks.contains(where: \.isActive) == true {
            notice = "这个项目已有任务执行中，请先暂停或取消。"; return false
        }
        let generation = epoch
        let project = currentProject
        let localProjectID = selectedLocalProjectID
        let cloudProjectID = selectedProjectID
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed.count <= 4000 else {
            notice = trimmed.isEmpty ? "请输入消息内容。" : "消息过长，请分段发送。"
            return false
        }
        aiSending = true
        streamingText = nil
        defer {
            if generation == epoch { aiSending = false; streamingText = nil }
        }
        if let vision {
            guard vision.id == visionDraftID, vision.generation == generation,
                  vision.accountID == credentials.user.id, vision.project == project,
                  vision.text == text, Date().timeIntervalSince(vision.createdAt) < 300 else {
                notice = "照片授权已失效，请重新选择照片并确认。"; return false
            }
            // 授权消费一次；点击确认后重新核实项目权限与照片身份，失败不发送。
            visionDraftID = nil
            onVisionConsumed()
            do {
                switch vision.project {
                case .local(let id):
                    guard let library = localLibrary?.library,
                          let current = try await library.projects().first(where: { $0.id == id }),
                          vision.photos.allSatisfy({ photo in
                              current.photos.contains { $0.id == photo.photoID && $0.previewSHA256 == photo.contentSHA }
                          }) else { throw APIError.permissionDenied }
                    for photo in vision.photos { _ = try await library.preview(projectID: id, photoID: photo.photoID) }
                case .cloud(let id):
                    let current = try await projectRead(id) { try await api.snapshot(projectID: id, credentials: credentials) }
                    guard vision.photos.allSatisfy({ photo in
                        current.previews.contains { $0.photo_id == photo.photoID && $0.content_sha256 == photo.contentSHA }
                    }) else { throw APIError.permissionDenied }
                }
            } catch { if generation == epoch { notice = message(error) }; return false }
        }
        guard generation == epoch, project == currentProject else { return false }
        do {
            let client = try await sharedAIClient()
            guard generation == epoch, project == currentProject else { return false }
            if aiMode == .task || vision != nil {
                try await startNativeTask(text: trimmed, vision: vision, client: client)
                await refreshNativeTasks()
                return true
            }
            let requestID = UUID().uuidString.lowercased()
            let userMessage = AIChatMessage(id: UUID().uuidString.lowercased(), role: "user", text: trimmed,
                                            requestID: requestID, state: .unknown, localProjectID: localProjectID, cloudProjectID: cloudProjectID)
            try await chat.append(userMessage)
            chatMessages.append(userMessage)
            let history = chatMessages.filter { $0.state == .sent || $0.id == userMessage.id }.suffix(20)
                .map { ["role": $0.role, "content": String($0.text.prefix(4000))] }
            do {
                let bytes = try await client.perform("chat", payload: aiPayload(["turns": history, "requestID": requestID]))
                let reply = try JSONSerialization.jsonObject(with: bytes) as? [String: Any]
                guard let text = reply?["text"] as? String, !text.isEmpty else { throw APIError.invalidResponse }
                // 迟到回复写回原账户/项目存储；只有当前上下文会刷新界面。
                try await chat.complete(userMessage, text: text)
            } catch {
                if generation == epoch, project == currentProject { notice = message(error) }
                // 持久编号已保存，后续只读核对现有网关回执，不再次提交模型请求。
                if generation == epoch { scheduleAutoResolve() }
            }
            if generation == epoch, project == currentProject { await loadChat(); await loadAIAccount() }
            return true
        } catch {
            if generation == epoch, project == currentProject { notice = message(error) }
            return false
        }
    }

    /// 核对「结果待确认」的 AI 消息。manual 为用户主动点按，失败必须给出提示；自动核对保持安静。
    func aiResolvePending(manual: Bool = false) async {
        guard let api, let credentials, let chat else {
            if manual { notice = offline ? "当前离线，无法核对结果，请稍后重试。" : "会话存储不可用，请重新登录后重试。" }
            return
        }
        let generation = epoch
        var changed = false
        var failure: String?
        for entry in chatMessages where entry.state == .unknown && entry.role == "user" {
            guard let requestID = entry.requestID else { continue }
            do {
                let job = try await api.aiJob(requestID: requestID, token: credentials.token)
                guard generation == epoch else { return }
                if job.state == "COMPLETED", let result = job.result {
                    try? await chat.complete(entry, text: result.text.isEmpty ? "（模型未返回文本）" : result.text)
                    changed = true
                } else if job.state == "REJECTED" {
                    try? await replaceMessage(entry, state: .failed, failure: "模型服务拒绝了请求，额度已释放。", in: chat)
                    changed = true
                }
            } catch {
                guard generation == epoch else { return }
                failure = message(error)
                break
            }
        }
        guard generation == epoch else { return }
        if changed {
            await loadChat()
            await loadAIAccount()
            if manual { notice = "已核对：待确认结果已更新。" }
        } else if let failure {
            notice = "未能核对结果：\(failure)"
        } else if manual {
            notice = "这些请求仍在处理中，结果确认后会自动显示，不会重复计费。"
        }
    }

    // MARK: - 待确认结果自动核对
    // 流式抖动/回退轮询超时会留下 unknown 消息：后台循环重查直至出结果，
    // 用户无需手动点「核对」；离开页面或换账号自动停止。
    @ObservationIgnored private var autoResolveTask: Task<Void, Never>?

    func scheduleAutoResolve() {
        guard autoResolveTask == nil else { return }
        guard chatMessages.contains(where: { $0.state == .unknown && $0.role == "user" }) else { return }
        let generation = epoch
        autoResolveTask = Task { [weak self] in
            // 最多约 5 分钟：每 3 秒核对一次，出结果/换纪元即停。
            for _ in 0..<100 {
                try? await Task.sleep(nanoseconds: 3_000_000_000)
                guard let self else { break }
                if Task.isCancelled || self.epoch != generation { break }
                guard self.chatMessages.contains(where: { $0.state == .unknown && $0.role == "user" }) else { break }
                await self.aiResolvePending(manual: false)
            }
            await MainActor.run { [weak self] in self?.autoResolveTask = nil }
        }
    }

    func stopAutoResolve() {
        autoResolveTask?.cancel()
        autoResolveTask = nil
    }

    func agentJobs() async throws -> [AgentJob] {
        guard let api, let credentials, let id = selectedProjectID, !offline else { return [] }
        return try await projectRead(id) { try await api.agentJobs(projectID: id, token: credentials.token) }
    }

    func projectHistory(projectID: String) async throws -> [VersionSummary] {
        guard let api, let credentials, let id = selectedProjectID, !offline else { return [] }
        return try await projectRead(id) { try await api.projectHistory(projectID: id, token: credentials.token) }
    }

    func versionDiff(projectID: String, from: Int, to: Int) async throws -> VersionDiff {
        guard let api, let credentials, let id = selectedProjectID, !offline else { throw APIError.permissionDenied }
        return try await projectRead(id) { try await api.versionDiff(projectID: id, from: from, to: to, token: credentials.token) }
    }

    func proposalDiff(proposalID: String) async throws -> ProposalDiff {
        guard let api, let credentials, let id = selectedProjectID, !offline else { throw APIError.permissionDenied }
        return try await projectRead(id) { try await api.proposalDiff(projectID: id, proposalID: proposalID, token: credentials.token) }
    }

    private(set) var billingCatalogState: BillingCatalog?

    func loadBillingCatalog() async {
        guard let api else { return }
        do { billingCatalogState = try await api.billingCatalog() }
        catch { billingCatalogState = nil }
    }

    func exportAccountData() async throws -> Data {
        guard let api, let credentials, !offline else { throw APIError.permissionDenied }
        let generation = epoch
        let bytes = try await api.accountData(token: credentials.token, userID: credentials.user.id)
        guard generation == epoch else { throw CancellationError() }
        return bytes
    }

    func eraseAccount(password: String, email: String) async throws {
        guard let api, let credentials, !offline else { throw APIError.permissionDenied }
        let generation = epoch
        let receipt = try await api.eraseAccount(token: credentials.token, password: password, email: email)
        guard generation == epoch else { throw CancellationError() }
        _ = await handleSession(APIError.server(status: 401, code: "ACCOUNT_CLOSED"))
        notice = "账号已关闭，回执：\(receipt.receipt_id)。本机原片仍保留，可在登录页离线导出；共享协作与财务审计按说明保留。"
    }

    private func replaceMessage(_ original: AIChatMessage, state: AIChatMessage.State, failure: String?, in chat: AIChatStore) async throws {
        var updated = original
        updated.state = state
        updated.failure = failure
        try await chat.replace(updated)
    }

    // MARK: - 评论协作（发送回复 / 解决 / 重开）

    // 邀请成员：返回私密链接由邀请者自行分享（服务端不发邮件）；仅 OWNER/ADMIN 可发起。
    func sendInvitation(email: String, role: String) async -> URL? {
        guard !busy, let api, let credentials, !offline else { return nil }
        let trimmed = email.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard trimmed.contains("@"), trimmed.count <= 254 else {
            notice = "请输入有效的邮箱地址。"
            return nil
        }
        guard ["ADMIN", "EDITOR", "REVIEWER", "VIEWER"].contains(role) else { return nil }
        let generation = epoch
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            let workspaces = try await api.workspaces(token: credentials.token)
            guard let workspace = workspaces.first(where: { ["OWNER", "ADMIN"].contains($0.role) }) else {
                notice = "只有工作区所有者或管理员可以邀请成员。"
                return nil
            }
            return try await api.createInvitation(workspaceID: workspace.id, email: trimmed,
                                                  role: role, projectID: nil, token: credentials.token)
        } catch {
            notice = message(error)
            return nil
        }
    }

    func replyComment(threadID: String, body: String) async {
        guard !busy, let snapshot, let api, let credentials,
              currentProject == .cloud(snapshot.project_id), !offline else { return }
        let trimmed = body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed.count <= 2000 else { notice = "评论内容为空或过长。"; return }
        let generation = epoch
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            try await projectRead(snapshot.project_id) {
                try await api.replyComment(projectID: snapshot.project_id, threadID: threadID,
                                           body: trimmed, idempotencyKey: UUID().uuidString.lowercased(),
                                           token: credentials.token)
            }
            await reloadSnapshot()
        } catch {
            notice = message(error)
        }
    }

    func resolveComment(threadID: String, resolve: Bool) async {
        guard !busy, let snapshot, let api, let credentials,
              currentProject == .cloud(snapshot.project_id), !offline else { return }
        let generation = epoch
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            try await projectRead(snapshot.project_id) {
                _ = try await api.resolveComment(projectID: snapshot.project_id, threadID: threadID,
                                                 resolve: resolve, snapshot: snapshot, token: credentials.token)
            }
            await reloadSnapshot()
        } catch {
            notice = message(error)
        }
    }

    // 项目授权管理：仅 OWNER/ADMIN；EDITOR 专属权限客户端先镜像校验，服务端最终权威。
    func updateGrant(userID: String, role: String, canApprove: Bool, canEditFormal: Bool) async {
        guard !busy, let snapshot, let api, let credentials,
              currentProject == .cloud(snapshot.project_id), !offline else {
            notice = grantBlockReason
            return
        }
        let input: GrantUpdateInput
        do { input = try GrantUpdateInput(role: role, canApprove: canApprove, canEditFormal: canEditFormal) }
        catch { notice = "只有编辑角色可以持有审批或正式编辑权限。"; return }
        let generation = epoch
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            try await projectRead(snapshot.project_id) {
                try await api.updateGrant(projectID: snapshot.project_id, userID: userID,
                                          input: input, token: credentials.token)
            }
            await reloadSnapshot()
        } catch {
            notice = message(error)
        }
    }

    // 成员权限操作的前置条件：静默返回会让「保存/撤销」看起来完全失灵，必须说明原因。
    private var grantBlockReason: String {
        if busy { return "上一个请求仍在处理，请稍候再试。" }
        if offline { return "当前离线，无法修改成员权限。" }
        if api == nil || credentials == nil { return "登录状态已失效，请重新登录。" }
        guard let snapshot, currentProject == .cloud(snapshot.project_id) else {
            return "项目上下文已变化，请返回项目页重新进入后再试。"
        }
        return "无法修改成员权限，请返回项目页重试。"
    }

    func revokeGrant(userID: String) async {
        guard !busy, let snapshot, let api, let credentials,
              currentProject == .cloud(snapshot.project_id), !offline else {
            notice = grantBlockReason
            return
        }
        let generation = epoch
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            try await projectRead(snapshot.project_id) {
                try await api.revokeGrant(projectID: snapshot.project_id, userID: userID, token: credentials.token)
            }
            await reloadSnapshot()
        } catch {
            notice = message(error)
        }
    }

    func image(photoID: String) async throws -> UIImage {
        guard let api, let credentials, let snapshot,
              let descriptor = snapshot.previews.first(where: { $0.photo_id == photoID }) else {
            throw APIError.invalidResponse
        }
        let generation = epoch
        let projectID = snapshot.project_id
        let key = "\(credentials.user.id):\(projectID):\(photoID):\(descriptor.content_sha256)" as NSString
        if let image = images.object(forKey: key) { return image }
        // 离线时已查看过的预览可从磁盘缓存继续读取；缓存不提升权限，仅复用已下载的授权副本。
        if let cached = await previewCache?.cached(sha: descriptor.content_sha256),
           let image = Self.decodePreviewImage(cached, descriptor: descriptor) {
            images.setObject(image, forKey: key, cost: Int(image.size.width * image.size.height * 4))
            return image
        }
        guard !offline else { throw URLError(.notConnectedToInternet) }
        do {
            let bytes = try await api.preview(projectID: projectID, descriptor: descriptor, token: credentials.token)
            try Task.checkCancellation()
            guard generation == epoch, selectedProjectID == projectID else { throw CancellationError() }
            guard let image = Self.decodePreviewImage(bytes, descriptor: descriptor) else { throw APIError.invalidResponse }
            await previewCache?.store(bytes, sha: descriptor.content_sha256)
            images.setObject(image, forKey: key, cost: Int(image.size.width * image.size.height * 4))
            return image
        } catch {
            if generation == epoch { _ = await handleProject(error, id: projectID) }
            throw error
        }
    }

    private static func decodePreviewImage(_ bytes: Data, descriptor: PreviewDescriptor) -> UIImage? {
        guard let source = CGImageSourceCreateWithData(bytes as CFData, nil),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let width = properties[kCGImagePropertyPixelWidth] as? Int,
              let height = properties[kCGImagePropertyPixelHeight] as? Int,
              width == descriptor.width, height == descriptor.height,
              let cgImage = CGImageSourceCreateThumbnailAtIndex(source, 0, [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceCreateThumbnailWithTransform: true,
                kCGImageSourceThumbnailMaxPixelSize: 2048
              ] as CFDictionary) else { return nil }
        return UIImage(cgImage: cgImage)
    }

    func boards() async throws -> [Board] {
        guard let api, let credentials, let id = selectedProjectID, !offline else { return [] }
        return try await projectRead(id) { try await api.boards(projectID: id, token: credentials.token) }
    }

    func projectSyncState(projectID: String) async throws -> ProjectSyncState {
        guard let api, let credentials, selectedProjectID == projectID, !offline else { throw APIError.permissionDenied }
        return try await projectRead(projectID) { try await api.syncState(projectID: projectID, token: credentials.token) }
    }

    func changeProjectSyncState(projectID: String, command: SyncModeCommand) async throws {
        guard let api, let credentials, selectedProjectID == projectID, !offline else { throw APIError.permissionDenied }
        _ = try await projectRead(projectID) { try await api.changeSyncState(projectID: projectID, command: command, token: credentials.token) }
        await reloadProjects()
        await reloadSnapshot()
    }

    func moveBoardNode(boardID: String, board: Board, nodeID: String, x: Double, y: Double) async throws -> CanvasCommandReceipt {
        guard let store, let id = selectedProjectID, id == board.project_id, boardID == board.id,
              !offline, !canvasSending else { throw APIError.permissionDenied }
        let command = try CanvasMoveCommand(board: board, nodeID: nodeID, x: x, y: y)
        let generation = epoch
        canvasSending = true
        defer { if generation == epoch { canvasSending = false } }
        try await store.prepareCanvas(boardID: boardID, command: command)
        guard generation == epoch, id == selectedProjectID else { throw CancellationError() }
        pendingBoards.insert(boardID)
        return try await submitPendingCanvas(boardID: boardID, command: command)
    }

    func loadCanvasPending(boardID: String) async {
        guard let store, let id = selectedProjectID else { return }
        let generation = epoch
        do {
            let pending = try await store.pendingCanvas(boardID: boardID)
            guard generation == epoch, id == selectedProjectID else { return }
            if let pending, pending.command.projectId == id { pendingBoards.insert(boardID) }
        } catch { if generation == epoch { notice = message(error) } }
    }

    func resolveCanvas(boardID: String) async throws -> CanvasCommandReceipt {
        guard let api, let credentials, let store, let id = selectedProjectID,
              !offline, !canvasSending else { throw APIError.permissionDenied }
        let generation = epoch
        canvasSending = true
        defer { if generation == epoch { canvasSending = false } }
        guard let pending = try await store.pendingCanvas(boardID: boardID), pending.command.projectId == id,
              generation == epoch, id == selectedProjectID else { throw APIError.unconfirmedCommand }
        // 用户明确点击后核对：查询失败不发送；只有权威返回 404 才继续原编号。
        let receipt = try await projectRead(id) {
            try await api.canvasReceipt(boardID: boardID, command: pending.command, token: credentials.token)
        }
        if let receipt {
            try await store.acknowledgeCanvas(boardID: boardID, receipt: receipt)
            guard generation == epoch, id == selectedProjectID else { throw CancellationError() }
            pendingBoards.remove(boardID)
            return receipt
        }
        return try await submitPendingCanvas(boardID: boardID, command: pending.command)
    }

    private func submitPendingCanvas(boardID: String, command: CanvasMoveCommand) async throws -> CanvasCommandReceipt {
        guard let api, let credentials, let store, selectedProjectID == command.projectId else { throw APIError.permissionDenied }
        let generation = epoch
        do {
            let receipt = try await projectRead(command.projectId) {
                try await api.submitCanvasCommand(boardID: boardID, command: command, token: credentials.token)
            }
            try await store.acknowledgeCanvas(boardID: boardID, receipt: receipt)
            guard generation == epoch, selectedProjectID == command.projectId else { throw CancellationError() }
            pendingBoards.remove(boardID)
            return receipt
        } catch APIError.server(let status, let code) where [400, 409].contains(status) && code != "IDEMPOTENCY_CONFLICT" {
            guard generation == epoch, selectedProjectID == command.projectId else { throw CancellationError() }
            // 即使本次返回冲突，也先排除更早一次传输已提交但丢回执的情况。
            if let receipt = try await api.canvasReceipt(boardID: boardID, command: command, token: credentials.token) {
                try await store.acknowledgeCanvas(boardID: boardID, receipt: receipt)
                if generation == epoch { pendingBoards.remove(boardID) }
                return receipt
            }
            try await store.rejectCanvas(boardID: boardID, commandID: command.commandId)
            if generation == epoch { pendingBoards.remove(boardID) }
            throw APIError.server(status: status, code: code)
        }
    }

    func applyProposal(proposal: Proposal, acceptedOperationIDs: [String]) async throws {
        guard let api, let credentials, let snapshot = snapshot, !offline else { throw APIError.permissionDenied }
        let command = try ProposalApplyCommand(snapshot: snapshot, proposal: proposal, acceptedOperationIDs: acceptedOperationIDs)
        _ = try await projectRead(snapshot.project_id) {
            try await api.applyProposal(command, token: credentials.token)
        }
        await reloadSnapshot()
    }

    func nodes(board: Board) async throws -> [BoardNode] {
        guard let api, let credentials, let id = selectedProjectID, !offline else { return [] }
        guard board.project_id == id else { throw APIError.permissionDenied }
        return try await projectRead(id) { try await api.nodes(boardID: board.id, token: credentials.token) }
    }

    func agentResults() async throws -> [AgentResult] {
        guard let api, let credentials, let id = selectedProjectID, !offline else { return [] }
        return try await projectRead(id) { try await api.agentResults(projectID: id, token: credentials.token) }
    }

    func research(projectID: String) async throws -> [ResearchEntry] {
        guard let api, let credentials, let id = selectedProjectID, !offline else { return [] }
        return try await projectRead(id) { try await api.research(projectID: id, token: credentials.token) }
    }

    func createResearch(projectID: String, draft: ResearchDraft) async throws {
        guard let api, let credentials, let id = selectedProjectID, !offline else { throw APIError.permissionDenied }
        try await projectRead(id) { try await api.createResearch(projectID: id, draft: draft, token: credentials.token) }
    }

    func updateResearch(projectID: String, referenceID: String, patch: ResearchPatch) async throws {
        guard let api, let credentials, let id = selectedProjectID, !offline else { throw APIError.permissionDenied }
        try await projectRead(id) { try await api.updateResearch(projectID: id, referenceID: referenceID, patch: patch, token: credentials.token) }
    }

    func deleteResearch(projectID: String, referenceID: String) async throws {
        guard let api, let credentials, let id = selectedProjectID, !offline else { throw APIError.permissionDenied }
        try await projectRead(id) { try await api.deleteResearch(projectID: id, referenceID: referenceID, token: credentials.token) }
    }

    private func projectRead<T>(_ id: String, action: () async throws -> T) async throws -> T {
        let generation = epoch
        do {
            let result = try await action()
            guard generation == epoch, id == selectedProjectID else { throw CancellationError() }
            return result
        } catch {
            if generation == epoch { _ = await handleProject(error, id: id) }
            throw error
        }
    }

    func logout() async {
        guard let api, let credentials else { return }
        let oldStore = store
        let oldPreviewCache = previewCache
        let oldChat = chat
        do { try vault.clear() }
        catch { notice = message(error); return }
        clearMemory()
        let generation = epoch
        do { try await oldStore?.clear() }
        catch { notice = "已退出登录；部分本机缓存未能清理，请重新打开应用后处理。" }
        await oldPreviewCache?.clear()
        try? await oldChat?.clear()
        do { try await api.logout(token: credentials.token) }
        catch { if generation == epoch { notice = "本机已退出。远端注销未确认，可在账户中撤销此会话。" } }
    }

    private func handleSession(_ error: Error) async -> Bool {
        guard (error as? APIError)?.revokesSession == true else { return false }
        let oldStore = store
        let oldPreviewCache = previewCache
        let oldChat = chat
        do { try vault.clear() }
        catch { notice = message(error); clearMemory(); return true }
        clearMemory()
        do { try await oldStore?.clear() }
        catch { notice = "登录已失效；部分本机缓存未能清理。"; return true }
        await oldPreviewCache?.clear()
        try? await oldChat?.clear()
        notice = "登录已失效，请重新登录。"
        return true
    }

    private func handleProject(_ error: Error, id: String) async -> Bool {
        if await handleSession(error) { return true }
        guard (error as? APIError)?.deniesProject == true else { return false }
        images.removeAllObjects()
        if selectedProjectID == id { resetProject() }
        projects.removeAll { $0.id == id }
        do { try await store?.revokeProject(id) }
        catch { notice = "访问已停止，但本机缓存清理失败。"; return true }
        notice = "项目访问权限已变化，已停止读取和提交。"
        return true
    }

    private func resetProject() {
        nativeProjection = nil
        nativeContext = nil
        nativeOwner = nil
        nativeSyncedStamp = nil
        nativeTaskError = nil
        nativeReviewReceipt = nil
        projectRequest = UUID()
        currentProject = nil
        chatMessages = []
        visionDraftID = nil
        snapshot = nil
        pending = nil
        cachedAt = nil
        loadingProject = false
    }

    private func clearMemory() {
        let previousAI = nativeAI
        nativeAI = nil
        Task { await previousAI?.close() }
        epoch = UUID()
        resetProject()
        credentials = nil
        projects = []
        api = nil
        store = nil
        previewCache = nil
        chat = nil
        chatMessages = []
        aiAccountState = nil
        aiAccountError = nil
        aiSending = false
        streamingText = nil
        visionDraftID = nil
        canvasSending = false
        pendingBoards = []
        billingCatalogState = nil
        syncOverview = []
        migration?.shutdown()
        migration = nil
        localLibrary?.shutdown()
        localLibrary = nil
        devices = []
        loadingDevices = false
        offline = false
        busy = false
        images.removeAllObjects()
    }

    private func isConnectivityError(_ error: Error) -> Bool {
        guard let error = error as? URLError else { return false }
        return [.notConnectedToInternet, .networkConnectionLost, .timedOut,
                .cannotFindHost, .cannotConnectToHost, .dnsLookupFailed].contains(error.code)
    }

    private func message(_ error: Error) -> String {
        if let value = error as? LocalizedError, let description = value.errorDescription { return description }
        return "暂时无法完成请求，请检查网络后重试。"
    }
}

extension AppModel {
    private func aiPayload(_ value: [String: Any]) throws -> Data {
        try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    }

    private func sharedAIClient() async throws -> NativeAIClient {
        guard let credentials, let client = nativeAI, !offline else { throw APIError.permissionDenied }
        let user = try JSONSerialization.jsonObject(with: JSONEncoder().encode(credentials.user))
        let directory = try privateAIDirectory(root.appendingPathComponent("AgentCore"))
        let payload = try aiPayload(["directory": directory.path,
                                     "origin": try configuration(address).identity,
                                     "token": credentials.token, "user": user])
        try await client.open(payload)
        return client
    }

    private func privateAIDirectory(_ directory: URL) throws -> URL {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        var url = directory
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        try url.setResourceValues(values)
        return url
    }

    private struct NativeSource {
        let payload: Data
        let projectID: String
        let photoIDs: [String: String]
    }

    private func nativeSource(vision: PreparedVision? = nil) async throws -> NativeSource {
        let hash: (String) -> String = { SHA256.hash(data: Data($0.utf8)).map { String(format: "%02x", $0) }.joined() }
        let reference: String
        var title = "摄影讨论", brief = "", photos: [[String: Any]] = [], sequence: [String] = []
        var cloudSequence: [String] = []
        switch currentProject {
        case .local(let id):
            reference = "local:" + id
            guard let library = localLibrary?.library,
                  let local = try await library.projects().first(where: { $0.id == id }) else { throw APIError.permissionDenied }
            title = local.title
            // 来源版本与画布摘要作为项目数据，不能改变宿主工具权限。
            brief = "本机项目版本：\(local.updatedAt.timeIntervalSince1970)"
            if let canvas = local.canvas {
                let allowed = Set(vision?.photos.map(\.photoID) ?? local.photos.map(\.id))
                let mirror = "native-" + hash(reference + ":" + (local.photos.isEmpty ? "true" : "false"))
                let nodes = canvas.nodes.filter { $0.photoID == nil || allowed.contains($0.photoID!) }.prefix(100).map { node in
                    LocalCanvasNode(id: node.id, kind: node.kind, photoID: node.photoID.map { hash(mirror + ":" + $0) },
                        text: node.text.map { String(decoding: $0.utf8.prefix(500), as: UTF8.self) },
                        x: node.x, y: node.y, width: node.width, height: node.height, zOrder: node.zOrder)
                }
                let summary = LocalCanvas(nodes: Array(nodes), annotations: [])
                brief += "\n当前画布摘要（最多 100 个节点；文字可能截取；非工具指令）：" + String(decoding: try JSONEncoder().encode(summary), as: UTF8.self)
            }
            photos = local.photos.map { ["id": $0.id, "filename": $0.filename, "sourceHash": $0.sourceSHA256, "status": $0.selection.rawValue.lowercased(), "width": $0.width, "height": $0.height] }
            sequence = local.sequencePhotoIDs ?? local.photos.filter { $0.selection == .KEEP }.map(\.id)
        case .cloud(let id):
            reference = "cloud:" + id
            guard let snapshot, snapshot.project_id == id, snapshot.principal.id == credentials?.user.id else { throw APIError.permissionDenied }
            title = snapshot.title
            brief = "云端项目版本：\(snapshot.revision)。这是执行上下文副本，正式变更必须提交云端权威提案。"
            photos = snapshot.snapshot.photos.map { photo in
                let preview = snapshot.previews.first { $0.photo_id == photo.id }
                return ["id": photo.id, "filename": photo.filename, "sourceHash": photo.source_sha256, "status": photo.selection.rawValue.lowercased(), "width": preview?.width ?? 1, "height": preview?.height ?? 1]
            }
            sequence = snapshot.snapshot.sequences.first?.items.map(\.photo_id) ?? []
            cloudSequence = sequence
            let kept = Set(snapshot.snapshot.photos.filter { $0.selection == .KEEP }.map(\.id))
            sequence = sequence.filter { kept.contains($0) }
        case nil:
            reference = "account-discussion"
        }
        if let vision {
            guard let credentials else { throw APIError.permissionDenied }
            let scope = AccountScope(configuration: try configuration(address), userID: credentials.user.id)
            let directory = try privateAIDirectory(root.appendingPathComponent("AIInput").appendingPathComponent(scope.key))
            for photo in vision.photos {
                guard let index = photos.firstIndex(where: { $0["id"] as? String == photo.photoID }) else { throw APIError.permissionDenied }
                let digest = SHA256.hash(data: photo.preview.jpeg).map { String(format: "%02x", $0) }.joined()
                let file = directory.appendingPathComponent(digest + ".jpg")
                try photo.preview.jpeg.write(to: file, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
                photos[index]["previewPath"] = file.path
            }
        }
        let projectID = "native-" + hash(reference + ":" + (photos.isEmpty ? "true" : "false"))
        let ids = Dictionary(uniqueKeysWithValues: photos.compactMap { item -> (String, String)? in
            guard let id = item["id"] as? String else { return nil }
            return (id, hash(projectID + ":" + id))
        })
        if !cloudSequence.isEmpty {
            let allowed = Set(vision?.photos.map(\.photoID) ?? Array(ids.keys))
            let actual = cloudSequence.filter { allowed.contains($0) }.compactMap { ids[$0] }
            brief += "\n云端本次范围的实际序列（执行副本仅将已保留照片放入本地序列表）：" + String(decoding: try JSONEncoder().encode(actual), as: UTF8.self)
        }
        return NativeSource(payload: try aiPayload(["context": ["reference": reference, "title": title, "brief": brief,
                                                                  "photos": photos, "sequence": sequence]]),
                            projectID: projectID, photoIDs: ids)
    }

    private func startNativeTask(text: String, vision: PreparedVision?, client: NativeAIClient) async throws {
        let generation = epoch, owner = currentProject
        if case .cloud(let id) = owner, let api, let credentials {
            let fresh = try await projectRead(id) { try await api.snapshot(projectID: id, credentials: credentials) }
            guard generation == epoch, currentProject == owner else { throw CancellationError() }
            snapshot = fresh
        }
        let source = try await nativeSource(vision: vision)
        if source.photoIDs.count > 500 && vision == nil { throw NativeAIError(code: "SCOPE_TOO_LARGE", message: "项目超过 500 张，请先选择本次讨论的照片。") }
        let bytes = try await client.perform("syncContext", payload: source.payload)
        let context = try JSONDecoder().decode(NativeAIContext.self, from: bytes)
        guard generation == epoch, currentProject == owner else { throw CancellationError() }
        nativeContext = context; nativeOwner = owner; nativeSyncedStamp = nativeSourceStamp
        var consent: Any = NSNull()
        if let vision {
            let config = try await client.perform("configuration", payload: aiPayload([:]))
            let value = try JSONSerialization.jsonObject(with: config) as? [String: Any]
            guard let provider = value?["consentProvider"] as? String else { throw APIError.permissionDenied }
            let result = try await client.perform("consent", payload: aiPayload(["request": ["requestId": UUID().uuidString.lowercased(),
                "projectId": context.projectId, "provider": provider, "mode": "task", "quota": vision.photos.count]]))
            let value2 = try JSONSerialization.jsonObject(with: result) as? [String: Any]
            guard let id = value2?["id"] as? Int else { throw APIError.invalidResponse }
            consent = id
        }
        guard generation == epoch, currentProject == owner else { throw CancellationError() }
        let scope = vision?.photos.compactMap { context.photoIDs[$0.photoID] } ?? Array(context.photoIDs.values).sorted()
        let budget = try JSONSerialization.jsonObject(with: JSONEncoder().encode(NativeAIBudget(images: vision?.photos.count ?? 0)))
        var command: [String: Any] = ["type": "StartConversation", "expectedAccountId": credentials?.user.id ?? "", "goal": text,
            "selectedPhotoIds": scope, "focusPhotoIds": vision == nil ? [] : scope, "targetCount": max(1, min(aiTargetCount, max(1, scope.count))),
            "budget": budget, "consentId": consent, "sourceBoard": NSNull()]
        if vision == nil, let task = nativeProjection?.tasks.first(where: { $0.state == "needs_input" }) {
            command = ["type": "SubmitFeedback", "taskId": task.id, "answer": text]
        }
        var canvasAccess = false
        if case .local(let id) = owner, let library = localLibrary?.library,
           let local = try await library.projects().first(where: { $0.id == id }) { canvasAccess = local.migration == nil }
        guard generation == epoch, currentProject == owner else { throw CancellationError() }
        _ = try await client.perform("agentRequest", payload: aiPayload(["canvasAccess": canvasAccess, "request": ["requestId": UUID().uuidString.lowercased(),
                                                                                     "projectId": context.projectId, "command": command]]))
    }

    func refreshNativeTasks() async {
        let generation = epoch, owner = currentProject
        guard credentials != nil, !offline, !aiSending, !busy else { return }
        do {
            let client = try await sharedAIClient()
            let context: NativeAIContext
            let stamp = nativeSourceStamp
            if let existing = nativeContext, nativeOwner == owner, nativeSyncedStamp == stamp {
                context = existing
            } else {
                let source = try await nativeSource()
                let contextBytes: Data
                do {
                    contextBytes = try await client.perform("syncContext", payload: source.payload)
                    if generation == epoch, owner == currentProject { nativeSyncedStamp = stamp }
                } catch let error as NativeAIError where error.code == "TASK_ACTIVE" {
                    let project = try await client.perform("getProject", payload: aiPayload(["project": source.projectID]))
                    let object = try JSONSerialization.jsonObject(with: project) as? [String: Any]
                    contextBytes = try aiPayload(["projectId": source.projectID, "photoIDs": source.photoIDs, "revision": object?["revision"] ?? 0])
                }
                context = try JSONDecoder().decode(NativeAIContext.self, from: contextBytes)
            }
            let bytes = try await client.perform("projection", payload: aiPayload(["project": context.projectId]))
            let projection = try JSONDecoder().decode(NativeAIProjection.self, from: bytes)
            guard generation == epoch, owner == currentProject else { return }
            nativeOwner = owner; nativeContext = context; nativeProjection = projection; nativeTaskError = nil
        } catch {
            if generation == epoch, owner == currentProject { nativeTaskError = message(error) }
        }
    }

    private var nativeSourceStamp: String {
        switch currentProject {
        case .local(let id): "local:\(id):\(selectedLocalProject?.updatedAt.timeIntervalSince1970 ?? -1)"
        case .cloud(let id): "cloud:\(id):\(snapshot?.revision ?? -1)"
        case nil: "account"
        }
    }

    func controlNativeTask(_ task: NativeAITask, action: String) async {
        guard let context = nativeContext, nativeOwner == currentProject, !aiSending else { return }
        let generation = epoch, owner = currentProject
        do {
            let client = try await sharedAIClient()
            if action == "query" {
                _ = try await client.perform("queryVisionResult", payload: aiPayload(["project": context.projectId, "task": task.id]))
            } else {
                var command: [String: Any] = ["type": action, "taskId": task.id]
                if action == "ResumeTask" { command["budget"] = try JSONSerialization.jsonObject(with: JSONEncoder().encode(task.budget)) }
                _ = try await client.perform("agentRequest", payload: aiPayload(["request": ["requestId": UUID().uuidString.lowercased(), "projectId": context.projectId, "command": command]]))
            }
            if generation == epoch, owner == currentProject { await refreshNativeTasks(); await loadAIAccount() }
        } catch { if generation == epoch, owner == currentProject { notice = message(error) } }
    }

    func nativePhotoName(_ coreID: String) -> String {
        guard let id = nativeContext?.photoIDs.first(where: { $0.value == coreID })?.key else { return "原照片已移出当前项目" }
        return observationPhotos.first(where: { $0.id == id })?.filename ?? id
    }
    func legacyAIChat() async -> [AIChatMessage] {
        guard let chat else { return [] }
        let generation = epoch
        let result = (try? await chat.messages(localProjectID: nil, cloudProjectID: nil)) ?? []
        return generation == epoch ? result : []
    }

    func saveNativePreference(_ preference: NativeAIPreference, task: NativeAITask, statement: String?) async {
        guard let context = nativeContext, nativeOwner == currentProject, !busy else { return }
        let generation = epoch, owner = currentProject
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            let client = try await sharedAIClient()
            _ = try await client.perform("preference", payload: aiPayload(["project": context.projectId, "task": task.id,
                                                                         "proposal": preference.id, "statement": statement as Any? ?? NSNull()]))
            if generation == epoch, currentProject == owner {
                busy = false; await refreshNativeTasks()
                notice = statement == nil ? "已忽略这条建议。" : "已保存为当前项目的偏好。"
            }
        } catch { if generation == epoch, currentProject == owner { notice = message(error) } }
    }

    func applyNativeDraft(_ draft: NativeAIDraft, selectedIDs: Set<String>) async -> Bool {
        guard !busy, !aiSending, let credentials, !selectedIDs.isEmpty, let owner = currentProject,
              nativeOwner == owner, nativeContext?.projectId == draft.projectId,
              selectedIDs.isSubset(of: Set(draft.items.map(\.photo_id))) else { return false }
        let generation = epoch
        let applicationID = draft.id + ":" + String(draft.revision) + ":" + SHA256.hash(data: Data(selectedIDs.sorted().joined(separator: ",").utf8)).map { String(format: "%02x", $0) }.joined()
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            let scope = AccountScope(configuration: try configuration(address), userID: credentials.user.id)
            let reviews = NativeAIReviewStore(root: root.appendingPathComponent("AIReviews"), scope: scope)
            let previous = try await reviews.record(applicationID: applicationID)
            if previous?.completed == true { nativeReviewReceipt = "这份提案已提交。"; return true }
            if case .local(let id) = owner, let library = localLibrary?.library,
               let local = try await library.projects().first(where: { $0.id == id }),
               local.appliedAIDraftIDs?.contains(applicationID) == true { nativeReviewReceipt = "这些建议已采纳。"; return true }
            if case .cloud(let id) = owner, let api {
                let fresh = try await projectRead(id) { try await api.snapshot(projectID: id, credentials: credentials) }
                guard generation == epoch, currentProject == owner else { throw CancellationError() }
                snapshot = fresh
            }
            let client = try await sharedAIClient()
            let source = try await nativeSource()
            let bytes = try await client.perform("syncContext", payload: source.payload)
            let context = try JSONDecoder().decode(NativeAIContext.self, from: bytes)
            // 已落盘的云端原命令可继续核对；新审批必须与原任务的基线相同。
            guard previous != nil || (!draft.stale && draft.state == "pending" && context.revision == draft.baseRevision) else {
                throw NativeAIError(code: "REVISION_CONFLICT", message: "项目已更新，请重新委托；这份草案已保留。")
            }
            let reverse = Dictionary(uniqueKeysWithValues: context.photoIDs.map { ($0.value, $0.key) })
            let chosen = draft.items.filter { selectedIDs.contains($0.photo_id) }.sorted { $0.position < $1.position }
            var decisions: [String: Selection] = [:], ordered: [String] = []
            for item in chosen {
                guard let id = reverse[item.photo_id], let selection = Selection(rawValue: item.proposed_action.uppercased()) else { throw APIError.invalidResponse }
                decisions[id] = selection
                if selection == .KEEP { ordered.append(id) }
            }
            guard generation == epoch, currentProject == owner else { throw CancellationError() }
            switch owner {
            case .local(let id):
                guard let library = localLibrary?.library,
                      let local = try await library.projects().first(where: { $0.id == id }) else { throw APIError.permissionDenied }
                // 更新时间已进入上下文版本；再做一次比较，防止审阅与写回之间发生本机编辑。
                let verify = try await nativeSource()
                guard source.payload == verify.payload else { throw APIError.unconfirmedCommand }
                try await library.applyAISelection(projectID: id, expectedUpdatedAt: local.updatedAt, applicationID: applicationID,
                                                   decisions: decisions, orderedPhotoIDs: ordered)
                if generation == epoch, currentProject == owner {
                    await localLibrary?.reload()
                    nativeReviewReceipt = "已采纳勾选的建议；保留列表按草案顺序排列。"
                }
            case .cloud(let id):
                guard let api, let snapshot, snapshot.project_id == id else { throw APIError.permissionDenied }
                let old = snapshot.snapshot.sequences.first?.items.map(\.photo_id) ?? []
                var operations: [ProposalCreateCommand.DraftOperation] = []
                for photo in old where decisions[photo] != nil && decisions[photo] != .KEEP {
                    operations.append(.init(op: "remove", operation_id: "remove-" + photo, photo_id: photo, to_index: nil))
                }
                for (index, photo) in ordered.enumerated() {
                    operations.append(.init(op: old.contains(photo) ? "move" : "add", operation_id: "place-" + photo, photo_id: photo, to_index: index))
                }
                let record = try await reviews.prepare(applicationID: applicationID, snapshot: snapshot, title: draft.title, operations: operations)
                try await projectRead(id) { try await api.createProposal(record.create, token: credentials.token) }
                try await projectRead(id) { try await api.submitProposal(record.submit, token: credentials.token) }
                try await reviews.complete(applicationID: applicationID)
                if generation == epoch, currentProject == owner { nativeReviewReceipt = "已提交云端提案，可在项目中逐项审批。"; await reloadSnapshot() }
            }
            if generation == epoch, currentProject == owner { busy = false; await refreshNativeTasks() }
            return true
        } catch {
            if generation == epoch, currentProject == owner { notice = message(error) }
            return false
        }
    }

    func nativeBoardNodes(_ board: NativeAIBoard) async throws -> [NativeAINode] {
        guard let context = nativeContext, nativeOwner == currentProject else { throw APIError.permissionDenied }
        let client = try await sharedAIClient()
        return try JSONDecoder().decode([NativeAINode].self, from: await client.perform("boardNodes", payload: aiPayload(["project": context.projectId, "board": board.id, "revision": board.revision])))
    }

    func applyNativeBoard(_ board: NativeAIBoard, nodes: [NativeAINode]) async -> Bool {
        guard !busy, !aiSending, case .local(let projectID) = currentProject,
              let library = localLibrary?.library, let context = nativeContext,
              nativeProjection?.tasks.contains(where: { $0.needsReview && $0.boardDrafts.contains(where: { $0.id == board.id }) }) == true else { return false }
        let generation = epoch, owner = currentProject
        busy = true
        defer { if generation == epoch { busy = false } }
        do {
            let applicationID = "board:" + board.id + ":" + String(board.revision)
            guard let local = try await library.projects().first(where: { $0.id == projectID }) else { throw APIError.permissionDenied }
            if local.appliedAIDraftIDs?.contains(applicationID) == true { nativeReviewReceipt = "这份画布草案已添加。"; return true }
            let client = try await sharedAIClient()
            let source = try await nativeSource()
            let synced = try JSONDecoder().decode(NativeAIContext.self, from: await client.perform("syncContext", payload: source.payload))
            guard synced.revision == context.revision else { throw APIError.unconfirmedCommand }
            // 再取核心权威节点，审阅页的入参不能替换模型执行结果。
            let authoritative = try await nativeBoardNodes(board)
            guard nodes == authoritative else { throw APIError.unconfirmedCommand }
            let reverse = Dictionary(uniqueKeysWithValues: context.photoIDs.map { ($0.value, $0.key) })
            let top = local.canvas?.nodes.map(\.y).max() ?? -200
            var accepted: [LocalCanvasNode] = []
            for node in authoritative {
                guard node.visible, node.rotation == 0 else { throw APIError.permissionDenied }
                let newID = UUID().uuidString.lowercased()
                if node.nodeType == "photo", let asset = node.assetId, let id = reverse[asset] {
                    accepted.append(.init(id: newID, kind: .photo, photoID: id, x: node.x, y: node.y + top + 500,
                                          width: node.width, height: node.height, zOrder: node.zOrder))
                } else if node.nodeType == "text", let raw = node.contentJson,
                          let value = try? JSONSerialization.jsonObject(with: Data(raw.utf8)) as? [String: Any], let text = value["text"] as? String {
                    accepted.append(.init(id: newID, kind: .note, text: text, x: node.x, y: node.y + top + 500,
                                          width: node.width, height: node.height, zOrder: node.zOrder))
                } else { throw NativeAIError(code: "UNSUPPORTED_NODE", message: "此草案包含手机画布暂不支持的节点，当前只能预览。") }
            }
            guard generation == epoch, currentProject == owner else { throw CancellationError() }
            try await library.applyAICanvas(projectID: projectID, expectedUpdatedAt: local.updatedAt, applicationID: applicationID, nodes: accepted)
            if generation == epoch {
                await localLibrary?.canvasModel(projectID: projectID).recordAIAddition(accepted.map(\.id))
            }
            if generation == epoch, currentProject == owner {
                await localLibrary?.reload(); nativeReviewReceipt = "已将草案添加到本机画布。"; busy = false; await refreshNativeTasks()
            }
            return true
        } catch { if generation == epoch, currentProject == owner { notice = message(error) }; return false }
    }

    func nativePhotoImage(_ coreID: String) async throws -> UIImage {
        let owner = currentProject
        guard let id = nativeContext?.photoIDs.first(where: { $0.value == coreID })?.key else { throw APIError.permissionDenied }
        let result: UIImage
        if case .local(let projectID) = owner, let library = localLibrary {
            result = try await library.image(projectID: projectID, photoID: id)
        } else { result = try await image(photoID: id) }
        guard currentProject == owner else { throw CancellationError() }
        return result
    }
}
