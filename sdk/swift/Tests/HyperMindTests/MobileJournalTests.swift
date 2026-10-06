import Foundation
import XCTest
@testable import HyperMind

final class MobileJournalTests: XCTestCase {
    func testNativeFileRestartAndIdentityFence() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let scope = try ContextScope(ownerID: "mobile-owner", projectID: "mobile-project")
        let binding = MobileJournalBinding(scope: scope, deviceID: "reviewed-device", sessionID: "app-session")
        let path = directory.appendingPathComponent("journal.json").path
        let first = try MobileJournal(path: path, binding: binding)
        let record = try first.enqueue(operationID: "operation-1", operationKind: "append_file", payloadDigest: String(repeating: "a", count: 64))
        XCTAssertEqual(record.state, .pending)
        XCTAssertEqual(record.identity.binding, binding)
        XCTAssertEqual(record.wake_id.count, 64)
        XCTAssertThrowsError(try MobileJournal(path: path, binding: binding))
        XCTAssertThrowsError(try first.enqueue(operationID: "operation-2", operationKind: "append_file", payloadDigest: String(repeating: "b", count: 64)))
        XCTAssertThrowsError(try first.dispatch(payload: Data("actual payload".utf8)))
        XCTAssertEqual(try first.status().records.first?.state, .pending)
        try first.close()
        let reopened = try MobileJournal(path: path, binding: binding)
        XCTAssertEqual(try reopened.status().records.first, record)
        XCTAssertNil(try reopened.status().cursor)
        XCTAssertThrowsError(try reopened.reconcile())
        try reopened.close()
        let other = MobileJournalBinding(scope: scope, deviceID: "other-device", sessionID: "app-session")
        XCTAssertThrowsError(try MobileJournal(path: path, binding: other))
        let bytes = try Data(contentsOf: URL(fileURLWithPath: path))
        XCTAssertFalse(String(decoding: bytes, as: UTF8.self).contains("actual payload"))
    }
}

extension MobileJournalTests {
    func testEncryptedUnknownRestartAndNativeReceipt() throws {
        guard let manifestPath = ProcessInfo.processInfo.environment["HM_SWIFT_MOBILE_CONFIG"] else {
            throw XCTSkip("requires the actual enrolled TLS/native-effect peer fixture")
        }
        struct Manifest: Decodable {
            let binding: MobileJournalBinding
            let connection: MobileJournalConnection
            let fixture: ContextJSON
            let peer_executable: String
            let payload1_digest: String
            let payload2_digest: String
        }
        let manifest = try JSONDecoder().decode(Manifest.self, from: Data(contentsOf: URL(fileURLWithPath: manifestPath)))
        let directory = URL(fileURLWithPath: manifestPath).deletingLastPathComponent()
        let journalPath = directory.appendingPathComponent("swift-journal.json").path
        var fixture = manifest.fixture
        func string(_ key: String) throws -> String {
            guard let value = fixture[key].string else { throw ContextClientError.invalidInput("fixture " + key) }; return value
        }
        func payload(_ key: String) throws -> Data {
            guard let bytes = fixture[key].array else { throw ContextClientError.invalidInput("fixture payload") }
            return Data(try bytes.map { value in
                guard let n = value.integer, n >= 0, n <= 255 else { throw ContextClientError.invalidInput("fixture byte") }; return UInt8(n)
            })
        }
        func peer() throws -> Process {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: manifest.peer_executable)
            process.arguments = ["--exact", "native_effect_peer", "--ignored", "--nocapture"]
            var environment = ProcessInfo.processInfo.environment
            environment["HM_MOBILE_DELIVERY_FIXTURE"] = try fixture.canonical()
            environment["HM_MOBILE_DELIVERY_NEGATIVE_COUNT"] = "0"
            process.environment = environment
            process.standardOutput = FileHandle.nullDevice; process.standardError = FileHandle.nullDevice
            try process.run()
            return process
        }
        func waitFile(_ path: String) throws {
            let deadline = Date().addingTimeInterval(15)
            while !FileManager.default.fileExists(atPath: path) {
                if Date() >= deadline { throw ContextClientError.invalidResponse("peer readiness timeout") }
                Thread.sleep(forTimeInterval: 0.01)
            }
        }
        let payload1 = try payload("payload1"), payload2 = try payload("payload2")
        let first = try MobileJournal(path: journalPath, binding: manifest.binding)
        let digest1 = manifest.payload1_digest
        _ = try first.enqueue(operationID: "operation-1", operationKind: "append_file", payloadDigest: digest1)
        let crashed = try peer()
        defer { if crashed.isRunning { crashed.terminate() }; crashed.waitUntilExit() }
        try waitFile(try string("ready"))
        let original = try first.connect(manifest.connection).session_id
        XCTAssertNotNil(original)
        let stage = try string("stage")
        let killer = Process()
        killer.executableURL = URL(fileURLWithPath: "/bin/sh")
        killer.arguments = ["-c", "while [ ! -f \"$1\" ]; do sleep 0.01; done; kill -KILL \"$2\"", "journal-killer", stage, String(crashed.processIdentifier)]
        try killer.run()
        XCTAssertThrowsError(try first.dispatch(payload: payload1))
        killer.waitUntilExit(); crashed.waitUntilExit()
        let unknown = try first.status().records[0]
        XCTAssertEqual(unknown.state, .unknown)
        XCTAssertEqual(unknown.dispatched_session_id, original)
        XCTAssertThrowsError(try first.enqueue(operationID: "operation-2", operationKind: "append_file", payloadDigest: String(repeating: "b", count: 64)))
        try first.close()
        let reopened = try MobileJournal(path: journalPath, binding: manifest.binding)
        XCTAssertEqual(try reopened.status().records[0], unknown)
        guard case .object(var fields) = fixture else { throw ContextClientError.invalidInput("fixture") }
        fields["role"] = .string("resume")
        fields["ready"] = .string(directory.appendingPathComponent("swift-resume-ready").path)
        fixture = .object(fields)
        let resumed = try peer()
        defer { if resumed.isRunning { resumed.terminate() }; resumed.waitUntilExit() }
        try waitFile(try string("ready"))
        let c = manifest.connection
        let reauthentication = MobileJournalConnection(address: c.address, serverName: c.server_name, caDER: Data(c.ca_der), certificateDER: Data(c.certificate_der), privateKeyDER: Data(c.private_key_der), grants: c.grants, serverCertificateSHA256: c.server_certificate_sha256)
        let fresh = try reopened.connect(reauthentication).session_id
        XCTAssertNotEqual(fresh, original)
        let unresolved = try reopened.reconcile()
        XCTAssertEqual(unresolved.state, .unknown)
        XCTAssertEqual(unresolved.receipt?.cursor.sequence, 3)
        XCTAssertThrowsError(try reopened.dispatch(payload: payload1))
        let terminal = try reopened.reconcile()
        XCTAssertEqual(terminal.state, .terminal)
        XCTAssertEqual(terminal.receipt?.outcome, .succeeded)
        XCTAssertEqual(terminal.receipt?.cursor.sequence, 5)
        XCTAssertThrowsError(try reopened.enqueue(operationID: "operation-1", operationKind: "append_file", payloadDigest: digest1))
        _ = try reopened.enqueue(operationID: "operation-2", operationKind: "append_file", payloadDigest: manifest.payload2_digest)
        let second = try reopened.dispatch(payload: payload2)
        XCTAssertEqual(second.state, .terminal)
        XCTAssertEqual(second.receipt?.cursor.sequence, 8)
        try reopened.close()
        resumed.waitUntilExit(); XCTAssertEqual(resumed.terminationStatus, 0)
        var expected = payload1; expected.append(payload2)
        XCTAssertEqual(try Data(contentsOf: URL(fileURLWithPath: try string("output"))), expected)
        let final = try MobileJournal(path: journalPath, binding: manifest.binding)
        XCTAssertEqual(try final.status().cursor?.sequence, 8)
        XCTAssertEqual(try final.status().records.count, 2)
        try final.close()
    }
}
