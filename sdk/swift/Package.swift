// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "HyperMind",
    platforms: [
        .macOS(.v13),
        .iOS(.v16)
    ],
    products: [
        .library(name: "HyperMind", targets: ["HyperMind"])
    ],
    targets: [
        .systemLibrary(name: "CHyperMind", path: "Sources/CHyperMind"),
        .target(name: "HyperMind", dependencies: ["CHyperMind"], path: "Sources/HyperMind"),
        .testTarget(name: "HyperMindTests", dependencies: ["HyperMind"], path: "Tests/HyperMindTests")
    ]
)
