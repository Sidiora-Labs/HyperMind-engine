import Foundation
import XCTest
@testable import HyperMind

final class ContextTests: XCTestCase {
    private func credentials() -> HyperMindCredentials { HyperMindCredentials(actor: 7, userHex: String(repeating: "11", count: 16), kekHex: String(repeating: "22", count: 32), projectionMapBytes: 16_777_216) }
    private func scope() throws -> ContextScope { try ContextScope(ownerID: "swift-owner", projectID: "swift-project") }
    private func source(_ id: String, _ ordinal: UInt64, _ text: String, ns: Int64 = 1_791_288_000_123_456_789) throws -> ContextSourceMessage {
        try ContextSourceMessage(id: id, ordinal: ordinal, role: .user, parts: [.text(text)], occurredAtNS: ns - 1, recordedAtNS: ns, authority: .userAsserted)
    }
    func testContextCanonicalTimestampsAndDigest() throws {
        XCTAssertEqual(ContextSHA256.hex("abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        let message = try source("source", 0, "Snow 雪 / newline\n")
        let wire = try message.wire()
        XCTAssertEqual(wire["recorded_at_ns"], .string("1791288000123456789"))
        XCTAssertEqual(wire["occurred_at_ns"], .string("1791288000123456788"))
        XCTAssertThrowsError(try ContextSourceMessage(id: "source", ordinal: 0, role: .user, parts: [.text("edited")], recordedAtNS: 1, authority: .userAsserted, sourceDigest: message.sourceDigest))
        XCTAssertEqual(try source("minimum", 1, "min", ns: Int64.min + 1).wire()["occurred_at_ns"], .string("-9223372036854775808"))
        XCTAssertThrowsError(try ContextTokenBudget(contextTokens: 1, reservedOutputTokens: 2))
    }
    func testContextEngineSourceJobsForkImportAndRestart() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("hypermind-context-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let trusted = try scope()
        let engine = try HyperMindEngine(credentials: credentials(), stateDirectory: directory.path, contextScope: trusted)
        let client = try HyperMindContextClient(engine: engine, scope: trusted, sessionID: "review/session?one", actorID: 7, ownership: .hypermind)
        let first = try source("reading/first", 0, "Temperature 20 C. 雪\n")
        let second = try source("reading-second", 1, "Temperature remains 20 C.")
        let original = Data([0xff, 0, 13, 10, 32, 65])
        let receipt1 = try await client.ingest(first, originalBytes: original)
        let receipt2 = try await client.ingest(second, originalBytes: Data("Original second reading\r\n".utf8))
        XCTAssertEqual(receipt1.scope, trusted)
        let budget = try ContextTokenBudget(contextTokens: 1024, reservedOutputTokens: 64)
        let diagnostics = try await client.activate(query: "temperature", budget: budget)
        XCTAssertEqual(diagnostics.version, 1); XCTAssertEqual(diagnostics.report.scope, trusted)
        XCTAssertLessThanOrEqual(diagnostics.report.token_count, budget.available)
        let recovered = try await client.recover(sourceID: first.id); XCTAssertEqual(recovered, original)
        let history = try await client.history()
        XCTAssertEqual(history.messages.first?.recordedAtNS, 1_791_288_000_123_456_789)
        let chunk = try ContextSourceChunk(sources: [first, second], spans: [try XCTUnwrap(receipt1.source_span), try XCTUnwrap(receipt2.source_span)])
        _ = try await client.job(requestID: "enqueue", action: .historianEnqueue(sessionID: "review/session?one", cursor: receipt2.cursor, policyRevision: 1, chunk: chunk, reservation: 100, nowMS: 0))
        let claimValue = try await client.job(requestID: "claim", action: .historianClaim(worker: "swift-worker", nowMS: 0, leaseMS: 60_000))
        let claim = try ContextHistorianClaim(response: claimValue)
        let summary = try ContextHistorianResult(sourceDigest: chunk.digest, tiers: ["Two observations record temperature at 20 C.", "Temperature observations: 20 C.", "Temperature: 20 C.", "20 C"].map { ContextSummaryTier(text: $0, coverage: chunk.spans) })
        _ = try await client.job(requestID: "complete", action: .historianComplete(claim: claim, result: summary, usage: .known(10), nowMS: 0))
        let jobs = try await client.job(requestID: "inspect-jobs", action: .inspect)
        XCTAssertNotEqual(jobs, .null)
        let note = try ContextNote(id: "reading-note", kind: .note, revision: 1, text: "Temperature requires monitoring", expiresAtNS: Int64.max)
        _ = try await client.job(requestID: "create-note", action: .notes(.create(note)))
        let noteRead = try await client.job(requestID: "read-note", action: .readNote(id: note.id, nowNS: 1_791_288_000_123_456_789, facts: [:]))
        XCTAssertEqual(noteRead["note"]["text"].string ?? noteRead["text"].string, note.text)
        let forkReceipt = try await client.fork(childSessionID: "child-session")
        XCTAssertEqual(forkReceipt.session_id, "child-session")
        let imported = try ContextImportBundle(scope: trusted, importID: "swift-import", entries: [ContextImportEntry(sourceID: "imported-reading", kind: "note", payload: .object(["text": .string("Temperature archive")]))])
        let importedReceipt = try await client.importBundle(imported, maxEntries: 1)
        XCTAssertTrue(importedReceipt.complete)
        _ = try await client.relate(.tombstone(id: "retire-second", sourceID: second.id))
        _ = try await client.remember(content: "A pressure review is planned.")
        let recalled = try await client.recall(query: "pressure review"); XCTAssertEqual(recalled["ok"], .bool(true))
        try engine.close()
        let reopened = try HyperMindEngine(credentials: credentials(), stateDirectory: directory.path, contextScope: trusted)
        defer { try? reopened.close() }
        let resumed = try HyperMindContextClient(engine: reopened, scope: trusted, sessionID: "review/session?one", actorID: 7, ownership: .hypermind)
        let restartBytes = try await resumed.recover(sourceID: first.id); XCTAssertEqual(restartBytes, original)
        let replayed = try await resumed.importBundle(imported, maxEntries: 1)
        XCTAssertEqual(replayed.accepted, 1)
        let inspected = try await resumed.inspect(); XCTAssertEqual(inspected.report.scope, trusted)
    }
    func testContextWrongScopeRefusedByActualEngine() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("hypermind-scope-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let engine = try HyperMindEngine(credentials: credentials(), stateDirectory: directory.path, contextScope: scope())
        defer { try? engine.close() }
        let wrong = try ContextScope(ownerID: "other-owner", projectID: "swift-project")
        let client = try HyperMindContextClient(engine: engine, scope: wrong, sessionID: "session", actorID: 7, ownership: .host)
        do { _ = try await client.ingest(source("source", 0, "Cannot widen scope"), originalBytes: Data("original".utf8)); XCTFail("wrong scope accepted") }
        catch is HyperMindError {} catch is ContextClientError {}
    }
}
