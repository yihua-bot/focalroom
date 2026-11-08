import Foundation
import XCTest
@testable import FocalRoomCore

/// 手机侧读的是桌面那一套投影里的 draftEvidence：只有数量与档位，正文不在这条通道上。
final class NativeDraftEvidenceTests: XCTestCase {
    private func task(_ evidence: Any?) throws -> NativeAITask {
        var payload: [String: Any] = ["id": "task", "goal": "挑出十张", "state": "review_ready", "reason": "",
                                      "messages": [], "boardDrafts": [], "inspectedPhotos": [], "preferenceProposals": [],
                                      "budget": try JSONSerialization.jsonObject(with: JSONEncoder().encode(NativeAIBudget()))]
        if let evidence { payload["draftEvidence"] = evidence }
        return try JSONDecoder().decode(NativeAITask.self, from: JSONSerialization.data(withJSONObject: payload))
    }

    func testLegacyTaskWithoutEvidenceStaysReadable() throws {
        XCTAssertNil(try task(nil).draftEvidence)
    }

    func testCountsAndPerPhotoTiersSurviveAndRevisionNullMeansUnrecorded() throws {
        let decoded = try task([[
            "draftId": "draft", "count": 12, "observed": 9, "unobserved": 3,
            "pageTier": 7, "detailTier": 2, "otherVersion": 0, "otherTool": 0,
            "crossContractPairs": 1, "omitted": 2, "scanned": 9, "draftRevision": NSNull(),
            "perPhoto": [
                ["photoId": "kept", "contract": "current_detail_tier", "used": true],
                ["photoId": "broken", "contract": NSNull(), "used": false,
                 "pixelVersion": "mismatch", "originalStatus": "missing"],
            ],
        ]])
        let evidence = try XCTUnwrap(decoded.draftEvidence?.first)
        XCTAssertEqual(evidence.draftId, "draft")
        XCTAssertEqual(evidence.observed, 9)
        XCTAssertEqual(evidence.detailTier, 2)
        XCTAssertEqual(evidence.omitted, 2)
        XCTAssertEqual(evidence.scanned, 9)
        XCTAssertNil(evidence.draftRevision, "版本号没记过就是没记过，界面不许猜")
        XCTAssertFalse(evidence.basisSummary)
        let kept = try XCTUnwrap(evidence.perPhoto.first { $0.photoId == "kept" })
        XCTAssertEqual(kept.contract, "current_detail_tier")
        XCTAssertTrue(kept.used)
        XCTAssertNil(kept.pixelVersion)
        let broken = try XCTUnwrap(evidence.perPhoto.first { $0.photoId == "broken" })
        XCTAssertNil(broken.contract)
        XCTAssertEqual(broken.pixelVersion, "mismatch")
        XCTAssertEqual(broken.originalStatus, "missing")
    }

    func testNamesOutsideTheWhitelistAreTreatedAsUnrecorded() throws {
        let decoded = try task([["draftId": "draft", "count": 1, "observed": 1, "unobserved": 0, "pageTier": 1,
                                 "detailTier": 0, "otherVersion": 0, "otherTool": 0, "crossContractPairs": 0,
                                 "omitted": 0, "scanned": 1, "draftRevision": 4,
                                 "perPhoto": [["photoId": "p", "contract": "wild_guess", "used": true,
                                               "pixelVersion": "verified", "originalStatus": "whatever"]]]])
        let evidence = try XCTUnwrap(decoded.draftEvidence?.first)
        let row = try XCTUnwrap(evidence.perPhoto.first)
        XCTAssertNil(row.contract)
        XCTAssertNil(row.pixelVersion)
        XCTAssertNil(row.originalStatus)
        XCTAssertEqual(evidence.draftRevision, 4)
    }

    func testDegradedEntryCarriesNoPerPhotoButSaysSo() throws {
        let decoded = try task([["draftId": "draft", "count": 40, "observed": 14, "unobserved": 26, "pageTier": 14,
                                 "detailTier": 0, "otherVersion": 0, "otherTool": 0, "crossContractPairs": 0,
                                 "omitted": 0, "scanned": 14, "draftRevision": NSNull(),
                                 "perPhoto": [], "basisSummary": true]])
        let evidence = try XCTUnwrap(decoded.draftEvidence?.first)
        XCTAssertTrue(evidence.basisSummary)
        XCTAssertTrue(evidence.perPhoto.isEmpty)
        XCTAssertEqual(evidence.scanned, evidence.observed)
    }

    // 口径只是锦上添花：形状不合时读不回来就什么也不说，不能把整份任务投影一起带走。
    func testMalformedEvidenceDropsOnlyTheEvidence() throws {
        let brokenPayloads: [Any] = [
            "not-an-array",
            [["draftId": "draft", "observed": 1]],
            [["draftId": "draft", "count": 1, "observed": 1, "unobserved": 0, "pageTier": 1, "detailTier": 0,
              "otherVersion": 0, "otherTool": 0, "crossContractPairs": 0, "omitted": 0, "scanned": 1]],
        ]
        for broken in brokenPayloads {
            let decoded = try task(broken)
            XCTAssertEqual(decoded.goal, "挑出十张")
            XCTAssertEqual(decoded.state, "review_ready")
            XCTAssertTrue(decoded.needsReview)
            XCTAssertNil(decoded.draftEvidence)
        }
    }
}
