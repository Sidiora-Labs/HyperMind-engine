"""Exact owner-scoped usage inspection; native accounting remains authoritative."""
from __future__ import annotations
from dataclasses import asdict
import re
from typing import TypedDict, NotRequired, Literal
from .context import Scope, ToolClient, _identifier
from .client import HyperMindError


class UsageAttribution(TypedDict):
    job_id: str
    worker_id: str
    session_id: str
    turn_id: str
    provider_id: str
    model_id: str
    source_ids: list[str]


class ExactPrice(TypedDict):
    nanodollar_numerator: int
    denominator: int
    unit: str


class MonetaryAmount(TypedDict):
    negative: bool
    magnitude: ExactPrice


class UsageTokens(TypedDict):
    input: NotRequired[int | None]
    output: NotRequired[int | None]
    cache_read: NotRequired[int | None]
    cache_write: NotRequired[int | None]
    reasoning: NotRequired[int | None]
    input_semantics: Literal['includes_cache', 'excludes_cache', 'unknown']


class UsageQuotaCount(TypedDict):
    kind: Literal['count']
    value: int


class UsageQuotaMoney(TypedDict):
    kind: Literal['usd']
    value: MonetaryAmount


class UsageWindow(TypedDict):
    name: str
    unit: str
    limit: UsageQuotaCount | UsageQuotaMoney | None
    remaining: UsageQuotaCount | UsageQuotaMoney | None
    used: UsageQuotaCount | UsageQuotaMoney | None
    refill_amount: UsageQuotaCount | UsageQuotaMoney | None
    starts_at_ns: int | None
    resets_at_ns: int | None
    interval_ms: int | None


class UsageFunding(TypedDict):
    credits: MonetaryAmount | None
    spent: MonetaryAmount | None


class ProviderUsage(TypedDict):
    provider_id: str
    format: Literal['open_ai', 'anthropic', 'ollama', 'account_quota']
    observed_at_ns: int
    expires_at_ns: int | None
    tokens: UsageTokens
    quota_windows: list[UsageWindow]
    balance: MonetaryAmount | None
    reported_charge: MonetaryAmount | None
    funding: UsageFunding | None
    error_observed: bool
    evidence_digest: str


class UsageCatalogCharge(TypedDict):
    basis: str
    model_id: str
    tier_index: int | None
    nanodollars: ExactPrice | None
    unknown: list[str]


class UsageReservation(TypedDict):
    id: str
    attribution: UsageAttribution
    reserved_tokens: int
    reserved_at_ms: int
    attempt: int
    expires_ms: int
    dispatched: bool
    observation_id: str | None
    settled_tokens: int | None


class UsageObservation(TypedDict):
    id: str
    reservation_id: str
    attribution: UsageAttribution
    accepted_response: bool
    evidence_digest: str
    provider_usage: ProviderUsage
    catalog_estimate: UsageCatalogCharge | None


class UsageLimits(TypedDict):
    total_tokens: int
    hourly_tokens: int
    daily_tokens: int
    job_tokens: int
    concurrency: int
    lease_ms: int


class UsageState(TypedDict):
    limits: UsageLimits
    spent: int
    unknown_usage: dict[str, int]
    reservations: list[UsageReservation]
    observations: list[UsageObservation]


class UsageRollup(TypedDict):
    known_tokens: int
    held_tokens: int
    unknown_reservations: int
    active_reservations: int
    observed_calls: int
    attributions: list[UsageAttribution]
    observations: list[str]


class UsageView(TypedDict):
    version: Literal[1]
    scope: dict[str, str | None]
    rollup: UsageRollup
    state: UsageState


class UsageDecodeError(ValueError): pass


def _exact(value, *, signed=False):
    low, high = (-(1 << 63), (1 << 63) - 1) if signed else (0, (1 << 64) - 1)
    if type(value) is int:
        if abs(value) > (1 << 53) - 1: raise UsageDecodeError('unsafe numeric usage quantity')
        number = value
    elif isinstance(value, str) and len(value) <= 20 and re.fullmatch(r'0|[1-9][0-9]*|-[1-9][0-9]*', value):
        number = int(value)
    else: raise UsageDecodeError('noncanonical usage quantity')
    if not low <= number <= high: raise UsageDecodeError('usage quantity outside native range')
    return number


def _decode(value, schema):
    if schema in ('u64', 'i64'): return _exact(value, signed=schema == 'i64')
    if schema == 'str':
        if not isinstance(value, str): raise UsageDecodeError('expected usage text')
        return value
    if schema == 'bool':
        if type(value) is not bool: raise UsageDecodeError('expected usage boolean')
        return value
    if schema == 'index':
        if type(value) is not int or not 0 <= value <= (1 << 53) - 1: raise UsageDecodeError('usage index')
        return value
    if isinstance(schema, tuple):
        kind, inner = schema
        if kind == 'null': return None if value is None else _decode(value, inner)
        if kind == 'list':
            if not isinstance(value, list): raise UsageDecodeError('expected usage list')
            return [_decode(v, inner) for v in value]
        if kind == 'map':
            if not isinstance(value, dict): raise UsageDecodeError('expected usage map')
            return {k: _decode(v, inner) for k, v in value.items()}
        if kind == 'enum':
            if value not in inner: raise UsageDecodeError('usage enum')
            return value
    if isinstance(schema, dict):
        if not isinstance(value, dict) or set(value) - set(schema): raise UsageDecodeError('usage object fields')
        output = {}
        for key, field in schema.items():
            optional = isinstance(field, tuple) and field[0] == 'optional'
            if key not in value:
                if optional: continue
                raise UsageDecodeError('missing usage field ' + key)
            output[key] = _decode(value[key], field[1] if optional else field)
        return output
    if callable(schema): return schema(value)
    raise UsageDecodeError('usage schema')


def _quota(value):
    if not isinstance(value, dict) or value.get('kind') not in ('count', 'usd'): raise UsageDecodeError('quota amount')
    return _decode(value, {'kind': ('enum', ('count', 'usd')), 'value': 'u64' if value['kind'] == 'count' else MONEY})


ATTRIBUTION = {**{k: 'str' for k in ('job_id', 'worker_id', 'session_id', 'turn_id', 'provider_id', 'model_id')}, 'source_ids': ('list', 'str')}
PRICE = {'nanodollar_numerator': 'u64', 'denominator': 'u64', 'unit': 'str'}
MONEY = {'negative': 'bool', 'magnitude': PRICE}
TOKENS = {**{k: ('optional', ('null', 'u64')) for k in ('input', 'output', 'cache_read', 'cache_write', 'reasoning')}, 'input_semantics': ('enum', ('includes_cache', 'excludes_cache', 'unknown'))}
WINDOW = {'name': 'str', 'unit': 'str', **{k: ('null', _quota) for k in ('limit', 'remaining', 'used', 'refill_amount')}, 'starts_at_ns': ('null', 'i64'), 'resets_at_ns': ('null', 'i64'), 'interval_ms': ('null', 'u64')}
SNAPSHOT = {'provider_id': 'str', 'format': ('enum', ('open_ai', 'anthropic', 'ollama', 'account_quota')), 'observed_at_ns': 'i64', 'expires_at_ns': ('null', 'i64'), 'tokens': TOKENS,
    'quota_windows': ('list', WINDOW), 'balance': ('null', MONEY), 'reported_charge': ('null', MONEY), 'funding': ('null', {'credits': ('null', MONEY), 'spent': ('null', MONEY)}), 'error_observed': 'bool', 'evidence_digest': 'str'}
CATALOG = {'basis': 'str', 'model_id': 'str', 'tier_index': ('null', 'index'), 'nanodollars': ('null', PRICE), 'unknown': ('list', 'str')}
RESERVATION = {'id': 'str', 'attribution': ATTRIBUTION, **{k: 'u64' for k in ('reserved_tokens', 'reserved_at_ms', 'attempt', 'expires_ms')}, 'dispatched': 'bool', 'observation_id': ('null', 'str'), 'settled_tokens': ('null', 'u64')}
OBSERVATION = {'id': 'str', 'reservation_id': 'str', 'attribution': ATTRIBUTION, 'accepted_response': 'bool', 'evidence_digest': 'str', 'provider_usage': SNAPSHOT, 'catalog_estimate': ('null', CATALOG)}
VIEW = {'version': 'index', 'scope': {'owner_id': 'str', 'project_id': 'str', 'workspace_id': ('null', 'str')},
    'rollup': {**{k: 'u64' for k in ('known_tokens', 'held_tokens', 'unknown_reservations', 'active_reservations', 'observed_calls')}, 'attributions': ('list', ATTRIBUTION), 'observations': ('list', 'str')},
    'state': {'limits': {k: 'u64' for k in ('total_tokens', 'hourly_tokens', 'daily_tokens', 'job_tokens', 'concurrency', 'lease_ms')}, 'spent': 'u64', 'unknown_usage': ('map', 'u64'), 'reservations': ('list', RESERVATION), 'observations': ('list', OBSERVATION)}}


def decode_usage_view(value, *, scope: Scope | None = None) -> UsageView:
    view = _decode(value, VIEW)
    if view['version'] != 1: raise UsageDecodeError('unsupported usage version')
    decoded_scope = Scope(**view['scope'])
    if scope is not None and decoded_scope != scope: raise UsageDecodeError('usage scope mismatch')
    for attribution in view['rollup']['attributions'] + [r['attribution'] for r in view['state']['reservations']] + [r['attribution'] for r in view['state']['observations']]:
        for key, identifier in attribution.items():
            for item in identifier if key == 'source_ids' else [identifier]: _identifier(item)
    return view


def decode_provider_usage(value) -> ProviderUsage:
    return _decode(value, SNAPSHOT)


class UsageClient:
    def __init__(self, client: ToolClient, scope: Scope, *, actor: int):
        if type(actor) is not int or not 1 <= actor <= 65535: raise ValueError('usage actor')
        self.client, self.scope, self.actor = client, scope, actor

    async def inspect(self) -> UsageView:
        envelope = await self.client.tool('inspect', {'uri': f'hm://{self.actor}/context-usage'})
        if not isinstance(envelope, dict) or envelope.get('ok') is not True:
            detail = (envelope.get('items') or [{}])[0] if isinstance(envelope, dict) else {}
            raise HyperMindError(detail.get('error', 'kOperationUnavailable'))
        try: return decode_usage_view(envelope['items'][0], scope=self.scope)
        except (KeyError, IndexError, TypeError, ValueError) as error:
            raise HyperMindError('kProtocolInvalid', 'invalid usage view') from error
