import Foundation
import SwiftUI
import Observation
import PhotosUI
import CoreTransferable
import UniformTypeIdentifiers
import FocalRoomCore

private struct PickedPhotoFile: Transferable {
    let url: URL

    static var transferRepresentation: some TransferRepresentation {
        FileRepresentation(importedContentType: .image) { received in
            Self(url: try ImportStaging.copy(received.file))
        }
    }
}

struct ExportTarget: Identifiable, Equatable, Sendable {
    let id = UUID()
    let staging: URL
    let title: String
    let count: Int
}

@MainActor @Observable
final class LocalLibraryModel {
    private(set) var projects: [LocalProject] = []
    private(set) var isImporting = false
    private(set) var isSaving = false
    private(set) var completed = 0
    private(set) var total = 0
    private(set) var imported = 0
    private(set) var duplicates = 0
    private(set) var failures: [String] = []
    private(set) var wasCancelled = false
    private(set) var importProjectID: String?
    private(set) var isExporting = false
    private(set) var exportTarget: ExportTarget?
    var notice: String?

    @ObservationIgnored let library: LocalLibrary
    @ObservationIgnored let readOnly: Bool
    @ObservationIgnored private let images = NSCache<NSString, UIImage>()
    @ObservationIgnored private var importTask: Task<Void, Never>?
    @ObservationIgnored private var active = true

    // LocalLibrary 是每文件一次读改写的 actor；迁云与本机导入必须共享同一实例，避免整文件互相覆盖。
    // readOnly 用于未登录的本机资料离线浏览：只读原片与记录，不提供导入、选片或迁云写入。
    init(library: LocalLibrary, readOnly: Bool = false) {
        self.library = library
        self.readOnly = readOnly
        images.totalCostLimit = 64 * 1024 * 1024
    }

    func reload() async {
        guard active else { return }
        do {
            let values = try await library.projects()
            guard active else { return }
            projects = values
        } catch { if active { notice = message(error) } }
    }

    func createProject(title: String) async -> String? {
        guard active, !isSaving, !readOnly else { return nil }
        isSaving = true
        defer { isSaving = false }
        do {
            let project = try await library.createProject(title: title)
            guard active else { return nil }
            await reload()
            return project.id
        } catch { notice = message(error); return nil }
    }

    func importFiles(_ urls: [URL], projectID: String) {
        // 文件导入没有系统相册身份，保留时会说明无法自动同步。
        startImport(projectID: projectID, count: urls.count) { (urls[$0], false, nil) }
    }

    func importPhotos(_ items: [PhotosPickerItem], projectID: String) {
        startImport(projectID: projectID, count: items.count) { index in
            guard let file = try await items[index].loadTransferable(type: PickedPhotoFile.self) else {
                throw LocalLibraryError.invalidFile
            }
            // 相册导入记录 PHAsset 身份，供「保留」同步系统相册收藏。
            return (file.url, true, items[index].itemIdentifier)
        }
    }

    private func startImport(projectID: String, count: Int, source: @escaping (Int) async throws -> (URL, Bool, String?)) {
        guard active, !isImporting, count > 0, !readOnly else {
            if readOnly { notice = "离线只读模式不能导入照片；请登录后操作。" }
            return
        }
        guard count <= 20 else { notice = "每批最多导入 20 张照片。"; return }
        isImporting = true
        importProjectID = projectID
        completed = 0; total = count; imported = 0; duplicates = 0; failures = []; wasCancelled = false
        importTask = Task {
            defer { isImporting = false; importTask = nil }
            for index in 0..<count {
                do {
                    try Task.checkCancellation()
                    let (url, staged, assetID) = try await source(index)
                    defer { if staged { ImportStaging.removeCopy(url) } }
                    let scoped = url.startAccessingSecurityScopedResource()
                    defer { if scoped { url.stopAccessingSecurityScopedResource() } }
                    let outcome = try await library.importFile(url, projectID: projectID, phAssetID: assetID)
                    guard active else { return }
                    switch outcome {
                    case .imported: imported += 1
                    case .duplicate: duplicates += 1
                    }
                    completed += 1
                    await reload()
                } catch is CancellationError {
                    wasCancelled = true
                    break
                } catch {
                    guard active else { return }
                    completed += 1
                    failures.append("第 \(index + 1) 张：\(message(error))")
                }
            }
            if active { await reload() }
        }
    }

    func cancelImport() { importTask?.cancel() }

    func prepareExport(projectID: String) async {
        guard active, !isExporting, exportTarget == nil else { return }
        guard let project = projects.first(where: { $0.id == projectID }), !project.photos.isEmpty else {
            notice = "项目没有可导出的照片。"
            return
        }
        isExporting = true
        defer { isExporting = false }
        do {
            let (directory, manifest) = try await library.stageExport(projectID: projectID,
                                                                      to: FileManager.default.temporaryDirectory)
            guard active else { try? FileManager.default.removeItem(at: directory); return }
            exportTarget = ExportTarget(staging: directory, title: manifest.title, count: manifest.entries.count)
        } catch { notice = message(error) }
    }

    // 选择目标文件夹的瞬间先清空 exportTarget，随后的系统收起回调因此不会误删暂存。
    func pickExportDestination(_ folder: URL) {
        guard let target = exportTarget else { return }
        exportTarget = nil
        Task { await completeExport(target: target, to: folder) }
    }

    func cancelExport() {
        guard let target = exportTarget else { return }
        exportTarget = nil
        try? FileManager.default.removeItem(at: target.staging)
    }

    private func completeExport(target: ExportTarget, to folder: URL) async {
        do {
            let destination = try await Task.detached(priority: .userInitiated) { () throws -> URL in
                let scoped = folder.startAccessingSecurityScopedResource()
                defer { if scoped { folder.stopAccessingSecurityScopedResource() } }
                let destination = try Self.uniqueDestination(in: folder, title: target.title)
                for item in try FileManager.default.contentsOfDirectory(atPath: target.staging.path) {
                    try FileManager.default.copyItem(at: target.staging.appendingPathComponent(item),
                                                     to: destination.appendingPathComponent(item))
                }
                try? FileManager.default.removeItem(at: target.staging)
                return destination
            }.value
            notice = "已导出 \(target.count) 份原片与 manifest.json 到「\(destination.lastPathComponent)」。"
        } catch {
            try? FileManager.default.removeItem(at: target.staging)
            notice = "导出未能完成，请重试并确认目标文件夹可写。"
        }
    }

    nonisolated private static func uniqueDestination(in folder: URL, title: String) throws -> URL {
        let formatter = DateFormatter()
        formatter.dateFormat = "MMdd-HHmm"
        let cleaned = title.replacingOccurrences(of: "/", with: "-")
            .replacingOccurrences(of: ":", with: "-")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let base = "FocalRoom-" + (cleaned.isEmpty ? "项目" : String(cleaned.prefix(60))) + "-" + formatter.string(from: Date())
        var candidate = folder.appendingPathComponent(base, isDirectory: true)
        var index = 2
        while FileManager.default.fileExists(atPath: candidate.path) {
            candidate = folder.appendingPathComponent("\(base)-\(index)", isDirectory: true)
            index += 1
        }
        try FileManager.default.createDirectory(at: candidate, withIntermediateDirectories: true)
        return candidate
    }

    @discardableResult
    func select(projectID: String, photoID: String, selection: Selection) async -> Bool {
        guard active, !isSaving else { return false }
        if readOnly { notice = "离线只读模式不修改选片；请登录后操作。"; return false }
        isSaving = true
        defer { isSaving = false }
        do {
            // 先取照片身份（相册导入的带 PHAsset 标识）供收藏同步。
            let photo = projects.first(where: { $0.id == projectID })?.photos.first(where: { $0.id == photoID })
            try await library.select(projectID: projectID, photoID: photoID, selection: selection)
            // 选片关键路径不等收藏同步：同步（含相册权限弹窗）放后台，失败提示回主界面不阻塞选片与翻页。
            if selection == .KEEP, let photo {
                Task { @MainActor [weak self] in
                    if let problem = await PhotoFavorites.syncKeptPhoto(photo), let self, self.active {
                        self.notice = problem
                    }
                }
            }
            if active { await reload() }
            return true
        } catch { if active { notice = message(error) }; return false }
    }

    func reportPickerFailure(_ error: Error) {
        if (error as NSError).domain == NSCocoaErrorDomain && (error as NSError).code == NSUserCancelledError { return }
        notice = message(error)
    }

    private func message(_ error: Error) -> String {
        if let error = error as? LocalLibraryError { return error.localizedDescription }
        return "无法读写所选文件，请检查可用空间或重新选择；云端照片可先下载到设备后重试。"
    }

    func image(projectID: String, photoID: String) async throws -> UIImage {
        guard active, let photo = projects.first(where: { $0.id == projectID })?.photos.first(where: { $0.id == photoID }) else {
            throw LocalLibraryError.invalidProject
        }
        let key = photo.previewSHA256 as NSString
        if let image = images.object(forKey: key) { return image }
        let data = try await library.preview(projectID: projectID, photoID: photoID)
        try Task.checkCancellation()
        guard active, let image = UIImage(data: data) else { throw LocalLibraryError.invalidFile }
        images.setObject(image, forKey: key, cost: photo.previewWidth * photo.previewHeight * 4)
        return image
    }

    func shutdown() {
        active = false
        importTask?.cancel()
        images.removeAllObjects()
        projects = []
    }
}
