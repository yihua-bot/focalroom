import Foundation
import CryptoKit

// 显式审阅后的云端提案写入记录；重试复用相同命令和原始版本，不生成第二份提案。
public actor NativeAIReviewStore {
    public struct Record: Codable, Sendable {
        public let scope: AccountScope
        public let applicationID: String
        public let create: ProposalCreateCommand
        public let submit: ProposalSubmitCommand
        public var completed: Bool
    }
    private let root: URL
    private let scope: AccountScope
    public init(root: URL, scope: AccountScope) { self.root = root.appendingPathComponent(scope.key); self.scope = scope }

    public func record(applicationID: String) throws -> Record? {
        let path = file(applicationID)
        guard FileManager.default.fileExists(atPath: path.path) else { return nil }
        let size = try path.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        guard size <= 1024 * 1024 else { throw APIError.oversizedResponse }
        let value = try JSONDecoder().decode(Record.self, from: Data(contentsOf: path))
        guard value.scope == scope, value.applicationID == applicationID else { throw APIError.invalidResponse }
        return value
    }
    public func prepare(applicationID: String, snapshot: ProjectSnapshot, title: String,
                        operations: [ProposalCreateCommand.DraftOperation]) throws -> Record {
        if let saved = try record(applicationID: applicationID) { return saved }
        let create = try ProposalCreateCommand(snapshot: snapshot, title: title, operations: operations)
        let value = Record(scope: scope, applicationID: applicationID, create: create,
                           submit: ProposalSubmitCommand(snapshot: snapshot, proposalID: create.command_id), completed: false)
        try write(value)
        return value
    }
    public func complete(applicationID: String) throws {
        guard var record = try record(applicationID: applicationID) else { throw APIError.unconfirmedCommand }
        record.completed = true
        try write(record)
    }
    private func file(_ id: String) -> URL {
        root.appendingPathComponent(SHA256.hash(data: Data(id.utf8)).map { String(format: "%02x", $0) }.joined() + ".json")
    }
    private func write(_ record: Record) throws {
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        var directory = root
        var attributes = URLResourceValues()
        attributes.isExcludedFromBackup = true
        try directory.setResourceValues(attributes)
        let data = try JSONEncoder().encode(record)
        guard data.count <= 1024 * 1024 else { throw APIError.oversizedResponse }
        #if os(iOS)
        try data.write(to: file(record.applicationID), options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
        #else
        try data.write(to: file(record.applicationID), options: .atomic)
        #endif
    }
}
