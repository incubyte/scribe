import AVFoundation
import Foundation
import Speech
import SwiftRs

private let inputSampleRate = 16_000.0

private struct BridgeError: LocalizedError {
  let errorDescription: String?

  init(_ message: String) {
    errorDescription = message
  }
}

private struct EventPayload: Encodable {
  let kind: String
  let text: String
  let start_s: Double?
  let end_s: Double?
}

private struct Reply: Encodable {
  var ok = true
  var error: String?
  var events: [EventPayload]?
  var finalised_through_s: Double?
  var session: Int?
  var status: String?

  static func failure(_ error: Error) -> Reply {
    Reply(ok: false, error: error.localizedDescription)
  }
}

private func encoded(_ reply: Reply) -> SRString {
  let data = (try? JSONEncoder().encode(reply)) ?? Data("{\"ok\":false}".utf8)
  return SRString(String(decoding: data, as: UTF8.self))
}

private func blocking(_ operation: @escaping @Sendable () async -> Reply) -> SRString {
  let semaphore = DispatchSemaphore(value: 0)
  let box = ReplyBox()
  Task.detached {
    box.reply = await operation()
    semaphore.signal()
  }
  semaphore.wait()
  return encoded(box.reply)
}

private final class ReplyBox: @unchecked Sendable {
  var reply = Reply(ok: false, error: "no reply")
}

private func makeTranscriber(_ locale: Locale) -> SpeechTranscriber {
  SpeechTranscriber(
    locale: locale,
    transcriptionOptions: [],
    reportingOptions: [.volatileResults],
    attributeOptions: [.audioTimeRange]
  )
}

private func supportedLocale(_ identifier: String) async throws -> Locale {
  guard let locale = await SpeechTranscriber.supportedLocale(equivalentTo: Locale(identifier: identifier))
  else {
    throw BridgeError("SpeechAnalyzer does not support \(identifier)")
  }
  return locale
}

private func payload(from result: SpeechTranscriber.Result) -> EventPayload {
  let text = String(result.text.characters)
  guard result.isFinal else {
    return EventPayload(kind: "partial", text: text, start_s: nil, end_s: nil)
  }
  return EventPayload(
    kind: "final",
    text: text,
    start_s: result.range.start.seconds,
    end_s: CMTimeRangeGetEnd(result.range).seconds
  )
}

private func decodeSamples(_ bytes: [UInt8]) -> [Float] {
  stride(from: 0, to: bytes.count - bytes.count % 4, by: 4).map { offset in
    let bits =
      UInt32(bytes[offset]) | UInt32(bytes[offset + 1]) << 8 | UInt32(bytes[offset + 2]) << 16
      | UInt32(bytes[offset + 3]) << 24
    return Float(bitPattern: bits)
  }
}

private final class LaneSession: @unchecked Sendable {
  let analyzer: SpeechAnalyzer
  let continuation: AsyncStream<AnalyzerInput>.Continuation
  let audio: AudioAdapter
  var collector: Task<Void, Never>?
  private let lock = NSLock()
  private var queued: [EventPayload] = []
  private var failure: String?
  private var heardSeconds = 0.0
  private var outstandingVolatileStart: Double?

  init(analyzer: SpeechAnalyzer, continuation: AsyncStream<AnalyzerInput>.Continuation, audio: AudioAdapter) {
    self.analyzer = analyzer
    self.continuation = continuation
    self.audio = audio
  }

  func collect(_ results: some AsyncSequence<SpeechTranscriber.Result, any Error>) async {
    do {
      for try await result in results {
        let event = payload(from: result)
        lock.withLock {
          queued.append(event)
          trackVolatility(of: result)
        }
      }
    } catch {
      lock.withLock { failure = error.localizedDescription }
    }
  }

  private func trackVolatility(of result: SpeechTranscriber.Result) {
    let start = result.range.start.seconds
    guard result.isFinal else {
      outstandingVolatileStart = min(outstandingVolatileStart ?? start, start)
      return
    }
    if let volatileStart = outstandingVolatileStart, CMTimeRangeGetEnd(result.range).seconds > volatileStart {
      outstandingVolatileStart = nil
    }
  }

  func feed(_ samples: [Float]) throws {
    lock.withLock { heardSeconds += Double(samples.count) / inputSampleRate }
    guard let buffer = try audio.buffer(for: samples) else { return }
    continuation.yield(AnalyzerInput(buffer: buffer))
  }

  func takeEvents() throws -> [EventPayload] {
    try takeProgress().events
  }

  func takeProgress() throws -> (events: [EventPayload], finalisedThrough: Double) {
    try lock.withLock {
      if let failure { throw BridgeError(failure) }
      defer { queued.removeAll() }
      return (queued, outstandingVolatileStart ?? heardSeconds)
    }
  }
}

private struct AudioAdapter {
  let inputFormat: AVAudioFormat
  let analyzerFormat: AVAudioFormat
  let converter: AVAudioConverter?

  static func make(for transcriber: SpeechTranscriber) async throws -> AudioAdapter {
    guard
      let analyzerFormat = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [transcriber]),
      let inputFormat = AVAudioFormat(
        commonFormat: .pcmFormatFloat32, sampleRate: inputSampleRate, channels: 1, interleaved: false)
    else {
      throw BridgeError("SpeechAnalyzer offered no usable audio format")
    }
    let converter = inputFormat == analyzerFormat ? nil : AVAudioConverter(from: inputFormat, to: analyzerFormat)
    return AudioAdapter(inputFormat: inputFormat, analyzerFormat: analyzerFormat, converter: converter)
  }

  func buffer(for samples: [Float]) throws -> AVAudioPCMBuffer? {
    guard !samples.isEmpty, let input = pcmBuffer(samples) else { return nil }
    guard let converter else { return input }
    return try convert(input, with: converter)
  }

  private func pcmBuffer(_ samples: [Float]) -> AVAudioPCMBuffer? {
    let frames = AVAudioFrameCount(samples.count)
    guard let buffer = AVAudioPCMBuffer(pcmFormat: inputFormat, frameCapacity: frames),
      let channel = buffer.floatChannelData
    else { return nil }
    buffer.frameLength = frames
    samples.withUnsafeBufferPointer { channel[0].update(from: $0.baseAddress!, count: samples.count) }
    return buffer
  }

  private func convert(_ input: AVAudioPCMBuffer, with converter: AVAudioConverter) throws -> AVAudioPCMBuffer? {
    let ratio = analyzerFormat.sampleRate / inputFormat.sampleRate
    let capacity = AVAudioFrameCount(Double(input.frameLength) * ratio) + 1_024
    guard let output = AVAudioPCMBuffer(pcmFormat: analyzerFormat, frameCapacity: capacity) else { return nil }
    var consumed = false
    var conversionError: NSError?
    converter.convert(to: output, error: &conversionError) { _, status in
      if consumed {
        status.pointee = .noDataNow
        return nil
      }
      consumed = true
      status.pointee = .haveData
      return input
    }
    if let conversionError { throw conversionError }
    return output.frameLength > 0 ? output : nil
  }
}

private final class SessionRegistry: @unchecked Sendable {
  private let lock = NSLock()
  private var sessions: [Int: LaneSession] = [:]
  private var nextToken = 1

  func add(_ session: LaneSession) -> Int {
    lock.withLock {
      let token = nextToken
      nextToken += 1
      sessions[token] = session
      return token
    }
  }

  func find(_ token: Int) throws -> LaneSession {
    guard let session = lock.withLock({ sessions[token] }) else {
      throw BridgeError("no speech session \(token)")
    }
    return session
  }

  func remove(_ token: Int) throws -> LaneSession {
    guard let session = lock.withLock({ sessions.removeValue(forKey: token) }) else {
      throw BridgeError("no speech session \(token)")
    }
    return session
  }
}

private let registry = SessionRegistry()

private func startSession(_ identifier: String) async throws -> Int {
  let transcriber = makeTranscriber(try await supportedLocale(identifier))
  let audio = try await AudioAdapter.make(for: transcriber)
  let analyzer = SpeechAnalyzer(modules: [transcriber])
  let (stream, continuation) = AsyncStream<AnalyzerInput>.makeStream()
  let session = LaneSession(analyzer: analyzer, continuation: continuation, audio: audio)
  session.collector = Task.detached { await session.collect(transcriber.results) }
  try await analyzer.start(inputSequence: stream)
  return registry.add(session)
}

private func finishSession(_ token: Int) async throws -> [EventPayload] {
  let session = try registry.remove(token)
  session.continuation.finish()
  try await session.analyzer.finalizeAndFinishThroughEndOfInput()
  await session.collector?.value
  return try session.takeEvents()
}

private func assetStatus(_ identifier: String) async throws -> String {
  let transcriber = makeTranscriber(try await supportedLocale(identifier))
  switch await AssetInventory.status(forModules: [transcriber]) {
  case .installed: return "installed"
  case .downloading: return "downloading"
  case .supported: return "supported"
  default: return "unsupported"
  }
}

private func installAsset(_ identifier: String) async throws {
  let locale = try await supportedLocale(identifier)
  _ = try await AssetInventory.reserve(locale: locale)
  let transcriber = makeTranscriber(locale)
  if let request = try await AssetInventory.assetInstallationRequest(supporting: [transcriber]) {
    try await request.downloadAndInstall()
  }
}

@_cdecl("scribe_speech_start")
public func scribeSpeechStart(locale: SRString) -> SRString {
  let identifier = locale.toString()
  return blocking {
    do {
      return Reply(session: try await startSession(identifier))
    } catch {
      return .failure(error)
    }
  }
}

@_cdecl("scribe_speech_push")
public func scribeSpeechPush(session token: Int, samples: SRData) -> SRString {
  do {
    let session = try registry.find(token)
    try session.feed(decodeSamples(samples.toArray()))
    let progress = try session.takeProgress()
    return encoded(Reply(events: progress.events, finalised_through_s: progress.finalisedThrough))
  } catch {
    return encoded(.failure(error))
  }
}

@_cdecl("scribe_speech_finish")
public func scribeSpeechFinish(session token: Int) -> SRString {
  blocking {
    do {
      return Reply(events: try await finishSession(token))
    } catch {
      return .failure(error)
    }
  }
}

@_cdecl("scribe_speech_asset_status")
public func scribeSpeechAssetStatus(locale: SRString) -> SRString {
  let identifier = locale.toString()
  return blocking {
    do {
      return Reply(status: try await assetStatus(identifier))
    } catch {
      return .failure(error)
    }
  }
}

@_cdecl("scribe_speech_install_asset")
public func scribeSpeechInstallAsset(locale: SRString) -> SRString {
  let identifier = locale.toString()
  return blocking {
    do {
      try await installAsset(identifier)
      return Reply()
    } catch {
      return .failure(error)
    }
  }
}
