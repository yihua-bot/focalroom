import Foundation

struct NativeAIError: Error, LocalizedError, Sendable {
    let code: String
    let message: String
    var errorDescription: String? { message }
}

enum NativeChatEvent: Sendable {
    case delta(String)
    case completed(String)
}

private final class NativeChatRelay {
    let continuation: AsyncThrowingStream<NativeChatEvent, Error>.Continuation
    init(_ continuation: AsyncThrowingStream<NativeChatEvent, Error>.Continuation) {
        self.continuation = continuation
    }
}

// C ABI 为阻塞调用，独立 actor 承接，避免阻塞 SwiftUI 主线程。
actor NativeAIClient {
    private var handle: UInt64?

    func open(_ payload: Data) throws {
        if handle != nil { return }
        var request = try dictionary(payload)
        request["operation"] = "open"
        let result = try dictionary(invoke(request))
        guard let value = result["handle"] as? UInt64 else { throw malformed() }
        handle = value
    }

    func perform(_ operation: String, payload: Data) throws -> Data {
        guard let handle else { throw NativeAIError(code: "AUTH_REQUIRED", message: String(localized: "请重新登录")) }
        var request = try dictionary(payload)
        request["operation"] = operation
        request["handle"] = handle
        return try invoke(request)
    }

    func chatStream(payload: Data) -> AsyncThrowingStream<NativeChatEvent, Error> {
        AsyncThrowingStream { continuation in
            Task {
                do {
                    guard let handle else { throw NativeAIError(code: "AUTH_REQUIRED", message: String(localized: "请重新登录")) }
                    var request = try dictionary(payload)
                    request["operation"] = "chat"
                    request["handle"] = handle
                    let data = try JSONSerialization.data(withJSONObject: request, options: [.sortedKeys])
                    guard data.count <= 16 * 1024 * 1024, let input = String(data: data, encoding: .utf8) else { throw malformed() }
                    let relay = Unmanaged.passRetained(NativeChatRelay(continuation))
                    defer { relay.release() }
                    let output = input.withCString { input in
                        focalroom_agent_chat_stream(input, { text, context in
                            guard let text, let pointer = UnsafeRawPointer(bitPattern: context) else { return }
                            let relay = Unmanaged<NativeChatRelay>.fromOpaque(pointer).takeUnretainedValue()
                            relay.continuation.yield(.delta(String(cString: text)))
                        }, UInt(bitPattern: relay.toOpaque()))
                    }
                    let result = try dictionary(decode(output))
                    guard let text = result["text"] as? String, !text.isEmpty else { throw malformed() }
                    continuation.yield(.completed(text))
                    continuation.finish()
                } catch { continuation.finish(throwing: error) }
            }
        }
    }

    func close() {
        guard let value = handle else { return }
        handle = nil
        _ = try? invoke(["operation": "close", "handle": value])
    }

    private func invoke(_ value: [String: Any]) throws -> Data {
        let data = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
        guard data.count <= 16 * 1024 * 1024, let input = String(data: data, encoding: .utf8) else { throw malformed() }
        let output = input.withCString { focalroom_agent_call($0) }
        return try decode(output)
    }

    private func decode(_ output: UnsafeMutablePointer<CChar>?) throws -> Data {
        guard let output else { throw malformed() }
        defer { focalroom_agent_free(output) }
        let envelope = try dictionary(Data(String(cString: output).utf8))
        guard envelope["ok"] as? Bool == true, let result = envelope["result"] else {
            let error = envelope["error"] as? [String: Any]
            throw NativeAIError(code: error?["code"] as? String ?? "INVALID_RESPONSE",
                                message: error?["message"] as? String ?? String(localized: "AI 返回的内容无法读取"))
        }
        return try JSONSerialization.data(withJSONObject: result, options: [.fragmentsAllowed])
    }

    private func dictionary(_ data: Data) throws -> [String: Any] {
        guard let result = try JSONSerialization.jsonObject(with: data) as? [String: Any] else { throw malformed() }
        return result
    }
    private func malformed() -> NativeAIError { .init(code: "INVALID_RESPONSE", message: String(localized: "AI 返回的内容无法读取")) }
}
