import Foundation
import XCTest
import ImageIO
import CoreGraphics
import UniformTypeIdentifiers
@testable import FocalRoomCore

final class GapClosureTests: XCTestCase {
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
