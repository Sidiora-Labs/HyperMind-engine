import asyncio
from dataclasses import asdict
import json
import os
from pathlib import Path
import unittest

from hypermind.context import Scope
from hypermind import UsageClient, decode_usage_view, decode_provider_usage
from hypermind.usage import UsageDecodeError
from test_fabric import ActualDaemon


class UsageConsumerTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_daemon_both_clients_and_exact_native_vectors(self):
        fixture = Path(os.environ['HM_USAGE_CONSUMER_FIXTURE_DIR'])
        manifest = json.loads((fixture / 'manifest.json').read_text())
        daemon = ActualDaemon()
        try:
            daemon.scope = Scope(**manifest['scope'])
            daemon.context.write_text(json.dumps({'version': 1, 'actor': manifest['actor'], 'scope': asdict(daemon.scope)}))
            fabric = json.loads(daemon.fabric_path.read_text()); fabric['backend']['scope'] = asdict(daemon.scope)
            daemon.fabric_path.write_text(json.dumps(fabric))
            lines = daemon.config.read_text().splitlines()
            replacements = {'data': str(Path(manifest['actor_directory']).parent), 'user': manifest['user_hex'], 'kek': manifest['kek_hex'], 'projection_map_bytes': '67108864'}
            daemon.config.write_text('\n'.join(key + '=' + replacements.get(key, value) for key, value in (line.split('=', 1) for line in lines)) + '\n')
            client = await daemon.start()
            usage = await UsageClient(client, daemon.scope, actor=7).inspect()
            expected = json.loads((fixture / 'usage_expected.json').read_text())
            self.assertEqual(usage, decode_usage_view(expected, scope=daemon.scope))
            self.assertEqual(usage['rollup']['held_tokens'], 9007199254740993)
            self.assertEqual(usage['rollup']['known_tokens'], 0)
            self.assertEqual(usage['rollup']['observed_calls'], 0)
            self.assertEqual(usage['rollup']['unknown_reservations'], 1)
            self.assertEqual(usage['state']['limits']['total_tokens'], 18446744073709551615)
            self.assertIsNone(usage['state']['reservations'][0]['settled_tokens'])
            self.assertEqual(usage['state']['observations'], [])
            self.assertEqual(usage['rollup']['attributions'][0]['turn_id'], 'precision-turn')
            recorded = json.loads((fixture / 'usage_parser_vectors.json').read_text())
            self.assertEqual(recorded['origin'], 'recorded_parser_vectors_only'); self.assertFalse(recorded['billed'])
            tokens = decode_provider_usage(recorded['tokens']); quota = decode_provider_usage(recorded['quota'])
            self.assertEqual(tokens['tokens']['input'], 9007199254740993); self.assertEqual(tokens['tokens']['output'], 0)
            self.assertIsNone(tokens['tokens']['cache_read']); self.assertIsNone(tokens['expires_at_ns'])
            self.assertEqual(tokens['reported_charge']['magnitude']['nanodollar_numerator'], 9007199254740993)
            window = quota['quota_windows'][0]
            self.assertEqual(window['limit']['value'], 18446744073709551615); self.assertEqual(window['remaining']['value'], 0); self.assertIsNone(window['used'])
            self.assertEqual(window['starts_at_ns'], -9007199254740993); self.assertEqual(window['resets_at_ns'], 9223372036854775807)
            self.assertEqual(window['interval_ms'], 9007199254740993)
            absent = json.loads(json.dumps(recorded['tokens'])); del absent['tokens']['cache_read']
            self.assertNotIn('cache_read', decode_provider_usage(absent)['tokens'])
            for malformed in (9007199254740993, '01', '+1', '-0', '18446744073709551616', '1.0', '1\n', '1 ', True):
                bad = json.loads(json.dumps(expected)); bad['rollup']['held_tokens'] = malformed
                with self.assertRaises(UsageDecodeError): decode_usage_view(bad)
            legacy = json.loads(json.dumps(expected)); legacy['rollup']['known_tokens'] = 0
            self.assertEqual(decode_usage_view(legacy)['rollup']['known_tokens'], 0)
            for malformed in (9007199254740993, '9223372036854775808', '-9223372036854775809', '+1', '-0'):
                bad = json.loads(json.dumps(recorded['tokens'])); bad['observed_at_ns'] = malformed
                with self.assertRaises(UsageDecodeError): decode_provider_usage(bad)
            leaked = json.loads(json.dumps(recorded['tokens'])); leaked['raw'] = {'content': 'excluded'}
            with self.assertRaises(UsageDecodeError): decode_provider_usage(leaked)
            connection = daemon.root / 'usage-client.json'
            connection.write_text(json.dumps({'target': f'localhost:{daemon.port}', 'token_hex': daemon.token.hex(), 'scope': asdict(daemon.scope),
                'ca': str(daemon.root / 'ca.pem'), 'certificate': str(daemon.root / 'client.pem'), 'private_key': str(daemon.root / 'client.key'),
                'expected': str(fixture / 'usage_expected.json'), 'vectors': str(fixture / 'usage_parser_vectors.json')})); connection.chmod(0o600)
            environment = {**os.environ, 'HM_USAGE_CLIENT_CONFIG': str(connection)}
            node = os.environ['HM_SDK_NODE']
            test = Path(__file__).resolve().parents[2] / 'typescript/packages/client/dist/usage.test.js'
            process = await asyncio.create_subprocess_exec(node, '--test', str(test), env=environment, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.STDOUT)
            output, _ = await asyncio.wait_for(process.communicate(), 30)
            self.assertEqual(process.returncode, 0, output.decode(errors='replace'))
            print(output.decode(errors='replace'))
            await daemon.stop()
            client = await daemon.start()
            self.assertEqual(await UsageClient(client, daemon.scope, actor=7).inspect(), usage)
        finally: await daemon.close()


if __name__ == '__main__': unittest.main()
