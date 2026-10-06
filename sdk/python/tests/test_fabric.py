import asyncio
from dataclasses import asdict
from hashlib import sha256
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import tempfile
import unittest

from hypermind.client import Client, HyperMindError
from hypermind.context import Scope
from hypermind.fabric import FabricClient


class ActualDaemon:
    def __init__(self, *, development=False):
        self.directory = tempfile.TemporaryDirectory()
        self.root = Path(self.directory.name)
        self.scope = Scope('sdk-owner', 'sdk-service')
        self.binary = Path(os.environ['HM_SDK_HM_BINARY'])
        self.token = os.urandom(32)
        self.development = development
        self.process = None
        self.clients = []
        self.home = self.root / 'runtime'; self.home.mkdir()
        self.fabric_path = self.root / 'fabric.json'
        limits = dict(resources=128, events=4096, queue_depth=256, consumers=256, grants=256, registers=256, dead_letters=256)
        self.fabric_path.write_text(json.dumps({'version': 1, 'actor': 7,
            'backend': {'version': 1, 'scope': asdict(self.scope), 'home': {'path': str(self.home), 'base': None, 'missing': {'policy': 'refuse'}},
                'operational': {'backend': 'sqlite'}, 'bus': {'backend': 'process_local', 'limits': limits}},
            'worker': {'module_id': 'sdk-digest', 'command': str(self.binary), 'args': ['fabric-worker'], 'cwd': str(self.home)},
            'client_key': list(os.urandom(32)), 'worker_key': list(os.urandom(32))}))
        self.fabric_path.chmod(0o600)
        self.context = self.root / 'context.json'
        self.context.write_text(json.dumps({'version': 1, 'actor': 7, 'scope': asdict(self.scope)})); self.context.chmod(0o600)
        self.config = self.root / 'server.conf'
        self.config.write_text('\n'.join([f'socket={self.root / "server.sock"}', f'data={self.root / "ledger"}',
            'user=' + os.urandom(16).hex(), 'kek=' + os.urandom(32).hex(), 'admin_token=' + os.urandom(32).hex(),
            'actor=7:' + self.token.hex(), 'projection_map_bytes=16777216']) + '\n')
        self.config.chmod(0o600)
        def openssl(*arguments): subprocess.run(['openssl', *arguments], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        openssl('req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', str(self.root / 'ca.key'), '-out', str(self.root / 'ca.pem'), '-days', '1', '-subj', '/CN=SDK test CA')
        for name, usage in [('server', 'serverAuth'), ('client', 'clientAuth')]:
            openssl('req', '-new', '-newkey', 'rsa:2048', '-nodes', '-keyout', str(self.root / f'{name}.key'), '-out', str(self.root / f'{name}.csr'), '-subj', '/CN=localhost')
            extension = self.root / f'{name}.ext'
            extension.write_text(f'subjectAltName=DNS:localhost,IP:127.0.0.1\nextendedKeyUsage={usage}\n')
            openssl('x509', '-req', '-in', str(self.root / f'{name}.csr'), '-CA', str(self.root / 'ca.pem'), '-CAkey', str(self.root / 'ca.key'), '-CAcreateserial', '-out', str(self.root / f'{name}.pem'), '-days', '1', '-extfile', str(extension))
        with socket.socket() as s:
            s.bind(('127.0.0.1', 0)); self.port = s.getsockname()[1]

    async def start(self):
        environment = os.environ.copy()
        environment['HM_FABRIC_CONFIG'] = str(self.fabric_path)
        environment['HM_EMBEDDING_PROVIDER'] = 'off'
        if self.development:
            environment.update(HM_DEVELOPMENT_PROVIDER='ollama', HM_DEVELOPMENT_ENDPOINT=os.environ.get('HM_DEVELOPMENT_ENDPOINT', 'http://127.0.0.1:11434'), HM_DEVELOPMENT_MODEL=os.environ.get('HM_DEVELOPMENT_MODEL', 'qwen3:0.6b'),
                HM_DEVELOPMENT_WORKERS=json.dumps([{'id': 'retrospective', 'operation': {'family': 'retrospective', 'checkpoint_id': 'checkpoint', 'lesson_ids': ['lesson']}}]))
        else: environment['HM_DEVELOPMENT_PROVIDER'] = 'off'
        self.log = open(self.root / 'daemon.log', 'ab')
        self.process = subprocess.Popen([str(self.binary), 'serve', '--config', str(self.config), '--context-scope', str(self.context),
            '--grpc-bind', f'127.0.0.1:{self.port}', '--tls-cert', str(self.root / 'server.pem'), '--tls-key', str(self.root / 'server.key'), '--tls-client-ca', str(self.root / 'ca.pem')], env=environment, stdout=self.log, stderr=self.log)
        deadline = asyncio.get_running_loop().time() + 20
        while True:
            if self.process.poll() is not None: raise AssertionError('actual daemon startup failed: ' + (self.root / 'daemon.log').read_text()[-4000:])
            try:
                reader, writer = await asyncio.open_connection('127.0.0.1', self.port)
                writer.close(); await writer.wait_closed(); break
            except OSError:
                if asyncio.get_running_loop().time() >= deadline: raise AssertionError('actual daemon startup timed out')
                await asyncio.sleep(.02)
        return await self.client()

    async def client(self, *, token=None):
        client = Client(f'localhost:{self.port}', token=self.token if token is None else token,
            ca=(self.root / 'ca.pem').read_bytes(), certificate=(self.root / 'client.pem').read_bytes(), private_key=(self.root / 'client.key').read_bytes())
        self.clients.append(client)
        await client.connect()
        return client

    async def stop(self):
        for client in self.clients: await client.close()
        self.clients.clear()
        if self.process is not None and self.process.poll() is None:
            self.process.terminate()
            try: await asyncio.to_thread(self.process.wait, 10)
            except subprocess.TimeoutExpired:
                self.process.kill(); await asyncio.to_thread(self.process.wait)
        if hasattr(self, 'log'): self.log.close()

    async def close(self):
        await self.stop()
        evidence = os.environ.get('HM_SDK_JOURNEY_LOG_DIR')
        if evidence:
            destination = Path(evidence); destination.mkdir(parents=True, exist_ok=True)
            (destination / ('development-daemon.log' if self.development else 'fabric-daemon.log')).write_bytes((self.root / 'daemon.log').read_bytes())
        self.directory.cleanup()


class FabricConsumerTests(unittest.IsolatedAsyncioTestCase):
    async def test_actual_daemon_dispatch_restart_and_uncertain_cancel(self):
        daemon = ActualDaemon()
        try:
            client = await daemon.start()
            fabric = FabricClient(client, daemon.scope, actor=7)
            view = await fabric.inspect()
            self.assertEqual(view['scope'], asdict(daemon.scope))
            self.assertTrue(view['receipts_available'])
            self.assertTrue(any(not f['available'] for f in view['descriptors']['families']))
            data = b'exact binary\x00\xff'
            result = await fabric.dispatch_digest('completed', data)
            self.assertEqual(result['result']['digest'], sha256(data).hexdigest())
            self.assertEqual(result['effect']['state'], 'terminal')
            self.assertEqual(await fabric.dispatch_digest('completed', data), result)
            with self.assertRaises(HyperMindError): await fabric.dispatch_digest('completed', b'changed')
            with self.assertRaises(HyperMindError): await FabricClient(client, Scope('foreign', 'sdk-service'), actor=7).dispatch_digest('foreign', b'foreign')
            await daemon.stop()
            client = await daemon.start(); fabric = FabricClient(client, daemon.scope, actor=7)
            self.assertEqual(await fabric.dispatch_digest('completed', data), result)
            controller = FabricClient(await daemon.client(), daemon.scope, actor=7)
            children_path = Path(f'/proc/{daemon.process.pid}/task/{daemon.process.pid}/children')
            children = [int(p) for p in children_path.read_text().split()]
            if not children:
                children = [int(p) for path in Path(f'/proc/{daemon.process.pid}/task').glob('*/children') for p in path.read_text().split()]
            self.assertEqual(len(set(children)), 1)
            os.kill(children[0], signal.SIGSTOP)
            task = asyncio.create_task(fabric.dispatch_digest('uncertain', b'pending original', timeout_ms=10000))
            deadline = asyncio.get_running_loop().time() + 5
            while True:
                busy = await controller.inspect()
                if (busy.get('active_effect') or {}).get('phase') == 'dispatched': break
                if asyncio.get_running_loop().time() > deadline: self.fail('real dispatch phase was not reached')
                await asyncio.sleep(.01)
            cancelled = await controller.cancel('cancel-request', 'uncertain')
            self.assertTrue(cancelled['cancel_requested'])
            self.assertTrue(cancelled['worker_stopped'])
            self.assertFalse(cancelled['automatic_replay'])
            self.assertEqual(cancelled['effect']['state'], 'uncertain')
            with self.assertRaises(HyperMindError) as failed: await task
            self.assertEqual(failed.exception.effect_state, 'unknown')
            with self.assertRaises(HyperMindError) as unavailable: await controller.reconcile('reconcile-request', 'uncertain')
            self.assertEqual(unavailable.exception.effect_state, 'unknown')
            receipts = await controller.receipts('receipt-request')
            self.assertTrue(receipts['receipts'])
            await daemon.stop()
            client = await daemon.start(); fabric = FabricClient(client, daemon.scope, actor=7)
            restored = await fabric.inspect()
            self.assertTrue(any(r['intent']['key'] == 'uncertain' and r['intent']['state'] == 'uncertain' for r in restored['receipts']))
        finally: await daemon.close()


if __name__ == '__main__': unittest.main()
