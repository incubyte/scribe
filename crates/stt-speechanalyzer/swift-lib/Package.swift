// swift-tools-version:5.9

import PackageDescription

let package = Package(
  name: "scribe-speech",
  platforms: [.macOS("26.0")],
  products: [
    .library(name: "scribe-speech", type: .static, targets: ["ScribeSpeech"])
  ],
  dependencies: [
    .package(url: "https://github.com/Brendonovich/swift-rs", exact: "1.0.9")
  ],
  targets: [
    .target(
      name: "ScribeSpeech",
      dependencies: [.product(name: "SwiftRs", package: "swift-rs")],
      path: "Sources"
    )
  ]
)
