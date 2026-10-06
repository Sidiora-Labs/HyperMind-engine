import Foundation

public struct ContextSourceChunk: Sendable {
    public let sources: [ContextSourceMessage]; public let spans: [ContextSourceSpan]; public let digest: String
    public init(sources: [ContextSourceMessage], spans: [ContextSourceSpan]) throws {
        guard !sources.isEmpty, sources.count == spans.count, Set(sources.map(\.id)).count == sources.count else { throw ContextClientError.invalidInput("chunk") }
        for (source, span) in zip(sources, spans) { guard source.id == span.source_id, source.sourceDigest == span.source_digest, span.byte_start == 0, span.byte_end > 0 else { throw ContextClientError.invalidInput("coverage") } }
        self.sources = sources; self.spans = spans
        let messages = sources.map { ContextSourceMessage.canonical($0.id,$0.ordinal,$0.role,$0.parts,$0.occurredAtNS,$0.recordedAtNS,$0.authority,$0.sourceDigest) }.joined(separator: ",")
        let coverage = spans.map { "{\"source_id\":\(quote($0.source_id)),\"source_digest\":\(quote($0.source_digest)),\"byte_start\":\($0.byte_start),\"byte_end\":\($0.byte_end)}" }.joined(separator: ",")
        digest = ContextSHA256.hex("[[\(messages)],[\(coverage)]]")
    }
    func wire() throws -> ContextJSON { .object(["sources": .array(try sources.map { try $0.wire() }), "spans": try encoded(spans), "digest": .string(digest)]) }
}
public struct ContextSummaryTier: Codable, Sendable {
    public let text: String; public let coverage: [ContextSourceSpan]
    public init(text: String, coverage: [ContextSourceSpan]) { self.text = text; self.coverage = coverage }
}
public struct ContextHistorianResult: Codable, Sendable {
    public let source_digest: String; public let tiers: [ContextSummaryTier]
    public init(sourceDigest: String, tiers: [ContextSummaryTier]) throws { guard tiers.count == 4 else { throw ContextClientError.invalidInput("four summary tiers required") }; source_digest = sourceDigest; self.tiers = tiers }
}
public struct ContextHistorianClaim: Codable, Sendable {
    public let job: ContextJSON; public let worker: String; public let attempt: UInt64
    public init(response: ContextJSON) throws {
        let response = response["request_id"].string == nil ? response : response["result"]
        guard let worker = response["worker"].string, let attempt = response["attempt"].integer, attempt > 0, response["job"]["id"].string != nil else { throw ContextClientError.invalidResponse("historian claim") }
        try identifier(worker); try safeInteger(UInt64(attempt), minimum: 1)
        self.worker = worker; self.attempt = UInt64(attempt); job = response["job"]
    }
}
public enum ContextUsage: Sendable { case unknown, known(UInt64)
    func wire() throws -> ContextJSON { switch self { case .unknown: return .string("Unknown"); case .known(let value): try safeInteger(value); return .object(["Known": .integer(Int64(value))]) } }
}
public enum ContextJobKind: String, Codable, Sendable { case historian, verification, curation, extraction, indexing, consolidation }
public struct ContextMaintenanceRequest: Sendable {
    public let kind: ContextJobKind; public let sources: [ContextSourceMessage]; public let cursor: ContextCursor; public let sourceRevision: UInt64; public let policyRevision: UInt64; public let reservation: UInt64
    public init(kind: ContextJobKind, sources: [ContextSourceMessage], cursor: ContextCursor, sourceRevision: UInt64, policyRevision: UInt64, reservation: UInt64) throws {
        try safeInteger(sourceRevision); try safeInteger(policyRevision); try safeInteger(reservation)
        self.kind = kind; self.sources = sources; self.cursor = cursor; self.sourceRevision = sourceRevision; self.policyRevision = policyRevision; self.reservation = reservation
    }
    func wire() throws -> ContextJSON { .object(["kind": .string(kind.rawValue), "sources": .array(try sources.map { try $0.wire() }), "cursor": try encoded(cursor), "source_revision": .integer(Int64(sourceRevision)), "policy_revision": .integer(Int64(policyRevision)), "reservation": .integer(Int64(reservation))]) }
}
public struct ContextPublicationFence: Codable, Sendable { public let input_digest: String; public let source_revision: UInt64; public let policy_revision: UInt64 }
public struct ContextJobLease: Codable, Sendable { public let job_id: String; public let attempt: UInt64; public let expires_ms: UInt64; public let fence: ContextPublicationFence }
public indirect enum ContextNotePredicate: Sendable {
    case always, exists(String), equals(key: String, value: String), not(ContextNotePredicate), all([ContextNotePredicate]), any([ContextNotePredicate])
    func wire(depth: Int = 0) throws -> ContextJSON {
        guard depth <= 32 else { throw ContextClientError.invalidInput("predicate depth") }
        switch self {
        case .always: return .string("True")
        case .exists(let key): return .object(["Exists": .string(key)])
        case .equals(let key, let value): return .object(["Equals": .object(["key": .string(key), "value": .string(value)])])
        case .not(let child): return .object(["Not": try child.wire(depth: depth+1)])
        case .all(let children): return .object(["All": .array(try children.map { try $0.wire(depth: depth+1) })])
        case .any(let children): return .object(["Any": .array(try children.map { try $0.wire(depth: depth+1) })])
        }
    }
}
public enum ContextNoteKind: String, Sendable { case anchor = "Anchor", note = "Note", primer = "Primer" }
public struct ContextNote: Sendable {
    public let id: String; public let kind: ContextNoteKind; public let revision: UInt64; public let text: String; public let parents: [String]; public let contradictions: [String]; public let expiresAtNS: Int64?; public let predicate: ContextNotePredicate; public let tombstoned: Bool
    public init(id: String, kind: ContextNoteKind, revision: UInt64, text: String, parents: [String] = [], contradictions: [String] = [], expiresAtNS: Int64? = nil, predicate: ContextNotePredicate = .always, tombstoned: Bool = false) throws {
        try identifier(id); try safeInteger(revision, minimum: 1); self.id=id; self.kind=kind; self.revision=revision; self.text=text; self.parents=parents; self.contradictions=contradictions; self.expiresAtNS=expiresAtNS; self.predicate=predicate; self.tombstoned=tombstoned
    }
    func wire() throws -> ContextJSON { .object(["id": .string(id), "kind": .string(kind.rawValue), "revision": .integer(Int64(revision)), "text": .string(text), "parents": .array(parents.map(ContextJSON.string)), "contradictions": .array(contradictions.map(ContextJSON.string)), "expires_at_ns": expiresAtNS.map { .string(String($0)) } ?? .null, "predicate": try predicate.wire(), "tombstoned": .bool(tombstoned)]) }
}
public enum ContextNotesCommand: Sendable {
    case create(ContextNote), revise(ContextNote, expectedRevision: UInt64), tombstone(id: String, expectedRevision: UInt64)
    case setGrant(principal: String, noteID: String, read: Bool, write: Bool), revokeGrant(principal: String, noteID: String)
    case setAttributeGrant(principal: String, key: String, write: Bool)
    case proposeAttribute(id: String, key: String, value: String, baseRevision: UInt64, proposer: String), acceptAttribute(proposalID: String), rejectAttribute(proposalID: String)
    func wire() throws -> ContextJSON {
        switch self {
        case .create(let note): return .object(["Create": try note.wire()])
        case .revise(let note, let revision): try safeInteger(revision); return .object(["Revise": .object(["note": try note.wire(), "expected_revision": .integer(Int64(revision))])])
        case .tombstone(let id, let revision): try identifier(id); try safeInteger(revision); return .object(["Tombstone": .object(["id": .string(id), "expected_revision": .integer(Int64(revision))])])
        case .setGrant(let principal, let id, let read, let write): return .object(["SetGrant": .object(["principal": .string(principal), "note_id": .string(id), "read": .bool(read), "write": .bool(write)])])
        case .revokeGrant(let principal, let id): return .object(["RevokeGrant": .object(["principal": .string(principal), "note_id": .string(id)])])
        case .setAttributeGrant(let principal, let key, let write): return .object(["SetAttributeGrant": .object(["principal": .string(principal), "key": .string(key), "write": .bool(write)])])
        case .proposeAttribute(let id, let key, let value, let revision, let proposer): try safeInteger(revision); return .object(["ProposeAttribute": .object(["id": .string(id), "key": .string(key), "value": .string(value), "base_revision": .integer(Int64(revision)), "proposer": .string(proposer)])])
        case .acceptAttribute(let id): return .object(["AcceptAttribute": .object(["proposal_id": .string(id)])])
        case .rejectAttribute(let id): return .object(["RejectAttribute": .object(["proposal_id": .string(id)])])
        }
    }
}
public enum ContextJobAction: Sendable {
    case inspect, notes(ContextNotesCommand), readNote(id: String, nowNS: Int64, facts: [String: String]), readAttribute(key: String), setPolicy(sessionID: String, expectedRevision: UInt64, revision: UInt64)
    case historianEnqueue(sessionID: String, cursor: ContextCursor, policyRevision: UInt64, chunk: ContextSourceChunk, reservation: UInt64, nowMS: UInt64)
    case historianClaim(worker: String, nowMS: UInt64, leaseMS: UInt64), historianHeartbeat(claim: ContextHistorianClaim, nowMS: UInt64, leaseMS: UInt64)
    case historianComplete(claim: ContextHistorianClaim, result: ContextHistorianResult, usage: ContextUsage, nowMS: UInt64), historianFail(claim: ContextHistorianClaim, usage: ContextUsage, nowMS: UInt64, cooldownMS: UInt64), historianCancel(id: String), historianExpire(nowMS: UInt64, cooldownMS: UInt64)
    case maintenanceEnqueue(sessionID: String, request: ContextMaintenanceRequest), maintenanceClaim(nowMS: UInt64), maintenanceComplete(lease: ContextJobLease, usage: ContextUsage, outputDigest: String, nowMS: UInt64), maintenanceFail(lease: ContextJobLease, usage: ContextUsage, nowMS: UInt64), maintenanceCancel(id: String, nowMS: UInt64), maintenanceSettle(id: String, attempt: UInt64, actual: UInt64)
    var wire: ContextJSON { get throws {
        func integer(_ value: UInt64) throws -> ContextJSON { try safeInteger(value); return .integer(Int64(value)) }
        func action(_ name: String, _ values: [String: ContextJSON] = [:]) -> ContextJSON { var result=values; result["action"] = .string(name); return .object(result) }
        switch self {
        case .inspect: return action("inspect")
        case .notes(let command): return action("notes", ["command": try command.wire()])
        case .readNote(let id, let ns, let facts): return action("read_note", ["id": .string(id), "now_ns": .string(String(ns)), "facts": .object(facts.mapValues(ContextJSON.string))])
        case .readAttribute(let key): return action("read_attribute", ["key": .string(key)])
        case .setPolicy(let session, let expected, let revision): return action("set_policy", ["session_id": .string(session), "expected_revision": try integer(expected), "revision": try integer(revision)])
        case .historianEnqueue(let session, let cursor, let policy, let chunk, let reservation, let now): return action("historian_enqueue", ["session_id": .string(session), "cursor": try encoded(cursor), "policy_revision": try integer(policy), "chunk": try chunk.wire(), "reservation": try integer(reservation), "now_ms": try integer(now)])
        case .historianClaim(let worker, let now, let lease): return action("historian_claim", ["worker": .string(worker), "now_ms": try integer(now), "lease_ms": try integer(lease)])
        case .historianHeartbeat(let claim, let now, let lease): return action("historian_heartbeat", ["claim": try encoded(claim), "now_ms": try integer(now), "lease_ms": try integer(lease)])
        case .historianComplete(let claim, let result, let usage, let now): return action("historian_complete", ["claim": try encoded(claim), "result": try encoded(result), "usage": try usage.wire(), "now_ms": try integer(now)])
        case .historianFail(let claim, let usage, let now, let cooldown): return action("historian_fail", ["claim": try encoded(claim), "usage": try usage.wire(), "now_ms": try integer(now), "cooldown_ms": try integer(cooldown)])
        case .historianCancel(let id): return action("historian_cancel", ["id": .string(id)])
        case .historianExpire(let now, let cooldown): return action("historian_expire", ["now_ms": try integer(now), "cooldown_ms": try integer(cooldown)])
        case .maintenanceEnqueue(let session, let request): return action("maintenance_enqueue", ["session_id": .string(session), "request": try request.wire()])
        case .maintenanceClaim(let now): return action("maintenance_claim", ["now_ms": try integer(now)])
        case .maintenanceComplete(let lease, let usage, let digest, let now): return action("maintenance_complete", ["lease": try encoded(lease), "usage": try usage.wire(), "output_digest": .string(digest), "now_ms": try integer(now)])
        case .maintenanceFail(let lease, let usage, let now): return action("maintenance_fail", ["lease": try encoded(lease), "usage": try usage.wire(), "now_ms": try integer(now)])
        case .maintenanceCancel(let id, let now): return action("maintenance_cancel", ["id": .string(id), "now_ms": try integer(now)])
        case .maintenanceSettle(let id, let attempt, let actual): return action("maintenance_settle", ["id": .string(id), "attempt": try integer(attempt), "actual": try integer(actual)])
        }
    } }
}
