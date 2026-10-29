import Foundation

// 对话消息：requestID 关联用户消息与模型回复；unknown 状态保留原编号供回执查询，不重发。
public struct AIChatMessage: Codable, Sendable, Identifiable, Equatable {
    public enum State: String, Codable, Sendable { case sent, unknown, failed }

    public let id: String
    public let role: String
    public let text: String
    public let createdAt: Date
    public let requestID: String?
    public let localProjectID: String?
    public var state: State
    public var failure: String?

    public init(id: String, role: String, text: String, createdAt: Date = Date(),
                requestID: String? = nil, state: State = .sent, failure: String? = nil, localProjectID: String? = nil) {
        self.id = id
        self.role = role
        self.text = text
        self.createdAt = createdAt
        self.requestID = requestID
        self.localProjectID = localProjectID
        self.state = state
        self.failure = failure
    }
}

// 每账户一个有界对话文件；损坏或与账户不符时拒绝写入路径，不静默重建。
public actor AIChatStore {
    private struct Document: Codable {
        let version: Int
        let scope: AccountScope
        var messages: [AIChatMessage]
    }

    public static let messageLimit = 100
    private static let sizeLimit = 1024 * 1024

    private let file: URL
    private let scope: AccountScope

    public init(root: URL, scope: AccountScope) {
        file = root.appendingPathComponent(scope.key + "-ai-chat.json")
        self.scope = scope
    }

    public func messages() throws -> [AIChatMessage] {
        try read().messages
    }

    // 旧消息没有本机项目归属，继续留在原账户对话中，不倒填到任意本机项目。
    public func messages(localProjectID: String?) throws -> [AIChatMessage] {
        try read().messages.filter { $0.localProjectID == localProjectID }
    }

    public func append(_ message: AIChatMessage) throws {
        var document = try read()
        document.messages.append(message)
        if document.messages.count > Self.messageLimit {
            document.messages.removeFirst(document.messages.count - Self.messageLimit)
        }
        try write(document)
    }

    public func replace(_ message: AIChatMessage) throws {
        var document = try read()
        guard let index = document.messages.firstIndex(where: { $0.id == message.id }) else {
            throw APIError.unconfirmedCommand
        }
        document.messages[index] = message
        try write(document)
    }

    // 发送轮询与切换项目后的恢复可能同时取得同一回执；一次落盘，回复不重复追加。
    public func complete(_ original: AIChatMessage, text: String) throws {
        var document = try read()
        guard original.role == "user", let requestID = original.requestID, !text.isEmpty,
              let index = document.messages.firstIndex(where: { $0.id == original.id }),
              document.messages[index].requestID == requestID,
              document.messages[index].localProjectID == original.localProjectID else { throw APIError.unconfirmedCommand }
        document.messages[index].state = .sent
        document.messages[index].failure = nil
        if !document.messages.contains(where: { $0.role == "assistant" && $0.requestID == requestID }) {
            document.messages.append(.init(id: UUID().uuidString.lowercased(), role: "assistant", text: text,
                                            requestID: requestID, localProjectID: original.localProjectID))
        }
        if document.messages.count > Self.messageLimit {
            document.messages.removeFirst(document.messages.count - Self.messageLimit)
        }
        try write(document)
    }

    public func clear() throws {
        if FileManager.default.fileExists(atPath: file.path) {
            try FileManager.default.removeItem(at: file)
        }
    }

    private func read() throws -> Document {
        guard FileManager.default.fileExists(atPath: file.path) else {
            return Document(version: 1, scope: scope, messages: [])
        }
        let size = try file.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        guard size <= Self.sizeLimit else { throw APIError.oversizedResponse }
        let document: Document
        do { document = try JSONDecoder().decode(Document.self, from: Data(contentsOf: file)) }
        catch { throw APIError.invalidResponse }
        guard document.version == 1, document.scope == scope,
              document.messages.count <= Self.messageLimit,
              Set(document.messages.map(\.id)).count == document.messages.count,
              document.messages.allSatisfy({ ["user", "assistant"].contains($0.role) && !$0.text.isEmpty &&
                  ($0.localProjectID == nil || UUID(uuidString: $0.localProjectID!) != nil) })
        else { throw APIError.invalidResponse }
        return document
    }

    private func write(_ document: Document) throws {
        let bytes = try JSONEncoder().encode(document)
        guard bytes.count <= Self.sizeLimit else { throw APIError.oversizedResponse }
        try FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        #if os(iOS)
        try bytes.write(to: file, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
        #else
        try bytes.write(to: file, options: .atomic)
        #endif
    }
}
