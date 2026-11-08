import Foundation
import XCTest
import ImageIO
import CoreGraphics
import UniformTypeIdentifiers
@testable import FocalRoomCore

// 交给模型的像素必须有写得出来的身份：像素确实落在 sRGB 上下文里，标签就要在字节里，
// 不能只靠接收方把无标签 JPEG 默认当成 sRGB。
final class VisionPreviewICCTests: XCTestCase {
    private func sourceJPEG(width: Int, height: Int) throws -> Data {
        let color = try XCTUnwrap(CGColorSpace(name: CGColorSpace.sRGB))
        let context = try XCTUnwrap(CGContext(data: nil, width: width, height: height, bitsPerComponent: 8,
                                              bytesPerRow: 0, space: color,
                                              bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue))
        context.setFillColor(CGColor(red: 0.2, green: 0.4, blue: 0.8, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: width, height: height))
        let output = NSMutableData()
        let destination = try XCTUnwrap(CGImageDestinationCreateWithData(output, UTType.jpeg.identifier as CFString, 1, nil))
        CGImageDestinationAddImage(destination, try XCTUnwrap(context.makeImage()),
                                   [kCGImageDestinationLossyCompressionQuality: 0.9] as CFDictionary)
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return output as Data
    }

    /// JPEG 段扫描：返回每个 APP2 ICC_PROFILE 段的 (序号, 总数, 配置字节)。
    private func iccSegments(in jpeg: Data) -> [(index: Int, count: Int, payload: Data)] {
        let bytes = [UInt8](jpeg)
        var segments: [(Int, Int, Data)] = []
        var cursor = 2
        while cursor + 4 <= bytes.count {
            guard bytes[cursor] == 0xFF else { cursor += 1; continue }
            let marker = bytes[cursor + 1]
            if marker == 0xDA { break }   // 压缩数据开始，段表到此为止
            let length = (Int(bytes[cursor + 2]) << 8) | Int(bytes[cursor + 3])
            guard length >= 16, cursor + 2 + length <= bytes.count else { break }
            let name = bytes[(cursor + 4)..<(cursor + 15)]
            if marker == 0xE2, Array(name) == Array("ICC_PROFILE".utf8) {
                let payload = jpeg.subdata(in: (cursor + 18)..<(cursor + 2 + length))
                segments.append((Int(bytes[cursor + 16]), Int(bytes[cursor + 17]), payload))
            }
            cursor += 2 + length
        }
        return segments
    }

    func testUploadBytesCarryTheSystemSRGBProfile() throws {
        let preview = try VisionPreview.make(jpeg: sourceJPEG(width: 2048, height: 1200))
        let icc = try XCTUnwrap(VisionPreview.srgbProfileBytes(), "系统应交出 sRGB 配置")
        XCTAssertEqual(Array(icc.subdata(in: 16..<20)), Array("RGB ".utf8), "取来的配置必须是 RGB")
        let segments = iccSegments(in: preview.jpeg)
        XCTAssertEqual(segments.count, 1, "无标签 JPEG 会被接收方当成 sRGB 默认，所以标签必须真的写进去")
        XCTAssertEqual(segments[0].index, 1)
        XCTAssertEqual(segments[0].count, 1)
        XCTAssertEqual(segments[0].payload, icc)
        // 写完标签的这份仍是一张读得回来、尺寸没变的 JPEG。
        let source = try XCTUnwrap(CGImageSourceCreateWithData(preview.jpeg as CFData, nil))
        let properties = try XCTUnwrap(CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any])
        XCTAssertEqual(preview.width, 1400)
        XCTAssertEqual(properties[kCGImagePropertyPixelWidth] as? Int, preview.width)
        XCTAssertNotNil(try XCTUnwrap(CGImageSourceCreateImageAtIndex(source, 0, nil)))
    }

    func testChunkingFollowsTheJpegICCConvention() throws {
        let chunk = 65519
        let payload = Data((0..<(chunk + 10)).map { UInt8($0 % 251) })
        let jpeg = Data([0xFF, 0xD8, 0xFF, 0xE0]) + Data(repeating: 0, count: 32)
        let tagged = VisionPreview.tag(jpeg, icc: payload)
        let segments = iccSegments(in: tagged)
        XCTAssertEqual(segments.count, 2, "超过单段上限要按 65519 切段")
        XCTAssertEqual(segments.map(\.index), [1, 2])
        XCTAssertEqual(segments.map(\.count), [2, 2])
        XCTAssertEqual(segments[0].payload.count, chunk)
        XCTAssertEqual(segments[0].payload + segments[1].payload, payload)
        // 标签插在 SOI 之后，原有字节一个不少地紧跟其后。
        XCTAssertEqual(tagged.subdata(in: 0..<2), Data([0xFF, 0xD8]))
        XCTAssertEqual(tagged.suffix(jpeg.count - 2), jpeg.subdata(in: 2..<jpeg.count))
    }

    /// 写标签只许动段表，不许动像素：补上 sRGB 声明前后，同一张图渲染出来的字节必须一模一样。
    func testTaggingLeavesEveryPixelUntouched() throws {
        let original = try sourceJPEG(width: 640, height: 400)
        let icc = try XCTUnwrap(VisionPreview.srgbProfileBytes())
        let tagged = VisionPreview.tag(original, icc: icc)
        XCTAssertGreaterThan(tagged.count, original.count)
        func pixels(_ data: Data) throws -> [UInt8] {
            let source = try XCTUnwrap(CGImageSourceCreateWithData(data as CFData, nil))
            let image = try XCTUnwrap(CGImageSourceCreateImageAtIndex(source, 0, nil))
            let space = try XCTUnwrap(CGColorSpace(name: CGColorSpace.sRGB))
            let context = try XCTUnwrap(CGContext(data: nil, width: image.width, height: image.height,
                                                  bitsPerComponent: 8, bytesPerRow: image.width * 4, space: space,
                                                  bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue))
            context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
            return [UInt8](Data(bytes: try XCTUnwrap(context.data), count: image.width * image.height * 4))
        }
        XCTAssertEqual(try pixels(tagged), try pixels(original), "声明要与像素本来就有的一致，不能顺手改像素")
    }

    func testTaggingRefusesToInventADeclaration() throws {
        let icc = try XCTUnwrap(VisionPreview.srgbProfileBytes())
        let jpeg = Data([0xFF, 0xD8, 0xFF, 0xD9])
        XCTAssertEqual(VisionPreview.tag(Data("not a jpeg".utf8), icc: icc), Data("not a jpeg".utf8),
                       "不是 JPEG 就不塞段")
        XCTAssertEqual(VisionPreview.tag(jpeg, icc: Data()), jpeg, "没有配置字节就不写空段")
        XCTAssertTrue(iccSegments(in: VisionPreview.tag(jpeg, icc: Data())).isEmpty)
    }
}
