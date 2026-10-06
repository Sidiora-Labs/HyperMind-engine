import os
import importlib.util
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
from hypermind.gideon import GideonContextAdapter, ProviderProfile
from gideon.cognition.history import ConversationLog
from gideon.cognition.context import PromptAssembler
from gideon.cognition.memory import MemoryJournal
from gideon.cognition.context_engine import get_engine, prepare_context_turn, assemble_context, set_engine


class GideonContextJourney(unittest.IsolatedAsyncioTestCase):
    async def test_installed_host_engine_retry_edit_fork_resume(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            previous_home = os.environ.get('GIDEON_HOME')
            os.environ['GIDEON_HOME'] = str(root / 'host-home')
            engine = None
            adapter = None
            try:
                scope = Scope('gideon-owner', 'gideon-project')
                options = dict(actor=7, user_hex='11' * 16, kek_hex='22' * 32, projection_map_bytes=67108864, context_scope=scope)
                engine = await Engine.open(root / 'engine', **options)
                log = ConversationLog(root / 'sessions')
                log.append('main', 'user', 'The observed release is copper-17.')
                log.append('main', 'assistant', 'Recorded copper-17.')
                log.append('main', 'user', 'What release was observed?')
                context = ContextClient(engine, scope, 'main-context', actor=7, context_owner='hypermind', conversation='main-conversation')
                adapter = GideonContextAdapter(context, log, 'main', budget=TokenBudget(16000, 2000, 1000), provider=ProviderProfile('gpt-4o'), context_owner='hypermind')
                self.assertFalse(adapter.owns_compaction)
                adapter.install()
                await prepare_context_turn('main')
                self.assertIs(get_engine(), adapter)
                self.assertTrue(adapter.owns_compaction)
                count = adapter.diagnostics['history']['message_count']
                cursor = context.cursor
                await prepare_context_turn('main')
                self.assertEqual(adapter.diagnostics['history']['message_count'], count)
                self.assertEqual(context.cursor, cursor)
                memory = MemoryJournal(workspace=root / 'memory')
                memory.init()
                builder = PromptAssembler(memory=memory, conversation_log=log)
                result = assemble_context(builder, 'What release was observed?', is_new_session=False, session_key='main', blocks_reads=True)
                self.assertIn('copper-17', result.message)
                self.assertEqual(result.metadata['hypermind']['reserved_output_tokens'], 2000)
                self.assertTrue(any(component.source == 'hypermind' for component in result.components))
                original = log.source_events('main')[0]
                log.append('main', 'user', 'The corrected release is copper-18.')
                replacement = log.source_events('main')[-1]
                await adapter.edit('release-edit', original.source_event_id, replacement.source_event_id)
                await prepare_context_turn('main')
                ids = [message['id'] for message in adapter.diagnostics['messages']]
                self.assertNotIn(original.source_event_id, ids)
                self.assertIn(replacement.source_event_id, ids)
                recovered = await engine.tool('inspect', {'uri': f'hm://7/context/main-context/source/{original.source_event_id}'})
                self.assertTrue(recovered['ok'], recovered)
                child = await adapter.fork('child-context', 'child-conversation', 'child')
                log.append('child', 'user', 'Use this frozen child context.')
                await child.prepare_turn('child')
                self.assertIn(replacement.source_event_id, [message['id'] for message in child.diagnostics['messages']])
                child_cursor = child.context.cursor
                child_state = child.checkpoint()
                adapter.uninstall()
                self.assertFalse(adapter.owns_compaction)
                await engine.close()
                engine = await Engine.open(root / 'engine', **options)
                resumed_context = ContextClient(engine, scope, 'child-context', actor=7, context_owner='hypermind', conversation='child-conversation')
                child = GideonContextAdapter.restore(resumed_context, log, child_state, budget=TokenBudget(16000, 2000, 1000), provider=ProviderProfile('gpt-4o'))
                resumed = await child.resume(child_cursor)
                self.assertEqual(resumed['report']['cursor'], {'epoch': child_cursor.epoch, 'sequence': child_cursor.sequence})
                await child.prepare_turn('child')
                self.assertEqual(child.context.cursor, child_cursor)
                self.assertEqual(log.resolve_source_event('main', original.source_event_id), original.raw_bytes)
                host_context = ContextClient(engine, scope, 'host-context', actor=7, context_owner='host')
                host_adapter = GideonContextAdapter(host_context, log, 'main', budget=TokenBudget(16000, 2000, 1000), provider=ProviderProfile('gpt-4o'), context_owner='host')
                self.assertFalse(host_adapter.owns_compaction)
                baseline = host_adapter.delegate.assemble(builder, 'native path', is_new_session=False, session_key='main', blocks_reads=True, active_recall=False)
                passthrough = host_adapter.assemble(builder, 'native path', is_new_session=False, session_key='main', blocks_reads=True, active_recall=False)
                self.assertEqual(passthrough.message, baseline.message)
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
