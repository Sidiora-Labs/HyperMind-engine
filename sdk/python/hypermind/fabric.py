"""Owner-safe operational actions over the existing native tool transport."""
from __future__ import annotations
from typing import TypedDict
from .context import Scope, Cursor, ToolClient, _identifier, _integer
from .development import _OwnerService, NativeResult


class FabricCancellation(TypedDict):
    cancel_requested: bool
    effect_id: str
    effect: NativeResult | None
    worker_stopped: bool
    automatic_replay: bool


class FabricDispatchResult(TypedDict):
    version: int
    scope: NativeResult
    result: NativeResult
    effect: NativeResult | None


class FabricClient(_OwnerService):
    def __init__(self, client: ToolClient, scope: Scope, *, actor: int):
        super().__init__(client, scope, actor=actor, family='fabric')

    async def dispatch_digest(self, request_id: str, content: bytes, *, timeout_ms: int = 30000) -> FabricDispatchResult:
        if type(content) is not bytes: raise ValueError('content must be bytes')
        _integer(timeout_ms, 1)
        if timeout_ms > 30000 or len(content) > 131072: raise ValueError('dispatch bounds')
        return await self._action(request_id, {'action': 'dispatch', 'operation': 'digest', 'bytes': list(content), 'timeout_ms': timeout_ms})

    async def cancel(self, request_id: str, effect_id: str) -> FabricCancellation:
        """Request cancellation; inspect worker_stopped and effect separately.

        A requested cancellation is not a terminal result or permission to
        retry a possibly accepted operation.
        """
        _identifier(effect_id)
        return await self._action(request_id, {'action': 'cancel', 'effect_id': effect_id})

    async def reconcile(self, request_id: str, effect_id: str) -> NativeResult:
        _identifier(effect_id)
        return await self._action(request_id, {'action': 'reconcile', 'effect_id': effect_id})

    async def receipts(self, request_id: str, *, after: Cursor = Cursor(), limit: int = 128) -> NativeResult:
        _integer(limit, 1)
        if limit > 256: raise ValueError('receipt limit')
        return await self._action(request_id, {'action': 'inspect', 'after': {'epoch': after.epoch, 'sequence': after.sequence}, 'limit': limit})
