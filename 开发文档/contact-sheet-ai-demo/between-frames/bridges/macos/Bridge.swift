// 只读接收外部编辑结果；不访问私有 Catalog，不上传文件。
import AppKit
import CryptoKit
import ImageIO

struct BridgeFailure: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}
struct Pair: Codable {
    let sourcePath: String
    let renderPath: String
    let externalPhotoId: String
    let externalVariantId: String
}
struct BatchRequest: Codable {
    let provider: String
    let catalogId: String
    let destination: String
    let items: [Pair]
}
struct BatchResult: Codable {
    let directory: String?
    let count: Int
    let error: String?
}

func sha256(_ url: URL) throws -> String {
    let handle = try FileHandle(forReadingFrom: url)
    defer { try? handle.close() }
    var hasher = SHA256()
    while let data = try handle.read(upToCount: 1024 * 1024), !data.isEmpty {
        hasher.update(data: data)
    }
    return hasher.finalize().map { String(format: "%02x", $0) }.joined()
}
func digest(_ text: String) -> String {
    SHA256.hash(data: Data(text.utf8)).map { String(format: "%02x", $0) }.joined()
}
func stableUUID(_ text: String) -> String {
    var bytes = Array(SHA256.hash(data: Data(text.utf8)).prefix(16))
    bytes[6] = (bytes[6] & 15) | 80
    bytes[8] = (bytes[8] & 63) | 128
    return UUID(uuid: (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15])).uuidString.lowercased()
}

// 对真实渲染文件取尺寸、位深与 ICC；不采用文件名或调用者声明的像素信息。
func imageInfo(_ url: URL) throws -> [String: Any] {
    guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
          CGImageSourceGetCount(source) == 1,
          let type = CGImageSourceGetType(source) as String?,
          let format = ["public.jpeg": "JPEG", "public.png": "PNG", "public.tiff": "TIFF"][type],
          let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [String: Any],
          let width = properties[kCGImagePropertyPixelWidth as String] as? Int,
          let height = properties[kCGImagePropertyPixelHeight as String] as? Int,
          width > 0, height > 0, width <= 100000, height <= 100000,
          width * height <= 400_000_000 else {
        throw BridgeFailure(message: "请选择已导出的单页 JPEG、TIFF 或 PNG 照片。")
    }
    let depth = properties[kCGImagePropertyDepth as String] as? Int ?? 8
    guard [8, 10, 12, 14, 16, 32].contains(depth) else {
        throw BridgeFailure(message: "这张照片的位深暂不支持，请重新导出为 8 位或 16 位照片。")
    }
    return ["width": width, "height": height, "bitDepth": depth, "mediaFormat": format]
}

func regularFile(_ url: URL) throws {
    let values = try url.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
    guard values.isRegularFile == true, let size = values.fileSize, size > 0, size <= 4 * 1024 * 1024 * 1024 else {
        throw BridgeFailure(message: "照片不可读取、为空或大于 4 GB。")
    }
}

func createBatch(_ request: BatchRequest) throws -> BatchResult {
    guard ["LIGHTROOM_CLASSIC", "CAPTURE_ONE"].contains(request.provider),
          !request.catalogId.isEmpty, !request.items.isEmpty, request.items.count <= 500 else {
        throw BridgeFailure(message: "每次请选择 1–500 张照片。")
    }
    let fm = FileManager.default
    let parent = URL(fileURLWithPath: request.destination, isDirectory: true).standardizedFileURL
    var isDirectory: ObjCBool = false
    guard fm.fileExists(atPath: parent.path, isDirectory: &isDirectory), isDirectory.boolValue else {
        throw BridgeFailure(message: "请重新选择保存文件夹。")
    }
    let batchID = UUID().uuidString.lowercased()
    let target = parent.appendingPathComponent("FocalRoom-" + batchID, isDirectory: true)
    let staging = parent.appendingPathComponent(".focalroom-partial-" + batchID, isDirectory: true)
    try fm.createDirectory(at: staging, withIntermediateDirectories: false)
    do {
        var items = [[String: Any]]()
        let catalog = digest(request.provider + "\n" + request.catalogId)
        for pair in request.items {
            guard !pair.externalPhotoId.isEmpty, !pair.externalVariantId.isEmpty else {
                throw BridgeFailure(message: "缺少照片对应关系，请重新选择原照片。")
            }
            let original = URL(fileURLWithPath: pair.sourcePath).standardizedFileURL
            let render = URL(fileURLWithPath: pair.renderPath).standardizedFileURL
            try regularFile(original)
            try regularFile(render)
            guard try render.resourceValues(forKeys: [.fileSizeKey]).fileSize! <= 512 * 1024 * 1024 else {
                throw BridgeFailure(message: "单张导出照片不能超过 512 MB。")
            }
            let sourceSHA = try sha256(original)
            var item = try imageInfo(render)
            let renderSHA = try sha256(render)
            let format = item["mediaFormat"] as! String
            let suffix = ["JPEG": "jpg", "TIFF": "tif", "PNG": "png"][format]!
            let filename = UUID().uuidString.lowercased() + "." + suffix
            let copy = staging.appendingPathComponent(filename)
            try fm.copyItem(at: render, to: copy)
            guard try sha256(copy) == renderSHA, try sha256(original) == sourceSHA else {
                throw BridgeFailure(message: "照片在处理期间发生变化，请等导出完成后重试。")
            }
            item["sourceSha256"] = sourceSHA
            item["externalPhotoId"] = pair.externalPhotoId
            item["externalVariantId"] = pair.externalVariantId
            item["variantId"] = stableUUID(request.provider + catalog + pair.externalVariantId + sourceSHA)
            item["renderId"] = stableUUID(renderSHA)
            item["renderFilename"] = filename
            item["renderSha256"] = renderSHA
            items.append(item)
        }
        let manifest: [String: Any] = [
            "schemaVersion": 3, "transferBatchId": batchID, "provider": request.provider,
            "catalogFingerprint": catalog, "issuedAt": ISO8601DateFormatter().string(from: Date()), "items": items
        ]
        let data = try JSONSerialization.data(withJSONObject: manifest, options: [.sortedKeys, .prettyPrinted])
        try data.write(to: staging.appendingPathComponent(".focalroom-manifest.json"), options: .atomic)
        // 批次目录最后发布；未完成的复制不会被误当成可导入结果。
        try fm.moveItem(at: staging, to: target)
        return BatchResult(directory: target.path, count: items.count, error: nil)
    } catch {
        try? fm.removeItem(at: staging) // 仅清理本次生成的临时副本。
        throw error
    }
}

final class BridgeApp: NSObject, NSApplicationDelegate, NSTableViewDataSource, NSTableViewDelegate {
    private var window: NSWindow!
    private let table = NSTableView()
    private let message = NSTextField(wrappingLabelWithString: "从 Capture One 导出 JPEG、TIFF 或 PNG 后，在这里选择对应的原照片。原照片只用于确认对应关系，不会被修改。")
    private let saveButton = NSButton(title: "保存并带回 FocalRoom…", target: nil, action: nil)
    private var renders = [URL]()
    private var originals = [String: URL]()

    func applicationDidFinishLaunching(_ notification: Notification) {
        let menu = NSMenu()
        let appItem = NSMenuItem()
        menu.addItem(appItem)
        let submenu = NSMenu()
        submenu.addItem(withTitle: "退出 FocalRoom Bridge", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        appItem.submenu = submenu
        NSApp.mainMenu = menu
        let view = NSStackView()
        view.orientation = .vertical; view.alignment = .leading; view.spacing = 16
        view.edgeInsets = NSEdgeInsets(top: 24, left: 24, bottom: 24, right: 24)
        let title = NSTextField(labelWithString: "把调色结果带回 FocalRoom")
        title.font = .systemFont(ofSize: 23, weight: .semibold)
        view.addArrangedSubview(title); view.addArrangedSubview(message)
        for (id, name, width) in [("render", "导出的照片", 300.0), ("source", "对应原照片", 300.0)] {
            let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(id))
            column.title = name; column.width = width; table.addTableColumn(column)
        }
        table.delegate = self; table.dataSource = self; table.rowHeight = 32
        table.target = self; table.doubleAction = #selector(chooseOriginal)
        let scroll = NSScrollView(); scroll.documentView = table; scroll.hasVerticalScroller = true
        scroll.translatesAutoresizingMaskIntoConstraints = false
        scroll.heightAnchor.constraint(equalToConstant: 260).isActive = true
        view.addArrangedSubview(scroll)
        scroll.widthAnchor.constraint(equalTo: view.widthAnchor, constant: -48).isActive = true
        message.widthAnchor.constraint(equalTo: view.widthAnchor, constant: -48).isActive = true
        let buttons = NSStackView(); buttons.spacing = 12
        let add = NSButton(title: "添加导出的照片…", target: self, action: #selector(addRenders))
        let pair = NSButton(title: "选择对应原照片…", target: self, action: #selector(chooseOriginal))
        saveButton.target = self; saveButton.action = #selector(save); saveButton.bezelStyle = .rounded
        buttons.addArrangedSubview(add); buttons.addArrangedSubview(pair); buttons.addArrangedSubview(saveButton)
        view.addArrangedSubview(buttons)
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 800, height: 440), styleMask: [.titled, .closable, .miniaturizable], backing: .buffered, defer: false)
        window.title = "FocalRoom · Capture One Bridge"; window.contentView = view
        view.widthAnchor.constraint(equalToConstant: 800).isActive = true
        window.center(); window.makeKeyAndOrderFront(nil); NSApp.activate(ignoringOtherApps: true)
        refresh()
    }
    func application(_ sender: NSApplication, openFiles filenames: [String]) {
        append(filenames.map { URL(fileURLWithPath: $0) }); sender.reply(toOpenOrPrint: .success)
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
    func numberOfRows(in tableView: NSTableView) -> Int { renders.count }
    func tableView(_ tableView: NSTableView, objectValueFor tableColumn: NSTableColumn?, row: Int) -> Any? {
        let file = renders[row]
        return tableColumn?.identifier.rawValue == "render" ? file.lastPathComponent : originals[file.path]?.lastPathComponent ?? "请选择原照片"
    }
    private func refresh() {
        table.reloadData()
        saveButton.isEnabled = !renders.isEmpty && renders.allSatisfy { originals[$0.path] != nil }
        if !renders.isEmpty && table.selectedRow < 0 { table.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false) }
    }
    private func alert(_ text: String) {
        let alert = NSAlert(); alert.messageText = text; alert.runModal()
    }
    private func append(_ urls: [URL]) {
        for url in urls.prefix(500) {
            do {
                try regularFile(url); _ = try imageInfo(url)
                if !renders.contains(url) { renders.append(url) }
            } catch { alert(error.localizedDescription + "\n" + url.lastPathComponent) }
        }
        if renders.count > 500 { renders = Array(renders.prefix(500)); alert("每次最多接收 500 张照片。") }
        refresh()
    }
    @objc private func addRenders() {
        let panel = NSOpenPanel(); panel.allowsMultipleSelection = true
        panel.allowedFileTypes = ["jpg", "jpeg", "png", "tif", "tiff"]
        panel.message = "选择 Capture One 导出的照片，不要选择 RAW 原片。"
        if panel.runModal() == .OK { append(panel.urls) }
    }
    @objc private func chooseOriginal() {
        let row = table.selectedRow
        guard row >= 0 && row < renders.count else { return }
        let render = renders[row]
        let panel = NSOpenPanel(); panel.message = "选择「\(render.lastPathComponent)」对应的原照片（与 FocalRoom 中导入的原照片一致）。"
        if panel.runModal() == .OK, let url = panel.url {
            originals[render.path] = url
            refresh()
            if row + 1 < renders.count { table.selectRowIndexes(IndexSet(integer: row + 1), byExtendingSelection: false) }
        }
    }
    @objc private func save() {
        let panel = NSOpenPanel(); panel.canChooseDirectories = true; panel.canChooseFiles = false; panel.canCreateDirectories = true
        panel.message = "选择保存位置；每次都会新建文件夹，不覆盖已有照片。"
        guard panel.runModal() == .OK, let destination = panel.url else { return }
        saveButton.isEnabled = false
        let pairs = renders.map { render in Pair(sourcePath: originals[render.path]!.path, renderPath: render.path, externalPhotoId: digest(originals[render.path]!.path), externalVariantId: digest(originals[render.path]!.path)) }
        let request = BatchRequest(provider: "CAPTURE_ONE", catalogId: "focalroom-manual-capture-one-v1", destination: destination.path, items: pairs)
        message.stringValue = "正在准备照片，请稍候…"
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let result = try createBatch(request)
                DispatchQueue.main.async {
                    self.message.stringValue = "已保存 \(result.count) 张。打开 FocalRoom 项目，选择「调色版本」→「导入调色结果」，再选择刚生成的文件夹。旧版客户端也可用「导入照片」作为独立照片导入。"
                    NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: result.directory!)])
                    self.refresh()
                }
            } catch {
                DispatchQueue.main.async { self.alert(error.localizedDescription); self.refresh() }
            }
        }
    }
}

if CommandLine.arguments.count == 4, CommandLine.arguments[1] == "--request" {
    let input = URL(fileURLWithPath: CommandLine.arguments[2])
    let output = URL(fileURLWithPath: CommandLine.arguments[3])
    let result: BatchResult
    do {
        let data = try Data(contentsOf: input)
        guard data.count < 4 * 1024 * 1024 else { throw BridgeFailure(message: "本次照片过多，请分批导出。") }
        result = try createBatch(JSONDecoder().decode(BatchRequest.self, from: data))
    } catch { result = BatchResult(directory: nil, count: 0, error: error.localizedDescription) }
    do { try JSONEncoder().encode(result).write(to: output, options: .atomic) }
    catch { fputs("无法保存导出结果。\n", stderr); exit(2) }
    exit(result.error == nil ? 0 : 1)
} else {
    let app = NSApplication.shared
    let delegate = BridgeApp()
    app.delegate = delegate; app.setActivationPolicy(.regular)
    withExtendedLifetime(delegate) { app.run() }
}
