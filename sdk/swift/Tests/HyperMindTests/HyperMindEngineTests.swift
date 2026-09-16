// These tests build and run only on Apple platforms with a Swift toolchain.
// This repository's Linux CI does not compile or execute them, and no passing
// Swift test run is claimed anywhere in the repository. Run them yourself with
// `swift test` after building the native library, as sdk/swift/README.md
// describes.

import XCTest

@testable import HyperMind

final class HyperMindEngineTests: XCTestCase {
    private let credentials = HyperMindCredentials(
        actor: 7,
        userHex: String(repeating: "11", count: 16),
        kekHex: String(repeating: "22", count: 32),
        projectionMapBytes: 16_777_216
    )

    private func openEngine() throws -> (HyperMindEngine, URL) {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("hypermind-swift-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let engine = try HyperMindEngine(credentials: credentials, stateDirectory: directory.path)
        return (engine, directory)
    }

    private func envelope(_ json: String) throws -> [String: Any] {
        let data = Data(json.utf8)
        let value = try JSONSerialization.jsonObject(with: data)
        return try XCTUnwrap(value as? [String: Any])
    }

    func testRememberThenRecall() async throws {
        let (engine, directory) = try openEngine()
        defer {
            try? engine.close()
            try? FileManager.default.removeItem(at: directory)
        }

        let remembered = try await engine.remember(argumentsJSON: """
            {"conversation":"swift-review","kind":"user",\
            "content":"The release region is eu-central-1.","retention":"durable"}
            """)
        XCTAssertEqual(try envelope(remembered)["ok"] as? Bool, true)

        let recalled = try await engine.recall(argumentsJSON: """
            {"mode":"lexical","query":"release region","limit":8}
            """)
        XCTAssertEqual(try envelope(recalled)["ok"] as? Bool, true)
    }

    func testUnknownVerbReportsTheKernelCode() async throws {
        let (engine, directory) = try openEngine()
        defer {
            try? engine.close()
            try? FileManager.default.removeItem(at: directory)
        }

        do {
            _ = try await engine.call(verb: "not_a_verb", argumentsJSON: "{}")
            XCTFail("an unknown verb must not succeed")
        } catch let error as HyperMindError {
            XCTAssertEqual(error.kernelCode, 47)
            XCTAssertEqual(error.message, "kCapabilityDenied")
        }
    }

    func testCallAfterCloseThrows() async throws {
        let (engine, directory) = try openEngine()
        defer { try? FileManager.default.removeItem(at: directory) }

        try engine.close()
        do {
            _ = try await engine.recall(argumentsJSON: """
                {"mode":"lexical","query":"release region","limit":8}
                """)
            XCTFail("a closed engine must refuse further calls")
        } catch is HyperMindError {
        }
    }

    func testCredentialsRedactSecrets() {
        let rendered = credentials.description
        XCTAssertFalse(rendered.contains(credentials.userHex))
        XCTAssertFalse(rendered.contains(credentials.kekHex))
    }
}
