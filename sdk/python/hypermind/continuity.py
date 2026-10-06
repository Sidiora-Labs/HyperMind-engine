from __future__ import annotations

import asyncio
from dataclasses import dataclass
from typing import Awaitable, Callable, Literal, Any

from .context import ActivationOptions, ContextClient, ContextClientError, ContextDiagnostics, Cursor, EffectState, SourceMessage, TokenBudget, MemoryCommand, ImportBundle, SourceRelation, ContextJobAction, HistoryReceipt, MemoryReceipt, ImportReceipt, Scope


@dataclass(frozen=True)
class ContinuationState:
    cursor: Cursor
    generation: int
    model_id: str | None
    profile: tuple[tuple[str, bool], ...] | None


@dataclass(frozen=True)
class MutationOutcome:
    operation: str
    effect_state: EffectState
    identity: str
    detail: str


@dataclass(frozen=True)
class ContinuationCheckpoint:
    scope: Scope
    actor: int
    session_id: str
    conversation: str
    context_owner: str
    accepted: ContinuationState | None
    uncertain: MutationOutcome | None
    unresolved: tuple[MutationOutcome, ...]


class ContinuationError(ContextClientError):
    pass


class ContextContinuation:
    def __init__(self, context: ContextClient, checkpoint: ContinuationCheckpoint | None = None):
        self.context = context
        self.accepted: ContinuationState | None = None
        self.uncertain: MutationOutcome | None = None
        self.generation_compatible = False
        self.unresolved: tuple[MutationOutcome, ...] = ()
        self._lock = asyncio.Lock()
        if checkpoint is not None:
            if (checkpoint.scope, checkpoint.actor, checkpoint.session_id, checkpoint.conversation, checkpoint.context_owner) != (context.scope, context.actor, context.session_id, context.conversation, context.context_owner):
                raise ContinuationError('checkpoint_identity_mismatch', 'not_dispatched')
            self.accepted, self.uncertain, self.unresolved = checkpoint.accepted, checkpoint.uncertain, checkpoint.unresolved
            if self.accepted: context.cursor = self.accepted.cursor

    def checkpoint(self) -> ContinuationCheckpoint:
        context = self.context
        return ContinuationCheckpoint(context.scope, context.actor, context.session_id, context.conversation, context.context_owner, self.accepted, self.uncertain, self.unresolved)

    def _accept(self, diagnostics: ContextDiagnostics, model_id: str | None, profile: tuple[tuple[str, bool], ...] | None) -> ContextDiagnostics:
        report = diagnostics['report']
        generation = report['generation']
        if type(generation) is not int or not 0 <= generation <= (1 << 53) - 1:
            raise ContinuationError('invalid_generation', 'unknown')
        self.accepted = ContinuationState(self.context.cursor, generation, model_id, profile)
        self.generation_compatible = True
        return diagnostics

    def _writable(self) -> None:
        if self.uncertain is not None:
            raise ContinuationError('uncertain_mutation_requires_reconciliation', 'not_dispatched', self.uncertain)

    async def _mutation(self, operation: str, identity: str, invoke: Callable[[], Awaitable[Any]]) -> Any:
        async with self._lock:
            self._writable()
            if any(outcome.operation == operation and outcome.identity == identity for outcome in self.unresolved):
                raise ContinuationError('unresolved_mutation_identity', 'not_dispatched')
            try:
                return await invoke()
            except BaseException as error:
                effect = getattr(error, 'effect_state', 'unknown')
                if effect not in ('not_dispatched', 'rejected'): effect = 'unknown'
                if effect == 'unknown':
                    self.uncertain = MutationOutcome(operation, effect, identity, type(error).__name__)
                if isinstance(error, asyncio.CancelledError): raise
                raise ContinuationError(operation + '_failed', effect, error) from error

    async def abandon_uncertain(self) -> MutationOutcome:
        async with self._lock:
            if self.uncertain is None: raise ValueError('no uncertain mutation')
            diagnostics = await self.context.inspect()
            self._accept(diagnostics, self.accepted.model_id if self.accepted else None, self.accepted.profile if self.accepted else None)
            outcome = self.uncertain
            self.unresolved += (outcome,)
            self.uncertain = None
            self.generation_compatible = False
            return outcome

    async def reconcile_source(self, message: SourceMessage) -> bool:
        async with self._lock:
            if self.uncertain is None or self.uncertain.operation != 'source' or self.uncertain.identity != message.id:
                raise ValueError('uncertain source identity required')
            envelope = self.context._check_envelope(await self.context.client.tool('inspect', {'uri': self.context.inspect_uri() + '/history'}))
            if not any(item.get('id') == message.id and item.get('source_digest') == message.freeze().source_digest for item in envelope['items'][0]['messages']): return False
            diagnostics = await self.context.inspect()
            self._accept(diagnostics, self.accepted.model_id if self.accepted else None, self.accepted.profile if self.accepted else None)
            self.uncertain = None
            self.generation_compatible = False
            return True

    async def activate(self, query: str, budget: TokenBudget, *, turn_text: str = '', options: ActivationOptions | None = None) -> ContextDiagnostics:
        selected = dict(options or {})
        model = selected.get('model_id', 'gpt-4o')
        profile = tuple(sorted(selected['profile'].items())) if 'profile' in selected else None
        if self.accepted and (model != self.accepted.model_id or profile != self.accepted.profile):
            self.generation_compatible = False
        generation = self.accepted.generation if self.accepted and self.generation_compatible else 0
        diagnostics = await self._mutation('activate', 'context-generation', lambda: self.context.activate(query, budget, turn_text=turn_text, generation=generation, options=selected))
        return self._accept(diagnostics, model, profile)

    async def inspect(self) -> ContextDiagnostics:
        async with self._lock:
            diagnostics = await self.context.inspect()
            model = self.accepted.model_id if self.accepted else None
            profile = self.accepted.profile if self.accepted else None
            compatible = self.generation_compatible
            result = self._accept(diagnostics, model, profile)
            self.generation_compatible = compatible
            return result

    async def reconnect(self, factory: Callable[[], Awaitable[ContextClient]]) -> ContextDiagnostics:
        async with self._lock:
            fresh = await factory()
            if (fresh.scope, fresh.actor, fresh.session_id, fresh.conversation, fresh.context_owner) != (self.context.scope, self.context.actor, self.context.session_id, self.context.conversation, self.context.context_owner):
                raise ContinuationError('reconnect_identity_mismatch', 'not_dispatched')
            fresh.cursor = self.accepted.cursor if self.accepted else self.context.cursor
            diagnostics = await fresh.inspect()
            previous = self.accepted
            self.context = fresh
            self._accept(diagnostics, previous.model_id if previous else None, previous.profile if previous else None)
            self.generation_compatible = False
            return diagnostics

    async def close(self) -> None:
        async with self._lock:
            await self.context.client.close()
            self.generation_compatible = False

    async def ingest_source(self, message: SourceMessage, original_bytes: bytes):
        return await self._mutation('source', message.id, lambda: self.context.ingest_source(message, original_bytes))

    async def relate(self, relation: SourceRelation) -> HistoryReceipt:
        return await self._mutation('relation', relation['id'], lambda: self.context.relate(relation))

    async def fork(self, child_session_id: str, child_conversation: str | None = None):
        return await self._mutation('fork', child_session_id, lambda: self.context.fork(child_session_id, child_conversation))

    async def memory(self, request_id: str, command: MemoryCommand):
        return await self._mutation('memory', request_id, lambda: self.context.memory(request_id, command))

    async def job(self, request_id: str, action: ContextJobAction) -> dict[str, Any]:
        return await self._mutation('job', request_id, lambda: self.context.job(request_id, action))

    async def import_bundle(self, bundle: ImportBundle, max_entries: int = 128):
        return await self._mutation('import', bundle['import_id'], lambda: self.context.import_bundle(bundle, max_entries))
