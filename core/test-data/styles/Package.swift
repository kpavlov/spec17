// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "StyleFixtures",
    targets: [
        .target(name: "StyleFixtures", path: "src"),
        .testTarget(name: "StyleFixturesTests", dependencies: ["StyleFixtures"], path: "tests"),
    ]
)
