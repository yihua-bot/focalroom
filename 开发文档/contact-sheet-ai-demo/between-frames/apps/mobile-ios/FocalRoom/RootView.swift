import SwiftUI
import FocalRoomCore

struct RootView: View {
    @Bindable var model: AppModel
    @State private var accountPresented = false

    var body: some View {
        Group {
            if model.starting {
                ProgressView("打开工作空间…").frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if model.credentials == nil {
                LoginView(model: model)
            } else {
                TabView {
                    workspace { ProjectsView(model: model) }
                        .tabItem { Label("项目", systemImage: "square.stack.3d.up") }
                    workspace { FocalAIView(model: model) }
                        .tabItem { Label("Focal AI", systemImage: "sparkle") }
                    workspace { BoardsView(model: model) }
                        .tabItem { Label("画布", systemImage: "rectangle.3.group") }
                    workspace { CommentsView(model: model) }
                        .tabItem { Label("消息", systemImage: "bubble.left.and.bubble.right") }
                }
                .sheet(isPresented: $accountPresented) { AccountView(model: model) }
            }
        }
        .alert("提示", isPresented: Binding(get: { model.notice != nil }, set: { if !$0 { model.notice = nil } })) {
            Button("知道了", role: .cancel) { model.notice = nil }
        } message: { Text(model.notice ?? "") }
    }

    private func workspace<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        NavigationStack {
            content().toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button { accountPresented = true } label: { Image(systemName: "person.crop.circle") }
                        .accessibilityLabel("账户")
                }
            }
        }
    }
}

private struct LoginView: View {
    @Bindable var model: AppModel
    @State private var email = ""
    @State private var password = ""
    @State private var code = ""
    @State private var mode: Mode = .password
    @State private var offlineLibrary = false
    @State private var offlineScopes: [LocalScopeEntry] = []
    @FocusState private var focus: Field?
    private enum Field { case email, password, code }
    private enum Mode: String, CaseIterable { case password = "密码登录"; case code = "验证码登录" }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 28) {
                VStack(alignment: .leading, spacing: 12) {
                    Image("BrandMark").resizable().scaledToFit().frame(width: 48, height: 48)
                    Text("FocalRoom").font(.system(size: 38, weight: .medium, design: .serif))
                    Text("让照片，成为作品。")
                        .font(.title3).foregroundStyle(.secondary)
                }
                .padding(.top, 70).padding(.bottom, 20)

                VStack(alignment: .leading, spacing: 18) {
                    Text("登录你的工作空间").font(.headline)
                    Picker("登录方式", selection: $mode) {
                        ForEach(Mode.allCases, id: \.self) { Text($0.rawValue).tag($0) }
                    }
                    .pickerStyle(.segmented)
                    .accessibilityIdentifier("login.mode")
                    TextField("邮箱", text: $email)
                        .textContentType(.emailAddress).keyboardType(.emailAddress)
                        .textInputAutocapitalization(.never).autocorrectionDisabled()
                        .focused($focus, equals: .email).submitLabel(.next)
                        .onSubmit { focus = mode == .password ? .password : nil }
                        .accessibilityIdentifier("login.email")
                    Divider()
                    if mode == .password {
                        SecureField("密码", text: $password)
                            .textContentType(.password).focused($focus, equals: .password).submitLabel(.go)
                            .onSubmit { submit() }
                            .accessibilityIdentifier("login.password")
                    } else {
                        HStack(spacing: 12) {
                            Button {
                                focus = nil
                                let entered = email
                                Task { await model.requestLoginCode(email: entered) }
                            } label: {
                                if model.busy { ProgressView().tint(.black) } else { Text("获取验证码").font(.subheadline).fontWeight(.medium) }
                            }
                            .foregroundStyle(.black).padding(.horizontal, 14).frame(minHeight: 44)
                            .background(.white.opacity(0.9), in: RoundedRectangle(cornerRadius: 10))
                            .disabled(!email.contains("@") || model.busy)
                            .opacity(email.contains("@") ? 1 : 0.4)
                            .accessibilityIdentifier("login.code.request")
                            TextField("6 位验证码", text: $code)
                                .keyboardType(.numberPad)
                                .textInputAutocapitalization(.never).autocorrectionDisabled()
                                .focused($focus, equals: .code).submitLabel(.go)
                                .onSubmit { submit() }
                                .accessibilityIdentifier("login.code")
                        }
                        if model.codeSent {
                            Text("验证码已发送到邮箱（10 分钟内有效）；新邮箱将自动创建账号。")
                                .font(.caption).foregroundStyle(.secondary)
                                .accessibilityIdentifier("login.code.sent")
                        }
                    }
                }
                .padding(22).background(.white.opacity(0.05), in: RoundedRectangle(cornerRadius: 16))

                DisclosureGroup("工作空间设置") {
                    VStack(alignment: .leading, spacing: 8) {
                        TextField("https://你的服务地址", text: $model.address)
                            .textContentType(.URL).keyboardType(.URL)
                            .textInputAutocapitalization(.never).autocorrectionDisabled()
                            .accessibilityIdentifier("login.server")
                        Text("使用独立工作空间时，可在这里更改服务地址。登录凭据保存在设备钥匙串中。")
                            .font(.caption).foregroundStyle(.secondary)
                    }.padding(.top, 12)
                }
                .font(.subheadline).foregroundStyle(.secondary)

                Button(action: submit) {
                    HStack {
                        Spacer()
                        if model.busy { ProgressView().tint(.black) } else { Text(mode == .password ? "进入 FocalRoom" : "验证并进入").fontWeight(.semibold) }
                        Spacer()
                    }.frame(minHeight: 52)
                }
                .foregroundStyle(.black).background(.white, in: RoundedRectangle(cornerRadius: 14))
                .disabled(!valid || model.busy).opacity(valid ? 1 : 0.4)
                .accessibilityIdentifier("login.submit")

                if !offlineScopes.isEmpty {
                    Button {
                        offlineLibrary = true
                    } label: {
                        Label("查看本机资料（离线只读）", systemImage: "internaldrive")
                            .frame(maxWidth: .infinity, minHeight: 44)
                    }
                    .accessibilityIdentifier("offline.library")
                    Text("不登录也可浏览与导出这台 iPhone 上已保存的本机原片。")
                        .font(.caption).foregroundStyle(.secondary)
                }

                Link("隐私政策", destination: AppLinks.privacyPolicy)
                    .font(.footnote).foregroundStyle(.secondary)
                    .accessibilityIdentifier("login.privacy")

                #if DEBUG
                Text("iPhone 开发预览").font(.caption2).foregroundStyle(.tertiary)
                #else
                Text("版本 \(Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0.0")")
                    .font(.caption2).foregroundStyle(.tertiary)
                #endif
            }
            .padding(.horizontal, 28).padding(.bottom, 32)
        }
        .scrollDismissesKeyboard(.interactively)
        .background(Color(white: 0.055))
        .onAppear { offlineScopes = LocalScopeIndex.list(root: model.root) }
        .fullScreenCover(isPresented: $offlineLibrary) {
            OfflineLibraryView(root: model.root, model: model)
        }
    }

    private var valid: Bool {
        guard email.contains("@"), !model.address.isEmpty else { return false }
        return mode == .password ? password.count >= 8 : code.count == 6 && code.allSatisfy(\.isNumber)
    }
    private func submit() {
        guard valid, !model.busy else { return }
        focus = nil
        let enteredPassword = password, enteredCode = code
        if mode == .password {
            Task {
                await model.login(email: email, password: enteredPassword)
                password = ""
            }
        } else {
            Task {
                await model.magicLogin(email: email, code: enteredCode)
                code = ""
            }
        }
    }
}

struct EmptyWorkspace: View {
    let title: String
    let detail: String
    var symbol = "square.stack.3d.up"

    var body: some View {
        ContentUnavailableView(title, systemImage: symbol, description: Text(detail))
    }
}

struct ProjectContext: View {
    let model: AppModel
    @State private var choosingProject = false

    var body: some View {
        VStack(spacing: 0) {
        Button { choosingProject = true } label: {
            HStack(spacing: 8) {
                Image(systemName: "folder")
                Text(model.currentProjectTitle ?? "选择项目").lineLimit(1)
                Image(systemName: "chevron.down").font(.caption)
                Spacer()
                if model.loadingProject { ProgressView() }
                if model.selectedLocalProjectID != nil { Label("本机", systemImage: "iphone").font(.caption) }
                else if model.offline { Label("离线缓存", systemImage: "wifi.slash").font(.caption) }
            }
            .frame(maxWidth: .infinity, minHeight: 24, alignment: .leading)
            .contentShape(Rectangle())
            .padding(.horizontal).padding(.vertical, 12)
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("project.context")
        .accessibilityLabel("选择项目")
        .accessibilityValue(model.currentProjectTitle ?? "尚未选择")
        .disabled(model.busy)
        .sheet(isPresented: $choosingProject) { ProjectChooser(model: model) }
        if let project = model.selectedLocalProject, let library = model.localLibrary {
            HStack {
                Text("\(project.photos.count) 张本机照片").foregroundStyle(.secondary)
                Spacer()
                NavigationLink("打开本机图库") {
                    LocalProjectView(library: library, migration: model.migration, model: model, projectID: project.id)
                }.accessibilityIdentifier("project.context.local.library")
            }.font(.caption).padding(.horizontal).padding(.bottom, 10)
        }
        }
    }
}

private struct ProjectChooser: View {
    let model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var query = ""
    @State private var refreshing = false
    @State private var opening: String?

    private var cloudProjects: [ProjectSummary] {
        model.projects.filter { query.isEmpty || $0.title.localizedCaseInsensitiveContains(query) }
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    if refreshing { Label { Text("正在刷新项目…") } icon: { ProgressView() } }
                    if model.offline {
                        Label("离线时仅显示已缓存的项目，画布与分析任务需联网。", systemImage: "wifi.slash")
                            .font(.footnote).foregroundStyle(.secondary)
                    }
                    ForEach(cloudProjects) { project in
                        Button {
                            opening = project.id
                            Task {
                                await model.openProject(project.id)
                                opening = nil
                                if model.snapshot?.project_id == project.id { dismiss() }
                            }
                        } label: {
                            HStack {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(project.title).foregroundStyle(.primary)
                                    Text(project.state != .SHARED ? "正在准备，暂不可打开" : project.sync_mode == .PRIVATE_SYNCED ? "私有同步" : "已同步")
                                        .font(.caption).foregroundStyle(.secondary)
                                }
                                Spacer()
                                if opening == project.id { ProgressView() }
                                else if model.selectedProjectID == project.id { Image(systemName: "checkmark") }
                            }.frame(minHeight: 44).contentShape(Rectangle())
                        }
                        .disabled(project.state != .SHARED || opening != nil || model.busy)
                        .accessibilityIdentifier("project.choose.\(project.id)")
                    }
                    if cloudProjects.isEmpty {
                        Text(query.isEmpty ? "还没有已同步项目。可直接选择本机项目，或在项目页新建。" : "未找到匹配的已同步项目")
                            .foregroundStyle(.secondary)
                            .accessibilityIdentifier("project.choose.empty")
                    }
                } header: { Text("已同步项目") }
                if let library = model.localLibrary {
                    let localProjects = library.projects.filter { query.isEmpty || $0.title.localizedCaseInsensitiveContains(query) }
                    if !localProjects.isEmpty {
                        Section {
                            ForEach(localProjects) { project in
                                Button {
                                    model.openLocalContext(project.id)
                                    if model.selectedLocalProjectID == project.id { dismiss() }
                                } label: {
                                    HStack {
                                        Label(project.title, systemImage: "iphone").foregroundStyle(.primary)
                                        Spacer()
                                        if model.selectedLocalProjectID == project.id { Image(systemName: "checkmark") }
                                    }.frame(minHeight: 44).contentShape(Rectangle())
                                }
                                .disabled(opening != nil || model.busy)
                                .accessibilityIdentifier("project.choose.local.\(project.id)")
                            }
                        } header: { Text("本机项目") } footer: {
                            Text("直接选中本机项目，不会迁云。AI 照片观察每次单独确认后仅发送所选预览，原片保留本机。")
                        }
                    }
                }
            }
            .navigationTitle("选择项目").navigationBarTitleDisplayMode(.inline)
            .searchable(text: $query, prompt: "搜索项目")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("完成") { dismiss() }.accessibilityIdentifier("project.choose.done")
                        .disabled(opening != nil)
                }
            }
            .task { await refresh() }
            .refreshable { await refresh() }
            .interactiveDismissDisabled(opening != nil)
        }
    }

    private func refresh() async {
        guard !refreshing else { return }
        refreshing = true
        defer { refreshing = false }
        await model.reloadProjects()
        await model.localLibrary?.reload()
    }
}

private struct AccountView: View {
    let model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var confirmLogout = false
    @State private var revokeTarget: DeviceRecord?
    @State private var inviteSheet = false

    var body: some View {
        NavigationStack {
            List {
                Section("账户") {
                    Text(model.credentials?.user.name ?? "")
                    Text(model.credentials?.user.email ?? "").foregroundStyle(.secondary)
                    NavigationLink("隐私与数据") { AccountDataView(model: model) }
                        .accessibilityIdentifier("account.data.entry")
                }
                Section("工作空间") {
                    Text(model.address).font(.footnote).textSelection(.enabled)
                    Label(model.offline ? "离线缓存，权限待联网核实" : "在线", systemImage: model.offline ? "wifi.slash" : "checkmark.circle")
                    Button("邀请成员加入工作区") { inviteSheet = true }
                        .disabled(model.offline || model.busy)
                        .accessibilityIdentifier("workspace.invite.entry")
                    Text("已查看的云端预览有受限离线缓存，退出时清理。本机项目保留原片、预览和选片记录，并按账户隔离。")
                        .font(.footnote).foregroundStyle(.secondary)
                }
                Section {
                    if model.devices.isEmpty && model.loadingDevices {
                        HStack { ProgressView(); Text("读取设备…").foregroundStyle(.secondary) }
                    } else if model.devices.isEmpty {
                        Text("尚未登记设备").foregroundStyle(.secondary)
                    } else {
                        ForEach(model.devices) { device in
                            HStack {
                                VStack(alignment: .leading, spacing: 4) {
                                    Text(device.name)
                                    Text(device.platform == "ios" ? "iPhone · \(device.current ? "当前设备" : "其他登录")" : "\(device.platform) · \(device.current ? "当前设备" : "其他登录")")
                                        .font(.caption).foregroundStyle(.secondary)
                                }
                                Spacer()
                                Button("撤销", role: .destructive) { revokeTarget = device }
                                    .buttonStyle(.borderless)
                                    .accessibilityIdentifier("device.revoke.\(device.id)")
                            }
                        }
                    }
                } header: {
                    Text("已登录设备")
                } footer: {
                    Text("撤销设备会立即使其登录会话失效；撤销当前设备等同于在此设备退出。")
                        .font(.footnote)
                }
                Section {
                    if let catalog = model.billingCatalogState {
                        Text(catalog.salesAvailable ? "订阅服务已开放。" : "订阅暂未开放；以下为当前服务能力说明。")
                            .font(.footnote).foregroundStyle(.secondary)
                        ForEach(catalog.plans) { plan in
                            VStack(alignment: .leading, spacing: 4) {
                                Text(plan.name).font(.subheadline.weight(.semibold))
                                Text(plan.features.joined(separator: " · "))
                                    .font(.caption2).foregroundStyle(.secondary)
                            }
                            .accessibilityIdentifier("billing.plan.\(plan.id)")
                        }
                    } else {
                        HStack(spacing: 8) {
                            ProgressView()
                            Text("读取账单目录…").foregroundStyle(.secondary)
                        }
                    }
                } header: {
                    Text("账单目录")
                }
                .accessibilityIdentifier("billing.section")
                Section {
                    Button("退出登录", role: .destructive) { confirmLogout = true }
                        .disabled(model.busy)
                        .accessibilityIdentifier("account.logout")
                }
            }
            .navigationTitle("账户")
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("完成") { dismiss() } } }
            .task {
                await model.loadDevices()
                await model.loadBillingCatalog()
            }
            .confirmationDialog("撤销设备", isPresented: Binding(get: { revokeTarget != nil }, set: { if !$0 { revokeTarget = nil } }), titleVisibility: .visible) {
                Button("撤销\(revokeTarget?.name ?? "")", role: .destructive) {
                    if let target = revokeTarget { Task { await model.revokeDevice(target.id) } }
                    revokeTarget = nil
                }
            } message: {
                Text("该设备上的登录会话将立即失效。")
            }
            .confirmationDialog("退出会清理云端缓存和待确认记录。本机原片与项目保留，重新登录同一账户后可访问。已发送的操作仍可能在服务端生效。", isPresented: $confirmLogout, titleVisibility: .visible) {
                Button("退出并清理本机缓存", role: .destructive) {
                    dismiss()
                    Task { await model.logout() }
                }
            }
            .sheet(isPresented: $inviteSheet) { InviteSheet(model: model) }
        }
    }
}
