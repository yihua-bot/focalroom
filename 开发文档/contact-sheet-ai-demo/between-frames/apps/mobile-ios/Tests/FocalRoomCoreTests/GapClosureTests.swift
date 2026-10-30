import Foundation
import XCTest
import ImageIO
import CoreGraphics
import UniformTypeIdentifiers
@testable import FocalRoomCore

final class GapClosureTests: XCTestCase {
    func testLocalProjectChatsStayIsolatedAndLateReceiptsAreIdempotent() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let scope = AccountScope(server: "https://example.com", userID: "one")
        let store = AIChatStore(root: root, scope: scope)
        let firstID = UUID().uuidString, secondID = UUID().uuidString
        let legacy = try JSONDecoder().decode(AIChatMessage.self, from: Data("""
        {"id":"old","role":"user","text":"旧账户消息","createdAt":0,"state":"sent"}
        """.utf8))
        XCTAssertNil(legacy.localProjectID)
        try await store.append(legacy)
        let first = AIChatMessage(id: "first", role: "user", text: "本机甲", requestID: UUID().uuidString,
                                  state: .unknown, localProjectID: firstID)
        let second = AIChatMessage(id: "second", role: "user", text: "本机乙", requestID: UUID().uuidString,
                                   state: .unknown, localProjectID: secondID)
        try await store.append(first)
        try await store.append(second)
        try await store.complete(first, text: "甲的迟到回复")
        try await store.complete(first, text: "甲的迟到回复")
        let reopened = AIChatStore(root: root, scope: scope)
        let firstMessages = try await reopened.messages(localProjectID: firstID)
        let secondMessages = try await reopened.messages(localProjectID: secondID)
        let legacyMessages = try await reopened.messages(localProjectID: nil)
        XCTAssertEqual(firstMessages.map(\.text), ["本机甲", "甲的迟到回复"])
        XCTAssertEqual(firstMessages.first?.state, .sent)
        XCTAssertEqual(secondMessages.map(\.text), ["本机乙"])
        XCTAssertEqual(secondMessages.first?.state, .unknown)
        XCTAssertEqual(legacyMessages.map(\.id), ["old"])
        let forged = AIChatMessage(id: first.id, role: "user", text: first.text, requestID: first.requestID, localProjectID: secondID)
        do { try await reopened.complete(forged, text: "串项目回复"); XCTFail("回执不能改换消息的本机项目归属") }
        catch APIError.unconfirmedCommand { }
        let other = AIChatStore(root: root, scope: AccountScope(server: scope.server, userID: "two"))
        let otherMessages = try await other.messages(localProjectID: firstID)
        XCTAssertTrue(otherMessages.isEmpty)
        let outbound = AIConversationInput(history: firstMessages.map { ($0.role, $0.text) })
        let json = String(decoding: try JSONEncoder().encode(outbound), as: UTF8.self)
        XCTAssertFalse(json.contains("本机乙"))
        XCTAssertFalse(json.contains("旧账户消息"))
    }

    func testSyncModeIsExplicitAndUnknownModesAreRejected() throws {
        let state = try JSONDecoder().decode(ProjectSyncState.self, from: Data("""
        {"project_id":"project","mode":"PRIVATE_SYNCED","version":2,"authority":"cloud"}
        """.utf8))
        let command = try SyncModeCommand(state: state, shared: true)
        XCTAssertEqual(command.expected_version, 2)
        XCTAssertEqual(command.mode, .SHARED)
        XCTAssertNotNil(UUID(uuidString: command.command_id))
        XCTAssertThrowsError(try JSONDecoder().decode(ProjectSyncState.self, from: Data("""
        {"project_id":"project","mode":"future-unknown","version":2,"authority":"cloud"}
        """.utf8)))
    }

    func testVisionPreviewBoundsAndMetadataRemoval() async throws {
        let color = try XCTUnwrap(CGColorSpace(name: CGColorSpace.sRGB))
        let context = try XCTUnwrap(CGContext(data: nil, width: 2048, height: 1200, bitsPerComponent: 8,
                                            bytesPerRow: 0, space: color, bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue))
        context.setFillColor(CGColor(red: 0.2, green: 0.4, blue: 0.8, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 2048, height: 1200))
        let bytes = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(bytes, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, try XCTUnwrap(context.makeImage()), [
            kCGImagePropertyGPSDictionary: [kCGImagePropertyGPSLatitude: 31.2, kCGImagePropertyGPSLatitudeRef: "N"],
            kCGImagePropertyExifDictionary: [kCGImagePropertyExifUserComment: "private source note"]
        ] as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        let source = try XCTUnwrap(CGImageSourceCreateWithData(bytes, nil))
        let sourceProperties = try XCTUnwrap(CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any])
        XCTAssertNotNil(sourceProperties[kCGImagePropertyGPSDictionary])
        let preview = try VisionPreview.make(jpeg: bytes as Data)
        XCTAssertEqual(preview.width, 1400)
        XCTAssertLessThanOrEqual(preview.height, 1400)
        let output = try XCTUnwrap(CGImageSourceCreateWithData(preview.jpeg as CFData, nil))
        let properties = try XCTUnwrap(CGImageSourceCopyPropertiesAtIndex(output, 0, nil) as? [CFString: Any])
        XCTAssertNil(properties[kCGImagePropertyGPSDictionary])
        XCTAssertNil((properties[kCGImagePropertyExifDictionary] as? [CFString: Any])?[kCGImagePropertyExifUserComment])
        XCTAssertThrowsError(try VisionPreview.make(jpeg: Data("not an image".utf8)))
        XCTAssertThrowsError(try VisionPreview.make(jpeg: Data(count: 16 * 1024 * 1024 + 1)))
        let input = AIConversationInput(history: [("user", "previous"), ("assistant", "reply"), ("user", "this time")], images: [preview])
        let json = try XCTUnwrap(JSONSerialization.jsonObject(with: JSONEncoder().encode(input)) as? [String: Any])
        let messages = try XCTUnwrap(json["input"] as? [[String: Any]])
        XCTAssertEqual((messages[0]["content"] as? [[String: Any]])?.count, 1)
        XCTAssertEqual((messages[2]["content"] as? [[String: Any]])?.count, 2, "仅当前用户消息附图，历史不重传图片")
        let transport = GapRecordingTransport()
        let client = APIClient(configuration: try APIConfiguration("https://example.com"), transport: transport)
        let requestID = UUID().uuidString
        let token = String(repeating: "a", count: 64)
        do { _ = try await client.aiSubmit(input: input, requestID: requestID, token: token); XCTFail("缺少本次授权不得发图") }
        catch APIError.permissionDenied { }
        let before = await transport.requests
        XCTAssertTrue(before.isEmpty)
        do { _ = try await client.aiSubmit(input: input, requestID: requestID, token: token, imageConsent: true); XCTFail() }
        catch is URLError { }
        let sent = await transport.requests
        XCTAssertEqual(sent.count, 1)
        XCTAssertEqual(sent[0].value(forHTTPHeaderField: "x-focalroom-image-consent"), "granted")
    }

    func testCanvasOutboxRestartIsolationAndReceiptBinding() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let config = try APIConfiguration("https://example.com")
        let scope = AccountScope(configuration: config, userID: "one")
        var store = ScopedStore(root: root, scope: scope)
        let board = Board(id: UUID().uuidString, project_id: "project", title: "board", state: "draft", revision: 4)
        let nodeID = UUID().uuidString
        let command = try CanvasMoveCommand(board: board, nodeID: nodeID, x: 10, y: -8)
        try await store.prepareCanvas(boardID: board.id, command: command)
        store = ScopedStore(root: root, scope: scope)
        let pending = try await store.pendingCanvas(boardID: board.id)
        XCTAssertEqual(pending?.command, command)
        do { try await store.prepareCanvas(boardID: board.id, command: command); XCTFail("不能覆盖未确认命令") } catch APIError.unconfirmedCommand { }
        let other = ScopedStore(root: root, scope: AccountScope(configuration: config, userID: "two"))
        let isolated = try await other.pendingCanvas(boardID: board.id)
        XCTAssertNil(isolated)
        do {
            try await store.acknowledgeCanvas(boardID: board.id, receipt: .init(ok: true, commandId: command.commandId, newRevision: 5, affectedNodeIds: []))
            XCTFail("不完整回执不能清除待确认")
        } catch APIError.unconfirmedCommand { }
        try await store.acknowledgeCanvas(boardID: board.id, receipt: .init(ok: true, commandId: command.commandId, newRevision: 5, affectedNodeIds: [nodeID]))
        let cleared = try await store.pendingCanvas(boardID: board.id)
        XCTAssertNil(cleared)
        XCTAssertThrowsError(try CanvasMoveCommand(board: board, nodeID: nodeID, x: .infinity, y: 0))
    }
}

private actor GapRecordingTransport: HTTPTransport {
    private(set) var requests: [URLRequest] = []
    func send(_ request: URLRequest, limit: Int) async throws -> HTTPResponse {
        requests.append(request)
        throw URLError(.notConnectedToInternet)
    }
}

// 画布复杂节点内容解码：文字类节点的文本与样式底色形状（与服务端 content_json/style_json 对齐）。
final class BoardNodeContentDecodingTests: XCTestCase {
    func testDecodesTextContentAndStyleColor() throws {
        let json = Data("""
        {"id":"n1","board_id":"b1","node_type":"sticky","x":0,"y":0,"width":200,"height":120,
         "rotation":0,"z_order":1,"photo_id":null,"visible":true,
         "content_json":{"text":"节奏"},"style_json":{"color":"#ffe082"}}
        """.utf8)
        let node = try JSONDecoder().decode(BoardNode.self, from: json)
        XCTAssertEqual(node.content_json?.text, "节奏")
        XCTAssertEqual(node.style_json?.color, "#ffe082")
        let bare = try JSONDecoder().decode(BoardNode.self, from: Data("""
        {"id":"n2","board_id":"b1","node_type":"photo","x":0,"y":0,"width":100,"height":80,
         "rotation":0,"z_order":0,"photo_id":"p1","visible":true}
        """.utf8))
        XCTAssertNil(bare.content_json)
        XCTAssertNil(bare.style_json)
    }
}
