import Foundation
import XCTest
import CryptoKit
import ImageIO
import CoreGraphics
import UniformTypeIdentifiers
@testable import FocalRoomCore

@MainActor
final class LocalLibraryTests: XCTestCase {
    func testOriginalBytesOrientationMetadataDeduplicationAndRestart() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("portrait.jpg")
        let bytes = try jpeg(orientation: 6)
        let originalSource = try XCTUnwrap(CGImageSourceCreateWithData(bytes as CFData, nil))
        let originalProperties = try XCTUnwrap(CGImageSourceCopyPropertiesAtIndex(originalSource, 0, nil) as? [CFString: Any])
        XCTAssertNotNil(originalProperties[kCGImagePropertyGPSDictionary])
        XCTAssertEqual((originalProperties[kCGImagePropertyExifDictionary] as? [CFString: Any])?[kCGImagePropertyExifUserComment] as? String,
                       "source metadata must stay local")
        try bytes.write(to: source)
        let scope = AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "local-user")
        let root = directory.appendingPathComponent("library")
        let library = LocalLibrary(root: root, scope: scope)
        let project = try await library.createProject(title: "方向与原片")
        guard case .imported(let photo) = try await library.importFile(source, projectID: project.id) else {
            return XCTFail("首次导入应保存照片")
        }
        XCTAssertEqual(try Data(contentsOf: source), bytes, "不能改写用户选择的原文件")
        XCTAssertEqual(photo.sourceSHA256, digest(bytes))
        XCTAssertEqual(photo.orientation, 6)
        XCTAssertEqual(photo.width, 96)
        XCTAssertEqual(photo.height, 64)
        XCTAssertEqual(photo.previewWidth, 64)
        XCTAssertEqual(photo.previewHeight, 96)
        let archived = root.appendingPathComponent(scope.key).appendingPathComponent("media").appendingPathComponent(photo.sourceSHA256 + ".original")
        XCTAssertEqual(try Data(contentsOf: archived), bytes, "本机保存的原片应逐字节一致")
        let preview = try await library.preview(projectID: project.id, photoID: photo.id)
        XCTAssertEqual(digest(preview), photo.previewSHA256)
        let previewSource = try XCTUnwrap(CGImageSourceCreateWithData(preview as CFData, nil))
        let properties = try XCTUnwrap(CGImageSourceCopyPropertiesAtIndex(previewSource, 0, nil) as? [CFString: Any])
        XCTAssertNil(properties[kCGImagePropertyGPSDictionary], "浏览预览不复制原片 GPS")
        let exif = properties[kCGImagePropertyExifDictionary] as? [CFString: Any]
        XCTAssertNil(exif?[kCGImagePropertyExifUserComment], "预览不能复制原片中的私密备注")
        XCTAssertNil(exif?[kCGImagePropertyExifDateTimeOriginal])
        XCTAssertEqual((properties[kCGImagePropertyOrientation] as? Int) ?? 1, 1)
        guard case .duplicate(let duplicate) = try await library.importFile(source, projectID: project.id) else {
            return XCTFail("同一项目重复导入不能制造第二个资产")
        }
        XCTAssertEqual(photo.id, duplicate.id)
        try await library.select(projectID: project.id, photoID: photo.id, selection: .KEEP)
        let reopened = LocalLibrary(root: root, scope: scope)
        let projects = try await reopened.projects()
        XCTAssertEqual(projects.first?.photos.count, 1)
        XCTAssertEqual(projects.first?.photos.first?.selection, .KEEP)
        XCTAssertEqual(projects.first?.photos.first?.id, photo.id)
        XCTAssertEqual(try Data(contentsOf: archived), bytes)
        let excluded = try root.appendingPathComponent(scope.key).resourceValues(forKeys: [.isExcludedFromBackupKey])
        XCTAssertEqual(excluded.isExcludedFromBackup, true, "本机原片不能随系统备份自动外传")
    }

    func testProjectsKeepIndependentAssetIdentityAndAccountsRemainIsolated() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("photo.jpg")
        try jpeg().write(to: source)
        let configuration = try APIConfiguration("https://example.com")
        let root = directory.appendingPathComponent("library")
        let library = LocalLibrary(root: root, scope: AccountScope(configuration: configuration, userID: "one"))
        let first = try await library.createProject(title: "一")
        let second = try await library.createProject(title: "二")
        _ = try await library.importFile(source, projectID: first.id)
        _ = try await library.importFile(source, projectID: second.id)
        let projects = try await library.projects()
        let photos = projects.flatMap(\.photos)
        XCTAssertEqual(photos.count, 2)
        XCTAssertNotEqual(photos[0].id, photos[1].id)
        XCTAssertEqual(photos[0].sourceSHA256, photos[1].sourceSHA256)
        let other = LocalLibrary(root: root, scope: AccountScope(configuration: configuration, userID: "two"))
        let otherProjects = try await other.projects()
        XCTAssertTrue(otherProjects.isEmpty)
        do { _ = try await other.preview(projectID: first.id, photoID: photos[0].id); XCTFail("不能跨账号读取") }
        catch LocalLibraryError.invalidProject { }
    }

    func testInvalidOversizedAndCancelledImportsNeverPublishAssets() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("fake.jpg")
        let original = Data("not an image".utf8)
        try original.write(to: source)
        let library = LocalLibrary(root: directory.appendingPathComponent("library"),
                                   scope: AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one"))
        let project = try await library.createProject(title: "失败保护")
        do { _ = try await library.importFile(source, projectID: project.id); XCTFail("损坏图片必须拒绝") }
        catch LocalLibraryError.unsupportedImage { }
        XCTAssertEqual(try Data(contentsOf: source), original)
        let large = directory.appendingPathComponent("oversized.jpg")
        XCTAssertTrue(FileManager.default.createFile(atPath: large.path, contents: nil))
        let file = try FileHandle(forWritingTo: large)
        try file.truncate(atOffset: 64 * 1024 * 1024 + 1)
        try file.close()
        do { _ = try await library.importFile(large, projectID: project.id); XCTFail("超限文件必须拒绝") }
        catch LocalLibraryError.tooLarge { }
        try jpeg().write(to: source)
        let job = Task { try await library.importFile(source, projectID: project.id) }
        job.cancel()
        do { _ = try await job.value; XCTFail("取消不能发布新资产") }
        catch is CancellationError { }
        let projects = try await library.projects()
        XCTAssertTrue(projects.first?.photos.isEmpty == true)
    }

    func testCorruptManifestAndPreviewAreNotSilentlyReplaced() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("photo.jpg")
        try jpeg().write(to: source)
        let scope = AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one")
        let root = directory.appendingPathComponent("library")
        let library = LocalLibrary(root: root, scope: scope)
        let project = try await library.createProject(title: "完整性")
        guard case .imported(let photo) = try await library.importFile(source, projectID: project.id) else { return XCTFail() }
        let base = root.appendingPathComponent(scope.key)
        try Data("wrong pixels".utf8).write(to: base.appendingPathComponent("media/\(photo.previewSHA256).jpg"))
        do { _ = try await library.preview(projectID: project.id, photoID: photo.id); XCTFail("不显示被替换的像素") }
        catch LocalLibraryError.corruptLibrary { }
        let manifest = base.appendingPathComponent("library.json")
        let broken = Data("broken manifest".utf8)
        try broken.write(to: manifest)
        do { _ = try await library.createProject(title: "不能覆盖旧库"); XCTFail() }
        catch LocalLibraryError.corruptLibrary { }
        XCTAssertEqual(try Data(contentsOf: manifest), broken)
    }

    func testPNGAndTIFFUseActualFormatAndPreserveOriginals() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let library = LocalLibrary(root: directory.appendingPathComponent("library"),
                                   scope: AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one"))
        let project = try await library.createProject(title: "格式验证")
        for type in [UTType.png, .tiff] {
            let bytes = try jpeg(type: type)
            let file = directory.appendingPathComponent(type.preferredFilenameExtension! + ".jpg")
            try bytes.write(to: file)
            guard case .imported(let photo) = try await library.importFile(file, projectID: project.id) else { return XCTFail() }
            XCTAssertEqual(photo.format, type.preferredFilenameExtension)
            XCTAssertEqual(photo.sourceSHA256, digest(bytes))
            XCTAssertEqual(try Data(contentsOf: file), bytes)
        }
    }

    func testHEICImportWithAvailableSystemCodec() async throws {
        guard (CGImageDestinationCopyTypeIdentifiers() as? [String])?.contains(UTType.heic.identifier) == true else {
            throw XCTSkip("当前测试系统缺少 HEIC 编码器，不能伪造格式验证通过")
        }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("photo.heic")
        let bytes = try jpeg(type: .heic)
        try bytes.write(to: source)
        let library = LocalLibrary(root: directory.appendingPathComponent("library"),
                                   scope: AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one"))
        let project = try await library.createProject(title: "HEIC")
        guard case .imported(let photo) = try await library.importFile(source, projectID: project.id) else { return XCTFail() }
        XCTAssertEqual(photo.sourceSHA256, digest(bytes))
        XCTAssertEqual(photo.format, "heic")
        let preview = try await library.preview(projectID: project.id, photoID: photo.id)
        XCTAssertNotNil(CGImageSourceCreateWithData(preview as CFData, nil))
    }

    private func jpeg(orientation: Int = 1, type: UTType = .jpeg) throws -> Data {
        let context = try XCTUnwrap(CGContext(data: nil, width: 96, height: 64, bitsPerComponent: 8, bytesPerRow: 0,
                                             space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        context.setFillColor(CGColor(red: 0.2, green: 0.4, blue: 0.6, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 96, height: 64))
        let image = try XCTUnwrap(context.makeImage())
        let data = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(data, type.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, [kCGImagePropertyOrientation: orientation,
            kCGImagePropertyGPSDictionary: [kCGImagePropertyGPSLatitude: 31.2, kCGImagePropertyGPSLatitudeRef: "N"],
            kCGImagePropertyExifDictionary: [kCGImagePropertyExifUserComment: "source metadata must stay local"]] as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }

    private func digest(_ data: Data) -> String { SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined() }
}

@MainActor
final class MigrationStateTests: XCTestCase {
    func testMigrationRecordStableFrozenAndResumableAcrossRestart() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let first = directory.appendingPathComponent("a.jpg"), second = directory.appendingPathComponent("b.jpg")
        try jpeg().write(to: first)
        try jpeg(orientation: 6).write(to: second)
        let scope = AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one")
        let root = directory.appendingPathComponent("library")
        let library = LocalLibrary(root: root, scope: scope)
        let project = try await library.createProject(title: "迁云状态")
        guard case .imported(let photoA) = try await library.importFile(first, projectID: project.id),
              case .imported(let photoB) = try await library.importFile(second, projectID: project.id) else { return XCTFail() }
        try await library.select(projectID: project.id, photoID: photoA.id, selection: .KEEP)

        let record = try await library.beginMigration(projectID: project.id, workspaceID: "workspace-1")
        XCTAssertEqual(record.phase, .enrolling)
        XCTAssertNil(record.cloudProjectID)
        let manifest = try JSONDecoder().decode(PromotionManifest.self, from: record.manifestJSON)
        XCTAssertEqual(manifest.promotion_id, record.promotionID)
        XCTAssertEqual(manifest.local_project_id, project.id)
        XCTAssertEqual(manifest.workspace_id, "workspace-1")
        XCTAssertEqual(manifest.title, "迁云状态")
        XCTAssertEqual(manifest.from_revision, 0)
        XCTAssertEqual(manifest.media_policy, PromotionMediaPolicy())
        XCTAssertEqual(manifest.snapshot.photos.map(\.id), [photoA.id, photoB.id])
        XCTAssertEqual(manifest.snapshot.photos.map(\.selection), [.KEEP, .UNDECIDED])
        XCTAssertEqual(manifest.snapshot.sequences.flatMap(\.items).map(\.photo_id), [photoA.id], "序列只承载保留照片")
        // 记录字节由落盘记录原样重发；此处校验语义一致而非重新编码相等。
        XCTAssertEqual(try JSONDecoder().decode(PromotionManifest.self, from: record.manifestJSON), manifest)

        do { _ = try await library.importFile(first, projectID: project.id); XCTFail("迁云期间冻结导入") }
        catch LocalLibraryError.projectFrozen { }
        do { _ = try await library.select(projectID: project.id, photoID: photoB.id, selection: .KEEP); XCTFail("迁云期间冻结选片") }
        catch LocalLibraryError.projectFrozen { }
        do { _ = try await library.beginMigration(projectID: project.id, workspaceID: "workspace-1"); XCTFail("不能重复发起") }
        catch LocalLibraryError.projectFrozen { }

        let digest = String(repeating: "a", count: 64)
        try await library.migrationEnrolled(projectID: project.id, cloudProjectID: "cloud-1", manifestDigest: digest)
        try await library.markUploaded(projectID: project.id, photoID: photoA.id)
        try await library.markUploaded(projectID: project.id, photoID: photoA.id)
        do { try await library.completeMigration(projectID: project.id); XCTFail("缺一张上传记录不得完成") }
        catch LocalLibraryError.invalidProject { }
        try await library.markUploaded(projectID: project.id, photoID: photoB.id)
        try await library.completeMigration(projectID: project.id)
        do { try await library.clearMigration(projectID: project.id); XCTFail("已迁云是终态，不能回退") }
        catch LocalLibraryError.invalidProject { }

        // 重启后迁移记录与冻结仍然生效。
        let reopened = LocalLibrary(root: root, scope: scope)
        let restored = try await reopened.projects()
        XCTAssertEqual(restored.first?.migration?.phase, .migrated)
        XCTAssertEqual(restored.first?.migration?.cloudProjectID, "cloud-1")
        XCTAssertEqual(restored.first?.migration?.uploadedPhotoIDs.count, 2)
        do { _ = try await reopened.select(projectID: project.id, photoID: photoB.id, selection: .KEEP); XCTFail("迁云后本机仍冻结") }
        catch LocalLibraryError.projectFrozen { }
    }

    func testEmptyProjectCannotEnroll() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let library = LocalLibrary(root: directory.appendingPathComponent("library"),
                                   scope: AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one"))
        let project = try await library.createProject(title: "空项目")
        do { _ = try await library.beginMigration(projectID: project.id, workspaceID: "w"); XCTFail("空项目不能迁云") }
        catch LocalLibraryError.invalidProject { }
    }

    func testTamperedMigrationRecordRefusesFurtherWrites() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("photo.jpg")
        try jpeg().write(to: source)
        let scope = AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one")
        let root = directory.appendingPathComponent("library")
        let library = LocalLibrary(root: root, scope: scope)
        let project = try await library.createProject(title: "完整性")
        guard case .imported(let photo) = try await library.importFile(source, projectID: project.id) else { return XCTFail() }
        _ = try await library.beginMigration(projectID: project.id, workspaceID: "w")
        let manifest = root.appendingPathComponent(scope.key).appendingPathComponent("library.json")
        var document = try JSONSerialization.jsonObject(with: Data(contentsOf: manifest)) as! [String: Any]
        var projects = document["projects"] as! [[String: Any]]
        var record = projects[0]["migration"] as! [String: Any]
        record["manifestJSON"] = "YnJva2Vu"  // 指向无法解码的清单字节
        projects[0]["migration"] = record
        document["projects"] = projects
        let tampered = try JSONSerialization.data(withJSONObject: document)
        try tampered.write(to: manifest)
        let reopened = LocalLibrary(root: root, scope: scope)
        do { _ = try await reopened.preview(projectID: project.id, photoID: photo.id); XCTFail("损坏迁移记录必须拒绝写入路径") }
        catch LocalLibraryError.corruptLibrary { }
    }

    func testUploadPayloadStaysWithinServerBound() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("noise.jpg")
        try noiseJPEG(side: 2048).write(to: source)
        let library = LocalLibrary(root: directory.appendingPathComponent("library"),
                                   scope: AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one"))
        let project = try await library.createProject(title: "大预览")
        guard case .imported(let photo) = try await library.importFile(source, projectID: project.id) else { return XCTFail() }
        let payload = try await library.uploadPayload(projectID: project.id, photoID: photo.id)
        XCTAssertLessThanOrEqual(payload.count, 8 * 1024 * 1024, "上传载荷必须落在服务端 8 MiB 限内")
        XCTAssertNotNil(CGImageSourceCreateWithData(payload as CFData, nil), "重编码后仍须是可解码 JPEG")
    }

    private func jpeg(orientation: Int = 1) throws -> Data {
        let context = try XCTUnwrap(CGContext(data: nil, width: 96, height: 64, bitsPerComponent: 8, bytesPerRow: 0,
                                              space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        context.setFillColor(CGColor(red: 0.6, green: 0.2, blue: 0.4, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 96, height: 64))
        let image = try XCTUnwrap(context.makeImage())
        let data = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, [kCGImagePropertyOrientation: orientation] as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }

    private func noiseJPEG(side: Int) throws -> Data {
        var pixels = [UInt8](repeating: 0, count: side * side * 4)
        for index in pixels.indices { pixels[index] = UInt8.random(in: 0...255) }
        let context = try XCTUnwrap(CGContext(data: &pixels, width: side, height: side, bitsPerComponent: 8, bytesPerRow: side * 4,
                                              space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        let image = try XCTUnwrap(context.makeImage())
        let data = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, [kCGImageDestinationLossyCompressionQuality: 1.0] as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }
}

@MainActor
final class ExportTests: XCTestCase {
    func testStageExportCopiesByteExactOriginalsWithCompleteManifest() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        // 同名文件来自不同来源目录：导出必须去重文件名且逐字节保留。
        let firstSource = directory.appendingPathComponent("a", isDirectory: true)
        let secondSource = directory.appendingPathComponent("b", isDirectory: true)
        try FileManager.default.createDirectory(at: firstSource, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: secondSource, withIntermediateDirectories: true)
        let firstBytes = try jpeg(seed: 1), secondBytes = try jpeg(seed: 2)
        try firstBytes.write(to: firstSource.appendingPathComponent("frame.jpg"))
        try secondBytes.write(to: secondSource.appendingPathComponent("frame.jpg"))
        let scope = AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one")
        let root = directory.appendingPathComponent("library")
        let library = LocalLibrary(root: root, scope: scope)
        let project = try await library.createProject(title: "导出校验")
        guard case .imported(let photoA) = try await library.importFile(firstSource.appendingPathComponent("frame.jpg"), projectID: project.id),
              case .imported(let photoB) = try await library.importFile(secondSource.appendingPathComponent("frame.jpg"), projectID: project.id) else { return XCTFail() }
        try await library.select(projectID: project.id, photoID: photoA.id, selection: .KEEP)
        let digest = String(repeating: "b", count: 64)
        _ = try await library.beginMigration(projectID: project.id, workspaceID: "w")
        try await library.migrationEnrolled(projectID: project.id, cloudProjectID: "cloud-export", manifestDigest: digest)
        try await library.markUploaded(projectID: project.id, photoID: photoA.id)
        try await library.markUploaded(projectID: project.id, photoID: photoB.id)
        try await library.completeMigration(projectID: project.id)

        let staging = directory.appendingPathComponent("staging")
        let (exported, manifest) = try await library.stageExport(projectID: project.id, to: staging)
        XCTAssertEqual(manifest.version, 1)
        XCTAssertEqual(manifest.projectID, project.id)
        XCTAssertEqual(manifest.migration?.phase, "migrated")
        XCTAssertEqual(manifest.migration?.cloudProjectID, "cloud-export")
        XCTAssertEqual(Set(manifest.entries.map(\.exportedFilename)),
                       ["frame.jpg", "frame-2.jpg"], "同名原片导出文件名必须去重")
        let entryA = try XCTUnwrap(manifest.entries.first { $0.photoID == photoA.id })
        XCTAssertEqual(entryA.selection, "KEEP")
        XCTAssertEqual(entryA.sourceSHA256, photoA.sourceSHA256)
        XCTAssertEqual(entryA.format, photoA.format)
        XCTAssertEqual(try Data(contentsOf: exported.appendingPathComponent(entryA.exportedFilename)), firstBytes, "导出原片必须逐字节一致")
        let entryB = try XCTUnwrap(manifest.entries.first { $0.photoID == photoB.id })
        XCTAssertEqual(try Data(contentsOf: exported.appendingPathComponent(entryB.exportedFilename)), secondBytes)
        let decoded = try JSONDecoder().decode(LocalExportManifest.self,
                                               from: Data(contentsOf: exported.appendingPathComponent("manifest.json")))
        XCTAssertEqual(decoded.entries.count, 2)
    }

    func testStageExportRefusesCorruptOriginalAndCleansStaging() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("photo.jpg")
        try jpeg(seed: 3).write(to: source)
        let scope = AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one")
        let root = directory.appendingPathComponent("library")
        let library = LocalLibrary(root: root, scope: scope)
        let project = try await library.createProject(title: "损坏保护")
        guard case .imported(let photo) = try await library.importFile(source, projectID: project.id) else { return XCTFail() }
        try Data("tampered".utf8).write(to: root.appendingPathComponent(scope.key)
            .appendingPathComponent("media").appendingPathComponent(photo.sourceSHA256 + ".original"))
        let staging = directory.appendingPathComponent("staging")
        do { _ = try await library.stageExport(projectID: project.id, to: staging); XCTFail("损坏原片必须拒绝导出") }
        catch LocalLibraryError.corruptLibrary { }
        let leftovers = try FileManager.default.contentsOfDirectory(atPath: staging.path)
        XCTAssertTrue(leftovers.isEmpty, "失败导出不得留下半成品目录")
    }

    func testOfflineScopeRebuildProducesSameLibraryKey() throws {
        let configuration = try APIConfiguration("https://example.com")
        let online = AccountScope(configuration: configuration, userID: "user-1")
        let offline = AccountScope(server: configuration.identity, userID: "user-1")
        XCTAssertEqual(online.key, offline.key, "离线按登记身份重建必须命中同一资料目录")
    }

    private func jpeg(seed: Int) throws -> Data {
        let context = try XCTUnwrap(CGContext(data: nil, width: 96, height: 64, bitsPerComponent: 8, bytesPerRow: 0,
                                              space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        context.setFillColor(CGColor(red: 0.9, green: Double(seed) * 0.2, blue: 0.3, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 96, height: 64))
        let image = try XCTUnwrap(context.makeImage())
        let data = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }
}

@MainActor
final class ResearchDraftTests: XCTestCase {
    func testValidNoteDraftTrimsAndOmitsEmptyOptionals() throws {
        let draft = try ResearchDraft(kind: .note, title: "  研究笔记  ", body: " 盲页开场节奏参考。 ", author: "  ", source: nil, license: nil)
        XCTAssertEqual(draft.title, "研究笔记")
        XCTAssertEqual(draft.body, "盲页开场节奏参考。")
        XCTAssertNil(draft.author, "空白可选项按 nil 处理")
        let json = try JSONEncoder().encode(draft)
        let object = try XCTUnwrap(JSONSerialization.jsonObject(with: json) as? [String: Any])
        XCTAssertNil(object["url"], "笔记不得携带 url 键，服务端 strictObject 会拒绝未知键")
        XCTAssertNil(object["author"], "nil 可选项不得编码为 null")
    }

    func testValidLinkDraftRequiresHTTPURL() throws {
        let link = try ResearchDraft(kind: .link, title: "参考链接", url: " https://example.com/reference ")
        XCTAssertEqual(link.url, "https://example.com/reference")
        XCTAssertThrowsError(try ResearchDraft(kind: .link, title: "x", url: "not-a-url"))
        XCTAssertThrowsError(try ResearchDraft(kind: .link, title: "x", url: "ftp://example.com/file"))
        XCTAssertThrowsError(try ResearchDraft(kind: .link, title: "x", url: "https://"))  // 无 host
    }

    func testInvalidDraftsAreRejectedLocally() {
        XCTAssertThrowsError(try ResearchDraft(kind: .note, title: "", body: "内容"))
        XCTAssertThrowsError(try ResearchDraft(kind: .note, title: String(repeating: "长", count: 201), body: "内容"))
        XCTAssertThrowsError(try ResearchDraft(kind: .note, title: "标题"))  // 笔记缺正文
        XCTAssertThrowsError(try ResearchDraft(kind: .note, title: "标题", body: String(repeating: "字", count: 10_001)))
        XCTAssertThrowsError(try ResearchDraft(kind: .note, title: "标题", body: "内容", url: "https://example.com"))  // 笔记带链接字段
        XCTAssertThrowsError(try ResearchDraft(kind: .link, title: "标题", body: "内容", url: "https://example.com"))  // 链接带正文字段
    }

    func testPatchRequiresAtLeastOneFieldAndOmitsNil() throws {
        XCTAssertThrowsError(try ResearchPatch())
        let patch = try ResearchPatch(title: "新标题")
        let json = try JSONEncoder().encode(patch)
        let object = try XCTUnwrap(JSONSerialization.jsonObject(with: json) as? [String: Any])
        XCTAssertEqual(object.keys.sorted(), ["title"])
        XCTAssertThrowsError(try ResearchPatch(title: " "))
        XCTAssertThrowsError(try ResearchPatch(url: "not-a-url"))
        // 空串可选字段按“不修改”处理：服务端 min(1) 拒绝空串。
        let cleared = try ResearchPatch(title: "新标题", author: " ", source: "", license: nil)
        let clearedJSON = try JSONEncoder().encode(cleared)
        let clearedObject = try XCTUnwrap(JSONSerialization.jsonObject(with: clearedJSON) as? [String: Any])
        XCTAssertEqual(clearedObject.keys.sorted(), ["title"])
        XCTAssertThrowsError(try ResearchPatch(title: "新标题", author: String(repeating: "长", count: 201)))
    }

    func testResearchEntryDecodesListShape() throws {
        let json = Data("""
        {"id":"0b8dfc88-1111-2222-3333-444455556666","kind":"link","title":"参考","body":null,
         "url":"https://example.com","author":"作者","source":"来源","license":"许可",
         "created_by":"user-1","created_by_name":"评审者"}
        """.utf8)
        let entry = try JSONDecoder().decode(ResearchEntry.self, from: json)
        XCTAssertTrue(entry.isLink)
        XCTAssertNil(entry.body)
        XCTAssertEqual(entry.created_by_name, "评审者")
    }
}

@MainActor
final class PreviewCacheTests: XCTestCase {
    func testStoreRetrieveAndCorruptionHandling() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let cache = PreviewCache(directory: directory)
        let sha = String(repeating: "a", count: 64)
        let bytes = Data("jpeg-bytes".utf8)
        await cache.store(bytes, sha: sha)
        let hit = await cache.cached(sha: sha)
        XCTAssertEqual(hit, bytes)
        // 无效摘要与未命中返回 nil。
        let miss = await cache.cached(sha: String(repeating: "b", count: 64))
        XCTAssertNil(miss)
        let invalid = await cache.cached(sha: "not-a-sha")
        XCTAssertNil(invalid)
        // 文件被篡改后按未命中处理并清理。
        try Data("tampered-but-longer".utf8).write(to: directory.appendingPathComponent("\(sha).jpg"))
        let corrupted = await cache.cached(sha: sha)
        XCTAssertNil(corrupted)
        XCTAssertFalse(FileManager.default.fileExists(atPath: directory.appendingPathComponent("\(sha).jpg").path))
    }

    func testEvictionKeepsMostRecentlyUsedWithinCountBound() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let cache = PreviewCache(directory: directory)
        var shas: [String] = []
        // 先填到上限内，触碰最早的条目，再施加越限压力：淘汰应绕开最近使用者。
        for index in 0..<(PreviewCache.countLimit - 1) {
            let sha = String(format: "%064d", index)
            await cache.store(Data("bytes-\(index)".utf8), sha: sha)
            shas.append(sha)
        }
        _ = await cache.cached(sha: shas[0])
        for index in (PreviewCache.countLimit - 1)..<(PreviewCache.countLimit + 10) {
            let sha = String(format: "%064d", index)
            await cache.store(Data("bytes-\(index)".utf8), sha: sha)
            shas.append(sha)
        }
        let survivor = await cache.cached(sha: shas[0])
        XCTAssertNotNil(survivor, "最近使用的条目不应被淘汰")
        let evicted = await cache.cached(sha: shas[1])
        XCTAssertNil(evicted, "超限后最久未用条目应被淘汰")
        let latest = await cache.cached(sha: shas[shas.count - 1])
        XCTAssertNotNil(latest)
    }

    func testClearRemovesEverything() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let cache = PreviewCache(directory: directory)
        let sha = String(repeating: "c", count: 64)
        await cache.store(Data("x".utf8), sha: sha)
        await cache.clear()
        let after = await cache.cached(sha: sha)
        XCTAssertNil(after)
        XCTAssertFalse(FileManager.default.fileExists(atPath: directory.path))
        // 清理后仍可继续写入。
        await cache.store(Data("y".utf8), sha: sha)
        let rewritten = await cache.cached(sha: sha)
        XCTAssertEqual(rewritten, Data("y".utf8))
    }

    func testOverviewMergesSnapshotsAndPendingOnly() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let configuration = try APIConfiguration("https://example.com")
        let store = ScopedStore(root: root, scope: AccountScope(configuration: configuration, userID: "one"))
        let credentials = Credentials(token: String(repeating: "a", count: 64), csrf: "csrf",
                                       user: User(id: "one", email: "one@fr.test", name: "一"))
        let snapshot = ProjectSnapshot(project_id: "p-1", workspace_id: "w", title: "项目一", state: .SHARED,
                                       revision: 3, cursor: "9",
                                       snapshot: SharedSnapshot(
                                         photos: [Photo(id: "photo-1", filename: "f.jpg",
                                                        source_sha256: String(repeating: "a", count: 64), selection: .UNDECIDED)],
                                         sequences: [PhotoSequence(id: "seq", title: "序列", items: [])]),
                                       principal: Principal(id: "one", role: "OWNER", can_approve: true, can_edit_formal: true),
                                       previews: [], comments: [])
        // saveProjects 会按项目清单清理快照，先写清单再写快照。
        try await store.saveProjects([
            ProjectSummary(id: "p-1", workspace_id: "w", title: "项目一", revision: 3, state: .SHARED),
            ProjectSummary(id: "p-2", workspace_id: "w", title: "项目二", revision: 0, state: .SHARED),
        ])
        try await store.saveSnapshot(snapshot)
        let overview = try await store.overview()
        XCTAssertEqual(overview.count, 1)
        XCTAssertEqual(overview.first?.projectID, "p-1")
        XCTAssertEqual(overview.first?.revision, 3)
        XCTAssertNil(overview.first?.pending)
    }
}

@MainActor
final class MediaMatrixTests: XCTestCase {
    func testImportMatrixAcrossFormatsAndOrientations() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let library = LocalLibrary(root: directory.appendingPathComponent("library"),
                                   scope: AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one"))
        let project = try await library.createProject(title: "媒体矩阵")
        var formats: [UTType] = [.jpeg, .png, .tiff]
        if (CGImageDestinationCopyTypeIdentifiers() as? [String])?.contains(UTType.heic.identifier) == true {
            formats.append(.heic)
        }
        var imported = 0
        for type in formats {
            for orientation in 1...8 {
                let file = directory.appendingPathComponent("m-\(type.identifier ?? "x")-\(orientation).img")
                try matrixImage(type: type, orientation: orientation).write(to: file)
                let outcome = try await library.importFile(file, projectID: project.id)
                guard case .imported(let photo) = outcome else { return XCTFail("矩阵导入失败：\(type) 方向 \(orientation)") }
                XCTAssertEqual(photo.orientation, orientation)
                XCTAssertEqual(photo.width, 96)
                XCTAssertEqual(photo.height, 64)
                XCTAssertLessThanOrEqual(max(photo.previewWidth, photo.previewHeight), 2048)
                let preview = try await library.preview(projectID: project.id, photoID: photo.id)
                let properties = try XCTUnwrap(CGImageSourceCopyPropertiesAtIndex(
                    XCTUnwrap(CGImageSourceCreateWithData(preview as CFData, nil)), 0, nil) as? [CFString: Any])
                XCTAssertEqual((properties[kCGImagePropertyOrientation] as? Int) ?? 1, 1, "预览方向必须归正")
                imported += 1
            }
        }
        XCTAssertEqual(imported, formats.count * 8)
        let projects = try await library.projects()
        XCTAssertEqual(projects.first?.photos.count, imported, "矩阵去重后应逐张保留")
    }

    func testPixelLimitRejectsOversizedFlatImage() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let library = LocalLibrary(root: directory.appendingPathComponent("library"),
                                   scope: AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one"))
        let project = try await library.createProject(title: "像素上限")
        let file = directory.appendingPathComponent("huge-flat.jpg")
        try flatImage(width: 9_000, height: 6_000).write(to: file)
        let bytes = try Data(contentsOf: file)
        XCTAssertLessThan(bytes.count, 1024 * 1024, "纯色大图应足够小，确保命中的是像素上限而非字节上限")
        do { _ = try await library.importFile(file, projectID: project.id); XCTFail("超过 5000 万像素必须拒绝") }
        catch LocalLibraryError.tooLarge { }
        let projects = try await library.projects()
        XCTAssertTrue(projects.first?.photos.isEmpty == true)
    }

    private func matrixImage(type: UTType, orientation: Int) throws -> Data {
        let context = try XCTUnwrap(CGContext(data: nil, width: 96, height: 64, bitsPerComponent: 8, bytesPerRow: 0,
                                              space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        context.setFillColor(CGColor(red: 0.3, green: 0.5, blue: 0.7, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 96, height: 64))
        let image = try XCTUnwrap(context.makeImage())
        let data = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(data, type.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, [kCGImagePropertyOrientation: orientation] as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }

    private func flatImage(width: Int, height: Int) throws -> Data {
        let context = try XCTUnwrap(CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                                              space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        context.setFillColor(CGColor(red: 0.5, green: 0.5, blue: 0.5, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: width, height: height))
        let image = try XCTUnwrap(context.makeImage())
        let data = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }
}

@MainActor
final class CanvasProposalCommandTests: XCTestCase {
    func testCanvasMoveCommandEncodingAndReceipt() throws {
        let board = Board(id: UUID().uuidString, project_id: "p-1", title: "画布", state: "draft", revision: 7)
        let nodeID = UUID().uuidString
        let command = try CanvasMoveCommand(board: board, nodeID: nodeID, x: 120.5, y: -40)
        XCTAssertEqual(command.schemaVersion, 3)
        XCTAssertEqual(command.baseRevision, 7)
        XCTAssertEqual(command.operations.count, 1)
        XCTAssertEqual(command.operations.first?.action.type, "moveNodes")
        let json = try JSONEncoder().encode(command)
        let object = try XCTUnwrap(JSONSerialization.jsonObject(with: json) as? [String: Any])
        XCTAssertEqual(Set(object.keys), ["schemaVersion", "commandId", "idempotencyKey", "projectId", "baseRevision", "operations"])
        let receipt = CanvasCommandReceipt(ok: true, commandId: command.commandId, newRevision: 8, affectedNodeIds: [nodeID])
        XCTAssertNoThrow(try receipt.validate(for: command))
        XCTAssertThrowsError(try CanvasCommandReceipt(ok: true, commandId: command.commandId, newRevision: 9, affectedNodeIds: []).validate(for: command))
        XCTAssertThrowsError(try CanvasCommandReceipt(ok: true, commandId: "other", newRevision: 8, affectedNodeIds: []).validate(for: command))
        XCTAssertThrowsError(try CanvasCommandReceipt(ok: true, commandId: command.commandId, newRevision: 8, affectedNodeIds: ["other-node"]).validate(for: command))
    }

    func testProposalDecode() throws {
        let json = Data("""
        {"id":"7a1","title":"中段节奏","author":"编辑","state":"SUBMITTED","base_revision":2,"revision":0,
         "operations":[{"op":"move","operation_id":"op-1","photo_id":"ph-1","to_index":2},
                       {"op":"remove","operation_id":"op-2","photo_id":"ph-2"}],
         "nonce":"nonce-1","project_id":"p-1","author_id":"u-1"}
        """.utf8)
        let proposal = try JSONDecoder().decode(Proposal.self, from: json)
        XCTAssertTrue(proposal.isSubmitted)
        XCTAssertEqual(proposal.operations.count, 2)
        XCTAssertEqual(proposal.operations.first?.to_index, 2)
        XCTAssertNil(proposal.operations.last?.to_index)
    }

    func testProposalApplyGuardsAndEncoding() throws {
        let snapshot = ProjectSnapshot(
            project_id: "p-1", workspace_id: "w-1", title: "项目", state: .SHARED, revision: 5, cursor: "1",
            snapshot: SharedSnapshot(
                photos: [Photo(id: "ph-1", filename: "a.jpg", source_sha256: String(repeating: "a", count: 64), selection: .UNDECIDED)],
                sequences: [PhotoSequence(id: "s", title: "序列", items: [])]),
            principal: Principal(id: "u-1", role: "OWNER", can_approve: true, can_edit_formal: true),
            previews: [], comments: [], proposals: nil)
        let proposal = Proposal(id: "pr-1", title: "提案", author: "编辑", state: "SUBMITTED",
                                base_revision: 5, revision: 0,
                                operations: [Proposal.Operation(op: "add", operation_id: "op-1", photo_id: "ph-1", to_index: 0)],
                                nonce: "n-1")
        let command = try ProposalApplyCommand(snapshot: snapshot, proposal: proposal, acceptedOperationIDs: ["op-1"])
        XCTAssertEqual(command.kind, "proposal.apply")
        XCTAssertEqual(command.expected_revision, 5)
        XCTAssertEqual(command.payload.accepted_operation_ids, ["op-1"])
        let json = try JSONEncoder().encode(command)
        let object = try XCTUnwrap(JSONSerialization.jsonObject(with: json) as? [String: Any])
        XCTAssertEqual(Set(object.keys), ["version", "command_id", "workspace_id", "project_id", "expected_revision", "idempotency_key", "kind", "payload"])
        // 未知操作编号与不可审批权限直接拒绝。
        XCTAssertThrowsError(try ProposalApplyCommand(snapshot: snapshot, proposal: proposal, acceptedOperationIDs: ["op-x"]))
        let viewerSnapshot = ProjectSnapshot(
            project_id: "p-1", workspace_id: "w-1", title: "项目", state: .SHARED, revision: 5, cursor: "1",
            snapshot: snapshot.snapshot,
            principal: Principal(id: "u-1", role: "VIEWER", can_approve: false, can_edit_formal: false),
            previews: [], comments: [], proposals: nil)
        XCTAssertThrowsError(try ProposalApplyCommand(snapshot: viewerSnapshot, proposal: proposal, acceptedOperationIDs: []))
    }
}

@MainActor
final class AIConversationTests: XCTestCase {
    func testConversationInputEncodingMatchesContract() throws {
        let input = AIConversationInput(history: [("user", "帮我看看节奏"), ("assistant", "好的，先看整体。"), ("user", "中段太密")],
                                        instructions: "你是摄影编辑助手", maxOutputTokens: 2048)
        let object = try XCTUnwrap(JSONSerialization.jsonObject(with: JSONEncoder().encode(input)) as? [String: Any])
        XCTAssertEqual(Set(object.keys), ["model", "input", "instructions", "max_output_tokens", "store"])
        XCTAssertEqual(object["model"] as? String, "deepseek-flash")
        XCTAssertEqual(object["store"] as? Bool, false)
        XCTAssertEqual(object["max_output_tokens"] as? Int, 2048)
        let messages = try XCTUnwrap(object["input"] as? [[String: Any]])
        XCTAssertEqual(messages.count, 3)
        XCTAssertEqual(messages.allSatisfy { $0["type"] as? String == "message" }, true)
        let userParts = try XCTUnwrap(messages[0]["content"] as? [[String: Any]])
        XCTAssertEqual(userParts[0]["type"] as? String, "input_text")
        let assistantParts = try XCTUnwrap(messages[1]["content"] as? [[String: Any]])
        XCTAssertEqual(assistantParts[0]["type"] as? String, "output_text")
        let bare = try XCTUnwrap(JSONSerialization.jsonObject(
            with: JSONEncoder().encode(AIConversationInput(history: [("user", "你好")]))) as? [String: Any])
        XCTAssertNil(bare["instructions"], "无指令时键应省略，服务端 strictObject 不受影响")
    }

    func testReplyTextExtractionAndValidation() throws {
        let json = Data("""
        {"id":"resp-1","status":"completed","model":"deepseek-flash",
         "output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"第一段"}]},
                   {"type":"reasoning","content":[]},
                   {"type":"message","role":"assistant","content":[{"type":"output_text","text":"第二段"}]}],
         "usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15}}
        """.utf8)
        let reply = try JSONDecoder().decode(AIReply.self, from: json)
        XCTAssertEqual(reply.text, "第一段\n第二段")
        XCTAssertNoThrow(try reply.validate())
        let badJSON = Data("""
        {"id":"resp-2","status":"failed","model":"deepseek-flash","output":[],
         "usage":{"input_tokens":10,"output_tokens":5,"total_tokens":20}}
        """.utf8)
        let badReply = try JSONDecoder().decode(AIReply.self, from: badJSON)
        XCTAssertThrowsError(try badReply.validate(), "非完成状态与计量不符都应拒绝")
    }

    func testChatStoreRoundtripTrimReplaceAndCorruption() async throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let scope = AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one")
        let store = AIChatStore(root: root, scope: scope)
        try await store.append(AIChatMessage(id: "m-1", role: "user", text: "你好", requestID: "r-1", state: .unknown))
        try await store.append(AIChatMessage(id: "m-2", role: "assistant", text: "回复"))
        var loaded = try await store.messages()
        XCTAssertEqual(loaded.map(\.id), ["m-1", "m-2"])

        // 重启（新实例）后凭账户范围恢复。
        let reopened = AIChatStore(root: root, scope: scope)
        loaded = try await reopened.messages()
        XCTAssertEqual(loaded.count, 2)
        try await reopened.replace(AIChatMessage(id: "m-1", role: "user", text: "你好", requestID: "r-1", state: .sent))
        loaded = try await reopened.messages()
        XCTAssertEqual(loaded.first?.state, .sent)
        do { try await reopened.replace(AIChatMessage(id: "missing", role: "user", text: "x")); XCTFail("未知编号不得替换") }
        catch APIError.unconfirmedCommand { }

        for index in 0..<(AIChatStore.messageLimit + 5) {
            try await reopened.append(AIChatMessage(id: "extra-\(index)", role: "user", text: "消息\(index)"))
        }
        loaded = try await reopened.messages()
        XCTAssertEqual(loaded.count, AIChatStore.messageLimit, "超出上限应裁剪最旧消息")
        XCTAssertEqual(loaded.last?.id, "extra-\(AIChatStore.messageLimit + 4)")

        let files = try FileManager.default.contentsOfDirectory(atPath: root.path)
        let chatFile = try XCTUnwrap(files.first(where: { $0.hasSuffix(".json") }))
        try Data("broken".utf8).write(to: root.appendingPathComponent(chatFile))
        do { _ = try await reopened.messages(); XCTFail("损坏对话应拒绝读取") }
        catch APIError.invalidResponse { }

        try await reopened.clear()
        let cleared = try await reopened.messages()
        XCTAssertTrue(cleared.isEmpty)
    }
}

@MainActor
final class ReadingSurfaceTests: XCTestCase {
    func testJSONValueDecodingAndDisplay() throws {
        let json = Data("""
        {"s":"KEEP","n":42,"f":3.5,"t":true,"z":null,"a":[1,"x"],"o":{"k":"v"}}
        """.utf8)
        let object = try JSONDecoder().decode([String: JSONValue].self, from: json)
        XCTAssertEqual(object["s"], .string("KEEP"))
        XCTAssertEqual(object["n"], .number(42))
        XCTAssertEqual(object["t"], .bool(true))
        XCTAssertEqual(object["z"], .null)
        XCTAssertEqual(object["a"], .array([.number(1), .string("x")]))
        XCTAssertEqual(object["s"]?.displayText, "KEEP")
        XCTAssertEqual(object["n"]?.displayText, "42")
        XCTAssertEqual(object["f"]?.displayText, "3.5")
        XCTAssertEqual(object["t"]?.displayText, "真")
        XCTAssertEqual(object["z"]?.displayText, "空")
        XCTAssertEqual(object["a"]?.displayText, "（列表变更）")
        // 往返编码保持等价
        let roundTrip = try JSONDecoder().decode([String: JSONValue].self, from: JSONEncoder().encode(object))
        XCTAssertEqual(roundTrip, object)
    }

    func testVersionDiffDecoding() throws {
        let json = Data("""
        {"project_id":"p-1",
         "from":{"revision":0,"created_at":"2026-10-28T10:00:00Z"},
         "to":{"revision":2,"created_at":"2026-10-28T11:00:00Z"},
         "photos":{"added":[],"removed":["ph-9"],
                   "changed":[{"id":"ph-1","fields":{"selection":{"before":"UNDECIDED","after":"KEEP"}}}]},
         "sequences":{"added":["seq-2"],"removed":[],"changed":[]}}
        """.utf8)
        let diff = try JSONDecoder().decode(VersionDiff.self, from: json)
        XCTAssertEqual(diff.from.revision, 0)
        XCTAssertEqual(diff.to.revision, 2)
        XCTAssertEqual(diff.photos.removed, ["ph-9"])
        let changed = try XCTUnwrap(diff.photos.changed.first)
        XCTAssertEqual(changed.fields["selection"]?.before, .string("UNDECIDED"))
        XCTAssertEqual(changed.fields["selection"]?.after, .string("KEEP"))
        XCTAssertEqual(changed.fields["selection"]?.after?.displayText, "KEEP")
        XCTAssertEqual(diff.sequences.added, ["seq-2"])
    }

    func testProposalDiffDecodingBothBranches() throws {
        let fresh = Data("""
        {"proposalId":"pr-1","state":"SUBMITTED","baseRevision":2,"currentRevision":2,"stale":false,
         "sequence":{"id":"seq-1","before":["ph-1","ph-2"],"after":["ph-1","ph-3","ph-2"],
                     "added":["ph-3"],"removed":[],
                     "moved":[{"photo_id":"ph-2","fromIndex":1,"toIndex":2}]}}
        """.utf8)
        let freshDiff = try JSONDecoder().decode(ProposalDiff.self, from: fresh)
        XCTAssertFalse(freshDiff.stale)
        let change = try XCTUnwrap(freshDiff.sequence)
        XCTAssertEqual(change.before, ["ph-1", "ph-2"])
        XCTAssertEqual(change.after, ["ph-1", "ph-3", "ph-2"])
        XCTAssertEqual(change.added, ["ph-3"])
        XCTAssertEqual(change.moved.first?.photo_id, "ph-2")
        XCTAssertEqual(change.moved.first?.toIndex, 2)
        let stale = Data("""
        {"proposalId":"pr-2","state":"SUBMITTED","baseRevision":1,"currentRevision":3,"stale":true,"sequence":null}
        """.utf8)
        let staleDiff = try JSONDecoder().decode(ProposalDiff.self, from: stale)
        XCTAssertTrue(staleDiff.stale)
        XCTAssertNil(staleDiff.sequence)
    }

    func testBillingCatalogDecoding() throws {
        let json = Data("""
        {"salesAvailable":false,
         "plans":[{"id":"free","name":"基础版","description":"","features":["local","selection"],"prices":[]},
                  {"id":"pro","name":"FocalRoom Pro","description":"","features":["ai"],
                   "prices":[{"id":"price-1","interval":"month","amount":1998,"currency":"CNY","includedCredits":2000}]}]}
        """.utf8)
        let catalog = try JSONDecoder().decode(BillingCatalog.self, from: json)
        XCTAssertFalse(catalog.salesAvailable)
        XCTAssertEqual(catalog.plans.map(\.id), ["free", "pro"])
        XCTAssertEqual(catalog.plans[1].prices.first?.includedCredits, 2000)
        XCTAssertEqual(catalog.plans[1].prices.first?.amount, 1998)
    }
}

@MainActor
final class LocalCanvasTests: XCTestCase {
    func testCanvasOperationsRoundtripUndoAndRestart() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("photo.jpg")
        try jpeg().write(to: source)
        let root = directory.appendingPathComponent("library")
        let scope = AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one")
        let library = LocalLibrary(root: root, scope: scope)
        let project = try await library.createProject(title: "画布项目")
        guard case .imported(let photo) = try await library.importFile(source, projectID: project.id) else { return XCTFail() }

        // 新建画布：照片节点 + 便签 + 批注
        let photoNode = LocalCanvasNode(id: UUID().uuidString.lowercased(), kind: .photo, photoID: photo.id,
                                        x: 0, y: 0, width: 240, height: 180, zOrder: 0)
        let noteNode = LocalCanvasNode(id: UUID().uuidString.lowercased(), kind: .note, text: "开场笔记",
                                       x: 300, y: 40, width: 200, height: 120, zOrder: 1)
        let undoRemovePhoto = try await library.applyCanvasOperation(projectID: project.id, .createNode(photoNode))
        _ = try await library.applyCanvasOperation(projectID: project.id, .createNode(noteNode))
        let annotation = LocalCanvasAnnotation(id: UUID().uuidString.lowercased(), nodeID: photoNode.id, body: "这里裁掉边缘")
        _ = try await library.applyCanvasOperation(projectID: project.id, .createAnnotation(annotation))
        var canvas = try await library.canvas(projectID: project.id)
        XCTAssertEqual(canvas?.nodes.count, 2)
        XCTAssertEqual(canvas?.annotations.count, 1)

        // 移动 + 撤销：逆操作精确还原坐标
        let inverseMove = try await library.applyCanvasOperation(projectID: project.id,
                                                                 .moveNode(id: photoNode.id, x: 500, y: 320))
        canvas = try await library.canvas(projectID: project.id)
        XCTAssertEqual(canvas?.nodes.first { $0.id == photoNode.id }?.x, 500)
        _ = try await library.applyCanvasOperation(projectID: project.id, inverseMove)
        canvas = try await library.canvas(projectID: project.id)
        XCTAssertEqual(canvas?.nodes.first { $0.id == photoNode.id }?.x, 0)

        // 删除照片节点连批注一起删，撤销完整恢复
        let undoRemove = try await library.applyCanvasOperation(projectID: project.id, .removeNode(id: photoNode.id))
        canvas = try await library.canvas(projectID: project.id)
        XCTAssertEqual(canvas?.nodes.count, 1)
        XCTAssertEqual(canvas?.annotations.count, 0)
        _ = try await library.applyCanvasOperation(projectID: project.id, undoRemove)
        canvas = try await library.canvas(projectID: project.id)
        XCTAssertEqual(canvas?.nodes.count, 2)
        XCTAssertEqual(canvas?.annotations.count, 1)

        // 重启（新实例）后画布持久
        let reopened = LocalLibrary(root: root, scope: scope)
        let restored = try await reopened.canvas(projectID: project.id)
        XCTAssertEqual(restored?.nodes.map(\.id).sorted(), [photoNode.id, noteNode.id].sorted())
        XCTAssertEqual(restored?.annotations.first?.body, "这里裁掉边缘")

        // 旧库无画布字段 → canvas 为 nil，兼容解码
        let emptyProject = try await reopened.createProject(title: "无画布")
        let none = try await reopened.canvas(projectID: emptyProject.id)
        XCTAssertNil(none)
    }

    func testCanvasRejectsInvalidReferencesAndBounds() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let source = directory.appendingPathComponent("photo.jpg")
        try jpeg().write(to: source)
        let scope = AccountScope(configuration: try APIConfiguration("https://example.com"), userID: "one")
        let library = LocalLibrary(root: directory.appendingPathComponent("library"), scope: scope)
        let project = try await library.createProject(title: "校验")
        guard case .imported(let photo) = try await library.importFile(source, projectID: project.id) else { return XCTFail() }

        // 照片引用不存在
        let ghost = LocalCanvasNode(id: UUID().uuidString.lowercased(), kind: .photo, photoID: UUID().uuidString.lowercased(),
                                    x: 0, y: 0, width: 240, height: 180, zOrder: 0)
        do { _ = try await library.applyCanvasOperation(projectID: project.id, .createNode(ghost)); XCTFail("幽灵照片引用必须拒绝") }
        catch LocalLibraryError.corruptLibrary { }
        // 坐标越界
        let far = LocalCanvasNode(id: UUID().uuidString.lowercased(), kind: .note, text: "x",
                                  x: 1_000_000, y: 0, width: 200, height: 120, zOrder: 0)
        do { _ = try await library.applyCanvasOperation(projectID: project.id, .createNode(far)); XCTFail("越界坐标必须拒绝") }
        catch LocalLibraryError.corruptLibrary { }
        // 尺寸过小
        let tiny = LocalCanvasNode(id: UUID().uuidString.lowercased(), kind: .note, text: "x",
                                   x: 0, y: 0, width: 10, height: 10, zOrder: 0)
        do { _ = try await library.applyCanvasOperation(projectID: project.id, .createNode(tiny)); XCTFail("过小尺寸必须拒绝") }
        catch LocalLibraryError.corruptLibrary { }
        // 对照片节点改文本
        let photoNode = LocalCanvasNode(id: UUID().uuidString.lowercased(), kind: .photo, photoID: photo.id,
                                        x: 0, y: 0, width: 240, height: 180, zOrder: 0)
        _ = try await library.applyCanvasOperation(projectID: project.id, .createNode(photoNode))
        do { _ = try await library.applyCanvasOperation(projectID: project.id, .updateNoteText(id: photoNode.id, text: "不该允许")); XCTFail("照片节点无文本") }
        catch LocalLibraryError.invalidProject { }
        // 批注锚定不存在的节点
        let orphan = LocalCanvasAnnotation(id: UUID().uuidString.lowercased(), nodeID: UUID().uuidString.lowercased(), body: "孤儿")
        do { _ = try await library.applyCanvasOperation(projectID: project.id, .createAnnotation(orphan)); XCTFail("孤儿批注必须拒绝") }
        catch LocalLibraryError.corruptLibrary { }
        // 迁云后画布冻结
        _ = try await library.beginMigration(projectID: project.id, workspaceID: "w")
        do { _ = try await library.applyCanvasOperation(projectID: project.id, .moveNode(id: photoNode.id, x: 1, y: 1)); XCTFail("迁云期间画布冻结") }
        catch LocalLibraryError.projectFrozen { }
    }

    private func jpeg() throws -> Data {
        let context = try XCTUnwrap(CGContext(data: nil, width: 96, height: 64, bitsPerComponent: 8, bytesPerRow: 0,
                                              space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        context.setFillColor(CGColor(red: 0.4, green: 0.6, blue: 0.3, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: 96, height: 64))
        let image = try XCTUnwrap(context.makeImage())
        let data = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(data, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, image, nil)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }
}

@MainActor
final class PhotoTransformTests: XCTestCase {
    func testLockedAndIndependentZoomAndPosition() {
        var state = ComparisonState()
        let transformed = PhotoTransform().magnified(by: 2).translated(x: 0.3, y: -0.2)
        state.update(.left, transform: transformed)
        XCTAssertEqual(state.left, state.right)
        state.setLocked(false)
        state.update(.right, transform: state.right.magnified(by: 2))
        XCTAssertEqual(state.left.zoom, 2)
        XCTAssertEqual(state.right.zoom, 4)
        state.setLocked(true)
        XCTAssertEqual(state.left, state.right)
        state.reset()
        XCTAssertEqual(state.left, PhotoTransform())
        XCTAssertEqual(state.right, PhotoTransform())
    }

    func testTransformBoundsAndResetAfterZoomOut() {
        let original = PhotoTransform()
        XCTAssertEqual(original.magnified(by: .nan), original)
        XCTAssertEqual(original.magnified(by: .infinity), original)
        let moved = original.magnified(by: 100).translated(x: 100, y: -100)
        XCTAssertEqual(moved.zoom, 5)
        XCTAssertEqual(moved.x, 1)
        XCTAssertEqual(moved.y, -1)
        XCTAssertEqual(moved.magnified(by: 0.001), original)
    }
}
