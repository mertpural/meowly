// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "UllageCat",
    platforms: [.macOS(.v14)],
    targets: [
        .executableTarget(name: "UllageCat", path: "Sources/UllageCat")
    ]
)
