import Foundation

public enum ContextClientError: Error, Equatable {
    case invalidInput(String)
    case invalidResponse(String)
    case operationFailed(effectState: String)
    case scopeMismatch
    case staleCursor
    case unsupportedVersion
}

public enum ContextJSON: Codable, Equatable, Sendable {
    case null, bool(Bool), integer(Int64), decimal(Double), string(String), array([ContextJSON]), object([String: ContextJSON])
    public init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() { self = .null }
        else if let v = try? c.decode(Bool.self) { self = .bool(v) }
        else if let v = try? c.decode(Int64.self) { self = .integer(v) }
        else if let v = try? c.decode(Double.self) { self = .decimal(v) }
        else if let v = try? c.decode(String.self) { self = .string(v) }
        else if let v = try? c.decode([ContextJSON].self) { self = .array(v) }
        else { self = .object(try c.decode([String: ContextJSON].self)) }
    }
    public func encode(to encoder: Encoder) throws {
        var c = encoder.singleValueContainer()
        switch self {
        case .null: try c.encodeNil()
        case .bool(let v): try c.encode(v)
        case .integer(let v): try c.encode(v)
        case .decimal(let v): try c.encode(v)
        case .string(let v): try c.encode(v)
        case .array(let v): try c.encode(v)
        case .object(let v): try c.encode(v)
        }
    }
    public subscript(key: String) -> ContextJSON { if case .object(let v) = self { return v[key] ?? .null }; return .null }
    public var string: String? { if case .string(let v) = self { return v }; return nil }
    public var integer: Int64? { if case .integer(let v) = self { return v }; return nil }
    public var array: [ContextJSON]? { if case .array(let v) = self { return v }; return nil }
    func canonical() throws -> String {
        switch self {
        case .null: return "null"
        case .bool(let v): return v ? "true" : "false"
        case .integer(let v): return String(v)
        case .decimal(let v): guard v.isFinite else { throw ContextClientError.invalidInput("nonfinite number") }; return String(v)
        case .string(let v): return quote(v)
        case .array(let v): return "[" + (try v.map { try $0.canonical() }).joined(separator: ",") + "]"
        case .object(let v): return "{" + (try v.keys.sorted().map { quote($0) + ":" + (try v[$0]!.canonical()) }).joined(separator: ",") + "}"
        }
    }
}
func quote(_ text: String) -> String {
    var output = "\""
    for scalar in text.unicodeScalars {
        switch scalar.value {
        case 34: output += "\\\""
        case 92: output += "\\\\"
        case 8: output += "\\b"
        case 9: output += "\\t"
        case 10: output += "\\n"
        case 12: output += "\\f"
        case 13: output += "\\r"
        case 0..<32: output += String(format: "\\u%04x", scalar.value)
        default: output.unicodeScalars.append(scalar)
        }
    }
    return output + "\""
}
func identifier(_ value: String) throws {
    guard !value.isEmpty, value.utf8.count <= 256, value.utf8.allSatisfy({ (33...126).contains($0) }) else { throw ContextClientError.invalidInput("identifier") }
}
func safeInteger(_ value: UInt64, minimum: UInt64 = 0) throws {
    guard value >= minimum, value <= 9_007_199_254_740_991 else { throw ContextClientError.invalidInput("integer range") }
}
func encoded<T: Encodable>(_ value: T) throws -> ContextJSON { try JSONDecoder().decode(ContextJSON.self, from: JSONEncoder().encode(value)) }

public struct ContextScope: Codable, Equatable, Sendable {
    public let owner_id: String
    public let project_id: String
    public let workspace_id: String?
    public init(ownerID: String, projectID: String, workspaceID: String? = nil) throws {
        try identifier(ownerID); try identifier(projectID); if let workspaceID { try identifier(workspaceID) }
        owner_id = ownerID; project_id = projectID; workspace_id = workspaceID
    }
    public func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(owner_id, forKey: .owner_id); try c.encode(project_id, forKey: .project_id); try c.encode(workspace_id, forKey: .workspace_id)
    }
}
public struct ContextCursor: Codable, Equatable, Sendable {
    public let epoch: UInt64
    public let sequence: UInt64
    public init(epoch: UInt64 = 1, sequence: UInt64 = 0) throws { try safeInteger(epoch, minimum: 1); try safeInteger(sequence); self.epoch = epoch; self.sequence = sequence }
}
public struct ContextTokenBudget: Codable, Equatable, Sendable {
    public let context_tokens: UInt64
    public let reserved_output_tokens: UInt64
    public let required_tokens: UInt64
    public var available: UInt64 { context_tokens - reserved_output_tokens - required_tokens }
    public init(contextTokens: UInt64, reservedOutputTokens: UInt64, requiredTokens: UInt64 = 0) throws {
        try safeInteger(contextTokens, minimum: 1); try safeInteger(reservedOutputTokens, minimum: 1); try safeInteger(requiredTokens)
        guard reservedOutputTokens <= contextTokens, requiredTokens <= contextTokens - reservedOutputTokens else { throw ContextClientError.invalidInput("capacity") }
        context_tokens = contextTokens; reserved_output_tokens = reservedOutputTokens; required_tokens = requiredTokens
    }
}
public enum ContextAuthority: String, Codable, Sendable { case userAsserted = "user_asserted", externalObserved = "external_observed", toolObserved = "tool_observed", runtimeFact = "runtime_fact", assistantGenerated = "assistant_generated", derivedInference = "derived_inference" }
public enum ContextRole: String, Codable, Sendable { case user, assistant, tool }
public enum ContextOwnership: String, Codable, Sendable { case host, hypermind }
public enum ContextPart: Equatable, Sendable {
    case text(String), toolCall(callID: String, name: String, arguments: String), toolResult(callID: String, content: String, failed: Bool), opaque(mediaType: String, reference: String, digest: String)
    var canonical: String {
        switch self {
        case .text(let v): return "{\"kind\":\"text\",\"text\":\(quote(v))}"
        case .toolCall(let id, let name, let args): return "{\"kind\":\"tool_call\",\"call_id\":\(quote(id)),\"name\":\(quote(name)),\"arguments\":\(quote(args))}"
        case .toolResult(let id, let content, let failed): return "{\"kind\":\"tool_result\",\"call_id\":\(quote(id)),\"content\":\(quote(content)),\"failed\":\(failed)}"
        case .opaque(let media, let reference, let digest): return "{\"kind\":\"opaque\",\"media_type\":\(quote(media)),\"reference\":\(quote(reference)),\"digest\":\(quote(digest))}"
        }
    }
}
public struct ContextSourceMessage: Sendable {
    public let id: String
    public let ordinal: UInt64
    public let role: ContextRole
    public let parts: [ContextPart]
    public let occurredAtNS: Int64?
    public let recordedAtNS: Int64
    public let authority: ContextAuthority
    public let sourceDigest: String
    public init(id: String, ordinal: UInt64, role: ContextRole, parts: [ContextPart], occurredAtNS: Int64? = nil, recordedAtNS: Int64, authority: ContextAuthority, sourceDigest: String? = nil) throws {
        try identifier(id); try safeInteger(ordinal)
        guard !parts.isEmpty, parts.count <= 4096 else { throw ContextClientError.invalidInput("source parts") }
        self.id = id; self.ordinal = ordinal; self.role = role; self.parts = parts; self.occurredAtNS = occurredAtNS; self.recordedAtNS = recordedAtNS; self.authority = authority
        let digest = ContextSHA256.hex(Self.canonical(id, ordinal, role, parts, occurredAtNS, recordedAtNS, authority, ""))
        if let sourceDigest, sourceDigest != digest { throw ContextClientError.invalidInput("source identity changed") }; self.sourceDigest = digest
    }
    static func canonical(_ id: String, _ ordinal: UInt64, _ role: ContextRole, _ parts: [ContextPart], _ occurred: Int64?, _ recorded: Int64, _ authority: ContextAuthority, _ digest: String) -> String {
        "{\"id\":\(quote(id)),\"ordinal\":\(ordinal),\"role\":\(quote(role.rawValue)),\"parts\":[\(parts.map(\.canonical).joined(separator: ","))],\"occurred_at_ns\":\(occurred.map { quote(String($0)) } ?? "null"),\"recorded_at_ns\":\(quote(String(recorded))),\"authority\":\(quote(authority.rawValue)),\"source_digest\":\(quote(digest))}"
    }
    public func wire() throws -> ContextJSON { try JSONDecoder().decode(ContextJSON.self, from: Data(Self.canonical(id, ordinal, role, parts, occurredAtNS, recordedAtNS, authority, sourceDigest).utf8)) }
}
public struct ContextSourceSpan: Codable, Equatable, Sendable {
    public let source_id: String; public let source_digest: String; public let byte_start: UInt64; public let byte_end: UInt64
    public init(sourceID: String, sourceDigest: String, byteStart: UInt64, byteEnd: UInt64) throws {
        try identifier(sourceID); try safeInteger(byteStart); try safeInteger(byteEnd)
        guard byteEnd >= byteStart, sourceDigest.count == 64, sourceDigest.utf8.allSatisfy({ (48...57).contains($0) || (97...102).contains($0) }) else { throw ContextClientError.invalidInput("source span") }
        source_id=sourceID; source_digest=sourceDigest; byte_start=byteStart; byte_end=byteEnd
    }
}
public struct ContextHistoryReceipt: Codable, Sendable { public let version: UInt32; public let scope: ContextScope; public let session_id: String; public let cursor: ContextCursor; public let last_lsn: UInt64; public let replayed: Bool; public let source_span: ContextSourceSpan? }
public enum ContextRelation: Sendable {
    case edit(id: String, originalID: String, replacementID: String), regenerate(id: String, originalID: String, replacementID: String), tombstone(id: String, sourceID: String)
    func wire() throws -> ContextJSON {
        switch self {
        case .edit(let id, let original, let replacement), .regenerate(let id, let original, let replacement):
            try identifier(id); try identifier(original); try identifier(replacement)
            let kind: String; if case .edit = self { kind = "edit" } else { kind = "regenerate" }
            return .object(["kind": .string(kind), "id": .string(id), "original_id": .string(original), "replacement_id": .string(replacement)])
        case .tombstone(let id, let source): try identifier(id); try identifier(source); return .object(["kind": .string("tombstone"), "id": .string(id), "source_id": .string(source)])
        }
    }
}
public struct ContextBlock: Codable, Sendable { public let id: String; public let text: String; public let authority: ContextAuthority; public let provenance: [ContextSourceSpan]; public let tokens: UInt64; public let required: Bool }
public struct ContextOmission: Codable, Sendable { public let id: String; public let reason: String }
public struct ContextReport: Codable, Sendable { public let version: UInt32; public let scope: ContextScope; public let session_id: String; public let cursor: ContextCursor; public let generation: UInt64; public let blocks: [ContextBlock]; public let included: [String]; public let omitted: [ContextOmission]; public let gaps: [String]; public let token_count: UInt64; public let digest: String }
public struct ContextDiagnostics: Decodable, Sendable { public let version: UInt32; public let session_id: String; public let report: ContextReport; public let history: ContextJSON; public let coverage: ContextJSON; public let cache: ContextJSON; public let jobs: ContextJSON; public let provenance: ContextJSON; public let migrations: ContextJSON; public let gaps: [String] }

extension ContextPart: Codable {
    public init(from decoder: Decoder) throws {
        let value = try ContextJSON(from: decoder)
        func text(_ key: String) throws -> String { guard let text = value[key].string else { throw ContextClientError.invalidResponse("part " + key) }; return text }
        switch try text("kind") {
        case "text": self = .text(try text("text"))
        case "tool_call": self = .toolCall(callID: try text("call_id"), name: try text("name"), arguments: try text("arguments"))
        case "tool_result": guard case .bool(let failed) = value["failed"] else { throw ContextClientError.invalidResponse("part failed") }; self = .toolResult(callID: try text("call_id"), content: try text("content"), failed: failed)
        case "opaque": self = .opaque(mediaType: try text("media_type"), reference: try text("reference"), digest: try text("digest"))
        default: throw ContextClientError.invalidResponse("part kind")
        }
    }
    public func encode(to encoder: Encoder) throws { try JSONDecoder().decode(ContextJSON.self, from: Data(canonical.utf8)).encode(to: encoder) }
}
extension ContextSourceMessage: Codable {
    public init(from decoder: Decoder) throws {
        let value = try ContextJSON(from: decoder)
        func text(_ key: String) throws -> String { guard let text = value[key].string else { throw ContextClientError.invalidResponse("source " + key) }; return text }
        func timestamp(_ key: String) throws -> Int64 { let text = try text(key); guard let number = Int64(text), String(number) == text else { throw ContextClientError.invalidResponse("canonical timestamp") }; return number }
        guard let ordinal = value["ordinal"].integer, ordinal >= 0, let role = ContextRole(rawValue: try text("role")), let authority = ContextAuthority(rawValue: try text("authority")) else { throw ContextClientError.invalidResponse("source role or authority") }
        let parts = try JSONDecoder().decode([ContextPart].self, from: JSONEncoder().encode(value["parts"]))
        try self.init(id: text("id"), ordinal: UInt64(ordinal), role: role, parts: parts, occurredAtNS: value["occurred_at_ns"] == .null ? nil : timestamp("occurred_at_ns"), recordedAtNS: timestamp("recorded_at_ns"), authority: authority, sourceDigest: text("source_digest"))
    }
    public func encode(to encoder: Encoder) throws { try wire().encode(to: encoder) }
}
public struct ContextHistoryInspection: Decodable, Sendable {
    public let version: UInt32; public let scope: ContextScope; public let session_id: String; public let cursor: ContextCursor
    public let messages: [ContextSourceMessage]; public let relations: [ContextJSON]; public let parent: ContextJSON; public let spans: [ContextSourceSpan]; public let unsupported_parts: [ContextJSON]
}
