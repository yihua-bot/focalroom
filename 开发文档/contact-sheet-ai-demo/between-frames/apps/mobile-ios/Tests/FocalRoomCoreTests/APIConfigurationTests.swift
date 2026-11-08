
import XCTest
@testable import FocalRoomCore

final class APIConfigurationPrefixTests: XCTestCase {
    func testPrefixedOriginBuildsEndpoints() throws {
        let config = try APIConfiguration("https://api.example.net/focalroom")
        XCTAssertEqual(config.origin.absoluteString, "https://api.example.net/focalroom")
        let url = try config.endpoint(["auth", "login"])
        XCTAssertEqual(url.absoluteString, "https://api.example.net/focalroom/api/v2/auth/login")
    }

    func testRejectsUnsafeAddresses() {
        XCTAssertThrowsError(try APIConfiguration("https://host/a?x=1"))
        XCTAssertThrowsError(try APIConfiguration("https://host/a#frag"))
        XCTAssertThrowsError(try APIConfiguration("https://host//double"))
        XCTAssertThrowsError(try APIConfiguration("http://host/focalroom"))
    }

    func testIdentityKeepsPrefix() throws {
        let config = try APIConfiguration("https://api.example.net/focalroom/")
        XCTAssertEqual(config.identity, "https://api.example.net/focalroom")
    }
}
