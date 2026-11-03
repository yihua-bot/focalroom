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
    public let preferenceProposals: [NativeAIPreference]
    public var isActive: Bool { ["queued", "planning", "running"].contains(state) }
    public var needsReview: Bool { state == "review_ready" }
}
public struct NativeAIMessage: Decodable, Sendable, Identifiable {
    public let id: Int
    public let role: String
    public let content: String
    public let createdAt: String
}
public struct NativeAIObservation: Decodable, Sendable, Identifiable {
    public let photo_id: String
    public let visible_facts: [String]
    public let interpretation: String
    public let uncertainty: String
    public var id: String { photo_id }
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
