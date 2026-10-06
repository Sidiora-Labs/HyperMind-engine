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
        self._pending_input: str | None = None
        self._tokenizer_identity: dict | None = None
        self.final_input_count: dict | None = None
        self.final_chat_count: dict | None = None

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
            if not callable(guard) or not guard(self) or getattr(context_engine, 'HYPERMIND_CONTINUITY_INTEGRATION', None) != 1:
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
            identity = getattr(self.context.client, 'tokenizer_identity', None)
            count = getattr(self.context.client, 'count_provider_input', None)
            if not callable(identity) or not callable(count):
                raise ContextBoundaryRefusal('host transport has no trusted final-input tokenizer')
            self._tokenizer_identity = await identity(self.provider.model_id)
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

    async def assemble_pending(self, builder, text: str, *, is_new_session: bool, **kwargs):
        from gideon.cognition.context_engine import assemble_context, get_engine, ContextBoundaryRefusal
        if get_engine() is not self:
            raise ContextBoundaryRefusal('continuity adapter is no longer installed')
        self._dispatching = True
        try:
            result = assemble_context(builder, text, is_new_session=is_new_session, **kwargs)
            if get_engine() is not self:
                raise ContextBoundaryRefusal('host context authority changed during assembly')
            self._pending_input = result.message
            return result
        finally:
            self._dispatching = False

    async def validate_dispatch(self, session_key: str, provider_input: str, model_id: str, provider_client=None) -> None:
        from gideon.cognition.context_engine import get_engine, ContextBoundaryRefusal
        if self.mode != 'primary':
            return
        try:
            if get_engine() is not self or session_key != self.session_key or self._ticket is None:
                raise ContextBoundaryRefusal('primary dispatch lost its installed turn authority')
            import inspect
            from pathlib import Path
            supported = '2d9537932fc26b290f2c688dd45f2a1016b6113a57cfdb528b56fe723a76d24e'
            if provider_client is None or type(provider_client).__name__ != 'OllamaProvider':
                raise ContextBoundaryRefusal('actual provider framing is not qualified for primary continuity')
            provider_source = Path(inspect.getfile(type(provider_client)))
            if sha256(provider_source.read_bytes()).hexdigest() != supported or provider_client.model != model_id:
                raise ContextBoundaryRefusal('provider serializer or actual model changed')
            output_limit = provider_client.options.get('num_predict')
            if type(output_limit) is not int or not 0 < output_limit <= self.budget.reserved_output_tokens:
                raise ContextBoundaryRefusal('actual provider output limit exceeds or lacks the reservation')
            if type(provider_client.context_window) is not int or provider_client.context_window < self.budget.context_tokens:
                raise ContextBoundaryRefusal('actual provider model window is smaller than the native budget')
            if model_id != self.provider.model_id:
                raise ContextBoundaryRefusal('actual provider model differs from native continuity model')
            if not isinstance(provider_input, str) or not self._pending_input or self._pending_input not in provider_input:
                raise ContextBoundaryRefusal('host hooks removed the verified native context')
            identity = self._tokenizer_identity
            if identity is None:
                raise ContextBoundaryRefusal('trusted tokenizer identity was not captured')
            counted = await self.context.client.count_provider_input(model_id, identity['generation'], provider_input.encode())
            if counted.get('input_sha256') != sha256(provider_input.encode()).hexdigest() or counted.get('identity') != identity:
                raise ContextBoundaryRefusal('final provider input or tokenizer revision changed')
            if type(counted.get('tokens')) is not int or counted['tokens'] < 0 or counted['tokens'] > self.budget.available:
                raise ContextBoundaryRefusal('final host provider input exceeds the output reservation')
            self.final_input_count = counted
            framed_counter = getattr(self.context.client, 'count_ollama_user_input', None)
            if not callable(framed_counter) or not identity.get('text_serializer_sha256'):
                raise ContextBoundaryRefusal('exact provider chat framing is unavailable')
            catalog = await provider_client._client.get('/api/tags')
            catalog.raise_for_status()
            actual_model = next((entry for entry in catalog.json().get('models', []) if entry.get('name') == model_id), None)
            if actual_model is None or actual_model.get('digest') != identity.get('model_revision'):
                raise ContextBoundaryRefusal('actual provider model revision differs from owner registration')
            shown = await provider_client._client.post('/api/show', json={'model': model_id})
            shown.raise_for_status()
            model = shown.json()
            if sha256(model.get('template', '').encode()).hexdigest() != identity['text_serializer_sha256'] or sha256(model.get('system', '').encode()).hexdigest() != identity['system_sha256']:
                raise ContextBoundaryRefusal('actual provider chat template or system differs from owner registration')
            framed = await framed_counter(model_id, identity['generation'], provider_input)
            if framed.get('identity') != identity or type(framed.get('tokens')) is not int or not 0 <= framed['tokens'] <= self.budget.available:
                raise ContextBoundaryRefusal('provider framed input exceeds the output reservation')
            self.final_chat_count = framed
            if self._prepared_digest != self._digest(self._events()) or self._binding() != self._provider_binding:
                raise ContextBoundaryRefusal('host transcript or provider changed during hooks')
            validated = await self.continuity.call({'action': 'post_hook', 'ticket': self._ticket}, generation=self._generation)
            if validated.get('mode') != 'primary' or validated['receipt']['rendered_digest'] != self._ticket['rendered_digest']:
                raise ContextBoundaryRefusal('post-hook native continuity receipt missing')
            self._ticket = None
            if self._prepared_digest != self._digest(self._events()) or self._binding() != self._provider_binding:
                raise ContextBoundaryRefusal('host transcript or provider changed before dispatch')
        except BaseException:
            if self._ticket is not None:
                try:
                    await asyncio.shield(self.cancel_turn())
                except BaseException:
                    pass
            raise

    async def assemble_for_dispatch(self, builder, text: str, *, is_new_session: bool, provider_client=None, **kwargs):
        try:
            result = await self.assemble_pending(builder, text, is_new_session=is_new_session, **kwargs)
            await self.validate_dispatch(self.session_key, result.message, self.provider.model_id, provider_client)
            return result
        except BaseException:
            if self._ticket is not None:
                try:
                    await asyncio.shield(self.cancel_turn())
                except BaseException:
                    pass
            raise

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
        self._pending_input = None
        super().after_turn(session_key)
