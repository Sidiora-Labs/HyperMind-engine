import Foundation

public actor HyperMindContextClient {
    public let engine: HyperMindEngine
    public let scope: ContextScope
    public let sessionID: String
    public let conversation: String
    public let actorID: UInt16
    public let ownership: ContextOwnership
    public private(set) var cursor: ContextCursor
    public private(set) var negotiatedVersion: UInt32?
    public init(engine: HyperMindEngine, scope: ContextScope, sessionID: String, actorID: UInt16, ownership: ContextOwnership, conversation: String? = nil) throws {
        try identifier(sessionID); try identifier(conversation ?? sessionID)
        guard actorID > 0 else { throw ContextClientError.invalidInput("actor") }
        self.engine = engine; self.scope = scope; self.sessionID = sessionID; self.actorID = actorID; self.ownership = ownership; self.conversation = conversation ?? sessionID; self.cursor = try ContextCursor()
    }
    private func call(_ verb: String, _ arguments: [String: ContextJSON]) async throws -> ContextJSON {
        let response = try await engine.call(verb: verb, argumentsJSON: ContextJSON.object(arguments).canonical())
        let value = try JSONDecoder().decode(ContextJSON.self, from: Data(response.utf8))
        guard value["ok"] == .bool(true) else { throw ContextClientError.operationFailed(effectState: value["effect_state"].string ?? "unknown") }
        guard let first = value["items"].array?.first else { throw ContextClientError.invalidResponse("missing item") }
        return first
    }
    private func decode<T: Decodable>(_ value: ContextJSON, as: T.Type) throws -> T { try JSONDecoder().decode(T.self, from: JSONEncoder().encode(value)) }
    private func accept(_ value: ContextJSON) throws -> ContextDiagnostics {
        let diagnostics = try decode(value, as: ContextDiagnostics.self)
        let report = diagnostics.report
        guard diagnostics.version == 1, report.version == 1 else { throw ContextClientError.unsupportedVersion }
        guard report.scope == scope, report.session_id == sessionID, diagnostics.session_id == sessionID else { throw ContextClientError.scopeMismatch }
        try safeInteger(report.cursor.epoch, minimum: 1); try safeInteger(report.cursor.sequence)
        guard report.cursor.epoch > cursor.epoch || (report.cursor.epoch == cursor.epoch && report.cursor.sequence >= cursor.sequence) else { throw ContextClientError.staleCursor }
        cursor = report.cursor; negotiatedVersion = 1; return diagnostics
    }
    public func activate(query: String, budget: ContextTokenBudget, turnText: String = "", generation: UInt64 = 0, modelID: String = "gpt-4o", utcOffsetSeconds: Int32? = nil, requiredMessageIDs: [String] = [], deferReductions: Bool = false) async throws -> ContextDiagnostics {
        try safeInteger(generation); try identifier(modelID); for id in requiredMessageIDs { try identifier(id) }
        if let offset = utcOffsetSeconds, !(-86400...86400).contains(offset) { throw ContextClientError.invalidInput("UTC offset") }
        let context: ContextJSON = .object(["version": .integer(1), "scope": try encoded(scope), "session_id": .string(sessionID), "budget": try encoded(budget), "generation": .integer(Int64(generation)), "model_id": .string(modelID), "query": .string(query), "utc_offset_seconds": utcOffsetSeconds.map { .integer(Int64($0)) } ?? .null, "required_message_ids": .array(requiredMessageIDs.map(ContextJSON.string)), "defer_reductions": .bool(deferReductions || ownership == .host)])
        return try accept(await call("activate", ["conversation": .string(conversation), "query": .string(query), "turn_text": .string(turnText), "budget_tokens": .integer(Int64(budget.available)), "context": context]))
    }
    public func inspect() async throws -> ContextDiagnostics { try accept(await call("inspect", ["uri": .string(uri())])) }
    private func uri(_ suffix: String = "") -> String {
        let allowed = CharacterSet(charactersIn: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~")
        return "hm://\(actorID)/context/\(sessionID.addingPercentEncoding(withAllowedCharacters: allowed)!)" + suffix
    }
    private func request() throws -> [String: ContextJSON] { ["version": .integer(1), "scope": try encoded(scope), "session_id": .string(sessionID), "conversation": .string(conversation)] }
    private func mutation(_ operation: String, request: ContextJSON, conversation: String? = nil, extra: [String: ContextJSON] = [:]) async throws -> ContextJSON {
        var context = extra; context["operation"] = .string(operation); context["request"] = request
        return try await call("remember", ["conversation": .string(conversation ?? self.conversation), "content": .string(""), "kind": .string("user"), "context": .object(context)])
    }
    private func receipt(_ value: ContextJSON, session: String? = nil) throws -> ContextHistoryReceipt {
        let result = try decode(value, as: ContextHistoryReceipt.self)
        guard result.version == 1 else { throw ContextClientError.unsupportedVersion }
        guard result.scope == scope, result.session_id == (session ?? sessionID) else { throw ContextClientError.scopeMismatch }
        try safeInteger(result.cursor.epoch, minimum: 1); try safeInteger(result.cursor.sequence)
        if session == nil { guard result.cursor.epoch > cursor.epoch || (result.cursor.epoch == cursor.epoch && result.cursor.sequence >= cursor.sequence) else { throw ContextClientError.staleCursor }; cursor = result.cursor }
        return result
    }
    public func ingest(_ message: ContextSourceMessage, originalBytes: Data) async throws -> ContextHistoryReceipt {
        var input = try request(); input["message"] = try message.wire(); input["original_bytes"] = .array(originalBytes.map { .integer(Int64($0)) })
        return try receipt(await mutation("source", request: .object(input)))
    }
    public func relate(_ relation: ContextRelation) async throws -> ContextHistoryReceipt {
        var input = try request(); input["relation"] = try relation.wire()
        return try receipt(await mutation("relation", request: .object(input)))
    }
    public func fork(childSessionID: String, childConversation: String? = nil) async throws -> ContextHistoryReceipt {
        let child = childConversation ?? childSessionID; try identifier(childSessionID); try identifier(child)
        let input: ContextJSON = .object(["version": .integer(1), "scope": try encoded(scope), "parent_session_id": .string(sessionID), "parent_conversation": .string(conversation), "child_session_id": .string(childSessionID), "child_conversation": .string(child)])
        return try receipt(await mutation("fork", request: input, conversation: child), session: childSessionID)
    }
    public func history() async throws -> ContextHistoryInspection {
        let result = try decode(await call("inspect", ["uri": .string(uri("/history"))]), as: ContextHistoryInspection.self)
        guard result.version == 1, result.scope == scope, result.session_id == sessionID else { throw ContextClientError.invalidResponse("history identity") }; return result
    }
    public func recover(sourceID: String) async throws -> Data {
        try identifier(sourceID)
        let allowed = CharacterSet(charactersIn: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~")
        let value = try await call("inspect", ["uri": .string(uri("/source/" + sourceID.addingPercentEncoding(withAllowedCharacters: allowed)!))])
        guard let bytes = value["original_bytes"].array else { throw ContextClientError.invalidResponse("source bytes") }
        return try Data(bytes.map { guard let byte = $0.integer, (0...255).contains(byte) else { throw ContextClientError.invalidResponse("source byte") }; return UInt8(byte) })
    }
    public func job(requestID: String, action: ContextJobAction) async throws -> ContextJSON {
        try identifier(requestID)
        return try await mutation("job", request: .object(["version": .integer(1), "scope": try encoded(scope), "request_id": .string(requestID), "action": try action.wire]))
    }
    public func importBundle(_ bundle: ContextImportBundle, maxEntries: UInt64 = 128) async throws -> ContextImportReceipt {
        guard bundle.scope == scope else { throw ContextClientError.scopeMismatch }; try safeInteger(maxEntries, minimum: 1)
        let value = try await mutation("import", request: bundle.wire(), extra: ["max_entries": .integer(Int64(maxEntries))])
        let result = try decode(value, as: ContextImportReceipt.self)
        guard result.version == 1, result.scope == scope, result.import_id == bundle.importID, result.bundle_digest == bundle.digest else { throw ContextClientError.invalidResponse("import identity") }
        return result
    }
    public func remember(content: String, kind: ContextMemoryKind = .user) async throws -> ContextJSON {
        try await call("remember", ["conversation": .string(conversation), "content": .string(content), "kind": .string(kind.rawValue)])
    }
    public func recall(query: String, limit: UInt64 = 8) async throws -> ContextJSON {
        try safeInteger(limit, minimum: 1)
        let raw = try await engine.recall(argumentsJSON: ContextJSON.object(["mode": .string("lexical"), "query": .string(query), "limit": .integer(Int64(limit))]).canonical())
        let envelope = try JSONDecoder().decode(ContextJSON.self, from: Data(raw.utf8))
        guard envelope["ok"] == .bool(true) else { throw ContextClientError.operationFailed(effectState: envelope["effect_state"].string ?? "unknown") }; return envelope
    }
}
public enum ContextMemoryKind: String, Sendable { case user, assistant, document }

public struct ContextImportEntry: Sendable {
    public let sourceID: String; public let kind: String; public let payload: ContextJSON; public let digest: String
    public init(sourceID: String, kind: String, payload: ContextJSON) throws { try identifier(sourceID); try identifier(kind); self.sourceID = sourceID; self.kind = kind; self.payload = payload; digest = ContextSHA256.hex(try payload.canonical()) }
    func canonical() throws -> String { "{\"source_id\":\(quote(sourceID)),\"kind\":\(quote(kind)),\"digest\":\(quote(digest)),\"payload\":\(try payload.canonical())}" }
}
public struct ContextImportBundle: Sendable {
    public let scope: ContextScope; public let importID: String; public let entries: [ContextImportEntry]; public let digest: String
    public init(scope: ContextScope, importID: String, entries: [ContextImportEntry]) throws {
        try identifier(importID); guard entries.count <= 100_000, Set(entries.map(\.sourceID)).count == entries.count else { throw ContextClientError.invalidInput("import entries") }
        self.scope = scope; self.importID = importID; self.entries = entries
        digest = ContextSHA256.hex(try Self.canonical(scope, importID, entries, ""))
    }
    static func canonical(_ scope: ContextScope, _ id: String, _ entries: [ContextImportEntry], _ digest: String) throws -> String {
        let scoped = "{\"owner_id\":\(quote(scope.owner_id)),\"project_id\":\(quote(scope.project_id)),\"workspace_id\":\(scope.workspace_id.map(quote) ?? "null")}"
        return "{\"version\":1,\"import_id\":\(quote(id)),\"scope\":\(scoped),\"entries\":[\(try entries.map { try $0.canonical() }.joined(separator: ","))],\"digest\":\(quote(digest))}"
    }
    public func wire() throws -> ContextJSON { try JSONDecoder().decode(ContextJSON.self, from: Data(Self.canonical(scope, importID, entries, digest).utf8)) }
}
public struct ContextImportReceipt: Decodable, Sendable { public let version: UInt32; public let scope: ContextScope; public let import_id: String; public let bundle_digest: String; public let accepted: UInt64; public let total: UInt64; public let complete: Bool; public let last_lsn: UInt64 }
