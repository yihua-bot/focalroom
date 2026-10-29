import XCTest

final class FocalRoomUITests: XCTestCase {
    @MainActor
    func testProjectSelectionInAITabCanvasAndMessages() throws {
        continueAfterFailure = false
        let environment = ProcessInfo.processInfo.environment
        guard let base = environment["FOCALROOM_IOS_TEST_BASE_URL"],
              let firstID = environment["FOCALROOM_IOS_PICKER_FIRST_ID"],
              let secondID = environment["FOCALROOM_IOS_PICKER_SECOND_ID"],
              let localFirstID = environment["FOCALROOM_IOS_PICKER_LOCAL_FIRST_ID"],
              let localSecondID = environment["FOCALROOM_IOS_PICKER_LOCAL_SECOND_ID"],
              let localPhotoID = environment["FOCALROOM_IOS_PICKER_LOCAL_PHOTO_ID"] else {
            throw XCTSkip("项目切换验证仅使用独立的本地后端夹具")
        }
        let app = XCUIApplication()
        app.launchEnvironment["FOCALROOM_IOS_REVIEW_URL"] = base
        app.launch()
        XCTAssertTrue(app.textFields["login.email"].waitForExistence(timeout: 10))
        app.textFields["login.email"].tap()
        app.textFields["login.email"].typeText("picker@fr.test")
        app.secureTextFields["login.password"].tap()
        app.secureTextFields["login.password"].typeText("iphone-local-test-2026")
        app.buttons["login.submit"].tap()
        dismissPasswordPrompt(app)
        XCTAssertTrue(app.buttons["local.create"].waitForExistence(timeout: 15))
        app.buttons["local.create"].tap()
        app.textFields["local.title"].tap()
        app.textFields["local.title"].typeText("选择器本机项目")
        dismissKeyboardTutorial(app)
        app.buttons["local.create.confirm"].tap()
        XCTAssertTrue(app.buttons["local.create.confirm"].waitForNonExistence(timeout: 5))
        XCTAssertTrue(app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'local.project.' AND label CONTAINS '选择器本机项目'")).firstMatch.waitForExistence(timeout: 5))

        for tab in ["Focal AI", "画布", "消息"] {
            app.tabBars.buttons[tab].tap()
            let context = app.buttons["project.context"]
            XCTAssertTrue(context.waitForExistence(timeout: 5))
            if tab != "Focal AI" { XCTAssertEqual(context.value as? String, "选择器本机项目", "跨 Tab 须保持选中的本机项目") }
            for (projectID, title, marker) in [(secondID, "切换项目乙", "乙项目内容"), (firstID, "切换项目甲", "甲项目内容")] {
                context.tap()
                let option = app.buttons["project.choose.\(projectID)"]
                XCTAssertTrue(option.waitForExistence(timeout: 10), "\(tab) 应能直接选择项目")
                option.tap()
                let selected = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == %@", title), object: context)
                XCTAssertEqual(XCTWaiter.wait(for: [selected], timeout: 10), .completed)
                XCTAssertFalse(app.buttons["project.choose.done"].exists, "打开项目后应收起选择页")
                if tab != "Focal AI" {
                    XCTAssertTrue(app.staticTexts[marker].waitForExistence(timeout: 10), "\(tab) 应刷新为所选项目内容")
                    let other = marker == "甲项目内容" ? "乙项目内容" : "甲项目内容"
                    XCTAssertFalse(app.staticTexts[other].exists, "不得残留另一项目的画布或消息")
                }
            }
            context.tap()
            let local = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'project.choose.local.' AND label CONTAINS '选择器本机项目'")).firstMatch
            XCTAssertTrue(local.waitForExistence(timeout: 5), "三个 Tab 均应显示本机项目")
            local.tap()
            XCTAssertTrue(app.buttons["project.context.local.library"].waitForExistence(timeout: 5))
            XCTAssertEqual(context.value as? String, "选择器本机项目", "点击本机项目应真正选中，而非仅导航到图库")
            XCTAssertFalse(app.buttons["project.choose.done"].exists)
            if tab == "画布" { XCTAssertTrue(app.otherElements["boards.local.project"].exists || app.staticTexts["本机画布暂不可用"].exists) }
            if tab == "消息" { XCTAssertTrue(app.staticTexts["本机项目暂无协作消息"].exists) }
            XCTAssertFalse(app.staticTexts["甲项目内容"].exists)
            app.buttons["project.context.local.library"].tap()
            XCTAssertTrue(app.navigationBars["本机图库"].waitForExistence(timeout: 5))
            popBack(app)
            XCTAssertEqual(context.value as? String, "选择器本机项目")
            context.tap()
            XCTAssertTrue(app.buttons["project.choose.done"].waitForExistence(timeout: 5))
            if tab == "消息" {
                let search = app.searchFields.firstMatch
                search.tap()
                search.typeText("没有这个项目")
                XCTAssertTrue(app.staticTexts["project.choose.empty"].waitForExistence(timeout: 5))
                XCTAssertFalse(app.buttons["project.choose.\(firstID)"].exists)
                // 系统搜索展开时收起导航工具栏，须先关闭搜索再结束选择。
                let closeSearch = app.buttons.matching(NSPredicate(format: "label IN %@", ["关闭", "取消", "Close", "Cancel"])).firstMatch
                XCTAssertTrue(closeSearch.waitForExistence(timeout: 5))
                closeSearch.tap()
                XCTAssertTrue(app.buttons["project.choose.done"].waitForExistence(timeout: 5))
            }
            let screenshot = XCTAttachment(screenshot: app.screenshot())
            screenshot.name = "\(tab) 项目选择"
            screenshot.lifetime = .keepAlways
            add(screenshot)
            app.buttons["project.choose.done"].tap()
            XCTAssertEqual(context.value as? String, "选择器本机项目", "关闭选择器后仍保留本机项目上下文")
        }
        app.tabBars.buttons["Focal AI"].tap()
        selectLocalProject(app, id: localFirstID, title: "本机项目甲")
        app.buttons["ai.attach"].tap()
        let localPhoto = app.buttons["ai.attach.photo.\(localPhotoID)"]
        XCTAssertTrue(localPhoto.waitForExistence(timeout: 5), "应直接读取选中本机项目的照片")
        localPhoto.tap()
        app.buttons["完成"].firstMatch.tap()
        app.textFields["ai.input"].tap()
        app.textFields["ai.input"].typeText("本机甲问题")
        app.buttons["ai.send"].tap()
        XCTAssertTrue(app.buttons["ai.vision.confirm"].waitForExistence(timeout: 10))
        app.buttons["取消"].firstMatch.tap()
        XCTAssertTrue(app.buttons["ai.send"].waitForExistence(timeout: 5))
        app.buttons["ai.send"].tap()
        XCTAssertTrue(app.buttons["ai.vision.confirm"].waitForExistence(timeout: 10))
        app.buttons["ai.vision.confirm"].tap()
        selectLocalProject(app, id: localSecondID, title: "本机项目乙")
        XCTAssertFalse(app.staticTexts["本机回复：本机甲问题"].exists, "甲的迟到结果不得进入乙的对话")
        let canSend = XCTNSPredicateExpectation(predicate: NSPredicate(format: "enabled == true"), object: app.textFields["ai.input"])
        XCTAssertEqual(XCTWaiter.wait(for: [canSend], timeout: 15), .completed)
        app.textFields["ai.input"].tap()
        app.textFields["ai.input"].typeText("本机乙问题")
        let sendReady = XCTNSPredicateExpectation(predicate: NSPredicate(format: "enabled == true"), object: app.buttons["ai.send"])
        XCTAssertEqual(XCTWaiter.wait(for: [sendReady], timeout: 15), .completed)
        app.buttons["ai.send"].tap()
        XCTAssertTrue(app.staticTexts["本机回复：本机乙问题"].waitForExistence(timeout: 20))
        XCTAssertFalse(app.staticTexts["本机回复：本机甲问题"].exists)
        selectLocalProject(app, id: localFirstID, title: "本机项目甲")
        XCTAssertTrue(app.staticTexts["本机回复：本机甲问题"].waitForExistence(timeout: 10))
        XCTAssertEqual(app.staticTexts.matching(NSPredicate(format: "label == '本机回复：本机甲问题'")).count, 1)
        XCTAssertFalse(app.staticTexts["本机回复：本机乙问题"].exists)
        app.terminate()
        app.launch()
        XCTAssertTrue(app.tabBars.buttons["Focal AI"].waitForExistence(timeout: 15))
        app.tabBars.buttons["Focal AI"].tap()
        selectLocalProject(app, id: localFirstID, title: "本机项目甲")
        XCTAssertTrue(app.staticTexts["本机回复：本机甲问题"].waitForExistence(timeout: 10), "重启后本机项目对话须能恢复")
        XCTAssertEqual(app.staticTexts.matching(NSPredicate(format: "label == '本机回复：本机甲问题'")).count, 1)
        XCTAssertFalse(app.staticTexts["本机回复：本机乙问题"].exists)
    }

    @MainActor
    private func dismissKeyboardTutorial(_ app: XCUIApplication) {
        let tutorial = app.staticTexts["Speed up your typing by sliding your finger across the letters to compose a word."]
        if tutorial.waitForExistence(timeout: 1) {
            let proceed = app.buttons["Continue"]
            XCTAssertTrue(proceed.exists)
            proceed.tap()
            XCTAssertTrue(proceed.waitForNonExistence(timeout: 5))
        }
    }

    @MainActor
    private func selectLocalProject(_ app: XCUIApplication, id: String, title: String) {
        let context = app.buttons["project.context"]
        XCTAssertTrue(context.waitForExistence(timeout: 5))
        context.tap()
        let option = app.buttons["project.choose.local.\(id)"]
        XCTAssertTrue(option.waitForExistence(timeout: 10))
        option.tap()
        let selected = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == %@", title), object: context)
        XCTAssertEqual(XCTWaiter.wait(for: [selected], timeout: 10), .completed)
        XCTAssertFalse(app.buttons["project.choose.done"].exists)
    }

    @MainActor
    func testNativeLoginLibrarySelectionAndLogout() throws {
        continueAfterFailure = false
        let environment = ProcessInfo.processInfo.environment
        guard let base = environment["FOCALROOM_IOS_TEST_BASE_URL"],
              let projectID = environment["FOCALROOM_IOS_TEST_PROJECT_ID"],
              let fileName = environment["FOCALROOM_IOS_TEST_FILE_NAME"] else {
            throw XCTSkip("请通过本地隔离后端验证脚本运行，不访问生产服务")
        }
        let app = XCUIApplication()
        app.launchEnvironment["FOCALROOM_IOS_REVIEW_URL"] = base
        app.launch()
        addUIInterruptionMonitor(withDescription: "测试账户的系统密码保存提示") { alert in
            let later = alert.buttons.matching(NSPredicate(format: "label IN %@", ["以后", "Not Now"])).firstMatch
            if later.exists { later.tap(); return true }
            return false
        }
        if app.buttons["账户"].waitForExistence(timeout: 3) {
            app.buttons["账户"].tap()
            logoutFromAccount(app)
        }
        XCTAssertTrue(app.textFields["login.email"].waitForExistence(timeout: 10))
        app.textFields["login.email"].tap()
        app.textFields["login.email"].typeText("iphone@fr.test")
        app.secureTextFields["login.password"].tap()
        app.secureTextFields["login.password"].typeText("iphone-local-test-2026")
        app.buttons["login.submit"].tap()
        dismissPasswordPrompt(app)
        let project = app.buttons["project.\(projectID)"]
        XCTAssertTrue(project.waitForExistence(timeout: 15))
        project.tap()
        // 提案基线必须与服务端当前版本一致：先在 rev2 上部分采纳（→rev3），再做 KEEP 选片（→rev4）。
        runProposalsSegment(app)
        let photo = app.buttons["photo.ios-photo-1"]
        XCTAssertTrue(photo.waitForExistence(timeout: 15))
        let libraryScreenshot = XCTAttachment(screenshot: app.screenshot())
        libraryScreenshot.name = "iPhone 图库"
        libraryScreenshot.lifetime = .keepAlways
        add(libraryScreenshot)
        photo.tap()
        XCTAssertTrue(app.images["preview.loaded.ios-photo-1"].waitForExistence(timeout: 15), "必须显示经过摘要核验的真实后端预览")
        XCTAssertTrue(app.buttons["selection.KEEP"].waitForExistence(timeout: 10))
        app.buttons["selection.KEEP"].tap()
        let selected = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == '已选' AND enabled == true"), object: app.buttons["selection.KEEP"])
        XCTAssertEqual(XCTWaiter.wait(for: [selected], timeout: 15), .completed)
        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "iPhone 授权预览与选片"
        screenshot.lifetime = .keepAlways
        add(screenshot)
        app.buttons["photo.compare"].tap()
        verifyComparison(app)
        runResearchSegment(app, projectID: projectID)
        verifySyncCenter(app, projectID: projectID)
        runHistorySegment(app)
        // 返回动画期间 TabBar 可能短暂缺席；先等其回到 AX 树再切标签。
        XCTAssertTrue(app.tabBars.firstMatch.waitForExistence(timeout: 15), "标签栏应可用")
        app.tabBars.buttons["消息"].tap()
        XCTAssertTrue(app.staticTexts["iPhone 本地隔离测试批注"].waitForExistence(timeout: 10))
        app.tabBars.buttons["画布"].tap()
        let board = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'board.'")).firstMatch
        XCTAssertTrue(board.waitForExistence(timeout: 10))
        board.tap()
        // 节点视图带标识符与手势后被提升为独立无障碍元素，内部预览元素被吸收；以节点元素断言渲染。
        let canvasNode = app.descendants(matching: .any)
            .matching(NSPredicate(format: "identifier BEGINSWITH 'canvas.node.'")).firstMatch
        XCTAssertTrue(canvasNode.waitForExistence(timeout: 10), "画布照片节点应渲染")
        XCTAssertTrue(app.descendants(matching: .any).matching(NSPredicate(format: "identifier == 'canvas.edit.hint'")).firstMatch
            .waitForExistence(timeout: 10), "草案画布应以编辑模式打开")
        // 在照片节点上执行真实拖拽：走 moveNodes 权威命令路径。
        let start = canvasNode.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
        let end = canvasNode.coordinate(withNormalizedOffset: CGVector(dx: 1.8, dy: 0.2))
        start.press(forDuration: 0.6, thenDragTo: end)
        let boardScreenshot = XCTAttachment(screenshot: app.screenshot())
        boardScreenshot.name = "iPhone 画布编辑"
        boardScreenshot.lifetime = .keepAlways
        add(boardScreenshot)
        app.tabBars.buttons["Focal AI"].tap()
        XCTAssertTrue(app.descendants(matching: .any)
            .matching(NSPredicate(format: "identifier == 'ai.balance'")).firstMatch
            .waitForExistence(timeout: 10), "应显示账户 AI 额度")
        let aiInput = app.textFields["ai.input"]
        XCTAssertTrue(aiInput.waitForExistence(timeout: 5))
        aiInput.tap()
        aiInput.typeText("帮我看看节奏")
        app.buttons["ai.send"].tap()
        let aiReply = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH '桩回复：'")).firstMatch
        XCTAssertTrue(aiReply.waitForExistence(timeout: 30), "应收到模型桩回复")
        let aiScreenshot = XCTAttachment(screenshot: app.screenshot())
        aiScreenshot.name = "iPhone Focal AI 对话"
        aiScreenshot.lifetime = .keepAlways
        add(aiScreenshot)
        // 收起键盘：return 键（中文键盘为“换行”）优先，否则滚动列表；键盘不收会遮挡标签栏。
        if app.keyboards.firstMatch.exists {
            let returnKey = app.keyboards.buttons.matching(NSPredicate(format: "label IN %@", ["return", "Return", "换行"])).firstMatch
            if returnKey.exists { returnKey.tap() } else { app.swipeUp() }
        }
        let keyboardGone = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"),
                                                     object: app.keyboards.firstMatch)
        XCTAssertEqual(XCTWaiter.wait(for: [keyboardGone], timeout: 5), .completed, "键盘应收起")
        app.buttons["ai.attach"].tap()
        let observedPhoto = app.buttons["ai.attach.photo.ios-photo-1"]
        XCTAssertTrue(observedPhoto.waitForExistence(timeout: 5))
        observedPhoto.tap()
        app.buttons["完成"].firstMatch.tap()
        aiInput.tap()
        aiInput.typeText("观察这张照片")
        app.buttons["ai.send"].tap()
        XCTAssertTrue(app.buttons["ai.vision.confirm"].waitForExistence(timeout: 15))
        app.buttons["取消"].firstMatch.tap()
        XCTAssertTrue(app.buttons["ai.send"].waitForExistence(timeout: 5))
        app.buttons["ai.send"].tap()
        XCTAssertTrue(app.buttons["ai.vision.confirm"].waitForExistence(timeout: 15), "取消授权后必须重新确认，不能自动发送")
        XCTAssertTrue(app.staticTexts["照片发送授权"].exists || app.navigationBars["照片发送授权"].exists)
        app.buttons["ai.vision.confirm"].tap()
        let visionReply = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH '桩回复：' AND label CONTAINS '观察 1 张预览'")).firstMatch
        XCTAssertTrue(visionReply.waitForExistence(timeout: 30), "仅同意后实际发送受限预览")
        if app.keyboards.firstMatch.exists {
            app.swipeUp()
        }
        app.tabBars.buttons["项目"].tap()
        // 中间段落的导航深度可能有残留：退到项目列表可见为止，最多三次。
        for _ in 0..<3 {
            if app.buttons["local.create"].waitForExistence(timeout: 5) { break }
            popBack(app)
        }
        XCTAssertTrue(app.buttons["local.create"].waitForExistence(timeout: 5), "应能退回项目列表")
        app.buttons["local.create"].tap()
        app.textFields["local.title"].tap()
        app.textFields["local.title"].typeText("iPhone 本机导入")
        app.buttons["local.create.confirm"].tap()
        let localProject = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'local.project.'")).firstMatch
        XCTAssertTrue(localProject.waitForExistence(timeout: 10))
        localProject.tap()
        pickTwoPhotos(app)
        let imported = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label CONTAINS '已导入 2 张'"), object: app.staticTexts["local.import.summary"])
        XCTAssertEqual(XCTWaiter.wait(for: [imported], timeout: 25), .completed)
        let localPhotos = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'local.photo.'"))
        XCTAssertEqual(localPhotos.count, 2)
        pickTwoPhotos(app)
        let duplicate = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label CONTAINS '跳过重复 2 张'"), object: app.staticTexts["local.import.summary"])
        XCTAssertEqual(XCTWaiter.wait(for: [duplicate], timeout: 25), .completed)
        XCTAssertEqual(localPhotos.count, 2)
        pickFile(app, filename: fileName)
        let fileImported = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label CONTAINS '已导入 1 张'"), object: app.staticTexts["local.import.summary"])
        XCTAssertEqual(XCTWaiter.wait(for: [fileImported], timeout: 25), .completed)
        XCTAssertEqual(localPhotos.count, 3)
        let localScreenshot = XCTAttachment(screenshot: app.screenshot())
        localScreenshot.name = "iPhone 本机图库与导入结果"
        localScreenshot.lifetime = .keepAlways
        add(localScreenshot)
        runLocalCanvasSegment(app)
        localPhotos.element(boundBy: 0).tap()
        app.buttons["local.selection.KEEP"].tap()
        let localSelected = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == '已选' AND enabled == true"), object: app.buttons["local.selection.KEEP"])
        XCTAssertEqual(XCTWaiter.wait(for: [localSelected], timeout: 10), .completed)
        app.buttons["local.compare"].tap()
        verifyComparison(app)
        migrateToCloud(app)
        app.terminate()
        app.launch()
        XCTAssertTrue(project.waitForExistence(timeout: 15), "钥匙串会话应在重启后恢复")
        XCTAssertTrue(localProject.waitForExistence(timeout: 10), "本机项目和导入记录应在重启后保留")
        let migratedBadge = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label CONTAINS '已迁云'"), object: localProject)
        XCTAssertEqual(XCTWaiter.wait(for: [migratedBadge], timeout: 10), .completed, "迁云状态应随本机记录持久化")
        app.buttons["账户"].tap()
        XCTAssertTrue(app.staticTexts["已登录设备"].waitForExistence(timeout: 10), "账户页应显示设备管理")
        XCTAssertTrue(app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'device.revoke.'")).firstMatch.waitForExistence(timeout: 10),
                      "本机设备应在登录后完成登记")
        // 账单区在长列表底部（含邀请入口后更低），先滚动让账单标题与方案行进入可视区。
        app.swipeUp()
        XCTAssertTrue(app.staticTexts["账单目录"].waitForExistence(timeout: 10), "账户页应显示账单目录")
        XCTAssertTrue(app.staticTexts["FocalRoom Pro"].waitForExistence(timeout: 10), "账单目录应列出服务方案")
        let deviceScreenshot = XCTAttachment(screenshot: app.screenshot())
        deviceScreenshot.name = "iPhone 账户设备管理"
        deviceScreenshot.lifetime = .keepAlways
        add(deviceScreenshot)
        logoutFromAccount(app)
        XCTAssertTrue(app.textFields["login.email"].waitForExistence(timeout: 10))
        app.terminate()
        app.launch()
        XCTAssertTrue(app.textFields["login.email"].waitForExistence(timeout: 10), "退出状态应在重启后保持")
        browseOfflineLibrary(app)
    }

    @MainActor
    func testZAccountDeletionRequiresExplicitConfirmation() throws {
        continueAfterFailure = false
        guard let base = ProcessInfo.processInfo.environment["FOCALROOM_IOS_TEST_BASE_URL"] else {
            throw XCTSkip("仅允许独立本地测试服务")
        }
        let app = XCUIApplication()
        app.launchEnvironment["FOCALROOM_IOS_REVIEW_URL"] = base
        app.launch()
        if app.buttons["账户"].waitForExistence(timeout: 2) {
            app.buttons["账户"].tap()
            logoutFromAccount(app)
        }
        XCTAssertTrue(app.textFields["login.email"].waitForExistence(timeout: 10))
        app.textFields["login.email"].tap()
        app.textFields["login.email"].typeText("erase-ui@fr.test")
        app.secureTextFields["login.password"].tap()
        app.secureTextFields["login.password"].typeText("iphone-local-test-2026")
        app.buttons["login.submit"].tap()
        dismissPasswordPrompt(app)
        XCTAssertTrue(app.buttons["账户"].waitForExistence(timeout: 15))
        app.buttons["账户"].tap()
        app.buttons["account.data.entry"].tap()
        XCTAssertTrue(app.buttons["account.data.export"].waitForExistence(timeout: 10))
        app.swipeUp()
        XCTAssertFalse(app.buttons["account.erase"].isEnabled, "未核对邮箱、密码和范围时不能删除")
        let email = app.textFields["account.erase.email"]
        XCTAssertTrue(email.waitForExistence(timeout: 5))
        email.tap(); email.typeText("erase-ui@fr.test")
        let password = app.secureTextFields["account.erase.password"]
        password.tap(); password.typeText("iphone-local-test-2026")
        app.swipeUp()
        let understood = app.switches["account.erase.understand"]
        understood.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)).tap()
        XCTAssertEqual(understood.value as? String, "1", "必须实际确认删除与保留范围")
        app.buttons["account.erase"].tap()
        XCTAssertTrue(app.buttons["确认删除账号"].waitForExistence(timeout: 5))
        app.buttons["取消"].firstMatch.tap()
        XCTAssertTrue(app.buttons["account.erase"].exists, "取消最终确认后账号仍保留")
        app.buttons["account.erase"].tap()
        app.buttons["确认删除账号"].tap()
        XCTAssertTrue(app.textFields["login.email"].waitForExistence(timeout: 15), "关闭回执后返回登录页")
    }

    @MainActor
    private func dismissPasswordPrompt(_ app: XCUIApplication) {
        let later = app.buttons.matching(NSPredicate(format: "label IN %@", ["以后", "Not Now"])).firstMatch
        guard later.waitForExistence(timeout: 4) else { return }
        // 系统 Sheet 暴露在宿主 AX 树中；用其实际屏幕框命中，避免跨服务坐标偏移。
        let frame = later.frame
        app.coordinate(withNormalizedOffset: .zero)
            .withOffset(CGVector(dx: frame.midX, dy: frame.midY)).tap()
        XCTAssertTrue(later.waitForNonExistence(timeout: 5), "系统密码提示应先关闭")
    }

    @MainActor
    private func logoutFromAccount(_ app: XCUIApplication) {
        let logout = app.buttons["account.logout"]
        for _ in 0..<4 {
            if logout.exists && logout.isHittable { break }
            app.swipeUp()
        }
        XCTAssertTrue(logout.waitForExistence(timeout: 3))
        logout.tap()
        app.buttons["退出并清理本机缓存"].tap()
    }

    @MainActor
    private func browseOfflineLibrary(_ app: XCUIApplication) {
        let offlineEntry = app.buttons["offline.library"]
        XCTAssertTrue(offlineEntry.waitForExistence(timeout: 10), "登录页应提供本机资料离线入口")
        offlineEntry.tap()
        let scope = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'offline.scope.'")).firstMatch
        XCTAssertTrue(scope.waitForExistence(timeout: 10), "离线入口应列出已登记账户资料")
        scope.tap()
        let project = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'local.project.'")).firstMatch
        XCTAssertTrue(project.waitForExistence(timeout: 10), "离线模式应列出本机项目")
        project.tap()
        let photos = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'local.photo.'"))
        XCTAssertEqual(photos.count, 3, "离线只读仍可浏览全部本机照片")
        photos.firstMatch.tap()
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS '离线只读'")).firstMatch.waitForExistence(timeout: 10),
                      "离线只读必须明确标识，不得冒充可编辑状态")
        let offlineScreenshot = XCTAttachment(screenshot: app.screenshot())
        offlineScreenshot.name = "iPhone 离线本机资料"
        offlineScreenshot.lifetime = .keepAlways
        add(offlineScreenshot)
        popBack(app)

        app.buttons["local.import"].tap()
        let export = app.buttons.matching(NSPredicate(format: "label CONTAINS '导出项目'")).firstMatch
        XCTAssertTrue(export.waitForExistence(timeout: 5), "离线模式应提供导出入口")
        export.tap()
        // 系统文件夹选择器部分渲染在远程进程；应用侧取消按钮呈现为 Other，需按元素类型分别轮询。
        // 本段是旅程终点，只需证明选择器出现，不做系统表单交互，避免跨进程不稳定。
        let cancelLabel = NSPredicate(format: "label == '取消' OR label == 'Cancel'")
        let signals = [
            app.buttons.matching(cancelLabel).firstMatch,
            app.otherElements.matching(cancelLabel).firstMatch,
            XCUIApplication(bundleIdentifier: "com.apple.DocumentsApp").buttons.matching(cancelLabel).firstMatch,
            app.staticTexts.matching(NSPredicate(format: "label CONTAINS '浏览' OR label CONTAINS 'Browse'")).firstMatch,
        ]
        let visible = signals.first { $0.waitForExistence(timeout: 15) }
        XCTAssertNotNil(visible, "导出应打开系统文件夹选择器")
        let exportSheet = XCTAttachment(screenshot: app.screenshot())
        exportSheet.name = "iPhone 导出文件夹选择器"
        exportSheet.lifetime = .keepAlways
        add(exportSheet)
    }

    @MainActor
    private func waitEditorGone(_ app: XCUIApplication) {
        let gone = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: app.buttons["research.save"])
        XCTAssertEqual(XCTWaiter.wait(for: [gone], timeout: 10), .completed, "保存成功后编辑表单应收起")
    }

    @MainActor
    private func runProposalsSegment(_ app: XCUIApplication) {
        app.buttons["proposals.entry"].tap()
        let row = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'proposals.row.'")).firstMatch
        XCTAssertTrue(row.waitForExistence(timeout: 10), "联调留下的 SUBMITTED 提案应出现在待审阅")
        row.tap()
        let toggles = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'proposals.op.'"))
        XCTAssertTrue(toggles.element(boundBy: 0).waitForExistence(timeout: 5))
        let accept = app.buttons["proposals.accept"]
        XCTAssertTrue(accept.waitForExistence(timeout: 5))
        // 取消第二项（move）：部分采纳只应用 add。
        toggles.element(boundBy: 1).tap()
        let oneLeft = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label CONTAINS '1 项'"), object: accept)
        XCTAssertEqual(XCTWaiter.wait(for: [oneLeft], timeout: 10), .completed, "取消第二项后勾选数应只剩一项")
        accept.tap()
        let confirm = app.buttons.matching(NSPredicate(format: "label CONTAINS '采纳 1 项'")).firstMatch
        XCTAssertTrue(confirm.waitForExistence(timeout: 5))
        confirm.tap()
        // 等详情表单完全收起再继续：收起动画会遮挡返回键；收起本身也是采纳成功的证明。
        let sheetGone = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: app.buttons["proposals.accept"])
        XCTAssertEqual(XCTWaiter.wait(for: [sheetGone], timeout: 10), .completed, "采纳后详情表单应收起")
        let submittedGone = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: app.staticTexts["待审阅"])
        XCTAssertEqual(XCTWaiter.wait(for: [submittedGone], timeout: 10), .completed, "唯一待审提案处理完毕后应清空待审阅区")
        // 表单收起动画期间存在两条导航栏，element(boundBy:0) 会解析到正在消失的那条；等已处理区出现再按系统返回键。
        XCTAssertTrue(app.staticTexts["已处理"].waitForExistence(timeout: 10), "处理后的提案应进入已处理区")
        let back = app.buttons["BackButton"].firstMatch
        XCTAssertTrue(back.waitForExistence(timeout: 10), "返回键应可用")
        back.tap()
    }

    @MainActor
    private func runLocalCanvasSegment(_ app: XCUIApplication) {
        app.buttons["local.import"].tap()
        let entry = app.buttons["local.canvas.entry"]
        XCTAssertTrue(entry.waitForExistence(timeout: 5), "本机项目菜单应提供画布入口")
        entry.tap()
        XCTAssertTrue(app.staticTexts["从右上角添加照片或便签开始布局"].waitForExistence(timeout: 10))

        // 便签：新建 → 撤销 → 重做
        app.buttons["canvas.add.note"].tap()
        let node = app.descendants(matching: .any)
            .matching(NSPredicate(format: "identifier BEGINSWITH 'local.canvas.node.'")).firstMatch
        XCTAssertTrue(node.waitForExistence(timeout: 10), "便签节点应出现")
        let noteText = app.staticTexts["便签：新便签"]
        XCTAssertTrue(noteText.waitForExistence(timeout: 5))
        app.buttons["canvas.undo"].tap()
        XCTAssertTrue(app.staticTexts["从右上角添加照片或便签开始布局"].waitForExistence(timeout: 10), "撤销后画布应清空")
        app.buttons["canvas.redo"].tap()
        XCTAssertTrue(noteText.waitForExistence(timeout: 10), "重做后便签应恢复")

        // 照片节点：从项目照片选择添加
        app.buttons["canvas.add.photo"].tap()
        let pick = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'canvas.pick.photo.'")).firstMatch
        XCTAssertTrue(pick.waitForExistence(timeout: 10), "照片选择表应列出项目照片")
        pick.tap()
        let canvasScreenshot = XCTAttachment(screenshot: app.screenshot())
        canvasScreenshot.name = "iPhone 本机画布编辑"
        canvasScreenshot.lifetime = .keepAlways
        add(canvasScreenshot)
        popBack(app)
    }

    @MainActor
    private func runHistorySegment(_ app: XCUIApplication) {
        app.buttons["history.entry"].tap()
        let rows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'version.row.'"))
        XCTAssertTrue(rows.firstMatch.waitForExistence(timeout: 10), "版本历史应列出已提交版本")
        // List 内按钮命中会被文本截获（研究段同类问题），按坐标选择两个版本。
        if rows.count >= 2 {
            rows.element(boundBy: 0).coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
            rows.element(boundBy: 1).coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        }
        let diffHeader = app.staticTexts.matching(NSPredicate(format: "label CONTAINS '差异'")).firstMatch
        XCTAssertTrue(diffHeader.waitForExistence(timeout: 15), "选择两个版本后应显示差异区")
        let historyScreenshot = XCTAttachment(screenshot: app.screenshot())
        historyScreenshot.name = "iPhone 版本历史与差异"
        historyScreenshot.lifetime = .keepAlways
        add(historyScreenshot)
        popBack(app)
    }

    @MainActor
    private func verifySyncCenter(_ app: XCUIApplication, projectID: String) {
        popBack(app)
        app.buttons["sync.center"].tap()
        XCTAssertTrue(app.staticTexts["已缓存项目"].waitForExistence(timeout: 10))
        let row = app.descendants(matching: .any)
            .matching(NSPredicate(format: "identifier BEGINSWITH 'sync.row.'")).firstMatch
        XCTAssertTrue(row.waitForExistence(timeout: 10), "同步中心应列出已缓存项目")
        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "iPhone 同步中心"
        screenshot.lifetime = .keepAlways
        add(screenshot)
        app.buttons["完成"].firstMatch.tap()
        // 重进夹具项目，恢复后续步骤假设的导航栈深度。
        let projectButton = app.buttons["project.\(projectID)"]
        XCTAssertTrue(projectButton.waitForExistence(timeout: 10))
        projectButton.tap()
    }

    private enum Anchor { case library, root, viewer, unknown }

    // 导航栏按钮在本模拟器上间歇不可解析；回退用左缘右滑（根视图上是安全空操作），不做盲点坐标点击。
    @MainActor
    private func popBack(_ app: XCUIApplication) {
        let button = app.navigationBars.buttons.element(boundBy: 0)
        if button.waitForExistence(timeout: 3) {
            button.tap()
            return
        }
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.01, dy: 0.5))
            .press(forDuration: 0.05, thenDragTo: app.coordinate(withNormalizedOffset: CGVector(dx: 0.7, dy: 0.5)))
    }

    @MainActor
    private func runResearchSegment(_ app: XCUIApplication, projectID: String) {
        // 比较表单收起后系统时序不定（照片页/图库/项目根，AX 树会短暂缺失导航栏）；
        // 以三个锚点轮询直至任一出现，再按位置回到图库。
        var anchor: Anchor = .unknown
        for _ in 0..<15 {
            if app.buttons["research.entry"].firstMatch.exists { anchor = .library; break }
            if app.buttons["project.\(projectID)"].firstMatch.exists { anchor = .root; break }
            if app.buttons["photo.compare"].firstMatch.exists { anchor = .viewer; break }
            Thread.sleep(forTimeInterval: 1)
        }
        switch anchor {
        case .library: break
        case .root:
            app.buttons["project.\(projectID)"].tap()
            XCTAssertTrue(app.buttons["research.entry"].firstMatch.waitForExistence(timeout: 10), "应进入图库层")
        case .viewer:
            // 表单收起动画期间导航栏可能整体不在 AX 树；用左缘右滑触发系统返回手势。
            Thread.sleep(forTimeInterval: 2)
            let edge = app.coordinate(withNormalizedOffset: CGVector(dx: 0.01, dy: 0.5))
            let target = app.coordinate(withNormalizedOffset: CGVector(dx: 0.7, dy: 0.5))
            edge.press(forDuration: 0.05, thenDragTo: target)
            if !app.buttons["research.entry"].firstMatch.waitForExistence(timeout: 10) {
                edge.press(forDuration: 0.05, thenDragTo: target)
            }
            XCTAssertTrue(app.buttons["research.entry"].firstMatch.waitForExistence(timeout: 10), "应退回图库层")
        case .unknown:
            XCTFail("比较表单收起后应有可识别的落点")
        }
        app.buttons["research.entry"].tap()
        XCTAssertTrue(app.staticTexts["还没有研究资料"].waitForExistence(timeout: 10))
        app.buttons["research.create"].tap()
        let title = app.textFields["research.title"]
        XCTAssertTrue(title.waitForExistence(timeout: 5))
        title.tap()
        title.typeText("iPhone 研究笔记")
        XCTAssertTrue(app.buttons["research.voice.start"].waitForExistence(timeout: 5), "语音入口应可见")
        let body = app.textViews["research.body"]
        XCTAssertTrue(body.waitForExistence(timeout: 5))
        body.tap()
        body.typeText("盲页开场节奏参考。")
        let researchScreenshot = XCTAttachment(screenshot: app.screenshot())
        researchScreenshot.name = "iPhone 研究笔记编辑与语音入口"
        researchScreenshot.lifetime = .keepAlways
        add(researchScreenshot)
        app.buttons["research.save"].tap()
        waitEditorGone(app)
        let row = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'research.row.'")).firstMatch
        XCTAssertTrue(row.waitForExistence(timeout: 10), "保存后应回到列表并显示新条目")
        // 卡片中心命中会被内部文本截获（与相册网格同类），按坐标执行系统触摸。
        row.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        XCTAssertTrue(title.waitForExistence(timeout: 5))
        title.tap()
        title.typeText("（更新）")
        app.buttons["research.save"].tap()
        waitEditorGone(app)
        XCTAssertTrue(row.waitForExistence(timeout: 10))
        // 列表保存后重载有一次数据刷新；稳定后重开详情做删除。
        Thread.sleep(forTimeInterval: 1)
        row.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        // 删除区在表单底部；先确认表单打开，再上滚露出删除区（懒加载行在元素点击时会重解析，改坐标点击）。
        row.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        XCTAssertTrue(app.buttons["research.save"].waitForExistence(timeout: 10), "详情表单应打开")
        app.swipeUp()
        let delete = app.buttons["research.delete"]
        XCTAssertTrue(delete.waitForExistence(timeout: 5), "上滚后应见删除入口")
        delete.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        let confirm = app.buttons.matching(NSPredicate(format: "label == '删除'")).firstMatch
        XCTAssertTrue(confirm.waitForExistence(timeout: 5))
        confirm.tap()
        XCTAssertTrue(app.staticTexts["还没有研究资料"].waitForExistence(timeout: 10), "删除后应回到空列表")
        popBack(app)
    }

    @MainActor
    private func migrateToCloud(_ app: XCUIApplication) {
        popBack(app)
        app.buttons["local.import"].tap()
        let migrate = app.buttons.matching(NSPredicate(format: "label CONTAINS '迁云'")).firstMatch
        XCTAssertTrue(migrate.waitForExistence(timeout: 5), app.debugDescription)
        migrate.tap()
        let confirm = app.buttons["local.migrate.confirm"].firstMatch
        XCTAssertTrue(confirm.waitForExistence(timeout: 5), app.debugDescription)
        confirm.tap()
        let open = app.buttons["local.migrate.open"].firstMatch
        XCTAssertTrue(open.waitForExistence(timeout: 120), "迁云应在真实后端上完成并显示云端项目入口")
        let progress = XCTAttachment(screenshot: app.screenshot())
        progress.name = "iPhone 迁云完成"
        progress.lifetime = .keepAlways
        add(progress)
        open.tap()
        let cloudPhotos = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'photo.'"))
        let cloudLibraryReady = XCTNSPredicateExpectation(predicate: NSPredicate(format: "count == 3"), object: cloudPhotos)
        XCTAssertEqual(XCTWaiter.wait(for: [cloudLibraryReady], timeout: 30), .completed, "云端项目应包含迁云的全部照片")
        let cloudScreenshot = XCTAttachment(screenshot: app.screenshot())
        cloudScreenshot.name = "iPhone 迁云后的云端图库"
        cloudScreenshot.lifetime = .keepAlways
        add(cloudScreenshot)
        popBack(app)
    }

    @MainActor
    private func verifyComparison(_ app: XCUIApplication) {
        XCTAssertTrue(app.buttons["compare.left.zoomIn"].waitForExistence(timeout: 10))
        app.buttons["compare.left.zoomIn"].tap()
        XCTAssertEqual(app.staticTexts["compare.left.zoom"].label, "2.0×")
        XCTAssertEqual(app.staticTexts["compare.right.zoom"].label, "2.0×")
        app.buttons["compare.lock"].tap()
        XCTAssertEqual(app.buttons["compare.lock"].value as? String, "独立调整")
        app.buttons["compare.right.zoomIn"].tap()
        XCTAssertEqual(app.staticTexts["compare.left.zoom"].label, "2.0×")
        XCTAssertEqual(app.staticTexts["compare.right.zoom"].label, "4.0×")
        app.buttons["compare.reset"].tap()
        XCTAssertEqual(app.staticTexts["compare.left.zoom"].label, "1.0×")
        XCTAssertEqual(app.staticTexts["compare.right.zoom"].label, "1.0×")
        app.buttons["compare.swap"].tap()
        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "iPhone 双图比较"
        screenshot.lifetime = .keepAlways
        add(screenshot)
        app.buttons["完成"].firstMatch.tap()
    }

    @MainActor
    private func pickTwoPhotos(_ app: XCUIApplication) {
        app.buttons["local.import"].tap()
        app.buttons["从相册导入"].tap()
        let images = app.images.matching(identifier: "PXGGridLayout-Info")
        if images.firstMatch.waitForExistence(timeout: 5) {
            XCTAssertGreaterThanOrEqual(images.count, 2)
            // iOS 26 网格的 AX Image 没有独立命中点；在实际图像框中心执行系统触摸。
            images.element(boundBy: 0).coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
            images.element(boundBy: 1).coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        } else {
            let cells = app.collectionViews.cells
            XCTAssertTrue(cells.firstMatch.waitForExistence(timeout: 10), app.debugDescription)
            XCTAssertGreaterThanOrEqual(cells.count, 2)
            cells.element(boundBy: 0).tap()
            cells.element(boundBy: 1).tap()
        }
        let add = app.navigationBars.buttons.matching(NSPredicate(format: "label BEGINSWITH '添加' OR label BEGINSWITH 'Add' OR label == '完成' OR label == 'Done'")).firstMatch
        XCTAssertTrue(add.waitForExistence(timeout: 5), app.debugDescription)
        add.tap()
    }

    @MainActor
    private func pickFile(_ app: XCUIApplication, filename: String) {
        app.buttons["local.import"].tap()
        app.buttons["从文件导入"].tap()
        let basename = String(filename.dropLast(4))
        let file = app.cells.matching(NSPredicate(format: "label CONTAINS %@", basename)).firstMatch
        if !file.waitForExistence(timeout: 5) {
            let browse = app.buttons.matching(NSPredicate(format: "label == '浏览' OR label == 'Browse'")).firstMatch
            if browse.exists { browse.tap() }
            let onDevice = app.descendants(matching: .any).matching(NSPredicate(format: "label IN %@", ["我的iPhone", "我的 iPhone", "On My iPhone"])).firstMatch
            XCTAssertTrue(onDevice.waitForExistence(timeout: 5), app.debugDescription)
            onDevice.tap()
            let folder = app.cells.matching(NSPredicate(format: "label BEGINSWITH 'FocalRoom'")).firstMatch
            XCTAssertTrue(folder.waitForExistence(timeout: 5), app.debugDescription)
            folder.tap()
        }
        XCTAssertTrue(file.waitForExistence(timeout: 5), app.debugDescription)
        file.tap()
        let open = app.buttons.matching(NSPredicate(format: "label == '打开' OR label == 'Open'")).firstMatch
        if open.waitForExistence(timeout: 3), open.isEnabled { open.tap() }
    }
}
