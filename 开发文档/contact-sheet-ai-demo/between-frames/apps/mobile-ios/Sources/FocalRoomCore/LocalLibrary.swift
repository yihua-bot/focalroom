import Foundation
import CryptoKit
import ImageIO
import CoreGraphics
import UniformTypeIdentifiers

public enum LocalLibraryError: Error, LocalizedError, Sendable {
    case invalidProject, invalidFile, unsupportedImage, tooLarge, corruptLibrary, capacityReached, projectFrozen, previewUnshrinkable

    public var errorDescription: String? {
        switch self {
        case .invalidProject: "本机项目不存在或名称无效。"
        case .invalidFile: "无法读取所选文件，请重新选择。"
        case .unsupportedImage: "暂支持单张 JPEG、PNG、HEIC 和 TIFF 图片。"
        case .tooLarge: "图片超过导入上限：单份 64 MiB、5000 万像素。"
        case .corruptLibrary: "本机资料校验失败，已停止写入。请保留数据并联系支持。"
        case .capacityReached: "本机资料已达到当前容量上限。"
        case .projectFrozen: "项目正在迁云或已迁云，本机写入已冻结；请打开云端项目继续选片。"
        case .previewUnshrinkable: "照片预览超过迁云上传上限，无法生成合规预览。"
        }
    }
}

public struct LocalPhoto: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let filename: String
    public let sourceSHA256: String
    public let previewSHA256: String
    public let byteCount: Int
    public let width: Int
    public let height: Int
    public let orientation: Int
    public let format: String
    public let profileName: String?
    public let previewWidth: Int
    public let previewHeight: Int
    public fileprivate(set) var selection: Selection
}

// 迁云记录：enrolling 尚无服务端确认；uploading 已绑定云端项目；migrated 为终态。
// manifestJSON 保存首次发送的精确字节，重试与续传复用同一字节串，服务端摘要才视为同一迁移。
public struct LocalMigration: Codable, Sendable, Equatable {
    public enum Phase: String, Codable, Sendable { case enrolling, uploading, migrated }
    public let promotionID: String
    public fileprivate(set) var phase: Phase
    public let manifestJSON: Data
    public fileprivate(set) var manifestDigest: String?
    public fileprivate(set) var cloudProjectID: String?
    public fileprivate(set) var uploadedPhotoIDs: [String]
    public var privateSync: Bool? = nil
}

// 导出清单：取回数据所需的全部身份与校验信息；原片逐字节复制，不依赖本应用即可核对。
public struct LocalExportManifest: Codable, Sendable, Equatable {
    public struct Entry: Codable, Sendable, Equatable {
        public let photoID: String
        public let filename: String
        public let exportedFilename: String
        public let sourceSHA256: String
        public let previewSHA256: String
        public let byteCount: Int
        public let width: Int
        public let height: Int
        public let format: String
        public let selection: String
    }

    public struct MigrationState: Codable, Sendable, Equatable {
        public let phase: String
        public let cloudProjectID: String?
    }

    public let version: Int
    public let projectID: String
    public let title: String
    public let exportedAt: Date
    public let entries: [Entry]
    public let migration: MigrationState?
}

public struct LocalProject: Codable, Sendable, Identifiable, Equatable {
    public let id: String
    public let title: String
    public let createdAt: Date
    public fileprivate(set) var updatedAt: Date
    public fileprivate(set) var photos: [LocalPhoto]
    public fileprivate(set) var migration: LocalMigration?
}

public enum LocalImportResult: Sendable {
    case imported(LocalPhoto), duplicate(LocalPhoto)
}

// FileRepresentation 的临时 URL 仅在回调期间有效；复制时仍执行相同的大小与取消检查。
public enum ImportStaging {
    public static func copy(_ source: URL) throws -> URL {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent("FocalRoom-import-" + UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let target = folder.appendingPathComponent(source.lastPathComponent.isEmpty ? "photo" : source.lastPathComponent)
        do {
            _ = try LocalMediaFiles.copyBounded(source, to: target)
            return target
        } catch {
            try? FileManager.default.removeItem(at: folder)
            throw error
        }
    }

    public static func removeCopy(_ url: URL) {
        guard url.deletingLastPathComponent().lastPathComponent.hasPrefix("FocalRoom-import-"),
              url.deletingLastPathComponent().deletingLastPathComponent().standardizedFileURL == FileManager.default.temporaryDirectory.standardizedFileURL
        else { return }
        try? FileManager.default.removeItem(at: url.deletingLastPathComponent())
    }
}

private enum LocalMediaFiles {
    static let sourceLimit = 64 * 1024 * 1024
    static let previewLimit = 16 * 1024 * 1024
    static let pixelLimit = 50_000_000

    static func digest(_ data: Data) -> String {
        SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }

    static func copyBounded(_ source: URL, to target: URL) throws -> (sha: String, size: Int) {
        try Task.checkCancellation()
        guard source.isFileURL else { throw LocalLibraryError.invalidFile }
        let values = try source.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey])
        guard values.isRegularFile == true, values.isSymbolicLink != true else { throw LocalLibraryError.invalidFile }
        guard (values.fileSize ?? 0) <= sourceLimit else { throw LocalLibraryError.tooLarge }
        guard FileManager.default.createFile(atPath: target.path, contents: nil) else { throw LocalLibraryError.invalidFile }
        #if os(iOS)
        try FileManager.default.setAttributes([.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication], ofItemAtPath: target.path)
        #endif
        let input = try FileHandle(forReadingFrom: source)
        defer { try? input.close() }
        let output = try FileHandle(forWritingTo: target)
        defer { try? output.close() }
        var hash = SHA256()
        var count = 0
        while let data = try input.read(upToCount: 1024 * 1024), !data.isEmpty {
            try Task.checkCancellation()
            count += data.count
            guard count <= sourceLimit else { throw LocalLibraryError.tooLarge }
            hash.update(data: data)
            try output.write(contentsOf: data)
        }
        guard count > 0 else { throw LocalLibraryError.invalidFile }
        try output.synchronize()
        return (hash.finalize().map { String(format: "%02x", $0) }.joined(), count)
    }

    static func verify(_ url: URL, sha: String, limit: Int) throws {
        guard (try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? Int.max) <= limit else {
            throw LocalLibraryError.corruptLibrary
        }
        let file = try FileHandle(forReadingFrom: url)
        defer { try? file.close() }
        var hash = SHA256()
        var count = 0
        while let data = try file.read(upToCount: 1024 * 1024), !data.isEmpty {
            try Task.checkCancellation()
            count += data.count
            guard count <= limit else { throw LocalLibraryError.corruptLibrary }
            hash.update(data: data)
        }
        guard hash.finalize().map({ String(format: "%02x", $0) }).joined() == sha else { throw LocalLibraryError.corruptLibrary }
    }

    static func write(_ data: Data, to url: URL) throws {
        #if os(iOS)
        try data.write(to: url, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
        #else
        try data.write(to: url, options: .atomic)
        #endif
    }
}

public actor LocalLibrary {
    private struct Document: Codable {
        let version: Int
        let scope: AccountScope
        var projects: [LocalProject]
    }

    private let scope: AccountScope
    private let directory: URL
    private var manifest: URL { directory.appendingPathComponent("library.json") }
    private var media: URL { directory.appendingPathComponent("media", isDirectory: true) }

    public init(root: URL, scope: AccountScope) {
        self.scope = scope
        directory = root.appendingPathComponent(scope.key, isDirectory: true)
    }

    public func projects() throws -> [LocalProject] {
        try read().projects.sorted { $0.updatedAt > $1.updatedAt }
    }

    public func createProject(title: String) throws -> LocalProject {
        let title = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !title.isEmpty, title.count <= 120 else { throw LocalLibraryError.invalidProject }
        var document = try read()
        guard document.projects.count < 500 else { throw LocalLibraryError.capacityReached }
        let now = Date()
        let project = LocalProject(id: UUID().uuidString.lowercased(), title: title, createdAt: now, updatedAt: now, photos: [])
        document.projects.append(project)
        try write(document)
        return project
    }

    public func importFile(_ url: URL, projectID: String) throws -> LocalImportResult {
        try Task.checkCancellation()
        var document = try read()
        guard let index = document.projects.firstIndex(where: { $0.id == projectID }) else { throw LocalLibraryError.invalidProject }
        guard document.projects[index].migration == nil else { throw LocalLibraryError.projectFrozen }
        guard document.projects.reduce(0, { $0 + $1.photos.count }) < 10_000 else { throw LocalLibraryError.capacityReached }
        try prepareDirectory()
        let stage = media.appendingPathComponent(".ingest-" + UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: stage) }
        let copied = try LocalMediaFiles.copyBounded(url, to: stage)
        if let duplicate = document.projects[index].photos.first(where: { $0.sourceSHA256 == copied.sha }) {
            try LocalMediaFiles.verify(originalURL(duplicate), sha: copied.sha, limit: LocalMediaFiles.sourceLimit)
            try LocalMediaFiles.verify(previewURL(duplicate), sha: duplicate.previewSHA256, limit: LocalMediaFiles.previewLimit)
            return .duplicate(duplicate)
        }
        let rendered = try render(stage, filename: url.lastPathComponent, sourceSHA: copied.sha, byteCount: copied.size)
        try Task.checkCancellation()
        let original = originalURL(rendered.photo)
        if FileManager.default.fileExists(atPath: original.path) {
            try LocalMediaFiles.verify(original, sha: copied.sha, limit: LocalMediaFiles.sourceLimit)
        } else { try FileManager.default.moveItem(at: stage, to: original) }
        let preview = previewURL(rendered.photo)
        if FileManager.default.fileExists(atPath: preview.path) {
            try LocalMediaFiles.verify(preview, sha: rendered.photo.previewSHA256, limit: LocalMediaFiles.previewLimit)
        } else { try LocalMediaFiles.write(rendered.data, to: preview) }
        // 两份不可变文件已完整保存后，才原子发布项目中的新资产。
        try Task.checkCancellation()
        document.projects[index].photos.append(rendered.photo)
        document.projects[index].updatedAt = Date()
        try write(document)
        return .imported(rendered.photo)
    }

    public func select(projectID: String, photoID: String, selection: Selection) throws {
        var document = try read()
        guard let project = document.projects.firstIndex(where: { $0.id == projectID }),
              let photo = document.projects[project].photos.firstIndex(where: { $0.id == photoID }) else {
            throw LocalLibraryError.invalidProject
        }
        guard document.projects[project].migration == nil else { throw LocalLibraryError.projectFrozen }
        document.projects[project].photos[photo].selection = selection
        document.projects[project].updatedAt = Date()
        try write(document)
    }

    public func preview(projectID: String, photoID: String) throws -> Data {
        guard let photo = try read().projects.first(where: { $0.id == projectID })?.photos.first(where: { $0.id == photoID }) else {
            throw LocalLibraryError.invalidProject
        }
        let url = previewURL(photo)
        try LocalMediaFiles.verify(url, sha: photo.previewSHA256, limit: LocalMediaFiles.previewLimit)
        return try Data(contentsOf: url)
    }

    public func beginMigration(projectID: String, workspaceID: String, privateSync: Bool = false) throws -> LocalMigration {
        var document = try read()
        guard let index = document.projects.firstIndex(where: { $0.id == projectID }) else { throw LocalLibraryError.invalidProject }
        let project = document.projects[index]
        guard project.migration == nil else { throw LocalLibraryError.projectFrozen }
        guard !project.photos.isEmpty, !workspaceID.isEmpty, workspaceID.count <= 160,
              project.photos.allSatisfy({ !$0.filename.isEmpty })
        else { throw LocalLibraryError.invalidProject }
        let promotionID = UUID().uuidString.lowercased()
        let kept = project.photos.filter { $0.selection == .KEEP }
        let manifest = PromotionManifest(
            workspaceID: workspaceID,
            localProjectID: project.id,
            promotionID: promotionID,
            title: project.title,
            snapshot: SharedSnapshot(
                photos: project.photos.map { photo in
                    Photo(id: photo.id, filename: photo.filename, source_sha256: photo.sourceSHA256, selection: photo.selection)
                },
                sequences: [PhotoSequence(id: "seq-\(promotionID)", title: "作品序列",
                                          items: kept.map { SequenceItem(id: "\($0.id)-seq", photo_id: $0.id) })]
            ))
        // 编码字节随编码器实例可能不同；唯一保证是记录里保存的精确字节被原样重发，这里只做语义校验。
        let bytes = try JSONEncoder().encode(manifest)
        guard let decoded = try? JSONDecoder().decode(PromotionManifest.self, from: bytes), decoded == manifest else {
            throw LocalLibraryError.corruptLibrary
        }
        var record = LocalMigration(promotionID: promotionID, phase: .enrolling, manifestJSON: bytes,
                                    manifestDigest: nil, cloudProjectID: nil, uploadedPhotoIDs: [])
        record.privateSync = privateSync
        document.projects[index].migration = record
        try write(document)
        return record
    }

    public func migrationEnrolled(projectID: String, cloudProjectID: String, manifestDigest: String) throws {
        var document = try read()
        guard let index = document.projects.firstIndex(where: { $0.id == projectID }),
              document.projects[index].migration?.phase == .enrolling,
              !cloudProjectID.isEmpty,
              manifestDigest.count == 64, manifestDigest.allSatisfy({ "0123456789abcdef".contains($0) })
        else { throw LocalLibraryError.invalidProject }
        document.projects[index].migration?.phase = .uploading
        document.projects[index].migration?.manifestDigest = manifestDigest
        document.projects[index].migration?.cloudProjectID = cloudProjectID
        try write(document)
    }

    public func markUploaded(projectID: String, photoID: String) throws {
        var document = try read()
        guard let index = document.projects.firstIndex(where: { $0.id == projectID }),
              let record = document.projects[index].migration, record.phase == .uploading,
              document.projects[index].photos.contains(where: { $0.id == photoID }),
              !record.uploadedPhotoIDs.contains(photoID)
        else { return }
        document.projects[index].migration?.uploadedPhotoIDs.append(photoID)
        try write(document)
    }

    public func completeMigration(projectID: String) throws {
        var document = try read()
        guard let index = document.projects.firstIndex(where: { $0.id == projectID }),
              let record = document.projects[index].migration, record.phase == .uploading,
              record.cloudProjectID != nil,
              Set(record.uploadedPhotoIDs) == Set(document.projects[index].photos.map(\.id))
        else { throw LocalLibraryError.invalidProject }
        document.projects[index].migration?.phase = .migrated
        try write(document)
    }

    public func clearMigration(projectID: String) throws {
        var document = try read()
        guard let index = document.projects.firstIndex(where: { $0.id == projectID }),
              let record = document.projects[index].migration, record.phase != .migrated
        else { throw LocalLibraryError.invalidProject }
        document.projects[index].migration = nil
        try write(document)
    }

    // 迁云上传载荷：校验本机预览字节；超出服务端 8 MiB 上限时用更低质量重编码（仅派生预览，不动原片）。
    public func uploadPayload(projectID: String, photoID: String) throws -> Data {
        let data = try preview(projectID: projectID, photoID: photoID)
        guard data.count > 8 * 1024 * 1024 else { return data }
        guard let source = CGImageSourceCreateWithData(data as CFData, nil),
              let image = CGImageSourceCreateThumbnailAtIndex(source, 0, [
                  kCGImageSourceCreateThumbnailFromImageAlways: true,
                  kCGImageSourceCreateThumbnailWithTransform: true,
                  kCGImageSourceThumbnailMaxPixelSize: 2048
              ] as CFDictionary)
        else { throw LocalLibraryError.previewUnshrinkable }
        for quality in [0.8, 0.6, 0.4, 0.2] {
            let output = NSMutableData()
            guard let destination = CGImageDestinationCreateWithData(output, UTType.jpeg.identifier as CFString, 1, nil) else {
                throw LocalLibraryError.previewUnshrinkable
            }
            CGImageDestinationAddImage(destination, image, [kCGImageDestinationLossyCompressionQuality: quality] as CFDictionary)
            guard CGImageDestinationFinalize(destination), output.length <= 8 * 1024 * 1024 else { continue }
            return output as Data
        }
        throw LocalLibraryError.previewUnshrinkable
    }

    // 导出暂存：先逐份校验原片摘要，再逐字节复制到新目录；任一步失败即清理整个暂存目录。
    public func stageExport(projectID: String, to base: URL) throws -> (directory: URL, manifest: LocalExportManifest) {
        try Task.checkCancellation()
        guard let project = try read().projects.first(where: { $0.id == projectID }),
              !project.photos.isEmpty else { throw LocalLibraryError.invalidProject }
        let directory = base.appendingPathComponent("FocalRoom-export-" + UUID().uuidString, isDirectory: true)
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            var used: Set<String> = []
            var entries: [LocalExportManifest.Entry] = []
            for photo in project.photos {
                try Task.checkCancellation()
                let original = originalURL(photo)
                try LocalMediaFiles.verify(original, sha: photo.sourceSHA256, limit: LocalMediaFiles.sourceLimit)
                let exported = Self.exportFilename(photo.filename, sourceSHA: photo.sourceSHA256, used: &used)
                try FileManager.default.copyItem(at: original, to: directory.appendingPathComponent(exported))
                entries.append(LocalExportManifest.Entry(
                    photoID: photo.id, filename: photo.filename, exportedFilename: exported,
                    sourceSHA256: photo.sourceSHA256, previewSHA256: photo.previewSHA256,
                    byteCount: photo.byteCount, width: photo.width, height: photo.height,
                    format: photo.format, selection: photo.selection.rawValue))
            }
            let manifest = LocalExportManifest(
                version: 1, projectID: project.id, title: project.title, exportedAt: Date(),
                entries: entries,
                migration: project.migration.map {
                    LocalExportManifest.MigrationState(phase: $0.phase.rawValue, cloudProjectID: $0.cloudProjectID)
                })
            try LocalMediaFiles.write(try JSONEncoder().encode(manifest),
                                      to: directory.appendingPathComponent("manifest.json"))
            return (directory, manifest)
        } catch {
            try? FileManager.default.removeItem(at: directory)
            throw error
        }
    }

    private static func exportFilename(_ raw: String, sourceSHA: String, used: inout Set<String>) -> String {
        let fallback = "photo-" + String(sourceSHA.prefix(12))
        var cleaned = raw.replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "\\", with: "_")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if cleaned.isEmpty || cleaned == "." || cleaned == ".." { cleaned = fallback }
        if !used.insert(cleaned).inserted {
            let base = (cleaned as NSString).deletingPathExtension
            let ext = (cleaned as NSString).pathExtension
            var index = 2
            repeat {
                cleaned = ext.isEmpty ? "\(base)-\(index)" : "\(base)-\(index).\(ext)"
                index += 1
            } while !used.insert(cleaned).inserted
        }
        return cleaned
    }

    private func originalURL(_ photo: LocalPhoto) -> URL { media.appendingPathComponent(photo.sourceSHA256 + ".original") }
    private func previewURL(_ photo: LocalPhoto) -> URL { media.appendingPathComponent(photo.previewSHA256 + ".jpg") }

    private func read() throws -> Document {
        guard FileManager.default.fileExists(atPath: manifest.path) else { return Document(version: 1, scope: scope, projects: []) }
        guard (try manifest.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? Int.max) <= 24 * 1024 * 1024 else {
            throw LocalLibraryError.corruptLibrary
        }
        let document: Document
        do { document = try JSONDecoder().decode(Document.self, from: Data(contentsOf: manifest)) }
        catch { throw LocalLibraryError.corruptLibrary }
        let photos = document.projects.flatMap(\.photos)
        let hash: (String) -> Bool = { $0.count == 64 && $0.allSatisfy { "0123456789abcdef".contains($0) } }
        guard document.version == 1, document.scope == scope, document.projects.count <= 500, photos.count <= 10_000,
              Set(document.projects.map(\.id)).count == document.projects.count,
              Set(photos.map(\.id)).count == photos.count,
              document.projects.allSatisfy({ UUID(uuidString: $0.id) != nil && !$0.title.isEmpty && $0.title.count <= 120 &&
                  Set($0.photos.map(\.sourceSHA256)).count == $0.photos.count }),
              photos.allSatisfy({ UUID(uuidString: $0.id) != nil && hash($0.sourceSHA256) && hash($0.previewSHA256) &&
                  $0.byteCount > 0 && $0.byteCount <= LocalMediaFiles.sourceLimit && $0.width > 0 && $0.height > 0 &&
                  $0.width <= LocalMediaFiles.pixelLimit / $0.height && (1...8).contains($0.orientation) &&
                  $0.previewWidth > 0 && $0.previewWidth <= 2048 && $0.previewHeight > 0 && $0.previewHeight <= 2048 }),
              document.projects.allSatisfy({ project in
                  guard let record = project.migration else { return true }
                  let ids = Set(project.photos.map(\.id))
                  let manifest = try? JSONDecoder().decode(PromotionManifest.self, from: record.manifestJSON)
                  return UUID(uuidString: record.promotionID) != nil &&
                      record.manifestJSON.count >= 16 && record.manifestJSON.count <= 8 * 1024 * 1024 &&
                      (record.manifestDigest == nil || hash(record.manifestDigest!)) &&
                      (record.cloudProjectID?.isEmpty == false) != (record.phase == .enrolling) &&
                      Set(record.uploadedPhotoIDs).count == record.uploadedPhotoIDs.count &&
                      Set(record.uploadedPhotoIDs).isSubset(of: ids) &&
                      manifest?.promotion_id == record.promotionID && manifest?.local_project_id == project.id
              })
        else { throw LocalLibraryError.corruptLibrary }
        return document
    }

    private func prepareDirectory() throws {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        var url = directory
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        try url.setResourceValues(values)
        try FileManager.default.createDirectory(at: media, withIntermediateDirectories: true)
    }

    private func write(_ document: Document) throws {
        let data = try JSONEncoder().encode(document)
        guard data.count <= 24 * 1024 * 1024 else { throw LocalLibraryError.capacityReached }
        try prepareDirectory()
        try LocalMediaFiles.write(data, to: manifest)
    }

    private func render(_ url: URL, filename: String, sourceSHA: String, byteCount: Int) throws -> (photo: LocalPhoto, data: Data) {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, [kCGImageSourceShouldCache: false] as CFDictionary),
              CGImageSourceGetCount(source) == 1, let typeID = CGImageSourceGetType(source) as String?,
              let type = UTType(typeID), [UTType.jpeg, .png, .heic, .tiff].contains(where: { type.conforms(to: $0) }),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let width = properties[kCGImagePropertyPixelWidth] as? Int,
              let height = properties[kCGImagePropertyPixelHeight] as? Int,
              width > 0, height > 0 else { throw LocalLibraryError.unsupportedImage }
        guard width <= LocalMediaFiles.pixelLimit / height else { throw LocalLibraryError.tooLarge }
        let orientation = (properties[kCGImagePropertyOrientation] as? Int) ?? 1
        guard (1...8).contains(orientation) else { throw LocalLibraryError.unsupportedImage }
        try Task.checkCancellation()
        guard let thumbnail = CGImageSourceCreateThumbnailAtIndex(source, 0, [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceThumbnailMaxPixelSize: 2048,
            kCGImageSourceShouldCacheImmediately: true
        ] as CFDictionary), let colorSpace = CGColorSpace(name: CGColorSpace.sRGB),
              let context = CGContext(data: nil, width: thumbnail.width, height: thumbnail.height, bitsPerComponent: 8,
                                      bytesPerRow: 0, space: colorSpace, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
        else { throw LocalLibraryError.unsupportedImage }
        context.setFillColor(CGColor(gray: 1, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: thumbnail.width, height: thumbnail.height))
        context.draw(thumbnail, in: CGRect(x: 0, y: 0, width: thumbnail.width, height: thumbnail.height))
        guard let image = context.makeImage() else { throw LocalLibraryError.unsupportedImage }
        let output = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(output, UTType.jpeg.identifier as CFString, 1, nil) else {
            throw LocalLibraryError.unsupportedImage
        }
        CGImageDestinationAddImage(destination, image, [kCGImageDestinationLossyCompressionQuality: 0.9,
                                                       kCGImagePropertyOrientation: 1] as CFDictionary)
        guard CGImageDestinationFinalize(destination), output.length <= LocalMediaFiles.previewLimit else {
            throw LocalLibraryError.unsupportedImage
        }
        let data = output as Data
        let photo = LocalPhoto(id: UUID().uuidString.lowercased(), filename: String(filename.prefix(240)),
                               sourceSHA256: sourceSHA, previewSHA256: LocalMediaFiles.digest(data), byteCount: byteCount,
                               width: width, height: height, orientation: orientation, format: type.preferredFilenameExtension ?? typeID,
                               profileName: (properties[kCGImagePropertyProfileName] as? String).map { String($0.prefix(200)) },
                               previewWidth: image.width, previewHeight: image.height, selection: .UNDECIDED)
        return (photo, data)
    }
}
