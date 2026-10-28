import Foundation
import CryptoKit

// 云端预览离线缓存：按内容摘要寻址的有界磁盘缓存。
// 缓存只是已授权派生预览的副本，不提升任何权限；损坏或索引不符按未命中处理并清理。
public actor PreviewCache {
    private struct Entry: Codable {
        let bytes: Int
        var lastAccess: Date
    }

    private struct Index: Codable {
        let version: Int
        var entries: [String: Entry]
    }

    public static let countLimit = 150
    public static let byteLimit = 128 * 1024 * 1024

    private let directory: URL
    private var index: Index
    private var loaded = false

    public init(directory: URL) {
        self.directory = directory
        index = Index(version: 1, entries: [:])
    }

    private var indexURL: URL { directory.appendingPathComponent("index.json") }
    private func fileURL(_ sha: String) -> URL { directory.appendingPathComponent("\(sha).jpg") }

    private static func validSHA(_ sha: String) -> Bool {
        sha.count == 64 && sha.allSatisfy { "0123456789abcdef".contains($0) }
    }

    private func loadIfNeeded() {
        guard !loaded else { return }
        loaded = true
        guard let data = try? Data(contentsOf: indexURL),
              let decoded = try? JSONDecoder().decode(Index.self, from: data),
              decoded.version == 1
        else {
            index = Index(version: 1, entries: [:])
            return
        }
        var entries = decoded.entries
        for (sha, entry) in entries where !Self.validSHA(sha) || entry.bytes <= 0 {
            entries.removeValue(forKey: sha)
        }
        index = Index(version: 1, entries: entries)
    }

    private func persist() {
        let data = try? JSONEncoder().encode(index)
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        if let data { try? data.write(to: indexURL, options: .atomic) }
    }

    public func cached(sha: String) -> Data? {
        loadIfNeeded()
        guard Self.validSHA(sha), var entry = index.entries[sha] else { return nil }
        let url = fileURL(sha)
        guard let data = try? Data(contentsOf: url), data.count == entry.bytes else {
            // 文件缺失或长度不符：清掉这条失效缓存。
            index.entries.removeValue(forKey: sha)
            try? FileManager.default.removeItem(at: url)
            persist()
            return nil
        }
        entry.lastAccess = Date()
        index.entries[sha] = entry
        persist()
        return data
    }

    public func store(_ data: Data, sha: String) {
        loadIfNeeded()
        guard Self.validSHA(sha), !data.isEmpty else { return }
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        do {
            let url = fileURL(sha)
            if FileManager.default.fileExists(atPath: url.path) {
                try FileManager.default.removeItem(at: url)
            }
            try data.write(to: url, options: .atomic)
            index.entries[sha] = Entry(bytes: data.count, lastAccess: Date())
            evictIfNeeded()
            persist()
        } catch {
            try? FileManager.default.removeItem(at: fileURL(sha))
        }
    }

    private func evictIfNeeded() {
        var total = index.entries.values.reduce(0) { $0 + $1.bytes }
        while index.entries.count > Self.countLimit || total > Self.byteLimit {
            guard let oldest = index.entries.min(by: { $0.value.lastAccess < $1.value.lastAccess }) else { break }
            index.entries.removeValue(forKey: oldest.key)
            try? FileManager.default.removeItem(at: fileURL(oldest.key))
            total -= oldest.value.bytes
        }
    }

    public func clear() {
        index = Index(version: 1, entries: [:])
        loaded = true
        try? FileManager.default.removeItem(at: directory)
    }
}
