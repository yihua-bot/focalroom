import Foundation

// 与桌面 Rust 投影一致；模型文字不会被客户端解析为正式操作。
public struct NativeAIProjection: Decodable, Sendable {
    public let tasks: [NativeAITask]
    public let drafts: [NativeAIDraft]
}
public struct NativeAITask: Decodable, Sendable, Identifiable {
    public let id: String
    public let goal: String
    public let state: String
    public let reason: String
    public let question: String?
    public let messages: [NativeAIMessage]
    public let boardDrafts: [NativeAIBoard]
    public let budget: NativeAIBudget
    public let inspectedPhotos: [NativeAIObservation]
    public let photoJudgments: [NativePhotoJudgment]?
    public let memoryTrace: NativeMemoryTrace?
    public let preferenceProposals: [NativeAIPreference]
    public let draftEvidence: [NativeDraftEvidence]?
    public var isActive: Bool { ["queued", "planning", "running"].contains(state) }
    public var needsReview: Bool { state == "review_ready" }

    private enum CodingKeys: String, CodingKey {
        case id, goal, state, reason, question, messages, boardDrafts, budget, inspectedPhotos
        case photoJudgments, memoryTrace, preferenceProposals, draftEvidence
    }
    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        id = try values.decode(String.self, forKey: .id)
        goal = try values.decode(String.self, forKey: .goal)
        state = try values.decode(String.self, forKey: .state)
        reason = try values.decode(String.self, forKey: .reason)
        question = try values.decodeIfPresent(String.self, forKey: .question)
        messages = try values.decode([NativeAIMessage].self, forKey: .messages)
        boardDrafts = try values.decode([NativeAIBoard].self, forKey: .boardDrafts)
        budget = try values.decode(NativeAIBudget.self, forKey: .budget)
        inspectedPhotos = try values.decode([NativeAIObservation].self, forKey: .inspectedPhotos)
        photoJudgments = try values.decodeIfPresent([NativePhotoJudgment].self, forKey: .photoJudgments)
        memoryTrace = try values.decodeIfPresent(NativeMemoryTrace.self, forKey: .memoryTrace)
        preferenceProposals = try values.decode([NativeAIPreference].self, forKey: .preferenceProposals)
        // 口径只是锦上添花：这一格形状不合时不能把整份任务投影带走，读不回来就什么也不说。
        draftEvidence = try? values.decodeIfPresent([NativeDraftEvidence].self, forKey: .draftEvidence)
    }
}

/// 草案的画面依据口径：只有数量与档位，没有观察正文。档位与图像版本都是有限几种写法，
/// 名字之外的任何值一律当成没记过——宁可不写这一张，也不猜一个档位摆给人看。
public struct NativeDraftEvidence: Decodable, Sendable, Identifiable {
    public struct PerPhoto: Decodable, Sendable, Identifiable {
        public static let tiers = ["current_page_tier", "current_detail_tier", "other_prompt_version", "other_tool"]
        public static let pixels = ["mismatch", "unreadable", "unrecorded"]
        public static let originals = ["missing", "changed", "matches", "unchecked"]
        public let photoId: String
        public let used: Bool
        public let contract: String?
        public let pixelVersion: String?
        public let originalStatus: String?
        public var id: String { photoId }

        private enum CodingKeys: String, CodingKey { case photoId, used, contract, pixelVersion, originalStatus }
        public init(from decoder: Decoder) throws {
            let values = try decoder.container(keyedBy: CodingKeys.self)
            photoId = try values.decode(String.self, forKey: .photoId)
            used = try values.decode(Bool.self, forKey: .used)
            func known(_ allowed: [String], _ key: CodingKeys) -> String? {
                guard let name = try? values.decodeIfPresent(String.self, forKey: key),
                      allowed.contains(name) else { return nil }
                return name
            }
            contract = known(Self.tiers, .contract)
            pixelVersion = known(Self.pixels, .pixelVersion)
            originalStatus = known(Self.originals, .originalStatus)
        }
    }
    public let draftId: String
    public let observed: Int
    public let unobserved: Int
    public let pageTier: Int
    public let detailTier: Int
    public let otherVersion: Int
    public let otherTool: Int
    public let crossContractPairs: Int
    public let omitted: Int
    public let scanned: Int
    public let draftRevision: Int?
    public let perPhoto: [PerPhoto]
    public let basisSummary: Bool
    public var id: String { draftId }

    private enum CodingKeys: String, CodingKey {
        case draftId, observed, unobserved, pageTier, detailTier, otherVersion, otherTool
        case crossContractPairs, omitted, scanned, draftRevision, perPhoto, basisSummary
    }
    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        draftId = try values.decode(String.self, forKey: .draftId)
        observed = try values.decode(Int.self, forKey: .observed)
        unobserved = try values.decode(Int.self, forKey: .unobserved)
        pageTier = try values.decode(Int.self, forKey: .pageTier)
        detailTier = try values.decode(Int.self, forKey: .detailTier)
        otherVersion = try values.decode(Int.self, forKey: .otherVersion)
        otherTool = try values.decode(Int.self, forKey: .otherTool)
        crossContractPairs = try values.decode(Int.self, forKey: .crossContractPairs)
        omitted = try values.decode(Int.self, forKey: .omitted)
        // 逐张档位没读回来时投影会就地取计数并标 basisSummary，那一格不许装成扫过更多张。
        scanned = try values.decode(Int.self, forKey: .scanned)
        draftRevision = try values.decodeIfPresent(Int.self, forKey: .draftRevision)
        perPhoto = try values.decode([PerPhoto].self, forKey: .perPhoto)
        basisSummary = try values.decodeIfPresent(Bool.self, forKey: .basisSummary) ?? false
    }
}
public struct NativeAIMessage: Decodable, Sendable, Identifiable {
    public let id: Int
    public let role: String
    public let content: String
    public let createdAt: String
}
public struct NativeMemoryTrace: Decodable, Sendable {
    public let events: [Event]
    public let hasEarlierEvents: Bool
    public struct Event: Decodable, Sendable, Identifiable {
        public enum Kind: String, Decodable, Sendable { case context_attempt, query_result }
        public let id: Int
        public let kind: Kind
        public let round: Int?
        public let createdAt: String
        public let recorded: Bool
        public let reads: [Read]
    }
    public struct Read: Decodable, Sendable, Identifiable {
        public enum TextVersion: String, Decodable, Sendable { case verified, changed, unrecorded, source_unavailable }
        public let memoryId: Int
        public var id: Int { memoryId }
        public let sourceProjectId: String?
        public let sourceProjectTitle: String?
        public let origin: String?
        public let scope: String?
        public let statusAtRead: String?
        public let selectionReason: String?
        public let confirmation: Confirmation?
        public let memoryTextVersion: TextVersion
        public let passage: String?
        public let currentStatus: String?
        public let disabled: Bool?
        public let applicability: String?
        public let evidence: Evidence?
        public let evidenceRefsTruncated: Bool
    }
    public struct Confirmation: Decodable, Sendable {
        public enum Action: String, Decodable, Sendable {
            case confirmHypothesis = "ConfirmHypothesis", rememberPreference = "RememberPreference"
        }
        public let requestId: String
        public let createdAt: String
        public let action: Action
    }
    public struct Evidence: Decodable, Sendable {
        public let recorded: Int
        public let available: Int
        public let excluded: Int
        public let missing: Int
    }
}
public struct NativeAIObservation: Decodable, Sendable, Identifiable {
    public let photo_id: String
    public let visible_facts: [String]
    public let interpretation: String
    public let uncertainty: String
    public var id: String { photo_id }
}
public struct NativePhotoJudgment: Decodable, Sendable, Identifiable {
    public enum ImageStatus: String, Decodable, Sendable {
        case verified, mismatch, unreadable, unavailable, unrecorded
    }
    public let id: String
    public let photoId: String
    public let judgment: String
    public let reasoning: String
    public let uncertainty: String
    public let observationUncertainty: String
    public let imageStatus: ImageStatus
    public let facts: [Fact]
    public let methods: [Method]
    public struct Fact: Decodable, Sendable {
        public let index: Int
        public let text: String
    }
    public struct Method: Decodable, Sendable {
        public let citationId: String
        public let title: String
        public let passage: String
        public let source: String?
        public let kind: String
        public let reviewStatus: String?
        public let disputed: Bool
        public let excerpt: Bool
    }
}
public struct NativeAIPreference: Decodable, Sendable, Identifiable {
    public let id: String
    public let statement: String
    public let applicability: String
    public let state: String
}
public struct NativeAIBudget: Codable, Sendable {
    public var maxModelTurns = 16
    public var maxToolCalls = 40
    public var maxDurationSeconds = 480
    public var maxCloudImages = 0
    public var maxCostMinorUsd = 100
    public var maxContextTokens = 48000
    public var maxTotalTokens = 500000
    public var maxOutputTokens = 8192
    public init(images: Int = 0) { maxCloudImages = images }
}
public struct NativeAIDraft: Decodable, Sendable, Identifiable {
    public let id: String
    public let projectId: String
    public let taskId: String
    public let title: String
    public let state: String
    public let stale: Bool
    public let baseRevision: Int
    public let revision: Int
    public let summary: String
    public let items: [Item]
    public struct Item: Decodable, Sendable {
        public let photo_id: String
        public let proposed_action: String
        public let position: Int
        public let rationale: String
    }
}
public struct NativeAIBoard: Decodable, Sendable, Identifiable {
    public let id: String
    public let title: String
    public let state: String
    public let revision: Int
}
public struct NativeAIContext: Codable, Sendable {
    public let projectId: String
    public let photoIDs: [String: String]
    public let revision: Int
}
public struct NativeAINode: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let nodeType: String
    public let assetId: String?
    public let contentJson: String?
    public let x: Double
    public let y: Double
    public let width: Double
    public let height: Double
    public let zOrder: Int
    public let rotation: Double
    public let visible: Bool
}

// 只有已完成回执携带正文；待确认状态不能用半截增量冒充结果。
public struct NativeChatJob: Decodable, Sendable {
    public let state: String
    public let text: String?
    private enum CodingKeys: String, CodingKey { case state, text }
    public init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        state = try values.decode(String.self, forKey: .state)
        text = try values.decodeIfPresent(String.self, forKey: .text)
        guard ["completed", "pending", "rejected", "expired", "unknown"].contains(state),
              (state == "completed" ? text?.isEmpty == false : text == nil) else {
            throw DecodingError.dataCorruptedError(forKey: .state, in: values, debugDescription: "Invalid chat receipt")
        }
    }
}
