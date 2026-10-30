import Foundation
import CryptoKit

public struct AccountScope: Codable, Sendable, Equatable {
    public let server: String
    public let userID: String

    public init(configuration: APIConfiguration, userID: String) {
        server = configuration.identity; self.userID = userID
    }

    // 离线访问按已登记的 server 身份字符串重建同一 scope key；identity 必须来自记录时的原值。
    public init(server: String, userID: String) {
        self.server = server; self.userID = userID
    }

    public var key: String {
        SHA256.hash(data: Data("\(server)\n\(userID)".utf8)).map { String(format: "%02x", $0) }.joined()
    }
}

public struct CachedProjects: Codable, Sendable {
    public let projects: [ProjectSummary]
    public let savedAt: Date
}

public struct CachedSnapshot: Codable, Sendable {
    public let snapshot: ProjectSnapshot
    public let savedAt: Date
}

public struct PendingSelection: Codable, Sendable, Equatable {
    public enum State: String, Codable, Sendable { case queued, unknown, rejected }
    public let command: SelectionCommand
    public var state: State
    public let createdAt: Date

    public init(command: SelectionCommand) {
        self.command = command; state = .queued; createdAt = Date()
    }
}

public struct PendingCanvas: Codable, Sendable, Equatable {
    public let boardID: String
    public let command: CanvasMoveCommand
    public let createdAt: Date
}

// 离线中心概览：有缓存快照或待提交意图的项目，附最近缓存时间与待提交状态。
public struct CachedProjectOverview: Codable, Sendable, Identifiable, Equatable {
    public let projectID: String
    public let title: String
    public let revision: Int
    public let savedAt: Date
    public let pending: PendingSelection?
    public let pendingCanvasMoves: Int

    public var id: String { projectID }
}

// 每个账号和服务各自一个有界文件；任何落盘失败都阻止发送正式命令。
public actor ScopedStore {
    private struct Document: Codable {
        let version: Int
        let scope: AccountScope
        var projects: CachedProjects?
        var snapshots: [String: CachedSnapshot]
        var pending: [String: PendingSelection]
        var canvas: [String: PendingCanvas]?
    }

    private let root: URL
    private let scope: AccountScope
    private var file: URL { root.appendingPathComponent(scope.key).appendingPathExtension("json") }

    public init(root: URL, scope: AccountScope) {
        self.root = root; self.scope = scope
    }

    public func pendingCanvas(boardID: String) throws -> PendingCanvas? { try read().canvas?[boardID] }

    public func prepareCanvas(boardID: String, command: CanvasMoveCommand) throws {
        try command.validate()
        var document = try read()
        guard UUID(uuidString: boardID) != nil, (document.canvas ?? [:]).count < 100,
              document.canvas?[boardID] == nil else { throw APIError.unconfirmedCommand }
        var pending = document.canvas ?? [:]
        pending[boardID] = PendingCanvas(boardID: boardID, command: command, createdAt: Date())
        document.canvas = pending
        try write(document)
    }

    public func acknowledgeCanvas(boardID: String, receipt: CanvasCommandReceipt) throws {
        var document = try read()
        guard let pending = document.canvas?[boardID] else { throw APIError.unconfirmedCommand }
        try receipt.validate(for: pending.command)
        document.canvas?.removeValue(forKey: boardID)
        try write(document)
    }

    // 仅在服务端明确拒绝版本后清除；超时、未知或回执不匹配保留原命令。
    public func rejectCanvas(boardID: String, commandID: String) throws {
        var document = try read()
        guard document.canvas?[boardID]?.command.commandId == commandID else { throw APIError.unconfirmedCommand }
        document.canvas?.removeValue(forKey: boardID)
        try write(document)
    }

    private func read() throws -> Document {
        guard FileManager.default.fileExists(atPath: file.path) else {
            return Document(version: 1, scope: scope, snapshots: [:], pending: [:])
        }
        let size = try file.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        guard size <= 24 * 1024 * 1024 else { throw APIError.oversizedResponse }
        let document = try JSONDecoder().decode(Document.self, from: Data(contentsOf: file))
        guard document.version == 1, document.scope == scope else { throw APIError.invalidResponse }
        guard (document.canvas ?? [:]).count <= 100 else { throw APIError.invalidResponse }
        for (id, value) in document.canvas ?? [:] {
            guard id == value.boardID, UUID(uuidString: id) != nil else { throw APIError.invalidResponse }
            try value.command.validate()
        }
        return document
    }

    private func write(_ document: Document) throws {
        let bytes = try JSONEncoder().encode(document)
        guard bytes.count <= 24 * 1024 * 1024 else { throw APIError.oversizedResponse }
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        #if os(iOS)
        try bytes.write(to: file, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
        #else
        try bytes.write(to: file, options: .atomic)
        #endif
        var url = file
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        try url.setResourceValues(values)
    }

    public func projects() throws -> CachedProjects? { try read().projects }
    public func snapshot(projectID: String) throws -> CachedSnapshot? { try read().snapshots[projectID] }
    public func pending(projectID: String) throws -> PendingSelection? { try read().pending[projectID] }

    public func overview() throws -> [CachedProjectOverview] {
        let document = try read()
        let titles = Dictionary((document.projects?.projects ?? []).map { ($0.id, $0.title) }, uniquingKeysWith: { first, _ in first })
        // 画布待提交按命令内 projectId 归属到项目计数。
        let canvasCounts = Dictionary(grouping: (document.canvas ?? [:]).values, by: { $0.command.projectId })
            .mapValues(\.count)
        let cachedRows = document.snapshots.map { id, value in
            CachedProjectOverview(projectID: id, title: value.snapshot.title, revision: value.snapshot.revision,
                                  savedAt: value.savedAt, pending: document.pending[id],
                                  pendingCanvasMoves: canvasCounts[id] ?? 0)
        }
        let pendingOnly = document.pending
            .filter { document.snapshots[$0.key] == nil }
            .map { id, value in
                CachedProjectOverview(projectID: id, title: titles[id] ?? "未知项目", revision: -1,
                                      savedAt: value.createdAt, pending: value, pendingCanvasMoves: canvasCounts[id] ?? 0)
            }
        let canvasOnly = canvasCounts.keys
            .filter { document.snapshots[$0] == nil && document.pending[$0] == nil }
            .map { id in
                CachedProjectOverview(projectID: id, title: titles[id] ?? "未知项目", revision: -1,
                                      savedAt: Date.distantPast, pending: nil, pendingCanvasMoves: canvasCounts[id] ?? 0)
            }
        return (cachedRows + pendingOnly + canvasOnly).sorted { $0.savedAt > $1.savedAt }
    }

    public func saveProjects(_ projects: [ProjectSummary]) throws {
        var document = try read()
        document.projects = CachedProjects(projects: projects, savedAt: Date())
        let ids = Set(projects.map(\.id))
        document.snapshots = document.snapshots.filter { ids.contains($0.key) }
        document.pending = document.pending.filter { ids.contains($0.key) }
        try write(document)
    }

    public func saveSnapshot(_ snapshot: ProjectSnapshot) throws {
        try snapshot.validate(projectID: snapshot.project_id, userID: scope.userID)
        var document = try read()
        // 缓存最多 12 个最近项目；待确认命令单独保留，不随快照淘汰。
        document.snapshots[snapshot.project_id] = CachedSnapshot(snapshot: snapshot, savedAt: Date())
        if document.snapshots.count > 12,
           let oldest = document.snapshots.filter({ $0.key != snapshot.project_id }).min(by: { $0.value.savedAt < $1.value.savedAt }) {
            document.snapshots.removeValue(forKey: oldest.key)
        }
        try write(document)
    }

    public func savePending(_ value: PendingSelection) throws {
        var document = try read()
        if let existing = document.pending[value.command.project_id], existing.command != value.command {
            throw APIError.unconfirmedCommand
        }
        document.pending[value.command.project_id] = value
        try write(document)
    }

    public func acknowledge(_ receipt: CommandReceipt, command: SelectionCommand) throws {
        try receipt.validate(for: command)
        var document = try read()
        guard document.pending[command.project_id]?.command == command else { throw APIError.unconfirmedCommand }
        document.pending.removeValue(forKey: command.project_id)
        // 旧缓存已不能代表已提交版本，重新获取权威快照后才显示正式结果。
        document.snapshots.removeValue(forKey: command.project_id)
        try write(document)
    }

    public func discardRejected(projectID: String) throws {
        var document = try read()
        guard document.pending[projectID]?.state == .rejected else { throw APIError.unconfirmedCommand }
        document.pending.removeValue(forKey: projectID)
        try write(document)
    }

    public func revokeProject(_ projectID: String) throws {
        var document = try read()
        if let cached = document.projects {
            document.projects = CachedProjects(projects: cached.projects.filter { $0.id != projectID }, savedAt: cached.savedAt)
        }
        document.snapshots.removeValue(forKey: projectID)
        document.pending.removeValue(forKey: projectID)
        document.canvas = document.canvas?.filter { $0.value.command.projectId != projectID }
        try write(document)
    }

    public func clear() throws {
        if FileManager.default.fileExists(atPath: file.path) { try FileManager.default.removeItem(at: file) }
    }
}
