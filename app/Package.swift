// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "OreoKey",
    platforms: [.macOS(.v13)],
    targets: [
        .systemLibrary(name: "COreoKey", path: "Sources/COreoKey"),
        .executableTarget(
            name: "OreoKey",
            dependencies: ["COreoKey"],
            path: "Sources/OreoKey",
            linkerSettings: [
                .linkedLibrary("oreokey_core"),
                .linkedFramework("AppKit"),
                .linkedFramework("Carbon"),
                .linkedFramework("ApplicationServices"),
                .linkedFramework("ServiceManagement"),
            ]
        ),
    ]
)
