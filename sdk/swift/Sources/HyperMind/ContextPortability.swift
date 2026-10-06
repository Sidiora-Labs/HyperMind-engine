import Foundation

public enum ContextPortabilityError: Error, Equatable, Sendable, CustomStringConvertible {
    case invalidArtifact
    case artifactTooLarge(byteCount: UInt64, restoreMaximumBytes: UInt64)
    public var description: String {
        switch self {
        case .invalidArtifact: return "Invalid memory artifact or digest."
        case .artifactTooLarge(let count, let maximum): return "Memory artifact has \(count) bytes; restoration supports at most \(maximum) bytes. Streaming restoration is unavailable."
        }
    }
}

public struct ContextMemoryArtifact: Decodable, Sendable {
    public let version: UInt32
    public let scope: ContextScope
    public let cursor: UInt64
    public let mediaType: String
    public let jsonl: Data
    public let artifactDigest: String
    public let exportDigest: String
    public let restoreMaximumBytes: UInt64
    public var byteCount: UInt64 { UInt64(jsonl.count) }
    private enum CodingKeys: String, CodingKey { case version, scope, cursor, mediaType = "media_type", jsonl = "bytes", byteCount = "byte_count", artifactDigest = "artifact_digest", exportDigest = "export_digest", restoreMaximumBytes = "restore_max_bytes" }
    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        version = try c.decode(UInt32.self, forKey: .version)
        scope = try c.decode(ContextScope.self, forKey: .scope)
        cursor = try c.decode(UInt64.self, forKey: .cursor)
        mediaType = try c.decode(String.self, forKey: .mediaType)
        jsonl = Data(try c.decode([UInt8].self, forKey: .jsonl))
        artifactDigest = try c.decode(String.self, forKey: .artifactDigest)
        exportDigest = try c.decode(String.self, forKey: .exportDigest)
        restoreMaximumBytes = try c.decode(UInt64.self, forKey: .restoreMaximumBytes)
        guard try c.decode(UInt64.self, forKey: .byteCount) == byteCount else { throw ContextPortabilityError.invalidArtifact }
        try validate()
    }
    public init(scope: ContextScope, cursor: UInt64, jsonl: Data, artifactDigest: String, exportDigest: String, restoreMaximumBytes: UInt64) throws {
        version = 1; self.scope = scope; self.cursor = cursor; self.jsonl = jsonl; self.artifactDigest = artifactDigest; self.exportDigest = exportDigest; self.restoreMaximumBytes = restoreMaximumBytes; mediaType = "application/x-ndjson"
        try validate()
    }
    private func validate() throws {
        guard version == 1, mediaType == "application/x-ndjson", !jsonl.isEmpty, jsonl.last == 10, jsonl.count <= 64 * 1024 * 1024,
              restoreMaximumBytes > 0, restoreMaximumBytes <= 64 * 1024 * 1024,
              artifactDigest.utf8.count == 64, exportDigest.utf8.count == 64,
              artifactDigest.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }),
              exportDigest.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }),
              let text = String(data: jsonl, encoding: .utf8), Data(text.utf8) == jsonl,
              ContextSHA256.hex(text) == artifactDigest else { throw ContextPortabilityError.invalidArtifact }
    }
    public func validateRestoration(scope: ContextScope) throws {
        try validate()
        guard self.scope == scope else { throw ContextClientError.scopeMismatch }
        guard byteCount <= restoreMaximumBytes else { throw ContextPortabilityError.artifactTooLarge(byteCount: byteCount, restoreMaximumBytes: restoreMaximumBytes) }
    }
}
public struct ContextMemoryRestoreReceipt: Decodable, Sendable {
    public let version: UInt32
    public let scope: ContextScope
    public let cursor: UInt64
    public let last_lsn: UInt64
    public let replayed: Bool
}
private struct ContextPortabilityEnvelope: Decodable {
    let ok: Bool
    let items: [ContextMemoryArtifact]
}
private struct ContextRestoreEnvelope: Decodable {
    let ok: Bool
    let items: [ContextMemoryRestoreReceipt]
}
extension HyperMindContextClient {
    public func exportNativeMemory() async throws -> ContextMemoryArtifact {
        let arguments: ContextJSON = .object(["uri": .string("hm://\(actorID)/context-memory-export")])
        let response = try await engine.inspect(argumentsJSON: arguments.canonical())
        let bytes = Data(response.utf8)
        let raw = try JSONDecoder().decode(ContextJSON.self, from: bytes)
        guard raw["ok"] == .bool(true) else { throw ContextOperationError(verb: "inspect", arguments: arguments, envelope: raw) }
        let envelope = try JSONDecoder().decode(ContextPortabilityEnvelope.self, from: bytes)
        guard envelope.ok, let artifact = envelope.items.first, artifact.scope == scope else { throw ContextClientError.scopeMismatch }
        return artifact
    }
    public func restoreNativeMemory(_ artifact: ContextMemoryArtifact, requestID: String) async throws -> ContextMemoryRestoreReceipt {
        try identifier(requestID); try artifact.validateRestoration(scope: scope)
        let command: ContextJSON = .object(["kind": .string("restore_jsonl"), "jsonl": .array(artifact.jsonl.map { .integer(Int64($0)) }), "artifact_digest": .string(artifact.artifactDigest)])
        let request: ContextJSON = .object(["version": .integer(1), "scope": try encoded(scope), "request_id": .string(requestID), "command": command])
        let arguments: ContextJSON = .object(["conversation": .string(conversation), "content": .string(""), "kind": .string("user"), "context": .object(["operation": .string("memory"), "request": request])])
        let response = try await engine.remember(argumentsJSON: arguments.canonical())
        let bytes = Data(response.utf8)
        let raw = try JSONDecoder().decode(ContextJSON.self, from: bytes)
        guard raw["ok"] == .bool(true) else { throw ContextOperationError(verb: "remember", arguments: arguments, envelope: raw) }
        let envelope = try JSONDecoder().decode(ContextRestoreEnvelope.self, from: bytes)
        guard envelope.ok, let receipt = envelope.items.first, receipt.version == 1, receipt.scope == scope else { throw ContextClientError.invalidResponse("memory restoration identity") }
        return receipt
    }
}
