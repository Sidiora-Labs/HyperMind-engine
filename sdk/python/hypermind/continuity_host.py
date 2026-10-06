from __future__ import annotations

import asyncio
from dataclasses import asdict
from hashlib import sha256
import json
from typing import Literal
from uuid import uuid4

from .context import ContextClient, ContextClientError, TokenBudget
from .gideon import GideonContextAdapter

ContinuityMode = Literal['off', 'pass_through', 'shadow', 'primary']


class ContextContinuityClient:
    def __init__(self, context: ContextClient):
        self.context = context

    async def generation(self) -> int:
        result = self.context._check_envelope(await self.context.client.tool('inspect', {'uri': self.context.inspect_uri()}))
        report = result['items'][0]['report']
        if report['version'] != 1 or report['scope'] != asdict(self.context.scope) or report['session_id'] != self.context.session_id:
            raise ContextClientError('continuity_binding_mismatch', 'rejected')
        generation = report['generation']
        if type(generation) is not int or generation < 1:
            raise ContextClientError('continuity_generation_missing', 'rejected')
        return generation

    async def call(self, action: dict, *, generation: int, request_id: str | None = None) -> dict:
        context = self.context
        request = {'version': 1, 'scope': asdict(context.scope), 'session_id': context.session_id,
                   'request_id': request_id or 'continuity-' + uuid4().hex,
                   'expected_generation': generation, 'action': action}
        result = context._check_envelope(await context.client.tool('remember', {
            'conversation': context.conversation, 'content': '', 'kind': 'user',
            'context': {'operation': 'continuity', 'request': request}}))
        return result['items'][0]


class GideonContinuityAdapter(GideonContextAdapter):
    name = 'hypermind-continuity'
    fail_closed = True

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.continuity = ContextContinuityClient(self.context)
        self.mode: ContinuityMode = 'off'
        self.strict = False
        self._pending_mode: tuple[ContinuityMode, bool] | None = None
        self._turn_active = False
        self._dispatching = False
        self._ticket: dict | None = None
        self._generation = 0
        self.comparison: dict | None = None
        self._provider_binding: str | None = None

    @property
    def owns_compaction(self) -> bool:
        return self.mode == 'primary' and super().owns_compaction

    def queue_mode(self, mode: ContinuityMode, *, strict: bool = False) -> None:
        if mode not in ('off', 'pass_through', 'shadow', 'primary') or type(strict) is not bool:
            raise ValueError('invalid continuity mode')
        if mode == 'primary':
            if self.context_owner != 'hypermind':
                raise ValueError('primary requires explicit native context ownership')
            from gideon.cognition import context_engine
            guard = getattr(context_engine, '_fails_closed', None)
            if not callable(guard) or not guard(self):
                raise ValueError('host runtime does not support primary refusal authority')
        self._pending_mode = (mode, strict)

    def _binding(self) -> str:
        return sha256(json.dumps({'provider': asdict(self.provider), 'budget': asdict(self.budget),
                                  'owner': self.context_owner}, sort_keys=True, separators=(',', ':')).encode()).hexdigest()

    async def prepare_turn(self, session_key: str) -> None:
        from gideon.cognition.context_engine import ContextBoundaryRefusal
        if session_key != self.session_key or self._turn_active:
            raise ContextBoundaryRefusal('continuity turn must reach an idle boundary')
        if self._pending_mode is not None:
            generation = await self.continuity.generation()
            state = await self.continuity.call({'action': 'inspect'}, generation=generation)
            mode, strict = self._pending_mode
            result = await self.continuity.call({'action': 'configure', 'mode': mode, 'strict': strict,
                                                 'expected_revision': state['policy']['revision']}, generation=generation)
            if result.get('turn_boundary') is not True or result['policy']['mode'] != mode:
                raise ContextBoundaryRefusal('native continuity mode was not accepted at boundary')
            self.mode, self.strict, self._pending_mode = mode, strict, None
        self.comparison = None
        self._ticket = None
        if self.mode in ('off', 'pass_through'):
            self._turn_active = True
            return
        if self.mode == 'primary':
            await super().prepare_turn(session_key)
        generation = await self.continuity.generation()
        action = 'pre_hook' if self.mode == 'primary' else 'pressure'
        result = await self.continuity.call({'action': action, 'budget': asdict(self.budget), 'required_ids': []}, generation=generation)
        if result.get('mode') != self.mode or not isinstance(result.get('plan'), dict):
            raise ContextBoundaryRefusal('native continuity mode or verified projection unavailable')
        plan = result['plan']
        report = plan['rendered']['report']
        if report['scope'] != asdict(self.context.scope) or report['session_id'] != self.context.session_id or report['generation'] != generation:
            raise ContextBoundaryRefusal('continuity projection binding differs from host')
        if report['token_count'] > self.budget.available:
            raise ContextBoundaryRefusal('continuity projection exceeds output reservation')
        if self.mode == 'shadow':
            if result.get('published') is not False:
                raise ContextBoundaryRefusal('shadow comparison published state')
            self.comparison = {'native_digest': plan['rendered']['digest'], 'native_tokens': report['token_count'],
                               'generation': generation, 'published': False}
        else:
            self._validate_messages(plan['rendered']['messages'])
            self.diagnostics = plan['rendered']
            self._ticket = result['ticket']
            self._prepared_digest = self._digest(self._events())
            self._provider_binding = self._binding()
        self._generation = generation
        self._turn_active = True

    def assemble(self, builder, text: str, *, is_new_session: bool, **kwargs):
        from gideon.cognition.context_engine import ContextBoundaryRefusal
        if not self._turn_active or kwargs.get('session_key') != self.session_key:
            raise ContextBoundaryRefusal('prepare the bound continuity turn before assembly')
        if self.mode != 'primary':
            result = self.delegate.assemble(builder, text, is_new_session=is_new_session, **kwargs)
            if self.comparison is not None:
                self.comparison = {**self.comparison, 'host_digest': sha256(result.message.encode()).hexdigest(),
                                   'host_bytes': len(result.message.encode())}
            return result
        if not self._dispatching or self._ticket is None or self._binding() != self._provider_binding:
            raise ContextBoundaryRefusal('primary requires the validated asynchronous dispatch boundary')
        return super().assemble(builder, text, is_new_session=is_new_session, **kwargs)

    async def assemble_for_dispatch(self, builder, text: str, *, is_new_session: bool, **kwargs):
        from gideon.cognition.context_engine import assemble_context, get_engine, ContextBoundaryRefusal
        if get_engine() is not self:
            raise ContextBoundaryRefusal('continuity adapter is no longer installed')
        self._dispatching = True
        try:
            result = assemble_context(builder, text, is_new_session=is_new_session, **kwargs)
            if get_engine() is not self:
                raise ContextBoundaryRefusal('host context authority changed during assembly')
            if self.mode == 'primary':
                if self._prepared_digest != self._digest(self._events()) or self._binding() != self._provider_binding:
                    raise ContextBoundaryRefusal('host transcript or provider changed during hooks')
                validated = await self.continuity.call({'action': 'post_hook', 'ticket': self._ticket}, generation=self._generation)
                if validated.get('mode') != 'primary' or validated['receipt']['rendered_digest'] != self._ticket['rendered_digest']:
                    raise ContextBoundaryRefusal('post-hook native continuity receipt missing')
                if self._prepared_digest != self._digest(self._events()) or self._binding() != self._provider_binding:
                    raise ContextBoundaryRefusal('host transcript or provider changed before dispatch')
                self._ticket = None
            return result
        except BaseException:
            if self._ticket is not None:
                try:
                    await asyncio.shield(self.cancel_turn())
                except BaseException:
                    pass
            raise
        finally:
            self._dispatching = False

    async def cancel_turn(self) -> None:
        if self._ticket is not None:
            await self.continuity.call({'action': 'cancel_hook', 'trace_id': self._ticket['trace_id']}, generation=await self.continuity.generation())
            self._ticket = None
        self._turn_active = False
        self._prepared_digest = None

    def after_turn(self, session_key: str) -> None:
        if self._ticket is not None:
            raise RuntimeError('cancel or validate the native hook before completing the turn')
        self._turn_active = False
        super().after_turn(session_key)
