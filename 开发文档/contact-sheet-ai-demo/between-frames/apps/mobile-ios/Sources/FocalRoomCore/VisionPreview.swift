import Foundation
import ImageIO
import CoreGraphics
import UniformTypeIdentifiers
import CryptoKit

// 只接收已核验的云端浏览 JPEG；重新生成受限像素，不外发原片或来源元数据。
public struct VisionPreview: Sendable, Equatable {
    public let jpeg: Data
    public let width: Int
    public let height: Int
    public var dataURL: String { "data:image/jpeg;base64," + jpeg.base64EncodedString() }

    public static func make(jpeg: Data) throws -> Self {
        guard !jpeg.isEmpty, jpeg.count <= 16 * 1024 * 1024,
              let source = CGImageSourceCreateWithData(jpeg as CFData, [kCGImageSourceShouldCache: false] as CFDictionary),
              CGImageSourceGetType(source) as String? == UTType.jpeg.identifier,
              CGImageSourceGetCount(source) == 1,
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let width = properties[kCGImagePropertyPixelWidth] as? Int,
              let height = properties[kCGImagePropertyPixelHeight] as? Int,
              width > 0, width <= 2048, height > 0, height <= 2048,
              let image = CGImageSourceCreateThumbnailAtIndex(source, 0, [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceCreateThumbnailWithTransform: true,
                kCGImageSourceThumbnailMaxPixelSize: 1400,
                kCGImageSourceShouldCacheImmediately: true
              ] as CFDictionary),
              let color = CGColorSpace(name: CGColorSpace.sRGB),
              let context = CGContext(data: nil, width: image.width, height: image.height, bitsPerComponent: 8,
                                      bytesPerRow: 0, space: color, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
        else { throw APIError.invalidResponse }
        context.setFillColor(CGColor(gray: 1, alpha: 1))
        context.fill(CGRect(x: 0, y: 0, width: image.width, height: image.height))
        context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
        let output = NSMutableData()
        guard let rendered = context.makeImage(),
              let target = CGImageDestinationCreateWithData(output, UTType.jpeg.identifier as CFString, 1, nil)
        else { throw APIError.invalidResponse }
        CGImageDestinationAddImage(target, rendered, [kCGImageDestinationLossyCompressionQuality: 0.75] as CFDictionary)
        guard CGImageDestinationFinalize(target), output.length > 0 else { throw APIError.oversizedResponse }
        // 像素确实是上面那个 sRGB 上下文重绘出来的，所以身份要写进文件，而不是等接收方把无标签
        // JPEG 默认当成 sRGB：ImageIO 重编码只留 JFIF，不写 ICC 段，读侧报出的 "sRGB IEC61966-2.1"
        // 是假定出来的名字，不是这份字节里的事实。
        let declared = taggedWithSRGBProfile(output as Data)
        guard declared.count <= 2 * 1024 * 1024 else { throw APIError.oversizedResponse }
        return Self(jpeg: declared, width: image.width, height: image.height)
    }

    /// 系统 sRGB 配置字节。拿不到时返回 nil，让这份上传件如实留白，而不是拦下用户的发送。
    static func srgbProfileBytes() -> Data? {
        guard let space = CGColorSpace(name: CGColorSpace.sRGB), let icc = space.copyICCData(),
              CFDataGetLength(icc) > 0, let bytes = CFDataGetBytePtr(icc)
        else { return nil }
        return Data(bytes: bytes, count: CFDataGetLength(icc))
    }

    /// JPEG 的 ICC 约定：配置切成最多 65519 字节的段，序号与总数各占一字节，紧接在 SOI 之后。
    /// 与桌面端写上传件用的是同一个形状，两端交出去的字节至少在这件事上可以对齐。
    static func tag(_ jpeg: Data, icc: Data) -> Data {
        let chunk = 65519
        guard jpeg.count >= 2, jpeg[jpeg.startIndex] == 0xFF, jpeg[jpeg.startIndex + 1] == 0xD8,
              !icc.isEmpty, icc.count <= chunk * 255
        else { return jpeg }
        let segments = (icc.count + chunk - 1) / chunk
        var out = Data(capacity: jpeg.count + icc.count + segments * 18)
        out.append(contentsOf: [0xFF, 0xD8])
        for segment in 0..<segments {
            let start = icc.startIndex + segment * chunk
            let piece = icc.subdata(in: start..<min(start + chunk, icc.endIndex))
            out.append(contentsOf: [0xFF, 0xE2])
            let length = UInt16(piece.count + 16)
            out.append(UInt8(length >> 8))
            out.append(UInt8(length & 0xFF))
            out.append(contentsOf: Array("ICC_PROFILE".utf8))
            out.append(0)
            out.append(UInt8(segment + 1))
            out.append(UInt8(segments))
            out.append(piece)
        }
        out.append(jpeg.subdata(in: jpeg.startIndex + 2..<jpeg.endIndex))
        return out
    }

    private static func taggedWithSRGBProfile(_ jpeg: Data) -> Data {
        guard let icc = srgbProfileBytes() else { return jpeg }
        return tag(jpeg, icc: icc)
    }
}
