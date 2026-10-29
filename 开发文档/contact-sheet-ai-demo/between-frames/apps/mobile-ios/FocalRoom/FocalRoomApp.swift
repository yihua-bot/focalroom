import SwiftUI

enum AppLinks {
    static let privacyPolicy = URL(string: "https://focroom.cc/privacy/")!
}

@main
struct FocalRoomApp: App {
    @State private var model = AppModel()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView(model: model)
                .preferredColorScheme(.dark)
                .tint(.white)
                .task { await model.restore() }
                .onChange(of: scenePhase) { _, phase in
                    if phase == .active { Task { await model.refreshOnResume() } }
                }
        }
    }
}
