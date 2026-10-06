"""Owner actions delegated to the native development service."""
from __future__ import annotations
from dataclasses import asdict, dataclass
from typing import Literal, TypeAlias, TypedDict
from .context import Scope, Cursor, ToolClient, _identifier, _integer
from .client import HyperMindError

JSONValue: TypeAlias = bool | int | float | str | None | list['JSONValue'] | dict[str, 'JSONValue']
NativeResult: TypeAlias = dict[str, JSONValue]


@dataclass(frozen=True)
class DevelopmentBudget:
    reserved_tokens: int
    max_input_bytes: int
    max_output_bytes: int
    max_mutations: int

    def __post_init__(self):
        for value in asdict(self).values(): _integer(value, 1)


@dataclass(frozen=True)
class SnapshotRequest:
    capability_id: str
    source_ids: tuple[str, ...] = ()
    record_ids: tuple[str, ...] = ()

    def wire(self):
        _identifier(self.capability_id)
        return {'capability_id': self.capability_id, 'source_ids': _ids(self.source_ids), 'record_ids': _ids(self.record_ids)}


@dataclass(frozen=True)
class ScheduleMode:
    mode: Literal['disabled', 'manual', 'timed']
    interval_ms: int | None = None

    def wire(self):
        if self.mode not in ('disabled', 'manual', 'timed'): raise ValueError('schedule mode')
        if self.mode == 'timed':
            _integer(self.interval_ms, 1)
            return {'mode': self.mode, 'interval_ms': self.interval_ms}
        if self.interval_ms is not None: raise ValueError('interval requires timed mode')
        return {'mode': self.mode}


@dataclass(frozen=True)
class ServiceSchedule:
    id: str
    mode: ScheduleMode
    worker_id: str
    snapshot: SnapshotRequest
    reservation: int
    timeout_ms: int
    backoff_ms: int = 0
    max_attempts: int = 1
    identical_failure_limit: int = 1

    def wire(self):
        _identifier(self.id); _identifier(self.worker_id)
        for v in (self.reservation, self.timeout_ms, self.max_attempts, self.identical_failure_limit): _integer(v, 1)
        _integer(self.backoff_ms)
        if self.timeout_ms > 60000 or self.max_attempts > 16: raise ValueError('schedule bounds')
        return {**asdict(self), 'mode': self.mode.wire(), 'snapshot': self.snapshot.wire()}


@dataclass(frozen=True)
class ProposalDecision:
    proposal_id: str
    expected_revision: int
    expected_digest: str
    decision: Literal['accept', 'reject']

    def wire(self, request_id: str):
        _identifier(self.proposal_id); _integer(self.expected_revision, 1)
        if self.decision not in ('accept', 'reject'): raise ValueError('proposal decision')
        if len(self.expected_digest) != 64 or any(c not in '0123456789abcdef' for c in self.expected_digest): raise ValueError('proposal digest')
        return {'request_id': request_id, **asdict(self)}


class ProfileInspection(TypedDict):
    enabled: bool
    proposals: list[NativeResult]
    source_count: int


class PrimerInspection(TypedDict):
    primers: list[NativeResult]
    pending: list[NativeResult]


def _ids(values):
    for value in values: _identifier(value)
    return sorted(set(values))


class _OwnerService:
    def __init__(self, client: ToolClient, scope: Scope, *, actor: int, family: str):
        _integer(actor, 1)
        if actor > 65535: raise ValueError('actor bounds')
        self.client, self.scope, self.actor, self.family = client, scope, actor, family

    def _accept(self, envelope, *, mutation: bool) -> NativeResult:
        if not isinstance(envelope, dict): raise HyperMindError('kProtocolInvalid', effect_state='unknown' if mutation else None)
        items = envelope.get('items')
        if envelope.get('ok') is not True:
            detail = items[0] if isinstance(items, list) and items and isinstance(items[0], dict) else {}
            effect = envelope.get('effect_state')
            if mutation and effect not in ('not_dispatched', 'rejected'): effect = 'unknown'
            raise HyperMindError(detail.get('error', 'kOperationUnavailable'), effect_state=effect)
        if not isinstance(items, list) or not items or not isinstance(items[0], dict):
            raise HyperMindError('kProtocolInvalid', effect_state='unknown' if mutation else None)
        result = items[0]
        def validate(value):
            if isinstance(value, dict):
                if 'scope' in value and value['scope'] != asdict(self.scope): raise HyperMindError('kCapabilityDenied', effect_state='unknown' if mutation else None)
                if 'version' in value and (type(value['version']) is not int or value['version'] != 1): raise HyperMindError('kProtocolVersion', effect_state='unknown' if mutation else None)
                for key, child in value.items():
                    if key == 'cursor':
                        if isinstance(child, dict): Cursor(**child)
                        elif isinstance(child, int): _integer(child)
                    # Native families may contain nested source metadata with
                    # independent version/scope contracts. Only envelope and
                    # known mutation receipts establish this request's scope.
            else: raise HyperMindError('kProtocolInvalid', effect_state='unknown' if mutation else None)
        try:
            validate(result)
            if isinstance(result.get('receipt'), dict): validate(result['receipt'])
        except (TypeError, ValueError) as error:
            raise HyperMindError('kProtocolInvalid', effect_state='unknown' if mutation else None) from error
        return result

    async def _action(self, request_id: str, action: NativeResult) -> NativeResult:
        _identifier(request_id)
        request = {'version': 1, 'scope': asdict(self.scope), 'request_id': request_id, 'action': action}
        result = await self.client.tool('remember', {'conversation': 'owner-service', 'content': '', 'kind': 'user', 'context': {'operation': self.family, 'request': request}})
        return self._accept(result, mutation=True)

    async def inspect(self) -> NativeResult:
        result = await self.client.tool('inspect', {'uri': f'hm://{self.actor}/context-{self.family}'})
        return self._accept(result, mutation=False)


class DevelopmentClient(_OwnerService):
    def __init__(self, client: ToolClient, scope: Scope, *, actor: int):
        super().__init__(client, scope, actor=actor, family='development')

    async def set_profile_enabled(self, request_id: str, enabled: bool) -> NativeResult:
        if type(enabled) is not bool: raise ValueError('enabled must be bool')
        return await self._action(request_id, {'action': 'profile_set_enabled', 'enabled': enabled})

    async def collect_profile_session(self, request_id: str, session_id: str, conversation: str) -> tuple[str, ...]:
        return await self._collect(request_id, 'profile_collect_session', session_id, conversation)

    async def collect_primer_session(self, request_id: str, session_id: str, conversation: str) -> tuple[str, ...]:
        return await self._collect(request_id, 'primer_collect_session', session_id, conversation)

    async def _collect(self, request_id, action, session_id, conversation):
        _identifier(session_id); _identifier(conversation)
        result = await self._action(request_id, {'action': action, 'session_id': session_id, 'conversation': conversation})
        values = result.get('source_ids')
        if not isinstance(values, list) or any(not isinstance(v, str) for v in values): raise HyperMindError('kProtocolInvalid', effect_state='unknown')
        return tuple(values)

    async def inspect_profile(self, request_id: str) -> ProfileInspection:
        return await self._action(request_id, {'action': 'profile_inspect'})

    async def inspect_primers(self, request_id: str) -> PrimerInspection:
        return await self._action(request_id, {'action': 'primer_inspect'})

    async def enqueue_primer(self, request_id: str, job_id: str, target_id: str, *, refresh: bool = False) -> NativeResult:
        _identifier(job_id); _identifier(target_id)
        if type(refresh) is not bool: raise ValueError('refresh must be bool')
        return await self._action(request_id, {'action': 'primer_enqueue', 'job_id': job_id, 'target_id': target_id, 'refresh': refresh})

    async def register_worker(self, request_id: str, *, worker_id: str, capability_id: str, session_id: str, conversation: str,
                              budget: DevelopmentBudget, lease_ms: int, source_ids: tuple[str, ...] = (),
                              record_ids: tuple[str, ...] = (), new_record_ids: tuple[str, ...] = ()) -> NativeResult:
        for identifier in (worker_id, capability_id, session_id, conversation): _identifier(identifier)
        _integer(lease_ms, 1)
        if lease_ms > 3600000: raise ValueError('lease bounds')
        return await self._action(request_id, {'action': 'register', 'worker_id': worker_id, 'capability_id': capability_id,
            'session_id': session_id, 'conversation': conversation, 'budget': asdict(budget), 'lease_ms': lease_ms,
            'source_ids': _ids(source_ids), 'record_ids': _ids(record_ids), 'new_record_ids': _ids(new_record_ids)})

    async def revoke_worker(self, request_id: str, capability_id: str, expected_revision: int) -> NativeResult:
        _identifier(capability_id); _integer(expected_revision, 1)
        return await self._action(request_id, {'action': 'revoke', 'capability_id': capability_id, 'expected_revision': expected_revision})

    async def configure_schedule(self, request_id: str, schedule: ServiceSchedule) -> NativeResult:
        return await self._action(request_id, {'action': 'configure', 'schedule': schedule.wire()})

    async def enqueue(self, request_id: str, schedule_id: str) -> NativeResult:
        _identifier(schedule_id)
        return await self._action(request_id, {'action': 'enqueue', 'schedule_id': schedule_id})

    async def dispatch(self, request_id: str, job_id: str) -> NativeResult:
        _identifier(job_id)
        return await self._action(request_id, {'action': 'dispatch', 'job_id': job_id})

    async def cancel(self, request_id: str, job_id: str) -> NativeResult:
        _identifier(job_id)
        return await self._action(request_id, {'action': 'cancel', 'job_id': job_id})

    async def review(self, request_id: str, decision: ProposalDecision) -> NativeResult:
        return await self._action(request_id, {'action': 'review', 'decision': decision.wire(request_id)})
