import importlib.util
import json
from pathlib import Path
import sys
import unittest

# Load the contract independently of optional native and network transports.
spec = importlib.util.spec_from_file_location('hypermind_context_contract', Path(__file__).parents[1] / 'hypermind/context.py')
context = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = context
spec.loader.exec_module(context)


class ContextContractTests(unittest.TestCase):
    def test_scope_budget_and_wire(self):
        scope = context.Scope('owner', 'project')
        client = context.ContextClient(None, scope, 'session', actor=65535, context_owner='host')
        value = json.loads(json.dumps(client.request_context(context.TokenBudget(100, 20, 5))))
        self.assertEqual(value, {'version': 1, 'scope': {'owner_id': 'owner', 'project_id': 'project', 'workspace_id': None}, 'session_id': 'session', 'budget': {'context_tokens': 100, 'reserved_output_tokens': 20, 'required_tokens': 5}, 'generation': 0})
        self.assertIsNone(client.version)
        with self.assertRaises(ValueError): context.Scope('owner with spaces', 'project')
        with self.assertRaises(ValueError): context.TokenBudget(10, 11, 0)
        with self.assertRaises(ValueError): context.Cursor(0, 0)
        with self.assertRaises(context.ContextClientError): client.accept({'ok': False, 'items': [], 'effect_state': 'unknown'})

    def test_source_digest_and_immutable_identity(self):
        part = {'text': 'hello λ', 'kind': 'text'}
        source = context.SourceMessage('m1', 0, 'user', (part,), None, 0, 'user_asserted').freeze()
        self.assertEqual(source.source_digest, '92d70f349962401c258abcb874228c42b5caf13d0b59dcfad98912bbb4a70b6a')
        part['text'] = 'changed'
        self.assertEqual(source.wire()['parts'][0]['text'], 'hello λ')
        with self.assertRaises(TypeError): source.parts[0]['text'] = 'changed'

    def test_current_scale_timestamp_contract(self):
        source = context.SourceMessage('m1', 0, 'user', ({'kind': 'text', 'text': 'hello λ'},), 1791287999123456789, 1791288000123456789, 'user_asserted').freeze()
        self.assertEqual(source.source_digest, '78af0610090d418ca2335976c873cfcc7647f9ce431f98696040d8fa1660500c')
        wire = json.loads(json.dumps(source.wire()))
        self.assertEqual(wire['recorded_at_ns'], '1791288000123456789')
        decoded = context.SourceMessage(**wire)
        self.assertEqual(decoded.wire(), source.wire())
        for bad in ('01', '-0', '9223372036854775808', 1.7912880001234568e18):
            with self.assertRaises(ValueError): context.SourceMessage(**{**wire, 'recorded_at_ns': bad}).wire()

    def test_context_operation_wire_and_opaque_identifiers(self):
        from urllib.parse import unquote
        scope = context.Scope('owner', 'project')
        client = context.ContextClient(None, scope, "s/a%?!'()*", actor=7, context_owner='hypermind', conversation='conversation')
        self.assertEqual(client.inspect_uri(), 'hm://7/context/s%2Fa%25%3F%21%27%28%29%2A')
        self.assertEqual(unquote(client.inspect_uri().split('/')[-1]), client.session_id)
        message = context.SourceMessage('m1', 0, 'user', ({'kind': 'text', 'text': 'hello λ'},), 1791287999123456789, 1791288000123456789, 'user_asserted')
        source = json.loads(json.dumps(client.source_request(message, b'hello')))
        self.assertEqual(source['conversation'], 'conversation')
        self.assertEqual(source['context']['operation'], 'source')
        self.assertEqual(source['context']['request']['message']['source_digest'], '78af0610090d418ca2335976c873cfcc7647f9ce431f98696040d8fa1660500c')
        self.assertEqual(source['context']['request']['original_bytes'], list(b'hello'))
        relation = {'kind': 'tombstone', 'id': 'r1', 'source_id': 'm1'}
        self.assertEqual(client.relation_request(relation)['context']['request']['relation'], relation)
        fork = client.fork_request('child/session', 'child-conversation')
        self.assertEqual(fork['conversation'], 'child-conversation')
        self.assertEqual(fork['context']['request']['parent_conversation'], 'conversation')
        action = {'action': 'maintenance_enqueue', 'session_id': client.session_id, 'request': {'kind': 'verification', 'sources': [message], 'cursor': {'epoch': 1, 'sequence': 1}, 'source_revision': 1, 'policy_revision': 1, 'reservation': 5}}
        job = json.loads(json.dumps(client.job_request('j1', action)))
        self.assertEqual(job['context']['operation'], 'job')
        self.assertEqual(job['context']['request']['action']['request']['sources'][0]['recorded_at_ns'], '1791288000123456789')


if __name__ == '__main__' : unittest.main()
