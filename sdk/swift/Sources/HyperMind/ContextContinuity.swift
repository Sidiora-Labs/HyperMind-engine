import Foundation

public struct ContextContinuationState: Codable, Equatable, Sendable {
    public let cursor: ContextCursor
    public let generation: UInt64
    public let modelID: String?
    public let profile: [String: Bool]?
}
public struct ContextMutationOutcome: Codable, Equatable, Sendable {
    public let operation: String
    public let effectState: String
    public let identity: String
    public let detail: String
    public init(operation: String, identity: String, detail: String) {
        self.operation = operation; self.identity = identity; self.detail = detail; self.effectState = "unknown"
    }
}
public struct ContextContinuationCheckpoint: Codable, Sendable {
    public let scope: ContextScope
    public let actorID: UInt16
    public let sessionID: String
    public let conversation: String
    public let ownership: ContextOwnership
    public let accepted: ContextContinuationState?
    public let uncertain: ContextMutationOutcome?
    public let unresolved: [ContextMutationOutcome]
    public init(scope: ContextScope, actorID: UInt16, sessionID: String, conversation: String, ownership: ContextOwnership, accepted: ContextContinuationState?, uncertain: ContextMutationOutcome?, unresolved: [ContextMutationOutcome] = []) {
        self.scope = scope; self.actorID = actorID; self.sessionID = sessionID; self.conversation = conversation; self.ownership = ownership; self.accepted = accepted; self.uncertain = uncertain; self.unresolved = unresolved
    }
}
public struct ContextContinuationError: Error, Sendable {
    public let code: String
    public let effectState: String
}
public actor ContextContinuation {
    public private(set) var context: HyperMindContextClient
    public private(set) var accepted: ContextContinuationState?
    public private(set) var uncertain: ContextMutationOutcome?
    public private(set) var unresolved: [ContextMutationOutcome] = []
    public private(set) var generationCompatible = false
    private var busy = false
    public init(context: HyperMindContextClient, checkpoint: ContextContinuationCheckpoint? = nil) throws {
        self.context = context
        if let checkpoint {
            guard checkpoint.scope == context.scope, checkpoint.actorID == context.actorID, checkpoint.sessionID == context.sessionID, checkpoint.conversation == context.conversation, checkpoint.ownership == context.ownership else { throw ContextContinuationError(code: "checkpoint_identity_mismatch", effectState: "not_dispatched") }
            if let state = checkpoint.accepted { try safeInteger(state.generation); try safeInteger(state.cursor.epoch, minimum: 1); try safeInteger(state.cursor.sequence) }
            self.accepted = checkpoint.accepted; self.uncertain = checkpoint.uncertain; self.unresolved = checkpoint.unresolved
        }
    }
    public func checkpoint() -> ContextContinuationCheckpoint {
        ContextContinuationCheckpoint(scope: context.scope, actorID: context.actorID, sessionID: context.sessionID, conversation: context.conversation, ownership: context.ownership, accepted: accepted, uncertain: uncertain, unresolved: unresolved)
    }
    private func begin() throws {
        guard !busy else { throw ContextContinuationError(code: "operation_in_progress", effectState: "not_dispatched") }; busy = true
    }
    private func accept(_ diagnostics: ContextDiagnostics, model: String?, profile: [String: Bool]?) throws -> ContextDiagnostics {
        let report = diagnostics.report; try safeInteger(report.generation)
        if let prior = accepted, report.cursor.epoch < prior.cursor.epoch || (report.cursor.epoch == prior.cursor.epoch && report.cursor.sequence < prior.cursor.sequence) { throw ContextContinuationError(code: "stale_cursor", effectState: "rejected") }
        accepted = ContextContinuationState(cursor: report.cursor, generation: report.generation, modelID: model, profile: profile); generationCompatible = true; return diagnostics
    }
    private func mutation<T>(_ operation: String, _ identity: String, _ invoke: () async throws -> T) async throws -> T {
        try begin(); defer { busy = false }
        guard uncertain == nil else { throw ContextContinuationError(code: "uncertain_mutation_requires_reconciliation", effectState: "not_dispatched") }
        guard !unresolved.contains(where: { $0.operation == operation && $0.identity == identity }) else { throw ContextContinuationError(code: "unresolved_mutation_identity", effectState: "not_dispatched") }
        guard !Task.isCancelled else { throw ContextContinuationError(code: "cancelled_before_dispatch", effectState: "not_dispatched") }
        do {
            let result = try await invoke()
            if Task.isCancelled { throw CancellationError() }
            return result
        } catch {
            let effect = (error as? ContextOperationError)?.effectState ?? "unknown"
            let known = effect == "not_dispatched" || effect == "rejected"
            if !known { uncertain = ContextMutationOutcome(operation: operation, identity: identity, detail: String(describing: type(of: error))) }
            throw ContextContinuationError(code: operation + "_failed", effectState: known ? effect : "unknown")
        }
    }
    public func activate(query: String, budget: ContextTokenBudget, modelID: String = "gpt-4o", profile: [String: Bool]? = nil) async throws -> ContextDiagnostics {
        if accepted?.modelID != modelID || accepted?.profile != profile { generationCompatible = false }
        let generation = generationCompatible ? (accepted?.generation ?? 0) : 0
        let result = try await mutation("activate", "context-generation") {
            if profile == nil { return try await context.activate(query: query, budget: budget, generation: generation, modelID: modelID) }
            let options: ContextJSON = .object(["version": .integer(1), "scope": try encoded(context.scope), "session_id": .string(context.sessionID), "budget": try encoded(budget), "generation": .integer(Int64(generation)), "model_id": .string(modelID), "query": .string(query), "profile": .object(profile!.mapValues(ContextJSON.bool)), "defer_reductions": .bool(context.ownership == .host)])
            let response = try await context.engine.call(verb: "activate", argumentsJSON: ContextJSON.object(["conversation": .string(context.conversation), "query": .string(query), "budget_tokens": .integer(Int64(budget.available)), "context": options]).canonical())
            let envelope = try JSONDecoder().decode(ContextJSON.self, from: Data(response.utf8))
            guard envelope["ok"] == .bool(true) else { throw ContextOperationError(verb: "activate", arguments: .object([:]), envelope: envelope) }
            guard let item = envelope["items"].array?.first else { throw ContextClientError.invalidResponse("missing item") }
            let diagnostics = try JSONDecoder().decode(ContextDiagnostics.self, from: JSONEncoder().encode(item))
            guard diagnostics.report.scope == context.scope, diagnostics.session_id == context.sessionID else { throw ContextClientError.scopeMismatch }; return diagnostics
        }
        return try accept(result, model: modelID, profile: profile)
    }
    public func inspect() async throws -> ContextDiagnostics {
        try begin(); defer { busy = false }; let compatible = generationCompatible
        let result = try accept(await context.inspect(), model: accepted?.modelID, profile: accepted?.profile); generationCompatible = compatible; return result
    }
    public func reconnect(_ fresh: HyperMindContextClient) async throws -> ContextDiagnostics {
        try begin(); defer { busy = false }
        guard fresh.scope == context.scope, fresh.actorID == context.actorID, fresh.sessionID == context.sessionID, fresh.conversation == context.conversation, fresh.ownership == context.ownership else { throw ContextContinuationError(code: "reconnect_identity_mismatch", effectState: "not_dispatched") }
        let result = try accept(await fresh.inspect(), model: accepted?.modelID, profile: accepted?.profile); context = fresh; generationCompatible = false; return result
    }
    public func close() throws { try begin(); defer { busy = false }; try context.engine.close(); generationCompatible = false }
    public func abandonUncertain() async throws -> ContextMutationOutcome {
        try begin(); defer { busy = false }; guard let outcome = uncertain else { throw ContextClientError.invalidInput("no uncertain mutation") }
        _ = try accept(await context.inspect(), model: accepted?.modelID, profile: accepted?.profile); unresolved.append(outcome); uncertain = nil; generationCompatible = false; return outcome
    }
    public func reconcileSource(_ message: ContextSourceMessage) async throws -> Bool {
        try begin(); defer { busy = false }
        guard uncertain?.operation == "source", uncertain?.identity == message.id else { throw ContextClientError.invalidInput("uncertain source identity required") }
        let frozen = message; let history = try await context.history()
        guard history.messages.contains(where: { $0.id == frozen.id && $0.sourceDigest == frozen.sourceDigest }) else { return false }
        _ = try accept(await context.inspect(), model: accepted?.modelID, profile: accepted?.profile); uncertain = nil; generationCompatible = false; return true
    }
    public func ingest(_ message: ContextSourceMessage, originalBytes: Data) async throws -> ContextHistoryReceipt { try await mutation("source", message.id) { try await context.ingest(message, originalBytes: originalBytes) } }
    public func fork(childSessionID: String, childConversation: String? = nil) async throws -> ContextHistoryReceipt { try await mutation("fork", childSessionID) { try await context.fork(childSessionID: childSessionID, childConversation: childConversation) } }
    public func relate(_ relation: ContextRelation) async throws -> ContextHistoryReceipt { try await mutation("relation", try relation.wire()["id"].string ?? "") { try await context.relate(relation) } }
    public func job(requestID: String, action: ContextJobAction) async throws -> ContextJSON { try await mutation("job", requestID) { try await context.job(requestID: requestID, action: action) } }
    public func importBundle(_ bundle: ContextImportBundle, maxEntries: UInt64 = 128) async throws -> ContextImportReceipt { try await mutation("import", bundle.importID) { try await context.importBundle(bundle, maxEntries: maxEntries) } }
}
