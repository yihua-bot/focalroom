import XCTest

// 采集 App Store 商店图：以英文界面走离线本机资料路径，把 PNG 写入测试运行器容器 Documents。
final class StoreScreenshots: XCTestCase {
    private var index = 0

    @MainActor
    func testCapture() throws {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchArguments += ["-AppleLanguages", "(en)", "-AppleLocale", "en_US"]
        app.launch()

        XCTAssertTrue(app.textFields["login.email"].waitForExistence(timeout: 20))
        shot(app, "login")

        let offline = app.buttons["offline.library"]
        XCTAssertTrue(offline.waitForExistence(timeout: 5), "没有离线入口")
        offline.tap()
        let scope = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "offline.scope.")).firstMatch
        XCTAssertTrue(scope.waitForExistence(timeout: 10), "离线账户列表为空")
        shot(app, "scopes")
        scope.tap()

        let project = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "local.project.")).firstMatch
        XCTAssertTrue(project.waitForExistence(timeout: 10), "项目列表为空")
        shot(app, "projects")
        project.tap()

        let photo = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "local.photo.")).firstMatch
        XCTAssertTrue(photo.waitForExistence(timeout: 10), "照片网格为空")
        shot(app, "grid")

        let keep = app.buttons["Keep"]
        if keep.waitForExistence(timeout: 3) {
            keep.tap()
            usleep(900_000)
            shot(app, "grid-kept")
            app.buttons["All"].tap()
            usleep(700_000)
        }
        app.swipeUp()
        usleep(700_000)
        shot(app, "grid-more")

        let first = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH %@", "local.photo.")).firstMatch
        XCTAssertTrue(first.waitForExistence(timeout: 5))
        first.tap()
        usleep(1_500_000)
        shot(app, "viewer")
    }

    @MainActor
    private func shot(_ app: XCUIApplication, _ name: String) {
        index += 1
        let data = app.screenshot().pngRepresentation
        let url = URL(fileURLWithPath: NSHomeDirectory())
            .appendingPathComponent("Documents/shot-\(String(format: "%02d", index))-\(name).png")
        do { try data.write(to: url) } catch { XCTFail("写入失败 \(url.path): \(error)") }
        NSLog("FRSHOT \(url.path)")
    }
}
