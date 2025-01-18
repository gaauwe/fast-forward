// swift-tools-version:5.3
import PackageDescription

let package = Package(
    name: "swift-lib",
    platforms: [
        .macOS(.v11),
    ],
    products: [
        .library(
            name: "swift-lib",
            type: .static,
            targets: ["swift-lib"]
        ),
    ],
    dependencies: [
        .package(
            name: "SwiftRs",
            url: "https://github.com/Brendonovich/swift-rs",
            from: "1.0.6"
        ),
    ],
    targets: [
        .target(
            name: "swift-lib",
            dependencies: [
                .product(
                    name: "SwiftRs",
                    package: "SwiftRs"
                )
            ],
            path: "src"
        )
    ]
)
