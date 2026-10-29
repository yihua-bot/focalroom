import SwiftUI
import UniformTypeIdentifiers

private struct AccountExportDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.json] }
    var data: Data
    init(data: Data) { self.data = data }
    init(configuration: ReadConfiguration) throws { data = configuration.file.regularFileContents ?? Data() }
    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper { FileWrapper(regularFileWithContents: data) }
}

struct AccountDataView: View {
    let model: AppModel
    @State private var exporting = false
    @State private var bytes = Data()
    @State private var busy = false
    @State private var password = ""
    @State private var email = ""
    @State private var understood = false
    @State private var confirming = false
    @State private var notice: String?

    var body: some View {
        Form {
            Section("隐私政策") {
                Link("查看完整隐私政策", destination: AppLinks.privacyPolicy)
                    .accessibilityIdentifier("account.privacy.policy")
            }
            Section("账号安全") {
                NavigationLink("设置密码") { SetPasswordView(model: model) }
                    .accessibilityIdentifier("account.password.entry")
            }
            Section("取回账号资料") {
                Text("导出个人资料、设备、所属工作区、拥有的项目、本人评论/研究/提案和计量记录，不含密码或会话凭据。原片请另用本机项目导出取回。")
                    .font(.footnote).foregroundStyle(.secondary)
                Button("导出账号资料") {
                    Task {
                        busy = true
                        defer { busy = false }
                        do { bytes = try await model.exportAccountData(); exporting = true }
                        catch { notice = error.localizedDescription }
                    }
                }.disabled(busy || model.offline).accessibilityIdentifier("account.data.export")
            }
            Section("删除账号") {
                Text("删除后立即撤销所有登录、移除账号资料，并清除未与其他人共享的云端项目内容。共享协作作品保留匿名历史，财务、计量与访问审计保留；对象介质清理由服务端重试处理，不等同于即时擦除所有备份。")
                    .font(.footnote).foregroundStyle(.secondary)
                Text("本机原片不会删除，仍可离线导出。请先取回资料；有进行中任务或有效订阅时须先处理。")
                    .font(.footnote).foregroundStyle(.secondary)
                TextField("输入当前账号邮箱", text: $email).textInputAutocapitalization(.never).autocorrectionDisabled()
                    .keyboardType(.emailAddress).accessibilityIdentifier("account.erase.email")
                SecureField("再次输入当前密码", text: $password).accessibilityIdentifier("account.erase.password")
                Toggle("我已了解删除与保留范围", isOn: $understood).accessibilityIdentifier("account.erase.understand")
                Button("删除账号", role: .destructive) { confirming = true }
                    .disabled(busy || model.offline || !understood || password.isEmpty || email.lowercased() != model.credentials?.user.email)
                    .accessibilityIdentifier("account.erase")
            }
            if let notice { Text(notice).font(.footnote).foregroundStyle(.orange) }
        }
        .navigationTitle("隐私与数据").navigationBarTitleDisplayMode(.inline)
        .fileExporter(isPresented: $exporting, document: AccountExportDocument(data: bytes), contentType: .json,
                      defaultFilename: "FocalRoom-account-data") { result in
            bytes = Data()
            switch result {
            case .success: notice = "账号资料已导出。"
            case .failure(let error):
                if (error as NSError).code != NSUserCancelledError { notice = "导出未完成，请重新选择保存位置。" }
            }
        }
        .alert("永久删除此账号？", isPresented: $confirming) {
            Button("取消", role: .cancel) {}
            Button("确认删除账号", role: .destructive) {
                let secret = password; password = ""
                Task {
                    busy = true
                    defer { busy = false }
                    do { try await model.eraseAccount(password: secret, email: email) }
                    catch { notice = "未获得删除完成回执，请先核对状态，勿反复提交。\n" + error.localizedDescription }
                }
            }
        } message: { Text("账号将无法恢复。本机原片保留，共享协作与财务审计按上述说明保留。") }
    }
}

/// 登录态设置或修改密码；验证码登录的新账号可不填当前密码。
struct SetPasswordView: View {
    let model: AppModel
    @State private var current = ""
    @State private var next = ""
    @State private var confirm = ""
    @State private var saved = false

    private var valid: Bool {
        next.count >= 8 && next == confirm && !model.busy
    }

    var body: some View {
        Form {
            Section {
                SecureField("当前密码（验证码登录的新账号可不填）", text: $current)
                    .textContentType(.password)
                    .accessibilityIdentifier("password.current")
                SecureField("新密码（至少 8 位）", text: $next)
                    .textContentType(.newPassword)
                    .accessibilityIdentifier("password.new")
                SecureField("再次输入新密码", text: $confirm)
                    .textContentType(.newPassword)
                    .accessibilityIdentifier("password.confirm")
            } footer: {
                Text("设置密码后，可在登录页使用「密码登录」；验证码登录继续可用。")
            }
            Section {
                Button {
                    let entered = next, enteredCurrent = current.isEmpty ? nil : current
                    current = ""; next = ""; confirm = ""
                    Task {
                        await model.setPassword(entered, current: enteredCurrent)
                        saved = true
                    }
                } label: {
                    if model.busy { ProgressView() } else { Text("保存新密码") }
                }
                .disabled(!valid)
                .accessibilityIdentifier("password.save")
                if saved, model.notice?.contains("已更新") == true {
                    Label("已保存", systemImage: "checkmark.circle.fill")
                        .foregroundStyle(.green)
                        .accessibilityIdentifier("password.saved")
                }
            }
        }
        .navigationTitle("设置密码")
    }
}
