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
        guard CGImageDestinationFinalize(target), output.length > 0, output.length <= 2 * 1024 * 1024 else {
            throw APIError.oversizedResponse
        }
        return Self(jpeg: output as Data, width: image.width, height: image.height)
    }
}
