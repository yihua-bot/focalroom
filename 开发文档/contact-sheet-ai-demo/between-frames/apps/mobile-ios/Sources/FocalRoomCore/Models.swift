import Foundation

public struct User: Codable, Sendable, Equatable {
    public let id: String
    public let email: String
    public let name: String
}

public struct Credentials: Codable, Sendable {
    public let token: String
    public let csrf: String
    public let user: User
}

public enum ProjectState: String, Codable, Sendable { case STAGING, SHARED }
public enum Selection: String, Codable, Sendable, CaseIterable {
    case UNDECIDED, KEEP, REJECT

    public var title: String {
        switch self {
        case .UNDECIDED: "待选"
        case .KEEP: "保留"
        case .REJECT: "排除"
        }
    }
}

public struct ProjectSummary: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let workspace_id: String
    public let title: String
    public let revision: Int
    public let state: ProjectState
    public var sync_mode: ProjectSyncState.Mode? = nil
}

public struct ProjectSyncState: Codable, Sendable, Equatable {
    public enum Mode: String, Codable, Sendable { case SYNC_ENROLLING, PRIVATE_SYNCED, SHARED, ENROLL_FAILED }
    public let project_id: String
    public let mode: Mode
    public let version: Int
    public let authority: String
}

public struct SyncModeCommand: Codable, Sendable {
    public let command_id: String
    public let expected_version: Int
    public let mode: ProjectSyncState.Mode
    public init(state: ProjectSyncState, shared: Bool) throws {
        guard state.authority == "cloud", state.version >= 0, state.version < Int.max,
              [.PRIVATE_SYNCED, .SHARED].contains(state.mode) else { throw APIError.permissionDenied }
        command_id = UUID().uuidString.lowercased()
        expected_version = state.version
        mode = shared ? .SHARED : .PRIVATE_SYNCED
    }
}

public struct AccountErasureReceipt: Codable, Sendable {
    public let closed: Bool
    public let receipt_id: String
    public let erased_private_projects: Int
    public let retained: [String]
    public let local_originals_retained: Bool
}

public struct Photo: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let filename: String
    public let source_sha256: String
    public let selection: Selection
}

public struct SequenceItem: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let photo_id: String
}

public struct PhotoSequence: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let title: String
    public let items: [SequenceItem]
}

public struct SharedSnapshot: Codable, Sendable, Equatable {
    public let photos: [Photo]
    public let sequences: [PhotoSequence]
}

public struct Principal: Codable, Sendable {
    public let id: String
    public let role: String
    public let can_approve: Bool
    public let can_edit_formal: Bool

    public var permitsSelection: Bool {
        role == "OWNER" || role == "ADMIN" || (role == "EDITOR" && can_edit_formal)
    }

    // 与服务端 principal.can_propose 等价：观察者不能修改画布草稿或批注。
    public var canPropose: Bool { role != "VIEWER" }
}

public struct PreviewDescriptor: Codable, Sendable {
    public let photo_id: String
    public let content_sha256: String
    public let width: Int
    public let height: Int
}

public struct Comment: Codable, Sendable, Identifiable {
    public struct Anchor: Codable, Sendable {
        public let type: String
        public let photo_id: String?
        public let x: Double?
        public let y: Double?
    }

    public let id: String
    public let author_id: String
    public let reviewer: String
    public let body: String
    public let anchor: Anchor
    public let parent_id: String?
    public let state: String
}

public struct ProjectSnapshot: Codable, Sendable {
    public let project_id: String
    public let workspace_id: String
    public let title: String
    public let state: ProjectState
    public let revision: Int
    public let cursor: String
    public let snapshot: SharedSnapshot
    public let principal: Principal
    public let previews: [PreviewDescriptor]
    public let comments: [Comment]
    // 服务端始终返回 proposals；可选解码保持旧版本缓存可读，默认 nil 兼容既有构造点。
    public var proposals: [Proposal]? = nil

    // 未知选片枚举由 Codable 拒绝；不忽略新状态后继续正式写入。
    public func validate(projectID: String, userID: String) throws {
        let photos = snapshot.photos
        let ids = Set(photos.map(\.id))
        let items = snapshot.sequences.flatMap(\.items)
        guard project_id == projectID, principal.id == userID,
              revision >= 0, !workspace_id.isEmpty,
              !photos.isEmpty, photos.count <= 10_000, ids.count == photos.count,
              Set(snapshot.sequences.map(\.id)).count == snapshot.sequences.count,
              !snapshot.sequences.isEmpty, snapshot.sequences.count <= 20,
              Set(items.map(\.id)).count == items.count,
              snapshot.sequences.allSatisfy({ sequence in
                  sequence.items.count <= 10_000 &&
                  Set(sequence.items.map(\.photo_id)).count == sequence.items.count &&
                  sequence.items.allSatisfy { ids.contains($0.photo_id) }
              }),
              photos.allSatisfy({ !$0.id.isEmpty && Self.isHash($0.source_sha256) }),
              Set(previews.map(\.photo_id)).count == previews.count,
              previews.allSatisfy({ ids.contains($0.photo_id) && Self.isHash($0.content_sha256) &&
                  $0.width > 0 && $0.height > 0 && $0.width <= 2048 && $0.height <= 2048 })
        else { throw APIError.invalidResponse }
        if let value = proposals {
            let photoIDs = Set(photos.map(\.id))
            guard value.allSatisfy({ !$0.id.isEmpty && !$0.title.isEmpty && !$0.author.isEmpty &&
                  ["DRAFT", "SUBMITTED", "APPLIED", "REJECTED"].contains($0.state) && $0.base_revision >= 0 && $0.revision >= 0 }),
                  Set(value.map(\.id)).count == value.count,
                  value.allSatisfy({ proposal in
                      Set(proposal.operations.map(\.operation_id)).count == proposal.operations.count &&
                      proposal.operations.allSatisfy { ["add", "move", "remove"].contains($0.op) && photoIDs.contains($0.photo_id) }
                  })
            else { throw APIError.invalidResponse }
        }
    }

    private static func isHash(_ value: String) -> Bool {
        value.count == 64 && value.allSatisfy { "0123456789abcdef".contains($0) }
    }
}

// 项目内提案：AI 或协作者对作品序列的建议，逐项人工采纳或驳回。
public struct Proposal: Codable, Sendable, Identifiable, Equatable {
    public struct Operation: Codable, Sendable, Equatable, Identifiable {
        public let op: String
        public let operation_id: String
        public let photo_id: String
        public let to_index: Int?

        public var id: String { operation_id }
        public var title: String {
            switch op {
            case "add": return "加入序列"
            case "move": return "调整顺序"
            case "remove": return "移出序列"
            default: return op
            }
        }
    }

    public let id: String
    public let title: String
    public let author: String
    public let state: String
    public let base_revision: Int
    public let revision: Int
    public let operations: [Operation]
    public let nonce: String

    public var isSubmitted: Bool { state == "SUBMITTED" }
    public var isDecided: Bool { state == "APPLIED" || state == "REJECTED" }
}

// 画布移动命令：与 boards 服务端 canvasCommandSchema（schemaVersion 3）逐字段一致。
public struct CanvasMoveCommand: Codable, Sendable, Equatable {
    public struct Move: Codable, Sendable, Equatable {
        public let nodeId: String
        public let x: Double
        public let y: Double
    }

    public struct Action: Codable, Sendable, Equatable {
        public let type: String
        public let moves: [Move]
    }

    public struct Operation: Codable, Sendable, Equatable {
        public let opId: String
        public let action: Action
    }

    public let schemaVersion: Int
    public let commandId: String
    public let idempotencyKey: String
    public let projectId: String
    public let baseRevision: Int
    public let operations: [Operation]

    public init(board: Board, nodeID: String, x: Double, y: Double) throws {
        guard UUID(uuidString: board.id) != nil, UUID(uuidString: nodeID) != nil,
              board.revision >= 0, board.revision < Int.max, !board.project_id.isEmpty, x.isFinite, y.isFinite,
              abs(x) <= 1_000_000, abs(y) <= 1_000_000 else { throw APIError.invalidResponse }
        let id = UUID().uuidString.lowercased()
        schemaVersion = 3
        commandId = id
        idempotencyKey = id
        projectId = board.project_id
        baseRevision = board.revision
        operations = [Operation(opId: UUID().uuidString.lowercased(),
                                action: Action(type: "moveNodes", moves: [Move(nodeId: nodeID, x: x, y: y)]))]
    }

    public func validate() throws {
        guard schemaVersion == 3, UUID(uuidString: commandId) != nil, UUID(uuidString: idempotencyKey) != nil,
              !projectId.isEmpty, projectId.count <= 160, baseRevision >= 0, baseRevision < Int.max,
              operations.count == 1, let operation = operations.first, UUID(uuidString: operation.opId) != nil,
              operation.action.type == "moveNodes", operation.action.moves.count == 1,
              operation.action.moves.allSatisfy({ UUID(uuidString: $0.nodeId) != nil && $0.x.isFinite && $0.y.isFinite &&
                  abs($0.x) <= 1_000_000 && abs($0.y) <= 1_000_000 }) else { throw APIError.invalidResponse }
    }
}

public struct CanvasCommandReceipt: Codable, Sendable, Equatable {
    public let ok: Bool
    public let commandId: String
    public let newRevision: Int
    public let affectedNodeIds: [String]

    public func validate(for command: CanvasMoveCommand) throws {
        try command.validate()
        let moved = command.operations.flatMap { $0.action.moves.map(\.nodeId) }
        guard ok, commandId == command.commandId,
              newRevision == command.baseRevision + 1,
              Set(affectedNodeIds) == Set(moved), affectedNodeIds.count == Set(affectedNodeIds).count
        else { throw APIError.unconfirmedCommand }
    }
}

// 提案采纳命令：accepted_operation_ids 为空即驳回；走项目命令权威与幂等事务。
public struct ProposalApplyCommand: Codable, Sendable, Equatable {
    public struct Payload: Codable, Sendable, Equatable {
        public let proposal_id: String
        public let expected_proposal_revision: Int
        public let approval_nonce: String
        public let accepted_operation_ids: [String]
    }

    public let version: String
    public let command_id: String
    public let workspace_id: String
    public let project_id: String
    public let expected_revision: Int
    public let idempotency_key: String
    public let kind: String
    public let payload: Payload

    public init(snapshot: ProjectSnapshot, proposal: Proposal, acceptedOperationIDs: [String]) throws {
        guard snapshot.state == .SHARED, snapshot.principal.can_approve, proposal.isSubmitted,
              Set(acceptedOperationIDs).isSubset(of: Set(proposal.operations.map(\.operation_id)))
        else { throw APIError.permissionDenied }
        let id = UUID().uuidString.lowercased()
        version = "2.0"
        command_id = id
        idempotency_key = id
        workspace_id = snapshot.workspace_id
        project_id = snapshot.project_id
        expected_revision = snapshot.revision
        kind = "proposal.apply"
        payload = Payload(proposal_id: proposal.id, expected_proposal_revision: proposal.revision,
                          approval_nonce: proposal.nonce, accepted_operation_ids: acceptedOperationIDs)
    }
}

// 提案创建/提交命令：AI 草案经人工确认后以用户身份进入权威审阅流；base_revision 必须等于 expected_revision。
public struct ProposalCreateCommand: Codable, Sendable, Equatable {
    public struct DraftOperation: Codable, Sendable, Equatable {
        public let op: String
        public let operation_id: String
        public let photo_id: String
        public let to_index: Int?
    }

    public struct Payload: Codable, Sendable, Equatable {
        public let title: String
        public let base_revision: Int
        public let operations: [DraftOperation]
    }

    public let version: String
    public let command_id: String
    public let workspace_id: String
    public let project_id: String
    public let expected_revision: Int
    public let idempotency_key: String
    public let kind: String
    public let payload: Payload

    public init(snapshot: ProjectSnapshot, title: String, operations: [DraftOperation]) throws {
        guard snapshot.state == .SHARED, snapshot.principal.canPropose,
              (1...200).contains(title.trimmingCharacters(in: .whitespacesAndNewlines).count),
              !operations.isEmpty, operations.count <= 200
        else { throw APIError.permissionDenied }
        let id = UUID().uuidString.lowercased()
        version = "2.0"
        command_id = id
        idempotency_key = id
        workspace_id = snapshot.workspace_id
        project_id = snapshot.project_id
        expected_revision = snapshot.revision
        kind = "proposal.create"
        payload = Payload(title: title.trimmingCharacters(in: .whitespacesAndNewlines),
                          base_revision: snapshot.revision, operations: operations)
    }
}

public struct ProposalSubmitCommand: Codable, Sendable, Equatable {
    public struct Payload: Codable, Sendable, Equatable {
        public let proposal_id: String
        public let expected_proposal_revision: Int
    }

    public let version: String
    public let command_id: String
    public let workspace_id: String
    public let project_id: String
    public let expected_revision: Int
    public let idempotency_key: String
    public let kind: String
    public let payload: Payload

    public init(snapshot: ProjectSnapshot, proposalID: String) {
        let id = UUID().uuidString.lowercased()
        version = "2.0"
        command_id = id
        idempotency_key = id
        workspace_id = snapshot.workspace_id
        project_id = snapshot.project_id
        expected_revision = snapshot.revision
        kind = "proposal.submit"
        payload = Payload(proposal_id: proposalID, expected_proposal_revision: 0)
    }
}

// AI 草案：从模型回复文本中解析出的选片建议；只允许 add/remove/move 与项目内照片。
// 模型输出是不可信数据——解析结果必须经照片闭包校验后才能生成正式命令。
public struct AIDraftProposal: Sendable, Equatable {
    public struct ParsedOperation: Sendable, Equatable, Identifiable {
        public let op: String
        public let photoFilename: String
        public let toIndex: Int?
        public var id: String { "\(op)-\(photoFilename)-\(toIndex ?? -1)" }
    }

    public let title: String
    public let operations: [ParsedOperation]

    /// 从回复文本提取 ```json 代码块中的 {title, operations:[{op, photo, to_index}]}；
    /// photo 按文件名匹配（大小写不敏感、去空白），to_index 仅接受 add/move 的非负整数。
    public static func parse(_ text: String, photos: [Photo]) -> AIDraftProposal? {
        guard let block = firstJSONBlock(in: text),
              let object = try? JSONSerialization.jsonObject(with: block) as? [String: Any],
              let rawTitle = object["title"] as? String,
              (1...200).contains(rawTitle.trimmingCharacters(in: .whitespacesAndNewlines).count),
              let rawOperations = object["operations"] as? [[String: Any]],
              !rawOperations.isEmpty, rawOperations.count <= 200
        else { return nil }
        let byName = Dictionary(uniqueKeysWithValues: photos.map { ($0.filename.lowercased(), $0) })
        var parsed: [ParsedOperation] = []
        for raw in rawOperations {
            guard let op = raw["op"] as? String,
                  ["add", "remove", "move"].contains(op),
                  let photoName = raw["photo"] as? String,
                  let photo = byName[photoName.lowercased().trimmingCharacters(in: .whitespaces)]
            else { return nil }
            let toIndex = raw["to_index"] as? Int
            if op == "add", !(0...10_000).contains(toIndex ?? -1) { return nil }
            if op == "move", toIndex == nil || toIndex! < 0 { return nil }
            parsed.append(ParsedOperation(op: op, photoFilename: photo.filename, toIndex: toIndex))
        }
        guard !parsed.isEmpty else { return nil }
        return AIDraftProposal(title: rawTitle.trimmingCharacters(in: .whitespacesAndNewlines), operations: parsed)
    }

    static func firstJSONBlock(in text: String) -> Data? {
        guard let fenceStart = text.range(of: "```") else { return nil }
        var rest = text[fenceStart.upperBound...]
        if let newline = rest.firstIndex(of: "\n") { rest = rest[rest.index(after: newline)...] }
        guard let fenceEnd = rest.range(of: "```") else { return nil }
        let body = rest[..<fenceEnd.lowerBound].trimmingCharacters(in: .whitespacesAndNewlines)
        return body.data(using: .utf8)
    }
}

public struct Board: Decodable, Sendable, Identifiable, Equatable {
    public let id: String
    public let project_id: String
    public let title: String
    public let state: String
    public let revision: Int

    public init(id: String, project_id: String, title: String, state: String, revision: Int) {
        self.id = id
        self.project_id = project_id
        self.title = title
        self.state = state
        self.revision = revision
    }

    public var isDraft: Bool { state == "draft" }
}

public struct BoardNode: Decodable, Sendable, Identifiable, Equatable {
    public let id: String
    public let board_id: String
    public let node_type: String
    // x/y 可变：画布命令成功后本地立即反映新位置。
    public var x: Double
    public var y: Double
    public let width: Double
    public let height: Double
    public let rotation: Double
    public let z_order: Int
    public let photo_id: String?
    public let visible: Bool
}

public struct AgentResult: Decodable, Sendable, Identifiable {
    public let id: String
    public let task_type: String
    public let model_id: String
    public let summary: String
    public let comment_count: Int
    public let created_by_name: String
}

public struct SelectionCommand: Codable, Sendable, Equatable {
    public struct Payload: Codable, Sendable, Equatable {
        public let photo_ids: [String]
        public let selection_state: Selection
    }

    public let version: String
    public let command_id: String
    public let workspace_id: String
    public let project_id: String
    public let expected_revision: Int
    public let idempotency_key: String
    public let kind: String
    public let payload: Payload

    public init(snapshot: ProjectSnapshot, photoID: String, selection: Selection) throws {
        guard snapshot.state == .SHARED, snapshot.principal.permitsSelection,
              snapshot.snapshot.photos.contains(where: { $0.id == photoID })
        else { throw APIError.permissionDenied }
        let id = UUID().uuidString.lowercased()
        version = "2.0"
        command_id = id
        workspace_id = snapshot.workspace_id
        project_id = snapshot.project_id
        expected_revision = snapshot.revision
        idempotency_key = id
        kind = "photo.set_selection"
        payload = Payload(photo_ids: [photoID], selection_state: selection)
    }
}

public struct CommandReceipt: Codable, Sendable {
    public let ok: Bool
    public let command_id: String
    public let new_revision: Int
    public let event_id: String
    public let affected_ids: [String]

    public func validate(for command: SelectionCommand) throws {
        // v2 的选片回执可能不返回 affected_ids；以原命令编号、版本和事件回执绑定。
        guard ok, command_id == command.command_id,
              new_revision == command.expected_revision + 1,
              !event_id.isEmpty,
              affected_ids.isEmpty || (affected_ids.count == command.payload.photo_ids.count && Set(affected_ids) == Set(command.payload.photo_ids))
        else { throw APIError.unconfirmedCommand }
    }
}

public struct WorkspaceSummary: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let title: String
    public let role: String

    public var permitsPromotion: Bool {
        ["OWNER", "ADMIN", "EDITOR"].contains(role)
    }
}

public struct DeviceRecord: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let name: String
    public let platform: String
    public let current: Bool
}

public struct DeviceRegistration: Codable, Sendable, Equatable {
    public let id: String
    public let name: String
    public let platform: String
}

// 研究参考：项目内辅助资料（笔记/链接），独立身份域，不进入照片资产、选片或正式修订。
public struct ResearchEntry: Decodable, Sendable, Identifiable, Equatable {
    public let id: String
    public let kind: String
    public let title: String
    public let body: String?
    public let url: String?
    public let author: String?
    public let source: String?
    public let license: String?
    public let created_by: String
    public let created_by_name: String

    public var isNote: Bool { kind == "note" }
    public var isLink: Bool { kind == "link" }
}

public struct ResearchRecord: Decodable, Sendable, Equatable {
    public let id: String
    public let kind: String
    public let title: String
    public let body: String?
    public let url: String?
    public let author: String?
    public let source: String?
    public let license: String?
    public let created_by: String

    public func validate(kind: ResearchDraft.Kind?) throws {
        guard kind == nil || kind!.rawValue == self.kind,
              !id.isEmpty, !title.isEmpty, !created_by.isEmpty,
              self.kind == "note" || self.kind == "link",
              self.kind != "note" || body != nil,
              self.kind != "link" || url != nil
        else { throw APIError.invalidResponse }
    }
}

// 与服务端 researchReferenceSchema 逐字段一致；可选项空串按 nil 处理，不发送空值。
public struct ResearchDraft: Encodable, Sendable, Equatable {
    public enum Kind: String, Encodable, Sendable { case note, link }

    public let kind: Kind
    public let title: String
    public let body: String?
    public let url: String?
    public let author: String?
    public let source: String?
    public let license: String?

    public init(kind: Kind, title: String, body: String? = nil, url: String? = nil,
                author: String? = nil, source: String? = nil, license: String? = nil) throws {
        let trimmedTitle = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard (1...200).contains(trimmedTitle.count) else { throw APIError.invalidResponse }
        self.kind = kind
        self.title = trimmedTitle
        self.body = try ResearchDraft.optional(body, cap: 10_000)
        self.url = try ResearchDraft.optional(url, cap: 2_000)
        self.author = try ResearchDraft.optional(author, cap: 200)
        self.source = try ResearchDraft.optional(source, cap: 500)
        self.license = try ResearchDraft.optional(license, cap: 500)
        switch kind {
        case .note:
            guard let body = self.body, !body.isEmpty else { throw APIError.invalidResponse }
            guard self.url == nil else { throw APIError.invalidResponse }
        case .link:
            guard let url = self.url, let value = URL(string: url),
                  ["https", "http"].contains(value.scheme?.lowercased() ?? ""), url.count <= 2_000,
                  let host = value.host, !host.isEmpty
            else { throw APIError.invalidResponse }
            guard self.body == nil else { throw APIError.invalidResponse }
        }
    }

    private static func `optional`(_ raw: String?, cap: Int) throws -> String? {
        guard let raw else { return nil }
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty ? true : trimmed.count <= cap else { throw APIError.invalidResponse }
        return trimmed.isEmpty ? nil : trimmed
    }
}

// 部分更新：nil 字段不发送（合成 Encodable 用 encodeIfPresent）；服务端不接受空串，暂无清除字段路径。
public struct ResearchPatch: Encodable, Sendable, Equatable {
    public let title: String?
    public let body: String?
    public let url: String?
    public let author: String?
    public let source: String?
    public let license: String?

    public init(title: String? = nil, body: String? = nil, url: String? = nil,
                author: String? = nil, source: String? = nil, license: String? = nil) throws {
        let fields = [title, body, url, author, source, license]
        guard fields.contains(where: { $0 != nil }) else { throw APIError.invalidResponse }
        if let title { let trimmed = title.trimmingCharacters(in: .whitespacesAndNewlines); guard (1...200).contains(trimmed.count) else { throw APIError.invalidResponse } }
        if let body { let trimmed = body.trimmingCharacters(in: .whitespacesAndNewlines); guard (1...10_000).contains(trimmed.count) else { throw APIError.invalidResponse } }
        if let url {
            let trimmed = url.trimmingCharacters(in: .whitespacesAndNewlines)
            guard let value = URL(string: trimmed), ["https", "http"].contains(value.scheme?.lowercased() ?? ""),
                  let host = value.host, !host.isEmpty, trimmed.count <= 2_000 else { throw APIError.invalidResponse }
        }
        self.title = title.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
        self.body = body.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
        self.url = url.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
        // 空串按“不修改”处理发送 nil：服务端 min(1) 会拒绝空串，且本 API 无清除字段路径。
        self.author = try ResearchPatch.optional(author, cap: 200)
        self.source = try ResearchPatch.optional(source, cap: 500)
        self.license = try ResearchPatch.optional(license, cap: 500)
    }

    private static func `optional`(_ raw: String?, cap: Int) throws -> String? {
        guard let raw else { return nil }
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.count <= cap else { throw APIError.invalidResponse }
        return trimmed.isEmpty ? nil : trimmed
    }
}

// 迁云清单与服务端 promotionSchema 逐字段一致；media_policy 只允许固定字面量。
public struct PromotionMediaPolicy: Codable, Sendable, Equatable {
    public let previews_only: Bool
    public let strip_gps: Bool
    public let max_long_edge: Int
    public let ai_upload_authorized: Bool

    public init() {
        previews_only = true; strip_gps = true; max_long_edge = 2048; ai_upload_authorized = false
    }
}

public struct PromotionManifest: Codable, Sendable, Equatable {
    public let workspace_id: String
    public let local_project_id: String
    public let promotion_id: String
    public let title: String
    public let from_revision: Int
    public let snapshot: SharedSnapshot
    public let media_policy: PromotionMediaPolicy

    public init(workspaceID: String, localProjectID: String, promotionID: String, title: String, snapshot: SharedSnapshot) {
        self.workspace_id = workspaceID
        self.local_project_id = localProjectID
        self.promotion_id = promotionID
        self.title = title
        from_revision = 0
        self.snapshot = snapshot
        media_policy = PromotionMediaPolicy()
    }
}

public struct PromotionStart: Codable, Sendable, Equatable {
    public let project_id: String
    public let state: String
    public let manifest_digest: String

    public func validate(manifest: PromotionManifest) throws {
        let hex = manifest_digest.count == 64 && manifest_digest.allSatisfy { "0123456789abcdef".contains($0) }
        guard state == "STAGING", hex, !project_id.isEmpty else { throw APIError.invalidResponse }
        guard project_id != manifest.promotion_id else { throw APIError.invalidResponse }
    }
}

public struct PromotionFinalized: Codable, Sendable, Equatable {
    public let project_id: String
    public let workspace_id: String
    public let authority: String
    public let promotion_id: String
    public let from_revision: Int
    public let manifest_digest: String
}

public struct PreviewUploadReceipt: Codable, Sendable, Equatable {
    public let photo_id: String
    public let sha256: String
    public let width: Int
    public let height: Int
    public let gps_stripped: Bool

    public func validate(photoID: String) throws {
        let hex = sha256.count == 64 && sha256.allSatisfy { "0123456789abcdef".contains($0) }
        guard photo_id == photoID, gps_stripped, hex,
              width > 0, height > 0, width <= 2048, height <= 2048 else { throw APIError.invalidResponse }
    }
}

public struct ProjectEvent: Codable, Sendable, Equatable, Identifiable {
    public let id: Int
    public let kind: String
    public let revision: Int
    public let actor_id: String
}

public struct ProjectEventsPage: Codable, Sendable, Equatable {
    public let events: [ProjectEvent]
    public let nextCursor: Int?

    // 满页代表可能还有增量；空页才证明已追平。
    public var hasMore: Bool { nextCursor != nil && !events.isEmpty }
}

// 账户 AI 额度与策略：与服务端 GET /ai/account 响应逐字段一致。
public struct AIAccount: Codable, Sendable, Equatable {
    public struct Recovery: Codable, Sendable, Equatable {
        public let enabled: Bool
    }

    public let enabled: Bool
    public let verified: Bool
    public let policy: String
    public let provider: String
    public let model: String
    public let unit: String
    public let granted: Int
    public let spent: Int
    public let reserved: Int
    public let available: Int
    public let recovery: Recovery
}

// 模型回复：output 以宽松子集解码，文本从 output_text 片段聚合；未知形状仍可解码不崩溃。
public struct AIReply: Codable, Sendable, Equatable {
    public struct OutputItem: Codable, Sendable, Equatable {
        public struct ContentPart: Codable, Sendable, Equatable {
            public let type: String?
            public let text: String?
        }

        public let type: String?
        public let role: String?
        public let content: [ContentPart]?
    }

    public struct Usage: Codable, Sendable, Equatable {
        public let input_tokens: Int
        public let output_tokens: Int
        public let total_tokens: Int
    }

    public let id: String
    public let status: String
    public let model: String
    public let output: [OutputItem]
    public let usage: Usage

    public var text: String {
        output.flatMap { $0.content ?? [] }.compactMap(\.text).joined(separator: "\n")
    }

    public func validate() throws {
        guard status == "completed", !id.isEmpty, !model.isEmpty,
              usage.input_tokens >= 0, usage.output_tokens >= 0,
              usage.total_tokens == usage.input_tokens + usage.output_tokens
        else { throw APIError.invalidResponse }
    }
}

// 任务回执：状态机 PENDING/COMPLETED/REJECTED/UNKNOWN；result 仅在完成且有留存时非空。
public struct AIJob: Codable, Sendable, Equatable {
    public let jobId: String
    public let state: String
    public let pollUrl: String
    public let result: AIReply?
    public let error: String?

    public func validate(requestID: String) throws {
        guard jobId == requestID, pollUrl.hasSuffix(requestID),
              ["PENDING", "COMPLETED", "REJECTED", "UNKNOWN"].contains(state) else { throw APIError.invalidResponse }
        if let result { try result.validate() }
    }
}

// 对话请求：与服务端 inputSchema（strictObject）逐字段一致；store 恒为 false、model 为字面量。
public struct AIConversationInput: Encodable, Sendable, Equatable {
    public struct Message: Encodable, Sendable, Equatable {
        public struct Part: Encodable, Sendable, Equatable {
            public let type: String
            public let text: String?
            public let image_url: String?
            public let detail: String?
        }

        let type: String
        let role: String
        let content: [Part]
    }

    let model: String
    let input: [Message]
    let instructions: String?
    let max_output_tokens: Int
    let store: Bool

    public init(history: [(role: String, text: String)], images: [VisionPreview] = [], instructions: String? = nil, maxOutputTokens: Int = 1024) {
        model = "deepseek-flash"
        input = history.enumerated().map { index, item in
            var parts = [Message.Part(type: item.role == "user" ? "input_text" : "output_text", text: item.text, image_url: nil, detail: nil)]
            if index == history.count - 1 && item.role == "user" {
                parts += images.map { Message.Part(type: "input_image", text: nil, image_url: $0.dataURL, detail: "low") }
            }
            return Message(type: "message", role: item.role, content: parts)
        }
        self.instructions = instructions
        max_output_tokens = maxOutputTokens
        store = false
    }

    public var imageCount: Int { input.flatMap(\.content).filter { $0.type == "input_image" }.count }
}

// 项目 Agent 任务（加密队列）读取；取消仅限本人或管理员、仅限排队/运行中。
public struct AgentJob: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let kind: String
    public let status: String
    public let attempts: Int
    public let error: String?
    public let result_id: String?
    public let created_by_name: String

    public var isActive: Bool { status == "QUEUED" || status == "RUNNING" }
}

// 任意 JSON 值：版本差异的 before/after 为任意结构，宽松解码用于展示。
public enum JSONValue: Codable, Sendable, Equatable {
    case null, bool(Bool), number(Double), string(String), array([JSONValue]), object([String: JSONValue])

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() { self = .null }
        else if let value = try? container.decode(Bool.self) { self = .bool(value) }
        else if let value = try? container.decode(Double.self) { self = .number(value) }
        else if let value = try? container.decode(String.self) { self = .string(value) }
        else if let value = try? container.decode([JSONValue].self) { self = .array(value) }
        else if let value = try? container.decode([String: JSONValue].self) { self = .object(value) }
        else { throw DecodingError.dataCorruptedError(in: container, debugDescription: "无法解码的 JSON 值") }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .null: try container.encodeNil()
        case .bool(let value): try container.encode(value)
        case .number(let value): try container.encode(value)
        case .string(let value): try container.encode(value)
        case .array(let value): try container.encode(value)
        case .object(let value): try container.encode(value)
        }
    }

    public var displayText: String {
        switch self {
        case .null: return "空"
        case .bool(let value): return value ? "真" : "假"
        case .number(let value):
            return value.truncatingRemainder(dividingBy: 1) == 0 ? String(Int(value)) : String(value)
        case .string(let value): return value
        case .array: return "（列表变更）"
        case .object: return "（结构变更）"
        }
    }
}

// 版本历史条目：revision 为身份；时间为 ISO 字符串原样展示。
public struct VersionSummary: Decodable, Sendable, Identifiable, Equatable {
    public let revision: Int
    public let actor_id: String
    public let command_id: String
    public let created_at: String

    public var id: Int { revision }
}

// 双版本差异：与服务端 versionDiff 响应逐字段一致。
public struct VersionDiff: Decodable, Sendable, Equatable {
    public struct Endpoint: Decodable, Sendable, Equatable {
        public let revision: Int
        public let created_at: String
    }

    public struct FieldChange: Decodable, Sendable, Equatable {
        public let before: JSONValue?
        public let after: JSONValue?
    }

    public struct ChangedEntity: Decodable, Sendable, Equatable, Identifiable {
        public let id: String
        public let fields: [String: FieldChange]
    }

    public struct EntityDiff: Decodable, Sendable, Equatable {
        public let added: [String]
        public let removed: [String]
        public let changed: [ChangedEntity]
    }

    public let project_id: String
    public let from: Endpoint
    public let to: Endpoint
    public let photos: EntityDiff
    public let sequences: EntityDiff
}

// 提案投影差异：stale 时 sequence 为 null，采纳会被服务端拒绝；
// 非 stale 时 sequence 是服务端按提案操作顺序投影的增删移清单（photo_id 数组）。
public struct ProposalDiff: Decodable, Sendable, Equatable {
    public struct SequenceChange: Decodable, Sendable, Equatable {
        public struct Move: Decodable, Sendable, Equatable {
            public let photo_id: String
            public let fromIndex: Int
            public let toIndex: Int
        }

        public let id: String
        public let before: [String]
        public let after: [String]
        public let added: [String]
        public let removed: [String]
        public let moved: [Move]
    }

    public let proposalId: String
    public let state: String
    public let baseRevision: Int
    public let currentRevision: Int
    public let stale: Bool
    public let sequence: SequenceChange?
}

// 账单目录：与 subscriptionCatalogSchema 逐字段一致；salesAvailable=false 时价格恒为空。
public struct BillingCatalog: Decodable, Sendable, Equatable {
    public struct Price: Decodable, Sendable, Equatable {
        public let id: String
        public let interval: String
        public let amount: Int
        public let currency: String
        public let includedCredits: Int
    }

    public struct Plan: Decodable, Sendable, Identifiable, Equatable {
        public let id: String
        public let name: String
        public let description: String
        public let features: [String]
        public let prices: [Price]
    }

    public let salesAvailable: Bool
    public let plans: [Plan]
}
