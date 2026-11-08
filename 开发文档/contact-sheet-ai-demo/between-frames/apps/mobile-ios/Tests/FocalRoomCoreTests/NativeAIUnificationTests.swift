import Foundation
import XCTest
import ImageIO
import CoreGraphics
import UniformTypeIdentifiers
@testable import FocalRoomCore

@MainActor final class NativeAIUnificationTests: XCTestCase {
    func testMemoryTraceKeepsAttemptQueryAndConfirmationProvenanceSeparate() throws {
        var read: [String: Any] = ["memoryId": 7, "scope": "cross_project", "sourceProjectId": "source", "sourceProjectTitle": "来源",
                                   "origin": "user_confirmed", "statusAtRead": "confirmed", "selectionReason": "author_keyword_ranked",
                                   "memoryTextVersion": "changed", "currentStatus": "retired", "disabled": true, "evidenceRefsTruncated": false,
                                   "evidence": ["recorded": 2, "available": 1, "excluded": 1, "missing": 0],
                                   "confirmation": ["requestId": "approved", "createdAt": "2026-11-05T10:00:00Z", "action": "ConfirmHypothesis"]]
        let trace: [String: Any] = ["hasEarlierEvents": true, "events": [
            ["id": 1, "kind": "context_attempt", "round": 2, "createdAt": "2026-11-05T11:00:00Z", "recorded": true, "reads": [read]],
            ["id": 2, "kind": "query_result", "createdAt": "2026-11-05T11:01:00Z", "recorded": false, "reads": []]]]
        let decoded = try JSONDecoder().decode(NativeMemoryTrace.self, from: JSONSerialization.data(withJSONObject: trace))
        XCTAssertTrue(decoded.hasEarlierEvents)
        XCTAssertEqual(decoded.events[0].kind, .context_attempt)
        XCTAssertEqual(decoded.events[0].round, 2)
        XCTAssertEqual(decoded.events[1].kind, .query_result)
        XCTAssertFalse(decoded.events[1].recorded)
        XCTAssertNil(decoded.events[1].round)
        let item = try XCTUnwrap(decoded.events.first?.reads.first)
        XCTAssertEqual(item.memoryTextVersion, .changed)
        XCTAssertNil(item.passage)
        XCTAssertEqual(item.confirmation?.action, .confirmHypothesis)
        XCTAssertEqual(item.currentStatus, "retired")
        read["confirmation"] = ["requestId": "fake", "createdAt": "now", "action": "ModelConfirmed"]
        XCTAssertThrowsError(try JSONDecoder().decode(NativeMemoryTrace.Read.self, from: JSONSerialization.data(withJSONObject: read)))
    }

    func testPhotoJudgmentsKeepLegacyTasksReadableAndPreserveHistoricalEvidence() throws {
        var task: [String: Any] = ["id": "task", "goal": "说明依据", "state": "completed", "reason": "",
                                  "messages": [], "boardDrafts": [], "inspectedPhotos": [], "preferenceProposals": [],
                                  "budget": try JSONSerialization.jsonObject(with: JSONEncoder().encode(NativeAIBudget()))]
        let legacy = try JSONDecoder().decode(NativeAITask.self, from: JSONSerialization.data(withJSONObject: task))
        XCTAssertNil(legacy.photoJudgments)
        var judgment: [String: Any] = ["id": "judgment", "photoId": "photo", "judgment": "一种可能的编辑作用",
                                      "reasoning": "依据背景观察", "uncertainty": "作者意图未知", "observationUncertainty": "分辨率有限",
                                      "imageStatus": "mismatch", "facts": [["index": 0, "text": "明亮背景"]],
                                      "methods": [["citationId": "source", "title": "方法", "passage": "编辑假设", "kind": "retrieved_claim", "reviewStatus": "imported", "disputed": true, "excerpt": false]]]
        task["photoJudgments"] = [judgment]
        let decoded = try JSONDecoder().decode(NativeAITask.self, from: JSONSerialization.data(withJSONObject: task))
        let item = try XCTUnwrap(decoded.photoJudgments?.first)
        XCTAssertEqual(item.imageStatus, .mismatch)
        XCTAssertEqual(item.facts.first?.text, "明亮背景")
        XCTAssertEqual(item.methods.first?.reviewStatus, "imported")
        XCTAssertEqual(item.methods.first?.disputed, true)
        XCTAssertNil(item.methods.first?.source)
        judgment["imageStatus"] = "guessed"
        XCTAssertThrowsError(try JSONDecoder().decode(NativePhotoJudgment.self, from: JSONSerialization.data(withJSONObject: judgment)))
    }

    func testAIAccountWithoutDetailCapabilityRemainsReadable() throws {
        var payload: [String: Any] = ["enabled": true, "verified": true, "policy": "deepseek-flash-v1",
                                    "provider": "DeepSeek", "model": "deepseek-flash", "unit": "tokens",
                                    "granted": 1000, "spent": 0, "reserved": 0, "available": 1000,
                                    "recovery": ["enabled": false]]
        let legacy = try JSONDecoder().decode(AIAccount.self, from: JSONSerialization.data(withJSONObject: payload))
        XCTAssertNil(legacy.visionDetail, "旧网关不能被当作已开放精查")
        payload["visionDetail"] = ["policy": "preview1400-high-v1", "maxEdge": 1400, "maxPhotosPerTask": 4]
        let upgraded = try JSONDecoder().decode(AIAccount.self, from: JSONSerialization.data(withJSONObject: payload))
        XCTAssertEqual(upgraded.visionDetail?.policy, "preview1400-high-v1")
        XCTAssertEqual(upgraded.visionDetail?.maxEdge, 1400)
        XCTAssertEqual(upgraded.visionDetail?.maxPhotosPerTask, 4)
    }

    func testReviewedSelectionIsAtomicVersionedIdempotentAndKeepsOriginals() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let scope = AccountScope(server: "https://example.test", userID: "one")
        let library = LocalLibrary(root: root.appendingPathComponent("library"), scope: scope)
        let project = try await library.createProject(title: "共享引擎审阅")
        var ids: [String] = [], sources: [(URL, Data)] = []
        for value in [0.2, 0.7] {
            let file = root.appendingPathComponent(UUID().uuidString + ".jpg")
            let bytes = try jpeg(value)
            try bytes.write(to: file)
            guard case .imported(let photo) = try await library.importFile(file, projectID: project.id) else { return XCTFail() }
            ids.append(photo.id); sources.append((file, bytes))
        }
        let beforeValue = try await library.projects().first
        let before = try XCTUnwrap(beforeValue)
        try await library.applyAISelection(projectID: project.id, expectedUpdatedAt: before.updatedAt, applicationID: "review-one",
                                           decisions: [ids[0]: .KEEP, ids[1]: .KEEP], orderedPhotoIDs: ids.reversed())
        let appliedValue = try await library.projects().first
        let applied = try XCTUnwrap(appliedValue)
        XCTAssertEqual(applied.sequencePhotoIDs, ids.reversed())
        try await library.applyAISelection(projectID: project.id, expectedUpdatedAt: before.updatedAt, applicationID: "review-one",
                                           decisions: [ids[0]: .KEEP, ids[1]: .KEEP], orderedPhotoIDs: ids.reversed())
        let replayed = try await library.projects().first
        XCTAssertEqual(applied, replayed, "回执重放不重复修改版本")
        do {
            try await library.applyAISelection(projectID: project.id, expectedUpdatedAt: before.updatedAt, applicationID: "stale-review",
                                               decisions: [ids[0]: .REJECT], orderedPhotoIDs: [])
            XCTFail("旧版本必须整份拒绝")
        } catch LocalLibraryError.invalidProject {}
        let unchanged = try await library.projects().first
        XCTAssertEqual(unchanged, applied)
        try await library.select(projectID: project.id, photoID: ids[1], selection: .KEEP)
        let keptAgain = try await library.projects().first?.sequencePhotoIDs
        XCTAssertEqual(keptAgain, Array(ids.reversed()), "重复点按保留不能重排已确认的序列")
        let restored = LocalLibrary(root: root.appendingPathComponent("library"), scope: scope)
        let restoredOrder = try await restored.projects().first?.sequencePhotoIDs
        XCTAssertEqual(restoredOrder, Array(ids.reversed()))
        for (file, bytes) in sources { XCTAssertEqual(try Data(contentsOf: file), bytes) }
        let migration = try await restored.beginMigration(projectID: project.id, workspaceID: "workspace")
        let manifest = try JSONDecoder().decode(PromotionManifest.self, from: migration.manifestJSON)
        XCTAssertEqual(manifest.snapshot.sequences[0].items.map(\.photo_id), ids.reversed(), "迁云保持已确认序列")
    }

    func testCloudAndLocalChatsAreSeparateAndCannotRebindLateReceipts() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let scope = AccountScope(server: "https://example.test", userID: "one")
        let store = AIChatStore(root: root, scope: scope)
        let id = UUID().uuidString
        let cloud = AIChatMessage(id: "cloud", role: "user", text: "云端问题", requestID: UUID().uuidString, state: .unknown, cloudProjectID: id)
        let local = AIChatMessage(id: "local", role: "user", text: "本机问题", localProjectID: id)
        try await store.append(cloud); try await store.append(local)
        try await store.complete(cloud, text: "云端回复")
        try await store.complete(cloud, text: "云端回复")
        let cloudMessages = try await store.messages(localProjectID: nil, cloudProjectID: id)
        let localMessages = try await store.messages(localProjectID: id)
        XCTAssertEqual(cloudMessages.map(\.text), ["云端问题", "云端回复"])
        XCTAssertEqual(localMessages.map(\.text), ["本机问题"])
        let forged = AIChatMessage(id: cloud.id, role: "user", text: cloud.text, requestID: cloud.requestID, cloudProjectID: "other")
        do { try await store.complete(forged, text: "错误归属"); XCTFail() } catch APIError.unconfirmedCommand {}
    }

    func testReviewedCanvasHasAtomicBatchUndoAndRedo() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let library = LocalLibrary(root: root, scope: AccountScope(server: "https://example.test", userID: "one"))
        let project = try await library.createProject(title: "画布审核")
        let nodes = [LocalCanvasNode(id: UUID().uuidString, kind: .note, text: "待确认内容", x: 10, y: 20, width: 200, height: 100, zOrder: 0)]
        try await library.applyAICanvas(projectID: project.id, expectedUpdatedAt: project.updatedAt, applicationID: "board-one", nodes: nodes)
        let applied = try await library.projects().first
        try await library.applyAICanvas(projectID: project.id, expectedUpdatedAt: project.updatedAt, applicationID: "board-one", nodes: nodes)
        let replayed = try await library.projects().first
        XCTAssertEqual(applied, replayed)
        let inverse = try await library.applyCanvasOperation(projectID: project.id, .removeNodes(ids: nodes.map(\.id)))
        let removed = try await library.canvas(projectID: project.id)
        XCTAssertEqual(removed?.nodes.count, 0)
        _ = try await library.applyCanvasOperation(projectID: project.id, inverse)
        let restored = try await library.canvas(projectID: project.id)
        XCTAssertEqual(restored?.nodes, nodes)
    }

    func testCloudProposalRetriesReuseFrozenCommandsAcrossRestart() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let scope = AccountScope(server: "https://example.test", userID: "one")
        let snapshot = ProjectSnapshot(project_id: "project", workspace_id: "workspace", title: "项目", state: .SHARED, revision: 4, cursor: "0",
            snapshot: SharedSnapshot(photos: [Photo(id: "a", filename: "a.jpg", source_sha256: String(repeating: "a", count: 64), selection: .KEEP)],
                                     sequences: [PhotoSequence(id: "sequence", title: "序列", items: [])]),
            principal: Principal(id: "one", role: "OWNER", can_approve: true, can_edit_formal: true), previews: [], comments: [])
        let operations = [ProposalCreateCommand.DraftOperation(op: "add", operation_id: "add-a", photo_id: "a", to_index: 0)]
        let store = NativeAIReviewStore(root: root, scope: scope)
        let first = try await store.prepare(applicationID: "review", snapshot: snapshot, title: "建议", operations: operations)
        let reopened = NativeAIReviewStore(root: root, scope: scope)
        let second = try await reopened.prepare(applicationID: "review", snapshot: snapshot, title: "不同标题", operations: operations)
        XCTAssertEqual(first.create, second.create)
        XCTAssertEqual(first.submit, second.submit)
        XCTAssertFalse(second.completed)
        try await reopened.complete(applicationID: "review")
        let finished = try await reopened.record(applicationID: "review")
        XCTAssertEqual(finished?.completed, true)
        let other = NativeAIReviewStore(root: root, scope: AccountScope(server: scope.server, userID: "two"))
        let foreign = try await other.record(applicationID: "review")
        XCTAssertNil(foreign)
    }

    private func jpeg(_ value: Double) throws -> Data {
        let context = try XCTUnwrap(CGContext(data: nil, width: 16, height: 16, bitsPerComponent: 8, bytesPerRow: 0,
            space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue))
        context.setFillColor(CGColor(gray: value, alpha: 1)); context.fill(CGRect(x: 0, y: 0, width: 16, height: 16))
        let image = try XCTUnwrap(context.makeImage()), output = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(output, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return output as Data
    }
}
