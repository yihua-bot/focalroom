import Foundation
import SwiftUI
import Observation
import FocalRoomCore

enum MigrationError: Error, LocalizedError {
    case noWorkspace, cannotAbandonCompleted

    var errorDescription: String? {
        switch self {
        case .noWorkspace: "当前账户没有可发布项目的工作区权限。"
        case .cannotAbandonCompleted: "云端已完成提交，不能取消；请继续完成迁云确认。"
        }
    }
}

// 迁云驱动：promote(幂等) → 逐张上传预览(幂等) → finalize；全部沿用记录中的原始清单字节与服务端回执。
// 中断后记录保留进度，继续/重试不会创建第二个云端项目；显式放弃先核对服务端状态再 abort。
@MainActor @Observable
final class LocalMigrationModel {
    private(set) var activeProjectID: String?
    private(set) var isWorking = false
    private(set) var uploaded = 0
    private(set) var total = 0
    private(set) var finishedCloudProjectID: String?
    private(set) var lastError: String?

    @ObservationIgnored private let library: LocalLibrary
    @ObservationIgnored private let api: APIClient
    @ObservationIgnored private let credentials: Credentials
    // 库记录由本模型直接写入；界面经 LocalLibraryModel 的快照读取，须在每次落盘后通知其刷新。
    @ObservationIgnored public var onRecordChanged: (() async -> Void)?
    @ObservationIgnored private var task: Task<Void, Never>?
    @ObservationIgnored private var active = true

    init(library: LocalLibrary, api: APIClient, credentials: Credentials) {
        self.library = library
        self.api = api
        self.credentials = credentials
    }

    private func recordChanged() async {
        await onRecordChanged?()
    }

    func migrate(projectID: String) {
        guard active, !isWorking else { return }
        isWorking = true
        activeProjectID = projectID
        lastError = nil
        finishedCloudProjectID = nil
        task = Task {
            defer { if active { isWorking = false }; task = nil }
            await run(projectID: projectID)
        }
    }

    func pause() { task?.cancel() }

    func abandon(projectID: String) {
        guard active, !isWorking else { return }
        task = Task {
            defer { task = nil }
            await abandonRun(projectID: projectID)
        }
    }

    private func project(_ id: String) async throws -> LocalProject {
        guard let value = try await library.projects().first(where: { $0.id == id }) else { throw LocalLibraryError.invalidProject }
        return value
    }

    private func run(projectID: String) async {
        do {
            if try await project(projectID).migration == nil {
                let workspaces = try await api.workspaces(token: credentials.token)
                guard let workspace = workspaces.first(where: \.permitsPromotion) else { throw MigrationError.noWorkspace }
                _ = try await library.beginMigration(projectID: projectID, workspaceID: workspace.id, privateSync: true)
                await recordChanged()
            }
            while active {
                let current = try await project(projectID)
                guard let record = current.migration else { throw LocalLibraryError.invalidProject }
                switch record.phase {
                case .enrolling:
                    let start = try await api.promote(manifestJSON: record.manifestJSON, token: credentials.token, privateSync: record.privateSync == true)
                    try await library.migrationEnrolled(projectID: projectID, cloudProjectID: start.project_id,
                                                        manifestDigest: start.manifest_digest)
                    await recordChanged()
                case .uploading:
                    guard let cloudProjectID = record.cloudProjectID, let digest = record.manifestDigest else {
                        throw LocalLibraryError.invalidProject
                    }
                    total = current.photos.count
                    uploaded = record.uploadedPhotoIDs.count
                    for photo in current.photos where !record.uploadedPhotoIDs.contains(photo.id) {
                        try Task.checkCancellation()
                        let payload = try await library.uploadPayload(projectID: projectID, photoID: photo.id)
                        _ = try await api.uploadPreview(projectID: cloudProjectID, photoID: photo.id,
                                                        jpeg: payload, token: credentials.token)
                        try await library.markUploaded(projectID: projectID, photoID: photo.id)
                        await recordChanged()
                        uploaded += 1
                    }
                    try Task.checkCancellation()
                    _ = try await api.finalizePromotion(projectID: cloudProjectID, manifestDigest: digest, token: credentials.token)
                    try await library.completeMigration(projectID: projectID)
                    await recordChanged()
                    finishedCloudProjectID = cloudProjectID
                    return
                case .migrated:
                    finishedCloudProjectID = record.cloudProjectID
                    return
                }
            }
        } catch is CancellationError {
            // 用户暂停：记录保留已上传进度，界面提供继续或放弃。
        } catch {
            if active { lastError = message(error) }
        }
    }

    private func abandonRun(projectID: String) async {
        do {
            var record = try await project(projectID).migration
            if record == nil { return }
            guard record?.phase != .migrated else { throw MigrationError.cannotAbandonCompleted }
            // enrolling 期间 promote 可能已落库；先幂等重发清单字节取回绑定，再决定能否取消。
            if record?.phase == .enrolling {
                let start = try await api.promote(manifestJSON: record!.manifestJSON, token: credentials.token, privateSync: record!.privateSync == true)
                try await library.migrationEnrolled(projectID: projectID, cloudProjectID: start.project_id,
                                                    manifestDigest: start.manifest_digest)
                await recordChanged()
                record = try await project(projectID).migration
            }
            guard let bound = record, let cloudProjectID = bound.cloudProjectID, let digest = bound.manifestDigest else {
                try await library.clearMigration(projectID: projectID)
                return
            }
            do { try await api.abortPromotion(projectID: cloudProjectID, manifestDigest: digest, token: credentials.token) }
            catch let error as APIError {
                if case .server(409, "AUTHORITY_MISMATCH") = error { throw MigrationError.cannotAbandonCompleted }
                throw error
            }
            try await library.clearMigration(projectID: projectID)
            await recordChanged()
        } catch {
            lastError = message(error)
        }
    }

    private func message(_ error: Error) -> String {
        if let value = error as? LocalizedError, let description = value.errorDescription { return description }
        return "迁云暂时无法继续，请检查网络后重试；已上传的部分不会重复。"
    }

    func shutdown() {
        active = false
        task?.cancel()
    }
}
