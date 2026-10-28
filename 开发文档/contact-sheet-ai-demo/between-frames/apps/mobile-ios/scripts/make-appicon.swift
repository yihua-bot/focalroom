// 生成 App Store 用 1024x1024 不透明图标：品牌图形合成到 #f4f3ee 暖白底
// 用法: swift make-appicon.swift <mark.png> <输出目录>
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers
import Foundation

let args = CommandLine.arguments
guard args.count == 3 else { FileHandle.standardError.write("用法: swift make-appicon.swift <mark.png> <输出目录>\n".data(using: .utf8)!); exit(2) }
let markURL = URL(fileURLWithPath: args[1])
let outDir = URL(fileURLWithPath: args[2])

guard let src = CGImageSourceCreateWithURL(markURL as CFURL, nil),
      let mark = CGImageSourceCreateImageAtIndex(src, 0, nil) else {
    FileHandle.standardError.write("无法读取 \(markURL.path)\n".data(using: .utf8)!); exit(1)
}

let size = 1024
let margin: CGFloat = 112 // 约 11% 边距：系统裁圆角后图形仍完整
guard let ctx = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: 0,
                          space: CGColorSpace(name: CGColorSpace.sRGB)!,
                          bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue) else {
    FileHandle.standardError.write("无法创建上下文\n".data(using: .utf8)!); exit(1)
}
// 官网主题色暖白底
ctx.setFillColor(CGColor(red: 0xF4/255.0, green: 0xF3/255.0, blue: 0xEE/255.0, alpha: 1))
ctx.fill(CGRect(x: 0, y: 0, width: size, height: size))
ctx.interpolationQuality = .high
ctx.draw(mark, in: CGRect(x: margin, y: margin, width: CGFloat(size) - 2 * margin, height: CGFloat(size) - 2 * margin))
guard let composed = ctx.makeImage() else { FileHandle.standardError.write("合成失败\n".data(using: .utf8)!); exit(1) }

// 写 1024 png
let iconURL = outDir.appendingPathComponent("appicon-1024.png")
guard let dest = CGImageDestinationCreateWithURL(iconURL as CFURL, UTType.png.identifier as CFString, 1, nil) else {
    FileHandle.standardError.write("无法创建输出\n".data(using: .utf8)!); exit(1)
}
CGImageDestinationAddImage(dest, composed, nil)
guard CGImageDestinationFinalize(dest) else { FileHandle.standardError.write("写入失败\n".data(using: .utf8)!); exit(1) }

// 验证：无 alpha、尺寸、以及 mark 与背景的对比（统计暗像素占比）
var luminance = 0.0, darkPixels = 0, totalPixels = 0
if let data = ctx.data {
    let ptr = data.bindMemory(to: UInt8.self, capacity: size * size * 4)
    for i in stride(from: 0, to: size * size * 4, by: 4) {
        let r = Double(ptr[i]), g = Double(ptr[i + 1]), b = Double(ptr[i + 2])
        let l = 0.2126 * r + 0.7152 * g + 0.0722 * b
        luminance += l; totalPixels += 1
        if l < 128 { darkPixels += 1 }
    }
}
print("OK \(iconURL.path) 1024x1024 无alpha")
print(String(format: "平均亮度 %.1f/255，深色像素占比 %.1f%%（图形可见性检查）", luminance / Double(totalPixels), Double(darkPixels) / Double(totalPixels) * 100))
