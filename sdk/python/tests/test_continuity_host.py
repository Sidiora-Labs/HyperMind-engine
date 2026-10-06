import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest

runtime = os.environ.get('GIDEON_RUNTIME_PATH')
if runtime:
    sys.path.insert(0, runtime)
from hypermind import Engine
native_library = os.environ.get('HYPERMIND_NATIVE_LIBRARY')
if native_library:
    native_spec = importlib.util.spec_from_file_location('hypermind._native', native_library)
    native = importlib.util.module_from_spec(native_spec)
    sys.modules[native_spec.name] = native
    native_spec.loader.exec_module(native)
from hypermind.context import ContextClient, Scope, TokenBudget
from hypermind.continuity_host import GideonContinuityAdapter
from hypermind.gideon import ProviderProfile
from gideon.cognition.context_engine import prepare_context_turn, assemble_context, set_engine, ContextBoundaryRefusal
from gideon.cognition.context import PromptAssembler
from gideon.cognition.history import ConversationLog
from gideon.cognition.memory import MemoryJournal


class InstalledContinuityJourney(unittest.IsolatedAsyncioTestCase):
    async def test_native_modes_preserve_host_bytes_shadow_cursor_and_primary_fences(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            previous_home = os.environ.get('GIDEON_HOME')
            os.environ['GIDEON_HOME'] = str(root / 'home')
            engine = adapter = None
            try:
                scope = Scope('owner', 'continuity-host')
                engine = await Engine.open(root / 'engine', actor=7, user_hex='11' * 16, kek_hex='22' * 32,
                                           projection_map_bytes=67108864, context_scope=scope)
                context = ContextClient(engine, scope, 'continuity-session', actor=7, context_owner='hypermind', conversation='host-conversation')
                budget = TokenBudget(16000, 2000, 1000)
                await context.activate('register explicit context session', budget)
                log = ConversationLog(root / 'sessions')
                log.append('main', 'user', 'The observed alloy is copper-29.')
                log.append('main', 'assistant', 'Recorded copper-29.')
                log.append('main', 'user', 'What alloy was observed?')
                memory = MemoryJournal(workspace=root / 'memory')
                memory.init()
                builder = PromptAssembler(memory=memory, conversation_log=log)
                adapter = GideonContinuityAdapter(context, log, 'main', budget=budget, provider=ProviderProfile('gpt-4o'), context_owner='hypermind')
                adapter.install()
                kwargs = dict(is_new_session=False, session_key='main', blocks_reads=True, active_recall=False)
                baseline = adapter.delegate.assemble(builder, 'What alloy was observed?', **kwargs)
                before = await engine.tool('inspect', {'uri': context.inspect_uri()})
                await prepare_context_turn('main')
                off = assemble_context(builder, 'What alloy was observed?', **kwargs)
                self.assertEqual(off.message.encode(), baseline.message.encode())
                self.assertEqual(off.metadata, baseline.metadata)
                self.assertFalse(adapter.owns_compaction)
                adapter.queue_mode('pass_through')
                self.assertEqual(adapter.mode, 'off')
                self.assertEqual(assemble_context(builder, 'What alloy was observed?', **kwargs).message, baseline.message)
                adapter.after_turn('main')
                await prepare_context_turn('main')
                self.assertEqual(adapter.mode, 'pass_through')
                self.assertEqual(assemble_context(builder, 'What alloy was observed?', **kwargs).message.encode(), baseline.message.encode())
                after = await engine.tool('inspect', {'uri': context.inspect_uri()})
                self.assertEqual(after['items'][0]['report']['cursor'], before['items'][0]['report']['cursor'])
                adapter.after_turn('main')
                await adapter.sync()
                await context.activate('What alloy was observed?', budget, options={'model_id': 'gpt-4o', 'profile': adapter.provider.wire()})
                serving_cursor = context.cursor
                policy_before_shadow = await adapter.continuity.call({'action': 'inspect'}, generation=await adapter.continuity.generation())
                adapter.queue_mode('shadow')
                await prepare_context_turn('main')
                shadow_before = await adapter.continuity.call({'action': 'inspect'}, generation=await adapter.continuity.generation())
                self.assertEqual(shadow_before['sequence'], policy_before_shadow['sequence'] + 1)
                self.assertEqual(len(shadow_before['receipts']), len(policy_before_shadow['receipts']) + 1)
                self.assertEqual(shadow_before['cursor'], policy_before_shadow['cursor'])
                result = await adapter.assemble_for_dispatch(builder, 'What alloy was observed?', **kwargs)
                self.assertEqual(result.message.encode(), baseline.message.encode())
                self.assertEqual(result.metadata, baseline.metadata)
                self.assertFalse(adapter.comparison['published'])
                self.assertEqual(context.cursor, serving_cursor)
                shadow_after = await adapter.continuity.call({'action': 'inspect'}, generation=await adapter.continuity.generation())
                self.assertEqual(shadow_before['sequence'], shadow_after['sequence'])
                self.assertEqual(shadow_before['cursor'], shadow_after['cursor'])
                self.assertEqual(shadow_before['receipts'], shadow_after['receipts'])
                adapter.after_turn('main')
                from gideon.cognition import context_engine
                if not callable(getattr(context_engine, '_fails_closed', None)):
                    state_before = await adapter.continuity.call({'action': 'inspect'}, generation=await adapter.continuity.generation())
                    with self.assertRaisesRegex(ValueError, 'primary refusal authority'):
                        adapter.queue_mode('primary')
                    self.assertEqual(adapter.mode, 'shadow')
                    self.assertIsNone(adapter._pending_mode)
                    self.assertFalse(adapter.owns_compaction)
                    state_after = await adapter.continuity.call({'action': 'inspect'}, generation=await adapter.continuity.generation())
                    self.assertEqual(state_after, state_before)
                    original = log.source_events('main')[0]
                    self.assertEqual(log.resolve_source_event('main', original.source_event_id), original.raw_bytes)
                    return
                adapter.queue_mode('primary')
                await prepare_context_turn('main')
                self.assertTrue(adapter.owns_compaction)
                with self.assertRaises(ContextBoundaryRefusal):
                    assemble_context(builder, 'What alloy was observed?', **kwargs)
                primary = await adapter.assemble_for_dispatch(builder, 'What alloy was observed?', **kwargs)
                self.assertIn('copper-29', primary.message)
                self.assertIn('hypermind', primary.metadata)
                self.assertIsNone(adapter._ticket)
                adapter.queue_mode('off')
                self.assertEqual(adapter.mode, 'primary')
                adapter.after_turn('main')
                await prepare_context_turn('main')
                self.assertEqual(adapter.mode, 'off')
                self.assertEqual(assemble_context(builder, 'What alloy was observed?', **kwargs).message.encode(), baseline.message.encode())
                adapter.after_turn('main')
                adapter.queue_mode('primary')
                await prepare_context_turn('main')
                log.append('main', 'user', 'New evidence arrived during the hook boundary.')
                with self.assertRaises(ContextBoundaryRefusal):
                    await adapter.assemble_for_dispatch(builder, 'What alloy was observed?', **kwargs)
                self.assertIsNone(adapter._ticket)
                state = await adapter.continuity.call({'action': 'inspect'}, generation=await adapter.continuity.generation())
                self.assertIsNone(state['active_hook'])
                await prepare_context_turn('main')
                adapter.provider = ProviderProfile('gpt-4o-mini')
                with self.assertRaises(ContextBoundaryRefusal):
                    await adapter.assemble_for_dispatch(builder, 'What alloy was observed?', **kwargs)
                self.assertIsNone(adapter._ticket)
                original = log.source_events('main')[0]
                self.assertEqual(log.resolve_source_event('main', original.source_event_id), original.raw_bytes)
            finally:
                if adapter:
                    adapter.uninstall()
                set_engine(None)
                if engine:
                    await engine.close()
                if previous_home is None:
                    os.environ.pop('GIDEON_HOME', None)
                else:
                    os.environ['GIDEON_HOME'] = previous_home


if __name__ == '__main__':
    unittest.main()
