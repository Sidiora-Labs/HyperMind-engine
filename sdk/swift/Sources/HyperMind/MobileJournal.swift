import CHyperMind
import Foundation

public struct MobileJournalBinding: Codable, Equatable, Sendable {
    public let scope: ContextScope
    public let device_id: String
    public let session_id: String
    public init(scope: ContextScope, deviceID: String, sessionID: String) {
        self.scope = scope; device_id = deviceID; session_id = sessionID
    }
}
public struct MobileMutationIdentity: Codable, Equatable, Sendable {
    public let binding: MobileJournalBinding
    public let operation_id: String
    public let operation_kind: String
    public let payload_digest: String
}
public enum MobileMutationState: String, Codable, Sendable { case pending, unknown, terminal }
public enum MobileReceiptOutcome: String, Codable, Sendable { case succeeded, failed, notApplied = "not_applied", unknown }
public struct MobileMutationReceipt: Decodable, Equatable, Sendable {
    public let identity: MobileMutationIdentity
    public let intent_version: UInt64
    public let cursor: ContextCursor
    public let receipt_digest: String
    public let outcome: MobileReceiptOutcome
    public let observed_session_id: String
}
public struct MobileMutationRecord: Decodable, Equatable, Sendable {
    public let identity: MobileMutationIdentity
    public let state: MobileMutationState
    public let dispatched_session_id: String?
    public let receipt: MobileMutationReceipt?
    public let wake_id: String
}
public struct MobileJournalStatus: Decodable, Equatable, Sendable {
    public let cursor: ContextCursor?
    public let records: [MobileMutationRecord]
    public let session_id: String?
}
public struct MobileJournalConnection: Codable, Sendable {
    public let address: String
    public let server_name: String
    public let ca_der: [UInt8]
    public let certificate_der: [UInt8]
    public let private_key_der: [UInt8]
    public let ticket: String?
    public let grants: [String]
    public let server_certificate_sha256: String
    public init(address: String, serverName: String, caDER: Data, certificateDER: Data,
                privateKeyDER: Data, ticket: String? = nil, grants: [String], serverCertificateSHA256: String) {
        self.address = address; server_name = serverName; ca_der = Array(caDER)
        certificate_der = Array(certificateDER); private_key_der = Array(privateKeyDER)
        self.ticket = ticket; self.grants = grants; server_certificate_sha256 = serverCertificateSHA256
    }
}

/// Owns one native durable journal. TLS authentication and effect receipt
/// validation run in the native encrypted adapter; receipt settlement is not
/// exposed as a caller operation.
public final class MobileJournal: @unchecked Sendable {
    private let handle: OpaquePointer
    private let lock = NSLock()
    private var closed = false
    public let binding: MobileJournalBinding

    public init(path: String, binding: MobileJournalBinding) throws {
        struct Configuration: Encodable { let path: String; let binding: MobileJournalBinding }
        let json = String(decoding: try JSONEncoder().encode(Configuration(path: path, binding: binding)), as: UTF8.self)
        var opened: OpaquePointer?
        let status = json.withCString { hm_mobile_journal_open($0, &opened) }
        guard status == HM_STATUS_OK, let opened else { throw HyperMindError.fromLastError(status) }
        self.handle = opened; self.binding = binding
    }
    deinit { hm_mobile_journal_free(handle) }

    private func call<T: Decodable, A: Encodable>(_ operation: String, _ arguments: A, as: T.Type) throws -> T {
        lock.lock(); defer { lock.unlock() }
        guard !closed else { throw HyperMindError(status: HM_STATUS_HANDLE_CLOSED, kernelCode: nil, message: "journal closed") }
        let json = String(decoding: try JSONEncoder().encode(arguments), as: UTF8.self)
        var result: UnsafeMutablePointer<CChar>?
        let status = operation.withCString { op in json.withCString { args in hm_mobile_journal_call(handle, op, args, &result) } }
        guard status == HM_STATUS_OK, let result else { throw HyperMindError.fromLastError(status) }
        defer { hm_string_free(result) }
        return try JSONDecoder().decode(T.self, from: Data(String(cString: result).utf8))
    }
    private struct Empty: Encodable {}
    public func status() throws -> MobileJournalStatus { try call("status", Empty(), as: MobileJournalStatus.self) }
    public func enqueue(operationID: String, operationKind: String, payloadDigest: String) throws -> MobileMutationRecord {
        struct Request: Encodable { let operation_id: String; let operation_kind: String; let payload_digest: String }
        return try call("enqueue", Request(operation_id: operationID, operation_kind: operationKind, payload_digest: payloadDigest), as: MobileMutationRecord.self)
    }
    public func connect(_ configuration: MobileJournalConnection) throws -> MobileJournalStatus {
        try call("connect", configuration, as: MobileJournalStatus.self)
    }
    public func disconnect() throws -> MobileJournalStatus { try call("disconnect", Empty(), as: MobileJournalStatus.self) }
    /// The native journal records Unknown before network IO. A failed call must
    /// be reconciled over a fresh authenticated connection; it must not retry.
    public func dispatch(payload: Data) throws -> MobileMutationRecord {
        struct Request: Encodable { let payload: [UInt8] }
        return try call("dispatch", Request(payload: Array(payload)), as: MobileMutationRecord.self)
    }
    public func reconcile() throws -> MobileMutationRecord { try call("reconcile", Empty(), as: MobileMutationRecord.self) }
    public func close() throws {
        lock.lock(); defer { lock.unlock() }
        if closed { return }
        var result: UnsafeMutablePointer<CChar>?
        let status = "close".withCString { op in "{}".withCString { args in hm_mobile_journal_call(handle, op, args, &result) } }
        if let result { hm_string_free(result) }
        guard status == HM_STATUS_OK else { throw HyperMindError.fromLastError(status) }
        closed = true
    }
}
