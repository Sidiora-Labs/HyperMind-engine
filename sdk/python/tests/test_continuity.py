import asyncio
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest

from hypermind import Engine
from hypermind.context import Scope, SourceMessage, TokenBudget
from hypermind.continuity import ContextContinuation, ContinuationError


class NativeContinuityTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_restart_fork_model_change_and_cancelled_mutation(self):
        target = Path(os.environ.get('HM_SDK_NATIVE_DIR', Path(__file__).resolve().parents[3] / 'target/debug'))
        spec = importlib.util.spec_from_file_location('hypermind._native', target / 'lib_native.so')
        module = importlib.util.module_from_spec(spec)
        sys.modules['hypermind._native'] = module
        spec.loader.exec_module(module)
        scope = Scope('continuity-owner', 'continuity-project')
        with tempfile.TemporaryDirectory() as directory:
            async def open_engine():
                return await Engine.open(directory, actor=7, user_hex='11' * 16, kek_hex='22' * 32, projection_map_bytes=67108864, context_scope=scope)
            engine = await open_engine()
            continuation = ContextContinuation(engine.context(scope, 'parent/%?', actor=7, context_owner='hypermind', conversation='parent'))
            source = SourceMessage('original', 0, 'user', ({'kind': 'text', 'text': 'retained original'},), 1791287999123456789, 1791288000123456789, 'user_asserted').freeze()
            await continuation.ingest_source(source, b'retained original')
            first = await continuation.activate('retained', TokenBudget(8192, 512, 0), options={'model_id': 'gpt-4o'})
            accepted = continuation.accepted
            await continuation.fork('child/%?', 'child')
            await continuation.close()
            engine = await open_engine()
            async def reconnect():
                return engine.context(scope, 'parent/%?', actor=7, context_owner='hypermind', conversation='parent')
            await continuation.reconnect(reconnect)
            self.assertEqual(continuation.accepted.cursor, accepted.cursor)
            self.assertEqual(continuation.accepted.generation, first['report']['generation'])
            self.assertFalse(continuation.generation_compatible)
            child = engine.context(scope, 'child/%?', actor=7, context_owner='hypermind', conversation='child')
            child_report = await child.activate('retained', TokenBudget(8192, 512, 0))
            self.assertIn('original', child_report['report']['included'])
            history = await engine.tool('inspect', {'uri': child.inspect_uri() + '/history'})
            encoded = history['items'][0]['messages'][0]
            self.assertEqual(encoded['source_digest'], source.source_digest)
            self.assertEqual(encoded['recorded_at_ns'], '1791288000123456789')
            changed = await continuation.activate('retained', TokenBudget(8192, 512, 0), options={'model_id': 'gpt-4o-mini'})
            self.assertEqual(continuation.accepted.model_id, 'gpt-4o-mini')
            self.assertGreater(changed['report']['generation'], accepted.generation)
            next_source = SourceMessage('cancelled', 1, 'user', ({'kind': 'text', 'text': 'possibly accepted'},), None, 1791288000123456790, 'user_asserted').freeze()
            task = asyncio.create_task(continuation.ingest_source(next_source, b'possibly accepted'))
            await asyncio.sleep(0)
            task.cancel()
            with self.assertRaises(asyncio.CancelledError): await task
            self.assertEqual(continuation.uncertain.effect_state, 'unknown')
            with self.assertRaises(ContinuationError) as refused:
                await continuation.ingest_source(next_source, b'possibly accepted')
            self.assertEqual(refused.exception.effect_state, 'not_dispatched')
            await asyncio.sleep(0.05)
            await continuation.inspect()
            self.assertIsNotNone(continuation.uncertain)
            await continuation.abandon_uncertain()
            checkpoint = continuation.checkpoint()
            continuation = ContextContinuation(continuation.context, checkpoint)
            self.assertEqual(len(continuation.unresolved), 1)
            with self.assertRaises(ContinuationError): await continuation.ingest_source(next_source, b'possibly accepted')
            await continuation.close()


if __name__ == '__main__':
    unittest.main()
