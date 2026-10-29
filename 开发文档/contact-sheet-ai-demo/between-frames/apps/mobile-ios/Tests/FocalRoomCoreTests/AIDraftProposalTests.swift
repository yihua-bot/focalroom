import Foundation
import XCTest
@testable import FocalRoomCore

// AI 草案解析契约：模型回复是不可信数据——合法提取、照片闭包校验与全部拒绝路径。
final class AIDraftProposalTests: XCTestCase {
    private let photos = [
        Photo(id: "p1", filename: "Sunrise_01.jpg", source_sha256: "a", selection: .UNDECIDED),
        Photo(id: "p2", filename: "Harbor.jpeg", source_sha256: "b", selection: .UNDECIDED),
    ]

    private func reply(_ json: String, prefix: String = "根据你的项目，我建议：\n", suffix: String = "\n以上供参考。") -> String {
        prefix + "```json\n" + json + "\n```" + suffix
    }

    func testParsesValidProposalWithFilenameTolerance() throws {
        let text = reply(#"{"title":"初选建议","operations":[{"op":"add","photo":"sunrise_01.JPG","to_index":0},{"op":"move","photo":" Harbor.jpeg ","to_index":2},{"op":"remove","photo":"Sunrise_01.jpg"}]}"#)
        let draft = try XCTUnwrap(AIDraftProposal.parse(text, photos: photos))
        XCTAssertEqual(draft.title, "初选建议")
        XCTAssertEqual(draft.operations.count, 3)
        XCTAssertEqual(draft.operations.map(\.op), ["add", "move", "remove"])
        XCTAssertEqual(draft.operations[0].photoFilename, "Sunrise_01.jpg", "文件名归一到项目内规范名")
        XCTAssertEqual(draft.operations[1].photoFilename, "Harbor.jpeg")
        XCTAssertTrue(draft.operations[2].toIndex == nil, "remove 不需要 to_index")
    }

    func testRejectsMalformedOrUntrustedInput() {
        // 无代码块 / 无 JSON
        XCTAssertNil(AIDraftProposal.parse("没有代码块的普通回复", photos: photos))
        XCTAssertNil(AIDraftProposal.parse("```\n不是 json\n```", photos: photos))
        // 标题空或超长
        XCTAssertNil(AIDraftProposal.parse(reply(#"{"title":"","operations":[{"op":"add","photo":"Sunrise_01.jpg","to_index":0}]}"#), photos: photos))
        let longTitle = String(repeating: "长", count: 201)
        XCTAssertNil(AIDraftProposal.parse(reply(#"{"title":"\#(longTitle)","operations":[{"op":"add","photo":"Sunrise_01.jpg","to_index":0}]}"#), photos: photos))
        // 操作为空
        XCTAssertNil(AIDraftProposal.parse(reply(#"{"title":"t","operations":[]}"#), photos: photos))
        // 未知操作 / 引用项目外照片 / move 缺索引 / add 负索引
        XCTAssertNil(AIDraftProposal.parse(reply(#"{"title":"t","operations":[{"op":"delete","photo":"Sunrise_01.jpg"}]}"#), photos: photos))
        XCTAssertNil(AIDraftProposal.parse(reply(#"{"title":"t","operations":[{"op":"add","photo":"机密文件.jpg","to_index":0}]}"#), photos: photos), "照片闭包外的文件必须整单拒绝")
        XCTAssertNil(AIDraftProposal.parse(reply(#"{"title":"t","operations":[{"op":"move","photo":"Sunrise_01.jpg"}]}"#), photos: photos))
        XCTAssertNil(AIDraftProposal.parse(reply(#"{"title":"t","operations":[{"op":"add","photo":"Sunrise_01.jpg","to_index":-1}]}"#), photos: photos))
        // 一条坏操作污染整份草案
        XCTAssertNil(AIDraftProposal.parse(reply(#"{"title":"t","operations":[{"op":"add","photo":"Sunrise_01.jpg","to_index":0},{"op":"add","photo":"别的.jpg","to_index":1}]}"#), photos: photos))
    }

    func testFirstJSONBlockHandlesLanguageTagAndNestedFences() {
        XCTAssertEqual(AIDraftProposal.firstJSONBlock(in: "```swift\nprint(1)\n```") == nil, false, "任意 fence 内容都按字节提取，交由 JSON 解析把关")
        XCTAssertNil(AIDraftProposal.firstJSONBlock(in: "``` \n{}"), "未闭合 fence 拒绝")
        let data = AIDraftProposal.firstJSONBlock(in: "说明 ```json\n{\"a\":1}\n``` 尾注")
        XCTAssertEqual(try? JSONSerialization.jsonObject(with: data ?? Data()) as? [String: Int], ["a": 1], "带语言标记与首尾噪声时仍取第一个块")
    }
}
