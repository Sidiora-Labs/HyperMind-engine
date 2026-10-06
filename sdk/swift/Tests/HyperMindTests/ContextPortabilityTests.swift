import Foundation
import XCTest
@testable import HyperMind

final class ContextPortabilityTests: XCTestCase {
    func testContextNativeJSONLRestoreExactMetadataTamperAndRestart() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("hypermind-portability-\(UUID().uuidString)")
        let firstRoot = directory.appendingPathComponent("first")
        let secondRoot = directory.appendingPathComponent("second")
        try FileManager.default.createDirectory(at: firstRoot, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: secondRoot, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let credentials = HyperMindCredentials(actor: 7, userHex: String(repeating: "11", count: 16), kekHex: String(repeating: "22", count: 32), projectionMapBytes: 16_777_216)
        let scope = try ContextScope(ownerID: "portability-owner", projectID: "portability-project")
        let first = try HyperMindEngine(credentials: credentials, stateDirectory: firstRoot.path, contextScope: scope)
        defer { try? first.close() }
        let firstClient = try HyperMindContextClient(engine: first, scope: scope, sessionID: "session", actorID: 7, ownership: .hypermind)
        let scopeJSON = try encoded(scope).canonical()
        let create = """
        {"conversation":"session","content":"","kind":"user","context":{"operation":"memory","request":{"version":1,"scope":\(scopeJSON),"request_id":"create-record","command":{"kind":"create","record":{"id":"reading","kind":"fact","category":"measurements","status":"active","revision":1,"revision_digest":"","content":"Temperature 20 C","authority":"user_asserted","confidence":1000000,"importance":500000,"occurred_at_ns":null,"recorded_at_ns":"1791288000123456789","expires_at_ns":null,"pinned":false,"provenance":[],"lineage":[],"contradictions":[],"last_lsn":0,"metadata":{"large":18446744073709551615,"tiny":1e-7}}}}}}
        """
        let created = try await first.remember(argumentsJSON: create)
        XCTAssertEqual(try JSONDecoder().decode(ContextJSON.self, from: Data(created.utf8))["ok"], .bool(true))
        let artifact = try await firstClient.exportNativeMemory()
        XCTAssertEqual(artifact.scope, scope); XCTAssertEqual(artifact.restoreMaximumBytes, 524_288)
        let artifactText = try XCTUnwrap(String(data: artifact.jsonl, encoding: .utf8))
        XCTAssertTrue(artifactText.contains("18446744073709551615"))
        XCTAssertTrue(artifactText.contains("1791288000123456789"))
        var damaged = artifact.jsonl; damaged[damaged.startIndex] ^= 1
        XCTAssertThrowsError(try ContextMemoryArtifact(scope: scope, cursor: artifact.cursor, jsonl: damaged, artifactDigest: artifact.artifactDigest, exportDigest: artifact.exportDigest, restoreMaximumBytes: artifact.restoreMaximumBytes))
        var large = artifact.jsonl; large.append(Data(repeating: 32, count: Int(artifact.restoreMaximumBytes))); large.append(10)
        let oversized = try ContextMemoryArtifact(scope: scope, cursor: artifact.cursor, jsonl: large, artifactDigest: ContextSHA256.hex(try XCTUnwrap(String(data: large, encoding: .utf8))), exportDigest: artifact.exportDigest, restoreMaximumBytes: artifact.restoreMaximumBytes)
        XCTAssertThrowsError(try oversized.validateRestoration(scope: scope)) { error in
            guard case ContextPortabilityError.artifactTooLarge = error else { return XCTFail("unexpected oversize error: \(error)") }
        }
        let second = try HyperMindEngine(credentials: credentials, stateDirectory: secondRoot.path, contextScope: scope)
        let secondClient = try HyperMindContextClient(engine: second, scope: scope, sessionID: "session", actorID: 7, ownership: .hypermind)
        let restored = try await secondClient.restoreNativeMemory(artifact, requestID: "restore-record")
        XCTAssertEqual(restored.scope, scope)
        let replayed = try await secondClient.restoreNativeMemory(artifact, requestID: "restore-record")
        XCTAssertTrue(replayed.replayed)
        let view = try await second.inspect(argumentsJSON: "{\"uri\":\"hm://7/context-memory\"}")
        XCTAssertTrue(view.contains("18446744073709551615")); XCTAssertTrue(view.contains("1791288000123456789"))
        let tampered: ContextJSON = .object(["conversation": .string("session"), "content": .string(""), "kind": .string("user"), "context": .object(["operation": .string("memory"), "request": .object(["version": .integer(1), "scope": try encoded(scope), "request_id": .string("tampered-restore"), "command": .object(["kind": .string("restore_jsonl"), "jsonl": .array(damaged.map { .integer(Int64($0)) }), "artifact_digest": .string(artifact.artifactDigest)])])])])
        let refused = try await second.remember(argumentsJSON: tampered.canonical())
        XCTAssertEqual(try JSONDecoder().decode(ContextJSON.self, from: Data(refused.utf8))["ok"], .bool(false))
        try second.close()
        let restarted = try HyperMindEngine(credentials: credentials, stateDirectory: secondRoot.path, contextScope: scope)
        defer { try? restarted.close() }
        let afterRestart = try await restarted.inspect(argumentsJSON: "{\"uri\":\"hm://7/context-memory\"}")
        XCTAssertTrue(afterRestart.contains("18446744073709551615")); XCTAssertTrue(afterRestart.contains("1791288000123456789"))
    }
}
