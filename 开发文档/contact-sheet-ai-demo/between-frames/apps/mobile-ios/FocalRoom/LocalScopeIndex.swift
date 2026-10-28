import Foundation
import FocalRoomCore

struct LocalScopeEntry: Codable, Identifiable, Equatable {
    let server: String
    let userID: String
    let email: String
    let name: String
    let updatedAt: Date

    var id: String { "\(server)|\(userID)" }
}

// 未登录时离线辨认本机资料归属的索引。明文存于应用容器（受设备加密保护），
// 只记录账户名与服务地址，不含任何会话凭据；索引损坏仅影响离线列表，不触碰资料本体。
enum LocalScopeIndex {
    static func file(root: URL) -> URL {
        root.appendingPathComponent("LocalLibrary").appendingPathComponent("index.json")
    }

    static func record(root: URL, server: String, userID: String, email: String, name: String) {
        var entries = list(root: root).filter { $0.id != "\(server)|\(userID)" }
        entries.append(LocalScopeEntry(server: server, userID: userID, email: email, name: name, updatedAt: Date()))
        guard entries.count <= 50, let data = try? JSONEncoder().encode(entries) else { return }
        let directory = root.appendingPathComponent("LocalLibrary")
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try? data.write(to: file(root: root), options: .atomic)
    }

    static func list(root: URL) -> [LocalScopeEntry] {
        guard let data = try? Data(contentsOf: file(root: root)),
              let entries = try? JSONDecoder().decode([LocalScopeEntry].self, from: data),
              entries.count <= 50,
              Set(entries.map(\.id)).count == entries.count
        else { return [] }
        return entries.sorted { $0.updatedAt > $1.updatedAt }
    }
}
