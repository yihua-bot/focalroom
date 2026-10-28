// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "FocalRoomCore",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [.library(name: "FocalRoomCore", targets: ["FocalRoomCore"])],
    targets: [
        .target(name: "FocalRoomCore"),
        .testTarget(name: "FocalRoomCoreTests", dependencies: ["FocalRoomCore"])
    ]
)
