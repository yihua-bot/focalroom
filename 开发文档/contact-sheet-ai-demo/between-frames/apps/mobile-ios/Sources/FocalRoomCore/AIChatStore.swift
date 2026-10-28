import Foundation

// 对话消息：requestID 关联用户消息与模型回复；unknown 状态保留原编号供回执查询，不重发。
public struct AIChatMessage: Codable, Sendable, Identifiable, Equatable {
    public enum State: String, Codable, Sendable { case sent, unknown, failed }

    public let id: String
    public let role: String
    public let text: String
    public let createdAt: Date
    public let requestID: String?
    public var state: State
    public var failure: String?

    public init(id: String, role: String, text: String, createdAt: Date = Date(),
                requestID: String? = nil, state: State = .sent, failure: String? = nil) {
        self.id = id
        self.role = role
        self.text = text
        self.createdAt = createdAt
        self.requestID = requestID
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
              document.messages.allSatisfy({ ["user", "assistant"].contains($0.role) && !$0.text.isEmpty })
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
