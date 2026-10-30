import Foundation
import CryptoKit

public enum APIError: Error, LocalizedError, Sendable, Equatable {
    case invalidServer, invalidResponse, oversizedResponse, permissionDenied, unconfirmedCommand
    case server(status: Int, code: String)

    public var errorDescription: String? {
        switch self {
        case .invalidServer: return "请输入 HTTPS 服务地址；内部调试仅允许本机回环 HTTP。"
        case .invalidResponse: return "服务返回的内容无法安全读取，请更新应用或重试。"
        case .oversizedResponse: return "内容超过本次读取上限，请在 Mac 查看。"
        case .permissionDenied: return "当前项目权限不允许此操作。"
        case .unconfirmedCommand: return "操作结果尚未确认，请核对后继续。"
        case .server(let status, let code):
            if code == "UNAUTHORIZED" { return "邮箱或密码错误，或登录凭据无效。" }
            if code == "REAUTH_REQUIRED" { return "请重新核对当前密码和账号邮箱。" }
            if code == "ACCOUNT_TASKS_ACTIVE" { return "请先等待或取消正在进行的 AI 任务。" }
            if code == "ACCOUNT_SUBSCRIPTION_ACTIVE" { return "账号仍有有效订阅，请先结束订阅，避免继续扣费。" }
            if code == "ACCOUNT_EXPORT_TOO_LARGE" { return "账号资料超过单次导出上限，请联系支持分批取回。" }
            if status == 401 { return "登录已失效，请重新登录。" }
            if status == 403 { return "项目访问权限已变化。" }
            if code == "REVISION_CONFLICT" { return "项目已更新，请刷新后重新选择。" }
            if status == 429 { return "请求过于频繁，请稍后重试。" }
            return "暂时无法完成请求，请稍后重试。"
        }
    }

    public var revokesSession: Bool {
        if case .server(let status, _) = self { return status == 401 }
        return false
    }

    public var deniesProject: Bool {
        if case .server(let status, _) = self { return status == 403 || status == 404 }
        return false
    }

    public var rejectsCommand: Bool {
        if case .server(let status, _) = self { return [400, 401, 403, 404, 409, 422].contains(status) }
        return false
    }
}

public struct APIConfiguration: Codable, Sendable, Equatable {
    public let origin: URL

    public init(_ address: String, allowLoopbackHTTP: Bool = false) throws {
        guard let value = URLComponents(string: address.trimmingCharacters(in: .whitespacesAndNewlines)),
              let host = value.host, !host.isEmpty, value.user == nil, value.password == nil,
              value.query == nil, value.fragment == nil,
              value.path.isEmpty || value.path == "/",
              value.scheme == "https" || (allowLoopbackHTTP && value.scheme == "http" &&
                  ["127.0.0.1", "localhost", "[::1]", "::1"].contains(host)),
              let url = value.url
        else { throw APIError.invalidServer }
        origin = url
    }

    public var identity: String { origin.absoluteString.trimmingCharacters(in: CharacterSet(charactersIn: "/")) }

    public func endpoint(_ segments: [String], query: [URLQueryItem] = []) throws -> URL {
        var url = origin.appendingPathComponent("api").appendingPathComponent("v2")
        for segment in segments {
            guard !segment.isEmpty, segment != ".", segment != "..", !segment.contains("/"),
                  !segment.contains("\\"), !segment.contains("%")
            else { throw APIError.invalidResponse }
            url.appendPathComponent(segment)
        }
        var components = URLComponents(url: url, resolvingAgainstBaseURL: false)!
        if !query.isEmpty { components.queryItems = query }
        return components.url!
    }

    public func previewURL(_ ticket: String) throws -> URL {
        let prefix = "/api/v2/media/"
        guard ticket.hasPrefix(prefix) else { throw APIError.invalidResponse }
        let token = String(ticket.dropFirst(prefix.count))
        guard token.count == 64, token.allSatisfy({ "0123456789abcdef".contains($0) })
        else { throw APIError.invalidResponse }
        return try endpoint(["media", token])
    }
}

public struct HTTPResponse: Sendable {
    public let data: Data
    public let status: Int
    public let contentType: String?

    public init(data: Data, status: Int, contentType: String? = nil) {
        self.data = data; self.status = status; self.contentType = contentType
    }
}

public protocol HTTPTransport: Sendable {
    func send(_ request: URLRequest, limit: Int) async throws -> HTTPResponse
}

// 不跟随重定向，不携带浏览器 Cookie，也不将授权预览写入系统 URL 缓存。
private final class RejectRedirects: NSObject, URLSessionTaskDelegate, Sendable {
    func urlSession(_ session: URLSession, task: URLSessionTask,
                    willPerformHTTPRedirection response: HTTPURLResponse,
                    newRequest request: URLRequest,
                    completionHandler: @escaping @Sendable (URLRequest?) -> Void) {
        completionHandler(nil)
    }
}

public struct URLSessionTransport: HTTPTransport {
    private let session: URLSession

    public init() {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.httpShouldSetCookies = false
        configuration.httpCookieStorage = nil
        configuration.urlCache = nil
        configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
        configuration.timeoutIntervalForRequest = 25
        configuration.timeoutIntervalForResource = 45
        configuration.httpMaximumConnectionsPerHost = 4
        session = URLSession(configuration: configuration, delegate: RejectRedirects(), delegateQueue: nil)
    }

    public func send(_ request: URLRequest, limit: Int) async throws -> HTTPResponse {
        let (bytes, response) = try await session.bytes(for: request)
        guard let response = response as? HTTPURLResponse else { throw APIError.invalidResponse }
        guard response.expectedContentLength <= limit else { throw APIError.oversizedResponse }
        var data = Data()
        for try await byte in bytes {
            guard data.count < limit else { throw APIError.oversizedResponse }
            data.append(byte)
        }
        return HTTPResponse(data: data, status: response.statusCode,
                            contentType: response.value(forHTTPHeaderField: "Content-Type"))
    }
}

public struct APIClient: Sendable {
    public let configuration: APIConfiguration
    private let transport: any HTTPTransport

    public init(configuration: APIConfiguration, transport: any HTTPTransport = URLSessionTransport()) {
        self.configuration = configuration
        self.transport = transport
    }

    private func call<T: Decodable>(_ type: T.Type, _ path: [String], token: String? = nil,
                                    body: Data? = nil, method: String? = nil, query: [URLQueryItem] = [],
                                    headers: [String: String] = [:]) async throws -> T {
        var request = URLRequest(url: try configuration.endpoint(path, query: query))
        request.httpMethod = method ?? (body == nil ? "GET" : "POST")
        request.httpBody = body
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        if body != nil { request.setValue("application/json", forHTTPHeaderField: "Content-Type") }
        for (name, value) in headers { request.setValue(value, forHTTPHeaderField: name) }
        if let token {
            guard token.count == 64, token.allSatisfy({ "0123456789abcdef".contains($0) })
            else { throw APIError.invalidResponse }
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        let response = try await transport.send(request, limit: 12 * 1024 * 1024)
        try check(response)
        do { return try JSONDecoder().decode(type, from: response.data) }
        catch { throw APIError.invalidResponse }
    }

    private func check(_ response: HTTPResponse) throws {
        guard (200..<300).contains(response.status) else {
            struct Failure: Decodable { struct Detail: Decodable { let code: String }; let error: Detail }
            let code = (try? JSONDecoder().decode(Failure.self, from: response.data).error.code) ?? "UNAVAILABLE"
            throw APIError.server(status: response.status, code: code)
        }
    }

    public func login(email: String, password: String) async throws -> Credentials {
        struct Input: Encodable { let email: String; let password: String }
        let result = try await call(Credentials.self, ["auth", "login"],
                                   body: JSONEncoder().encode(Input(email: email, password: password)))
        guard result.token.count == 64, result.token.allSatisfy({ "0123456789abcdef".contains($0) }),
              !result.user.id.isEmpty else { throw APIError.invalidResponse }
        return result
    }

    /// 请求邮箱验证码：服务端发送 6 位码到邮箱；同邮箱旧码作废。
    public func magicRequest(email: String) async throws {
        struct Input: Encodable { let email: String }
        struct Result: Decodable { let sent: Bool }
        let result = try await call(Result.self, ["auth", "magic", "request"],
                                   body: JSONEncoder().encode(Input(email: email)))
        guard result.sent else { throw APIError.invalidResponse }
    }

    /// 验证码登录：新邮箱自动建号（随机未知密码），返回与密码登录一致的会话。
    public func magicVerify(email: String, code: String) async throws -> Credentials {
        struct Input: Encodable { let email: String; let code: String }
        let result = try await call(Credentials.self, ["auth", "magic", "verify"],
                                   body: JSONEncoder().encode(Input(email: email, code: code)))
        guard result.token.count == 64, result.token.allSatisfy({ "0123456789abcdef".contains($0) }),
              !result.user.id.isEmpty else { throw APIError.invalidResponse }
        return result
    }

    /// 登录态设置或修改密码；验证码新建账号可不带当前密码。
    public func setPassword(token: String, password: String, current: String?) async throws {
        struct Input: Encodable { let password: String; let current_password: String? }
        struct Result: Decodable { let updated: Bool }
        let result = try await call(Result.self, ["auth", "set-password"], token: token,
                                    body: JSONEncoder().encode(Input(password: password, current_password: current)))
        guard result.updated else { throw APIError.invalidResponse }
    }

    public func currentUser(token: String) async throws -> User { try await call(User.self, ["auth", "me"], token: token) }

    public func logout(token: String) async throws {
        struct Result: Decodable { let logged_out: Bool }
        let result = try await call(Result.self, ["auth", "logout"], token: token, body: Data("{}".utf8))
        guard result.logged_out else { throw APIError.invalidResponse }
    }

    public func accountData(token: String, userID: String) async throws -> Data {
        let value = try await call(JSONValue.self, ["account", "data-export"], token: token, body: Data("{}".utf8))
        guard case .object(let object) = value, object["user_id"] == .string(userID), object["version"] == .number(1) else {
            throw APIError.invalidResponse
        }
        let encoder = JSONEncoder(); encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return try encoder.encode(value)
    }

    public func eraseAccount(token: String, password: String, email: String) async throws -> AccountErasureReceipt {
        struct Input: Encodable { let password: String; let confirm_email: String; let understand_retention = true }
        let receipt = try await call(AccountErasureReceipt.self, ["account", "erase"], token: token,
                                     body: JSONEncoder().encode(Input(password: password, confirm_email: email)))
        guard receipt.closed, UUID(uuidString: receipt.receipt_id) != nil, receipt.local_originals_retained else {
            throw APIError.unconfirmedCommand
        }
        return receipt
    }

    public func projects(token: String) async throws -> [ProjectSummary] {
        let projects = try await call([ProjectSummary].self, ["projects"], token: token)
        guard Set(projects.map(\.id)).count == projects.count,
              projects.allSatisfy({ !$0.id.isEmpty && $0.revision >= 0 }) else { throw APIError.invalidResponse }
        return projects
    }

    public func snapshot(projectID: String, credentials: Credentials) async throws -> ProjectSnapshot {
        let value = try await call(ProjectSnapshot.self, ["projects", projectID, "snapshot"], token: credentials.token)
        try value.validate(projectID: projectID, userID: credentials.user.id)
        return value
    }

    public func preview(projectID: String, descriptor: PreviewDescriptor, token: String) async throws -> Data {
        struct Ticket: Decodable { let url: String }
        let ticket = try await call(Ticket.self, ["projects", projectID, "previews", descriptor.photo_id, "ticket"],
                                    token: token, body: Data("{}".utf8))
        // 媒体票据只允许回到当前服务的固定路径，不向返回的任意 URL 发送凭据。
        let request = URLRequest(url: try configuration.previewURL(ticket.url))
        let response = try await transport.send(request, limit: 16 * 1024 * 1024)
        try check(response)
        guard response.contentType?.lowercased().hasPrefix("image/jpeg") == true,
              SHA256.hash(data: response.data).map({ String(format: "%02x", $0) }).joined() == descriptor.content_sha256
        else { throw APIError.invalidResponse }
        return response.data
    }

    public func select(_ command: SelectionCommand, token: String) async throws -> CommandReceipt {
        let result = try await call(CommandReceipt.self, ["projects", command.project_id, "commands"], token: token,
                                    body: JSONEncoder().encode(command))
        try result.validate(for: command)
        return result
    }

    public func boards(projectID: String, token: String) async throws -> [Board] {
        let values = try await call([Board].self, ["boards"], token: token, query: [.init(name: "project", value: projectID)])
        guard values.allSatisfy({ $0.project_id == projectID && $0.revision >= 0 }) else { throw APIError.invalidResponse }
        return values
    }

    public func submitCanvasCommand(boardID: String, command: CanvasMoveCommand, token: String) async throws -> CanvasCommandReceipt {
        let encoder = JSONEncoder()
        encoder.outputFormatting = .sortedKeys
        let receipt = try await call(CanvasCommandReceipt.self, ["boards", boardID, "commands"], token: token,
                                     body: encoder.encode(command))
        try receipt.validate(for: command)
        return receipt
    }

    public func canvasReceipt(boardID: String, command: CanvasMoveCommand, token: String) async throws -> CanvasCommandReceipt? {
        do {
            let result = try await call(CanvasCommandReceipt.self, ["boards", boardID, "commands", command.commandId, "receipt"], token: token)
            try result.validate(for: command)
            return result
        } catch APIError.server(404, _) { return nil }
    }

    public func applyProposal(_ command: ProposalApplyCommand, token: String) async throws -> CommandReceipt {
        let receipt = try await call(CommandReceipt.self, ["projects", command.project_id, "commands"], token: token,
                                     body: JSONEncoder().encode(command))
        // 驳回（空采纳）不改正式修订；采纳推进一版。
        let expected = command.expected_revision + (command.payload.accepted_operation_ids.isEmpty ? 0 : 1)
        guard receipt.ok, receipt.command_id == command.command_id,
              receipt.new_revision == expected, !receipt.event_id.isEmpty else { throw APIError.unconfirmedCommand }
        return receipt
    }

    // 宽松项目命令提交：仅核对 ok/编号/事件，供联调构造提案等非选片命令；产品路径不使用。
    public func submitProjectCommandBody(projectID: String, body: Data, token: String) async throws -> CommandReceipt {
        let receipt = try await call(CommandReceipt.self, ["projects", projectID, "commands"], token: token, body: body)
        guard receipt.ok, !receipt.command_id.isEmpty, !receipt.event_id.isEmpty else { throw APIError.invalidResponse }
        return receipt
    }

    // 提案创建/提交：产品路径。创建为非正式命令（不推进版本）；回执仅核对 ok/事件绑定。
    public func createProposal(_ command: ProposalCreateCommand, token: String) async throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = .sortedKeys
        _ = try await submitProjectCommandBody(projectID: command.project_id,
                                               body: try encoder.encode(command), token: token)
    }

    public func submitProposal(_ command: ProposalSubmitCommand, token: String) async throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = .sortedKeys
        _ = try await submitProjectCommandBody(projectID: command.project_id,
                                               body: try encoder.encode(command), token: token)
    }

    // 评论回复：既有线程追加；body 与幂等键必填，服务端同事务审计与回执。
    public func replyComment(projectID: String, threadID: String, body: String, idempotencyKey: String, token: String) async throws {
        struct Input: Encodable { let body: String; let idempotency_key: String }
        struct Result: Decodable { let id: String }
        let result = try await call(Result.self, ["projects", projectID, "comments", threadID, "replies"],
                                    token: token, body: try JSONEncoder().encode(Input(body: body, idempotency_key: idempotencyKey)))
        guard !result.id.isEmpty else { throw APIError.invalidResponse }
    }

    // 评论处理（解决/重开）：正式命令路径，状态由服务端校验。
    public func resolveComment(projectID: String, threadID: String, resolve: Bool, snapshot: ProjectSnapshot, token: String) async throws -> CommandReceipt {
        struct Envelope: Encodable {
            let version: String
            let command_id: String
            let workspace_id: String
            let project_id: String
            let expected_revision: Int
            let idempotency_key: String
            let kind: String
            let payload: Payload
            struct Payload: Encodable { let thread_id: String; let state: String }
        }
        let id = UUID().uuidString.lowercased()
        let envelope = Envelope(version: "2.0", command_id: id, workspace_id: snapshot.workspace_id,
                                project_id: snapshot.project_id, expected_revision: snapshot.revision,
                                idempotency_key: id, kind: "comment.resolve",
                                payload: .init(thread_id: threadID, state: resolve ? "RESOLVED" : "REOPENED"))
        let encoder = JSONEncoder()
        encoder.outputFormatting = .sortedKeys
        return try await submitProjectCommandBody(projectID: projectID, body: try encoder.encode(envelope), token: token)
    }

    public func nodes(boardID: String, token: String) async throws -> [BoardNode] {
        let values = try await call([BoardNode].self, ["boards", boardID, "nodes"], token: token)
        guard values.allSatisfy({ $0.board_id == boardID && $0.x.isFinite && $0.y.isFinite &&
            $0.width.isFinite && $0.height.isFinite && $0.rotation.isFinite && $0.width > 0 && $0.height > 0 &&
            abs($0.x) <= 1e9 && abs($0.y) <= 1e9 && $0.width <= 10_000 && $0.height <= 10_000 })
        else { throw APIError.invalidResponse }
        return values
    }

    public func agentResults(projectID: String, token: String) async throws -> [AgentResult] {
        struct Results: Decodable { let tasks: [AgentResult] }
        return try await call(Results.self, ["projects", projectID, "agent-tasks"], token: token).tasks
    }

    public func aiAccount(token: String) async throws -> AIAccount {
        let value = try await call(AIAccount.self, ["ai", "account"], token: token)
        guard value.available == value.granted - value.spent - value.reserved,
              value.available >= 0, !value.policy.isEmpty, !value.model.isEmpty else { throw APIError.invalidResponse }
        return value
    }

    // 异步提交：同号重发返回当前任务状态不重复调用模型；结果留存供回执查询恢复。
    // 命令体用 sortedKeys 固定键序——本工具链 JSONEncoder 跨实例键序不稳定，字节序不同会被服务端判为另一请求。
    public func aiSubmit(input: AIConversationInput, requestID: String, token: String, imageConsent: Bool = false) async throws -> AIJob {
        guard UUID(uuidString: requestID) != nil, input.imageCount <= 4,
              input.imageCount == 0 || imageConsent else { throw APIError.permissionDenied }
        let encoder = JSONEncoder()
        encoder.outputFormatting = .sortedKeys
        var headers = ["x-focalroom-ai-policy": "deepseek-flash-v1", "x-focalroom-request-id": requestID]
        if input.imageCount > 0 { headers["x-focalroom-image-consent"] = "granted" }
        let job = try await call(AIJob.self, ["ai", "deepseek-flash-v1", "responses", "async"], token: token,
                                 body: try encoder.encode(input),
                                 headers: headers)
        try job.validate(requestID: requestID)
        return job
    }

    public func aiJob(requestID: String, token: String) async throws -> AIJob {
        guard UUID(uuidString: requestID) != nil else { throw APIError.invalidResponse }
        let job = try await call(AIJob.self, ["ai", "deepseek-flash-v1", "responses", "async", requestID], token: token)
        try job.validate(requestID: requestID)
        return job
    }

    /// 流式事件：start 携带回执编号；delta 为增量文本；done 携带完整回复（含计量）。
    public enum AIStreamEvent: Sendable {
        case start(requestID: String)
        case delta(String)
        case done(AIReply)
        case failure(code: String, message: String)
    }

    /// 流式提交：与 aiSubmit 相同的鉴权/额度/幂等边界；增量经 SSE 转发。
    /// 直接使用 URLSession 字节流（不经 transport 全量缓冲）；服务端不支持时抛错由调用方回退轮询。
    /// 流式提交：与 aiSubmit 相同的鉴权/额度/幂等边界；增量经 SSE 转发，事件经回调实时交付。
    /// 服务端不支持或流中断时抛错，由调用方凭原编号回退提交+轮询（幂等读回执，不重复计费）。
    public func aiStream(input: AIConversationInput, requestID: String, token: String, imageConsent: Bool = false,
                         onEvent: @MainActor @Sendable (AIStreamEvent) -> Void) async throws {
        guard UUID(uuidString: requestID) != nil, input.imageCount <= 4,
              input.imageCount == 0 || imageConsent else { throw APIError.permissionDenied }
        var request = URLRequest(url: configuration.origin.appendingPathComponent("api/v2/ai/deepseek-flash-v1/responses/stream"))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        request.setValue("deepseek-flash-v1", forHTTPHeaderField: "x-focalroom-ai-policy")
        request.setValue(requestID, forHTTPHeaderField: "x-focalroom-request-id")
        if input.imageCount > 0 { request.setValue("granted", forHTTPHeaderField: "x-focalroom-image-consent") }
        let encoder = JSONEncoder(); encoder.outputFormatting = .sortedKeys
        request.httpBody = try encoder.encode(input)
        let (bytes, response) = try await URLSession.shared.bytes(for: request)
        guard let http = response as? HTTPURLResponse else { throw APIError.invalidResponse }
        guard http.statusCode == 200 else {
            struct Failure: Decodable { struct Detail: Decodable { let code: String; let message: String? }; let error: Detail }
            var raw = ""
            for try await line in bytes.lines { raw += line }
            if let data = raw.data(using: .utf8),
               let failure = try? JSONDecoder().decode(Failure.self, from: data) {
                throw APIError.server(status: http.statusCode, code: failure.error.code)
            }
            throw APIError.server(status: http.statusCode, code: "UNAVAILABLE")
        }
        struct Envelope: Decodable {
            let type: String
            let text: String?
            let request_id: String?
            let reply: AIReply?
            let code: String?
            let message: String?
        }
        for try await line in bytes.lines {
            guard line.hasPrefix("data:") else { continue }
            let payload = String(line.dropFirst(5)).trimmingCharacters(in: .whitespaces)
            guard !payload.isEmpty, let data = payload.data(using: .utf8),
                  let envelope = try? JSONDecoder().decode(Envelope.self, from: data) else { continue }
            switch envelope.type {
            case "start":
                if let id = envelope.request_id { await onEvent(.start(requestID: id)) }
            case "delta":
                if let text = envelope.text { await onEvent(.delta(text)) }
            case "done":
                guard let reply = envelope.reply else { throw APIError.invalidResponse }
                try reply.validate()
                await onEvent(.done(reply))
                return
            case "error":
                await onEvent(.failure(code: envelope.code ?? "AI_RESULT_UNKNOWN",
                                 message: envelope.message ?? "模型结果无法确认"))
                return
            default: break
            }
        }
        throw APIError.invalidResponse
    }

    public func agentJobs(projectID: String, token: String) async throws -> [AgentJob] {
        struct Envelope: Decodable { let jobs: [AgentJob] }
        let envelope = try await call(Envelope.self, ["projects", projectID, "agent-jobs"], token: token)
        guard Set(envelope.jobs.map(\.id)).count == envelope.jobs.count,
              envelope.jobs.allSatisfy({ !$0.kind.isEmpty && !$0.status.isEmpty })
        else { throw APIError.invalidResponse }
        return envelope.jobs
    }

    public func projectHistory(projectID: String, token: String) async throws -> [VersionSummary] {
        let values = try await call([VersionSummary].self, ["projects", projectID, "history"], token: token)
        guard Set(values.map(\.revision)).count == values.count,
              values.allSatisfy({ $0.revision >= 0 && !$0.command_id.isEmpty })
        else { throw APIError.invalidResponse }
        return values
    }

    public func versionDiff(projectID: String, from: Int, to: Int, token: String) async throws -> VersionDiff {
        let diff = try await call(VersionDiff.self,
                                  ["projects", projectID, "versions", String(from), "diff", String(to)],
                                  token: token)
        guard diff.project_id == projectID,
              diff.from.revision == from, diff.to.revision == to
        else { throw APIError.invalidResponse }
        return diff
    }

    public func proposalDiff(projectID: String, proposalID: String, token: String) async throws -> ProposalDiff {
        let diff = try await call(ProposalDiff.self,
                                  ["projects", projectID, "proposals", proposalID, "diff"],
                                  token: token)
        guard diff.proposalId == proposalID,
              diff.stale ? diff.sequence == nil : true,
              diff.baseRevision >= 0, diff.currentRevision >= 0
        else { throw APIError.invalidResponse }
        return diff
    }

    // 目录端点无需会话（公开服务能力说明）。
    public func billingCatalog() async throws -> BillingCatalog {
        let catalog = try await call(BillingCatalog.self, ["billing", "catalog"])
        guard Set(catalog.plans.map(\.id)).count == catalog.plans.count,
              catalog.plans.allSatisfy({ !$0.id.isEmpty && !$0.name.isEmpty })
        else { throw APIError.invalidResponse }
        return catalog
    }

    public func workspaces(token: String) async throws -> [WorkspaceSummary] {
        let values = try await call([WorkspaceSummary].self, ["workspaces"], token: token)
        guard Set(values.map(\.id)).count == values.count,
              values.allSatisfy({ !$0.id.isEmpty && !$0.title.isEmpty && !$0.role.isEmpty })
        else { throw APIError.invalidResponse }
        return values
    }

    // 邀请：创建后返回私密链接，由邀请者自行分享；服务端不发送邮件。
    public func createInvitation(workspaceID: String, email: String, role: String, projectID: String?, token: String) async throws -> URL {
        struct Input: Encodable {
            let email: String
            let role: String
            let project_id: String?
        }
        struct Result: Decodable { let id: String; let invite_url: String; let expires_in: Int }
        let result = try await call(Result.self, ["workspaces", workspaceID, "invitations"], token: token,
                                    body: try JSONEncoder().encode(Input(email: email.lowercased(), role: role, project_id: projectID)))
        guard URL(string: result.invite_url)?.scheme?.hasPrefix("http") == true,
              result.expires_in > 0, !result.id.isEmpty else { throw APIError.invalidResponse }
        return URL(string: result.invite_url)!
    }

    public func registerDevice(key: String, name: String, platform: String, token: String) async throws -> DeviceRegistration {
        struct Input: Encodable { let device_key: String; let name: String; let platform: String }
        guard UUID(uuidString: key) != nil else { throw APIError.invalidResponse }
        let result = try await call(DeviceRegistration.self, ["devices"], token: token,
                                    body: JSONEncoder().encode(Input(device_key: key, name: name, platform: platform)))
        guard !result.id.isEmpty else { throw APIError.invalidResponse }
        return result
    }

    public func bindDevice(id: String, token: String) async throws {
        struct Result: Decodable { let bound: Bool }
        let result = try await call(Result.self, ["devices", id, "session"], token: token, body: Data("{}".utf8))
        guard result.bound else { throw APIError.invalidResponse }
    }

    public func devices(token: String) async throws -> [DeviceRecord] {
        let values = try await call([DeviceRecord].self, ["devices"], token: token)
        guard Set(values.map(\.id)).count == values.count,
              values.allSatisfy({ !$0.id.isEmpty && !$0.name.isEmpty && !$0.platform.isEmpty })
        else { throw APIError.invalidResponse }
        return values
    }

    public func revokeDevice(id: String, token: String) async throws {
        struct Result: Decodable { let revoked: Bool }
        let result = try await call(Result.self, ["devices", id], token: token, body: nil, method: "DELETE")
        guard result.revoked else { throw APIError.invalidResponse }
    }

    // 回执查询：404 表示服务端确无此命令，客户端才允许按原编号重发。
    public func commandReceipt(projectID: String, commandID: String, token: String) async throws -> CommandReceipt? {
        struct Envelope: Decodable { let command_id: String; let reply: CommandReceipt }
        do {
            let envelope = try await call(Envelope.self, ["projects", projectID, "commands", commandID], token: token)
            guard envelope.command_id == commandID, envelope.reply.ok else { throw APIError.invalidResponse }
            return envelope.reply
        } catch let error as APIError {
            if case .server(404, "NOT_FOUND") = error { return nil }
            throw error
        }
    }

    public func research(projectID: String, token: String) async throws -> [ResearchEntry] {
        let values = try await call([ResearchEntry].self, ["projects", projectID, "research"], token: token)
        guard Set(values.map(\.id)).count == values.count,
              values.allSatisfy({ $0.isNote || $0.isLink && $0.url != nil }),
              values.allSatisfy({ !$0.title.isEmpty && !$0.created_by_name.isEmpty }),
              values.allSatisfy({ !$0.isNote || $0.body != nil })
        else { throw APIError.invalidResponse }
        return values
    }

    public func createResearch(projectID: String, draft: ResearchDraft, token: String) async throws -> ResearchRecord {
        let record = try await call(ResearchRecord.self, ["projects", projectID, "research"], token: token,
                                    body: JSONEncoder().encode(draft))
        try record.validate(kind: draft.kind)
        return record
    }

    public func updateResearch(projectID: String, referenceID: String, patch: ResearchPatch, token: String) async throws -> ResearchRecord {
        let record = try await call(ResearchRecord.self, ["projects", projectID, "research", referenceID], token: token,
                                    body: JSONEncoder().encode(patch), method: "PATCH")
        try record.validate(kind: nil)
        return record
    }

    public func deleteResearch(projectID: String, referenceID: String, token: String) async throws {
        struct Result: Decodable { let deleted: Bool; let id: String }
        let result = try await call(Result.self, ["projects", projectID, "research", referenceID], token: token,
                                    body: nil, method: "DELETE")
        guard result.deleted, result.id == referenceID else { throw APIError.invalidResponse }
    }

    // 重试必须发送与首次完全相同的清单字节，服务端摘要校验才认可同一迁移。
    public func promote(manifestJSON: Data, token: String, privateSync: Bool = false) async throws -> PromotionStart {
        let result = try await call(PromotionStart.self, [privateSync ? "private-promotions" : "promotions"], token: token, body: manifestJSON)
        let manifest = try JSONDecoder().decode(PromotionManifest.self, from: manifestJSON)
        try result.validate(manifest: manifest)
        return result
    }

    public func syncState(projectID: String, token: String) async throws -> ProjectSyncState {
        let value = try await call(ProjectSyncState.self, ["projects", projectID, "sync-state"], token: token)
        guard value.project_id == projectID, value.version >= 0 else { throw APIError.invalidResponse }
        return value
    }

    public func changeSyncState(projectID: String, command: SyncModeCommand, token: String) async throws -> ProjectSyncState {
        let encoder = JSONEncoder(); encoder.outputFormatting = .sortedKeys
        let result = try await call(ProjectSyncState.self, ["projects", projectID, "sync-state"], token: token, body: encoder.encode(command))
        guard result.project_id == projectID, result.mode == command.mode, result.version == command.expected_version + 1,
              result.authority == "cloud" else { throw APIError.unconfirmedCommand }
        return result
    }

    public func uploadPreview(projectID: String, photoID: String, jpeg: Data, token: String) async throws -> PreviewUploadReceipt {
        guard jpeg.count <= 8 * 1024 * 1024 else { throw APIError.oversizedResponse }
        var request = URLRequest(url: try configuration.endpoint(["projects", projectID, "previews", photoID]))
        request.httpMethod = "PUT"
        request.httpBody = jpeg
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        request.setValue("image/jpeg", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        let response = try await transport.send(request, limit: 1024 * 1024)
        try check(response)
        do {
            let receipt = try JSONDecoder().decode(PreviewUploadReceipt.self, from: response.data)
            try receipt.validate(photoID: photoID)
            return receipt
        } catch let error as APIError { throw error }
        catch { throw APIError.invalidResponse }
    }

    public struct FinalizeInput: Encodable {
        public let manifest_digest: String
        public init(manifestDigest: String) { self.manifest_digest = manifestDigest }
    }

    public func finalizePromotion(projectID: String, manifestDigest: String, token: String) async throws -> PromotionFinalized {
        let result = try await call(PromotionFinalized.self, ["projects", projectID, "finalize"], token: token,
                                    body: JSONEncoder().encode(FinalizeInput(manifestDigest: manifestDigest)))
        guard result.project_id == projectID, result.authority == "CLOUD",
              result.manifest_digest == manifestDigest else { throw APIError.invalidResponse }
        return result
    }

    public func abortPromotion(projectID: String, manifestDigest: String, token: String) async throws {
        struct Result: Decodable { let aborted: Bool; let manifest_digest: String }
        let result = try await call(Result.self, ["projects", projectID, "abort"], token: token,
                                    body: JSONEncoder().encode(FinalizeInput(manifestDigest: manifestDigest)))
        guard result.aborted, result.manifest_digest == manifestDigest else { throw APIError.invalidResponse }
    }

    public func projectEvents(projectID: String, after: Int, token: String) async throws -> ProjectEventsPage {
        let page = try await call(ProjectEventsPage.self, ["projects", projectID, "events"], token: token,
                                  query: [.init(name: "after", value: String(after)),
                                          .init(name: "limit", value: "100")])
        guard page.events.allSatisfy({ $0.id > after && !$0.kind.isEmpty && $0.revision >= 0 && !$0.actor_id.isEmpty }),
              page.events.map(\.id) == page.events.map(\.id).sorted()
        else { throw APIError.invalidResponse }
        if let next = page.nextCursor, page.events.isEmpty || next != page.events.last?.id { throw APIError.invalidResponse }
        return page
    }
}
