import AppKit
import AVFoundation
import CoreGraphics
import ScreenCaptureKit

struct CaptureConfig: Decodable {
    let sourceId: String
    let kind: String
    let region: Region?
    let microphone: Bool
}
struct Region: Decodable { let x: Double; let y: Double; let width: Double; let height: Double }

func availableContent() async throws -> SCShareableContent {
    try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
}

func sourceList() async throws {
    let content = try await availableContent()
    var result: [[String: Any]] = []
    for display in content.displays {
        let title = "Display \(display.displayID)"
        result.append(["id": String(display.displayID), "kind": "screen", "title": title, "width": display.width, "height": display.height])
        result.append(["id": String(display.displayID), "kind": "region", "title": "Region on \(title)", "width": display.width, "height": display.height])
    }
    for window in content.windows where window.frame.width >= 200 && window.frame.height >= 100 {
        let owner = window.owningApplication?.applicationName ?? "Application"
        let title = (window.title?.isEmpty == false) ? "\(owner) — \(window.title!)" : owner
        result.append(["id": String(window.windowID), "kind": "window", "title": title,
                       "width": Int(window.frame.width * 2), "height": Int(window.frame.height * 2)])
    }
    let json = try JSONSerialization.data(withJSONObject: result)
    print(String(data: json, encoding: .utf8)!, terminator: "\n")
}

final class CaptureSession: NSObject, SCStreamOutput, SCStreamDelegate {
    private let config: CaptureConfig
    private let outputURL: URL
    private let cursorURL: URL
    private var stream: SCStream?
    private var writer: AVAssetWriter?
    private var videoInput: AVAssetWriterInput?
    private var audioInput: AVAssetWriterInput?
    private var file: FileHandle?
    private var timer: DispatchSourceTimer?
    private var frame = CGRect.zero
    private var startUptime = 0.0
    private var startedWriter = false
    private let lock = NSLock()

    init(config: CaptureConfig, outputURL: URL, cursorURL: URL) {
        self.config = config; self.outputURL = outputURL; self.cursorURL = cursorURL
    }

    func start() async throws {
        let content = try await availableContent()
        let source: SCContentFilter
        let settings = SCStreamConfiguration()
        if config.kind == "window" {
            guard let window = content.windows.first(where: { String($0.windowID) == config.sourceId }) else { throw NSError(domain: "Screenbox", code: 1, userInfo: [NSLocalizedDescriptionKey: "Window disappeared"]) }
            source = SCContentFilter(desktopIndependentWindow: window)
            frame = window.frame
            settings.width = max(2, Int(window.frame.width * 2) / 2 * 2)
            settings.height = max(2, Int(window.frame.height * 2) / 2 * 2)
        } else {
            guard let display = content.displays.first(where: { String($0.displayID) == config.sourceId }) else { throw NSError(domain: "Screenbox", code: 2, userInfo: [NSLocalizedDescriptionKey: "Display disappeared"]) }
            let excluded = content.applications.filter { $0.bundleIdentifier == "com.morso.screenbox" }
            source = SCContentFilter(display: display, excludingApplications: excluded, exceptingWindows: [])
            frame = display.frame
            settings.width = display.width
            settings.height = display.height
            if config.kind == "region", let region = config.region {
                settings.sourceRect = CGRect(x: frame.width * region.x, y: frame.height * region.y,
                                             width: frame.width * region.width, height: frame.height * region.height)
                frame = CGRect(x: frame.minX + settings.sourceRect.minX, y: frame.minY + settings.sourceRect.minY,
                               width: settings.sourceRect.width, height: settings.sourceRect.height)
                settings.width = max(2, Int(Double(display.width) * region.width) / 2 * 2)
                settings.height = max(2, Int(Double(display.height) * region.height) / 2 * 2)
            }
        }
        settings.minimumFrameInterval = CMTime(value: 1, timescale: 30)
        settings.queueDepth = 5
        settings.showsCursor = true
        settings.captureMicrophone = config.microphone
        settings.capturesAudio = false

        let writer = try AVAssetWriter(outputURL: outputURL, fileType: .mp4)
        let video = AVAssetWriterInput(mediaType: .video, outputSettings: [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: settings.width, AVVideoHeightKey: settings.height,
            AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: 12_000_000]
        ])
        video.expectsMediaDataInRealTime = true
        writer.add(video)
        videoInput = video
        if config.microphone {
            let audio = AVAssetWriterInput(mediaType: .audio, outputSettings: [
                AVFormatIDKey: kAudioFormatMPEG4AAC, AVSampleRateKey: 48_000,
                AVNumberOfChannelsKey: 1, AVEncoderBitRateKey: 128_000
            ])
            audio.expectsMediaDataInRealTime = true
            writer.add(audio)
            audioInput = audio
        }
        self.writer = writer
        FileManager.default.createFile(atPath: cursorURL.path, contents: nil)
        file = try FileHandle(forWritingTo: cursorURL)
        let stream = SCStream(filter: source, configuration: settings, delegate: self)
        try stream.addStreamOutput(self, type: .screen, sampleHandlerQueue: DispatchQueue(label: "screenbox.video"))
        if config.microphone { try stream.addStreamOutput(self, type: .microphone, sampleHandlerQueue: DispatchQueue(label: "screenbox.audio")) }
        try await stream.startCapture()
        self.stream = stream
        startUptime = ProcessInfo.processInfo.systemUptime
        let cursorTimer = DispatchSource.makeTimerSource(queue: DispatchQueue(label: "screenbox.cursor"))
        cursorTimer.schedule(deadline: .now(), repeating: .milliseconds(33))
        cursorTimer.setEventHandler { [weak self] in self?.saveCursor() }
        cursorTimer.resume()
        timer = cursorTimer
    }

    private func saveCursor() {
        guard let location = CGEvent(source: nil)?.location else { return }
        let x = (location.x - frame.minX) / frame.width
        let y = (location.y - frame.minY) / frame.height
        guard x.isFinite && y.isFinite else { return }
        let t = max(0, Int((ProcessInfo.processInfo.systemUptime - startUptime) * 1000))
        let line = "{\"tMs\":\(t),\"x\":\(x),\"y\":\(y)}\n"
        if let data = line.data(using: .utf8) { try? file?.write(contentsOf: data) }
    }

    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard sampleBuffer.isValid else { return }
        lock.lock(); defer { lock.unlock() }
        guard let writer = writer else { return }
        if type == .screen {
            if !startedWriter {
                writer.startWriting()
                writer.startSession(atSourceTime: CMSampleBufferGetPresentationTimeStamp(sampleBuffer))
                startedWriter = true
            }
            if videoInput?.isReadyForMoreMediaData == true { videoInput?.append(sampleBuffer) }
        } else if type == .microphone && startedWriter && audioInput?.isReadyForMoreMediaData == true {
            audioInput?.append(sampleBuffer)
        }
    }

    func stream(_ stream: SCStream, didStopWithError error: Error) {
        fputs("Capture interrupted: \(error.localizedDescription)\n", stderr)
    }

    func stop() async throws {
        timer?.cancel(); timer = nil
        try await stream?.stopCapture()
        file?.closeFile(); file = nil
        lock.withLock {
            videoInput?.markAsFinished()
            audioInput?.markAsFinished()
        }
        guard let writer = writer, startedWriter else { throw NSError(domain: "Screenbox", code: 3, userInfo: [NSLocalizedDescriptionKey: "No video frames were captured"]) }
        await writer.finishWriting()
        if writer.status != .completed { throw writer.error ?? NSError(domain: "Screenbox", code: 4) }
    }
}

@main struct Main {
    static func main() async {
        do {
            let args = CommandLine.arguments
            if args.count == 2 && args[1] == "sources" { try await sourceList(); return }
            guard args.count == 5 && args[1] == "record", let configData = args[2].data(using: .utf8) else {
                throw NSError(domain: "Screenbox", code: 5, userInfo: [NSLocalizedDescriptionKey: "Invalid capture command"])
            }
            let config = try JSONDecoder().decode(CaptureConfig.self, from: configData)
            let session = CaptureSession(config: config, outputURL: URL(fileURLWithPath: args[3]), cursorURL: URL(fileURLWithPath: args[4]))
            try await session.start()
            print("READY", terminator: "\n")
            fflush(stdout)
            while let line = readLine() { if line == "stop" { break } }
            try await session.stop()
        } catch {
            print("ERROR: \(error.localizedDescription)", terminator: "\n")
            fflush(stdout)
            exit(1)
        }
    }
}
