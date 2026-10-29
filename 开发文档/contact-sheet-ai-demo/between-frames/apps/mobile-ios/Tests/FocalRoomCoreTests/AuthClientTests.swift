import Foundation
import Testing
@testable import FocalRoomCore

// 验证码登录与设置密码的客户端契约：请求形状、令牌校验与错误路径。
final class AuthStubTransport: HTTPTransport, @unchecked Sendable {
    private(set) var requests: [URLRequest] = []
    private let lock = NSLock()
    var respond: ((URLRequest) -> HTTPResponse)?

    func send(_ request: URLRequest, limit: Int) async throws -> HTTPResponse {
        try lock.withLock {
            requests.append(request)
            guard let respond else { throw URLError(.notConnectedToInternet) }
            return respond(request)
        }
    }
}

@Suite struct AuthClientTests {
    private let token = String(repeating: "b", count: 64)
    private func credentialsJSON(token: String) -> Data {
        Data("""
        {"token":"\(token)","csrf":"csrf-1","user":{"id":"user-1","email":"a@fr.test","name":"A"}}
        """.utf8)
    }

    @Test func magicRequestPostsEmailAndRequiresSent() async throws {
        let transport = AuthStubTransport()
        transport.respond = { _ in HTTPResponse(data: Data("{\"sent\":true}".utf8), status: 200) }
        let client = APIClient(configuration: try APIConfiguration("https://example.com"), transport: transport)
        try await client.magicRequest(email: "A@FR.test")
        let request = try #require(transport.requests.first)
        #expect(request.url?.path == "/api/v2/auth/magic/request")
        let body = try #require(JSONSerialization.jsonObject(with: request.httpBody ?? Data()) as? [String: String])
        #expect(body.count == 1 && body["email"] == "A@FR.test", "请求只含输入邮箱，邮箱规整由服务端负责")
        transport.respond = { _ in HTTPResponse(data: Data("{\"sent\":false}".utf8), status: 200) }
        await #expect(throws: APIError.invalidResponse) {
            _ = try await client.magicRequest(email: "b@fr.test")
        }
    }

    @Test func magicVerifyReturnsCredentialsAndRejectsMalformedTokens() async throws {
        let transport = AuthStubTransport()
        transport.respond = { [token] _ in HTTPResponse(data: self.credentialsJSON(token: token), status: 200) }
        let client = APIClient(configuration: try APIConfiguration("https://example.com"), transport: transport)
        let value = try await client.magicVerify(email: "a@fr.test", code: "123456")
        #expect(value.token == token && value.user.id == "user-1")
        let request = try #require(transport.requests.first)
        #expect(request.url?.path == "/api/v2/auth/magic/verify")
        transport.respond = { _ in HTTPResponse(data: self.credentialsJSON(token: "not-hex-token"), status: 200) }
        await #expect(throws: APIError.invalidResponse) {
            _ = try await client.magicVerify(email: "a@fr.test", code: "123456")
        }
    }

    @Test func setPasswordSendsBearerAndOptionalCurrent() async throws {
        let transport = AuthStubTransport()
        transport.respond = { _ in HTTPResponse(data: Data("{\"updated\":true}".utf8), status: 200) }
        let client = APIClient(configuration: try APIConfiguration("https://example.com"), transport: transport)
        let token = String(repeating: "a", count: 64)
        try await client.setPassword(token: token, password: "new-password-123", current: "old-password-9")
        let request = try #require(transport.requests.first)
        #expect(request.url?.path == "/api/v2/auth/set-password")
        #expect(request.value(forHTTPHeaderField: "Authorization") == "Bearer \(token)")
        let body = try #require(JSONSerialization.jsonObject(with: request.httpBody ?? Data()) as? [String: String])
        #expect(body["password"] == "new-password-123" && body["current_password"] == "old-password-9")
        try await client.setPassword(token: token, password: "fresh-password-456", current: nil)
        let second = try #require(transport.requests.last)
        let secondBody = try #require(JSONSerialization.jsonObject(with: second.httpBody ?? Data()) as? [String: String])
        #expect(secondBody["current_password"] == nil, "验证码新建账号允许省略当前密码")
        transport.respond = { _ in HTTPResponse(data: Data("{\"updated\":false}".utf8), status: 200) }
        await #expect(throws: APIError.invalidResponse) {
            _ = try await client.setPassword(token: token, password: "another-789", current: nil)
        }
    }
}
