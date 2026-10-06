"""Versioned context contract over the existing HyperMind tool verbs."""
from __future__ import annotations

from dataclasses import asdict, dataclass, replace
from hashlib import sha256
import json
from types import MappingProxyType
import re
from urllib.parse import quote
from typing import Any, Literal, Protocol, TypedDict

CONTEXT_VERSION = 1
MAX_SAFE_INTEGER = (1 << 53) - 1
Authority = Literal['user_asserted', 'external_observed', 'tool_observed', 'runtime_fact', 'assistant_generated', 'derived_inference']
EffectState = Literal['not_dispatched', 'unknown', 'rejected']
ContextOwner = Literal['host', 'hypermind']


def _identifier(value: str) -> None:
    if not isinstance(value, str) or not value or len(value.encode()) > 256 or any(not 33 <= ord(c) <= 126 for c in value):
        raise ValueError('invalid identifier')


def _integer(value: int, minimum: int = 0) -> None:
    if type(value) is not int or not minimum <= value <= MAX_SAFE_INTEGER:
        raise ValueError('integer outside interoperable range')


@dataclass(frozen=True)
class Scope:
    owner_id: str
    project_id: str
    workspace_id: str | None = None

    def __post_init__(self):
        _identifier(self.owner_id)
        _identifier(self.project_id)
        if self.workspace_id is not None: _identifier(self.workspace_id)


@dataclass(frozen=True)
class Cursor:
    epoch: int = 1
    sequence: int = 0

    def __post_init__(self):
        _integer(self.epoch, 1)
        _integer(self.sequence)


@dataclass(frozen=True)
class TokenBudget:
    context_tokens: int
    reserved_output_tokens: int
    required_tokens: int

    def __post_init__(self):
        _integer(self.context_tokens, 1)
        _integer(self.reserved_output_tokens, 1)
        _integer(self.required_tokens)
        if self.available < 0: raise ValueError('capacity exceeded')

    @property
    def available(self) -> int:
        return self.context_tokens - self.reserved_output_tokens - self.required_tokens


@dataclass(frozen=True)
class SourceMessage:
    id: str
    ordinal: int
    role: Literal['user', 'assistant', 'tool']
    parts: tuple[dict[str, Any], ...]
    occurred_at_ns: int | str | None
    recorded_at_ns: int | str
    authority: Authority
    source_digest: str = ''

    def wire(self) -> dict[str, Any]:
        _identifier(self.id)
        _integer(self.ordinal)
        if self.role not in ('user', 'assistant', 'tool') or self.authority not in ('user_asserted', 'external_observed', 'tool_observed', 'runtime_fact', 'assistant_generated', 'derived_inference'):
            raise ValueError('invalid source authority or role')
        if not 1 <= len(self.parts) <= 4096: raise ValueError('invalid source parts')
        shapes = {'text': ('kind', 'text'), 'tool_call': ('kind', 'call_id', 'name', 'arguments'), 'tool_result': ('kind', 'call_id', 'content', 'failed'), 'opaque': ('kind', 'media_type', 'reference', 'digest')}
        parts = []
        for part in self.parts:
            keys = shapes.get(part.get('kind'))
            if keys is None or set(part) != set(keys): raise ValueError('invalid source part')
            if any(type(part[key]) is not (bool if key == 'failed' else str) for key in keys): raise ValueError('invalid source part value')
            parts.append({key: part[key] for key in keys})
        value = {'id': self.id, 'ordinal': self.ordinal, 'role': self.role, 'parts': parts, 'occurred_at_ns': _timestamp_decimal(self.occurred_at_ns, True), 'recorded_at_ns': _timestamp_decimal(self.recorded_at_ns), 'authority': self.authority, 'source_digest': ''}
        digest = sha256(json.dumps(value, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode()).hexdigest()
        if self.source_digest and self.source_digest != digest: raise ValueError('source identity changed')
        value['source_digest'] = digest
        return value

    def freeze(self) -> SourceMessage:
        value = self.wire()
        # Freeze a serialized snapshot, independent of the caller's mutable parts.
        return replace(self, parts=tuple(MappingProxyType(part) for part in value['parts']), source_digest=value['source_digest'])


class SourceSpan(TypedDict):
    source_id: str
    source_digest: str
    byte_start: int
    byte_end: int


class ContextBlock(TypedDict):
    id: str
    text: str
    authority: Authority
    provenance: list[SourceSpan]
    tokens: int
    required: bool


class ContextReport(TypedDict):
    version: int
    scope: dict[str, Any]
    session_id: str
    cursor: dict[str, int]
    generation: int
    blocks: list[ContextBlock]
    included: list[str]
    omitted: list[dict[str, str]]
    gaps: list[str]
    token_count: int
    digest: str


class CapabilityProfile(TypedDict):
    user: bool
    assistant: bool
    tool: bool
    text: bool
    tool_calls: bool
    tool_results: bool
    opaque: bool

class RenderedMessage(TypedDict):
    id: str
    role: Literal['user', 'assistant', 'tool']
    parts: list[dict[str, Any]]
    authority: Authority
    provenance: list[SourceSpan]

class ActivationOptions(TypedDict, total=False):
    model_id: str
    profile: CapabilityProfile
    tier: Literal['detailed', 'condensed', 'brief', 'outline']
    defer_reductions: bool
    required_message_ids: list[str]
    utc_offset_seconds: int | None
    memory_scopes: list[dict[str, str | None]]
    evidence_grants: list[EvidenceGrant]

class ContextDiagnostics(TypedDict):
    version: int
    session_id: str
    report: ContextReport
    messages: list[RenderedMessage]
    history: dict[str, Any]
    coverage: Any
    cache: Any
    jobs: Any
    provenance: Any
    migrations: Any
    gaps: Any


class ToolClient(Protocol):
    async def tool(self, verb: str, arguments: dict[str, Any]) -> dict[str, Any]: ...


class HostAdapter(Protocol):
    context_owner: Literal['host', 'hypermind']
    async def source_messages(self, scope: Scope, session_id: str, cursor: Cursor) -> tuple[SourceMessage, ...]: ...
    async def apply_context(self, diagnostics: ContextDiagnostics) -> None: ...


class ContextClientError(RuntimeError):
    def __init__(self, code: str, effect_state: EffectState, detail: Any = None):
        super().__init__(code)
        self.code, self.effect_state, self.detail = code, effect_state, detail


class ContextClient:
    def __init__(self, client: ToolClient, scope: Scope, session_id: str, *, actor: int, context_owner: Literal['host', 'hypermind'], conversation: str | None = None):
        _identifier(session_id)
        conversation = session_id if conversation is None else conversation
        _identifier(conversation)
        self.conversation = conversation
        if type(actor) is not int or not 0 <= actor <= 65535: raise ValueError('invalid actor')
        if context_owner not in ('host', 'hypermind'): raise ValueError('explicit context owner required')
        self.client, self.scope, self.session_id, self.actor = client, scope, session_id, actor
        self.context_owner, self.cursor, self.version = context_owner, Cursor(), None

    def request_context(self, budget: TokenBudget, generation: int = 0, options: ActivationOptions | None = None) -> dict[str, Any]:
        _integer(generation)
        if options and set(options) - {'model_id', 'profile', 'tier', 'defer_reductions', 'required_message_ids', 'utc_offset_seconds', 'memory_scopes', 'evidence_grants'}: raise ValueError('unsupported context option')
        return {'version': CONTEXT_VERSION, 'scope': asdict(self.scope), 'session_id': self.session_id, 'budget': asdict(budget), 'generation': generation, **(options or {})}

    def accept(self, envelope: dict[str, Any]) -> ContextDiagnostics:
        if envelope.get('ok') is not True:
            effect = envelope.get('effect_state', 'unknown')
            raise ContextClientError('context_failed', effect if effect in ('not_dispatched', 'unknown', 'rejected') else 'unknown', envelope)
        try:
            diagnostics = envelope['items'][0]
            report = diagnostics['report']
            if diagnostics['version'] != CONTEXT_VERSION or report['version'] != CONTEXT_VERSION:
                raise ContextClientError('unsupported_version', 'rejected')
            if report['scope'] != asdict(self.scope) or report['session_id'] != self.session_id:
                raise ContextClientError('scope_mismatch', 'rejected')
            cursor = Cursor(**report['cursor'])
            if cursor.epoch < self.cursor.epoch or (cursor.epoch == self.cursor.epoch and cursor.sequence < self.cursor.sequence):
                raise ContextClientError('stale_cursor', 'rejected')
        except (KeyError, IndexError, TypeError, ValueError) as error:
            raise ContextClientError('invalid_response', 'unknown') from error
        self.version, self.cursor = CONTEXT_VERSION, cursor
        return diagnostics

    async def activate(self, query: str, budget: TokenBudget, *, turn_text: str = '', generation: int = 0, options: ActivationOptions | None = None) -> ContextDiagnostics:
        context = self.request_context(budget, generation, options)
        return self.accept(await self.client.tool('activate', {'conversation': self.conversation, 'query': query, 'turn_text': turn_text, 'budget_tokens': budget.available, 'context': context}))

    def inspect_uri(self) -> str:
        return f'hm://{self.actor}/context/{encode_context_identifier(self.session_id)}'

    async def inspect(self) -> ContextDiagnostics:
        return self.accept(await self.client.tool('inspect', {'uri': self.inspect_uri()}))

    def retrieval_request(self, operation: RetrievalOperation) -> dict[str, Any]:
        return {'conversation': self.conversation, 'content': '', 'kind': 'user', 'context': _job_wire(operation)}

    async def retrieval(self, operation: RetrievalOperation) -> dict[str, Any]:
        return self._check_envelope(await self.client.tool('remember', self.retrieval_request(operation)))

    async def register_retrieval_source(self, source: RetrievalSourceRecord, expected_revision: int) -> dict[str, Any]:
        return await self.retrieval({'operation': 'retrieval_source', 'source': source, 'expected_revision': expected_revision})

    async def tombstone_retrieval_source(self, kind: EvidenceKind, id: str, expected_revision: int) -> dict[str, Any]:
        return await self.retrieval({'operation': 'retrieval_tombstone', 'kind': kind, 'id': id, 'expected_revision': expected_revision})

    async def grant_evidence(self, grant: EvidenceGrant) -> dict[str, Any]:
        return await self.retrieval({'operation': 'retrieval_grant', 'grant': grant})

    async def revoke_evidence(self, recipient_scope: Scope, kind: EvidenceKind, id: str) -> dict[str, Any]:
        return await self.retrieval({'operation': 'retrieval_revoke', 'recipient_scope': asdict(recipient_scope), 'kind': kind, 'id': id})

    async def configure_embedding(self, enabled: bool, expected_revision: int) -> dict[str, Any]:
        return await self.retrieval({'operation': 'embedding', 'enabled': enabled, 'expected_revision': expected_revision})

    async def backfill(self, maximum_items: int, maximum_bytes: int) -> BackfillReport:
        envelope = await self.retrieval({'operation': 'backfill', 'maximum_items': maximum_items, 'maximum_bytes': maximum_bytes})
        return envelope['items'][0]

    async def inspect_retrieval(self) -> dict[str, Any]:
        return self._check_envelope(await self.client.tool('inspect', {'uri': f'hm://{self.actor}/context-retrieval'}))

    async def export_memory(self) -> MemoryExportArtifact:
        envelope = self._check_envelope(await self.client.tool('inspect', {'uri': f'hm://{self.actor}/context-memory-export'}))
        return decode_memory_export(envelope['items'][0], self.scope)

    async def restore_memory(self, request_id: str, artifact: MemoryExportArtifact) -> MemoryReceipt:
        verified = decode_memory_export({'version': 1, 'scope': asdict(artifact.scope), 'cursor': artifact.cursor, 'media_type': 'application/x-ndjson', 'bytes': list(artifact.data), 'byte_count': len(artifact.data), 'artifact_digest': artifact.artifact_digest, 'export_digest': artifact.export_digest, 'restore_max_bytes': artifact.restore_max_bytes}, self.scope)
        if len(verified.data) > verified.restore_max_bytes: raise ValueError('streaming restore is unsupported; artifact exceeds restore_max_bytes')
        return await self.memory(request_id, {'kind': 'restore_jsonl', 'jsonl': list(verified.data), 'artifact_digest': verified.artifact_digest})

    async def inspect_memory(self, record_id: str | None = None, *, owner_scope: Scope | None = None, source_id: str | None = None) -> MemoryView:
        uri = f'hm://{self.actor}/context-memory'
        if record_id is not None: uri += '/' + encode_context_identifier(record_id)
        if source_id is not None:
            if record_id is None: raise ValueError('source inspection requires a record')
            uri += '/source/' + encode_context_identifier(source_id)
        owner = self.scope if owner_scope is None else owner_scope
        if owner_scope is not None: uri += '?scope=' + quote(json.dumps(asdict(owner), separators=(',', ':')), safe='')
        envelope = self._check_envelope(await self.client.tool('inspect', {'uri': uri}))
        view = envelope['items'][0]
        if view['version'] != 1 or view['scope'] != asdict(owner): raise ContextClientError('invalid_memory_view', 'unknown')
        return view

    def memory_request(self, request_id: str, command: MemoryCommand) -> dict[str, Any]:
        _identifier(request_id)
        return {'conversation': self.conversation, 'content': '', 'kind': 'user', 'context': {'operation': 'memory', 'request': {'version': 1, 'scope': asdict(self.scope), 'request_id': request_id, 'command': _job_wire(command)}}}

    async def memory(self, request_id: str, command: MemoryCommand) -> MemoryReceipt:
        envelope = self._check_envelope(await self.client.tool('remember', self.memory_request(request_id, command)))
        receipt = envelope['items'][0]
        if receipt['version'] != 1 or receipt['scope'] != asdict(self.scope): raise ContextClientError('invalid_memory_receipt', 'unknown')
        return receipt

    def import_request(self, bundle: ImportBundle, max_entries: int = 128) -> dict[str, Any]:
        _integer(max_entries, 1)
        if max_entries > 256 or bundle['scope'] != asdict(self.scope): raise ValueError('invalid import scope or batch size')
        return {'conversation': self.conversation, 'content': '', 'kind': 'user', 'context': {'operation': 'import', 'request': bundle, 'max_entries': max_entries}}

    async def import_bundle(self, bundle: ImportBundle, max_entries: int = 128) -> ImportReceipt:
        envelope = self._check_envelope(await self.client.tool('remember', self.import_request(bundle, max_entries)))
        receipt = envelope['items'][0]
        if receipt['version'] not in (1, 2) or receipt['scope'] != asdict(self.scope) or receipt['import_id'] != bundle['import_id'] or receipt['bundle_digest'] != bundle['digest']:
            raise ContextClientError('invalid_import_receipt', 'unknown')
        return receipt

    def source_request(self, message: SourceMessage, original_bytes: bytes) -> dict[str, Any]:
        return {'conversation': self.conversation, 'content': '', 'kind': 'user', 'context': {'operation': 'source', 'request': {'version': 1, 'scope': asdict(self.scope), 'session_id': self.session_id, 'conversation': self.conversation, 'message': message.wire(), 'original_bytes': list(original_bytes)}}}

    async def ingest_source(self, message: SourceMessage, original_bytes: bytes) -> HistoryReceipt:
        return self._history_receipt(await self.client.tool('remember', self.source_request(message, original_bytes)))

    def relation_request(self, relation: SourceRelation) -> dict[str, Any]:
        return {'conversation': self.conversation, 'content': '', 'kind': 'user', 'context': {'operation': 'relation', 'request': {'version': 1, 'scope': asdict(self.scope), 'session_id': self.session_id, 'conversation': self.conversation, 'relation': relation}}}

    async def relate(self, relation: SourceRelation) -> HistoryReceipt:
        return self._history_receipt(await self.client.tool('remember', self.relation_request(relation)))

    def fork_request(self, child_session_id: str, child_conversation: str | None = None) -> dict[str, Any]:
        _identifier(child_session_id)
        child_conversation = child_session_id if child_conversation is None else child_conversation
        _identifier(child_conversation)
        return {'conversation': child_conversation, 'content': '', 'kind': 'user', 'context': {'operation': 'fork', 'request': {'version': 1, 'scope': asdict(self.scope), 'parent_session_id': self.session_id, 'parent_conversation': self.conversation, 'child_session_id': child_session_id, 'child_conversation': child_conversation}}}

    async def fork(self, child_session_id: str, child_conversation: str | None = None) -> HistoryReceipt:
        return self._history_receipt(await self.client.tool('remember', self.fork_request(child_session_id, child_conversation)), child_session_id)

    def job_request(self, request_id: str, action: ContextJobAction) -> dict[str, Any]:
        _identifier(request_id)
        return {'conversation': self.conversation, 'content': '', 'kind': 'user', 'context': {'operation': 'job', 'request': {'version': 1, 'scope': asdict(self.scope), 'request_id': request_id, 'action': _job_wire(action)}}}

    async def job(self, request_id: str, action: ContextJobAction) -> dict[str, Any]:
        return self._check_envelope(await self.client.tool('remember', self.job_request(request_id, action)))

    def _check_envelope(self, result: dict[str, Any]) -> dict[str, Any]:
        if result.get('ok') is not True:
            effect = result.get('effect_state', 'unknown')
            raise ContextClientError('context_operation_failed', effect if effect in ('not_dispatched', 'unknown', 'rejected') else 'unknown', result)
        return result

    def _history_receipt(self, result: dict[str, Any], session_id: str | None = None) -> HistoryReceipt:
        self._check_envelope(result)
        session_id = self.session_id if session_id is None else session_id
        try:
            receipt = result['items'][0]
            if receipt['version'] != 1 or receipt['scope'] != asdict(self.scope) or receipt['session_id'] != session_id:
                raise ContextClientError('invalid_receipt', 'unknown')
            cursor = Cursor(**receipt['cursor'])
        except (KeyError, IndexError, TypeError, ValueError) as error:
            raise ContextClientError('invalid_receipt', 'unknown') from error
        if session_id == self.session_id: self.cursor = cursor
        return receipt

    async def remember(self, content: str, *, kind: Literal['user', 'assistant', 'document'] = 'user') -> dict[str, Any]:
        if kind not in ('user', 'assistant', 'document'): raise ValueError('unsupported message kind')
        result = await self.client.tool('remember', {'conversation': self.conversation, 'content': content, 'kind': kind})
        if result.get('ok') is not True:
            effect = result.get('effect_state', 'unknown')
            raise ContextClientError('remember_failed', effect if effect in ('not_dispatched', 'unknown', 'rejected') else 'unknown', result)
        return result

class EditRelation(TypedDict):
    kind: Literal['edit', 'regenerate']
    id: str
    original_id: str
    replacement_id: str

class TombstoneRelation(TypedDict):
    kind: Literal['tombstone']
    id: str
    source_id: str

SourceRelation = EditRelation | TombstoneRelation

class HistoryReceipt(TypedDict):
    version: int
    scope: dict[str, Any]
    session_id: str
    cursor: dict[str, int]
    last_lsn: int
    replayed: bool
    source_span: SourceSpan | None

class SourceChunk(TypedDict):
    sources: list[SourceMessage]
    spans: list[SourceSpan]
    digest: str

class SummaryTier(TypedDict):
    text: str
    coverage: list[SourceSpan]

class HistorianResult(TypedDict):
    source_digest: str
    tiers: tuple[SummaryTier, SummaryTier, SummaryTier, SummaryTier]

class PendingHistorianState(TypedDict):
    status: Literal['pending']
    ready_at_ms: int

class ClaimedHistorianState(TypedDict):
    status: Literal['claimed']
    worker: str
    expires_at_ms: int

class TerminalHistorianState(TypedDict):
    status: Literal['complete', 'cancelled']

class HistorianJob(TypedDict):
    id: str
    session_id: str
    cursor: dict[str, int]
    policy_revision: int
    chunk: SourceChunk
    attempt: int
    state: PendingHistorianState | ClaimedHistorianState | TerminalHistorianState
    result: HistorianResult | None

class HistorianClaim(TypedDict):
    job: HistorianJob
    worker: str
    attempt: int

JobKind = Literal['historian', 'verification', 'curation', 'extraction', 'indexing', 'consolidation']

class MaintenanceRequest(TypedDict):
    kind: JobKind
    sources: list[SourceMessage]
    cursor: dict[str, int]
    source_revision: int
    policy_revision: int
    reservation: int

class PublicationFence(TypedDict):
    input_digest: str
    source_revision: int
    policy_revision: int

class JobLease(TypedDict):
    job_id: str
    attempt: int
    expires_ms: int
    fence: PublicationFence

KnownUsage = TypedDict('KnownUsage', {'Known': int})
Usage = Literal['Unknown'] | KnownUsage
ExistsPredicate = TypedDict('ExistsPredicate', {'Exists': str})
EqualsValue = TypedDict('EqualsValue', {'key': str, 'value': str})
EqualsPredicate = TypedDict('EqualsPredicate', {'Equals': EqualsValue})
NotPredicate = TypedDict('NotPredicate', {'Not': 'NotePredicate'})
AllPredicate = TypedDict('AllPredicate', {'All': 'list[NotePredicate]'})
AnyPredicate = TypedDict('AnyPredicate', {'Any': 'list[NotePredicate]'})
NotePredicate = Literal['True'] | ExistsPredicate | EqualsPredicate | NotPredicate | AllPredicate | AnyPredicate

class Note(TypedDict):
    id: str
    kind: Literal['Anchor', 'Note', 'Primer']
    revision: int
    text: str
    parents: list[str]
    contradictions: list[str]
    expires_at_ns: int | str | None
    predicate: NotePredicate
    tombstoned: bool

class NoteGrant(TypedDict):
    principal: str
    note_id: str
    read: bool
    write: bool

class AttributeProposal(TypedDict):
    id: str
    key: str
    value: str
    base_revision: int
    proposer: str

CreateNote = TypedDict('CreateNote', {'Create': Note})
ReviseNoteFields = TypedDict('ReviseNoteFields', {'note': Note, 'expected_revision': int})
ReviseNote = TypedDict('ReviseNote', {'Revise': ReviseNoteFields})
TombstoneNoteFields = TypedDict('TombstoneNoteFields', {'id': str, 'expected_revision': int})
TombstoneNote = TypedDict('TombstoneNote', {'Tombstone': TombstoneNoteFields})
SetGrant = TypedDict('SetGrant', {'SetGrant': NoteGrant})
AttributeGrantFields = TypedDict('AttributeGrantFields', {'principal': str, 'key': str, 'write': bool})
SetAttributeGrant = TypedDict('SetAttributeGrant', {'SetAttributeGrant': AttributeGrantFields})
RevokeGrantFields = TypedDict('RevokeGrantFields', {'principal': str, 'note_id': str})
RevokeGrant = TypedDict('RevokeGrant', {'RevokeGrant': RevokeGrantFields})
ProposeAttribute = TypedDict('ProposeAttribute', {'ProposeAttribute': AttributeProposal})
ProposalId = TypedDict('ProposalId', {'proposal_id': str})
AcceptAttribute = TypedDict('AcceptAttribute', {'AcceptAttribute': ProposalId})
RejectAttribute = TypedDict('RejectAttribute', {'RejectAttribute': ProposalId})
NotesCommand = CreateNote | ReviseNote | TombstoneNote | SetGrant | SetAttributeGrant | RevokeGrant | ProposeAttribute | AcceptAttribute | RejectAttribute

class Notes(TypedDict):
    action: Literal['notes']
    command: NotesCommand

class ReadNote(TypedDict):
    action: Literal['read_note']
    id: str
    now_ns: int | str
    facts: dict[str, str]

class ReadAttribute(TypedDict):
    action: Literal['read_attribute']
    key: str

class SetPolicy(TypedDict):
    action: Literal['set_policy']
    session_id: str
    expected_revision: int
    revision: int

class HistorianEnqueue(TypedDict):
    action: Literal['historian_enqueue']
    session_id: str
    cursor: dict[str, int]
    policy_revision: int
    chunk: SourceChunk
    reservation: int
    now_ms: int

class HistorianClaimAction(TypedDict):
    action: Literal['historian_claim']
    worker: str
    now_ms: int
    lease_ms: int

class HistorianHeartbeat(TypedDict):
    action: Literal['historian_heartbeat']
    claim: HistorianClaim
    now_ms: int
    lease_ms: int

class HistorianComplete(TypedDict):
    action: Literal['historian_complete']
    claim: HistorianClaim
    result: HistorianResult
    usage: Usage
    now_ms: int

class HistorianFail(TypedDict):
    action: Literal['historian_fail']
    claim: HistorianClaim
    usage: Usage
    now_ms: int
    cooldown_ms: int

class HistorianCancel(TypedDict):
    action: Literal['historian_cancel']
    id: str

class HistorianExpire(TypedDict):
    action: Literal['historian_expire']
    now_ms: int
    cooldown_ms: int

class MaintenanceEnqueue(TypedDict):
    action: Literal['maintenance_enqueue']
    session_id: str
    request: MaintenanceRequest

class MaintenanceClaim(TypedDict):
    action: Literal['maintenance_claim']
    now_ms: int

class MaintenanceComplete(TypedDict):
    action: Literal['maintenance_complete']
    lease: JobLease
    usage: Usage
    output_digest: str
    now_ms: int

class MaintenanceFail(TypedDict):
    action: Literal['maintenance_fail']
    lease: JobLease
    usage: Usage
    now_ms: int

class MaintenanceCancel(TypedDict):
    action: Literal['maintenance_cancel']
    id: str
    now_ms: int

class MaintenanceSettle(TypedDict):
    action: Literal['maintenance_settle']
    id: str
    attempt: int
    actual: int

class InspectJob(TypedDict):
    action: Literal['inspect']

ContextJobAction = Notes | ReadNote | ReadAttribute | SetPolicy | HistorianEnqueue | HistorianClaimAction | HistorianHeartbeat | HistorianComplete | HistorianFail | HistorianCancel | HistorianExpire | MaintenanceEnqueue | MaintenanceClaim | MaintenanceComplete | MaintenanceFail | MaintenanceCancel | MaintenanceSettle | InspectJob


def encode_context_identifier(value: str) -> str:
    _identifier(value)
    return quote(value, safe='')


def _job_wire(value):
    if isinstance(value, SourceMessage): return value.wire()
    if isinstance(value, dict): return {key: _timestamp_decimal(item, True) if key.endswith("_ns") else _job_wire(item) for key, item in value.items()}
    if isinstance(value, (tuple, list)): return [_job_wire(item) for item in value]
    return value


JsonValue = type(None) | bool | int | float | str | list['JsonValue'] | dict[str, 'JsonValue']

class ImportEntry(TypedDict):
    source_id: str
    kind: str
    digest: str
    payload: JsonValue

class ImportBundle(TypedDict):
    version: int
    import_id: str
    scope: dict[str, Any]
    entries: list[ImportEntry]
    digest: str

class ImportReceipt(TypedDict):
    version: int
    scope: dict[str, Any]
    import_id: str
    bundle_digest: str
    accepted: int
    total: int
    complete: bool
    last_lsn: int


def make_import_bundle(scope: Scope, import_id: str, entries: list[tuple[str, str, JsonValue]]) -> ImportBundle:
    _identifier(import_id)
    wire_entries = []
    for source_id, kind, payload in entries:
        _identifier(source_id)
        canonical = json.loads(json.dumps(payload, sort_keys=True, ensure_ascii=False, allow_nan=False))
        digest = sha256(json.dumps(canonical, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()).hexdigest()
        wire_entries.append({'source_id': source_id, 'kind': kind, 'digest': digest, 'payload': canonical})
    bundle = {'version': 1, 'import_id': import_id, 'scope': asdict(scope), 'entries': wire_entries, 'digest': ''}
    bundle['digest'] = sha256(json.dumps(bundle, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()).hexdigest()
    return bundle


def _timestamp_decimal(value: int | str | None, optional: bool = False) -> str | None:
    if value is None and optional: return None
    if type(value) is int: number = value
    elif isinstance(value, str) and re.fullmatch(r'0|-?[1-9][0-9]*', value): number = int(value)
    else: raise ValueError('invalid timestamp')
    if not -(1 << 63) <= number < (1 << 63): raise ValueError('invalid timestamp')
    return str(number)


RecordKind = Literal['fact', 'episode', 'note', 'conditional_note', 'anchor', 'summary', 'primer']
RecordStatus = Literal['active', 'archived', 'stale', 'tombstoned']

class MemoryProvenance(TypedDict):
    source_id: str
    source_digest: str
    span_start: int
    span_end: int
    quoted_digest: str

class MemoryLineage(TypedDict):
    child_record_id: str
    child_revision: int
    parent_record_id: str
    parent_revision_digest: str
    relation: str
    created_at_ns: int | str

class ConditionClause(TypedDict):
    field: str
    comparison: str
    value: JsonValue

class SmartCondition(TypedDict):
    operator: str
    clauses: list[ConditionClause]

class MemoryRecord(TypedDict):
    id: str
    kind: RecordKind
    category: str
    status: RecordStatus
    revision: int
    revision_digest: str
    content: str
    authority: Authority
    confidence: int
    importance: int
    occurred_at_ns: int | str | None
    recorded_at_ns: int | str
    expires_at_ns: int | str | None
    pinned: bool
    provenance: list[MemoryProvenance]
    lineage: list[MemoryLineage]
    contradictions: list[str]
    last_lsn: int
    predicate: NotePredicate | None
    smart_condition: SmartCondition | None
    retention_until_ns: int | str | None
    metadata: JsonValue

class MemorySource(TypedDict):
    id: str
    digest: str
    content: list[int]
    locator: str
    occurred_at_ns: int | str | None
    recorded_at_ns: int | str
    tombstoned: bool

class MemoryVerification(TypedDict):
    id: str
    record_id: str
    revision_digest: str
    state: Literal['supported', 'contradicted', 'unresolved']
    evidence_source_id: str | None
    confidence: int
    created_at_ns: int | str

class MemoryGrant(TypedDict):
    principal_digest: str | None
    id: str
    principal: dict[str, str | None]
    record_ids: list[str]
    categories: list[str]
    read: bool
    expires_at_ns: int | str | None
    revoked: bool
    revision: int
    record_revisions: dict[str, str]

class CreateMemory(TypedDict):
    kind: Literal['create']
    record: MemoryRecord
class ReviseMemory(TypedDict):
    kind: Literal['revise']
    record: MemoryRecord
    expected_revision: int
class TombstoneMemory(TypedDict):
    kind: Literal['tombstone']
    id: str
    expected_revision: int
class SetMemoryStatus(TypedDict):
    kind: Literal['set_status']
    id: str
    status: RecordStatus
    expected_revision: int
class AddMemorySource(TypedDict):
    kind: Literal['source']
    source: MemorySource
class TombstoneMemorySource(TypedDict):
    kind: Literal['tombstone_source']
    id: str
class VerifyMemory(TypedDict):
    kind: Literal['verify']
    verification: MemoryVerification
class SetMemoryGrant(TypedDict):
    kind: Literal['set_grant']
    grant: MemoryGrant
class RevokeMemoryGrant(TypedDict):
    kind: Literal['revoke_grant']
    id: str
class AddMemoryLineage(TypedDict):
    kind: Literal['lineage']
    lineage: MemoryLineage
MemoryCommand = CreateMemory | ReviseMemory | TombstoneMemory | SetMemoryStatus | AddMemorySource | TombstoneMemorySource | VerifyMemory | SetMemoryGrant | RevokeMemoryGrant | AddMemoryLineage

class MemoryReceipt(TypedDict):
    version: int
    scope: dict[str, str | None]
    cursor: int
    last_lsn: int
    replayed: bool


def seal_memory_record(record: MemoryRecord) -> MemoryRecord:
    keys = ('id', 'kind', 'category', 'status', 'revision', 'revision_digest', 'content', 'authority', 'confidence', 'importance', 'occurred_at_ns', 'recorded_at_ns', 'expires_at_ns', 'pinned', 'provenance', 'lineage', 'contradictions', 'last_lsn', 'predicate', 'smart_condition', 'retention_until_ns', 'metadata')
    value = _job_wire({key: record[key] for key in keys})
    value['provenance'] = [{key: item[key] for key in ('source_id', 'source_digest', 'span_start', 'span_end', 'quoted_digest')} for item in value['provenance']]
    value['lineage'] = [{key: item[key] for key in ('child_record_id', 'child_revision', 'parent_record_id', 'parent_revision_digest', 'relation', 'created_at_ns')} for item in value['lineage']]
    if value['smart_condition'] is not None:
        condition = value['smart_condition']
        value['smart_condition'] = {'operator': condition['operator'], 'clauses': [{'field': c['field'], 'comparison': c['comparison'], 'value': json.loads(json.dumps(c['value'], sort_keys=True))} for c in condition['clauses']]}
    value['revision_digest'] = ''
    last_lsn = value['last_lsn']; value['last_lsn'] = 0
    value['metadata'] = json.loads(json.dumps(value['metadata'], sort_keys=True, ensure_ascii=False, allow_nan=False))
    value['revision_digest'] = sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False, allow_nan=False).encode()).hexdigest()
    value['last_lsn'] = last_lsn
    return value


def make_memory_record(id: str, kind: RecordKind, content: str, recorded_at_ns: int | str) -> MemoryRecord:
    _identifier(id)
    return seal_memory_record({'id': id, 'kind': kind, 'category': 'general', 'status': 'active', 'revision': 1, 'revision_digest': '', 'content': content, 'authority': 'user_asserted', 'confidence': 1000000, 'importance': 500000, 'occurred_at_ns': None, 'recorded_at_ns': recorded_at_ns, 'expires_at_ns': None, 'pinned': False, 'provenance': [], 'lineage': [], 'contradictions': [], 'last_lsn': 0, 'predicate': None, 'smart_condition': None, 'retention_until_ns': None, 'metadata': None})


class MemoryListView(TypedDict):
    version: int
    scope: dict[str, str | None]
    principal: dict[str, str | None]
    cursor: int
    records: list[MemoryRecord]
class MemoryRecordView(TypedDict):
    version: int
    scope: dict[str, str | None]
    record: MemoryRecord
    cursor: int
class MemorySourceView(TypedDict):
    version: int
    scope: dict[str, str | None]
    record_id: str
    source: MemorySource
MemoryView = MemoryListView | MemoryRecordView | MemorySourceView


EvidenceKind = Literal['memory', 'conversation', 'file', 'commit', 'document', 'entity', 'relationship']
class RetrievalSourceRecord(TypedDict):
    scope: dict[str, str | None]
    kind: EvidenceKind
    id: str
    revision: int
    text: str
    content_digest: str
    authority: Authority
    provenance: list[SourceSpan]
    occurred_at_ns: int | str | None
    recorded_at_ns: int | str
    expires_at_ns: int | str | None
    tombstoned: bool
class EvidenceGrant(TypedDict):
    source_scope: dict[str, str | None]
    recipient_scope: dict[str, str | None]
    kind: EvidenceKind
    source_id: str
    source_digest: str
    source_revision: int
    expires_at_ns: int | str
class RegisterRetrieval(TypedDict):
    operation: Literal['retrieval_source']
    expected_revision: int
    source: RetrievalSourceRecord
class TombstoneRetrieval(TypedDict):
    operation: Literal['retrieval_tombstone']
    kind: EvidenceKind
    id: str
    expected_revision: int
class GrantRetrieval(TypedDict):
    operation: Literal['retrieval_grant']
    grant: EvidenceGrant
class RevokeRetrieval(TypedDict):
    operation: Literal['retrieval_revoke']
    recipient_scope: dict[str, str | None]
    kind: EvidenceKind
    id: str
class ConfigureEmbedding(TypedDict):
    operation: Literal['embedding']
    expected_revision: int
    enabled: bool
class BackfillRetrieval(TypedDict):
    operation: Literal['backfill']
    maximum_items: int
    maximum_bytes: int
RetrievalOperation = RegisterRetrieval | TombstoneRetrieval | GrantRetrieval | RevokeRetrieval | ConfigureEmbedding | BackfillRetrieval


class VectorFingerprint(TypedDict):
    model: str
    revision: str
    dimensions: int
class EmbeddingRegistration(TypedDict):
    id: str
    revision: int
    mode: Literal['off', 'local', 'remote_compatible', 'managed']
    fingerprint: VectorFingerprint | None
class BackfillReport(TypedDict):
    registration: EmbeddingRegistration | None
    checkpoint: str | None
    embedded: int
    remaining: int
    unavailable: str | None
    ledger_tail: int
    usage: str

class RestoreMemoryJsonl(TypedDict):
    kind: Literal['restore_jsonl']
    jsonl: list[int]
    artifact_digest: str

class ImportMemoryRows(TypedDict):
    kind: Literal['import_rows']
    import_id: str
    bundle_digest: str
    start: int
    total: int
    entries: list[ImportEntry]
class CommitMemoryImport(TypedDict):
    kind: Literal['commit_import']
    import_id: str
    bundle_digest: str
class MemoryExport(TypedDict):
    version: int
    scope: dict[str, str | None]
    cursor: int
    events: list[MemoryExportEvent]
    digest: str
class RestoreMemoryExport(TypedDict):
    kind: Literal['restore_export']
    export: MemoryExport
MemoryCommand = MemoryCommand | RestoreMemoryJsonl | ImportMemoryRows | CommitMemoryImport | RestoreMemoryExport

class MemoryExportEvent(TypedDict):
    version: int
    scope: dict[str, str | None]
    principal: dict[str, str | None]
    request_id: str
    digest: str
    cursor: int
    command: MemoryCommand

@dataclass(frozen=True)
class MemoryExportArtifact:
    scope: Scope
    cursor: int
    data: bytes
    artifact_digest: str
    export_digest: str
    restore_max_bytes: int
    events: tuple[MemoryExportEvent, ...]


def decode_memory_export(metadata: dict[str, Any], expected_scope: Scope) -> MemoryExportArtifact:
    def invalid(): raise ValueError('invalid memory export artifact')
    if metadata.get('version') != 1 or metadata.get('scope') != asdict(expected_scope) or metadata.get('media_type') != 'application/x-ndjson': invalid()
    encoded = metadata.get('bytes')
    if not isinstance(encoded, list) or len(encoded) > 64 * 1024 * 1024 or any(type(byte) is not int or not 0 <= byte <= 255 for byte in encoded): invalid()
    data = bytes(encoded)
    if type(metadata.get('byte_count')) is not int or metadata['byte_count'] != len(data) or metadata.get('artifact_digest') != sha256(data).hexdigest(): invalid()
    if not data or not data.endswith(b'\n'): invalid()
    try:
        lines = data.decode('utf-8').split('\n')[:-1]
        header = json.loads(lines[0])
    except (UnicodeDecodeError, json.JSONDecodeError, IndexError) as error:
        raise ValueError('invalid memory export encoding') from error
    if not isinstance(header, dict) or set(header) != {'kind', 'version', 'scope', 'cursor', 'event_count', 'digest'} or header['kind'] != 'manifest' or header['version'] != 1 or header['scope'] != asdict(expected_scope): invalid()
    if json.dumps(header, sort_keys=True, ensure_ascii=False, separators=(',', ':'), allow_nan=False) != lines[0]: invalid()
    if type(header['cursor']) is not int or header['cursor'] != len(lines) - 1 or type(header['event_count']) is not int or header['event_count'] != header['cursor'] or metadata.get('cursor') != header['cursor']: invalid()
    events, originals = [], []
    for ordinal, line in enumerate(lines[1:], 1):
        if not line.startswith('{"event":') or not line.endswith(',"kind":"event"}'): invalid()
        raw = line[len('{"event":'):-len(',"kind":"event"}')]
        try: event = json.loads(raw)
        except json.JSONDecodeError as error: raise ValueError('invalid memory export event') from error
        if not isinstance(event, dict) or set(event) != {'version', 'scope', 'principal', 'request_id', 'digest', 'cursor', 'command'} or event['version'] != 1 or event['scope'] != asdict(expected_scope) or event['principal'] != asdict(expected_scope) or type(event['cursor']) is not int or event['cursor'] != ordinal: invalid()
        _identifier(event['request_id'])
        _validate_export_nanoseconds(event['command'])
        events.append(event); originals.append(raw)
    blank = '{"version":1,"scope":' + json.dumps(asdict(expected_scope), separators=(',', ':'), ensure_ascii=False) + ',"cursor":' + str(header['cursor']) + ',"events":[' + ','.join(originals) + '],"digest":""}'
    digest = sha256(blank.encode()).hexdigest()
    if header['digest'] != digest or metadata.get('export_digest') != digest: invalid()
    limit = metadata.get('restore_max_bytes')
    if type(limit) is not int or limit != 524288: invalid()
    return MemoryExportArtifact(expected_scope, header['cursor'], data, metadata['artifact_digest'], digest, limit, tuple(events))


def _validate_export_nanoseconds(value, depth=0):
    if depth > 32: raise ValueError('memory export nesting exceeds capacity')
    if isinstance(value, dict):
        for key, item in value.items():
            if key in ('metadata', 'payload', 'value'): continue
            if key.endswith('_ns'):
                if item is not None and (not isinstance(item, str) or _timestamp_decimal(item) != item): raise ValueError('noncanonical memory export timestamp')
            else: _validate_export_nanoseconds(item, depth + 1)
    elif isinstance(value, list):
        for item in value: _validate_export_nanoseconds(item, depth + 1)
