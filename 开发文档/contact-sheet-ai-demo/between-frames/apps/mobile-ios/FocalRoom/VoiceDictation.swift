import SwiftUI
import AVFAudio
import Speech
import Observation

// 语音转写：iOS 26 设备端 SpeechAnalyzer/SpeechTranscriber，音频不上传；
// 低版本、缺资源或无权限时明确提示并回退键盘输入，不伪装成可用。
@MainActor @Observable
final class VoiceDictationModel {
    private(set) var isListening = false
    private(set) var transcript = ""
    private(set) var errorText: String?
    // SpeechAnalyzer 为 iOS 26 类型；存储属性不能标注 available，统一装箱为 AnyObject。
    @ObservationIgnored private var liveSession: AnyObject?
    @ObservationIgnored private var task: Task<Void, Never>?

    func start() {
        guard !isListening else { return }
        isListening = true
        errorText = nil
        task = Task { await run() }
    }

    func stop() {
        if #available(iOS 26.0, *), let session = liveSession as? LiveSession {
            session.stop()
            liveSession = nil
        }
        task?.cancel()
        task = nil
        isListening = false
    }

    func reset() {
        transcript = ""
        errorText = nil
    }

    private func run() async {
        do {
            guard await AVAudioApplication.requestRecordPermission() else {
                fail("未获麦克风权限；可在系统设置中开启后重试，或使用键盘输入。")
                return
            }
            guard #available(iOS 26.0, *) else {
                fail("语音输入需要 iOS 26 或更新系统；当前系统请使用键盘输入。")
                return
            }
            try await transcribeLive()
        } catch is CancellationError {
        } catch {
            fail("语音转写暂不可用，请使用键盘输入。")
        }
        stop()
    }

    @available(iOS 26.0, *)
    private func transcribeLive() async throws {
        let locale = Locale(identifier: "zh-CN")
        guard SpeechTranscriber.isAvailable else {
            fail("此设备不支持设备端语音转写，请使用键盘输入。")
            return
        }
        guard await SpeechTranscriber.supportedLocale(equivalentTo: locale) != nil,
              await SpeechTranscriber.installedLocales.contains(where: { $0.identifier.hasPrefix("zh") })
        else {
            fail("设备缺少中文语音资源；可在系统设置中安装后重试，或使用键盘输入。")
            return
        }
        let transcriber = SpeechTranscriber(locale: locale, preset: .progressiveTranscription)
        let (stream, continuation) = AsyncStream<AnalyzerInput>.makeStream()
        let analyzer = try SpeechAnalyzer(modules: [transcriber])
        let session = LiveSession(analyzer: analyzer, continuation: continuation)
        let engine = session.engine
        let format = engine.inputNode.outputFormat(forBus: 0)
        guard format.sampleRate > 0, format.channelCount > 0 else {
            fail("当前设备没有可用的音频输入，请使用键盘输入。")
            return
        }
        liveSession = session
        engine.inputNode.installTap(onBus: 0, bufferSize: 4096, format: format) { buffer, _ in
            continuation.yield(AnalyzerInput(buffer: buffer))
        }
        engine.prepare()
        try engine.start()

        // 渐进结果按音频时间段聚合；volatile 段的文本会被同一时间键的新结果替换。
        let results = try await transcriber.results
        let consuming = Task { [weak self] in
            var segments: [Double: String] = [:]
            for try await result in results {
                guard let self, !Task.isCancelled else { break }
                segments[result.range.start.seconds] = String(result.text.characters)
                self.transcript = segments.sorted { $0.key < $1.key }.map(\.value).joined(separator: " ")
            }
        }
        defer { consuming.cancel() }
        try await analyzer.start(inputSequence: stream)
        try await consuming.value
    }

    private func fail(_ text: String) {
        errorText = text
        isListening = false
    }
}

@available(iOS 26.0, *)
private final class LiveSession: @unchecked Sendable {
    let engine = AVAudioEngine()
    private let analyzer: SpeechAnalyzer
    private let continuation: AsyncStream<AnalyzerInput>.Continuation

    init(analyzer: SpeechAnalyzer, continuation: AsyncStream<AnalyzerInput>.Continuation) {
        self.analyzer = analyzer
        self.continuation = continuation
    }

    func stop() {
        continuation.finish()
        engine.inputNode.removeTap(onBus: 0)
        engine.stop()
        Task { await analyzer.cancelAndFinishNow() }
    }
}

struct VoiceEntrySection: View {
    @Bindable var voice: VoiceDictationModel
    let insert: (String) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if voice.isListening {
                Label("正在聆听…", systemImage: "waveform")
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("research.voice.listening")
                if !voice.transcript.isEmpty {
                    Text(voice.transcript).font(.subheadline).foregroundStyle(.secondary)
                }
                Button("停止") { voice.stop() }
                    .accessibilityIdentifier("research.voice.stop")
            } else {
                Button {
                    voice.start()
                } label: {
                    Label("语音输入", systemImage: "mic.fill")
                }
                .accessibilityIdentifier("research.voice.start")
                Text("转写在本机完成，音频不会上传。需要 iOS 26 与中文语音资源。")
                    .font(.caption2).foregroundStyle(.tertiary)
            }
            if let error = voice.errorText {
                Text(error).font(.caption).foregroundStyle(.secondary)
            }
            if !voice.isListening, !voice.transcript.isEmpty {
                Button("把转写并入笔记") {
                    insert(voice.transcript)
                    voice.reset()
                }
                .accessibilityIdentifier("research.voice.insert")
            }
        }
    }
}
