import Foundation
import XCTest
import CryptoKit
import ImageIO
import CoreGraphics
import UniformTypeIdentifiers
@testable import FocalRoomCore

@MainActor
final class LiveAPIIntegrationTests: XCTestCase {
    func testRealBackendLoginMediaLostReceiptReplayAndLogout() async throws {
        guard let base = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_BASE_URL"] else {
            throw XCTSkip("独立本地后端集成测试需通过 scripts/verify-live.mjs 启动")
        }
        let configuration = try APIConfiguration(base, allowLoopbackHTTP: true)
        XCTAssertEqual(configuration.origin.host, "127.0.0.1")
        let client = APIClient(configuration: configuration)
        let credentials = try await client.login(email: "iphone@fr.test", password: "iphone-local-test-2026")
        let me = try await client.currentUser(token: credentials.token)
        XCTAssertEqual(me, credentials.user)
        let projects = try await client.projects(token: credentials.token)
        XCTAssertEqual(projects.count, 1)
        let project = try XCTUnwrap(projects.first)
        let snapshot = try await client.snapshot(projectID: project.id, credentials: credentials)
        XCTAssertEqual(snapshot.snapshot.photos.count, 3)
        XCTAssertEqual(snapshot.comments.first?.body, "iPhone 本地隔离测试批注")
        let preview = try await client.preview(projectID: project.id, descriptor: XCTUnwrap(snapshot.previews.first), token: credentials.token)
        XCTAssertGreaterThan(preview.count, 100)
        let boards = try await client.boards(projectID: project.id, token: credentials.token)
        let board = try XCTUnwrap(boards.first)
        let nodes = try await client.nodes(boardID: board.id, token: credentials.token)
        XCTAssertEqual(nodes.first?.photo_id, "ios-photo-1")
        let results = try await client.agentResults(projectID: project.id, token: credentials.token)
        XCTAssertTrue(results.isEmpty, "不得以模型夹具伪造已完成的 AI 任务")

        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let scope = AccountScope(configuration: configuration, userID: credentials.user.id)
        let store = ScopedStore(root: root, scope: scope)
        let command = try SelectionCommand(snapshot: snapshot, photoID: "ios-photo-1", selection: .KEEP)
        try await store.savePending(PendingSelection(command: command))
        let lossy = APIClient(configuration: configuration, transport: LoseFirstCommandResponse())
        do { _ = try await lossy.select(command, token: credentials.token); XCTFail("应模拟丢失回执") }
        catch let error as URLError { XCTAssertEqual(error.code, .networkConnectionLost) }
        let afterLostResponse = try await client.snapshot(projectID: project.id, credentials: credentials)
        XCTAssertEqual(afterLostResponse.revision, snapshot.revision + 1)

        let reopened = ScopedStore(root: root, scope: scope)
        let saved = try await reopened.pending(projectID: project.id)
        let original = try XCTUnwrap(saved?.command)
        XCTAssertEqual(command, original)
        let receipt = try await client.select(original, token: credentials.token)
        let repeated = try await client.select(original, token: credentials.token)
        XCTAssertEqual(receipt.command_id, repeated.command_id)
        XCTAssertEqual(receipt.event_id, repeated.event_id)
        let afterRetry = try await client.snapshot(projectID: project.id, credentials: credentials)
        XCTAssertEqual(afterRetry.revision, afterLostResponse.revision, "原编号重试不得再次推进版本")
        try await reopened.acknowledge(receipt, command: original)
        let acknowledged = try await reopened.pending(projectID: project.id)
        XCTAssertNil(acknowledged)

        // 使用有效正式命令验证服务端角色保护，不能只测试 UI 按钮隐藏。
        let viewer = try await client.login(email: "viewer@fr.test", password: "iphone-local-test-2026")
        let readOnly = try await client.snapshot(projectID: project.id, credentials: viewer)
        XCTAssertFalse(readOnly.principal.permitsSelection)
        let forbidden = try SelectionCommand(snapshot: afterRetry, photoID: "ios-photo-1", selection: .REJECT)
        do { _ = try await client.select(forbidden, token: viewer.token); XCTFail("Viewer 不得提交正式命令") }
        catch APIError.server(let status, _) { XCTAssertEqual(status, 403) }
        try await client.logout(token: credentials.token)
        do { _ = try await client.currentUser(token: credentials.token); XCTFail("退出后不能恢复旧会话") }
        catch APIError.server(let status, _) { XCTAssertEqual(status, 401) }
    }

    // 命名保证在本类首个联调用例之后运行，避免共享夹具的项目计数断言被本轮新增项目影响。
    func testRealBackendMigrationResumeAndCursorSync() async throws {
        guard let base = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_BASE_URL"] else {
            throw XCTSkip("独立本地后端集成测试需通过 scripts/verify-live.mjs 启动")
        }
        let configuration = try APIConfiguration(base, allowLoopbackHTTP: true)
        let client = APIClient(configuration: configuration)
        let credentials = try await client.login(email: "iphone@fr.test", password: "iphone-local-test-2026")
        let workspaces = try await client.workspaces(token: credentials.token)
        let workspace = try XCTUnwrap(workspaces.first(where: \.permitsPromotion))

        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let root = directory.appendingPathComponent("library")
        var library = LocalLibrary(root: root, scope: AccountScope(configuration: configuration, userID: credentials.user.id))
        let project = try await library.createProject(title: "本机迁云联调")
        var photoIDs: [String] = []
        for index in 0..<2 {
            let source = directory.appendingPathComponent("migrate-\(index).jpg")
            try jpeg(seed: index).write(to: source)
            guard case .imported(let photo) = try await library.importFile(source, projectID: project.id) else { return XCTFail() }
            photoIDs.append(photo.id)
        }
        try await library.select(projectID: project.id, photoID: photoIDs[0], selection: .KEEP)

        let record = try await library.beginMigration(projectID: project.id, workspaceID: workspace.id)
        let start = try await client.promote(manifestJSON: record.manifestJSON, token: credentials.token)
        try await library.migrationEnrolled(projectID: project.id, cloudProjectID: start.project_id,
                                            manifestDigest: start.manifest_digest)

        // 模拟上传途中断网：第一次预览上传丢失响应，记录保留 enrolling→uploading 状态与空上传清单。
        let lossy = APIClient(configuration: configuration, transport: DropPreviewUploads())
        let payloadOne = try await library.uploadPayload(projectID: project.id, photoID: photoIDs[0])
        do {
            _ = try await lossy.uploadPreview(projectID: start.project_id, photoID: photoIDs[0], jpeg: payloadOne, token: credentials.token)
            XCTFail("应模拟上传丢失")
        } catch let error as URLError { XCTAssertEqual(error.code, .networkConnectionLost) }

        // 进程重启（新建 actor 实例）后凭记录续跑：同一字节重发 promote 取回同一云端项目。
        library = LocalLibrary(root: root, scope: AccountScope(configuration: configuration, userID: credentials.user.id))
        let reopened = try await library.projects()
        let resumed = try XCTUnwrap(reopened.first?.migration)
        XCTAssertEqual(resumed.phase, .uploading)
        XCTAssertEqual(resumed.manifestJSON, record.manifestJSON)
        XCTAssertEqual(resumed.cloudProjectID, start.project_id)
        let replay = try await client.promote(manifestJSON: resumed.manifestJSON, token: credentials.token)
        XCTAssertEqual(replay.project_id, start.project_id, "重试不得创建第二个云端项目")
        XCTAssertEqual(replay.manifest_digest, start.manifest_digest)

        for id in photoIDs {
            let payload = try await library.uploadPayload(projectID: project.id, photoID: id)
            _ = try await client.uploadPreview(projectID: start.project_id, photoID: id, jpeg: payload, token: credentials.token)
            try await library.markUploaded(projectID: project.id, photoID: id)
        }
        _ = try await client.finalizePromotion(projectID: start.project_id, manifestDigest: start.manifest_digest, token: credentials.token)
        try await library.completeMigration(projectID: project.id)

        do { try await library.select(projectID: project.id, photoID: photoIDs[1], selection: .KEEP); XCTFail("迁云后本机写入仍冻结") }
        catch LocalLibraryError.projectFrozen { }

        let migrated = try await client.projects(token: credentials.token).first { $0.id == start.project_id }
        XCTAssertEqual(migrated?.state, .SHARED)
        let snapshot = try await client.snapshot(projectID: start.project_id, credentials: credentials)
        XCTAssertEqual(snapshot.snapshot.photos.map(\.id), photoIDs)
        XCTAssertEqual(snapshot.snapshot.photos.map(\.selection), [.KEEP, .UNDECIDED])
        XCTAssertEqual(snapshot.snapshot.sequences.flatMap(\.items).map(\.photo_id), [photoIDs[0]], "云端序列只承载本机保留照片")
        XCTAssertEqual(snapshot.previews.count, 2)

        // 迁云不隐式授权其他账户。
        let viewer = try await client.login(email: "viewer@fr.test", password: "iphone-local-test-2026")
        do { _ = try await client.snapshot(projectID: start.project_id, credentials: viewer); XCTFail("迁云不得向未授权账户开放") }
        catch APIError.server(let status, _) { XCTAssertEqual(status, 403) }

        // 事件游标：另一设备提交命令后，按 after=cursor 拉到增量才刷新快照。
        let cursor = try XCTUnwrap(Int(snapshot.cursor))
        let command = try SelectionCommand(snapshot: snapshot, photoID: photoIDs[1], selection: .REJECT)
        _ = try await client.select(command, token: credentials.token)
        var after = cursor
        var pulled: [ProjectEvent] = []
        while true {
            let page = try await client.projectEvents(projectID: start.project_id, after: after, token: credentials.token)
            pulled.append(contentsOf: page.events)
            if let next = page.nextCursor { after = next } else { break }
        }
        XCTAssertEqual(pulled.map(\.kind), ["ProjectVersionCommitted"])
        XCTAssertEqual(pulled.first?.revision, snapshot.revision + 1)
        let fresh = try await client.snapshot(projectID: start.project_id, credentials: credentials)
        XCTAssertEqual(fresh.revision, snapshot.revision + 1)
        XCTAssertEqual(fresh.snapshot.photos.first { $0.id == photoIDs[1] }?.selection, .REJECT)
        let drained = try await client.projectEvents(projectID: start.project_id, after: Int(fresh.cursor)!, token: credentials.token)
        XCTAssertTrue(drained.events.isEmpty && drained.nextCursor == nil, "追平后不再有增量")
    }

    // 命名保持在本类其他联调用例之后运行，避免共享夹具的项目计数断言被新增项目影响。
    func testRealBackendSessionDevicesAndReceiptQuery() async throws {
        guard let base = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_BASE_URL"] else {
            throw XCTSkip("独立本地后端集成测试需通过 scripts/verify-live.mjs 启动")
        }
        let configuration = try APIConfiguration(base, allowLoopbackHTTP: true)
        let client = APIClient(configuration: configuration)
        let credentials = try await client.login(email: "iphone@fr.test", password: "iphone-local-test-2026")
        let deviceKey = UUID().uuidString.lowercased()

        // 幂等注册：同 key 同设备；绑定后列表标记当前会话。
        let registered = try await client.registerDevice(key: deviceKey, name: "iPhone 联调", platform: "ios", token: credentials.token)
        let reregistered = try await client.registerDevice(key: deviceKey, name: "iPhone 联调", platform: "ios", token: credentials.token)
        XCTAssertEqual(registered.id, reregistered.id)
        try await client.bindDevice(id: registered.id, token: credentials.token)
        let listed = try await client.devices(token: credentials.token)
        let own = try XCTUnwrap(listed.first { $0.id == registered.id })
        XCTAssertTrue(own.current)

        // UNKNOWN 恢复：提交丢回执后按原编号查回执，确认已提交且不重发。
        // 使用迁云项目验证，避免推进联调夹具项目的正式版本计数。
        let projects = try await client.projects(token: credentials.token)
        let project = try XCTUnwrap(projects.first { $0.title == "本机迁云联调" && $0.state == .SHARED })
        let snapshot = try await client.snapshot(projectID: project.id, credentials: credentials)
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let store = ScopedStore(root: root, scope: AccountScope(configuration: configuration, userID: credentials.user.id))
        let target = try XCTUnwrap(snapshot.snapshot.photos.first { $0.selection == .UNDECIDED } ?? snapshot.snapshot.photos.first)
        let command = try SelectionCommand(snapshot: snapshot, photoID: target.id, selection: .REJECT)
        try await store.savePending(PendingSelection(command: command))
        let lossy = APIClient(configuration: configuration, transport: LoseFirstCommandResponse())
        do { _ = try await lossy.select(command, token: credentials.token); XCTFail("应模拟丢失回执") }
        catch let error as URLError { XCTAssertEqual(error.code, .networkConnectionLost) }
        let receipt = try await client.commandReceipt(projectID: project.id, commandID: command.command_id, token: credentials.token)
        let confirmed = try XCTUnwrap(receipt, "丢回执后权威端应可查到回执")
        try confirmed.validate(for: command)
        try await store.acknowledge(confirmed, command: command)
        let acknowledged = try await store.pending(projectID: project.id)
        XCTAssertNil(acknowledged)
        let after = try await client.snapshot(projectID: project.id, credentials: credentials)
        XCTAssertEqual(after.revision, snapshot.revision + 1, "回执确认路径不得再次推进版本")
        let unknown = try await client.commandReceipt(projectID: project.id, commandID: UUID().uuidString, token: credentials.token)
        XCTAssertNil(unknown, "未知编号必须返回无回执")

        // 撤销设备：其会话立即失效，另一会话不受影响。
        let second = try await client.login(email: "iphone@fr.test", password: "iphone-local-test-2026")
        let secondDevice = try await client.registerDevice(key: UUID().uuidString.lowercased(), name: "Mac 联调", platform: "mac", token: second.token)
        try await client.bindDevice(id: secondDevice.id, token: second.token)
        try await client.revokeDevice(id: registered.id, token: second.token)
        do { _ = try await client.currentUser(token: credentials.token); XCTFail("被撤销设备的会话应失效") }
        catch APIError.server(let status, _) { XCTAssertEqual(status, 401) }
        _ = try await client.currentUser(token: second.token)
    }

    // 命名保持在本类其他联调用例之后运行（L < M < R < S），不影响共享夹具的项目计数断言。
    func testRealBackendResearchReferencesCRUD() async throws {
        guard let base = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_BASE_URL"],
              let projectID = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_PROJECT_ID"] else {
            throw XCTSkip("独立本地后端集成测试需通过 scripts/verify-live.mjs 启动")
        }
        let configuration = try APIConfiguration(base, allowLoopbackHTTP: true)
        let client = APIClient(configuration: configuration)
        let credentials = try await client.login(email: "iphone@fr.test", password: "iphone-local-test-2026")

        let initial = try await client.research(projectID: projectID, token: credentials.token)
        XCTAssertTrue(initial.isEmpty, "夹具项目初始不应有研究资料")

        let noteDraft = try ResearchDraft(kind: .note, title: "iPhone 研究笔记", body: "盲页开场节奏参考。", author: "内部整理")
        let note = try await client.createResearch(projectID: projectID, draft: noteDraft, token: credentials.token)
        XCTAssertEqual(note.kind, "note")
        XCTAssertEqual(note.title, "iPhone 研究笔记")
        let linkDraft = try ResearchDraft(kind: .link, title: "参考链接", url: "https://example.com/reference")
        let link = try await client.createResearch(projectID: projectID, draft: linkDraft, token: credentials.token)
        XCTAssertEqual(link.kind, "link")

        let listed = try await client.research(projectID: projectID, token: credentials.token)
        XCTAssertEqual(listed.map(\.id), [note.id, link.id], "按创建时间排序")
        XCTAssertEqual(listed.first?.created_by_name, credentials.user.name)

        let patch = try ResearchPatch(title: "iPhone 研究笔记（更新）", license: "内部参考")
        let updated = try await client.updateResearch(projectID: projectID, referenceID: note.id, patch: patch, token: credentials.token)
        XCTAssertEqual(updated.title, "iPhone 研究笔记（更新）")
        XCTAssertEqual(updated.license, "内部参考")

        let viewer = try await client.login(email: "viewer@fr.test", password: "iphone-local-test-2026")
        let viewerDraft = try ResearchDraft(kind: .note, title: "越权", body: "不应成功")
        do { _ = try await client.createResearch(projectID: projectID, draft: viewerDraft, token: viewer.token); XCTFail("Viewer 不能添加研究参考") }
        catch APIError.server(let status, _) { XCTAssertEqual(status, 403) }

        try await client.deleteResearch(projectID: projectID, referenceID: link.id, token: credentials.token)
        let remaining = try await client.research(projectID: projectID, token: credentials.token)
        XCTAssertEqual(remaining.map(\.id), [note.id], "删除链接后只剩笔记")
        // 清理本人笔记，后续 UI 旅程以空列表起测。
        try await client.deleteResearch(projectID: projectID, referenceID: note.id, token: credentials.token)
        let cleared = try await client.research(projectID: projectID, token: credentials.token)
        XCTAssertTrue(cleared.isEmpty)
    }

    // 命名保持在本类其他联调用例之后运行（… < U），并为本轮 UI 旅程留下一条 SUBMITTED 提案。
    func testRealBackendUnifiedCanvasAndProposals() async throws {
        guard let base = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_BASE_URL"],
              let projectID = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_PROJECT_ID"] else {
            throw XCTSkip("独立本地后端集成测试需通过 scripts/verify-live.mjs 启动")
        }
        let configuration = try APIConfiguration(base, allowLoopbackHTTP: true)
        let client = APIClient(configuration: configuration)
        let credentials = try await client.login(email: "iphone@fr.test", password: "iphone-local-test-2026")
        let token = credentials.token

        // 画布：移动命令推进 Board 修订；过期基线被权威拒绝。
        let boards = try await client.boards(projectID: projectID, token: token)
        let board = try XCTUnwrap(boards.first { $0.isDraft })
        XCTAssertGreaterThanOrEqual(board.revision, 1)
        let nodes = try await client.nodes(boardID: board.id, token: token)
        let photoNode = try XCTUnwrap(nodes.first { $0.photo_id != nil })
        let move = try CanvasMoveCommand(board: board, nodeID: photoNode.id, x: photoNode.x + 40, y: photoNode.y + 20)
        let receipt = try await client.submitCanvasCommand(boardID: board.id, command: move, token: token)
        XCTAssertEqual(receipt.newRevision, board.revision + 1)
        XCTAssertEqual(receipt.affectedNodeIds, [photoNode.id])
        let queried = try await client.canvasReceipt(boardID: board.id, command: move, token: token)
        XCTAssertEqual(queried, receipt, "只读查询恢复原回执，不再次执行命令")
        let afterQuery = try await client.boards(projectID: projectID, token: token)
        XCTAssertEqual(afterQuery.first { $0.id == board.id }?.revision, receipt.newRevision)
        let refreshed = try await client.nodes(boardID: board.id, token: token)
        let moved = try XCTUnwrap(refreshed.first { $0.id == photoNode.id })
        XCTAssertEqual(moved.x, photoNode.x + 40, accuracy: 0.001)
        let stale = try CanvasMoveCommand(board: board, nodeID: photoNode.id, x: 0, y: 0)
        do { _ = try await client.submitCanvasCommand(boardID: board.id, command: stale, token: token); XCTFail("过期基线应冲突") }
        catch APIError.server(let status, let code) {
            XCTAssertEqual(status, 409)
            XCTAssertEqual(code, "REVISION_CONFLICT")
        }

        // 提案一（move + remove）：全量采纳，作品序列与正式版本随之变化。
        var snapshot = try await client.snapshot(projectID: projectID, credentials: credentials)
        let baseRevision = snapshot.revision
        func envelope(kind: String, expectedRevision: Int, payload: [String: Any]) throws -> (id: String, body: Data) {
            let id = UUID().uuidString.lowercased()
            let dict: [String: Any] = ["version": "2.0", "command_id": id, "idempotency_key": id,
                                       "workspace_id": snapshot.workspace_id, "project_id": projectID,
                                       "expected_revision": expectedRevision, "kind": kind, "payload": payload]
            return (id, try JSONSerialization.data(withJSONObject: dict))
        }
        let opMove = UUID().uuidString.lowercased(), opRemove = UUID().uuidString.lowercased()
        let create1 = try envelope(kind: "proposal.create", expectedRevision: baseRevision, payload: [
            "title": "联调整理提案", "base_revision": baseRevision,
            "operations": [["op": "move", "operation_id": opMove, "photo_id": "ios-photo-2", "to_index": 2],
                           ["op": "remove", "operation_id": opRemove, "photo_id": "ios-photo-3"]],
        ])
        _ = try await client.submitProjectCommandBody(projectID: projectID, body: create1.body, token: token)
        let submit1 = try envelope(kind: "proposal.submit", expectedRevision: baseRevision, payload: [
            "proposal_id": create1.id, "expected_proposal_revision": 0,
        ])
        _ = try await client.submitProjectCommandBody(projectID: projectID, body: submit1.body, token: token)
        snapshot = try await client.snapshot(projectID: projectID, credentials: credentials)
        let submitted1 = try XCTUnwrap(snapshot.proposals?.first { $0.id == create1.id && $0.isSubmitted })
        XCTAssertEqual(submitted1.operations.count, 2)
        XCTAssertFalse(submitted1.nonce.isEmpty)
        let applied = try ProposalApplyCommand(snapshot: snapshot, proposal: submitted1,
                                               acceptedOperationIDs: [opMove, opRemove])
        _ = try await client.applyProposal(applied, token: token)
        snapshot = try await client.snapshot(projectID: projectID, credentials: credentials)
        XCTAssertEqual(snapshot.revision, baseRevision + 1)
        XCTAssertEqual(snapshot.snapshot.sequences.flatMap(\.items).map(\.photo_id), ["ios-photo-1", "ios-photo-2"])
        XCTAssertEqual(snapshot.proposals?.first { $0.id == create1.id }?.state, "APPLIED")

        // 提案二（add）：整体驳回，不推进正式版本。
        let opAdd2 = UUID().uuidString.lowercased()
        let create2 = try envelope(kind: "proposal.create", expectedRevision: baseRevision + 1, payload: [
            "title": "驳回路径提案", "base_revision": baseRevision + 1,
            "operations": [["op": "add", "operation_id": opAdd2, "photo_id": "ios-photo-3", "to_index": 0]],
        ])
        _ = try await client.submitProjectCommandBody(projectID: projectID, body: create2.body, token: token)
        let submit2 = try envelope(kind: "proposal.submit", expectedRevision: baseRevision + 1, payload: [
            "proposal_id": create2.id, "expected_proposal_revision": 0,
        ])
        _ = try await client.submitProjectCommandBody(projectID: projectID, body: submit2.body, token: token)
        snapshot = try await client.snapshot(projectID: projectID, credentials: credentials)
        let submitted2 = try XCTUnwrap(snapshot.proposals?.first { $0.id == create2.id && $0.isSubmitted })
        let rejected = try ProposalApplyCommand(snapshot: snapshot, proposal: submitted2, acceptedOperationIDs: [])
        _ = try await client.applyProposal(rejected, token: token)
        snapshot = try await client.snapshot(projectID: projectID, credentials: credentials)
        XCTAssertEqual(snapshot.revision, baseRevision + 1, "驳回不得推进正式版本")
        XCTAssertEqual(snapshot.proposals?.first { $0.id == create2.id }?.state, "REJECTED")

        // 提案三（add + move）：保留 SUBMITTED 供 UI 旅程做部分采纳。
        let opAdd3 = UUID().uuidString.lowercased(), opMove3 = UUID().uuidString.lowercased()
        let create3 = try envelope(kind: "proposal.create", expectedRevision: baseRevision + 1, payload: [
            "title": "UI 部分采纳提案", "base_revision": baseRevision + 1,
            "operations": [["op": "add", "operation_id": opAdd3, "photo_id": "ios-photo-3", "to_index": 2],
                           ["op": "move", "operation_id": opMove3, "photo_id": "ios-photo-1", "to_index": 1]],
        ])
        _ = try await client.submitProjectCommandBody(projectID: projectID, body: create3.body, token: token)
        let submit3 = try envelope(kind: "proposal.submit", expectedRevision: baseRevision + 1, payload: [
            "proposal_id": create3.id, "expected_proposal_revision": 0,
        ])
        _ = try await client.submitProjectCommandBody(projectID: projectID, body: submit3.body, token: token)
        snapshot = try await client.snapshot(projectID: projectID, credentials: credentials)
        XCTAssertEqual(snapshot.proposals?.first { $0.id == create3.id }?.state, "SUBMITTED")
        XCTAssertEqual(snapshot.revision, baseRevision + 1)
    }

    // 命名保持在本类其他联调用例之后（U 组内先于画布用例，互不依赖项目修订）。
    func testRealBackendUnifiedAIConversation() async throws {
        guard let base = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_BASE_URL"],
              let projectID = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_PROJECT_ID"] else {
            throw XCTSkip("独立本地后端集成测试需通过 scripts/verify-live.mjs 启动")
        }
        let configuration = try APIConfiguration(base, allowLoopbackHTTP: true)
        let client = APIClient(configuration: configuration)
        let credentials = try await client.login(email: "iphone@fr.test", password: "iphone-local-test-2026")
        let token = credentials.token

        let account = try await client.aiAccount(token: token)
        XCTAssertTrue(account.enabled, "夹具已配置模型桩")
        XCTAssertTrue(account.verified, "夹具账户已验证邮箱")
        XCTAssertGreaterThan(account.available, 0)
        XCTAssertEqual(account.provider, "DeepSeek")

        // 异步提交 + 轮询：结果留存在服务端，凭同一 requestID 可恢复。
        let requestID = UUID().uuidString.lowercased()
        let input = AIConversationInput(history: [("user", "帮我看看选片节奏")])
        _ = try await client.aiSubmit(input: input, requestID: requestID, token: token)
        var job = try await client.aiJob(requestID: requestID, token: token)
        var attempts = 0
        while job.state == "PENDING" && attempts < 20 {
            try await Task.sleep(nanoseconds: 300_000_000)
            job = try await client.aiJob(requestID: requestID, token: token)
            attempts += 1
        }
        XCTAssertEqual(job.state, "COMPLETED", "桩传输应很快完成")
        let result = try XCTUnwrap(job.result, "异步路径应留存结果供恢复")
        XCTAssertTrue(result.text.hasPrefix("桩回复："), "应收到确定性桩回复，实际：\(result.text)")
        XCTAssertEqual(result.usage.total_tokens, 20)

        // 同号重提：返回当前任务状态，不重复调用模型、不报冲突。
        let replay = try await client.aiSubmit(input: input, requestID: requestID, token: token)
        XCTAssertEqual(replay.state, "COMPLETED")
        XCTAssertEqual(replay.result?.id, result.id)
        let afterReplay = try await client.aiAccount(token: token)
        XCTAssertEqual(afterReplay.spent, account.spent + 20, "重提不得重复计费")

        // 真正经过图片校验与额度事务；提交响应丢失后只 GET 原编号，不重传像素。
        let preview = try VisionPreview.make(jpeg: jpeg(seed: 1))
        let visionInput = AIConversationInput(history: [("user", "观察测试预览")], images: [preview])
        let visionID = UUID().uuidString.lowercased()
        let lossyTransport = LoseAIResponse()
        let lossy = APIClient(configuration: configuration, transport: lossyTransport)
        do {
            _ = try await lossy.aiSubmit(input: visionInput, requestID: visionID, token: token, imageConsent: true)
            XCTFail("模拟响应丢失")
        } catch is URLError { }
        let didDropResponse = await lossyTransport.didDropResponse
        XCTAssertTrue(didDropResponse, "必须实际丢弃成功提交响应，再仅凭原编号恢复")
        var visionJob = try await client.aiJob(requestID: visionID, token: token)
        for _ in 0..<20 where visionJob.state == "PENDING" {
            try await Task.sleep(nanoseconds: 300_000_000)
            visionJob = try await client.aiJob(requestID: visionID, token: token)
        }
        XCTAssertEqual(visionJob.state, "COMPLETED")
        XCTAssertTrue(visionJob.result?.text.contains("观察 1 张预览") == true)
        let afterVision = try await client.aiAccount(token: token)
        XCTAssertEqual(afterVision.spent, afterReplay.spent + 20)

        let jobs = try await client.agentJobs(projectID: projectID, token: token)
        XCTAssertTrue(jobs.isEmpty, "夹具项目初始无分析任务")
    }

    // 命名保持在本类其他联调用例之后（U < V），此时夹具项目已完成选片与提案采纳各一次。
    func testRealBackendVersionHistoryAndBilling() async throws {
        guard let base = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_BASE_URL"],
              let projectID = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_PROJECT_ID"] else {
            throw XCTSkip("独立本地后端集成测试需通过 scripts/verify-live.mjs 启动")
        }
        let configuration = try APIConfiguration(base, allowLoopbackHTTP: true)
        let client = APIClient(configuration: configuration)
        let credentials = try await client.login(email: "iphone@fr.test", password: "iphone-local-test-2026")
        let token = credentials.token

        let snapshot = try await client.snapshot(projectID: projectID, credentials: credentials)
        let history = try await client.projectHistory(projectID: projectID, token: token)
        XCTAssertGreaterThanOrEqual(history.count, 3, "迁云初始版 + 选片 + 提案采纳至少三版")
        XCTAssertEqual(history.map(\.revision), history.map(\.revision).sorted(by: >), "按版本倒序")
        XCTAssertTrue(history.contains { $0.revision == snapshot.revision }, "历史应包含当前版本")
        XCTAssertTrue(history.contains { $0.revision == 0 }, "迁云初始版本应在历史中")

        let diff = try await client.versionDiff(projectID: projectID, from: 0, to: snapshot.revision, token: token)
        XCTAssertEqual(diff.from.revision, 0)
        XCTAssertEqual(diff.to.revision, snapshot.revision)
        let selectionChange = try XCTUnwrap(
            diff.photos.changed.first { $0.id == "ios-photo-1" && $0.fields["selection"] != nil },
            "跨版本差异应包含 ios-photo-1 的选片字段变化")
        XCTAssertEqual(selectionChange.fields["selection"]?.after?.displayText, "KEEP")
        XCTAssertFalse(diff.sequences.changed.isEmpty, "提案采纳改变过序列")

        // 提案投影：联调旅程留下的 SUBMITTED 提案在当前基线上不 stale，投影含新增照片。
        let proposals = try XCTUnwrap(snapshot.proposals)
        let submitted = try XCTUnwrap(proposals.first { $0.isSubmitted }, "联调旅程应留下待审提案")
        let projection = try await client.proposalDiff(projectID: projectID, proposalID: submitted.id, token: token)
        XCTAssertEqual(projection.proposalId, submitted.id)
        XCTAssertFalse(projection.stale, "提案基线与当前版本一致")
        let change = try XCTUnwrap(projection.sequence)
        XCTAssertTrue(change.added.contains("ios-photo-3"), "投影应包含提案新增的照片")
        XCTAssertEqual(change.before, ["ios-photo-1", "ios-photo-2"], "投影前序列为当前权威序列")

        let catalog = try await client.billingCatalog()
        XCTAssertFalse(catalog.salesAvailable, "夹具未配置支付网关")
        XCTAssertEqual(catalog.plans.map(\.id).sorted(), ["free", "pro"])
        XCTAssertTrue(catalog.plans.allSatisfy { $0.prices.isEmpty }, "未开放销售时价格恒为空")
    }

    private func jpeg(seed: Int) throws -> Data {
        let context = try XCTUnwrap(CGContext(data: nil, width: 160, height: 120, bitsPerComponent: 8, bytesPerRow: 0,
                                              space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        context.setFillColor(CGColor(red: Double(seed) * 0.3, green: 0.5, blue: 0.8, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 160, height: 120))
        let image = try XCTUnwrap(context.makeImage())
        let data = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }
}

private actor LoseFirstCommandResponse: HTTPTransport {
    private var dropped = false
    private let underlying = URLSessionTransport()
    func send(_ request: URLRequest, limit: Int) async throws -> HTTPResponse {
        let response = try await underlying.send(request, limit: limit)
        if request.url?.path.hasSuffix("/commands") == true && !dropped {
            dropped = true
            guard response.status == 200 else { throw APIError.invalidResponse }
            throw URLError(.networkConnectionLost)
        }
        return response
    }
}

private actor DropPreviewUploads: HTTPTransport {
    private let underlying = URLSessionTransport()
    func send(_ request: URLRequest, limit: Int) async throws -> HTTPResponse {
        let response = try await underlying.send(request, limit: limit)
        if request.httpMethod == "PUT", request.url?.path.contains("/previews/") == true {
            throw URLError(.networkConnectionLost)
        }
        return response
    }
}

private actor LoseAIResponse: HTTPTransport {
    private let underlying = URLSessionTransport()
    private(set) var didDropResponse = false
    func send(_ request: URLRequest, limit: Int) async throws -> HTTPResponse {
        let response = try await underlying.send(request, limit: limit)
        // 异步提交可能返回 202；丢回执演练必须覆盖所有成功响应，而非只拦 200。
        if request.httpMethod == "POST", request.url?.path.hasSuffix("/responses/async") == true,
           (200..<300).contains(response.status) {
            didDropResponse = true
            throw URLError(.networkConnectionLost)
        }
        return response
    }
}
