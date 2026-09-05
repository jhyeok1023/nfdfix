// swift-tools-version: 6.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "nfdfix",
    targets: [
        .target(name: "NFDFixCore"),
        .executableTarget(
            name: "nfdfix",
            dependencies: ["NFDFixCore"]
        ),
        .testTarget(
            name: "NFDFixCoreTests",
            dependencies: ["NFDFixCore"]
        ),
    ],
    swiftLanguageModes: [.v6]
)
