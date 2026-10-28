import Foundation
import SwiftUI
import Observation
import ImageIO
import UIKit
import FocalRoomCore

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
    private(set) var credentials: Credentials?
    private(set) var projects: [ProjectSummary] = []
    private(set) var snapshot: ProjectSnapshot?
    private(set) var selectedProjectID: String?
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
    private(set) var canvasSending = false
    private(set) var pendingBoards: Set<String> = []
    var notice: String?
    var address = "https://team.focroom.cc"

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
            await reloadProjects()
            await registerCurrentDevice()
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

    private func activate(_ value: Credentials, configuration: APIConfiguration) {
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
        localLibrary = libraryModel
        let migrationModel = LocalMigrationModel(library: library, api: APIClient(configuration: configuration), credentials: value)
        migrationModel.onRecordChanged = { [weak libraryModel] in await libraryModel?.reload() }
        migration = migrationModel
    }

    func refreshOnResume() async {
        guard !starting, !busy, !loadingProject, credentials != nil else { return }
        await reloadProjects()
        if selectedProjectID != nil { await syncSelectedProject() }
        await localLibrary?.reload()
        await registerCurrentDevice()
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

    func openLocalContext() {
        guard !busy else { return }
        resetProject()
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
        guard projects.contains(where: { $0.id == id && $0.state == .SHARED }) else { return }
        resetProject()
        selectedProjectID = id
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
        guard let chat else { return }
        chatMessages = (try? await chat.messages()) ?? []
    }

    func prepareVision(text: String, photoIDs: Set<String>) async throws -> PreparedVision {
        guard let api, let credentials, let id = selectedProjectID, !offline, !aiSending,
              !photoIDs.isEmpty, photoIDs.count <= 4 else { throw APIError.permissionDenied }
        let generation = epoch
        visionDraftID = nil
        let snapshot = try await projectRead(id) { try await api.snapshot(projectID: id, credentials: credentials) }
        var photos: [PreparedVision.Photo] = []
        for photoID in photoIDs.sorted() {
            guard let photo = snapshot.snapshot.photos.first(where: { $0.id == photoID }),
                  let descriptor = snapshot.previews.first(where: { $0.photo_id == photoID }) else { throw APIError.permissionDenied }
            let bytes = try await api.preview(projectID: id, descriptor: descriptor, token: credentials.token)
            let preview = try await Task.detached(priority: .userInitiated) { try VisionPreview.make(jpeg: bytes) }.value
            guard generation == epoch, id == selectedProjectID else { throw CancellationError() }
            photos.append(.init(descriptor: descriptor, filename: photo.filename, preview: preview))
        }
        let draft = PreparedVision(id: UUID(), generation: generation, accountID: credentials.user.id,
                                   projectID: id, text: text, createdAt: Date(), photos: photos)
        visionDraftID = draft.id
        return draft
    }

    func cancelVision() { visionDraftID = nil }

    func aiSend(text: String, vision: PreparedVision? = nil, onVisionConsumed: () -> Void = {}) async {
        guard !aiSending, let api, let credentials, let chat else { return }
        guard canSendAI else {
            notice = "请先确认 AI 服务已开放、邮箱已验证且账户额度可用，再发送消息。"
            return
        }
        let generation = epoch
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed.count <= 8000 else {
            notice = trimmed.isEmpty ? "请输入消息内容。" : "消息过长，请分段发送。"
            return
        }
        aiSending = true
        defer { if generation == epoch { aiSending = false } }
        if let vision {
            guard vision.id == visionDraftID, vision.generation == generation,
                  vision.accountID == credentials.user.id, vision.projectID == selectedProjectID,
                  vision.text == text, Date().timeIntervalSince(vision.createdAt) < 300 else {
                notice = "照片授权已失效，请重新选择照片并确认。"; return
            }
            // 授权消费一次；点击确认后重新核实项目权限与照片身份，失败不发送。
            visionDraftID = nil
            onVisionConsumed()
            do {
                let current = try await projectRead(vision.projectID) {
                    try await api.snapshot(projectID: vision.projectID, credentials: credentials)
                }
                guard vision.photos.allSatisfy({ photo in
                    current.previews.contains { $0.photo_id == photo.descriptor.photo_id && $0.content_sha256 == photo.descriptor.content_sha256 }
                }) else { throw APIError.permissionDenied }
            } catch { if generation == epoch { notice = message(error) }; return }
        }
        let requestID = UUID().uuidString.lowercased()
        // 先按待确认持久化：响应丢失时凭原编号查询回执，不重复发送、不重复计费。
        let storedText = trimmed + (vision.map { "\n（本次授权观察：" + $0.photos.map(\.filename).joined(separator: "、") + "）" } ?? "")
        let userMessage = AIChatMessage(id: UUID().uuidString.lowercased(), role: "user", text: storedText,
                                        requestID: requestID, state: .unknown)
        do { try await chat.append(userMessage) }
        catch { notice = "无法保存消息，请重试。"; return }
        guard generation == epoch, vision == nil || vision?.projectID == selectedProjectID else { return }
        chatMessages.append(userMessage)
        let history = chatMessages.filter { $0.state == .sent || $0.id == userMessage.id }.suffix(20).map { ($0.role, $0.text) }
        do {
            _ = try await api.aiSubmit(input: AIConversationInput(history: Array(history), images: vision?.photos.map(\.preview) ?? []),
                                       requestID: requestID, token: credentials.token, imageConsent: vision != nil)
            guard generation == epoch else { return }
            let job = try await pollAIJob(requestID: requestID, api: api, token: credentials.token, generation: generation)
            guard generation == epoch else { return }
            if job.state == "COMPLETED", let result = job.result {
                try? await replaceMessage(userMessage, state: .sent, failure: nil)
                try? await chat.append(AIChatMessage(id: UUID().uuidString.lowercased(), role: "assistant",
                                                     text: result.text.isEmpty ? "（模型未返回文本）" : result.text,
                                                     requestID: requestID, state: .sent))
                chatMessages = (try? await chat.messages()) ?? chatMessages
            } else if job.state == "REJECTED" {
                try? await replaceMessage(userMessage, state: .failed, failure: "模型服务拒绝了请求，额度已释放。")
                chatMessages = (try? await chat.messages()) ?? chatMessages
                notice = "模型服务拒绝了请求，额度已释放。"
            } else {
                notice = "结果尚未确认；可稍后用“查询待确认结果”核对，不会重复计费。"
            }
            await loadAIAccount()
        } catch {
            // 网络中断等：消息保留待确认状态，凭原编号可恢复。
            if generation == epoch { notice = message(error) }
        }
    }

    func aiResolvePending() async {
        guard let api, let credentials, let chat else { return }
        let generation = epoch
        var changed = false
        for message in chatMessages where message.state == .unknown && message.role == "user" {
            guard let requestID = message.requestID else { continue }
            do {
                let job = try await api.aiJob(requestID: requestID, token: credentials.token)
                guard generation == epoch else { return }
                if job.state == "COMPLETED", let result = job.result {
                    try? await replaceMessage(message, state: .sent, failure: nil)
                    try? await chat.append(AIChatMessage(id: UUID().uuidString.lowercased(), role: "assistant",
                                                         text: result.text, requestID: requestID, state: .sent))
                    changed = true
                } else if job.state == "REJECTED" {
                    try? await replaceMessage(message, state: .failed, failure: "模型服务拒绝了请求，额度已释放。")
                    changed = true
                }
            } catch { break }
        }
        if changed, generation == epoch {
            chatMessages = (try? await chat.messages()) ?? chatMessages
            await loadAIAccount()
        }
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

    private func pollAIJob(requestID: String, api: APIClient, token: String, generation: UUID) async throws -> AIJob {
        var job = try await api.aiJob(requestID: requestID, token: token)
        var attempts = 0
        while job.state == "PENDING" && attempts < 120 {
            try await Task.sleep(nanoseconds: 500_000_000)
            guard generation == epoch else { throw CancellationError() }
            job = try await api.aiJob(requestID: requestID, token: token)
            attempts += 1
        }
        return job
    }

    private func replaceMessage(_ original: AIChatMessage, state: AIChatMessage.State, failure: String?) async throws {
        guard let chat else { return }
        var updated = original
        updated.state = state
        updated.failure = failure
        try await chat.replace(updated)
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
        projectRequest = UUID()
        selectedProjectID = nil
        snapshot = nil
        pending = nil
        cachedAt = nil
        loadingProject = false
    }

    private func clearMemory() {
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
