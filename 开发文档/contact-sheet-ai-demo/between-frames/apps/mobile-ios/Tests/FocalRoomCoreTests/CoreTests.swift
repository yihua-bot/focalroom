import Foundation
import XCTest
@testable import FocalRoomCore

@MainActor
final class CoreTests: XCTestCase {
    func testServerRejectsUntrustedAddressForms() throws {
        for address in ["http://example.com", "https://user:secret@example.com", "https://example.com/path",
                        "https://example.com?token=secret", "https://example.com#fragment", "file:///tmp/data"] {
            XCTAssertThrowsError(try APIConfiguration(address))
        }
        XCTAssertThrowsError(try APIConfiguration("http://192.168.1.2", allowLoopbackHTTP: true))
        XCTAssertThrowsError(try APIConfiguration("http://127.0.0.1"))
        XCTAssertEqual(try APIConfiguration("http://127.0.0.1:19876", allowLoopbackHTTP: true).origin.host, "127.0.0.1")
        XCTAssertEqual(try APIConfiguration("https://example.com/").identity, "https://example.com")
    }

    func testMediaTicketsCannotEscapeOriginOrEndpoint() throws {
        let configuration = try APIConfiguration("https://example.com")
        for path in ["https://evil.test/api/v2/media/" + digest, "//evil.test/api/v2/media/" + digest,
                     "/api/v2/media/../auth/me", "/api/v2/media/" + digest + "?x=1"] {
            XCTAssertThrowsError(try configuration.previewURL(path))
        }
        let url = try configuration.previewURL("/api/v2/media/" + digest)
        XCTAssertEqual(url.host, "example.com")
        for id in ["..", "a/b", "%2e%2e", "a\\b"] {
            XCTAssertThrowsError(try configuration.endpoint(["projects", id, "snapshot"]))
        }
    }

    func testUnknownPhotoStatesAndWrongScopeAreRejected() throws {
        let bytes = fixture(selection: "NEW_UNKNOWN_STATE")
        XCTAssertThrowsError(try JSONDecoder().decode(ProjectSnapshot.self, from: bytes))
        let value = try snapshot()
        XCTAssertThrowsError(try value.validate(projectID: "other-project", userID: "user-1"))
        XCTAssertThrowsError(try value.validate(projectID: "project-1", userID: "other-user"))
        let invalid = String(data: fixture(), encoding: .utf8)!.replacingOccurrences(of: "\"photo_id\":\"photo-1\"", with: "\"photo_id\":\"missing\"")
        XCTAssertThrowsError(try JSONDecoder().decode(ProjectSnapshot.self, from: Data(invalid.utf8)).validate(projectID: "project-1", userID: "user-1"))
    }

    func testReviewerCannotCreateFormalSelection() throws {
        let value = try JSONDecoder().decode(ProjectSnapshot.self, from: fixture(role: "REVIEWER"))
        XCTAssertThrowsError(try SelectionCommand(snapshot: value, photoID: "photo-1", selection: .KEEP))
    }

    func testPendingSurvivesReopenAndCannotBeOverwrittenOrFalselyAcknowledged() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let scope = AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "user-1")
        let store = ScopedStore(root: root, scope: scope)
        let value = try snapshot()
        let command = try SelectionCommand(snapshot: value, photoID: "photo-1", selection: .KEEP)
        try await store.savePending(PendingSelection(command: command))
        let reopened = ScopedStore(root: root, scope: scope)
        let restored = try await reopened.pending(projectID: value.project_id)
        XCTAssertEqual(restored?.command, command)
        let another = try SelectionCommand(snapshot: value, photoID: "photo-1", selection: .REJECT)
        do { try await reopened.savePending(PendingSelection(command: another)); XCTFail("不得覆盖未确认命令") }
        catch APIError.unconfirmedCommand { }
        let falseReceipt = try JSONDecoder().decode(CommandReceipt.self, from: Data("""
        {"ok":true,"command_id":"wrong","new_revision":1,"event_id":"2","affected_ids":["photo-1"]}
        """.utf8))
        do { try await reopened.acknowledge(falseReceipt, command: command); XCTFail("不得采信其他回执") }
        catch APIError.unconfirmedCommand { }
        let stillPending = try await reopened.pending(projectID: value.project_id)
        XCTAssertEqual(stillPending?.command, command)
        do { try await reopened.discardRejected(projectID: value.project_id); XCTFail("未知结果不能当失败丢弃") }
        catch APIError.unconfirmedCommand { }
    }

    func testCacheSeparatesAccountsAndServersAndRevocationClearsProject() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let config = try APIConfiguration("https://example.com")
        let a = ScopedStore(root: root, scope: AccountScope(configuration: config, userID: "user-1"))
        let b = ScopedStore(root: root, scope: AccountScope(configuration: config, userID: "user-2"))
        let c = ScopedStore(root: root, scope: AccountScope(configuration: try APIConfiguration("https://other.test"), userID: "user-1"))
        try await a.saveSnapshot(snapshot())
        let bSnapshot = try await b.snapshot(projectID: "project-1")
        let cSnapshot = try await c.snapshot(projectID: "project-1")
        XCTAssertNil(bSnapshot)
        XCTAssertNil(cSnapshot)
        try await a.revokeProject("project-1")
        let revoked = try await a.snapshot(projectID: "project-1")
        XCTAssertNil(revoked)
    }

    func testPreviewChecksHashAndNeverForwardsBearerToTicketURL() async throws {
        let transport = RecordingTransport(preview: Data("wrong-pixels".utf8))
        let client = APIClient(configuration: try APIConfiguration("https://example.com"), transport: transport)
        let descriptor = try XCTUnwrap(snapshot().previews.first)
        do {
            _ = try await client.preview(projectID: "project-1", descriptor: descriptor, token: digest)
            XCTFail("像素摘要不符必须拒绝")
        } catch APIError.invalidResponse { }
        let requests = await transport.requests
        XCTAssertEqual(requests.count, 2)
        XCTAssertEqual(requests[0].value(forHTTPHeaderField: "Authorization"), "Bearer " + digest)
        XCTAssertNil(requests[1].value(forHTTPHeaderField: "Authorization"))
    }

    private let digest = String(repeating: "a", count: 64)

    private func snapshot() throws -> ProjectSnapshot {
        let value = try JSONDecoder().decode(ProjectSnapshot.self, from: fixture())
        try value.validate(projectID: "project-1", userID: "user-1")
        return value
    }

    private func fixture(selection: String = "UNDECIDED", role: String = "OWNER") -> Data {
        Data("""
        {"project_id":"project-1","workspace_id":"workspace-1","title":"测试项目","state":"SHARED","revision":0,"cursor":"1",
         "principal":{"id":"user-1","role":"\(role)","can_approve":true,"can_edit_formal":true},
         "snapshot":{"photos":[{"id":"photo-1","filename":"photo.jpg","source_sha256":"\(digest)","selection":"\(selection)"}],
         "sequences":[{"id":"sequence-1","title":"主序列","items":[{"id":"item-1","photo_id":"photo-1"}]}]},
         "previews":[{"photo_id":"photo-1","content_sha256":"\(digest)","width":96,"height":64}],"comments":[]}
        """.utf8)
    }
}

private actor RecordingTransport: HTTPTransport {
    var requests: [URLRequest] = []
    let preview: Data
    init(preview: Data) { self.preview = preview }
    func send(_ request: URLRequest, limit: Int) async throws -> HTTPResponse {
        requests.append(request)
        if request.httpMethod == "POST" {
            return HTTPResponse(data: Data("{\"url\":\"/api/v2/media/\(String(repeating: "b", count: 64))\"}".utf8), status: 200)
        }
        return HTTPResponse(data: preview, status: 200, contentType: "image/jpeg")
    }
}
