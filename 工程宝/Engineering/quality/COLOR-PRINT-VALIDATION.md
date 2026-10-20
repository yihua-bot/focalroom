# FocalRoom Color / RAW / Print Validation Protocol

## 标准样本集

- 8/16-bit sRGB/Adobe RGB/Display P3/ProPhoto TIFF 或 JPEG（有效 ICC）；ICC v2、ICC v4、自定义 scanner ICC、灰阶 ICC。
- RAW：CR2/CR3、NEF、ARW、RAF、RW2、DNG；至少 3 家传感器图像，明确内部 RAW renderer 不等同外部厂商引擎。
- 100MP 原片、1GB+ 16-bit TIFF 扫描件、透明 PNG/HEIC、EXIF 1–8 方向、损坏 ICC / 没有 ICC、嵌入 JPEG 的 RAW。
- 已调整的 Lightroom/Capture One 导出 8-bit JPEG 与 16-bit TIFF 同一场景，并保留外部权威样图和导出配置。

## 色彩验证路径

1. 冻结输入源 SHA256、源 ICC digest、预期解码、目标显示 profile、观测仪器/模式。
2. 在参考色彩管理库与 FocalRoom Native Renderer 中转换到共同可比较颜色空间；采样色块/均匀梯度/肤色/暗部，报告 ΔE00 和统计分布。
3. Native Viewer、Canvas 和 Publications 比较同一 image rendition 像素/显示值；输出由同一变换定义驱动，但不得直接比较未指定 ICC 的 raw RGB 字节。
4. 对不同显示器重复同等条件；移动窗口时确保 profile 改变引发正确缓存失效。辅助人员观察不得替代仪器误差验证。
5. Web 另建立“带管理 profile 的代理”边界，并在浏览器/OS 的 ICC 限制下明确 warning。

## RAW 与大图检视

- 辨别 Camera-embedded JPEG、native preview、external editor rendition 和 decode-from-RAW 的不同身份，并在 UI 的 `Source` 面板列出。
- 对 100MP 和巨幅 TIFF，测 Fit, 100% 与快速拖动下可见区域真实瓦片、CPU/GPU 峰值、解码时间；不允许显示缓存被 1400px 上采样当作真 100%。
- 源文件在线/掉线/权限撤销/修改后，原 Viewer 均不会静默继续宣称读取的是源文件。

## 印前验证

- `PrintContract` 由目标供应商提供：开本、页数、纸张、装订、成品/出血/安全区、输出 TIFF 或 PDF/X 版本、ICC 与色彩空间、输出锐化、字体、透明度、裁切线。
- Preflight 必须检查：缺图、不同版本/断链、图框裁切超出、最终有效 PPI、字体缺失/嵌入、文字溢出、页码、图片是否 8-bit/16-bit 与 ICC、PDF output intent、出血和 trim-box、源 Rendition 内容 hash。
- PDF/X-4 不能仅把元信息中的字符串写为“PDF/X-4”；需要合规渲染和独立专业验证。艺术微喷 TIFF 必须按供应商真实接受的文件/ICC 规格交付。
- 同一冻结 Edition 可再版；若设计允许时间戳等非决定性元数据，报告说明为何二进制 hash 不能绝对重现，关键视觉资产/布局/ICC/文件内容仍需可比对。

## 未决实验（必须可测）

A. 原生 NSView/Metal 与 Tauri WebView 合成是否支持精确 ColorSync、多屏及 VoiceOver？
B. 统一 Canvas 命令模型的 React + native renderer 能否在触控板高频移动中同步不漂移？
C. libvips/ColorSync 输出 P3/AdobeRGB/ProPhoto 的转换差异在何条件可接受？
D. Capture One SDK 在目标版本可否稳定提供 Variant ID、Session/Catalog 定位和回流触发？
E. 针对合作印厂的 PDF/X、OutputIntent 和纸张 ICC 应采用何种专业输出引擎，是否需要已认证第三方组件？
