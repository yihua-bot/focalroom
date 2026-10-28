import SwiftUI
import UniformTypeIdentifiers
import FocalRoomCore

// 未登录时的本机资料只读入口：按登记过的账户资料目录浏览，可导出，不可导入/选片/迁云。
struct OfflineLibraryView: View {
    let root: URL
    let model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var scopes: [LocalScopeEntry] = []
    @State private var library: LocalLibraryModel?

    var body: some View {
        NavigationStack {
            Group {
                if let library {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 22) {
                            if library.projects.isEmpty {
                                EmptyWorkspace(title: "此账户没有本机项目", detail: "登录后导入的照片会保留在本机目录。", symbol: "internaldrive")
                            } else {
                                LocalProjectsSection(library: library, model: model, query: "")
                            }
                        }.padding(20)
                    }
                } else if scopes.isEmpty {
                    EmptyWorkspace(title: "没有可离线访问的资料", detail: "登录过至少一次后，本机资料才可离线查看。", symbol: "internaldrive")
                } else {
                    List {
                        ForEach(scopes) { scope in
                            Button {
                                open(scope)
                            } label: {
                                VStack(alignment: .leading, spacing: 6) {
                                    Text(scope.email)
                                    Text(scope.server).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                                }
                            }
                            .accessibilityIdentifier("offline.scope.\(scope.id)")
                        }
                    }
                }
            }
            .navigationTitle(library == nil ? "本机资料" : "离线本机项目")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("完成") { dismiss() }
                }
                if library != nil {
                    ToolbarItem(placement: .primaryAction) {
                        Button("切换账户") { library = nil }
                    }
                }
            }
            .onAppear { if scopes.isEmpty { scopes = LocalScopeIndex.list(root: root) } }
        }
    }

    private func open(_ scope: LocalScopeEntry) {
        let actor = LocalLibrary(root: root.appendingPathComponent("LocalLibrary"),
                                  scope: AccountScope(server: scope.server, userID: scope.userID))
        let value = LocalLibraryModel(library: actor, readOnly: true)
        library = value
        Task { await value.reload() }
    }
}

struct FolderPicker: UIViewControllerRepresentable {
    let onPick: (URL) -> Void

    func makeUIViewController(context: Context) -> UIDocumentPickerViewController {
        let picker = UIDocumentPickerViewController(forOpeningContentTypes: [.folder], asCopy: false)
        picker.allowsMultipleSelection = false
        picker.delegate = context.coordinator
        return picker
    }

    func updateUIViewController(_ uiViewController: UIDocumentPickerViewController, context: Context) {}

    func makeCoordinator() -> Coordinator { Coordinator(onPick: onPick) }

    final class Coordinator: NSObject, UIDocumentPickerDelegate {
        let onPick: (URL) -> Void

        init(onPick: @escaping (URL) -> Void) {
            self.onPick = onPick
        }

        func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
            if let url = urls.first { onPick(url) }
        }
    }
}
