import Foundation
import XCTest
@testable import FocalRoomCore

final class NativeChatJobTests: XCTestCase {
    func testOnlyCompletedReceiptCanContainFinalText() throws {
        let decoder = JSONDecoder()
        let completed = try decoder.decode(NativeChatJob.self, from: Data(#"{"state":"completed","text":"完整回复"}"#.utf8))
        XCTAssertEqual(completed.text, "完整回复")
        for state in ["pending", "unknown", "rejected", "expired"] {
            XCTAssertNil(try decoder.decode(NativeChatJob.self, from: Data("{\"state\":\"\(state)\",\"text\":null}".utf8)).text)
            XCTAssertThrowsError(try decoder.decode(NativeChatJob.self, from: Data("{\"state\":\"\(state)\",\"text\":\"半截\"}".utf8)))
        }
        for json in [#"{"state":"completed"}"#, #"{"state":"completed","text":""}"#, #"{"state":"other"}"#] {
            XCTAssertThrowsError(try decoder.decode(NativeChatJob.self, from: Data(json.utf8)))
        }
    }
}
